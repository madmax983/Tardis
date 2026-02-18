//! The Weaver
//!
//! "It takes the threads of time and knots them together."
//!
//! A proactive relationship discovery engine that scans the Knowledge Graph
//! for potential connections between entities and uses Vortex (LLM) to verify them.

#![cfg(feature = "nova")]

use crate::error::{ChronosError, ChronosResult};
use crate::experimental::psychic_paper::{Intent, PsychicPaper};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tardis_common::id::ModelHandle;
use tardis_gallifrey::domain::Entity;
use tardis_gallifrey::Gallifrey;
use tardis_vortex::{InferenceParams, Vortex};
use tracing::warn;

/// A suggested relationship found by the Weaver.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestedRelationship {
    /// The source entity.
    pub source: Entity,
    /// The target entity.
    pub target: Entity,
    /// The suggested relationship type.
    pub relationship_type: String,
    /// A description of why this relationship exists.
    pub description: String,
    /// Confidence score (0.0 - 1.0).
    pub confidence: f32,
}

/// The Weaver engine.
#[derive(Debug)]
pub struct Weaver {
    vortex: Arc<Vortex>,
    gallifrey: Arc<Gallifrey>,
    paper: PsychicPaper,
    model_handle: Option<ModelHandle>,
}

impl Weaver {
    /// Create a new Weaver.
    #[must_use]
    pub const fn new(
        vortex: Arc<Vortex>,
        gallifrey: Arc<Gallifrey>,
        model_handle: Option<ModelHandle>,
    ) -> Self {
        Self {
            vortex,
            gallifrey,
            paper: PsychicPaper::new(),
            model_handle,
        }
    }

    /// Weave potential connections for a given entity (or a random one).
    ///
    /// If `entity_name` is provided, it focuses on that entity.
    /// Otherwise, it picks an entity that might need connections.
    ///
    /// # Errors
    ///
    /// Returns an error if the knowledge graph cannot be accessed or inference fails.
    pub async fn weave(
        &self,
        entity_name: Option<&str>,
    ) -> ChronosResult<Vec<SuggestedRelationship>> {
        let model = self.model_handle.ok_or_else(|| {
            ChronosError::Common(tardis_common::Error::Internal(
                "No model loaded for Weaver".to_string(),
            ))
        })?;

        // 1. Find the anchor entity
        let anchor = if let Some(name) = entity_name {
            self.find_entity_by_name(name).await?
        } else {
            self.find_random_entity().await?
        };

        let Some(anchor) = anchor else {
            return Ok(Vec::new());
        };

        // 2. Find candidates using semantic search (if embedding exists)
        let candidates = if let Some(embedding) = &anchor.embedding {
            self.gallifrey
                .search_knowledge(embedding, 5)
                .await
                .map_err(|e| ChronosError::Common(tardis_common::Error::Internal(e.to_string())))?
        } else {
            // Fallback: Entity has no embedding.
            warn!("Entity '{}' has no embedding, skipping weave.", anchor.name);
            return Ok(Vec::new());
        };

        // 3. Filter candidates and evaluate
        let mut suggestions = Vec::new();

        for candidate in candidates {
            if candidate.id == anchor.id {
                continue;
            }

            // Ask Vortex
            if let Some(suggestion) = self.evaluate_pair(&anchor, &candidate, model).await? {
                suggestions.push(suggestion);
            }
        }

        Ok(suggestions)
    }

    async fn find_entity_by_name(&self, name: &str) -> ChronosResult<Option<Entity>> {
        let mut target = None;
        // Linear scan via scan_history
        self.gallifrey
            .knowledge()
            .scan_history(|history| {
                if target.is_some() {
                    return;
                }
                if let Some(entity) = history.iter().find(|e| e.name.eq_ignore_ascii_case(name)) {
                    target = Some(entity.clone());
                }
            })
            .map_err(|e| ChronosError::Common(tardis_common::Error::Internal(e.to_string())))?;

        Ok(target)
    }

    async fn find_random_entity(&self) -> ChronosResult<Option<Entity>> {
        let mut found = None;
        // Just take the last one found in a scan (effectively "random" / latest)
        self.gallifrey
            .knowledge()
            .scan_history(|history| {
                if found.is_none() {
                    if let Some(e) = history.last() {
                        found = Some(e.clone());
                    }
                }
            })
            .map_err(|e| ChronosError::Common(tardis_common::Error::Internal(e.to_string())))?;
        Ok(found)
    }

