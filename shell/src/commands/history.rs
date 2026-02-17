//! History command.

use crate::command_pattern::{CommandResult, ShellCommand, ShellContext};
use anyhow::Result;
use async_trait::async_trait;

/// The `HistoryCommand` command.
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

    fn usage(&self) -> &str {
        "history"
    }

    async fn execute(&self, ctx: &ShellContext<'_>, _args: &[String]) -> Result<CommandResult> {
        let messages = ctx.gallifrey.conversation().get_messages(ctx.session_id)?;

        if messages.is_empty() {
            println!("No messages in current session.");
        } else {
            println!("Session history ({} messages):", messages.len());
            println!();

            for msg in messages.iter().take(20) {
                let role = match msg.role {
                    tardis_gallifrey::stores::Role::User => "You",
                    tardis_gallifrey::stores::Role::Assistant => "Tardis",
                    tardis_gallifrey::stores::Role::System => "System",
                };

                let content = if msg.content.len() > 80 {
                    format!("{}...", &msg.content[..77])
                } else {
                    msg.content.clone()
                };

                println!(
                    "  [{}] {}: {}",
                    msg.timestamp.format("%H:%M"),
                    role,
                    content
                );
            }

            if messages.len() > 20 {
                println!("  ... and {} more", messages.len() - 20);
            }
        }
        Ok(CommandResult::Ok)
    }
}
