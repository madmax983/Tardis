//! Help command.

use crate::command_pattern::{CommandResult, ShellCommand, ShellContext};
use anyhow::Result;
use async_trait::async_trait;

/// The `HelpCommand` command.
#[derive(Debug)]
pub struct HelpCommand;

#[async_trait]
impl ShellCommand for HelpCommand {
    fn name(&self) -> &str {
        "help"
    }

    fn description(&self) -> &str {
        "Show help information"
    }

    fn usage(&self) -> &str {
        "help [command]"
    }

    async fn execute(&self, ctx: &ShellContext<'_>, args: &[String]) -> Result<CommandResult> {
        if args.is_empty() {
            // General help
            println!("Tardis Shell Commands:");
            println!();
            println!("  BUILT-IN COMMANDS:");

            // Sort commands for consistent output
            let mut commands: Vec<_> = ctx.registry.commands().collect();
            commands.sort_by_key(|c| c.name());

            for cmd in commands {
                println!("    {:15} {}", cmd.name(), cmd.description());
            }
            println!();
            println!("  INPUT MODES:");
            println!("    <query>           Natural language (default) -> RAG response");
            println!("    !<command>        Shell command (e.g., !ls -la)");
            println!("    @<time> <query>   Time-travel query (e.g., @yesterday what did we discuss)");
            println!("    ?<query>          Direct LLM query (no RAG)");
            println!();
            println!("  EXAMPLES:");
            println!("    > What did we discuss about Rust?");
            println!("    > remember that the API uses JWT tokens");
            println!("    > @last-week show file changes");
            println!("    > !cargo build");
            println!("    > ?Write a poem about time travel");
            println!();
        } else {
            // Specific command help
            let cmd_name = &args[0];
            if let Some(cmd) = ctx.registry.get(cmd_name) {
                println!("Command: {}", cmd.name());
                println!("Description: {}", cmd.description());
                println!("Usage: {}", cmd.usage());
            } else {
                println!("Unknown command: '{}'", cmd_name);
            }
        }
        Ok(CommandResult::Ok)
    }
}
