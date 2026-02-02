//! # Tardis Gallifrey
//!
//! The temporal knowledge store for Tardis OS, integrating `GallifreyDB`.
//!
//! Gallifrey provides three specialized stores:
//! - **Knowledge Store**: Entity-relationship graph with embeddings
//! - **Conversation Store**: Chat history with cross-session continuity
//! - **System State Store**: OS state snapshots for time-travel debugging
//!
//! All stores support bi-temporal queries:
//! - **Valid time**: When the fact was true in the real world
//! - **Transaction time**: When the fact was recorded in the system

#![warn(missing_docs)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

pub mod error;
pub mod query;
pub mod stores;
pub mod temporal;

// Re-export main types
pub use error::{GallifreyError, GallifreyResult};
pub use stores::{ConversationStore, KnowledgeStore, SystemStateStore};
pub use temporal::{BiTemporalInterval, TimeRange};

use std::sync::Arc;

/// The main Gallifrey database instance.
#[derive(Debug)]
pub struct Gallifrey {
    knowledge: Arc<KnowledgeStore>,
    conversation: Arc<ConversationStore>,
    system_state: Arc<SystemStateStore>,
}

impl Gallifrey {
    /// Create a new Gallifrey instance.
    #[must_use]
    pub fn new() -> Self {
        Self {
            knowledge: Arc::new(KnowledgeStore::new()),
            conversation: Arc::new(ConversationStore::new()),
            system_state: Arc::new(SystemStateStore::new()),
        }
    }

    /// Get access to the knowledge store.
    #[must_use]
    pub fn knowledge(&self) -> Arc<KnowledgeStore> {
        Arc::clone(&self.knowledge)
    }

    /// Get access to the conversation store.
    #[must_use]
    pub fn conversation(&self) -> Arc<ConversationStore> {
        Arc::clone(&self.conversation)
    }

    /// Get access to the system state store.
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
