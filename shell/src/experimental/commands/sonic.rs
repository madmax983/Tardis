use crate::command_pattern::{CommandResult, ShellCommand, ShellContext};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use crate::experimental::sonic::SonicScrewdriver;
use std::path::Path;

async fn run_sonic(ctx: &ShellContext<'_>, args: &[String], force_fix: bool) -> Result<CommandResult> {
    if args.is_empty() {
        println!("Usage: sonic <file> | sonic repair <file> | sonic diagnose");
        return Ok(CommandResult::Ok);
    }

    let loaded_models = ctx.chronos.vortex().list_loaded_models();
    let model_handle = loaded_models.first().map(|(h, _)| *h);

    let screwdriver = SonicScrewdriver::new(
        ctx.telemetry_store.clone(),
        Some(Arc::clone(&ctx.gallifrey)),
        Some(ctx.chronos.vortex()),
        model_handle,
    );

    if args[0] == "diagnose" {
        println!("{}", screwdriver.buzz());
        match screwdriver.diagnose().await {
            Ok(report) => println!("{report}"),
            Err(e) => println!("Diagnosis failed: {e}"),
        }
        return Ok(CommandResult::Ok);
    }

    let path_str = &args[args.len() - 1];
    let path = Path::new(path_str);

    let repair = force_fix || (args.len() > 1 && (args[0] == "repair" || args[0] == "fix"));

    let result = if repair {
        screwdriver.repair(path)
    } else {
        screwdriver.inspect(path)
    };

    match result {
        Ok(report) => println!("{report}"),
        Err(e) => println!("Sonic Screwdriver error: {e}"),
    }
    Ok(CommandResult::Ok)
}

/// The `SonicCommand` command.
#[derive(Debug)]
pub struct SonicCommand;

#[async_trait]
impl ShellCommand for SonicCommand {
    fn name(&self) -> &str {
        "sonic"
    }

    fn description(&self) -> &str {
        "System diagnosis and file repair tool"
    }

    fn usage(&self) -> &str {
        "sonic <file> | sonic repair <file> | sonic diagnose"
    }

    async fn execute(&self, ctx: &ShellContext<'_>, args: &[String]) -> Result<CommandResult> {
        run_sonic(ctx, args, false).await
    }
}

/// The `FixCommand` command.
#[derive(Debug)]
pub struct FixCommand;

#[async_trait]
impl ShellCommand for FixCommand {
    fn name(&self) -> &str {
        "fix"
    }

    fn description(&self) -> &str {
        "Repair a file (alias for sonic repair)"
    }

    fn usage(&self) -> &str {
        "fix <file>"
    }

    async fn execute(&self, ctx: &ShellContext<'_>, args: &[String]) -> Result<CommandResult> {
        run_sonic(ctx, args, true).await
    }
}
