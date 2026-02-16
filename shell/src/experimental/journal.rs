//! The Time Lord's Journal 📖
//!
//! "A diary of a madman in a blue box."
//!
//! A persistent log for user thoughts, which are analyzed by Vortex
//! to extract sentiment and tags, linking them into the Knowledge Graph.

use anyhow::{Context, Result};
use chrono::Utc;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tardis_common::id::EntityId;
use tardis_gallifrey::domain::Entity;
use tardis_gallifrey::Gallifrey;
use tardis_vortex::{InferenceParams, Vortex};

/// A journal for recording thoughts and reflections.
#[derive(Debug)]
pub struct Journal {
    gallifrey: Arc<Gallifrey>,
    vortex: Arc<Vortex>,
}

impl Journal {
    /// Create a new Journal.
    #[must_use]
    pub fn new(gallifrey: Arc<Gallifrey>, vortex: Arc<Vortex>) -> Self {
        Self { gallifrey, vortex }
    }

    /// Log a new entry.
    ///
    /// # Errors
    ///
    /// Returns an error if the entry cannot be stored or analyzed.
    pub async fn log(&self, content: &str) -> Result<EntityId> {
        let now = Utc::now();
        let mut tags = Vec::new();
        let mut sentiment = "neutral".to_string();

        // 1. Analyze with Vortex (if model loaded)
        // We check for loaded models first.
        let loaded_models = self.vortex.list_loaded_models();
        if let Some((handle, _)) = loaded_models.first() {
            let prompt = format!(
                "Analyze the following journal entry. \
                Return a JSON object with 'tags' (array of strings) and 'sentiment' (string: positive, negative, neutral).\n\n\
                Entry: \"{content}\"\n\n\
                JSON:"
            );

            let params = InferenceParams {
                max_tokens: 100,
                temperature: 0.3,
                ..InferenceParams::default()
            };

            if let Ok(response) = self.vortex.infer(*handle, &prompt, params).await {
                // Try to parse JSON from response
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&response) {
                    if let Some(t) = json["tags"].as_array() {
                        tags = t
                            .iter()
                            .filter_map(|v| v.as_str().map(ToString::to_string))
                            .collect();
                    }
                    if let Some(s) = json["sentiment"].as_str() {
                        sentiment = s.to_string();
                    }
                }
            }
        }

        // 2. Create Entity
        let entity = Entity {
            id: EntityId::new(),
            entity_type: "JournalEntry".to_string(),
            name: format!("Journal Entry {}", now.format("%Y-%m-%d %H:%M")),
            properties: {
                let mut props = HashMap::new();
                props.insert("content".to_string(), json!(content));
                props.insert("tags".to_string(), json!(tags));
                props.insert("sentiment".to_string(), json!(sentiment));
                props.insert("timestamp".to_string(), json!(now.to_rfc3339()));
                props
            },
            embedding: None, // TODO: Generate embedding
            temporal: tardis_common::temporal::BiTemporalInterval::now(),
            source: Some("User".to_string()),
        };

        // 3. Store
        let id = self
            .gallifrey
            .insert(entity)
            .await
            .context("Failed to store journal entry")?;

        Ok(id)
    }

    /// Read recent journal entries.
    ///
    /// # Errors
    ///
    /// Returns an error if retrieval fails.
    pub fn read(&self, limit: usize) -> Result<Vec<String>> {
        // We use find_by_type since we don't have a specific query language yet
        let entities = self
            .gallifrey
            .knowledge()
            .find_by_type("JournalEntry")
            .map_err(|e| anyhow::anyhow!("Failed to retrieve entries: {e}"))?;

        // Sort by valid time (descending)
        let mut sorted_entities = entities;
        sorted_entities.sort_by(|a, b| b.temporal.valid_time.start.cmp(&a.temporal.valid_time.start));

        let entries: Vec<String> = sorted_entities
            .into_iter()
            .take(limit)
            .map(|e| {
                let content = e
                    .properties
                    .get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("[No Content]");
                let tags = e
                    .properties
                    .get("tags")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    })
                    .unwrap_or_default();
                let sentiment = e
                    .properties
                    .get("sentiment")
                    .and_then(|v| v.as_str())
                    .unwrap_or("neutral");
                let time = e.temporal.valid_time.start.format("%Y-%m-%d %H:%M");

                format!(
                    "[{time}] {content}\n    Tags: [{tags}] | Sentiment: {sentiment}"
                )
            })
            .collect();

        Ok(entries)
    }

    /// Reflect on recent entries.
    ///
    /// # Errors
    ///
    /// Returns an error if retrieval or inference fails.
    pub async fn reflect(&self, limit: usize) -> Result<String> {
        let entries = self.read(limit)?;

        if entries.is_empty() {
            return Ok("The journal is empty. There is nothing to reflect upon.".to_string());
        }

        let combined_entries = entries.join("\n\n");

        let loaded_models = self.vortex.list_loaded_models();
        let (handle, _) = loaded_models
            .first()
            .ok_or_else(|| anyhow::anyhow!("No AI model loaded for reflection."))?;

        let prompt = format!(
            "You are a philosophical AI assistant. Read the user's recent journal entries and offer a deep, reflective insight about their state of mind and patterns.\n\n\
            Entries:\n{combined_entries}\n\n\
            Reflection:"
        );

        let reflection = self
            .vortex
            .infer(
                *handle,
                &prompt,
                InferenceParams {
                    max_tokens: 300,
                    temperature: 0.7,
                    ..InferenceParams::default()
                },
            )
            .await
            .context("Failed to generate reflection")?;

        Ok(reflection)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_journal_cycle() {
        let gallifrey = Arc::new(Gallifrey::new());
        let vortex = Arc::new(Vortex::new().unwrap());

        // Mock Vortex
        vortex.set_mock_inference(Box::new(|_, prompt, _| {
            if prompt.contains("JSON") {
                Ok(r#"{"tags": ["test", "journal"], "sentiment": "positive"}"#.to_string())
            } else {
                Ok("This is a profound reflection.".to_string())
            }
        }));

        // Register a mock model handle so list_loaded_models returns something
        let _ = vortex.register_mock_model("dummy").unwrap();


        let journal = Journal::new(gallifrey.clone(), vortex.clone());

        // Log
        let id = journal.log("Today was a good day.").await.unwrap();
        // Check if stored
        let history = gallifrey.get_history(id).await.unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].entity_type, "JournalEntry");
        assert_eq!(history[0].properties["sentiment"], "positive");

        // Read
        let entries = journal.read(5).unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].contains("Today was a good day"));
        assert!(entries[0].contains("Tags: [test, journal]"));

        // Reflect
        let reflection = journal.reflect(5).await.unwrap();
        assert_eq!(reflection, "This is a profound reflection.");
    }
}
