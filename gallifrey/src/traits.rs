use crate::domain::{Change, Entity, Message, Relationship, Snapshot};
use async_trait::async_trait;
use tardis_common::id::{EntityId, SessionId};
use tardis_common::Result;

/// The Gallifrey service trait.
#[async_trait]
pub trait GallifreyService: Send + Sync + std::fmt::Debug {
    /// Insert a node into the knowledge graph.
    async fn insert(&self, node: Entity) -> Result<EntityId>;

    /// Update an existing node.
    async fn update(&self, id: EntityId, properties: serde_json::Value) -> Result<()>;

    /// Get the history of an entity.
    async fn get_history(&self, id: EntityId) -> Result<Vec<Entity>>;

    /// Semantic search for entities.
    async fn search_knowledge(&self, embedding: &[f32], limit: usize) -> Result<Vec<Entity>>;

    /// Get recent messages from a session.
    async fn get_recent_messages(&self, session_id: SessionId, limit: usize) -> Result<Vec<Message>>;

    /// Semantic search for messages.
    async fn search_conversation(
        &self,
        embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<Message>>;

    /// Create a new conversation session.
    fn create_session(&self) -> Result<SessionId>;

    /// End a conversation session.
    fn end_session(&self, session_id: SessionId) -> Result<()>;

    /// Find a system snapshot at a specific time.
    async fn find_snapshot(
        &self,
        timestamp: chrono::DateTime<chrono::Utc>,
    ) -> Result<Option<Snapshot>>;

    /// Record a system change.
    async fn record_change(&self, change: Change) -> Result<()>;

    /// Get system changes in a time range.
    fn get_system_changes(
        &self,
        from: chrono::DateTime<chrono::Utc>,
        to: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<Change>>;

    /// Scan entity history.
    #[cfg(feature = "nova")]
    fn scan_history(&self, callback: Box<dyn FnMut(&[Entity]) + Send>) -> Result<()>;

    /// Scan relationships.
    #[cfg(feature = "nova")]
    fn scan_relationships(&self, callback: Box<dyn FnMut(&[Relationship]) + Send>) -> Result<()>;

    /// Insert a relationship.
    fn insert_relationship(&self, rel: Relationship) -> Result<()>;

    /// Get an entity at a specific point in bi-temporal time.
    fn get_entity_at(
        &self,
        id: EntityId,
        valid_time: chrono::DateTime<chrono::Utc>,
        transaction_time: chrono::DateTime<chrono::Utc>,
    ) -> Result<Option<Entity>>;
}

#[async_trait]
impl GallifreyService for crate::Gallifrey {
    async fn insert(&self, node: Entity) -> Result<EntityId> {
        self.insert(node).await
    }

    async fn update(&self, id: EntityId, properties: serde_json::Value) -> Result<()> {
        self.update(id, properties).await
    }

    async fn get_history(&self, id: EntityId) -> Result<Vec<Entity>> {
        self.get_history(id).await
    }

    async fn search_knowledge(&self, embedding: &[f32], limit: usize) -> Result<Vec<Entity>> {
        self.search_knowledge(embedding, limit).await
    }

    async fn get_recent_messages(&self, session_id: SessionId, limit: usize) -> Result<Vec<Message>> {
        self.get_recent_messages(session_id, limit).await
    }

    async fn search_conversation(
        &self,
        embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<Message>> {
        self.search_conversation(embedding, limit).await
    }

    fn create_session(&self) -> Result<SessionId> {
        self.create_session()
    }

    fn end_session(&self, session_id: SessionId) -> Result<()> {
        self.end_session(session_id)
    }

    async fn find_snapshot(
        &self,
        timestamp: chrono::DateTime<chrono::Utc>,
    ) -> Result<Option<Snapshot>> {
        self.find_snapshot(timestamp).await
    }

    async fn record_change(&self, change: Change) -> Result<()> {
        self.record_change(change).await
    }

    fn get_system_changes(
        &self,
        from: chrono::DateTime<chrono::Utc>,
        to: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<Change>> {
        self.get_system_changes(from, to)
    }

    #[cfg(feature = "nova")]
    fn scan_history(&self, mut callback: Box<dyn FnMut(&[Entity]) + Send>) -> Result<()> {
        self.scan_history(|entities| callback(entities))
    }

    #[cfg(feature = "nova")]
    fn scan_relationships(&self, mut callback: Box<dyn FnMut(&[Relationship]) + Send>) -> Result<()> {
        self.scan_relationships(|rels| callback(rels))
    }

    fn insert_relationship(&self, rel: Relationship) -> Result<()> {
        self.insert_relationship(rel)
    }

    fn get_entity_at(
        &self,
        id: EntityId,
        valid_time: chrono::DateTime<chrono::Utc>,
        transaction_time: chrono::DateTime<chrono::Utc>,
    ) -> Result<Option<Entity>> {
        self.get_entity_at(id, valid_time, transaction_time)
    }
}
