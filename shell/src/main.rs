//! # Tardis Shell
//!
//! The AI-native command-line interface for Tardis OS.

#![allow(clippy::unused_self)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::needless_continue)]
#![allow(clippy::unused_async)]

use anyhow::Result;
use std::sync::Arc;
use tardis_chronos::Chronos;
use tardis_gallifrey::Gallifrey;
use tardis_vortex::Vortex;
use tracing::info;

mod commands;
mod repl;
mod router;

use repl::Repl;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env().add_directive("tardis=info".parse()?),
        )
        .init();

    info!("Starting Tardis Shell");

    // Initialize subsystems
    let vortex = Arc::new(Vortex::new()?);
    let gallifrey = Arc::new(Gallifrey::new());
    let chronos = Arc::new(Chronos::new(Arc::clone(&vortex), Arc::clone(&gallifrey)));

    // Print banner
    print_banner();

    // Create and run REPL
    let mut repl = Repl::new(chronos, gallifrey)?;
    repl.run().await?;

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
