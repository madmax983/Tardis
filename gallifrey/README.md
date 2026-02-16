# Tardis Gallifrey 🕰️

> "People assume that time is a strict progression of cause to effect, but *actually* from a non-linear, non-subjective viewpoint - it's more like a big ball of wibbly wobbly... time-y wimey... stuff."

**Gallifrey** is the temporal knowledge store for Tardis OS. Unlike traditional databases that only store the *current* state of the world, Gallifrey tracks history across two dimensions of time, allowing the system to remember not just *what* is true, but *when* it was true and *when* we learned it.

## 🏗️ The Three Stores

Gallifrey manages three specialized storage engines:

1.  **Knowledge Store (`KnowledgeStore`)**:
    -   A graph database for long-term semantic memory.
    -   Stores entities (people, places, concepts) and their relationships.
    -   Supports vector embeddings for semantic search.

2.  **Conversation Store (`ConversationStore`)**:
    -   An append-only log of all interactions with the user.
    -   Maintains context across sessions (`SessionId`).
    -   Enables the system to "remember" past conversations.

3.  **System State Store (`SystemStateStore`)**:
    -   Records snapshots of the OS state (files, processes, configuration).
    -   Enables time-travel debugging and "undo" functionality.

## ⏳ Bi-Temporality Explained

Gallifrey implements **Bi-Temporal Modeling**, tracking two time axes for every piece of data:

1.  **Valid Time**: The time period when a fact was true in the real world.
    -   *Example:* "I lived in London from 2010 to 2015."
2.  **Transaction Time**: The time period when the fact was present in the database.
    -   *Example:* "I told the system about living in London on 2023-10-27."

This allows powerful queries like:
-   *"What is the user's address now?"* (Current Valid, Current Transaction)
-   *"What did we think the user's address was yesterday?"* (Past Transaction, Current Valid)
-   *"What was the user's address in 2010 (according to what we know now)?"* (Past Valid, Current Transaction)

### Append-Only Architecture

To support this, Gallifrey uses an **Append-Only** model. Updates do not overwrite data; they:
1.  Close the `transaction_time` of the old version (making it historical).
2.  Insert a new version with the current `transaction_time`.

## 🚀 Getting Started

```rust,no_run
use tardis_gallifrey::Gallifrey;
use tardis_gallifrey::domain::Entity;
use tardis_common::id::EntityId;
use tardis_common::temporal::BiTemporalInterval;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Initialize the store
    let gallifrey = Gallifrey::new();

    // 2. Create an entity representing a fact
    let entity = Entity {
        id: EntityId::new(),
        entity_type: "Fact".to_string(),
        name: "The Sky".to_string(),
        properties: HashMap::from([
            ("color".to_string(), serde_json::json!("blue"))
        ]),
        embedding: None,
        temporal: BiTemporalInterval::now(),
        source: Some("User Observation".to_string()),
    };

    // 3. Insert it into the Knowledge Store
    let id = gallifrey.insert(entity).await?;

    // 4. Update it (creates a new version, preserving history)
    gallifrey.update(id, serde_json::json!({"color": "dark_blue"})).await?;

    // 5. Retrieve history to see both versions
    let history = gallifrey.get_history(id).await?;
    assert_eq!(history.len(), 2);

    Ok(())
}
```

## 🧪 Experimental Features (Nova)

When compiled with the `nova` feature, Gallifrey exposes experimental modules:

-   **`TimeCapsule`**: For bulk import/export of knowledge subgraphs.
-   **`Timeline`**: Advanced visualization of entity history.

To enable these features:

```toml
[dependencies]
tardis-gallifrey = { version = "0.1.0", features = ["nova"] }
```