    async fn evaluate_pair(
        &self,
        a: &Entity,
        b: &Entity,
        model: ModelHandle,
    ) -> ChronosResult<Option<SuggestedRelationship>> {
        let prompt = format!(
            "I am analyzing the relationship between two entities in a knowledge graph.\n\
            Entity A: {} ({})\n\
            Properties: {}\n\n\
            Entity B: {} ({})\n\
            Properties: {}\n\n\
            Is there a likely semantic or functional relationship between them? \
            If yes, provide the relationship type (e.g., 'OWNS', 'LOCATED_IN', 'PART_OF', 'RELATED_TO', 'FRIEND_OF') and a short description.\n\
            If no, or if the relationship is weak, return null.\n\n\
            Format: JSON\n\
            {{\n  \"relationship\": \"TYPE\",\n  \"description\": \"Short explanation\",\n  \"confidence\": 0.8\n}}",
            a.name, a.entity_type, serde_json::to_string(&a.properties).unwrap_or_default(),
            b.name, b.entity_type, serde_json::to_string(&b.properties).unwrap_or_default()
        );

        let response = self
            .vortex
            .infer(
                model,
                &prompt,
                InferenceParams {
                    max_tokens: 500,
                    temperature: 0.3,
                    ..Default::default()
                },
            )
            .await
            .map_err(|e| ChronosError::Common(tardis_common::Error::Internal(e.to_string())))?;

        let parsed = self
            .paper
            .interpret(&response, Intent::Json)
            .map_err(|e| ChronosError::Common(tardis_common::Error::Internal(e)))?;

        if parsed.is_null() {
            return Ok(None);
        }

        if let Some(obj) = parsed.as_object() {
            if let (Some(rel_type), Some(desc), Some(conf)) = (
                obj.get("relationship").and_then(Value::as_str),
                obj.get("description").and_then(Value::as_str),
                obj.get("confidence").and_then(Value::as_f64),
            ) {
                // Heuristic filter
                if conf < 0.5 {
                    return Ok(None);
                }

                return Ok(Some(SuggestedRelationship {
                    source: a.clone(),
                    target: b.clone(),
                    relationship_type: rel_type.to_uppercase(),
                    description: desc.to_string(),
                    confidence: conf as f32,
                }));
            }
        }

        Ok(None)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tardis_common::id::EntityId;
    use tardis_common::temporal::BiTemporalInterval;

    #[tokio::test]
    async fn test_weaver() {
        let gallifrey = Arc::new(Gallifrey::new());
        let vortex = Arc::new(Vortex::new().unwrap());

        // Mock inference
        vortex.set_mock_inference(Box::new(|_, _, _| {
            Ok(serde_json::json!({
                "relationship": "RELATED_TO",
                "description": "Mock description",
                "confidence": 0.9
            }).to_string())
        }));

        // Insert some entities
        let mut props = HashMap::new();
        props.insert("type".to_string(), serde_json::json!("Test"));

        let e1 = Entity {
            id: EntityId::new(),
            entity_type: "Test".to_string(),
            name: "Entity A".to_string(),
            properties: props.clone(),
            embedding: Some(vec![1.0, 0.0, 0.0]),
            temporal: BiTemporalInterval::now(),
            source: None,
        };
        gallifrey.insert(e1).await.unwrap();

        let e2 = Entity {
            id: EntityId::new(),
            entity_type: "Test".to_string(),
            name: "Entity B".to_string(),
            properties: props,
            embedding: Some(vec![0.9, 0.1, 0.0]),
            temporal: BiTemporalInterval::now(),
            source: None,
        };
        gallifrey.insert(e2).await.unwrap();

        // Mock model handle
        let handle = ModelHandle::new(1);

        let weaver = Weaver::new(vortex, gallifrey, Some(handle));

        // Weave!
        let results = weaver.weave(Some("Entity A")).await.unwrap();

        // Note: Semantic search mock in KnowledgeStore is a TODO stub that returns *all* entities with embedding
        // So it should find Entity B (and Entity A, but filter excludes self).

        assert!(!results.is_empty());
        let result = &results[0];
        assert_eq!(result.source.name, "Entity A");
        assert_eq!(result.target.name, "Entity B");
        assert_eq!(result.relationship_type, "RELATED_TO");
    }
}
