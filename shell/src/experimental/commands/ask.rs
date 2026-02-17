use crate::command_pattern::{CommandResult, ShellCommand, ShellContext};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use tardis_chronos::experimental::curiosity::Curiosity;

/// The `AskCommand` command.
#[derive(Debug)]
pub struct AskCommand;

#[async_trait]
impl ShellCommand for AskCommand {
    fn name(&self) -> &str {
        "ask"
    }

    fn description(&self) -> &str {
        "Ask a clarifying question to fill knowledge gaps"
    }

    fn usage(&self) -> &str {
        "ask"
    }

    async fn execute(&self, ctx: &ShellContext<'_>, _args: &[String]) -> Result<CommandResult> {
        let loaded_models = ctx.chronos.vortex().list_loaded_models();
        if let Some((handle, _)) = loaded_models.first() {
            let curiosity = Curiosity::new(
                Arc::clone(&ctx.gallifrey),
                ctx.chronos.vortex(),
                *handle,
            );

            println!("🤔 Curiosity is scanning...");
            match curiosity.ask().await {
                 Ok(question) => println!("{question}"),
                 Err(e) => println!("Curiosity failed: {e}"),
            }
        } else {
            println!("Curiosity needs a loaded model. Use 'models load <path>'.");
        }
        Ok(CommandResult::Ok)
    }
}
