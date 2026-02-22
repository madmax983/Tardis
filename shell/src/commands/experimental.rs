//! Experimental commands for the Tardis shell.
//!
//! These commands are only available when the `nova` feature is enabled.
//!
//! # Features
//!
//! - **Sonic Screwdriver**: System diagnosis and repair.
//! - **Biographer**: Narrative generation.
//! - **Chronograph**: Timeline visualization.
//! - **Heatmap**: Temporal activity analysis.
//! - **Curiosity**: Active learning.
//! - **Time Capsule**: Backup and restore entity subgraphs.
//! - **Medium**: Seance with past system states.

use crate::commands::traits::{CommandContext, CommandResult, ShellCommand};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

#[cfg(feature = "nova")]
use crate::experimental::{
    biographer::Biographer, chronograph::ChronoGraph, heatmap_cmd, sonic::SonicScrewdriver,
};
#[cfg(feature = "nova")]
use tardis_chronos::experimental::curiosity::Curiosity;
#[cfg(feature = "nova")]
use tardis_chronos::experimental::dreamer::Dreamer;
#[cfg(feature = "nova")]
use tardis_chronos::experimental::medium::Medium;
#[cfg(feature = "nova")]
use tardis_chronos::experimental::weaver::Weaver;
#[cfg(feature = "nova")]
use tardis_chronos::experimental::astrolabe::Astrolabe;
#[cfg(feature = "nova")]
use tardis_gallifrey::experimental::time_capsule::TimeCapsule;

/// Chameleon command (System Persona).
#[cfg(feature = "nova")]
#[derive(Debug)]
pub struct ChameleonCommand;

#[cfg(feature = "nova")]
#[async_trait]
impl ShellCommand for ChameleonCommand {
    fn name(&self) -> &'static str {
        "chameleon"
    }

    fn description(&self) -> &'static str {
        "Set the system persona"
    }

    async fn execute(&self, args: &[String], _context: &CommandContext) -> Result<CommandResult> {
        if args.is_empty() {
            println!("Usage: chameleon <persona> | chameleon off | chameleon reset");
            return Ok(CommandResult::Continue);
        }

        let input = args.join(" ");
        let persona = match input.to_lowercase().as_str() {
            "off" | "reset" | "default" | "none" => None,
            _ => Some(input),
        };

        Ok(CommandResult::SetPersona(persona))
    }
}

/// Sonic Screwdriver tool command.
///
/// Provides system diagnostics and automated repair capabilities.
///
/// # Usage
///
/// ```text
/// sonic diagnose          # Run system diagnostics
/// sonic <file>            # Inspect a file for issues
/// sonic repair <file>     # Attempt to repair a file
/// ```
#[cfg(feature = "nova")]
#[derive(Debug)]
pub struct SonicCommand;

#[cfg(feature = "nova")]
#[async_trait]
impl ShellCommand for SonicCommand {
    fn name(&self) -> &'static str {
        "sonic"
    }

    fn description(&self) -> &'static str {
        "Sonic Screwdriver tool"
    }

    async fn execute(&self, args: &[String], context: &CommandContext) -> Result<CommandResult> {
        if args.is_empty() {
            println!("Usage: sonic <file> | sonic repair <file> | sonic diagnose");
            return Ok(CommandResult::Continue);
        }

        let loaded_models = context.chronos.vortex().list_loaded_models();
        let model_handle = loaded_models.first().map(|(h, _)| *h);

        let screwdriver = SonicScrewdriver::new(
            context.telemetry.clone(),
            Some(Arc::clone(&context.gallifrey)),
            Some(context.chronos.vortex().clone()),
            model_handle,
        );

        if args[0] == "diagnose" {
            println!("{}", screwdriver.buzz());
            match screwdriver.diagnose().await {
                Ok(report) => println!("{report}"),
                Err(e) => println!("Diagnosis failed: {e}"),
            }
            return Ok(CommandResult::Continue);
        }

        let path = std::path::Path::new(&args[args.len() - 1]);
        let repair = args.len() > 1 && (args[0] == "repair" || args[0] == "fix");

        let result = if repair {
            screwdriver.repair(path)
        } else {
            screwdriver.inspect(path)
        };

        match result {
            Ok(report) => println!("{report}"),
            Err(e) => println!("Sonic Screwdriver error: {e}"),
        }
        Ok(CommandResult::Continue)
    }
}

