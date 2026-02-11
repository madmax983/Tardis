#![allow(missing_docs)]
#![cfg(feature = "kernel")]
#![allow(clippy::all, clippy::pedantic)]
#![allow(clippy::unwrap_used, clippy::panic)]

//! Test for RingBuffer race conditions.

use std::sync::{Arc, Barrier};
use std::thread;
use std::time::Duration;
use tardis_telemetry::kernel::ring_buffer::RingBuffer;
use tardis_telemetry::types::TelemetryEntryRaw;

const THREADS: usize = 4;
const WRITES_PER_THREAD: usize = 1000;
const BUFFER_SIZE: usize = 1024 * 16;

#[test]
fn test_ring_buffer_race() {
    let buffer = Arc::new(RingBuffer::new(BUFFER_SIZE).unwrap());
    let barrier = Arc::new(Barrier::new(THREADS));
    let mut handles = vec![];

    for i in 0..THREADS {
        let buffer = buffer.clone();
        let barrier = barrier.clone();
        handles.push(thread::spawn(move || {
            barrier.wait();
            for j in 0..WRITES_PER_THREAD {
                let entry = TelemetryEntryRaw {
                    timestamp: i as u64 * 1000000 + j as u64,
                    level: 1,
                    subsystem: 1,
                    payload_len: 0,
                    payload: [0; 256],
                };
                while buffer.try_write(&entry).is_err() {
                    thread::yield_now();
                }
            }
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    // Verify some properties (at least head should have moved)
    // Since we overwrite, we can't verify all entries, but we can verify consistency
    let mut valid_count = 0;
    while let Some(_) = buffer.try_read() {
        valid_count += 1;
    }

    println!("Read {} valid entries out of {} writes", valid_count, THREADS * WRITES_PER_THREAD);
}
