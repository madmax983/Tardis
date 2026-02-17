//! # The Historian
//!
//! A module that analyzes the bi-temporal history of an entity to tell the story of "what we thought we knew" vs "what actually happened".
//! It detects "Retcons" (corrections made after the fact) and "Prophecies" (events recorded before they happened)
//! and uses Vortex to generate a natural language narrative.

#[cfg(feature = "nova")]
use crate::ChronosResult;
#[cfg(feature = "nova")]
use chrono::{DateTime, Utc};
#[cfg(feature = "nova")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "nova")]
use std::sync::Arc;
#[cfg(feature = "nova")]
use tardis_common::id::EntityId;
#[cfg(feature = "nova")]
use tardis_gallifrey::Gallifrey;
#[cfg(feature = "nova")]
use tardis_vortex::Vortex;

#[cfg(feature = "nova")]
/// A detected anomaly in the timeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TemporalAnomaly {
    /// Information was recorded after it happened (Retroactive Continuity).
    Retcon {
        /// When it was recorded.
        recorded: DateTime<Utc>,
        /// When it actually happened.
        actual: DateTime<Utc>,
        /// The delay in seconds.
        delay_seconds: i64,
    },
    /// Information was recorded before it happened.
    Prophecy {
        /// When it was recorded.
        recorded: DateTime<Utc>,
        /// When it is predicted to happen.
        predicted: DateTime<Utc>,
        /// The lead time in seconds.
        lead_seconds: i64,
    },
    /// Standard operation.
    Normal,
}

#[cfg(feature = "nova")]
/// A segment of history for analysis.
#[derive(Debug, Serialize)]
struct HistorySegment {
    /// The state of the entity.
    entity_state: String, // JSON string of properties
    /// The transaction time (when we knew it).
    recorded_at: DateTime<Utc>,
    /// The valid time (when it was true).
    valid_from: DateTime<Utc>,
    /// detected anomaly
    anomaly: TemporalAnomaly,
}

#[cfg(feature = "nova")]
/// The Historian service.
#[derive(Debug)]
pub struct Historian {
    vortex: Arc<Vortex>,
    gallifrey: Arc<Gallifrey>,
}

#[cfg(feature = "nova")]
impl Historian {
    /// Create a new `Historian`.
    #[must_use]
    pub const fn new(vortex: Arc<Vortex>, gallifrey: Arc<Gallifrey>) -> Self {
        Self { vortex, gallifrey }
    }

    /// Analyze the history of an entity and return structured segments.
    async fn analyze_history(&self, id: EntityId) -> ChronosResult<Vec<HistorySegment>> {
        let history = self.gallifrey.get_history(id).await?;
        let mut segments = Vec::new();

        for entity in history {
            let recorded = entity.temporal.transaction_time.start;
            let actual = entity.temporal.valid_time.start;
            let diff = (recorded - actual).num_seconds();

            let anomaly = if diff > 5 {
                TemporalAnomaly::Retcon {
                    recorded,
                    actual,
                    delay_seconds: diff,
                }
            } else if diff < -5 {
                TemporalAnomaly::Prophecy {
                    recorded,
                    predicted: actual,
                    lead_seconds: -diff,
                }
            } else {
                TemporalAnomaly::Normal
            };

            segments.push(HistorySegment {
                entity_state: serde_json::to_string(&entity.properties).unwrap_or_default(),
                recorded_at: recorded,
                valid_from: actual,
                anomaly,
            });
        }

        // Sort by transaction time (the order we learned things)
        segments.sort_by(|a, b| a.recorded_at.cmp(&b.recorded_at));

        Ok(segments)
    }

