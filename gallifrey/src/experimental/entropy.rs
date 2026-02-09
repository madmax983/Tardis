use tardis_common::domain::Entity;

/// Metrics representing the "Temporal Drift" of the system.
///
/// Tracks the deviation between when facts were true (Valid Time)
/// and when they were recorded (Transaction Time).
#[derive(Debug, Clone, Default)]
pub struct SystemEntropy {
    /// Total drift in milliseconds (absolute difference).
    pub total_drift_ms: i64,
    /// Average drift in milliseconds.
    pub average_drift_ms: f64,
    /// Number of "Retcons" (Valid Time < Transaction Time).
    pub retcon_count: usize,
    /// Number of "Prophecies" (Valid Time > Transaction Time).
    pub prophecy_count: usize,
    /// Number of perfectly synchronized updates.
    pub sync_count: usize,
    /// A normalized stability score (0.0 = Chaotic, 1.0 = Stable).
    pub stability_score: f64,
}

impl SystemEntropy {
    /// Render the entropy as an ASCII gauge.
    ///
    /// format: `[<<<<|====|>>>>]`
    /// - `<`: Retcons (Past)
    /// - `=`: Sync (Present)
    /// - `>`: Prophecies (Future)
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    #[allow(clippy::cast_possible_truncation)]
    #[allow(clippy::cast_sign_loss)]
    pub fn render_ascii(&self) -> String {
        let total = self.retcon_count + self.prophecy_count + self.sync_count;
        if total == 0 {
            return "[No Data]".to_string();
        }

        let width: usize = 20;
        let retcon_share = (self.retcon_count as f64 / total as f64 * width as f64).round() as usize;
        let prophecy_share = (self.prophecy_count as f64 / total as f64 * width as f64).round() as usize;
        let sync_share = width.saturating_sub(retcon_share + prophecy_share);

        format!(
            "[{}{}{}] Drift: {:.0}ms (Stab: {:.2})",
            "<".repeat(retcon_share),
            "=".repeat(sync_share),
            ">".repeat(prophecy_share),
            self.average_drift_ms,
            self.stability_score
        )
    }
}

/// A gauge to measure the temporal entropy of a system.
#[derive(Debug, Default)]
pub struct EntropyGauge;

impl EntropyGauge {
    /// Calculate the entropy of a given history.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn calculate(&self, history: &[Entity]) -> SystemEntropy {
        if history.is_empty() {
            return SystemEntropy::default();
        }

        let mut total_drift = 0;
        let mut retcons = 0;
        let mut prophecies = 0;
        let mut syncs = 0;

        for entity in history {
            let valid = entity.temporal.valid_time.start;
            let transaction = entity.temporal.transaction_time.start;

            let diff = (valid - transaction).num_milliseconds();
            total_drift += diff.abs();

            // Consider a small epsilon for sync if needed, but strict equality is fine for now
            match diff.cmp(&0) {
                std::cmp::Ordering::Less => retcons += 1,
                std::cmp::Ordering::Greater => prophecies += 1,
                std::cmp::Ordering::Equal => syncs += 1,
            }
        }

        let count = history.len() as f64;
        let avg_drift = total_drift as f64 / count;

        // Stability score: 1.0 / (1.0 + log10(avg_drift + 1))
        // Drift of 0 -> 1.0
        // Drift of 1000ms -> ~0.25
        let stability = if avg_drift <= 0.0 {
            1.0
        } else {
            1.0 / (1.0 + (avg_drift + 1.0).log10())
        };

        SystemEntropy {
            total_drift_ms: total_drift,
            average_drift_ms: avg_drift,
            retcon_count: retcons,
            prophecy_count: prophecies,
            sync_count: syncs,
            stability_score: stability,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tardis_common::id::EntityId;
    use tardis_common::temporal::{BiTemporalInterval, TimeRange};
    use chrono::{DateTime, Utc};

    fn create_entity(valid_start: DateTime<Utc>, trans_start: DateTime<Utc>) -> Entity {
        Entity {
            id: EntityId::new(),
            entity_type: "Test".to_string(),
            name: "Test".to_string(),
            properties: HashMap::new(),
            embedding: None,
            temporal: BiTemporalInterval {
                valid_time: TimeRange { start: valid_start, end: None },
                transaction_time: TimeRange { start: trans_start, end: None },
            },
            source: None,
        }
    }

    #[test]
    fn test_entropy_calculation() {
        let now = Utc::now();
        let hour = chrono::Duration::hours(1);

        let e1 = create_entity(now, now); // Sync
        let e2 = create_entity(now - hour, now); // Retcon (valid before transaction)
        let e3 = create_entity(now + hour, now); // Prophecy (valid after transaction)

        let history = vec![e1, e2, e3];
        let gauge = EntropyGauge;
        let entropy = gauge.calculate(&history);

        assert_eq!(entropy.sync_count, 1);
        assert_eq!(entropy.retcon_count, 1);
        assert_eq!(entropy.prophecy_count, 1);
        assert!(entropy.average_drift_ms > 0.0);

        println!("{}", entropy.render_ascii());
    }

    #[test]
    fn test_empty_history() {
        let gauge = EntropyGauge;
        let entropy = gauge.calculate(&[]);
        assert_eq!(entropy.total_drift_ms, 0);
        assert_eq!(entropy.render_ascii(), "[No Data]");
    }
}
