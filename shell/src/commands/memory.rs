//! Memory command.

use crate::command_pattern::{CommandResult, ShellCommand, ShellContext};
use anyhow::Result;
use async_trait::async_trait;

/// The `RememberCommand` command.
#[derive(Debug)]
pub struct RememberCommand;

#[async_trait]
impl ShellCommand for RememberCommand {
    fn name(&self) -> &str {
        "remember"
    }

    fn description(&self) -> &str {
        "Store information for later recall"
    }

    fn usage(&self) -> &str {
        "remember <text>"
    }

    async fn execute(&self, ctx: &ShellContext<'_>, args: &[String]) -> Result<CommandResult> {
        if args.is_empty() {
            println!("Usage: {}", self.usage());
            return Ok(CommandResult::Ok);
        }

        let content = args.join(" ");
        match ctx
            .chronos
            .remember(&content, tardis_chronos::MemoryCategory::Knowledge)
            .await
        {
            Ok(id) => println!("Remembered: {id}"),
            Err(e) => println!("Failed to remember: {e}"),
        }
        Ok(CommandResult::Ok)
    }
}

/// The `RecallCommand` command.
#[derive(Debug)]
pub struct RecallCommand;

#[async_trait]
impl ShellCommand for RecallCommand {
    fn name(&self) -> &str {
        "recall"
    }

    fn description(&self) -> &str {
        "Retrieve stored memories"
    }

    fn usage(&self) -> &str {
        "recall <query>"
    }

    async fn execute(&self, ctx: &ShellContext<'_>, args: &[String]) -> Result<CommandResult> {
        if args.is_empty() {
             println!("Usage: {}", self.usage());
             return Ok(CommandResult::Ok);
        }

        let query = args.join(" ");
        match ctx.chronos.recall(&query, 5).await {
            Ok(results) => {
                for result in results {
                    println!("- {}", result.content);
                }
            }
            Err(e) => println!("Failed to recall: {e}"),
        }
        Ok(CommandResult::Ok)
    }
}
