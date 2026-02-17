//! The Biographer
//!
//! Generates narrative biographies for entities based on their temporal history.
//! This connects `Gallifrey` (Time) + `Vortex` (LLM).

use std::fmt::Write;
use std::sync::Arc;
use tardis_gallifrey::Gallifrey;
use tardis_vortex::{InferenceParams, ModelHandle, Vortex};

/// The Biographer engine.
#[derive(Debug)]
pub struct Biographer {
    gallifrey: Arc<Gallifrey>,
    vortex: Arc<Vortex>,
    model: Option<ModelHandle>,
}

impl Biographer {
    /// Create a new Biographer.
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

    /// Generate a biography for an entity.
    ///
    /// # Errors
    ///
    /// Returns an error if the entity is not found or if the biography generation fails.
    pub async fn biography(&self, name: &str) -> anyhow::Result<String> {
        // 1. Find the entity ID by name
        // We scan history because we want *any* version of the entity that had this name.
        let mut target_id = None;

        // Use scan_history (gated by nova in gallifrey)
        #[cfg(feature = "nova")]
        self.gallifrey.knowledge().scan_history(|history| {
            if target_id.is_some() {
                return;
            }
            // Check if any version has the name
            if history.iter().any(|e| e.name.eq_ignore_ascii_case(name)) {
                if let Some(first) = history.first() {
                    target_id = Some(first.id);
                }
            }
        })?;

        let id = target_id.ok_or_else(|| anyhow::anyhow!("Entity '{name}' not found"))?;

        // 2. Get full history
        let history = self.gallifrey.get_history(id).await?;
        if history.is_empty() {
            return Err(anyhow::anyhow!("No history found for entity '{name}'"));
        }

        // 3. Format history for the LLM
        let mut prompt = String::new();
        writeln!(&mut prompt, "You are a historian. Write a short, narrative biography for the entity '{name}' based on the following timeline of events. Focus on how it changed over time.")?;
        writeln!(&mut prompt, "\nTimeline:")?;

        for (i, version) in history.iter().enumerate() {
            let valid_from = version.temporal.valid_time.start;
            let valid_to = version.temporal.valid_time.end;

            // Format time range nicely
            let time_str = match valid_to {
                Some(end) => format!(
                    "From {} to {}",
                    valid_from.format("%Y-%m-%d %H:%M"),
                    end.format("%Y-%m-%d %H:%M")
                ),
                None => format!("From {} onwards", valid_from.format("%Y-%m-%d %H:%M")),
            };

            writeln!(&mut prompt, "{}. [{time_str}]", i + 1)?;
            writeln!(&mut prompt, "   Name: {}", version.name)?;
            writeln!(&mut prompt, "   Type: {}", version.entity_type)?;
            if !version.properties.is_empty() {
                writeln!(&mut prompt, "   Properties: {:?}", version.properties)?;
            }
        }

        writeln!(&mut prompt, "\nBiography:")?;

        // 4. Generate biography
        if let Some(handle) = self.model {
            let params = InferenceParams {
                max_tokens: 500,
                temperature: 0.7,
                ..Default::default()
            };

            let bio = self.vortex.infer(handle, &prompt, params).await?;
            Ok(bio)
        } else {
            // Fallback if no model is loaded
            Ok(format!("(No model loaded. Prompt would be:)\n\n{prompt}"))
        }
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
    async fn test_biography_generation() {
        // Setup
        let gallifrey = Arc::new(Gallifrey::new());
        let vortex = Arc::new(Vortex::new().unwrap());

        // Mock inference
        vortex.set_mock_inference(Box::new(|_, prompt, _| {
            Ok(format!(
                "Mock biography based on prompt length: {}",
                prompt.len()
            ))
        }));

        // Mock load model to fail gracefully if we try (but we pass None for model handle)
        vortex.set_mock_load_model(Box::new(|_, _| {
            Err(tardis_vortex::VortexError::ModelNotFound {
                path: "dummy".into(),
            })
        }));

        let entity = Entity {
            id: EntityId::new(),
            entity_type: "Person".to_string(),
            name: "The Doctor".to_string(),
            properties: HashMap::from([("status".to_string(), serde_json::json!("Exiled"))]),
            embedding: None,
            temporal: BiTemporalInterval::now(),
            source: None,
        };

        gallifrey.insert(entity).await.unwrap();

        // Test fallback (no model)
        let bio = Biographer::new(gallifrey.clone(), vortex.clone(), None);
        let result = bio.biography("The Doctor").await.unwrap();

        assert!(result.contains("The Doctor"));
        assert!(result.contains("Exiled"));
        assert!(result.contains("(No model loaded. Prompt would be:)"));
    }
}
