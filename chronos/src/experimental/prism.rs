//! The Prism: Multi-Perspective Analysis Engine.
//!
//! "Truth is a matter of perspective."
//!
//! This module breaks down a query into its spectral components, analyzing it
//! from multiple viewpoints (personas) simultaneously to provide a comprehensive answer.

use crate::error::{ChronosError, ChronosResult};
use crate::experimental::psychic_paper;
use crate::{Chronos, RagConfig};
use std::sync::Arc;
use tardis_common::id::ModelHandle;
use tardis_vortex::{InferenceParams, Vortex};
use tracing::{info, instrument, warn};

/// A result from a single perspective.
#[derive(Debug, Clone)]
pub struct PerspectiveResult {
    /// The perspective (persona) adopted.
    pub perspective: String,
    /// The answer from that perspective.
    pub response: String,
}

/// The Prism engine.
#[derive(Debug)]
pub struct Prism {
    chronos: Arc<Chronos>,
    vortex: Arc<Vortex>,
}

impl Prism {
    /// Create a new Prism engine.
    #[must_use]
    pub fn new(chronos: Arc<Chronos>) -> Self {
        let vortex = chronos.vortex().clone();
        Self {
            chronos,
            vortex,
        }
    }

    /// Refract a query through multiple perspectives.
    ///
    /// # Errors
    ///
    /// Returns an error if the analysis fails.
    #[instrument(skip(self))]
    pub async fn refract(
        &self,
        query: &str,
        model: ModelHandle,
    ) -> ChronosResult<Vec<PerspectiveResult>> {
        info!("Refracting query: {}", query);

        // 1. Identify Perspectives
        let prompt = format!(
            "Analyze the following query and identify 3 distinct, relevant professional perspectives or personas that would provide valuable insight.\n\
            Query: \"{query}\"\n\
            Return ONLY a JSON list of strings (e.g. [\"Security Engineer\", \"Product Manager\"])."
        );

        let params = InferenceParams::default()
            .with_temperature(0.7)
            .with_max_tokens(100);

        let response = self
            .vortex
            .infer(model, &prompt, params)
            .await
            .map_err(|e| ChronosError::Common(tardis_common::Error::Internal(e.to_string())))?;

        let perspectives_val = psychic_paper::extract_json(&response)
            .map_err(|e| ChronosError::Common(tardis_common::Error::Internal(e)))?;

        let mut perspectives: Vec<String> =
            serde_json::from_value(perspectives_val).unwrap_or_default();

        if perspectives.is_empty() {
            // Fallback
            perspectives = vec![
                "Analytical Observer".to_string(),
                "Critical Skeptic".to_string(),
                "Creative Visionary".to_string(),
            ];
        }

        // Limit to 5
        perspectives.truncate(5);

        info!("Identified perspectives: {:?}", perspectives);

        // 2. Query each perspective in parallel
        let mut handles = Vec::new();

        for persona in perspectives {
            let p_clone = persona.clone();
            let q_clone = query.to_string();
            let chronos = self.chronos.clone();

            handles.push(tokio::spawn(async move {
                let config = RagConfig {
                    persona: Some(p_clone.clone()),
                    ..RagConfig::default()
                };

                match chronos.query(&q_clone, config).await {
                    Ok(resp) => Some(PerspectiveResult {
                        perspective: p_clone,
                        response: resp.text,
                    }),
                    Err(e) => {
                        warn!("Perspective '{}' failed: {}", p_clone, e);
                        None
                    }
                }
            }));
        }

        let mut results = Vec::new();
        for handle in handles {
            if let Ok(Some(res)) = handle.await {
                results.push(res);
            }
        }

        Ok(results)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tardis_common::traits::{
        ConversationService, KnowledgeService, LlmService, SystemStateService,
    };
    use tardis_vortex::ModelLoadConfig;
    use tardis_vortex::VortexLlmService;

    #[tokio::test]
    async fn test_refract_flow() {
        // Setup Dependencies
        let vortex = Arc::new(Vortex::new().unwrap());

        // Mock Vortex Behavior
        // We use a simple state to differentiate calls if possible, or just look at prompt.
        vortex.set_mock_inference(Box::new(|_, prompt, _| {
            if prompt.contains("identify 3 distinct") {
                Ok("[\"Historian\", \"Futurist\"]".to_string())
            } else {
                // RAG Query
                Ok(format!("Answer for: {prompt}"))
            }
        }));

        // Mock load model
        vortex.set_mock_load_model(Box::new(|_, _| Ok(ModelHandle::new(1))));

        let handle = vortex
            .load_model("dummy", ModelLoadConfig::default())
            .await
            .unwrap();

        // Construct Chronos with dummy services
        let llm_service =
            Arc::new(VortexLlmService::new(vortex.clone(), handle)) as Arc<dyn LlmService>;

        let knowledge =
            Arc::new(tardis_gallifrey::stores::KnowledgeStore::new()) as Arc<dyn KnowledgeService>;
        let conversation = Arc::new(tardis_gallifrey::stores::ConversationStore::new())
            as Arc<dyn ConversationService>;
        let system_state = Arc::new(tardis_gallifrey::stores::SystemStateStore::new())
            as Arc<dyn SystemStateService>;

        let chronos = Arc::new(Chronos::new(
            llm_service,
            knowledge,
            conversation,
            system_state,
        ));

        let prism = Prism::new(chronos);

        let results = prism
            .refract("What is the meaning of life?", handle)
            .await
            .unwrap();

        assert_eq!(results.len(), 2);
        // We can't guarantee order with async spawn, so check existence
        let perspectives: Vec<String> = results.iter().map(|r| r.perspective.clone()).collect();
        assert!(perspectives.contains(&"Historian".to_string()));
        assert!(perspectives.contains(&"Futurist".to_string()));

        // Check content
        let response = &results[0].response;
        assert!(response.contains("Answer for:"));
    }
}
