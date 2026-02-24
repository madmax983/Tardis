//! The Cartographer: Semantic Mapper.
//!
//! "I have a map of the universe. It's a bit out of date, but it's got the main bits."
//!
//! This module visualizes the knowledge graph by projecting high-dimensional embeddings
//! (768d) onto a 2D grid using a deterministic random projection (Johnson-Lindenstrauss lemma).
//! It generates an ASCII map of the semantic space.

use crate::error::ChronosResult;
use std::collections::HashMap;
use std::fmt::Write;
use std::sync::Arc;
use tardis_gallifrey::domain::Entity;
use tardis_gallifrey::Gallifrey;

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

    /// Generate a map of the current knowledge graph.
    ///
    /// # Arguments
    ///
    /// * `width` - Width of the map in characters.
    /// * `height` - Height of the map in characters.
    ///
    /// # Errors
    ///
    /// Returns an error if the knowledge graph cannot be scanned or string formatting fails.
    #[allow(clippy::cast_precision_loss)]
    #[allow(clippy::cast_possible_truncation)]
    #[allow(clippy::cast_sign_loss)]
    pub fn map(&self, width: usize, height: usize) -> ChronosResult<String> {
        // Store entity and its embedding
        let mut entities: Vec<(Entity, Vec<f32>)> = Vec::new();

        // Scan all entities
        self.gallifrey
            .knowledge()
            .scan_history(|history| {
                // Find current version
                if let Some(e) = history.iter().find(|e| e.temporal.is_current()) {
                    if let Some(emb) = &e.embedding {
                        entities.push((e.clone(), emb.clone()));
                    }
                }
            })
            .map_err(|e| crate::error::ChronosError::Common(e.into()))?;

        if entities.is_empty() {
            return Ok("The universe is dark. No knowledge found.".to_string());
        }

        // Project to 2D
        let points: Vec<(Entity, f32, f32)> = entities
            .into_iter()
            .map(|(e, emb)| {
                let (x, y) = Self::project(&emb);
                (e, x, y)
            })
            .collect();

        // Normalize coordinates to grid
        let (min_x, max_x) = points
            .iter()
            .fold((f32::INFINITY, f32::NEG_INFINITY), |(min, max), (_, x, _)| {
                (min.min(*x), max.max(*x))
            });
        let (min_y, max_y) = points
            .iter()
            .fold((f32::INFINITY, f32::NEG_INFINITY), |(min, max), (_, _, y)| {
                (min.min(*y), max.max(*y))
            });

        let range_x = (max_x - min_x).max(1e-6);
        let range_y = (max_y - min_y).max(1e-6);

        // Render grid
        let mut grid = vec![vec![' '; width]; height];
        let mut labels: HashMap<(usize, usize), Vec<String>> = HashMap::new();

        for (entity, x, y) in points {
            let grid_x = (((x - min_x) / range_x) * (width as f32 - 1.0)).round() as usize;
            let grid_y = (((y - min_y) / range_y) * (height as f32 - 1.0)).round() as usize;

            if grid_x < width && grid_y < height {
                grid[grid_y][grid_x] = '*';
                labels.entry((grid_x, grid_y)).or_default().push(entity.name);
            }
        }

        // Build the string
        let mut output = String::new();
        let border_line = "─".repeat(width);
        writeln!(output, "┌{border_line}┐")
            .map_err(|_| crate::error::ChronosError::Common(tardis_common::Error::Internal("Fmt error".into())))?;

        for (y, row) in grid.iter().enumerate() {
            output.push('│');
            for (x, &ch) in row.iter().enumerate() {
                // Check if we should draw a label marker instead
                if let Some(names) = labels.get(&(x, y)) {
                    if names.len() > 1 {
                        output.push('+'); // Cluster
                    } else {
                        output.push(names[0].chars().next().unwrap_or('?')); // First letter
                    }
                } else {
                    output.push(ch);
                }
            }
            output.push_str("│\n");
        }
        writeln!(output, "└{border_line}┘")
             .map_err(|_| crate::error::ChronosError::Common(tardis_common::Error::Internal("Fmt error".into())))?;

        // Legend for clusters/points
        output.push_str("\nLegend:\n");
        let mut count = 0;
        // Sort keys for deterministic output
        let mut keys: Vec<_> = labels.keys().collect();
        keys.sort_unstable();

        for key in keys {
            if count >= 10 {
                output.push_str("... (more)\n");
                break;
            }
            if let Some(names) = labels.get(key) {
                if !names.is_empty() {
                    let symbol = if names.len() > 1 { "+" } else { &names[0][0..1] };
                    let (x, y) = key;
                    writeln!(output, "{symbol} ({x}, {y}): {}", names.join(", "))
                        .map_err(|_| crate::error::ChronosError::Common(tardis_common::Error::Internal("Fmt error".into())))?;
                    count += 1;
                }
            }
        }

        Ok(output)
    }

    /// Project 768d embedding to 2D using deterministic random projection.
    fn project(embedding: &[f32]) -> (f32, f32) {
        let mut x = 0.0;
        let mut y = 0.0;

        // Use a simple LCG to generate weights deterministically
        // Seed 1 for X axis, Seed 2 for Y axis
        let mut rng_x = Lcg::new(1);
        let mut rng_y = Lcg::new(2);

        for &val in embedding {
            // Weights in range [-1.0, 1.0]
            let w_x = rng_x.next_f32() * 2.0 - 1.0;
            let w_y = rng_y.next_f32() * 2.0 - 1.0;

            x += val * w_x;
            y += val * w_y;
        }

        (x, y)
    }
}

/// Linear Congruential Generator for deterministic pseudo-random numbers.
struct Lcg {
    state: u64,
}

impl Lcg {
    const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    const fn next(&mut self) -> u64 {
        // Knuth's constants
        self.state = self.state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        self.state
    }

    #[allow(clippy::cast_precision_loss)]
    fn next_f32(&mut self) -> f32 {
        // Use upper 32 bits
        let val = (self.next() >> 32) as u32;
        // Normalize to [0.0, 1.0)
        val as f32 / u32::MAX as f32
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tardis_common::temporal::BiTemporalInterval;
    use tardis_common::id::EntityId;

    fn create_test_entity(name: &str, embedding: Option<Vec<f32>>) -> Entity {
        Entity {
            id: EntityId::new(),
            entity_type: "Test".to_string(),
            name: name.to_string(),
            properties: HashMap::new(),
            embedding,
            temporal: BiTemporalInterval::now(),
            source: None,
        }
    }

    #[test]
    fn test_lcg_deterministic() {
        let mut rng1 = Lcg::new(42);
        let mut rng2 = Lcg::new(42);

        assert_eq!(rng1.next(), rng2.next());
        assert_eq!(rng1.next_f32(), rng2.next_f32());
    }

    #[tokio::test]
    async fn test_cartographer_map() {
        let gallifrey = Arc::new(Gallifrey::new());

        // Add entities
        let e1 = create_test_entity("Alpha", Some(vec![1.0; 768]));
        let e2 = create_test_entity("Beta", Some(vec![-1.0; 768]));
        let e3 = create_test_entity("Gamma", None); // Should be ignored

        gallifrey.insert(e1).await.unwrap();
        gallifrey.insert(e2).await.unwrap();
        gallifrey.insert(e3).await.unwrap();

        let cartographer = Cartographer::new(gallifrey);
        let map = cartographer.map(20, 10).unwrap();

        println!("{}", map);
        assert!(map.contains("Alpha"));
        assert!(map.contains("Beta"));
        assert!(!map.contains("Gamma"));
        assert!(map.contains("Legend:"));
    }
}
