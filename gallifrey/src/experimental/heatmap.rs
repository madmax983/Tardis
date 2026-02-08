use chrono::{DateTime, Utc};
use std::fmt::Write;
use tardis_common::domain::Entity;

/// A 2D heatmap visualizing entity activity across Valid Time and Transaction Time.
#[derive(Debug)]
pub struct TemporalHeatmap {
    /// The grid of counts (row-major: grid[y][x]).
    pub grid: Vec<Vec<usize>>,
    /// Valid time range (X-axis).
    pub valid_range: (DateTime<Utc>, DateTime<Utc>),
    /// Transaction time range (Y-axis).
    pub transaction_range: (DateTime<Utc>, DateTime<Utc>),
    /// Number of bins on X-axis.
    pub x_bins: usize,
    /// Number of bins on Y-axis.
    pub y_bins: usize,
}

impl TemporalHeatmap {
    /// Generate a heatmap from a history of entities.
    ///
    /// # Arguments
    ///
    /// * `history` - The list of entity versions.
    /// * `x_bins` - Resolution of the Valid Time axis.
    /// * `y_bins` - Resolution of the Transaction Time axis.
    ///
    /// # Panics
    ///
    /// Panics if `x_bins` or `y_bins` is zero.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    #[allow(clippy::cast_possible_truncation)]
    #[allow(clippy::cast_sign_loss)]
    #[allow(clippy::needless_range_loop)]
    pub fn new(history: &[Entity], x_bins: usize, y_bins: usize) -> Self {
        assert!(x_bins > 0, "x_bins must be > 0");
        assert!(y_bins > 0, "y_bins must be > 0");

        if history.is_empty() {
            let now = Utc::now();
            return Self {
                grid: vec![vec![0; x_bins]; y_bins],
                valid_range: (now, now),
                transaction_range: (now, now),
                x_bins,
                y_bins,
            };
        }

        // 1. Determine bounds
        let now = Utc::now();
        let mut v_min = now;
        let mut v_max = now;
        let mut t_min = now;
        let mut t_max = now;

        let mut first = true;

        for entity in history {
            let vt = entity.temporal.valid_time;
            let tt = entity.temporal.transaction_time;

            if first {
                v_min = vt.start;
                v_max = vt.end.unwrap_or(now);
                t_min = tt.start;
                t_max = tt.end.unwrap_or(now);
                first = false;
            } else {
                if vt.start < v_min {
                    v_min = vt.start;
                }
                let v_end = vt.end.unwrap_or(now);
                if v_end > v_max {
                    v_max = v_end;
                }

                if tt.start < t_min {
                    t_min = tt.start;
                }
                let t_end = tt.end.unwrap_or(now);
                if t_end > t_max {
                    t_max = t_end;
                }
            }
        }

        // Add a small buffer to max to avoid edge cases
        if v_min == v_max {
            v_max = v_min + chrono::Duration::seconds(1);
        }
        if t_min == t_max {
            t_max = t_min + chrono::Duration::seconds(1);
        }

        let mut grid = vec![vec![0; x_bins]; y_bins];

        let v_duration = (v_max - v_min).num_milliseconds() as f64;
        let t_duration = (t_max - t_min).num_milliseconds() as f64;

        // 2. Populate grid
        for entity in history {
            let vt = entity.temporal.valid_time;
            let tt = entity.temporal.transaction_time;
            let vt_end = vt.end.unwrap_or(now);
            let tt_end = tt.end.unwrap_or(now);

            // Calculate overlapping bins
            let v_start_offset = (vt.start - v_min).num_milliseconds() as f64;
            let v_end_offset = (vt_end - v_min).num_milliseconds() as f64;
            let t_start_offset = (tt.start - t_min).num_milliseconds() as f64;
            let t_end_offset = (tt_end - t_min).num_milliseconds() as f64;

            let x_start_idx = ((v_start_offset / v_duration) * x_bins as f64).floor() as usize;
            let x_end_idx = ((v_end_offset / v_duration) * x_bins as f64).ceil() as usize;

            let y_start_idx = ((t_start_offset / t_duration) * y_bins as f64).floor() as usize;
            let y_end_idx = ((t_end_offset / t_duration) * y_bins as f64).ceil() as usize;

            // Clamp indices
            let x_start = x_start_idx.clamp(0, x_bins - 1);
            let x_end = x_end_idx.clamp(1, x_bins); // Exclusive end for loop
            let y_start = y_start_idx.clamp(0, y_bins - 1);
            let y_end = y_end_idx.clamp(1, y_bins);

            for y in y_start..y_end {
                for x in x_start..x_end {
                    grid[y][x] += 1;
                }
            }
        }

        Self {
            grid,
            valid_range: (v_min, v_max),
            transaction_range: (t_min, t_max),
            x_bins,
            y_bins,
        }
    }

