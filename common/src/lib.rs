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

/// Core domain entities (Message, Knowledge, SystemState).
pub mod domain;

/// Unified error handling types.
pub mod error;

/// Type-safe identifiers (EntityId, SessionId, etc.).
pub mod id;

/// LLM inference configuration and parameters.
pub mod llm;

/// Bi-temporal time primitives (Valid Time vs Transaction Time).
pub mod temporal;

/// Shared service interfaces (Traits for dependency injection).
pub mod traits;

// Re-export commonly used items
pub use error::{Error, Result};
pub use id::{EntityId, ModelHandle, SessionId, SnapshotId};
pub use temporal::{BiTemporalInterval, TemporalQuery, TimeRange};
