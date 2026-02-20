//! Heatmap command for Tardis Shell.
//!
//! Visualizes the temporal history of an entity.

use anyhow::{anyhow, Result};
use std::fmt::Write;
use std::sync::Arc;
use tardis_gallifrey::domain::Entity;
use tardis_gallifrey::experimental::entropy::EntropyGauge;
use tardis_gallifrey::experimental::heatmap::TemporalHeatmap;
use tardis_gallifrey::Gallifrey;

/// Run the heatmap command.
///
/// # Errors
///
/// Returns an error if the entity is not found or arguments are invalid.
pub fn run(gallifrey: &Arc<Gallifrey>, args: &[String]) -> Result<String> {
    if args.is_empty() {
        return Ok("Usage: heatmap <entity_name>".to_string());
    }
    let name = &args[0];
    let knowledge = gallifrey.knowledge();

    let mut found_history: Option<Vec<Entity>> = None;

    // Scan to find the entity history by name
    knowledge.scan_history(|history| {
        if found_history.is_some() {
            return;
        }
        // Check if any version of this entity matches the name
        if history.iter().any(|e| e.name == *name) {
            found_history = Some(history.to_vec());
        }
    })?;

    if let Some(history) = found_history {
        // Create heatmap (20 bins X, 10 bins Y)
        let heatmap = TemporalHeatmap::new(&history, 40, 15);
        let entropy = EntropyGauge::measure(&history);

        let mut report = String::new();
        writeln!(report, "🔥 Temporal Heatmap: {name}")?;
        writeln!(report, "========================================")?;

        // Render ASCII heatmap
        write!(report, "{}", heatmap.render_ascii())?;

        writeln!(report, "\n📉 System Entropy Stats:")?;
        writeln!(report, "------------------------")?;
        writeln!(report, "Total Versions: {}", history.len())?;
        writeln!(report, "Stability Score: {:.2}", entropy.stability_score)?;
        writeln!(report, "Retcons (Late): {}", entropy.retcon_count)?;
        writeln!(report, "Prophecies (Early): {}", entropy.prophecy_count)?;
        writeln!(report, "Syncs (On Time): {}", entropy.sync_count)?;
        writeln!(report, "Avg Drift: {:.2} ms", entropy.average_drift_ms)?;

        Ok(report)
    } else {
        Err(anyhow!("Entity '{name}' not found."))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tardis_common::id::EntityId;
    use tardis_common::temporal::BiTemporalInterval;

    fn create_test_entity(name: &str) -> Entity {
        Entity {
            id: EntityId::new(),
            entity_type: "Test".to_string(),
            name: name.to_string(),
            properties: HashMap::new(),
            embedding: None,
            temporal: BiTemporalInterval::now(),
            source: None,
        }
    }

    #[tokio::test]
    async fn test_heatmap_run() {
        let gallifrey = Arc::new(Gallifrey::new());
        let entity = create_test_entity("TestEntity");
        gallifrey.insert(entity).await.unwrap();

        let args = vec!["TestEntity".to_string()];
        // run is now synchronous
        let result = run(&gallifrey, &args);

        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("Temporal Heatmap: TestEntity"));
        assert!(output.contains("Stability Score"));
    }

    #[tokio::test]
    async fn test_heatmap_not_found() {
        let gallifrey = Arc::new(Gallifrey::new());
        let args = vec!["NonExistent".to_string()];
        let result = run(&gallifrey, &args);

        assert!(result.is_err());
    }
}
