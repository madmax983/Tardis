//! Models command.

use crate::command_pattern::{CommandResult, ShellCommand, ShellContext};
use anyhow::Result;
use async_trait::async_trait;

/// The `ModelsCommand` command.
#[derive(Debug)]
pub struct ModelsCommand;

#[async_trait]
impl ShellCommand for ModelsCommand {
    fn name(&self) -> &str {
        "models"
    }

    fn description(&self) -> &str {
        "List available LLM models"
    }

    fn usage(&self) -> &str {
        "models"
    }

    async fn execute(&self, ctx: &ShellContext<'_>, _args: &[String]) -> Result<CommandResult> {
        // Access via Chronos -> Vortex
        let loaded = ctx.chronos.vortex().list_loaded_models();

        println!("Available models:");
        println!();
        if loaded.is_empty() {
             println!("  (No models loaded yet)");
        } else {
             for (handle, info) in loaded {
                 println!("  - [{}] {}", handle, info.path.display());
             }
        }
        println!();
        println!("Use 'models load <path>' to load a model.");
        Ok(CommandResult::Ok)
    }
}
