//! Shared traits for system services.
//!
//! These traits define the core architectural boundaries of Tardis OS. By defining services
//! as traits, we enable:
//!
//! 1.  **Decoupling**: Consumers (like `Chronos`) depend on abstract interfaces, not concrete implementations.
//! 2.  **Testability**: We can easily swap in mock implementations for unit testing.
//! 3.  **Flexibility**: We can change the underlying storage or inference engine without breaking consumers.
//!
//! # The Big Four Services
//!
//! *   [`LlmService`]: The brain. Handles text generation and embedding.
//! *   [`KnowledgeService`]: The long-term memory. Stores facts and relationships. (Deprecated: Use `tardis_gallifrey::KnowledgeStore` directly)
//! *   [`ConversationService`]: The short-term memory. Manages chat history. (Deprecated: Use `tardis_gallifrey::ConversationStore` directly)
//! *   [`SystemStateService`]: The nervous system. Tracks OS state changes. (Deprecated: Use `tardis_gallifrey::SystemStateStore` directly)
//!
//! # `async_trait`
//!
//! You'll notice all traits are annotated with `#[async_trait]`. This is because Rust
//! currently does not support async functions in traits natively (without boxing).
//! This macro boxes the returned futures, making the traits object-safe (`dyn Trait`).
//!
//! # Example: Mocking for Tests
//!
//! ```rust
//! use tardis_common::traits::LlmService;
//! use tardis_common::llm::InferenceParams;
//! use tardis_common::Result;
//! use async_trait::async_trait;
//! use std::any::Any;
//!
//! #[derive(Debug)]
//! struct MockLlm;
//!
//! #[async_trait]
//! impl LlmService for MockLlm {
//!     async fn infer(&self, prompt: &str, _params: InferenceParams) -> Result<String> {
//!         Ok(format!("Mock response to: {}", prompt))
//!     }
//!
//!     async fn embed(&self, _text: &str) -> Result<Vec<f32>> {
//!         Ok(vec![0.0; 384])
//!     }
//!
//!     fn as_any(&self) -> &dyn Any {
//!         self
//!     }
//! }
//! ```

use crate::llm::InferenceParams;
use crate::Result;
use async_trait::async_trait;
use std::any::Any;
use std::fmt::Debug;

/// Interface for LLM inference services.
///
/// This trait abstracts over different LLM backends (e.g., local `Vortex`, remote APIs).
/// It provides methods for text generation (`infer`) and vector embedding (`embed`).
#[async_trait]
pub trait LlmService: Send + Sync + Debug {
    /// Generate text based on a prompt.
    ///
    /// # Arguments
    ///
    /// * `prompt` - The input text or prompt for the model.
    /// * `params` - Configuration for generation (temperature, max tokens, etc.).
    async fn infer(&self, prompt: &str, params: InferenceParams) -> Result<String>;

    /// Generate embeddings for text.
    ///
    /// Used for semantic search. Returns a vector of floats representing the semantic meaning.
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;

    /// Downcast to concrete type.
    ///
    /// Useful when you need access to backend-specific methods not exposed by the trait.
    fn as_any(&self) -> &dyn Any;
}
