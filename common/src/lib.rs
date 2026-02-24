//! # Tardis Common
//!
//! Shared types, traits, and utilities used across all Tardis OS subsystems.
//!
//! This crate provides:
//! - Common error types and result aliases
//! - Identifier types (handles, IDs)
//! - Temporal primitives for bi-temporal data
//! - Shared traits for subsystem interfaces
//! - LLM configuration and domain entities

#![warn(missing_docs)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

pub mod domain;
pub mod error;
pub mod id;
pub mod llm;
pub mod temporal;
pub mod traits;

// Re-export commonly used items
pub use error::{Error, Result};
pub use id::{EntityId, MessageId, ModelHandle, SessionId, SnapshotId};
pub use temporal::{BiTemporalInterval, TemporalQuery, TimeRange};
