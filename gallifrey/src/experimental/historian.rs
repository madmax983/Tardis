//! The Historian: Temporal Anomaly Detector.
//!
//! "History is a burden. Stories can make us fly."
//!
//! This module analyzes the bi-temporal history of entities to detect:
//! - **Retcons**: Updates where Valid Time is significantly earlier than Transaction Time (rewriting the past).
//! - **Prophecies**: Updates where Valid Time is significantly later than Transaction Time (predicting the future).

use chrono::{DateTime, Duration, Utc};
use std::fmt::Write;
use tardis_common::domain::Entity;
use tardis_common::id::EntityId;

/// A detected change to the past.
#[derive(Debug, Clone)]
pub struct Retcon {
    /// The entity ID.
    pub entity_id: EntityId,
    /// When the change was recorded (Transaction Time).
    pub recorded_at: DateTime<Utc>,
    /// The time the change refers to (Valid Time).
    pub valid_at: DateTime<Utc>,
    /// The magnitude of the retcon (Recorded - Valid).
    pub magnitude: Duration,
}

/// A detected prediction of the future.
#[derive(Debug, Clone)]
pub struct Prophecy {
    /// The entity ID.
    pub entity_id: EntityId,
    /// When the prediction was recorded (Transaction Time).
    pub recorded_at: DateTime<Utc>,
    /// The time the prediction refers to (Valid Time).
    pub valid_at: DateTime<Utc>,
    /// The horizon of the prophecy (Valid - Recorded).
    pub horizon: Duration,
}

/// A report of temporal anomalies.
#[derive(Debug, Default, Clone)]
pub struct HistoryReport {
    /// Detected retcons.
    pub retcons: Vec<Retcon>,
    /// Detected prophecies.
    pub prophecies: Vec<Prophecy>,
    /// Total entities analyzed.
    pub total_entities: usize,
}

/// The Historian analyzer.
#[derive(Debug, Default)]
pub struct Historian {
    /// Threshold for detection (to ignore minor clock drift).
    threshold: Duration,
}

impl Historian {
    /// Create a new Historian.
    ///
    /// # Arguments
    ///
    /// * `threshold_ms` - Minimum milliseconds difference to trigger detection.
    #[must_use]
    pub const fn new(threshold_ms: i64) -> Self {
        Self {
            threshold: Duration::milliseconds(threshold_ms),
        }
    }

    /// Analyze a list of entities for temporal anomalies.
    #[must_use]
    pub fn analyze(&self, entities: &[Entity]) -> HistoryReport {
        let mut report = HistoryReport {
            retcons: Vec::new(),
            prophecies: Vec::new(),
            total_entities: entities.len(),
        };

        for entity in entities {
            let vt = entity.temporal.valid_time.start;
            let tt = entity.temporal.transaction_time.start;

            if vt < tt {
                let diff = tt - vt;
                if diff > self.threshold {
                    report.retcons.push(Retcon {
                        entity_id: entity.id,
                        recorded_at: tt,
                        valid_at: vt,
                        magnitude: diff,
                    });
                }
            } else if vt > tt {
                let diff = vt - tt;
                if diff > self.threshold {
                    report.prophecies.push(Prophecy {
                        entity_id: entity.id,
                        recorded_at: tt,
                        valid_at: vt,
                        horizon: diff,
                    });
                }
            }
        }

        // Sort by magnitude (descending)
        report
            .retcons
            .sort_by(|a, b| b.magnitude.cmp(&a.magnitude));
        report
            .prophecies
            .sort_by(|a, b| b.horizon.cmp(&a.horizon));

        report
    }

    /// Generate a markdown summary of the report.
    #[must_use]
    pub fn summary(&self, report: &HistoryReport) -> String {
        let mut out = String::new();
        let _ = writeln!(out, "# Temporal Integrity Report\n");
        let _ = writeln!(out, "Analyzed {} entity versions.\n", report.total_entities);

        if !report.retcons.is_empty() {
            let _ = writeln!(out, "## ⚠️ Retcons Detected (Rewriting History)");
            let _ = writeln!(out, "| Entity ID | Recorded At | Valid At | Magnitude |");
            let _ = writeln!(out, "|-----------|-------------|----------|-----------|");
            for r in &report.retcons {
                let _ = writeln!(
                    out,
                    "| {} | {} | {} | {} |",
                    r.entity_id,
                    r.recorded_at.format("%Y-%m-%d %H:%M:%S"),
                    r.valid_at.format("%Y-%m-%d %H:%M:%S"),
                    format_duration(r.magnitude)
                );
            }
            out.push('\n');
        }

        if !report.prophecies.is_empty() {
            let _ = writeln!(out, "## 🔮 Prophecies Detected (Future Knowledge)");
            let _ = writeln!(out, "| Entity ID | Recorded At | Valid At | Horizon |");
            let _ = writeln!(out, "|-----------|-------------|----------|---------|");
            for p in &report.prophecies {
                let _ = writeln!(
                    out,
                    "| {} | {} | {} | {} |",
                    p.entity_id,
                    p.recorded_at.format("%Y-%m-%d %H:%M:%S"),
                    p.valid_at.format("%Y-%m-%d %H:%M:%S"),
                    format_duration(p.horizon)
                );
            }
            out.push('\n');
        }

        if report.retcons.is_empty() && report.prophecies.is_empty() {
            let _ = writeln!(out, "✅ Timeline is linear. No anomalies detected.");
        }

        out
    }
}

fn format_duration(d: Duration) -> String {
    if d.num_days() > 0 {
        format!("{}d", d.num_days())
    } else if d.num_hours() > 0 {
        format!("{}h", d.num_hours())
    } else if d.num_minutes() > 0 {
        format!("{}m", d.num_minutes())
    } else {
        format!("{}s", d.num_seconds())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tardis_common::temporal::{BiTemporalInterval, TimeRange};

    fn create_entity(
        valid_start: DateTime<Utc>,
        trans_start: DateTime<Utc>,
    ) -> Entity {
        Entity {
            id: EntityId::new(),
            entity_type: "Test".to_string(),
            name: "Test".to_string(),
            properties: HashMap::new(),
            embedding: None,
            temporal: BiTemporalInterval {
                valid_time: TimeRange {
                    start: valid_start,
                    end: None,
                },
                transaction_time: TimeRange {
                    start: trans_start,
                    end: None,
                },
            },
            source: None,
        }
    }

    #[test]
    fn test_historian_analysis() {
        let now = Utc::now();

        // 1. Normal: Valid ~ Transaction
        let normal = create_entity(now, now);

        // 2. Retcon: Valid is 1 hour BEFORE Transaction
        // We record NOW that something happened 1 hour ago.
        let retcon = create_entity(now - Duration::hours(1), now);

        // 3. Prophecy: Valid is 1 hour AFTER Transaction
        // We record NOW that something will happen in 1 hour.
        let prophecy = create_entity(now + Duration::hours(1), now);

        let entities = vec![normal, retcon, prophecy];

        // Threshold of 1 minute
        let historian = Historian::new(60_000);
        let report = historian.analyze(&entities);

        assert_eq!(report.total_entities, 3);
        assert_eq!(report.retcons.len(), 1);
        assert_eq!(report.prophecies.len(), 1);

        assert!(report.retcons[0].magnitude >= Duration::minutes(59));
        assert!(report.prophecies[0].horizon >= Duration::minutes(59));

        println!("{}", historian.summary(&report));
    }
}
