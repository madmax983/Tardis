use crate::command_pattern::{CommandResult, ShellCommand, ShellContext};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use crate::experimental::chronograph::ChronoGraph;

/// The `TimelineCommand` command.
#[derive(Debug)]
pub struct TimelineCommand;

#[async_trait]
impl ShellCommand for TimelineCommand {
    fn name(&self) -> &str {
        "timeline"
    }

    fn description(&self) -> &str {
        "Show timeline of an entity"
    }

    fn usage(&self) -> &str {
        "timeline <entity_name>"
    }

    async fn execute(&self, ctx: &ShellContext<'_>, args: &[String]) -> Result<CommandResult> {
        if args.is_empty() {
             println!("Usage: {}", self.usage());
             return Ok(CommandResult::Ok);
        }

        let entity_name = &args[0];
        let graph = ChronoGraph::new(Arc::clone(&ctx.gallifrey));

        match graph.generate_timeline(entity_name) {
             Ok(timeline) => println!("{timeline}"),
             Err(e) => println!("Failed to generate timeline: {e}"),
        }
        Ok(CommandResult::Ok)
    }
}
