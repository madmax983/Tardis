//! # Tardis Shell
//!
//! The AI-native command-line interface for Tardis OS.

use anyhow::Result;
use std::sync::Arc;
use tardis_chronos::Chronos;
use tardis_gallifrey::Gallifrey;
use tardis_telemetry::{init, TelemetryConfig};
use tardis_vortex::Vortex;
use tracing::info;

mod commands;
#[cfg(feature = "nova")]
mod dashboard;
mod repl;
mod router;

use repl::Repl;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize telemetry
    let config = TelemetryConfig::default();
    let handle = init(config)?;

    info!("Starting Tardis Shell");

    // Initialize subsystems
    let vortex = Arc::new(Vortex::new()?);
    let gallifrey = Arc::new(Gallifrey::new());

    let mut chronos = Chronos::new(vortex.clone(), gallifrey.clone());

    if let Some(store) = handle.store() {
        chronos = chronos.with_telemetry(Arc::clone(store));
    }

    let chronos = Arc::new(chronos);

    // Print banner
    print_banner();

    // Create and run REPL
    let mut repl = Repl::new(chronos, gallifrey)?;
    repl.run().await?;

    // Shutdown telemetry
    handle.shutdown().await;

    info!("Tardis Shell exiting");
    Ok(())
}

fn print_banner() {
    println!();
    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║                                                            ║");
    println!("║              ████████╗ █████╗ ██████╗ ██████╗ ██╗███████╗  ║");
    println!("║              ╚══██╔══╝██╔══██╗██╔══██╗██╔══██╗██║██╔════╝  ║");
    println!("║                 ██║   ███████║██████╔╝██║  ██║██║███████╗  ║");
    println!("║                 ██║   ██╔══██║██╔══██╗██║  ██║██║╚════██║  ║");
    println!("║                 ██║   ██║  ██║██║  ██║██████╔╝██║███████║  ║");
    println!("║                 ╚═╝   ╚═╝  ╚═╝╚═╝  ╚═╝╚═════╝ ╚═╝╚══════╝  ║");
    println!("║                                                            ║");
    println!("║           AI Shell with Temporal Memory v0.1.0             ║");
    println!("║                                                            ║");
    println!("╚════════════════════════════════════════════════════════════╝");
    println!();
    println!("Type 'help' for commands, or just start talking!");
    println!();
}
