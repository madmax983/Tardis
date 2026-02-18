//! The Weaver 🕸️
//!
//! "Everything is connected."
//!
//! The Weaver finds narrative threads between entities in the knowledge graph.
//! It uses `Gallifrey` to retrieve histories and `Vortex` to generate
//! stories about how two entities might be related.

use std::fmt::Write;
use std::sync::Arc;
use tardis_common::id::EntityId;
use tardis_gallifrey::Gallifrey;
use tardis_vortex::{InferenceParams, ModelHandle, Vortex};

/// The Weaver engine.
#[derive(Debug)]
pub struct Weaver {
    gallifrey: Arc<Gallifrey>,
    vortex: Arc<Vortex>,
    model: Option<ModelHandle>,
}

impl Weaver {
    /// Create a new Weaver.
    #[must_use]
    pub const fn new(
        gallifrey: Arc<Gallifrey>,
        vortex: Arc<Vortex>,
        model: Option<ModelHandle>,
    ) -> Self {
        Self {
            gallifrey,
            vortex,
            model,
        }
    }

    /// Weave a narrative thread between two entities.
    ///
    /// # Errors
    ///
    /// Returns an error if either entity is not found or if the narrative generation fails.
    pub async fn weave(&self, name_a: &str, name_b: &str) -> anyhow::Result<String> {
        let id_a = self
            .find_entity(name_a)
            .ok_or_else(|| anyhow::anyhow!("Entity '{name_a}' not found"))?;
        let id_b = self
            .find_entity(name_b)
            .ok_or_else(|| anyhow::anyhow!("Entity '{name_b}' not found"))?;

        let history_a = self.gallifrey.get_history(id_a).await?;
        let history_b = self.gallifrey.get_history(id_b).await?;

        if history_a.is_empty() {
            return Err(anyhow::anyhow!("No history found for entity '{name_a}'"));
        }
        if history_b.is_empty() {
            return Err(anyhow::anyhow!("No history found for entity '{name_b}'"));
        }

        // Construct the prompt
        let mut prompt = String::new();
        writeln!(
            &mut prompt,
            "You are a master storyteller. Find a connection between two entities based on their histories and write a short narrative weaving them together."
        )?;
        writeln!(&mut prompt, "\nEntity A: {name_a}")?;
        for version in &history_a {
            let time = version.temporal.valid_time.start.format("%Y-%m-%d");
            writeln!(
                &mut prompt,
                " - [{time}] {} ({:?})",
                version.name, version.properties
            )?;
        }

        writeln!(&mut prompt, "\nEntity B: {name_b}")?;
        for version in &history_b {
            let time = version.temporal.valid_time.start.format("%Y-%m-%d");
            writeln!(
                &mut prompt,
                " - [{time}] {} ({:?})",
                version.name, version.properties
            )?;
        }

        writeln!(&mut prompt, "\nNarrative Connection:")?;

        // Generate narrative
        if let Some(handle) = self.model {
            let params = InferenceParams {
                max_tokens: 800,
                temperature: 0.8, // Slightly higher creativity for weaving
                ..Default::default()
            };

            let story = self.vortex.infer(handle, &prompt, params).await?;
            Ok(story)
        } else {
            Ok(format!(
                "(No model loaded. Prompt would be:)\n\n{prompt}\n\n[Imagine a story connecting {name_a} and {name_b}]"
            ))
        }
    }

    /// Find an entity ID by name (case-insensitive).
    fn find_entity(&self, name: &str) -> Option<EntityId> {
        let mut target_id = None;
        // Use scan_history to find the entity.
        // Note: This is inefficient for large DBs but fine for experimental features.
        let _ = self.gallifrey.knowledge().scan_history(|history| {
            if target_id.is_some() {
                return;
            }
            if history.iter().any(|e| e.name.eq_ignore_ascii_case(name)) {
                if let Some(first) = history.first() {
                    target_id = Some(first.id);
                }
            }
        });
        target_id
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tardis_common::id::EntityId;
    use tardis_common::temporal::BiTemporalInterval;
    use tardis_gallifrey::domain::Entity;

    #[tokio::test]
    async fn test_weaver_connection() {
        let gallifrey = Arc::new(Gallifrey::new());
        let vortex = Arc::new(Vortex::new().unwrap());

        // Mock inference
        vortex.set_mock_inference(Box::new(|_, prompt, _| {
            Ok(format!(
                "Mock story connecting entities. Prompt length: {}",
                prompt.len()
            ))
        }));

        // Insert Entity A
        let entity_a = Entity {
            id: EntityId::new(),
            entity_type: "Person".to_string(),
            name: "Alice".to_string(),
            properties: HashMap::from([("job".to_string(), serde_json::json!("Engineer"))]),
            embedding: None,
            temporal: BiTemporalInterval::now(),
            source: None,
        };
        gallifrey.insert(entity_a.clone()).await.unwrap();

        // Insert Entity B
        let entity_b = Entity {
            id: EntityId::new(),
            entity_type: "Project".to_string(),
            name: "Tardis".to_string(),
            properties: HashMap::from([("language".to_string(), serde_json::json!("Rust"))]),
            embedding: None,
            temporal: BiTemporalInterval::now(),
            source: None,
        };
        gallifrey.insert(entity_b.clone()).await.unwrap();

        // Create Weaver (no model loaded, so it falls back unless we mock load_model)
        // Let's mock load_model so we can test the inference path
        let mock_handle = ModelHandle::new(1);
        vortex.set_mock_load_model(Box::new(move |_, _| Ok(mock_handle)));

        let handle = vortex
            .load_model("dummy", Default::default())
            .await
            .unwrap();

        let weaver = Weaver::new(gallifrey, vortex, Some(handle));

        let story = weaver.weave("Alice", "Tardis").await.unwrap();

        assert!(story.contains("Mock story"));
        assert!(story.contains("Prompt length"));
    }

    #[tokio::test]
    async fn test_weaver_missing_entity() {
        let gallifrey = Arc::new(Gallifrey::new());
        let vortex = Arc::new(Vortex::new().unwrap());

        let weaver = Weaver::new(gallifrey, vortex, None);

        let result = weaver.weave("Ghost", "Shadow").await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Entity 'Ghost' not found"));
    }
}
