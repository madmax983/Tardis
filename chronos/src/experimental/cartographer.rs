//! The Cartographer 🗺️
//!
//! A high-dimensional projection engine that maps the Knowledge Graph's embedding space
//! into a 2D ASCII visualization.
//!
//! "Where we're going, we don't need roads... we need a map."

use crate::error::{ChronosError, ChronosResult};
use std::fmt::Write;
use std::sync::Arc;
use tardis_gallifrey::domain::Entity;
use tardis_gallifrey::Gallifrey;

/// A point on the map.
#[derive(Debug, Clone)]
struct Point {
    x: f32,
    y: f32,
    label: char,
    #[allow(dead_code)] // Used for future features (e.g. tooltips)
    entity: Entity,
}

/// The Cartographer engine.
#[derive(Debug)]
pub struct Cartographer {
    gallifrey: Arc<Gallifrey>,
}

impl Cartographer {
    /// Create a new Cartographer.
    #[must_use]
    pub const fn new(gallifrey: Arc<Gallifrey>) -> Self {
        Self { gallifrey }
    }

    /// generate an ASCII map of the knowledge base.
    ///
    /// # Errors
    ///
    /// Returns an error if the process fails.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    pub fn map(&self, _query: Option<&str>) -> ChronosResult<String> {
        let mut points = Vec::new();

        // 1. Scan entities and project them
        self.gallifrey
            .knowledge()
            .scan_history(|history| {
                // Filter for current entities only
                // Just take the first current version found (simulating current snapshot)
                if let Some(entity) = history.iter().find(|e| e.temporal.is_current()) {
                    if let Some(emb) = &entity.embedding {
                        let (x, y) = project(emb);
                        points.push(Point {
                            x,
                            y,
                            label: get_label(&entity.entity_type),
                            entity: entity.clone(),
                        });
                    }
                }
            })
            .map_err(|e| ChronosError::Common(e.into()))?;

        if points.is_empty() {
            return Ok("The void is empty. No knowledge found with embeddings.".to_string());
        }

        // 2. Normalize coordinates to fit terminal (60x20 approx)
        let width = 60;
        let height = 20;

        let (min_x, max_x) = points
            .iter()
            .fold((f32::INFINITY, f32::NEG_INFINITY), |(min, max), p| {
                (min.min(p.x), max.max(p.x))
            });
        let (min_y, max_y) = points
            .iter()
            .fold((f32::INFINITY, f32::NEG_INFINITY), |(min, max), p| {
                (min.min(p.y), max.max(p.y))
            });

        let range_x = (max_x - min_x).max(0.0001);
        let range_y = (max_y - min_y).max(0.0001);

        // Grid: 2D array of char
        let mut grid = vec![vec![' '; width]; height];

        for p in &points {
            // Normalize to [0, 1] then scale to width/height
            // Invert Y for typical screen coords (top is 0)
            let nx = ((p.x - min_x) / range_x * (width as f32 - 1.0)).round() as usize;
            let ny = ((p.y - min_y) / range_y * (height as f32 - 1.0)).round() as usize;

            // Bounds check just in case
            let nx = nx.min(width - 1);
            let ny = ny.min(height - 1);

            // Simple collision handling: overwrite
            grid[ny][nx] = p.label;
        }

        // 3. Render
        let mut output = String::new();
        output.push('┌');
        output.push_str(&"─".repeat(width));
        output.push_str("┐\n");

        for row in grid {
            output.push('│');
            for cell in row {
                output.push(cell);
            }
            output.push('│');
            output.push('\n');
        }

        output.push('└');
        output.push_str(&"─".repeat(width));
        output.push_str("┘\n");
        output.push_str("Legend: @=Person #=System *=Fact ?=Other\n");
        // We use writeln! to avoid clippy::format_push_string
        let _ = writeln!(output, "Mapped {} entities.", points.len());

        Ok(output)
    }
}

fn get_label(entity_type: &str) -> char {
    match entity_type.to_lowercase().as_str() {
        "person" | "user" => '@',
        "system" | "service" | "component" => '#',
        "fact" | "concept" | "idea" => '*',
        _ => '?',
    }
}

/// Project high-dimensional vector to 2D using deterministic random projection.
fn project(embedding: &[f32]) -> (f32, f32) {
    // We use a fixed seed for stability.
    // X axis projection vector
    let mut rng_x = Lcg::new(12_345);
    let x: f32 = embedding
        .iter()
        .map(|&v| v * rng_x.next_f32_centered())
        .sum();

    // Y axis projection vector
    let mut rng_y = Lcg::new(67_890);
    let y: f32 = embedding
        .iter()
        .map(|&v| v * rng_y.next_f32_centered())
        .sum();

    (x, y)
}

/// A simple Linear Congruential Generator for deterministic random numbers.
struct Lcg {
    state: u64,
}

impl Lcg {
    const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Returns a float in range [-1.0, 1.0]
    #[allow(clippy::cast_precision_loss)]
    fn next_f32_centered(&mut self) -> f32 {
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        let val = (self.state >> 32) as f32 / 4_294_967_296.0; // [0.0, 1.0]
        val * 2.0 - 1.0
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp, clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tardis_common::id::EntityId;
    use tardis_common::temporal::BiTemporalInterval;

    #[test]
    fn test_lcg_deterministic() {
        let mut rng1 = Lcg::new(123);
        let mut rng2 = Lcg::new(123);
        assert_eq!(rng1.next_f32_centered(), rng2.next_f32_centered());
    }

    #[test]
    fn test_projection_stability() {
        let emb = vec![0.1, 0.2, 0.3, 0.4];
        let p1 = project(&emb);
        let p2 = project(&emb);
        assert_eq!(p1, p2);
    }

    #[tokio::test]
    async fn test_cartographer_map_generation() {
        let gallifrey = Arc::new(Gallifrey::new());
        let cartographer = Cartographer::new(gallifrey.clone());

        // Insert some test data
        let e1 = Entity {
            id: EntityId::new(),
            entity_type: "Person".to_string(),
            name: "Nova".to_string(),
            properties: HashMap::new(),
            embedding: Some(vec![1.0, 0.0, 0.0]),
            temporal: BiTemporalInterval::now(),
            source: None,
        };
        gallifrey.insert(e1).await.unwrap();

        let e2 = Entity {
            id: EntityId::new(),
            entity_type: "System".to_string(),
            name: "Kernel".to_string(),
            properties: HashMap::new(),
            embedding: Some(vec![0.0, 1.0, 0.0]),
            temporal: BiTemporalInterval::now(),
            source: None,
        };
        gallifrey.insert(e2).await.unwrap();

        let map = cartographer.map(None).unwrap();
        assert!(map.contains("Legend:"));
        assert!(map.contains('@')); // Person
        assert!(map.contains('#')); // System
        assert!(map.contains("Mapped 2 entities"));
    }
}