    /// Render the heatmap as an ASCII string.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    #[allow(clippy::cast_possible_truncation)]
    #[allow(clippy::cast_sign_loss)]
    #[allow(clippy::uninlined_format_args)]
    pub fn render_ascii(&self) -> String {
        let mut output = String::new();
        // Symbols from low density to high
        let symbols = [' ', '.', ':', '+', '*', '#', '@'];

        // Find max value for normalization
        let max_val = self.grid.iter().flatten().max().copied().unwrap_or(0);

        writeln!(&mut output, "Temporal Heatmap (Y: Transaction, X: Valid)").ok();
        writeln!(
            &mut output,
            "Y-Range: {} to {}",
            self.transaction_range.0, self.transaction_range.1
        )
        .ok();
        writeln!(
            &mut output,
            "X-Range: {} to {}",
            self.valid_range.0, self.valid_range.1
        )
        .ok();
        writeln!(&mut output, "Max Count: {}", max_val).ok();
        writeln!(&mut output, "┌{}┐", "─".repeat(self.x_bins)).ok();

        // Render rows (reversed Y to have time go up)
        for row in self.grid.iter().rev() {
            write!(&mut output, "│").ok();
            for &count in row {
                let symbol_idx = if max_val == 0 {
                    0
                } else {
                    let normalized = (count as f64 / max_val as f64) * (symbols.len() - 1) as f64;
                    normalized.round() as usize
                };
                write!(&mut output, "{}", symbols[symbol_idx]).ok();
            }
            writeln!(&mut output, "│").ok();
        }
        writeln!(&mut output, "└{}┘", "─".repeat(self.x_bins)).ok();

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tardis_common::id::EntityId;
    use tardis_common::temporal::{BiTemporalInterval, TimeRange};

    fn create_entity(
        valid_start: DateTime<Utc>,
        valid_end: Option<DateTime<Utc>>,
        trans_start: DateTime<Utc>,
        trans_end: Option<DateTime<Utc>>,
    ) -> Entity {
        Entity {
            id: EntityId::new(),
            entity_type: "Test".to_string(),
            name: "Test".to_string(),
            properties: HashMap::new(),
            embedding: None,
            temporal: BiTemporalInterval {
                valid_time: TimeRange {
                    start: valid_start,
                    end: valid_end,
                },
                transaction_time: TimeRange {
                    start: trans_start,
                    end: trans_end,
                },
            },
            source: None,
        }
    }

    #[test]
    fn test_heatmap_generation() {
        let now = Utc::now();
        let hour = chrono::Duration::hours(1);

        // Entity 1: Hour 0-1 valid, recorded at Hour 0-1
        let e1 = create_entity(now, Some(now + hour), now, Some(now + hour));
        // Entity 2: Hour 1-2 valid, recorded at Hour 1-2
        let e2 = create_entity(
            now + hour,
            Some(now + hour * 2),
            now + hour,
            Some(now + hour * 2),
        );

        let history = vec![e1, e2];
        let heatmap = TemporalHeatmap::new(&history, 2, 2);

        // Check bounds
        assert!(heatmap.valid_range.1 >= now + hour * 2);
        assert!(heatmap.transaction_range.1 >= now + hour * 2);

        // Expect diagonal activation
        // Grid is [y][x].
        // y=0 (early transaction), x=0 (early valid) -> e1
        // y=1 (late transaction), x=1 (late valid) -> e2

        // Note: Floating point index calculation might land e1 slightly in multiple bins if not perfect,
        // but with 2 bins and clear separation, it should be fine.

        // Let's print the grid to be sure in debug
        println!("{:?}", heatmap.grid);

        assert!(heatmap.grid[0][0] >= 1);
        assert!(heatmap.grid[1][1] >= 1);

        println!("{}", heatmap.render_ascii());
    }

    #[test]
    fn test_heatmap_empty() {
        let history = vec![];
        let heatmap = TemporalHeatmap::new(&history, 5, 5);
        assert_eq!(heatmap.grid.len(), 5);
        assert_eq!(heatmap.grid[0].len(), 5);
        assert_eq!(heatmap.grid[0][0], 0);
    }
}