/// Navigate command.
#[cfg(feature = "nova")]
#[derive(Debug)]
pub struct NavigateCommand;

#[cfg(feature = "nova")]
#[async_trait]
impl ShellCommand for NavigateCommand {
    fn name(&self) -> &'static str {
        "navigate"
    }

    fn description(&self) -> &'static str {
        "Find a semantic path between two entities"
    }

    async fn execute(&self, args: &[String], context: &CommandContext) -> Result<CommandResult> {
        if args.len() < 2 {
            println!("Usage: navigate <start> <end>");
            return Ok(CommandResult::Continue);
        }

        let start = &args[0];
        let end = &args[1];

        // Astrolabe accepts Option<Arc<Vortex>>.
        // We pass the vortex engine if available, though it might not be strictly needed for
        // pathfinding if embeddings are already present.
        let vortex = Some(context.chronos.vortex().clone());

        let astrolabe = Astrolabe::new(
            Arc::clone(&context.gallifrey),
            vortex,
        );

        println!("🧭 Navigating the knowledge graph from '{start}' to '{end}'...");
        match astrolabe.navigate(start, end) {
             Ok(path) => {
                 println!("\nPath found ({} steps):", path.len() - 1);
                 for (i, node) in path.iter().enumerate() {
                     if i < path.len() - 1 {
                         print!("{node} -> ");
                     } else {
                         println!("{node}");
                     }
                 }
                 println!();
             }
             Err(e) => println!("Navigation failed: {e}"),
        }
        Ok(CommandResult::Continue)
    }
}

/// Dreamer command.
#[cfg(feature = "nova")]
#[derive(Debug)]
pub struct DreamCommand;

#[cfg(feature = "nova")]
#[async_trait]
impl ShellCommand for DreamCommand {
    fn name(&self) -> &'static str {
        "dream"
    }

    fn description(&self) -> &'static str {
        "Consolidate memories from conversation"
    }

    async fn execute(&self, _args: &[String], context: &CommandContext) -> Result<CommandResult> {
        let loaded_models = context.chronos.vortex().list_loaded_models();
        if let Some((handle, _)) = loaded_models.first() {
            let dreamer = Dreamer::new(
                Arc::clone(&context.gallifrey),
                context.chronos.vortex().clone(),
                *handle,
            );

            println!("💤 Entering REM sleep...");
            match dreamer.dream(context.session_id).await {
                Ok(ids) => {
                    if ids.is_empty() {
                        println!("No new memories formed.");
                    } else {
                        println!(
                            "✨ Consolidated {} new memories into long-term storage.",
                            ids.len()
                        );
                    }
                }
                Err(e) => println!("Nightmare encountered: {e}"),
            }
        } else {
            println!("Dreaming requires a loaded model. Use 'models load <path>'.");
        }
        Ok(CommandResult::Continue)
    }
}

/// Repair command (alias for sonic repair).
///
/// A shortcut for `sonic repair`.
///
/// # Usage
///
/// ```text
/// fix <file>
/// ```
#[cfg(feature = "nova")]
#[derive(Debug)]
pub struct FixCommand;

#[cfg(feature = "nova")]
#[async_trait]
impl ShellCommand for FixCommand {
    fn name(&self) -> &'static str {
        "fix"
    }

    fn description(&self) -> &'static str {
        "Repair a file (alias for sonic repair)"
    }

    async fn execute(&self, args: &[String], context: &CommandContext) -> Result<CommandResult> {
        if args.is_empty() {
            println!("Usage: fix <file>");
            return Ok(CommandResult::Continue);
        }

        let loaded_models = context.chronos.vortex().list_loaded_models();
        let model_handle = loaded_models.first().map(|(h, _)| *h);

        let screwdriver = SonicScrewdriver::new(
            context.telemetry.clone(),
            Some(Arc::clone(&context.gallifrey)),
            Some(context.chronos.vortex().clone()),
            model_handle,
        );

        let path = std::path::Path::new(&args[0]);
        match screwdriver.repair(path) {
            Ok(report) => println!("{report}"),
            Err(e) => println!("Sonic Screwdriver error: {e}"),
        }
        Ok(CommandResult::Continue)
    }
}

