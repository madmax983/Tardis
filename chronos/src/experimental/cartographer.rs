//! The Cartographer: Knowledge Graph Visualization.
//!
//! "Here be dragons."
//!
//! This module projects high-dimensional embeddings into a 2D map using
//! a deterministic random projection (Johnson-Lindenstrauss lemmaish)
//! driven by a simple Linear Congruential Generator (LCG).

use tardis_gallifrey::domain::Entity;

/// A simple Linear Congruential Generator.
///
/// Constants from Knuth (MMIX).
struct Lcg {
    state: u64,
}

impl Lcg {
    const A: u64 = 6_364_136_223_846_793_005;
    const C: u64 = 1_442_695_040_888_963_407;

    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next(&mut self) -> u64 {
        self.state = self.state.wrapping_mul(Self::A).wrapping_add(Self::C);
        self.state
    }

    /// Returns a float in [0.0, 1.0).
    fn next_f32(&mut self) -> f32 {
        // Use top 24 bits for mantissa
        let v = self.next() >> 40;
        (v as f32) * 5.960_464_5e-8 // 1.0 / 2^24
    }

    /// Returns a standard normal distributed float (mean 0, std 1).
    /// Uses Box-Muller transform.
    fn next_gaussian(&mut self) -> f32 {
        let u1 = self.next_f32();
        let u2 = self.next_f32();

        let r = (-2.0 * u1.ln()).sqrt();
        let theta = 2.0 * std::f32::consts::PI * u2;

        r * theta.cos()
    }
}

/// The Cartographer engine.
#[derive(Debug, Default)]
pub struct Cartographer;

impl Cartographer {
    /// Create a new Cartographer.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Project a high-dimensional vector to 2D.
    ///
    /// Uses a deterministic random projection seeded with a constant.
    /// This ensures the map is stable across runs as long as the embeddings are stable.
    fn project(&self, embedding: &[f32]) -> (f32, f32) {
        let mut lcg = Lcg::new(0xCAFE_BABE_DEAD_BEEF);
        let mut x = 0.0;
        let mut y = 0.0;

        for &val in embedding {
            // Project to X axis
            let rx = lcg.next_gaussian();
            x += val * rx;

            // Project to Y axis
            // Note: We need to consume the RNG state consistently.
            // Iterating: val_0 -> (rx0, ry0), val_1 -> (rx1, ry1)...
            let ry = lcg.next_gaussian();
            y += val * ry;
        }

        // Normalize? No, raw projection is fine for relative positioning.
        // The scale depends on sqrt(d), but we auto-scale in render.
        (x, y)
    }

    /// Render entities to an ASCII map.
    #[must_use]
    #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn render(&self, entities: &[Entity], width: usize, height: usize) -> String {
        if entities.is_empty() {
            return "The map is blank. No known lands.".to_string();
        }

        let mut points = Vec::new();
        let mut min_x = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_y = f32::NEG_INFINITY;

        for entity in entities {
            if let Some(emb) = &entity.embedding {
                let (x, y) = self.project(emb);
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

        if points.is_empty() {
            return "No entities have coordinates (embeddings) yet.".to_string();
        }

        // Add some padding
        let range_x = (max_x - min_x).max(1.0);
        let range_y = (max_y - min_y).max(1.0);

        let pad_x = range_x * 0.1;
        let pad_y = range_y * 0.1;

        min_x -= pad_x;
        max_x += pad_x;
        min_y -= pad_y;
        max_y += pad_y;

        let scale_x = (width as f32) / (max_x - min_x);
        let scale_y = (height as f32) / (max_y - min_y);

        let mut grid = vec![vec![' '; width]; height];

        for (x, y, entity) in points {
            let col = ((x - min_x) * scale_x).floor() as isize;
            let row = ((y - min_y) * scale_y).floor() as isize;

            let col = col.clamp(0, (width - 1) as isize) as usize;
            let row = row.clamp(0, (height - 1) as isize) as usize;

            // Pick a symbol
            let ch = match entity.entity_type.as_str() {
                "Person" => '@',
                "Project" => '#',
                "Concept" => '*',
                "System" => '$',
                _ => entity.name.chars().next().unwrap_or('?'),
            };

            grid[row][col] = ch;
        }

        // Draw
        let mut output = String::new();
        output.push_str(&format!("Map of Knowledge ({} entities)\n", entities.len()));
        output.push_str(&"-".repeat(width + 2));
        output.push('\n');

        for row in grid {
            output.push('|');
            for ch in row {
                output.push(ch);
            }
            output.push('|');
            output.push('\n');
        }
        output.push_str(&"-".repeat(width + 2));
        output.push('\n');

        // Legend (Top 5 visible)
        // ... (omitted for brevity, maybe add later)

        output
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tardis_common::id::EntityId;
    use tardis_common::temporal::BiTemporalInterval;

    #[test]
    fn test_lcg_determinism() {
        let mut lcg1 = Lcg::new(12345);
        let mut lcg2 = Lcg::new(12345);

        for _ in 0..100 {
            assert_eq!(lcg1.next(), lcg2.next());
            assert_eq!(lcg1.next_f32(), lcg2.next_f32());
            assert_eq!(lcg1.next_gaussian(), lcg2.next_gaussian());
        }
    }

    #[test]
    fn test_projection_stability() {
        let cart = Cartographer::new();
        let emb = vec![0.1, 0.2, -0.5, 0.9];

        let p1 = cart.project(&emb);
        let p2 = cart.project(&emb);

        assert_eq!(p1, p2);
    }

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
    fn test_render_empty() {
        let cart = Cartographer::new();
        let entities = vec![];
        let map = cart.render(&entities, 10, 5);
        assert!(map.contains("blank"));
    }

    #[test]
    fn test_render_points() {
        let cart = Cartographer::new();
        let e1 = create_test_entity("A", Some(vec![1.0, 0.0]));
        let e2 = create_test_entity("B", Some(vec![-1.0, 0.0]));
        let entities = vec![e1, e2];

        let map = cart.render(&entities, 20, 10);
        assert!(map.contains("Map of Knowledge"));
        assert!(map.contains('|'));
    }
}
