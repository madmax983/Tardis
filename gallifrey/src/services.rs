//! Service implementations for Gallifrey.

use crate::stores::{ConversationStore, KnowledgeStore, SystemStateStore};
use tardis_common::domain::{Change, Entity, Message, Session, Snapshot};
use tardis_common::id::{EntityId, SessionId};
use tardis_common::traits::{ConversationService, KnowledgeService, SystemStateService};
use tardis_common::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

#[async_trait]
impl KnowledgeService for KnowledgeStore {
    async fn insert_entity(&self, entity: Entity) -> Result<EntityId> {
        self.insert_entity(entity).map_err(tardis_common::Error::from)
    }

    async fn update_entity(&self, id: EntityId, updates: HashMap<String, serde_json::Value>) -> Result<()> {
        self.update_entity(id, updates).map_err(tardis_common::Error::from)
    }

    async fn get_entity(&self, id: EntityId) -> Result<Option<Entity>> {
        self.get_entity(id).map_err(tardis_common::Error::from)
    }

    async fn get_entity_history(&self, id: EntityId) -> Result<Vec<Entity>> {
        self.get_entity_history(id).map_err(tardis_common::Error::from)
    }

    async fn semantic_search(&self, embedding: &[f32], limit: usize) -> Result<Vec<Entity>> {
        self.semantic_search(embedding, limit).map_err(tardis_common::Error::from)
    }
}

#[async_trait]
impl ConversationService for ConversationStore {
    async fn create_session(&self) -> Result<SessionId> {
        self.create_session().map_err(tardis_common::Error::from)
    }

    async fn get_session(&self, id: SessionId) -> Result<Option<Session>> {
        self.get_session(id).map_err(tardis_common::Error::from)
    }

    async fn end_session(&self, id: SessionId) -> Result<()> {
        self.end_session(id).map_err(tardis_common::Error::from)
    }

    async fn add_message(&self, message: Message) -> Result<EntityId> {
        self.add_message(message).map_err(tardis_common::Error::from)
    }

    async fn get_recent_messages(&self, session_id: SessionId, limit: usize) -> Result<Vec<Message>> {
        self.get_recent_messages(session_id, limit).map_err(tardis_common::Error::from)
    }

    async fn semantic_search(&self, embedding: &[f32], limit: usize) -> Result<Vec<Message>> {
        self.semantic_search(embedding, limit).map_err(tardis_common::Error::from)
    }
}

#[async_trait]
impl SystemStateService for SystemStateStore {
    async fn find_snapshot_at(&self, timestamp: DateTime<Utc>) -> Result<Option<Snapshot>> {
        self.find_snapshot_at(timestamp).map_err(tardis_common::Error::from)
    }

    async fn record_change(&self, change: Change) -> Result<()> {
        self.record_change(change).map_err(tardis_common::Error::from)
    }
}
