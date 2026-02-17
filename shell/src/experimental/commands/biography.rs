use crate::command_pattern::{CommandResult, ShellCommand, ShellContext};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use crate::experimental::biographer::Biographer;

/// The `BiographyCommand` command.
#[derive(Debug)]
pub struct BiographyCommand;

#[async_trait]
impl ShellCommand for BiographyCommand {
    fn name(&self) -> &str {
        "biography"
    }

    fn description(&self) -> &str {
        "Generate a narrative biography for an entity"
    }

    fn usage(&self) -> &str {
        "biography <entity_name>"
    }

    async fn execute(&self, ctx: &ShellContext<'_>, args: &[String]) -> Result<CommandResult> {
        if args.is_empty() {
             println!("Usage: {}", self.usage());
             return Ok(CommandResult::Ok);
        }
        let entity_name = args.join(" ");

        let loaded_models = ctx.chronos.vortex().list_loaded_models();
        let model_handle = loaded_models.first().map(|(h, _)| *h);

        let biographer = Biographer::new(
            Arc::clone(&ctx.gallifrey),
            ctx.chronos.vortex(),
            model_handle,
        );

        match biographer.biography(&entity_name).await {
             Ok(bio) => println!("\n{bio}\n"),
             Err(e) => println!("Failed to generate biography: {e}"),
        }
        Ok(CommandResult::Ok)
    }
}
