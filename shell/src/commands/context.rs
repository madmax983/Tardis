//! Context command.

use crate::command_pattern::{CommandResult, ShellCommand, ShellContext};
use anyhow::Result;
use async_trait::async_trait;

/// The `ContextCommand` command.
#[derive(Debug)]
pub struct ContextCommand;

#[async_trait]
impl ShellCommand for ContextCommand {
    fn name(&self) -> &str {
        "context"
    }

    fn description(&self) -> &str {
        "Inspect or modify session context"
    }

    fn usage(&self) -> &str {
        "context [subcommand]"
    }

    async fn execute(&self, ctx: &ShellContext<'_>, args: &[String]) -> Result<CommandResult> {
        if args.is_empty() {
             println!("Current Context:");
             println!();
             println!("  Session ID: {}", ctx.session_id);

             let loaded = ctx.chronos.vortex().list_loaded_models();
             if let Some((handle, info)) = loaded.first() {
                 println!("  Model: [{}] {}", handle, info.name);
             } else {
                 println!("  Model: (none loaded)");
             }

             println!("  Project: (none set)");
             println!();
             println!("Use 'context project:<name>' to set project context.");
        } else {
            // Handle subcommands if any
            let sub = &args[0];
            println!("Subcommand '{}' not implemented yet.", sub);
        }
        Ok(CommandResult::Ok)
    }
}
