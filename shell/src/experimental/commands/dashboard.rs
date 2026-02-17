use crate::command_pattern::{CommandResult, ShellCommand, ShellContext};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

/// The `DashboardCommand` command.
#[derive(Debug)]
pub struct DashboardCommand;

#[async_trait]
impl ShellCommand for DashboardCommand {
    fn name(&self) -> &str {
        "dashboard"
    }

    fn description(&self) -> &str {
        "Launch the TUI dashboard"
    }

    fn usage(&self) -> &str {
        "dashboard"
    }

    async fn execute(&self, ctx: &ShellContext<'_>, _args: &[String]) -> Result<CommandResult> {
        let dashboard_result = crate::dashboard::tui::Dashboard::new(
            Arc::clone(&ctx.gallifrey),
            ctx.telemetry_store.clone(),
            ctx.prophet.clone(),
        );

        match dashboard_result {
            Ok(mut dashboard) => {
                if let Err(e) = dashboard.run() {
                     println!("Dashboard failed: {e}");
                }
            }
            Err(e) => println!("Failed to initialize dashboard: {e}"),
        }
        Ok(CommandResult::Ok)
    }
}
