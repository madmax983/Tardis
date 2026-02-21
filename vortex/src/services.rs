//! Service implementations for Vortex.

use crate::Vortex;
use tardis_common::id::ModelHandle;
use tardis_common::llm::InferenceParams;
use tardis_common::traits::LlmService;
use tardis_common::Result;
use async_trait::async_trait;
use std::sync::Arc;
use std::any::Any;

/// A wrapper around Vortex that binds it to a specific model.
///
/// This adapter implements the [`LlmService`] trait, allowing it to be used
/// by components that require a generic LLM interface (like Chronos).
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
    pub fn engine(&self) -> Arc<Vortex> {
        self.engine.clone()
    }

    /// Get the model handle.
    #[must_use]
    pub const fn model(&self) -> ModelHandle {
        self.model
    }
}

#[async_trait]
impl LlmService for VortexLlmService {
    fn as_any(&self) -> &dyn Any {
        self
    }

    async fn infer(&self, prompt: &str, params: InferenceParams) -> Result<String> {
        self.engine
            .infer(self.model, prompt, params)
            .await
            .map_err(|e| tardis_common::Error::Internal(e.to_string()))
    }

    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        self.engine
            .embed(self.model, text)
            .await
            .map_err(|e| tardis_common::Error::Internal(e.to_string()))
    }
}
