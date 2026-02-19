//! Reproduction test for `RingBuffer` race condition.
//!
//! Spawns multiple threads to write to the `RingBuffer` concurrently
//! and verifies data integrity.

#![cfg(feature = "kernel")]
#![allow(clippy::all, clippy::pedantic)]
#![allow(clippy::unwrap_used, clippy::panic)]

use std::sync::{Arc, Barrier};
use std::thread;
use tardis_telemetry::kernel::RingBuffer;
use tardis_telemetry::types::{EventType, Level, SpanId, Subsystem, TelemetryEntry, TraceId};

#[test]
fn race_condition_repro() {
    // Use static to avoid stack overflow
    static BUFFER: RingBuffer = RingBuffer::new();
    let buffer_ref = &BUFFER;

    const THREADS: usize = 8;
    const WRITES_PER_THREAD: usize = 100_000;

    let barrier = Arc::new(Barrier::new(THREADS));
    let mut handles = vec![];

    // Producers
    for t_id in 0..THREADS {
        let b = barrier.clone();
        handles.push(thread::spawn(move || {
            b.wait(); // Synchronize start
            for i in 0..WRITES_PER_THREAD {
                let timestamp = (t_id * WRITES_PER_THREAD + i) as u64;
                let entry = TelemetryEntry {
                    timestamp_ns: timestamp,
                    level: Level::Info,
                    subsystem: Subsystem::Kernel,
                    event_type: EventType::Log,
                    span_id: SpanId::NONE,
                    trace_id: TraceId::NONE,
                    parent_span_id: SpanId::NONE,
                    payload_len: 0,
                };

                // Write a larger payload to increase critical section duration
                let mut payload_str = format!("payload-{}-{}", t_id, i);
                while payload_str.len() < 150 {
                    payload_str.push('.');
                }
                let payload = payload_str.into_bytes();

                buffer_ref.try_write(&entry, &payload);
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    // Verify consistency
    let mut valid_count = 0;

    while let Some((entry, payload)) = buffer_ref.try_read() {
        let t_id = entry.timestamp_ns / (WRITES_PER_THREAD as u64);
        let i = entry.timestamp_ns % (WRITES_PER_THREAD as u64);

        let mut expected_str = format!("payload-{}-{}", t_id, i);
        while expected_str.len() < 150 {
            expected_str.push('.');
        }
        let expected = expected_str.into_bytes();

        let payload_vec: Vec<u8> = payload;
        // Check strict equality.
        if payload_vec != expected {
            // CORRUPTION DETECTED
            panic!("Corruption detected! Entry: t_id={}, i={}. Expected len={}, Got len={}. Payload prefix: {:?}",
                 t_id, i, expected.len(), payload_vec.len(), String::from_utf8_lossy(&payload_vec.iter().take(20).cloned().collect::<Vec<u8>>()));
        }
        valid_count += 1;
    }

    println!(
        "Read {} valid entries out of {} writes",
        valid_count,
        THREADS * WRITES_PER_THREAD
    );
}
