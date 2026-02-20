//! Experimental commands for the Tardis shell.
//!
//! These commands are only available when the `nova` feature is enabled.

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
use tardis_gallifrey::experimental::time_capsule::TimeCapsule;
#[cfg(feature = "nova")]
use tardis_vortex::InferenceParams;

/// Chameleon command (System Persona).
#[cfg(feature = "nova")]
#[derive(Debug)]
pub struct ChameleonCommand;

#[cfg(feature = "nova")]
#[async_trait]
impl ShellCommand for ChameleonCommand {
    fn name(&self) -> &str {
        "chameleon"
    }

    fn description(&self) -> &str {
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
#[cfg(feature = "nova")]
#[derive(Debug)]
pub struct SonicCommand;

#[cfg(feature = "nova")]
#[async_trait]
impl ShellCommand for SonicCommand {
    fn name(&self) -> &str {
        "sonic"
    }

    fn description(&self) -> &str {
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
            Some(context.chronos.vortex()),
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

/// Medium command.
#[cfg(feature = "nova")]
#[derive(Debug)]
pub struct MediumCommand;

#[cfg(feature = "nova")]
#[async_trait]
impl ShellCommand for MediumCommand {
    fn name(&self) -> &str {
        "medium"
    }

    fn description(&self) -> &str {
        "Talk to the past state of an entity"
    }

    async fn execute(&self, args: &[String], context: &CommandContext) -> Result<CommandResult> {
        if args.len() < 2 {
            println!("Usage: medium <entity> <time> (e.g. '2023-10-01T12:00:00Z')");
            return Ok(CommandResult::Continue);
        }

        let entity_name = &args[0];
        let time_str = &args[1];

        let time = match chrono::DateTime::parse_from_rfc3339(time_str) {
            Ok(t) => t.with_timezone(&chrono::Utc),
            Err(e) => {
                println!("Invalid time format: {e}");
                return Ok(CommandResult::Continue);
            }
        };

        let loaded_models = context.chronos.vortex().list_loaded_models();
        let model_handle = if let Some((h, _)) = loaded_models.first() {
            *h
        } else {
            println!("Medium requires a loaded model. Use 'models load <path>'.");
            return Ok(CommandResult::Continue);
        };

        let medium = Medium::new(
            Arc::clone(&context.gallifrey),
        );

        let session = match medium.summon(entity_name, time, model_handle) {
            Ok(s) => s,
            Err(e) => {
                println!("Summoning failed: {e}");
                return Ok(CommandResult::Continue);
            }
        };

        println!("🕯️ Seance started with {} from {}. Type 'exit' to end.", session.entity_name, session.time);
        println!("---------------------------------------------------");

        // Initial system prompt injection (simulated conversation start)
        // In a real chat loop we would maintain a buffer of messages.
        let mut conversation_history = format!("<<SYS>>\n{}\n<</SYS>>\n\n", session.system_prompt);

        use std::io::{self, Write};
        let stdin = io::stdin();
        let mut stdout = io::stdout();

        loop {
            print!("medium> ");
            stdout.flush()?;

            let mut input = String::new();
            if stdin.read_line(&mut input).is_err() {
                break;
            }

            let input = input.trim();
            if input.eq_ignore_ascii_case("exit") || input.eq_ignore_ascii_case("quit") {
                break;
            }
            if input.is_empty() {
                continue;
            }

            // Append user message
            conversation_history.push_str(&format!("[INST] {} [/INST] ", input));

            // Run inference
            let params = InferenceParams {
                max_tokens: 512,
                temperature: 0.8, // Higher temperature for "spirit" like creativity
                ..Default::default()
            };

            // Use the last N chars to avoid context overflow if no truncation
            // Ideally we use a proper window manager, but for this experiment:
            let context_str = if conversation_history.len() > 4000 {
                // Crude truncation keeping system prompt and tail
                // NOTE: This is just a safety valve, real implementation needs token counting
                let split = conversation_history.len() - 3000;
                let tail = &conversation_history[split..];
                format!("<<SYS>>\n{}\n<</SYS>>\n...{}", session.system_prompt, tail)
            } else {
                conversation_history.clone()
            };

            match context.chronos.vortex().infer(model_handle, &context_str, params).await {
                Ok(response) => {
                    println!("{}", response);
                    conversation_history.push_str(&response);
                    conversation_history.push('\n');
                }
                Err(e) => println!("The connection is weak... (Error: {e})"),
            }
        }

        println!("🕯️ Seance ended.");
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
    fn name(&self) -> &str {
        "dream"
    }

    fn description(&self) -> &str {
        "Consolidate memories from conversation"
    }

    async fn execute(&self, _args: &[String], context: &CommandContext) -> Result<CommandResult> {
        let loaded_models = context.chronos.vortex().list_loaded_models();
        if let Some((handle, _)) = loaded_models.first() {
            let dreamer = Dreamer::new(
                Arc::clone(&context.gallifrey),
                context.chronos.vortex(),
                *handle,
            );

            println!("💤 Entering REM sleep...");
            match dreamer.dream(context.session_id).await {
                Ok(ids) => {
                    if ids.is_empty() {
                        println!("No new memories formed.");
                    } else {
                        println!("✨ Consolidated {} new memories into long-term storage.", ids.len());
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
#[cfg(feature = "nova")]
#[derive(Debug)]
pub struct FixCommand;

#[cfg(feature = "nova")]
#[async_trait]
impl ShellCommand for FixCommand {
    fn name(&self) -> &str {
        "fix"
    }

    fn description(&self) -> &str {
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
            Some(context.chronos.vortex()),
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
#[cfg(feature = "nova")]
#[derive(Debug)]
pub struct DashboardCommand;

#[cfg(feature = "nova")]
#[async_trait]
impl ShellCommand for DashboardCommand {
    fn name(&self) -> &str {
        "dashboard"
    }

    fn description(&self) -> &str {
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
#[cfg(feature = "nova")]
#[derive(Debug)]
pub struct TimelineCommand;

#[cfg(feature = "nova")]
#[async_trait]
impl ShellCommand for TimelineCommand {
    fn name(&self) -> &str {
        "timeline"
    }

    fn description(&self) -> &str {
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
#[cfg(feature = "nova")]
#[derive(Debug)]
pub struct MapCommand;

#[cfg(feature = "nova")]
#[async_trait]
impl ShellCommand for MapCommand {
    fn name(&self) -> &str {
        "map"
    }

    fn description(&self) -> &str {
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
#[cfg(feature = "nova")]
#[derive(Debug)]
pub struct HeatmapCommand;

#[cfg(feature = "nova")]
#[async_trait]
impl ShellCommand for HeatmapCommand {
    fn name(&self) -> &str {
        "heatmap"
    }

    fn description(&self) -> &str {
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
#[cfg(feature = "nova")]
#[derive(Debug)]
pub struct BiographerCommand;

#[cfg(feature = "nova")]
#[async_trait]
impl ShellCommand for BiographerCommand {
    fn name(&self) -> &str {
        "biography"
    }

    fn description(&self) -> &str {
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
            context.chronos.vortex(),
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
#[cfg(feature = "nova")]
#[derive(Debug)]
pub struct CuriosityCommand;

#[cfg(feature = "nova")]
#[async_trait]
impl ShellCommand for CuriosityCommand {
    fn name(&self) -> &str {
        "curiosity"
    }

    fn description(&self) -> &str {
        "Ask the Curiosity Engine"
    }

    async fn execute(&self, _args: &[String], context: &CommandContext) -> Result<CommandResult> {
        let loaded_models = context.chronos.vortex().list_loaded_models();
        if let Some((handle, _)) = loaded_models.first() {
            let curiosity = Curiosity::new(
                Arc::clone(&context.gallifrey),
                context.chronos.vortex(),
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
#[cfg(feature = "nova")]
#[derive(Debug)]
pub struct CapsuleCommand;

#[cfg(feature = "nova")]
#[async_trait]
impl ShellCommand for CapsuleCommand {
    fn name(&self) -> &str {
        "capsule"
    }

    fn description(&self) -> &str {
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
