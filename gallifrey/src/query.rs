//! Query types for Gallifrey.

use serde::{Deserialize, Serialize};
use tardis_common::domain::Entity;

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
