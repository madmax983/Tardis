//! # Tardis Chronos
//!
//! The RAG (Retrieval-Augmented Generation) orchestration engine for Tardis OS.
//!
//! Chronos bridges Vortex (LLM) and Gallifrey (temporal storage) to provide:
//! - Multi-source retrieval (knowledge, conversation, system state)
//! - Temporal-aware context augmentation
//! - Query analysis with temporal reference extraction
//! - Memory consolidation and summarization

#![warn(missing_docs)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

pub mod error;
pub mod pipeline;
pub mod memory;

// Re-export main types
pub use error::{ChronosError, ChronosResult};
pub use pipeline::{Chronos, MemoryCategory, RagConfig, RagResponse};
