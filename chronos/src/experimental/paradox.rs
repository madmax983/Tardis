//! The Paradox Engine: Time Anomaly Detection.
//!
//! "History can be rewritten, but the ink never dries."
//!
//! This module analyzes the timeline for logical inconsistencies, identity drift,
//! and causal loops.

use crate::error::{ChronosError, ChronosResult};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tardis_common::id::{EntityId, ModelHandle};
use tardis_gallifrey::{domain::Entity, Gallifrey};
use tardis_vortex::{InferenceParams, Vortex};
use tracing::instrument;

/// Type of temporal paradox.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParadoxType {
    /// The Ship of Theseus: Entity has changed so much it's no longer the same.
    IdentityDrift,
    /// Effect precedes Cause: Information appears before it was created.
    Retrocausality,
    /// Zombie Process: Entity is active but has no recent history/updates.
    Zombie,
    /// Temporal Loop: Circular reference in causality (hard to detect, placeholder).
    Loop,
}

/// A report of a detected paradox.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParadoxReport {
    /// The entity involved.
    pub entity_id: EntityId,
    /// The type of paradox.
    pub paradox_type: ParadoxType,
    /// Severity (0.0 to 1.0).
    pub severity: f32,
    /// Technical description.
    pub description: String,
    /// Narrative explanation (from Vortex).
    pub narrative: Option<String>,
}

/// The Paradox Engine.
#[derive(Debug)]
pub struct ParadoxEngine {
    gallifrey: Arc<Gallifrey>,
    vortex: Option<Arc<Vortex>>,
    model: Option<ModelHandle>,
}

impl ParadoxEngine {
    /// Create a new Paradox Engine.
    #[must_use]
    pub fn new(
        gallifrey: Arc<Gallifrey>,
        vortex: Option<Arc<Vortex>>,
        model: Option<ModelHandle>,
    ) -> Self {
        Self {
            gallifrey,
            vortex,
            model,
        }
    }

    /// Analyze an entity for paradoxes.
    #[instrument(skip(self))]
    pub async fn analyze(&self, entity_id: EntityId) -> ChronosResult<Vec<ParadoxReport>> {
        let history = self
            .gallifrey
            .get_history(entity_id)
            .await
            .map_err(|e| ChronosError::Common(tardis_common::Error::Internal(e.to_string())))?;

        if history.is_empty() {
            return Ok(Vec::new());
        }

        let mut reports = Vec::new();

        // 1. Check Identity Drift (Ship of Theseus)
        if let Some(drift) = self.detect_drift(&history) {
            reports.push(drift);
        }

        // 2. Check Retrocausality
        if let Some(retro) = self.detect_retrocausality(&history) {
            reports.push(retro);
        }

        // 3. Explain with Vortex if available
        if let (Some(vortex), Some(model)) = (&self.vortex, self.model) {
            for report in &mut reports {
                if let Ok(explanation) = self.explain(vortex, model, report).await {
                    report.narrative = Some(explanation);
                }
            }
        }

        Ok(reports)
    }

    fn detect_drift(&self, history: &[Entity]) -> Option<ParadoxReport> {
        // Need at least 2 versions to compare
        if history.len() < 2 {
            return None;
        }

        let first = history.first()?;
        let last = history.last()?;

        let emb1 = first.embedding.as_ref()?;
        let emb2 = last.embedding.as_ref()?;

        let similarity = cosine_similarity(emb1, emb2);

        // Threshold for "This is not the same ship"
        // If similarity < 0.6, it's drifting.
        if similarity < 0.6 {
            Some(ParadoxReport {
                entity_id: first.id,
                paradox_type: ParadoxType::IdentityDrift,
                severity: 1.0 - similarity,
                description: format!(
                    "Identity drift detected. Similarity between initial and current version is {:.2}.",
                    similarity
                ),
                narrative: None,
            })
        } else {
            None
        }
    }

    fn detect_retrocausality(&self, history: &[Entity]) -> Option<ParadoxReport> {
        for entity in history {
            // Check for "Orphaned" entities (no source).
            // This is a weak check but demonstrates the concept.
            if entity.source.is_none() {
                 return Some(ParadoxReport {
                    entity_id: entity.id,
                    paradox_type: ParadoxType::Retrocausality,
                    severity: 0.3,
                    description: "Entity has no source origin. It exists without cause.".to_string(),
                    narrative: None,
                });
            }

            // Check if valid_time is unreasonably far in the past compared to transaction_time?
            // Gallifrey doesn't forbid this (it's history insertion).
            // But if transaction_time is NOW, and valid_time is FUTURE, it's a prophecy.
            if entity.temporal.valid_time.start > entity.temporal.transaction_time.start + chrono::Duration::days(1) {
                return Some(ParadoxReport {
                    entity_id: entity.id,
                    paradox_type: ParadoxType::Retrocausality,
                    severity: 0.5,
                    description: "Entity validity starts in the future relative to when it was recorded.".to_string(),
                    narrative: None,
                });
            }
        }
        None
    }

