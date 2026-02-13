//! # Tardis Gallifrey
//!
//! The temporal knowledge store for Tardis OS, integrating `GallifreyDB`.
//!
//! Gallifrey provides three specialized stores:
//! - **Knowledge Store**: Entity-relationship graph with embeddings. Used for long-term semantic memory.
//! - **Conversation Store**: Chat history with cross-session continuity. Used for context retention.
//! - **System State Store**: OS state snapshots. Used for time-travel debugging and "undo" functionality.
//!
//! ## Bi-Temporality
//!
//! All stores support bi-temporal queries, tracking two dimensions of time:
//! 1. **Valid Time**: When a fact was true in the real world.
//! 2. **Transaction Time**: When a fact was recorded in the system.
//!
//! This allows answering questions like:
//! - "What is the system state *now*?" (Current Valid, Current Transaction)
//! - "What did we *think* the system state was yesterday?" (Past Transaction, Past Valid)
//!
//! ## Getting Started
//!
//! ```rust
//! use tardis_gallifrey::Gallifrey;
//! use tardis_gallifrey::domain::Entity;
//! use tardis_common::id::EntityId;
//! use tardis_gallifrey::temporal::{BiTemporalInterval, TemporalQuery};
//! use std::collections::HashMap;
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // 1. Initialize the store
//! let gallifrey = Gallifrey::new();
//!
//! // 2. Create an entity representing a fact
//! let entity = Entity {
//!     id: EntityId::new(),
//!     entity_type: "Fact".to_string(),
//!     name: "The Sky".to_string(),
//!     properties: HashMap::from([
//!         ("color".to_string(), serde_json::json!("blue"))
//!     ]),
//!     embedding: None, // In real usage, this would be a vector
//!     temporal: BiTemporalInterval::now(),
//!     source: Some("User Observation".to_string()),
//! };
//!
//! // 3. Insert it into the Knowledge Store
//! let id = gallifrey.knowledge().insert_entity(entity)?;
//!
//! // 4. Update it (creates a new version, preserving history)
//! let updates: HashMap<String, serde_json::Value> = serde_json::from_value(serde_json::json!({"color": "dark_blue"}))?;
//! gallifrey.knowledge().update_entity(id, updates)?;
//!
//! // 5. Retrieve history to see both versions
//! let history = gallifrey.knowledge().get_entity_history(id)?;
//! assert_eq!(history.len(), 2);
//! # Ok(())
//! # }
//! ```

#![warn(missing_docs)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

pub mod domain;
pub mod error;
#[cfg(feature = "nova")]
/// Experimental features (Nova).
pub mod experimental;
pub mod query;
pub mod stores;
pub mod temporal;

// Re-export main types
pub use error::{GallifreyError, GallifreyResult};
pub use stores::{ConversationStore, KnowledgeStore, SystemStateStore};
pub use temporal::{BiTemporalInterval, TimeRange};

use std::sync::Arc;

/// The main Gallifrey database instance.
///
/// This struct acts as a facade over the specialized stores (`KnowledgeStore`, `ConversationStore`, `SystemStateStore`).
/// It provides a unified async API for the rest of the system.
#[derive(Debug)]
pub struct Gallifrey {
    knowledge: Arc<KnowledgeStore>,
    conversation: Arc<ConversationStore>,
    system_state: Arc<SystemStateStore>,
}

impl Gallifrey {
    /// Create a new Gallifrey instance with empty stores.
    #[must_use]
    pub fn new() -> Self {
        Self {
            knowledge: Arc::new(KnowledgeStore::new()),
            conversation: Arc::new(ConversationStore::new()),
            system_state: Arc::new(SystemStateStore::new()),
        }
    }

    /// Get access to the underlying knowledge store.
    #[must_use]
    pub fn knowledge(&self) -> Arc<KnowledgeStore> {
        Arc::clone(&self.knowledge)
    }

    /// Get access to the underlying conversation store.
    #[must_use]
    pub fn conversation(&self) -> Arc<ConversationStore> {
        Arc::clone(&self.conversation)
    }

    /// Get access to the underlying system state store.
    #[must_use]
    pub fn system_state(&self) -> Arc<SystemStateStore> {
        Arc::clone(&self.system_state)
    }
}

impl Default for Gallifrey {
    fn default() -> Self {
        Self::new()
    }
}
