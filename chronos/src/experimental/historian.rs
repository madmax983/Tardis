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
use tardis_common::traits::{GallifreyService, VortexService};

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
    vortex: Arc<dyn VortexService>,
    gallifrey: Arc<dyn GallifreyService>,
}

#[cfg(feature = "nova")]
impl Historian {
    /// Create a new `Historian`.
    #[must_use]
    pub fn new(vortex: Arc<dyn VortexService>, gallifrey: Arc<dyn GallifreyService>) -> Self {
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
            .load_model("default", tardis_common::llm::ModelLoadConfig::default())
            .await?;

        let story = self
            .vortex
            .infer(
                handle,
                &prompt,
                tardis_common::llm::InferenceParams::default(),
            )
            .await?;

        Ok(story)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use tardis_common::domain::{Change, Entity, Message, Snapshot};
    use tardis_common::id::{EntityId, ModelHandle, SessionId};
    use tardis_common::llm::{InferenceParams, ModelLoadConfig};
    use tardis_common::temporal::{BiTemporalInterval, TemporalQuery, TimeRange};
    use tardis_common::traits::QueryResult;
    use tardis_common::Result;

    #[derive(Debug)]
    struct MockVortex;

    #[async_trait]
    impl VortexService for MockVortex {
        async fn load_model(&self, _path: &str, _config: ModelLoadConfig) -> Result<ModelHandle> {
            Ok(ModelHandle::new(0))
        }
        async fn unload_model(&self, _handle: ModelHandle) -> Result<()> {
            Ok(())
        }
        async fn infer(
            &self,
            _handle: ModelHandle,
            prompt: &str,
            _params: InferenceParams,
        ) -> Result<String> {
            // Check if prompt contains the retcon detection
            if prompt.contains("Retcon") && prompt.contains("delay_seconds") {
                return Ok("Narrative: We realized late that the system was offline.".to_string());
            }
            Ok("Default narrative".to_string())
        }
        async fn embed(&self, _handle: ModelHandle, _text: &str) -> Result<Vec<f32>> {
            Ok(vec![])
        }
    }

    #[derive(Debug)]
    struct MockGallifrey;

    #[async_trait]
    impl GallifreyService for MockGallifrey {
        async fn insert(&self, _entity: Entity) -> Result<EntityId> {
            Ok(EntityId::new())
        }
        async fn get_history(&self, _id: EntityId) -> Result<Vec<Entity>> {
            let now = Utc::now();
            let ten_mins_ago = now - chrono::Duration::seconds(600);
            let five_mins_ago = now - chrono::Duration::seconds(300);

            // Record 1: Normal (Trans=10 mins ago, Valid=10 mins ago)
            let e1 = Entity {
                id: EntityId::new(),
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
                        end: None,
                    },
                },
                source: None,
            };

            // Record 2: Retcon (Trans=Now, Valid=5 mins ago) -> Trans > Valid by 300s
            // Wait, Retcon definition: Recorded > Actual + 5s.
            // Trans=Now, Valid=5 mins ago. Diff = 300s. Correct.
            let e2 = Entity {
                id: EntityId::new(),
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

            Ok(vec![e1, e2])
        }
        async fn search_knowledge(&self, _embedding: &[f32], _limit: usize) -> Result<Vec<Entity>> {
            Ok(vec![])
        }
        async fn query(&self, _query: &str, _temporal: TemporalQuery) -> Result<QueryResult> {
            Ok(QueryResult {
                nodes: vec![],
                execution_time_ms: 0,
                truncated: false,
            })
        }
        async fn get_recent_messages(&self, _id: SessionId, _limit: usize) -> Result<Vec<Message>> {
            Ok(vec![])
        }
        async fn search_conversation(
            &self,
            _embedding: &[f32],
            _limit: usize,
        ) -> Result<Vec<Message>> {
            Ok(vec![])
        }
        async fn find_snapshot(&self, _ts: DateTime<Utc>) -> Result<Option<Snapshot>> {
            Ok(None)
        }
        async fn get_snapshot_history(&self, _limit: usize) -> Result<Vec<Snapshot>> {
            Ok(vec![])
        }
        async fn record_change(&self, _change: Change) -> Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_historian_narrative() {
        let vortex = Arc::new(MockVortex);
        let gallifrey = Arc::new(MockGallifrey);
        let historian = Historian::new(vortex, gallifrey);

        let story = historian.tell_story(EntityId::new()).await.unwrap();
        assert!(story.contains("Narrative"));
    }

    #[tokio::test]
    async fn test_analyze_logic() {
        let vortex = Arc::new(MockVortex);
        let gallifrey = Arc::new(MockGallifrey);
        let historian = Historian::new(vortex, gallifrey);

        let segments = historian.analyze_history(EntityId::new()).await.unwrap();
        assert_eq!(segments.len(), 2);

        // First segment (sorted by transaction time, which was 10 mins ago)
        match segments[0].anomaly {
            TemporalAnomaly::Normal => {}
            _ => panic!("Expected Normal, got {:?}", segments[0].anomaly),
        }

        // Second segment (Trans=Now, Valid=5 mins ago -> Retcon)
        match segments[1].anomaly {
            TemporalAnomaly::Retcon { delay_seconds, .. } => {
                assert!(delay_seconds >= 299);
            }
            _ => panic!("Expected Retcon, got {:?}", segments[1].anomaly),
        }
    }
}
