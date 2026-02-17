//! System command.

use crate::command_pattern::{CommandResult, ShellCommand, ShellContext};
use anyhow::Result;
use async_trait::async_trait;

/// The `ExitCommand` command.
#[derive(Debug)]
pub struct ExitCommand;

#[async_trait]
impl ShellCommand for ExitCommand {
    fn name(&self) -> &str {
        "exit"
    }

    fn description(&self) -> &str {
        "Exit the shell"
    }

    fn usage(&self) -> &str {
        "exit"
    }

    async fn execute(&self, _ctx: &ShellContext<'_>, _args: &[String]) -> Result<CommandResult> {
        println!("Goodbye!");
        Ok(CommandResult::Exit)
    }
}

/// The `QuitCommand` command.
#[derive(Debug)]
pub struct QuitCommand;

#[async_trait]
impl ShellCommand for QuitCommand {
    fn name(&self) -> &str {
        "quit"
    }

    fn description(&self) -> &str {
        "Exit the shell (alias for exit)"
    }

    fn usage(&self) -> &str {
        "quit"
    }

    async fn execute(&self, _ctx: &ShellContext<'_>, _args: &[String]) -> Result<CommandResult> {
        println!("Goodbye!");
        Ok(CommandResult::Exit)
    }
}

/// The `ClearCommand` command.
#[derive(Debug)]
pub struct ClearCommand;

#[async_trait]
impl ShellCommand for ClearCommand {
    fn name(&self) -> &str {
        "clear"
    }

    fn description(&self) -> &str {
        "Clear the screen"
    }

    fn usage(&self) -> &str {
        "clear"
    }

    async fn execute(&self, _ctx: &ShellContext<'_>, _args: &[String]) -> Result<CommandResult> {
        print!("\x1B[2J\x1B[1;1H");
        Ok(CommandResult::ClearScreen)
    }
}