/// Dashboard command.
///
/// Launches a TUI (Text User Interface) dashboard to monitor system status.
///
/// # Usage
///
/// ```text
/// dashboard
/// ```
#[cfg(feature = "nova")]
#[derive(Debug)]
pub struct DashboardCommand;

#[cfg(feature = "nova")]
#[async_trait]
impl ShellCommand for DashboardCommand {
    fn name(&self) -> &'static str {
        "dashboard"
    }

    fn description(&self) -> &'static str {
        "Show system dashboard"
    }

    async fn execute(&self, _args: &[String], context: &CommandContext) -> Result<CommandResult> {
        match crate::dashboard::tui::Dashboard::new(
            Arc::clone(&context.gallifrey),
            context.telemetry.clone(),
            context.prophet.clone(),
        ) {
            Ok(mut dashboard) => {
                if let Err(e) = dashboard.run() {
                    println!("Dashboard failed: {e}");
                }
            }
            Err(e) => println!("Failed to initialize dashboard: {e}"),
        }
        Ok(CommandResult::Continue)
    }
}

/// Timeline command.
///
/// Visualizes the history of changes to a specific entity.
///
/// # Usage
///
/// ```text
/// timeline <entity_name>
/// ```
#[cfg(feature = "nova")]
#[derive(Debug)]
pub struct TimelineCommand;

#[cfg(feature = "nova")]
#[async_trait]
impl ShellCommand for TimelineCommand {
    fn name(&self) -> &'static str {
        "timeline"
    }

    fn description(&self) -> &'static str {
        "Show entity timeline"
    }

    async fn execute(&self, args: &[String], context: &CommandContext) -> Result<CommandResult> {
        if args.is_empty() {
            println!("Usage: timeline <entity_name>");
            return Ok(CommandResult::Continue);
        }
        let entity_name = &args[0];

        let graph = ChronoGraph::new(context.gallifrey.clone());
        match graph.generate_timeline(entity_name) {
            Ok(timeline) => println!("{timeline}"),
            Err(e) => println!("Failed to generate timeline: {e}"),
        }
        Ok(CommandResult::Continue)
    }
}

/// Entity map command.
///
/// Generates a text-based map of an entity's relationships.
///
/// # Usage
///
/// ```text
/// map <entity_name> [depth]
/// ```
///
/// - `depth`: How many levels of relationships to traverse (default: 2).
#[cfg(feature = "nova")]
#[derive(Debug)]
pub struct MapCommand;

#[cfg(feature = "nova")]
#[async_trait]
impl ShellCommand for MapCommand {
    fn name(&self) -> &'static str {
        "map"
    }

    fn description(&self) -> &'static str {
        "Show entity map"
    }

    async fn execute(&self, args: &[String], context: &CommandContext) -> Result<CommandResult> {
        if args.is_empty() {
            println!("Usage: map <entity_name> [depth]");
            return Ok(CommandResult::Continue);
        }
        let entity_name = &args[0];
        let depth = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(2);

        let graph = ChronoGraph::new(context.gallifrey.clone());
        match graph.generate_map(entity_name, depth, None) {
            Ok(map) => println!("{map}"),
            Err(e) => println!("Failed to generate map: {e}"),
        }
        Ok(CommandResult::Continue)
    }
}

/// Heatmap command.
///
/// Generates a temporal heatmap showing activity density for an entity.
///
/// # Usage
///
/// ```text
/// heatmap <entity_name>
/// ```
#[cfg(feature = "nova")]
#[derive(Debug)]
pub struct HeatmapCommand;

#[cfg(feature = "nova")]
#[async_trait]
impl ShellCommand for HeatmapCommand {
    fn name(&self) -> &'static str {
        "heatmap"
    }

    fn description(&self) -> &'static str {
        "Show temporal heatmap"
    }

    async fn execute(&self, args: &[String], context: &CommandContext) -> Result<CommandResult> {
        match heatmap_cmd::run(&context.gallifrey, args) {
            Ok(report) => println!("{report}"),
            Err(e) => println!("Failed to generate heatmap: {e}"),
        }
        Ok(CommandResult::Continue)
    }
}

