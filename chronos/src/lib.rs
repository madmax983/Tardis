//! # Tardis Chronos
//!
//! The RAG (Retrieval-Augmented Generation) orchestration engine for Tardis OS.
//!
//! Chronos bridges Vortex (LLM) and Gallifrey (temporal storage) to provide:
//! - Multi-source retrieval (knowledge, conversation, system state)
//! - Temporal-aware context augmentation
//! - Query analysis with temporal reference extraction
//! - Memory consolidation and summarization
//!
//! ## Example: The Hero's Journey
//!
//! This example demonstrates how to initialize the RAG pipeline and ask a question.
//!
//! ```rust,no_run
//! use std::sync::Arc;
//! use tardis_chronos::{Chronos, RagConfig};
//! use tardis_vortex::services::VortexLlmService;
//! use tardis_gallifrey::stores::{KnowledgeStore, ConversationStore, SystemStateStore};
//!
//! # async fn example(
//! #     llm: Arc<VortexLlmService>,
//! #     knowledge: Arc<KnowledgeStore>,
//! #     conversation: Arc<ConversationStore>,
//! #     system_state: Arc<SystemStateStore>
//! # ) -> anyhow::Result<()> {
//! // 2. Create the Chronos RAG engine
//! let chronos = Chronos::new(llm, knowledge, conversation, system_state);
//!
//! // 3. Ask a question!
//! // Chronos will:
//! // - Analyze the query for intent and temporal references (e.g., "yesterday")
//! // - Retrieve relevant context from Gallifrey
//! // - Augment the prompt with context and system instructions
//! // - Generate a response using Vortex
//! let response = chronos.query(
//!     "What did we discuss yesterday about the satellite launch?",
//!     RagConfig::default()
//! ).await?;
//!
//! println!("Answer: {}", response.text);
//! println!("Sources used: {}", response.sources.len());
//! # Ok(())
//! # }
//! ```

#![warn(missing_docs)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

pub mod error;
pub mod pipeline;

// Re-export main types
pub use error::{ChronosError, ChronosResult};
pub use pipeline::{Chronos, MemoryCategory, RagConfig, RagResponse};

#[cfg(feature = "nova")]
pub mod experimental;