    async fn explain(
        &self,
        vortex: &Vortex,
        model: ModelHandle,
        report: &ParadoxReport,
    ) -> ChronosResult<String> {
        let prompt = format!(
            "Analyze this temporal paradox:\nType: {:?}\nDescription: {}\n\nExplain why this might happen in a time-travel operating system. Be philosophical.",
            report.paradox_type, report.description
        );

        let params = InferenceParams {
            max_tokens: 150,
            temperature: 0.8,
            ..Default::default()
        };

        vortex
            .infer(model, &prompt, params)
            .await
            .map_err(|e| ChronosError::Common(tardis_common::Error::Internal(e.to_string())))
    }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot_product: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot_product / (norm_a * norm_b)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use tardis_common::temporal::BiTemporalInterval;
    use std::collections::HashMap;

    #[test]
    fn test_identity_drift_detection() {
        // We can test detect_drift directly by creating an instance with mocked Gallifrey (or just ignoring it since detect_drift doesn't use it)
        // But detect_drift is private. We'll use a wrapper or just trust the compiler?
        // No, we should test it.
        // We can recreate the logic or make a testable harness.
        // Or we can just invoke `detect_drift` if we put the test in the same module (which we are).

        let gallifrey = Arc::new(Gallifrey::new());
        let engine = ParadoxEngine::new(gallifrey, None, None);
        let id = EntityId::new();

        let e1 = Entity {
            id,
            entity_type: "Ship".to_string(),
            name: "Theseus".to_string(),
            properties: HashMap::new(),
            embedding: Some(vec![1.0, 0.0, 0.0]),
            temporal: BiTemporalInterval::now(),
            source: Some("Source".to_string()),
        };

        let e2 = Entity {
            id,
            entity_type: "Ship".to_string(),
            name: "Theseus 2".to_string(),
            properties: HashMap::new(),
            embedding: Some(vec![0.0, 1.0, 0.0]), // Orthogonal! Sim = 0.0
            temporal: BiTemporalInterval::now(),
            source: Some("Source".to_string()),
        };

        let history = vec![e1, e2];
        let report = engine.detect_drift(&history).expect("Should detect drift");

        assert_eq!(report.paradox_type, ParadoxType::IdentityDrift);
        assert!(report.severity > 0.9);
    }

    #[test]
    fn test_no_drift() {
        let gallifrey = Arc::new(Gallifrey::new());
        let engine = ParadoxEngine::new(gallifrey, None, None);
        let id = EntityId::new();

        let e1 = Entity {
            id,
            entity_type: "Ship".to_string(),
            name: "Theseus".to_string(),
            properties: HashMap::new(),
            embedding: Some(vec![1.0, 0.0, 0.0]),
            temporal: BiTemporalInterval::now(),
            source: Some("Source".to_string()),
        };

        let e2 = Entity {
            id,
            entity_type: "Ship".to_string(),
            name: "Theseus".to_string(),
            properties: HashMap::new(),
            embedding: Some(vec![0.9, 0.1, 0.0]), // Very similar
            temporal: BiTemporalInterval::now(),
            source: Some("Source".to_string()),
        };

        let history = vec![e1, e2];
        assert!(engine.detect_drift(&history).is_none());
    }

    #[test]
    fn test_retrocausality_orphan() {
        let gallifrey = Arc::new(Gallifrey::new());
        let engine = ParadoxEngine::new(gallifrey, None, None);
        let id = EntityId::new();

        let e1 = Entity {
            id,
            entity_type: "Mystery".to_string(),
            name: "Unknown".to_string(),
            properties: HashMap::new(),
            embedding: None,
            temporal: BiTemporalInterval::now(),
            source: None, // Orphan
        };

        let history = vec![e1];
        let report = engine.detect_retrocausality(&history).expect("Should detect orphan");
        assert_eq!(report.paradox_type, ParadoxType::Retrocausality);
    }
}
