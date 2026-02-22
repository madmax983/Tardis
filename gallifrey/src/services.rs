//! Service implementations for Gallifrey.

use crate::stores::{ConversationStore, KnowledgeStore, SystemStateStore};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use tardis_common::domain::{Change, Entity, Message, Session, Snapshot};
use tardis_common::id::{EntityId, SessionId};
use tardis_common::traits::{ConversationService, KnowledgeService, SystemStateService};
use tardis_common::Result;

#[async_trait]
impl KnowledgeService for KnowledgeStore {
    async fn insert_entity(&self, entity: Entity) -> Result<EntityId> {
        self.insert_entity(entity)
            .map_err(tardis_common::Error::from)
    }

    async fn update_entity(
        &self,
        id: EntityId,
        updates: HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        self.update_entity(id, updates)
            .map_err(tardis_common::Error::from)
    }

    async fn get_entity(&self, id: EntityId) -> Result<Option<Entity>> {
        self.get_entity(id).map_err(tardis_common::Error::from)
    }

    async fn get_entity_history(&self, id: EntityId) -> Result<Vec<Entity>> {
        self.get_entity_history(id)
            .map_err(tardis_common::Error::from)
    }

    async fn semantic_search(&self, embedding: &[f32], limit: usize) -> Result<Vec<Entity>> {
        self.semantic_search(embedding, limit)
            .map_err(tardis_common::Error::from)
    }

    async fn find_entity_by_name(&self, name: &str) -> Result<Option<Entity>> {
        let mut found = None;
        let now = Utc::now();

        self.scan_history(|history| {
            if found.is_some() {
                return;
            }
            if let Some(e) = history.iter().find(|e| {
                e.name.eq_ignore_ascii_case(name) && e.temporal.active_at(now, now)
            }) {
                found = Some(e.clone());
            }
        })
        .map_err(tardis_common::Error::from)?;

        Ok(found)
    }

    async fn search_history(
        &self,
        query: &str,
        time: Option<DateTime<Utc>>,
        limit: usize,
    ) -> Result<Vec<Entity>> {
        let mut results = Vec::new();
        let query_lower = query.to_lowercase();
        // Split query into words for simple keyword matching
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();

        let query_time = time.unwrap_or_else(Utc::now);
        // For historical query, we check what was valid at that time
        // AND what was known at that time (bi-temporal snapshot).
        let transaction_time = query_time;

        self.scan_history(|history| {
            if results.len() >= limit {
                return;
            }

            // Find version active at query_time
            if let Some(entity) = history
                .iter()
                .find(|e| e.temporal.active_at(query_time, transaction_time))
            {
                let name_lower = entity.name.to_lowercase();

                // Simple relevance heuristic:
                // 1. Entity name contains query (or part of it)
                // 2. Query contains entity name
                let relevant = query_lower.contains(&name_lower)
                    || query_words
                        .iter()
                        .any(|w| w.len() > 3 && name_lower.contains(w));

                if relevant {
                    results.push(entity.clone());
                }
            }
        })
        .map_err(tardis_common::Error::from)?;

        Ok(results)
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
        self.add_message(message)
            .map_err(tardis_common::Error::from)
    }

    async fn get_recent_messages(
        &self,
        session_id: SessionId,
        limit: usize,
    ) -> Result<Vec<Message>> {
        self.get_recent_messages(session_id, limit)
            .map_err(tardis_common::Error::from)
    }

    async fn semantic_search(&self, embedding: &[f32], limit: usize) -> Result<Vec<Message>> {
        self.semantic_search(embedding, limit)
            .map_err(tardis_common::Error::from)
    }
}

#[async_trait]
impl SystemStateService for SystemStateStore {
    async fn find_snapshot_at(&self, timestamp: DateTime<Utc>) -> Result<Option<Snapshot>> {
        self.find_snapshot_at(timestamp)
            .map_err(tardis_common::Error::from)
    }

    async fn record_change(&self, change: Change) -> Result<()> {
        self.record_change(change)
            .map_err(tardis_common::Error::from)
    }
}
