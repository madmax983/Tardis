#![cfg(feature = "nova")]
#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]

use chrono::{Duration, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tardis_chronos::experimental::doctor::{HealthStatus, SystemDoctor};
use tardis_gallifrey::domain::{Change, ChangeType};
use tardis_gallifrey::Gallifrey;
use tardis_telemetry::gallifrey::TelemetryStore;
use tardis_telemetry::types::{Level, Subsystem, TraceId};
use tardis_telemetry::userspace::layer::EventData;

#[tokio::test]
async fn test_doctor_correlates_changes_with_errors() {
    // 1. Setup
    let telemetry = Arc::new(TelemetryStore::new());
    let gallifrey = Arc::new(Gallifrey::new());
    let doctor = SystemDoctor::new(Arc::clone(&telemetry), Arc::clone(&gallifrey));

    // 2. Record a system change (e.g. 2 minutes ago)
    let change_time = Utc::now() - Duration::minutes(2);
    let change = Change {
        timestamp: change_time,
        path: "/etc/tardis/config.toml".to_string(),
        change_type: ChangeType::Update,
        old_value: None,
        new_value: None,
    };
    gallifrey.system_state().record_change(change).unwrap();

    // 3. Record an error (e.g. 1 minute ago)
    let error_event = EventData {
        span_id: None,
        trace_id: TraceId::generate(),
        timestamp: Utc::now() - Duration::minutes(1),
        level: Level::Error,
        message: "Connection failed".to_string(),
        fields: HashMap::new(),
        subsystem: Subsystem::Kernel,
    };
    telemetry.record_event(error_event).await.unwrap();

    // 4. Diagnose
    let diagnosis = doctor.diagnose().await;

    // 5. Verify
    assert_ne!(diagnosis.status, HealthStatus::Healthy);
    assert!(!diagnosis.root_causes.is_empty(), "Should find root causes");

    let cause = &diagnosis.root_causes[0];
    assert!(cause.contains("/etc/tardis/config.toml"));
    assert!(cause.contains("Update"));
}
