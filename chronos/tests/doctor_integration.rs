#![cfg(feature = "nova")]
#![allow(missing_docs)]
#![allow(clippy::unwrap_used, clippy::panic)]

use chrono::{Duration, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tardis_chronos::experimental::doctor::{HealthStatus, SystemDoctor};
use tardis_gallifrey::domain::{
    Change, ChangeType, ProcessState, SnapshotTrigger, SystemState,
};
use tardis_gallifrey::Gallifrey;
use tardis_telemetry::gallifrey::TelemetryStore;
use tardis_telemetry::types::{Level, Subsystem, TraceId};
use tardis_telemetry::userspace::layer::EventData;

#[tokio::test]
async fn test_doctor_integration() {
    // 1. Setup
    let telemetry = Arc::new(TelemetryStore::new());
    let gallifrey = Arc::new(Gallifrey::new());
    let doctor = SystemDoctor::new(Arc::clone(&telemetry), Arc::clone(&gallifrey));

    // 2. Inject a past snapshot (1 hour ago) with low memory usage
    // Note: We can't force a past timestamp via take_snapshot publicly, so this tests the mechanism
    // but the timestamp will be "now". The change detection (below) allows explicit timestamps.
    let _past_timestamp = Utc::now() - Duration::hours(1);
    let mut processes = HashMap::new();
    processes.insert(
        1234,
        ProcessState {
            pid: 1234,
            name: "vortex-service".to_string(),
            status: "Running".to_string(),
            memory_bytes: 100_000_000, // 100 MB
            cpu_percent: 10.0,
        },
    );

    let state = SystemState {
        processes,
        config: HashMap::new(),
        files: HashMap::new(),
    };

    gallifrey
        .system_state()
        .take_snapshot("healthy-state", SnapshotTrigger::Scheduled, state) // This uses Utc::now() inside, so we can't easily force a past timestamp via public API unless we mock time or use a different method.
        // Wait, SystemStateStore::take_snapshot uses Utc::now(). I can't force it to be in the past.
        // I might need to use internal methods or just accept that I can't test the time-travel aspect easily without mocking time.
        // HOWEVER, `find_snapshot_at` finds the *closest* snapshot before the timestamp.
        // If I take a snapshot NOW, and ask for a snapshot from 1 hour ago, I won't find it.
        //
        // Workaround: I can't easily inject a past snapshot via public API if it enforces `Utc::now()`.
        // BUT, `SystemStateStore` is in `gallifrey`. Maybe I can manually insert it if I have access?
        // No, private fields.
        //
        // Alternative: The test can just verify the *Change* detection, which I can inject because `Change` struct has a timestamp field that I control when creating the `Change` object!
        // `SystemStateStore::record_change` takes a `Change` object.
        .unwrap();

    // Let's focus on the Change detection first, as that's easier to control.
    let change = Change {
        timestamp: Utc::now() - Duration::minutes(2), // 2 minutes ago
        path: "/etc/tardis/config.toml".to_string(),
        change_type: ChangeType::Update,
        old_value: None,
        new_value: None,
    };
    gallifrey.system_state().record_change(change).unwrap();

    // 3. Inject a Telemetry error to make the patient "sick"
    let error_event = EventData {
        span_id: None,
        trace_id: TraceId::generate(),
        timestamp: Utc::now(),
        level: Level::Error,
        message: "Connection refused".to_string(),
        fields: HashMap::new(),
        subsystem: Subsystem::Vortex,
    };
    telemetry.record_event(error_event).await.unwrap();

    // 4. Diagnose
    let diagnosis = doctor.diagnose().await;

    // 5. Assertions
    assert_ne!(diagnosis.status, HealthStatus::Healthy, "System should be unhealthy due to error");

    // Check if root causes contain our change
    let found_change = diagnosis.root_causes.iter().any(|cause|
        cause.contains("Configuration change") && cause.contains("/etc/tardis/config.toml")
    );

    assert!(found_change, "Doctor should identify the recent config change as a root cause");

    // Check prescription
    if let Some(prescription) = diagnosis.prescription {
        assert!(prescription.description.contains("Check recent system state changes"), "Prescription should mention checking changes");
    } else {
        panic!("Doctor should prescribe a fix");
    }
}
