//! Service implementations for Vortex.

use crate::Vortex;
use std::sync::Arc;
use tardis_common::id::ModelHandle;
use tardis_common::llm::InferenceParams;
use tardis_common::Result;

/// A wrapper around Vortex that binds it to a specific model.
///
/// This adapter allows it to be used by components that require a bound LLM interface (like Chronos).
#[derive(Debug)]
pub struct VortexLlmService {
    engine: Arc<Vortex>,
    model: ModelHandle,
}

impl VortexLlmService {
    /// Create a new service adapter.
    #[must_use]
    pub const fn new(engine: Arc<Vortex>, model: ModelHandle) -> Self {
        Self { engine, model }
    }

    /// Get the underlying Vortex engine.
    #[must_use]
    pub const fn engine(&self) -> &Arc<Vortex> {
        &self.engine
    }

    /// Generate text based on a prompt.
    ///
    /// # Errors
    ///
    /// Returns an error if inference fails.
    pub async fn infer(&self, prompt: &str, params: InferenceParams) -> Result<String> {
        self.engine
            .infer(self.model, prompt, params)
            .await
            .map_err(|e| tardis_common::Error::Internal(e.to_string()))
    }

    /// Generate embeddings for text.
    ///
    /// # Errors
    ///
    /// Returns an error if embedding generation fails.
    pub async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        self.engine
            .embed(self.model, text)
            .await
            .map_err(|e| tardis_common::Error::Internal(e.to_string()))
    }
}
