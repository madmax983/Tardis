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
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::unused_async)]
#![allow(clippy::missing_const_for_fn)]
#![allow(clippy::unused_self)]
#![allow(clippy::format_push_string)]
#![allow(clippy::uninlined_format_args)]

pub mod error;
pub mod memory;
pub mod pipeline;

// Re-export main types
pub use error::{ChronosError, ChronosResult};
pub use pipeline::{Chronos, MemoryCategory, RagConfig, RagResponse};
