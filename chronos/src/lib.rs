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
//! use tardis_vortex::{Vortex, VortexLlmService, ModelLoadConfig};
//! use tardis_gallifrey::Gallifrey;
//!
//! # async fn example() -> anyhow::Result<()> {
//! // 1. Initialize Vortex (LLM)
//! let vortex = Arc::new(Vortex::new()?);
//! let handle = vortex.load_model("model.safetensors", ModelLoadConfig::default()).await?;
//! let llm = Arc::new(VortexLlmService::new(vortex, handle));
//!
//! // 2. Initialize Gallifrey (Memory)
//! let gallifrey = Gallifrey::new();
//!
//! // 3. Create the Chronos RAG engine
//! // Connects semantic reasoning (LLM) with bi-temporal memory (Gallifrey)
//! let chronos = Chronos::new(
//!     llm,
//!     gallifrey.knowledge(),
//!     gallifrey.conversation(),
//!     gallifrey.system_state()
//! );
//!
//! // 4. Ask a question!
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