    /// Generate a narrative about the entity's history.
    ///
    /// # Errors
    ///
    /// Returns an error if history cannot be retrieved or LLM inference fails.
    pub async fn tell_story(&self, id: EntityId) -> ChronosResult<String> {
        let segments = self.analyze_history(id).await?;
        if segments.is_empty() {
            return Ok("No history found for this entity.".to_string());
        }

        let segments_json = serde_json::to_string_pretty(&segments).map_err(|e| {
            crate::error::ChronosError::Common(tardis_common::Error::Serialization(e))
        })?;

        let prompt = format!(
            "You are a Temporal Historian. Analyze the following audit log. \
            Identify 'Retcons' (where we learned something late) and 'Prophecies' (where we predicted the future). \
            Write a short, engaging narrative explaining how our knowledge of this entity evolved over time.\n\n\
            Log:\n{segments_json}"
        );

        let handle = self
            .vortex
            .load_model("default", tardis_vortex::ModelLoadConfig::default())
            .await?;

        let story = self
            .vortex
            .infer(handle, &prompt, tardis_vortex::InferenceParams::default())
            .await?;

        Ok(story)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tardis_common::id::{EntityId, ModelHandle};
    use tardis_common::temporal::{BiTemporalInterval, TimeRange};
    use tardis_gallifrey::domain::Entity;

    #[tokio::test]
    async fn test_historian_narrative() {
        let vortex = Arc::new(Vortex::new().unwrap());
        vortex.set_mock_inference(Box::new(|_, prompt, _| {
            if prompt.contains("Retcon") && prompt.contains("delay_seconds") {
                return Ok("Narrative: We realized late that the system was offline.".to_string());
            }
            Ok("Default narrative".to_string())
        }));
        vortex.set_mock_load_model(Box::new(|_, _| Ok(ModelHandle::new(0))));

        let gallifrey = Arc::new(Gallifrey::new());
        let id = EntityId::new();
        setup_history(&gallifrey, id).await;

        let historian = Historian::new(vortex, gallifrey);

        let story = historian.tell_story(id).await.unwrap();
        assert!(story.contains("Narrative"));
    }

    // Helper to setup history
    async fn setup_history(gallifrey: &Gallifrey, id: EntityId) {
        let now = Utc::now();
        let ten_mins_ago = now - chrono::Duration::seconds(600);
        let five_mins_ago = now - chrono::Duration::seconds(300);

        let e1 = Entity {
            id,
            entity_type: "Test".to_string(),
            name: "Test Entity".to_string(),
            properties: std::collections::HashMap::new(),
            embedding: None,
            temporal: BiTemporalInterval {
                valid_time: TimeRange {
                    start: ten_mins_ago,
                    end: None,
                },
                transaction_time: TimeRange {
                    start: ten_mins_ago,
                    end: Some(now),
                },
            },
            source: None,
        };

        let e2 = Entity {
            id,
            entity_type: "Test".to_string(),
            name: "Test Entity".to_string(),
            properties: std::collections::HashMap::new(),
            embedding: None,
            temporal: BiTemporalInterval {
                valid_time: TimeRange {
                    start: five_mins_ago,
                    end: None,
                },
                transaction_time: TimeRange {
                    start: now,
                    end: None,
                },
            },
            source: None,
        };

        gallifrey.knowledge().insert_entity(e1).unwrap();
        gallifrey.knowledge().insert_entity(e2).unwrap();
    }

    #[tokio::test]
    async fn test_analyze_logic() {
        let vortex = Arc::new(Vortex::new().unwrap());
        let gallifrey = Arc::new(Gallifrey::new());
        let id = EntityId::new();
        setup_history(&gallifrey, id).await;

        let historian = Historian::new(vortex, gallifrey);

        let segments = historian.analyze_history(id).await.unwrap();
        assert_eq!(segments.len(), 2);

        match segments[0].anomaly {
            TemporalAnomaly::Normal => {}
            _ => panic!("Expected Normal, got {:?}", segments[0].anomaly),
        }

        match segments[1].anomaly {
            TemporalAnomaly::Retcon { delay_seconds, .. } => {
                assert!(delay_seconds >= 299);
            }
            _ => panic!("Expected Retcon, got {:?}", segments[1].anomaly),
        }
    }
}
