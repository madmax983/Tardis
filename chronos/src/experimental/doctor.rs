//! System Doctor module.
//!
//! The System Doctor analyzes telemetry data to diagnose system health.
//! It uses the `TelemetryStore` to retrieve recent spans and events,
//! calculates metrics, and generates a diagnosis.

use chrono::Utc;
use std::fmt::Write;
use std::sync::Arc;
use std::time::Duration;
use tardis_telemetry::gallifrey::TelemetryStore;
use tardis_telemetry::types::Level;

/// The System Doctor.
#[derive(Debug, Clone)]
pub struct SystemDoctor {
    store: Arc<TelemetryStore>,
}

impl SystemDoctor {
    /// Create a new System Doctor.
    #[must_use]
    pub const fn new(store: Arc<TelemetryStore>) -> Self {
        Self { store }
    }

    /// Diagnose the system health over the specified time window.
    ///
    /// # Errors
    ///
    /// Returns an error if the diagnosis fails (e.g. store access error).
    /// Currently infallible but async for future LLM integration.
    #[allow(clippy::unused_async)]
    pub async fn diagnose(&self, window: Duration) -> String {
        let now = Utc::now();
        let window_duration =
            chrono::Duration::from_std(window).unwrap_or_else(|_| chrono::Duration::minutes(5));
        let window_ms = window_duration.num_milliseconds();

        // Get context around now
        let context = self.store.context_around(now, window_ms);

        // Analyze context
        let mut error_count = 0;
        let mut warning_count = 0;
        let mut spans_count = 0;
        let mut total_duration_ms = 0;

        for span in &context.spans {
            spans_count += 1;
            match span.data.level {
                Level::Error => error_count += 1,
                Level::Warn => warning_count += 1,
                _ => {}
            }

            if let Some(end) = span.data.end_time {
                let duration = (end - span.data.start_time).num_milliseconds();
                total_duration_ms += duration;
            }
        }

        let event_count = context.events.len();
        let avg_latency = if spans_count > 0 {
            #[allow(clippy::cast_precision_loss)]
            {
                total_duration_ms as f64 / f64::from(spans_count)
            }
        } else {
            0.0
        };

        // Construct report
        let mut report = String::new();
        writeln!(report, "🏥 SYSTEM DOCTOR DIAGNOSIS 🏥").ok();
        writeln!(report, "=============================").ok();
        writeln!(report, "Time Window: Last {} seconds", window.as_secs()).ok();
        writeln!(report, "Spans Analyzed:  {spans_count}").ok();
        writeln!(report, "Events Analyzed: {event_count}").ok();
        writeln!(report, "Errors:          {error_count}").ok();
        writeln!(report, "Warnings:        {warning_count}").ok();
        writeln!(report, "Avg Latency:     {avg_latency:.2}ms").ok();
        report.push('\n');

        if error_count == 0 && warning_count == 0 {
            writeln!(report, "✅ STATUS: HEALTHY").ok();
            report.push_str(
                "System is operating within normal parameters. No significant anomalies detected.",
            );
        } else {
            if error_count > 0 {
                writeln!(report, "❌ STATUS: CRITICAL").ok();
            } else {
                writeln!(report, "⚠️ STATUS: DEGRADED").ok();
            }

            writeln!(report, "Diagnosis:").ok();
            if error_count > 0 {
                writeln!(
                    report,
                    "- High error rate detected ({error_count} errors). Investigate logs immediately."
                )
                .ok();
            }
            if warning_count > 5 {
                writeln!(
                    report,
                    "- Elevated warning count ({warning_count} warnings). Potential instability."
                )
                .ok();
            }
            if avg_latency > 1000.0 {
                writeln!(
                    report,
                    "- System latency is high (>1s). Performance degradation detected."
                )
                .ok();
            }
        }

        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tardis_telemetry::types::{SpanId, Subsystem, TraceId};
    use tardis_telemetry::userspace::layer::SpanData;

    #[tokio::test]
    async fn test_healthy_diagnosis() {
        let store = Arc::new(TelemetryStore::new());
        let doctor = SystemDoctor::new(store);

        let report = doctor.diagnose(Duration::from_secs(60)).await;
        assert!(report.contains("✅ STATUS: HEALTHY"));
    }

    #[tokio::test]
    async fn test_critical_diagnosis() {
        let store = Arc::new(TelemetryStore::new());

        // Inject an error span
        let span = SpanData {
            trace_id: TraceId::generate(),
            span_id: SpanId::generate(),
            parent_id: None,
            name: "failed_op".to_string(),
            subsystem: Subsystem::Kernel,
            start_instant: std::time::Instant::now(),
            start_time: Utc::now(),
            end_time: Some(Utc::now()),
            attributes: HashMap::new(),
            level: Level::Error,
        };

        store.record_span(span).await.unwrap();

        let doctor = SystemDoctor::new(store);
        let report = doctor.diagnose(Duration::from_secs(60)).await;

        assert!(report.contains("❌ STATUS: CRITICAL"));
        assert!(report.contains("High error rate detected"));
    }
}
