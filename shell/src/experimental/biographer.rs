//! `Biographer` 📜
//!
//! A narrative generator for the bi-temporal knowledge graph.
//!
//! It retrieves the full history of an entity and uses the Vortex LLM
//! to generate a human-readable biography, explaining how the entity
//! changed over time and highlighting any retrospective corrections.

use anyhow::{anyhow, Result};
use std::fmt::Write;
use std::sync::Arc;
use tardis_common::id::EntityId;
use tardis_gallifrey::domain::Entity;
use tardis_gallifrey::Gallifrey;
use tardis_vortex::{InferenceParams, ModelHandle, Vortex};

/// The `Biographer` generator.
#[derive(Debug)]
pub struct Biographer {
    gallifrey: Arc<Gallifrey>,
    vortex: Arc<Vortex>,
    model: Option<ModelHandle>,
}

impl Biographer {
    /// Create a new `Biographer`.
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

    /// Generate a narrative biography of an entity.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The entity is not found.
    /// - No LLM model is loaded.
    /// - Inference fails.
    pub async fn narrate(&self, entity_name: &str) -> Result<String> {
        let handle = self
            .model
            .ok_or_else(|| anyhow!("No AI model loaded. Please load a model first."))?;

        let entity = self.find_entity_by_name(entity_name)?;
        let history = self.get_sorted_history(entity.id)?;

        let prompt = Self::construct_prompt(&entity, &history)?;

        let params = InferenceParams {
            max_tokens: 512,
            temperature: 0.7,
            ..InferenceParams::default()
        };

        let biography = self.vortex.infer(handle, &prompt, params).await?;

        Ok(biography)
    }

    fn find_entity_by_name(&self, name: &str) -> Result<Entity> {
        let knowledge = self.gallifrey.knowledge();
        let mut found = None;

        // Scan history to find any version of the entity with the given name
        knowledge.scan_history(|history| {
            if found.is_some() {
                return;
            }
            // Prefer current version, but take any if current not found
            if let Some(e) = history
                .iter()
                .find(|e| e.name == name && e.temporal.is_current())
            {
                found = Some(e.clone());
            } else if let Some(e) = history.iter().find(|e| e.name == name) {
                // If we haven't found a current one yet, take this historical one
                // But we continue searching in case we find a current one in another bucket (unlikely for ID map)
                // Actually scan_history iterates values of hashmap, so one bucket per ID.
                // If we found it here, this is THE history for this ID.
                found = Some(e.clone());
            }
        })?;

        found.ok_or_else(|| anyhow!("Entity '{name}' not found"))
    }

    fn get_sorted_history(&self, id: EntityId) -> Result<Vec<Entity>> {
        let mut history = self
            .gallifrey
            .knowledge()
            .get_entity_history(id)
            .map_err(|e| anyhow!(e.to_string()))?;

        // Sort by valid time start, then transaction time start
        history.sort_by(|a, b| {
            a.temporal
                .valid_time
                .start
                .cmp(&b.temporal.valid_time.start)
                .then(
                    a.temporal
                        .transaction_time
                        .start
                        .cmp(&b.temporal.transaction_time.start),
                )
        });

        Ok(history)
    }

    fn construct_prompt(entity: &Entity, history: &[Entity]) -> Result<String> {
        let mut prompt = String::new();

        writeln!(
            prompt,
            "You are a biographer for the Timey Wimey Operating System."
        )?;
        writeln!(
            prompt,
            "Write a short, engaging biography for the entity '{}' ({}), explaining its history.",
            entity.name, entity.entity_type
        )?;
        writeln!(
            prompt,
            "Highlight changes and any retrospective corrections (where Transaction Time differs from Valid Time)."
        )?;
        writeln!(prompt, "\nHistory:")?;

        for (i, version) in history.iter().enumerate() {
            writeln!(prompt, "\n[Version {}]", i + 1)?;
            writeln!(
                prompt,
                "Valid: {} -> {}",
                version.temporal.valid_time.start,
                version
                    .temporal
                    .valid_time
                    .end
                    .map_or_else(|| "FOREVER".to_string(), |t| t.to_string())
            )?;
            writeln!(
                prompt,
                "Transaction: {} -> {}",
                version.temporal.transaction_time.start,
                version
                    .temporal
                    .transaction_time
                    .end
                    .map_or_else(|| "CURRENT".to_string(), |t| t.to_string())
            )?;

            let props = serde_json::to_string(&version.properties)
                .unwrap_or_else(|_| "{}".to_string());
            writeln!(prompt, "Properties: {props}")?;
        }

        writeln!(prompt, "\nBiography:")?;

        Ok(prompt)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tardis_common::temporal::BiTemporalInterval;

    fn create_test_entity(name: &str) -> Entity {
        Entity {
            id: EntityId::new(),
            entity_type: "Person".to_string(),
            name: name.to_string(),
            properties: HashMap::new(),
            embedding: None,
            temporal: BiTemporalInterval::now(),
            source: None,
        }
    }

    #[tokio::test]
    async fn test_biographer_narrate() {
        let gallifrey = Arc::new(Gallifrey::new());
        let vortex = Arc::new(Vortex::new().unwrap());

        // Mock inference
        let mock_response = "This is a biography of The Doctor.";
        vortex.set_mock_inference(Box::new(move |_, prompt, _| {
            assert!(prompt.contains("The Doctor"));
            assert!(prompt.contains("History:"));
            Ok(mock_response.to_string())
        }));

        // Use a dummy handle for testing (mock ignores it or we can check it)
        let handle = ModelHandle::new(123);

        let biographer = Biographer::new(gallifrey.clone(), vortex, Some(handle));

        // Insert entity
        let entity = create_test_entity("The Doctor");
        gallifrey.insert(entity).await.unwrap();

        let result = biographer.narrate("The Doctor").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), mock_response);
    }

    #[tokio::test]
    async fn test_biographer_no_model() {
        let gallifrey = Arc::new(Gallifrey::new());
        let vortex = Arc::new(Vortex::new().unwrap());
        let biographer = Biographer::new(gallifrey, vortex, None);

        let result = biographer.narrate("Whatever").await;
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "No AI model loaded. Please load a model first."
        );
    }

    #[tokio::test]
    async fn test_biographer_entity_not_found() {
        let gallifrey = Arc::new(Gallifrey::new());
        let vortex = Arc::new(Vortex::new().unwrap());
        let handle = ModelHandle::new(1);
        let biographer = Biographer::new(gallifrey, vortex, Some(handle));

        let result = biographer.narrate("Ghost").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }
}
