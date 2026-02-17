//! Serendipity command for Tardis Shell.
//!
//! "Everything is connected."
//!
//! Scans the knowledge graph for hidden connections using the Serendipity Engine.

use anyhow::Result;
use std::fmt::Write;
use std::sync::Arc;
use tardis_chronos::experimental::serendipity::SerendipityEngine;
use tardis_gallifrey::Gallifrey;
use tardis_vortex::{ModelHandle, Vortex};

/// Run the serendipity command.
///
/// # Errors
///
/// Returns an error if the engine fails.
pub async fn run(
    gallifrey: Arc<Gallifrey>,
    vortex: Arc<Vortex>,
    model: ModelHandle,
    args: &[String],
) -> Result<String> {
    let threshold = args
        .first()
        .and_then(|s| s.parse::<f32>().ok())
        .unwrap_or(0.5);

    let engine = SerendipityEngine::new(gallifrey, vortex, model);
    let connections = engine.find_connections(threshold).await?;

    let mut output = String::new();

    if connections.is_empty() {
        writeln!(output, "No serendipitous connections found (threshold: {threshold:.2}).")?;
        writeln!(output, "Try adding more entities or lowering the threshold.")?;
    } else {
        writeln!(output, "🌟 Serendipity found {} potential connections!", connections.len())?;
        writeln!(output, "==================================================")?;

        for (i, conn) in connections.iter().enumerate() {
            writeln!(output, "\n#{}: {} <--> {} (Score: {:.2})",
                i + 1, conn.source.name, conn.target.name, conn.similarity)?;
            writeln!(output, "   Hypothesis: {}", conn.hypothesis)?;
        }
    }

    Ok(output)
}