/// Biography command.
///
/// Generates a narrative biography for an entity using the LLM.
///
/// # Usage
///
/// ```text
/// biography <entity_name>
/// ```
///
/// # Prerequisites
///
/// - Requires a loaded LLM model.
#[cfg(feature = "nova")]
#[derive(Debug)]
pub struct BiographerCommand;

#[cfg(feature = "nova")]
#[async_trait]
impl ShellCommand for BiographerCommand {
    fn name(&self) -> &'static str {
        "biography"
    }

    fn description(&self) -> &'static str {
        "Generate entity biography"
    }

    async fn execute(&self, args: &[String], context: &CommandContext) -> Result<CommandResult> {
        if args.is_empty() {
            println!("Usage: biography <entity_name>");
            return Ok(CommandResult::Continue);
        }
        let entity_name = args.join(" ");

        let loaded_models = context.chronos.vortex().list_loaded_models();
        let model_handle = loaded_models.first().map(|(h, _)| *h);

        let biographer = Biographer::new(
            Arc::clone(&context.gallifrey),
            context.chronos.vortex().clone(),
            model_handle,
        );

        match biographer.biography(&entity_name).await {
            Ok(bio) => println!("\n{bio}\n"),
            Err(e) => println!("Failed to generate biography: {e}"),
        }
        Ok(CommandResult::Continue)
    }
}

/// Curiosity command.
///
/// Triggers the Curiosity Engine to actively scan for knowledge gaps and ask questions.
///
/// # Usage
///
/// ```text
/// curiosity
/// ```
///
/// # Prerequisites
///
/// - Requires a loaded LLM model.
#[cfg(feature = "nova")]
#[derive(Debug)]
pub struct CuriosityCommand;

#[cfg(feature = "nova")]
#[async_trait]
impl ShellCommand for CuriosityCommand {
    fn name(&self) -> &'static str {
        "curiosity"
    }

    fn description(&self) -> &'static str {
        "Ask the Curiosity Engine"
    }

    async fn execute(&self, _args: &[String], context: &CommandContext) -> Result<CommandResult> {
        let loaded_models = context.chronos.vortex().list_loaded_models();
        if let Some((handle, _)) = loaded_models.first() {
            let curiosity = Curiosity::new(
                Arc::clone(&context.gallifrey),
                context.chronos.vortex().clone(),
                *handle,
            );

            println!("🤔 Curiosity is scanning...");
            match curiosity.ask().await {
                Ok(question) => println!("{question}"),
                Err(e) => println!("Curiosity failed: {e}"),
            }
        } else {
            println!("Curiosity needs a loaded model. Use 'models load <path>'.");
        }
        Ok(CommandResult::Continue)
    }
}

/// Time Capsule command.
///
/// Manages "Time Capsules" - portable snapshots of entity subgraphs.
///
/// # Usage
///
/// ```text
/// capsule capture <entity> <file> [depth]   # Save entity subgraph to file
/// capsule restore <file>                    # Restore subgraph from file
/// ```
#[cfg(feature = "nova")]
#[derive(Debug)]
pub struct CapsuleCommand;

