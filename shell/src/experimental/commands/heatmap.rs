use crate::command_pattern::{CommandResult, ShellCommand, ShellContext};
use anyhow::Result;
use async_trait::async_trait;
use crate::experimental::heatmap_cmd;

/// The `HeatmapCommand` command.
#[derive(Debug)]
pub struct HeatmapCommand;

#[async_trait]
impl ShellCommand for HeatmapCommand {
    fn name(&self) -> &str {
        "heatmap"
    }

    fn description(&self) -> &str {
        "Visualize temporal history as a heatmap"
    }

    fn usage(&self) -> &str {
        "heatmap <entity_name>"
    }

    async fn execute(&self, ctx: &ShellContext<'_>, args: &[String]) -> Result<CommandResult> {
        match heatmap_cmd::run(&ctx.gallifrey, args) {
             Ok(report) => println!("{report}"),
             Err(e) => println!("Failed to generate heatmap: {e}"),
        }
        Ok(CommandResult::Ok)
    }
}
