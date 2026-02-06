//! Shared traits for Tardis subsystem interfaces.
//!
//! These traits define the contracts between subsystems, designed to support
//! both direct function calls (monolithic) and potential future IPC (microkernel).

use crate::domain::Entity;
use serde::{Deserialize, Serialize};

/// Query results from Gallifrey.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    /// Result nodes
    pub nodes: Vec<Entity>,
    /// Query execution time in milliseconds
    pub execution_time_ms: u64,
    /// Whether results were truncated
    pub truncated: bool,
}
