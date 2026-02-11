//! # Tardis Shell
//!
//! The AI-native command-line interface for Tardis OS.

use anyhow::Result;
use std::sync::Arc;
use tardis_chronos::Chronos;
use tardis_gallifrey::Gallifrey;
use tardis_vortex::Vortex;
use tardis_telemetry::{init, TelemetryConfig};
use tracing::info;

#[cfg(feature = "nova")]
use tardis_chronos::experimental::prophecy::Prophet;

mod commands;
#[cfg(feature = "nova")]
mod dashboard;
#[cfg(feature = "nova")]
mod experimental;
mod repl;
mod router;

use repl::Repl;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize telemetry
    // This replaces tracing_subscriber::fmt().init()
    let telemetry_handle = init(TelemetryConfig::default())?;

    // Capture the store
    // Telemetry is compiled with "std" feature in shell dependency, so store field exists
    let telemetry_store = telemetry_handle.store.clone();


    info!("Starting Tardis Shell");

    // Initialize subsystems
    let vortex = Arc::new(Vortex::new()?);
    let gallifrey = Arc::new(Gallifrey::new());

    let chronos = Arc::new(Chronos::new(vortex.clone(), gallifrey.clone()));

    // Print banner
    print_banner();

    #[cfg(feature = "nova")]
    let prophet = Arc::new(Prophet::new(vortex.clone()));

    // Create and run REPL
    #[cfg(feature = "nova")]
    let mut repl = Repl::new(chronos, gallifrey, telemetry_store, Some(prophet))?;

    #[cfg(not(feature = "nova"))]
    let mut repl = Repl::new(chronos, gallifrey, telemetry_store)?;

    repl.run().await?;

    // Ensure telemetry flushes
    telemetry_handle.shutdown().await;

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
