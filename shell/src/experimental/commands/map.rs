use crate::command_pattern::{CommandResult, ShellCommand, ShellContext};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use crate::experimental::chronograph::ChronoGraph;

/// The `MapCommand` command.
#[derive(Debug)]
pub struct MapCommand;

#[async_trait]
impl ShellCommand for MapCommand {
    fn name(&self) -> &str {
        "map"
    }

    fn description(&self) -> &str {
        "Visualize entity relationships"
    }

    fn usage(&self) -> &str {
        "map <entity_name> [depth]"
    }

    async fn execute(&self, ctx: &ShellContext<'_>, args: &[String]) -> Result<CommandResult> {
        if args.is_empty() {
             println!("Usage: {}", self.usage());
             return Ok(CommandResult::Ok);
        }

        let entity_name = &args[0];
        let depth = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(2);

        let graph = ChronoGraph::new(Arc::clone(&ctx.gallifrey));

        match graph.generate_map(entity_name, depth, None) {
             Ok(map) => println!("{map}"),
             Err(e) => println!("Failed to generate map: {e}"),
        }
        Ok(CommandResult::Ok)
    }
}
