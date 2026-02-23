//! The Cartographer 🗺️
//!
//! Visualizes the knowledge graph by projecting high-dimensional embeddings
//! onto a 2D ASCII map.
//!
//! "I have seen the universe, and it is made of text."

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

    /// Generate an ASCII map of the knowledge graph.
    ///
    /// # Errors
    ///
    /// Returns an error if the knowledge store cannot be accessed.
    #[allow(clippy::cast_precision_loss)]
    #[allow(clippy::cast_possible_truncation)]
    #[allow(clippy::cast_sign_loss)]
    pub fn map(&self, width: usize, height: usize) -> ChronosResult<String> {
        let knowledge = self.gallifrey.knowledge();
        let mut points = Vec::new();

        // 1. Collect entities with embeddings
        // We need to scan history to get all entities, then filter for current ones with embeddings.
        knowledge
            .scan_history(|history| {
                // Find the current version
                if let Some(entity) = history.iter().find(|e| e.temporal.is_current()) {
                    if let Some(embedding) = &entity.embedding {
                        points.push((entity.name.clone(), embedding.clone()));
                    }
                }
            })
            .map_err(|e| ChronosError::Common(e.into()))?;

        if points.is_empty() {
            return Ok("The universe is dark (no embeddings found).".to_string());
        }

        // 2. Project to 2D
        // We use a simple random projection.
        // x = dot(emb, random_vector_1)
        // y = dot(emb, random_vector_2)
        // The random vectors are deterministic based on a seed.
        let projected: Vec<(String, (f32, f32))> = points
            .into_iter()
            .map(|(name, emb)| {
                let x = project(&emb, 12_345); // Seed 1
                let y = project(&emb, 67_890); // Seed 2
                (name, (x, y))
            })
            .collect();

        // 3. Normalize to grid
        // Find min/max X and Y
        let mut min_x = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_y = f32::NEG_INFINITY;

        for (_, (x, y)) in &projected {
            if *x < min_x {
                min_x = *x;
            }
            if *x > max_x {
                max_x = *x;
            }
            if *y < min_y {
                min_y = *y;
            }
            if *y > max_y {
                max_y = *y;
            }
        }

        // Avoid division by zero if all points are the same
        if (max_x - min_x).abs() < f32::EPSILON {
            max_x += 1.0;
        }
        if (max_y - min_y).abs() < f32::EPSILON {
            max_y += 1.0;
        }

        // Clamp width/height
        let width = width.max(10).min(200);
        let height = height.max(5).min(100);

        // Initialize grid
        let mut grid = vec![vec![' '; width]; height];

        for (name, (x, y)) in &projected {
            // Normalize to [0, 1]
            let norm_x = (x - min_x) / (max_x - min_x);
            let norm_y = (y - min_y) / (max_y - min_y);

            // Map to grid indices
            // Clamp to [0, width-1]
            let col = ((norm_x * (width as f32 - 1.0)).round() as usize).min(width - 1);
            let row = ((norm_y * (height as f32 - 1.0)).round() as usize).min(height - 1);

            // Flip Y coordinate because terminal row 0 is at the top
            // Visual Y increases upwards, Terminal Y increases downwards.
            let grid_row = height - 1 - row;

            let char_code = name.chars().next().unwrap_or('?');

            // If collision, use '+'
            if grid[grid_row][col] != ' ' && grid[grid_row][col] != '+' {
                grid[grid_row][col] = '+';
            } else if grid[grid_row][col] == ' ' {
                grid[grid_row][col] = char_code;
            }
        }

        // 4. Render
        let mut output = String::with_capacity((width + 3) * (height + 2));
        output.push_str(&format!("┌{}┐\n", "─".repeat(width)));
        for row in grid {
            output.push('│');
            for cell in row {
                output.push(cell);
            }
            output.push('│');
            output.push('\n');
        }
        output.push_str(&format!("└{}┘", "─".repeat(width)));
        // Add legend/stats
        output.push_str(&format!("\nMapped {} entities.", projected.len()));

        Ok(output)
    }
}

/// Deterministic projection of a vector onto a random direction.
fn project(v: &[f32], seed: u64) -> f32 {
    let mut sum = 0.0;
    // We only use the first 768 dimensions if the vector is longer,
    // or as many as available.
    for (i, &val) in v.iter().enumerate() {
        // Generate a deterministic weight in [-1.0, 1.0]
        let weight = pseudo_random_weight(seed, i as u64);
        sum += val * weight;
    }
    sum
}

/// Generates a deterministic pseudo-random weight in [-1.0, 1.0].
fn pseudo_random_weight(seed: u64, index: u64) -> f32 {
    // Simple Linear Congruential Generator
    // Constants from Numerical Recipes
    let a: u64 = 1_664_525;
    let c: u64 = 1_013_904_223;
    // m = 2^32 (implicit in u32, but we use u64 arithmetic)

    // Mix seed and index
    // We use wrapping arithmetic to avoid overflow panics in debug mode
    let mut state = seed.wrapping_add(index);
    state = state.wrapping_mul(a).wrapping_add(c);

    // Get upper bits for better randomness
    let result = (state >> 16) & 0xFFFF;

    // Normalize to [0.0, 1.0]
    #[allow(clippy::cast_precision_loss)]
    let normalized = result as f32 / 65535.0;

    // Scale to [-1.0, 1.0]
    (normalized * 2.0) - 1.0
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_project_deterministic() {
        let v = vec![0.1, 0.2, 0.3];
        let p1 = project(&v, 12_345);
        let p2 = project(&v, 12_345);
        assert!((p1 - p2).abs() < f32::EPSILON);
    }

    #[test]
    fn test_project_different_seeds() {
        let v = vec![0.1, 0.2, 0.3];
        let p1 = project(&v, 12_345);
        let p2 = project(&v, 67_890);
        assert!((p1 - p2).abs() > f32::EPSILON);
    }

    #[test]
    fn test_pseudo_random_weight_range() {
        for i in 0..100 {
            let w = pseudo_random_weight(12_345, i);
            assert!(w >= -1.0 && w <= 1.0);
        }
    }
}