#[cfg(feature = "nova")]
#[async_trait]
impl ShellCommand for CapsuleCommand {
    fn name(&self) -> &'static str {
        "capsule"
    }

    fn description(&self) -> &'static str {
        "Manage time capsules"
    }

    async fn execute(&self, args: &[String], context: &CommandContext) -> Result<CommandResult> {
        if args.len() < 2 {
            println!("Usage: capsule capture <entity> <file> [depth] | capsule restore <file>");
            return Ok(CommandResult::Continue);
        }

        let subcommand = &args[0];
        match subcommand.as_str() {
            "capture" => {
                if args.len() < 3 {
                    println!("Usage: capsule capture <entity> <file> [depth]");
                    return Ok(CommandResult::Continue);
                }
                let entity_name = &args[1];
                let file_path = &args[2];
                let depth = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(1);

                match TimeCapsule::capture(&context.gallifrey, entity_name, depth) {
                    Ok(capsule) => match capsule.save_to_file(file_path) {
                        Ok(()) => println!(
                            "Captured {} entities and {} relationships to {}",
                            capsule.entities.len(),
                            capsule.relationships.len(),
                            file_path
                        ),
                        Err(e) => println!("Failed to save capsule: {e}"),
                    },
                    Err(e) => println!("Failed to capture capsule: {e}"),
                }
            }
            "restore" => {
                let file_path = &args[1];
                match TimeCapsule::load_from_file(file_path) {
                    Ok(capsule) => {
                        let count = capsule.entities.len();
                        match capsule.restore(&context.gallifrey).await {
                            Ok(()) => {
                                println!("Restored {count} entities from {file_path}");
                            }
                            Err(e) => println!("Failed to restore capsule: {e}"),
                        }
                    }
                    Err(e) => println!("Failed to load capsule: {e}"),
                }
            }
            _ => println!("Unknown capsule command: {subcommand}"),
        }
        Ok(CommandResult::Continue)
    }
}

/// Weave command.
#[cfg(feature = "nova")]
#[derive(Debug)]
pub struct WeaveCommand;

#[cfg(feature = "nova")]
#[async_trait]
impl ShellCommand for WeaveCommand {
    fn name(&self) -> &'static str {
        "weave"
    }

    fn description(&self) -> &'static str {
        "Generate a narrative connection between two entities"
    }

    async fn execute(&self, args: &[String], context: &CommandContext) -> Result<CommandResult> {
        if args.len() < 2 {
            println!("Usage: weave <entity1> <entity2>");
            return Ok(CommandResult::Continue);
        }

        let entity1 = &args[0];
        let entity2 = &args[1];

        let loaded_models = context.chronos.vortex().list_loaded_models();
        if let Some((handle, _)) = loaded_models.first() {
            let weaver = Weaver::new(
                Arc::clone(&context.gallifrey),
                context.chronos.vortex().clone(),
                *handle,
            );

            println!("🕸️  Weaving destiny between '{entity1}' and '{entity2}'...");
            match weaver.weave(entity1, entity2).await {
                Ok(story) => println!("\n{story}\n"),
                Err(e) => println!("The threads snapped: {e}"),
            }
        } else {
            println!("The Weaver needs a loaded model. Use 'models load <path>'.");
        }
        Ok(CommandResult::Continue)
    }
}

/// Medium command.
///
/// Summon the system state from the past.
///
/// # Usage
///
/// ```text
/// medium <time> <query>
/// ```
#[cfg(feature = "nova")]
#[derive(Debug)]
pub struct MediumCommand;

#[cfg(feature = "nova")]
#[async_trait]
impl ShellCommand for MediumCommand {
    fn name(&self) -> &'static str {
        "medium"
    }

    fn description(&self) -> &'static str {
        "Summon system state from the past"
    }

    async fn execute(&self, args: &[String], context: &CommandContext) -> Result<CommandResult> {
        if args.len() < 2 {
            println!("Usage: medium <time> <query>");
            println!("Example: medium 2023-10-27T12:00:00Z \"Who was the active user?\"");
            return Ok(CommandResult::Continue);
        }

        let time_str = &args[0];
        let query = args[1..].join(" ");

        let time = match chrono::DateTime::parse_from_rfc3339(time_str) {
            Ok(t) => t.with_timezone(&chrono::Utc),
            Err(e) => {
                println!("Invalid time format: {e}. Use ISO 8601 (e.g., 2023-10-27T12:00:00Z)");
                return Ok(CommandResult::Continue);
            }
        };

        let loaded_models = context.chronos.vortex().list_loaded_models();
        if let Some((handle, _)) = loaded_models.first() {
            let medium = Medium::new(
                Arc::clone(&context.gallifrey),
                context.chronos.vortex().clone(),
                *handle,
            );

            println!("🕯️ Summoning the past at {time}...");
            match medium.summon(&query, time).await {
                Ok(response) => println!("\n👻 The Spirit answers:\n\n{response}\n"),
                Err(e) => println!("The connection was severed: {e}"),
            }
        } else {
            println!("The Medium needs a loaded model. Use 'models load <path>'.");
        }
        Ok(CommandResult::Continue)
    }
}
