//! The Saga 📜
//!
//! A narrative engine that tells the story of the journey between two entities.
//!
//! It uses the Astrolabe to find a semantic path and the LLM to narrate the transitions.

use crate::experimental::astrolabe::Astrolabe;
use anyhow::{anyhow, Result};
use std::fmt::Write;
use std::sync::Arc;
use tardis_common::id::ModelHandle;
use tardis_common::llm::InferenceParams;
use tardis_common::traits::LlmService;
use tardis_gallifrey::Gallifrey;

/// The Saga engine.
#[derive(Debug)]
pub struct Saga {
    astrolabe: Astrolabe,
    llm: Arc<dyn LlmService>,
    model: Option<ModelHandle>,
}

impl Saga {
    /// Create a new Saga instance.
    #[must_use]
    pub fn new(
        gallifrey: Arc<Gallifrey>,
        llm: Arc<dyn LlmService>,
        model: Option<ModelHandle>,
    ) -> Self {
        Self {
            astrolabe: Astrolabe::new(gallifrey),
            llm,
            model,
        }
    }

    /// Tell the saga of the journey between two entities.
    ///
    /// # Errors
    ///
    /// Returns an error if pathfinding fails or inference fails.
    pub async fn tell(&self, start_name: &str, end_name: &str) -> Result<String> {
        let path = self
            .astrolabe
            .navigate(start_name, end_name)
            .map_err(|e| anyhow!(e.to_string()))?;

        if path.is_empty() {
            return Ok(format!(
                "There is no path from {start_name} to {end_name}."
            ));
        }

        // Construct the prompt
        let mut narrative_prompt = String::new();
        narrative_prompt
            .push_str("<s>[INST] You are The Bard, a legendary storyteller for the Tardis OS.\n");
        narrative_prompt.push_str(
            "Recount the saga of the journey from the start to the end based on this path:\n\n",
        );

        for (i, segment) in path.iter().enumerate() {
            let step_number = i + 1;
            let entity = &segment.entity;
            let via = segment.via.as_deref().unwrap_or("connected to");

            let _ = write!(
                narrative_prompt,
                "{step_number}. [{}] ({})\n   -> via [{via}] ->\n",
                entity.name, entity.entity_type
            );
        }

        narrative_prompt.push_str("\nWeave these steps into a cohesive, epic, short story (max 200 words). Focus on the causal links and the meaning of the journey.\n");
        narrative_prompt.push_str("The Saga: [/INST]");

        let mut params = InferenceParams::default()
            .with_temperature(0.7)
            .with_max_tokens(300);

        if let Some(model) = self.model {
            params = params.with_model(model);
        }

        let story = self
            .llm
            .infer(&narrative_prompt, params)
            .await
            .map_err(|e| anyhow!(e.to_string()))?;

        Ok(story)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::any::Any;

    #[derive(Debug)]
    struct MockLlm;

    #[async_trait]
    impl LlmService for MockLlm {
        async fn infer(
            &self,
            _prompt: &str,
            _params: InferenceParams,
        ) -> tardis_common::Result<String> {
            Ok("This is a mock saga.".to_string())
        }

        async fn embed(&self, _text: &str) -> tardis_common::Result<Vec<f32>> {
            Ok(vec![0.0; 384])
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    #[tokio::test]
    async fn test_saga_creation() {
        let gallifrey = Arc::new(Gallifrey::new());
        let llm = Arc::new(MockLlm);
        let saga = Saga::new(gallifrey, llm, None);

        // We can't easily test `tell` without populating Gallifrey with data that Astrolabe can navigate.
        // But we can verify it compiles and instantiates.
        assert!(format!("{:?}", saga).contains("Saga"));
    }
}
