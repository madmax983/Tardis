//! Query execution for Gallifrey.

use crate::error::GallifreyResult;
use serde::{Deserialize, Serialize};

/// Parsed query structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedQuery {
    /// Query string.
    pub query: String,
    /// Query parameters.
    pub params: Vec<String>,
}

/// Query result structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    /// Result rows.
    pub rows: Vec<serde_json::Value>,
}

/// Query executor.
#[derive(Debug)]
pub struct QueryExecutor {
    // TODO: Add connection to stores
}

impl QueryExecutor {
    /// Create a new query executor.
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }

    /// Execute a query.
    ///
    /// # Errors
    ///
    /// Returns an error if execution fails.
    pub const fn execute(&self, _query: &ParsedQuery) -> GallifreyResult<QueryResult> {
        // TODO: Implement actual query execution

        Ok(QueryResult {
            rows: Vec::new(),
        })
    }
}

impl Default for QueryExecutor {
    fn default() -> Self {
        Self::new()
    }
}
