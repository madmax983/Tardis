//! Temporal Heatmap visualization.
//!
//! Generates a 2D density plot of Valid Time vs Transaction Time.

use chrono::{DateTime, Duration, Utc};
use std::fmt::Write;
use tardis_common::domain::Entity;

/// A heatmap of temporal density.
#[derive(Debug)]
pub struct TemporalHeatmap {
    /// Grid of counts (rows = transaction time, cols = valid time).
    grid: Vec<Vec<usize>>,
    /// Start of the time window.
    start: DateTime<Utc>,
    /// End of the time window.
    end: DateTime<Utc>,
    /// Resolution of each cell.
    resolution: Duration,
    /// Width of the grid (number of cols).
    width: usize,
    /// Height of the grid (number of rows).
    height: usize,
}

impl TemporalHeatmap {
    /// Create a new heatmap.
    #[must_use]
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    pub fn new(start: DateTime<Utc>, end: DateTime<Utc>, resolution: Duration) -> Self {
        let total_duration = end - start;
        let total_ms = total_duration.num_milliseconds();
        let mut res_ms = resolution.num_milliseconds();

        // Avoid division by zero or negative duration
        if total_ms <= 0 || res_ms <= 0 {
            return Self {
                grid: Vec::new(),
                start,
                end,
                resolution,
                width: 0,
                height: 0,
            };
        }

        let max_cells = 100;
        let required_cells = (total_ms as f64 / res_ms as f64).ceil() as usize;

        if required_cells > max_cells {
            // Auto-scale resolution to fit within max_cells
            res_ms = (total_ms as f64 / max_cells as f64).ceil() as i64;
        }

        let adjusted_resolution = Duration::milliseconds(res_ms);
        let num_cells = (total_ms as f64 / res_ms as f64).ceil() as usize;
        let size = num_cells;

        Self {
            grid: vec![vec![0; size]; size],
            start,
            end,
            resolution: adjusted_resolution,
            width: size,
            height: size,
        }
    }

    /// Analyze entities and populate the heatmap.
    pub fn analyze(&mut self, entities: &[Entity]) {
        if self.width == 0 || self.height == 0 {
            return;
        }

        for entity in entities {
            let valid_time = entity.temporal.valid_time.start;
            let transaction_time = entity.temporal.transaction_time.start;

            if let (Some(x), Some(y)) = (self.to_coord(valid_time), self.to_coord(transaction_time))
            {
                if y < self.height && x < self.width {
                    self.grid[y][x] += 1;
                }
            }
        }
    }

    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    fn to_coord(&self, time: DateTime<Utc>) -> Option<usize> {
        if time < self.start || time >= self.end {
            return None;
        }
        let diff = time - self.start;
        Some(
            (diff.num_milliseconds() as f64 / self.resolution.num_milliseconds() as f64).floor()
                as usize,
        )
    }

    /// Render the heatmap as ASCII art.
    ///
    /// # Errors
    ///
    /// Returns an error if string formatting fails.
    pub fn render_ascii(&self) -> Result<String, std::fmt::Error> {
        let mut out = String::new();

        writeln!(out, "Temporal Heatmap (Y: Transaction Time, X: Valid Time)")?;
        writeln!(out, "Range: {} to {}", self.start, self.end)?;
        writeln!(out, "Resolution: {}s", self.resolution.num_seconds())?;
        writeln!(out, "┌{}┐", "─".repeat(self.width))?;

        for y in (0..self.height).rev() {
            // Top is later time
            write!(out, "│")?;
            for x in 0..self.width {
                let count = self.grid[y][x];
                let char = match count {
                    0 => ' ',
                    1..=2 => '.',
                    3..=5 => ':',
                    6..=9 => '+',
                    10..=20 => '#',
                    _ => '@',
                };
                write!(out, "{char}")?;
            }
            writeln!(out, "│")?;
        }
        writeln!(out, "└{}┘", "─".repeat(self.width))?;

        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tardis_common::id::EntityId;
    use tardis_common::temporal::{BiTemporalInterval, TimeRange};

    #[test]
    fn test_heatmap_generation() -> Result<(), Box<dyn std::error::Error>> {
        let now = Utc::now();
        let start = now - Duration::hours(10);
        let end = now + Duration::hours(10);
        let resolution = Duration::hours(1); // 20x20 grid

        let mut heatmap = TemporalHeatmap::new(start, end, resolution);

        let mut entities = Vec::new();

        // Create an entity at "now" (center)
        let entity1 = Entity {
            id: EntityId::new(),
            entity_type: "Test".to_string(),
            name: "Test1".to_string(),
            properties: HashMap::new(),
            embedding: None,
            temporal: BiTemporalInterval::now(),
            source: None,
        };
        entities.push(entity1);

        // Create an entity in the past (Valid Time = past, Transaction Time = now)
        // This represents "learning something about the past"
        let past = now - Duration::hours(5);
        let entity2 = Entity {
            id: EntityId::new(),
            entity_type: "Test".to_string(),
            name: "Test2".to_string(),
            properties: HashMap::new(),
            embedding: None,
            temporal: BiTemporalInterval {
                valid_time: TimeRange::starting_at(past),
                transaction_time: TimeRange::starting_at(now),
            },
            source: None,
        };
        entities.push(entity2);

        heatmap.analyze(&entities);
        let ascii = heatmap.render_ascii()?;
        println!("{ascii}");

        assert!(ascii.contains("Temporal Heatmap"));
        assert!(ascii.contains('.')); // Should have some dots

        Ok(())
    }
}
