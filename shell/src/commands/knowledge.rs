use crate::commands::traits::{CommandContext, CommandResult, ShellCommand};
use anyhow::Result;
use async_trait::async_trait;

/// Command to show history.
#[derive(Debug)]
pub struct HistoryCommand;

#[async_trait]
impl ShellCommand for HistoryCommand {
    fn name(&self) -> &str {
        "history"
    }

    fn description(&self) -> &str {
        "Show conversation history"
    }

    async fn execute(&self, _args: &[String], context: &CommandContext) -> Result<CommandResult> {
        crate::commands::history(&context.gallifrey, context.session_id);
        Ok(CommandResult::Continue)
    }
}

/// Command to remember information.
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

    async fn execute(&self, args: &[String], context: &CommandContext) -> Result<CommandResult> {
        let content = args.join(" ");
        match context
            .chronos
            .remember(&content, tardis_chronos::MemoryCategory::Knowledge)
            .await
        {
            Ok(id) => println!("Remembered: {id}"),
            Err(e) => println!("Failed to remember: {e}"),
        }
        Ok(CommandResult::Continue)
    }
}

/// Command to recall information.
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

    async fn execute(&self, args: &[String], context: &CommandContext) -> Result<CommandResult> {
        let query = args.join(" ");
        match context.chronos.recall(&query, 5).await {
            Ok(results) => {
                for result in results {
                    println!("- {}", result.content);
                }
            }
            Err(e) => println!("Failed to recall: {e}"),
        }
        Ok(CommandResult::Continue)
    }
}

/// Command to create a system snapshot.
#[derive(Debug)]
pub struct SnapshotCommand;
#[async_trait]
impl ShellCommand for SnapshotCommand {
    fn name(&self) -> &str {
        "snapshot"
    }
    fn description(&self) -> &str {
        "Save system state snapshot (Not implemented)"
    }
    async fn execute(&self, _args: &[String], _context: &CommandContext) -> Result<CommandResult> {
        println!("Snapshot not yet implemented.");
        Ok(CommandResult::Continue)
    }
}

/// Command to restore a system snapshot.
#[derive(Debug)]
pub struct RestoreCommand;
#[async_trait]
impl ShellCommand for RestoreCommand {
    fn name(&self) -> &str {
        "restore"
    }
    fn description(&self) -> &str {
        "Restore a saved snapshot (Not implemented)"
    }
    async fn execute(&self, _args: &[String], _context: &CommandContext) -> Result<CommandResult> {
        println!("Restore not yet implemented.");
        Ok(CommandResult::Continue)
    }
}
