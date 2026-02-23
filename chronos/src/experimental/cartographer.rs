//! The Cartographer: Knowledge Graph Visualizer.
//!
//! "A map of the world that does not include Utopia is not worth glancing at."
//!
//! This module projects high-dimensional entity embeddings onto a 2D plane
//! to create a visual map of the semantic space.

use crate::error::{ChronosError, ChronosResult};
use std::sync::Arc;
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

    /// Generate a 2D map of the knowledge graph.
    ///
    /// # Errors
    ///
    /// Returns an error if the graph cannot be scanned.
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    pub fn map(&self, width: usize, height: usize) -> ChronosResult<String> {
        use std::fmt::Write;

        // Enforce minimum dimensions
        let width = width.max(10);
        let height = height.max(5);

        let knowledge = self.gallifrey.knowledge();
        let mut entities = Vec::new();

        // Scan for current entities with embeddings
        knowledge
            .scan_history(|history| {
                if let Some(e) = history
                    .iter()
                    .find(|e| e.temporal.is_current() && e.embedding.is_some())
                {
                    entities.push(e.clone());
                }
            })
            .map_err(|e| ChronosError::Common(e.into()))?;

        if entities.is_empty() {
            return Ok("The void is empty. No embedded entities found.".to_string());
        }

        // Generate basis vectors for projection
        // We use a deterministic seed so the map is stable.
        // We use the dimension of the first entity found.
        let dim = entities
            .first()
            .and_then(|e| e.embedding.as_ref())
            .map_or(768, Vec::len);

        let basis_x = Self::generate_basis_vector(dim, 1337);
        let basis_y = Self::generate_basis_vector(dim, 42069);

        // Project entities
        let mut points = Vec::new();
        let mut min_x = f32::MAX;
        let mut max_x = f32::MIN;
        let mut min_y = f32::MAX;
        let mut max_y = f32::MIN;

        for entity in &entities {
            if let Some(emb) = &entity.embedding {
                let x = Self::dot_product(emb, &basis_x);
                let y = Self::dot_product(emb, &basis_y);

                points.push((x, y, entity));

                if x < min_x {
                    min_x = x;
                }
                if x > max_x {
                    max_x = x;
                }
                if y < min_y {
                    min_y = y;
                }
                if y > max_y {
                    max_y = y;
                }
            }
        }

        // Avoid division by zero if all points are the same
        if (max_x - min_x).abs() < f32::EPSILON {
            max_x += 1.0;
        }
        if (max_y - min_y).abs() < f32::EPSILON {
            max_y += 1.0;
        }

        // Create grid
        let mut grid = vec![vec![' '; width]; height];
        let mut labels = Vec::new();

        for (i, (x, y, entity)) in points.iter().enumerate() {
            // Normalize to [0, 1] then scale to [0, width-1]
            let norm_x = (x - min_x) / (max_x - min_x);
            let norm_y = (y - min_y) / (max_y - min_y);

            let grid_x = (norm_x * (width as f32 - 1.0)).round() as usize;
            let grid_y = (norm_y * (height as f32 - 1.0)).round() as usize;

            // Use a character to represent the entity
            // We'll use A, B, C... then digits, then symbols
            let symbol = if i < 26 {
                (b'A' + i as u8) as char
            } else if i < 36 {
                (b'0' + (i - 26) as u8) as char
            } else {
                '*'
            };

            grid[grid_y][grid_x] = symbol;
            labels.push(format!(
                "{symbol}: {} ({})",
                entity.name, entity.entity_type
            ));
        }

        // Render to string
        let mut output = String::new();
        let _ = writeln!(output, "🗺️  Semantic Atlas");
        let _ = writeln!(output, "{}", "=".repeat(width));

        for row in grid {
            output.push('|');
            for cell in row {
                output.push(cell);
            }
            output.push_str("|\n");
        }
        let _ = writeln!(output, "{}", "=".repeat(width));
        let _ = writeln!(output, "Legend:");
        for label in labels {
            let _ = writeln!(output, "  {label}");
        }

        Ok(output)
    }

    fn generate_basis_vector(dim: usize, seed: u64) -> Vec<f32> {
        let mut rng = Lcg::new(seed);
        (0..dim).map(|_| rng.next_f32()).collect()
    }

    fn dot_product(a: &[f32], b: &[f32]) -> f32 {
        a.iter().zip(b).map(|(x, y)| x * y).sum()
    }
}

/// A simple Linear Congruential Generator for deterministic "randomness".
struct Lcg {
    state: u64,
}

impl Lcg {
    const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    const fn next(&mut self) -> u64 {
        // Constants from Knuth/MMIX
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.state
    }

    #[allow(clippy::cast_precision_loss)]
    fn next_f32(&mut self) -> f32 {
        // Map u64 to [-1.0, 1.0]
        let v = self.next();
        // Use top 24 bits for f32 mantissa
        let v_small = (v >> 40) as f32;
        let max = (1 << 24) as f32;
        (v_small / max) * 2.0 - 1.0
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tardis_common::id::EntityId;
    use tardis_common::temporal::BiTemporalInterval;
    use tardis_gallifrey::domain::Entity;

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

    #[tokio::test]
    async fn test_cartographer_map() {
        let gallifrey = Arc::new(Gallifrey::new());
        let cartographer = Cartographer::new(gallifrey.clone());

        // Create entities with dummy embeddings
        // We assume 768 dimensions, but our dot product works on any size.
        // Let's use 3 dimensions for simplicity in test, Cartographer generates 768 dim basis.
        // Wait, dot product requires matching length.
        // So we must use 768 dim embeddings if we hardcode 768 in generate_basis_vector.
        // Or we can make generate_basis_vector adapt to embedding size?
        // But in real system embeddings are fixed size (768 or 1536).
        // Let's mock 768-dim embeddings.

        let vec1 = vec![0.1; 768];
        let vec2 = vec![-0.1; 768];
        let vec3 = vec![0.0; 768];

        let e1 = create_test_entity("Alpha", Some(vec1));
        let e2 = create_test_entity("Beta", Some(vec2));
        let e3 = create_test_entity("Gamma", Some(vec3));

        gallifrey.insert(e1).await.unwrap();
        gallifrey.insert(e2).await.unwrap();
        gallifrey.insert(e3).await.unwrap();

        let map = cartographer.map(40, 10).unwrap();
        println!("{map}");

        assert!(map.contains("Semantic Atlas"));
        assert!(map.contains("Alpha"));
        assert!(map.contains("Beta"));
        assert!(map.contains("Gamma"));
    }
}
