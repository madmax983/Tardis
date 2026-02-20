use crate::commands::traits::{CommandContext, CommandResult, ShellCommand};
use anyhow::Result;
use async_trait::async_trait;

/// Command to display help information.
#[derive(Debug)]
pub struct HelpCommand;

#[async_trait]
impl ShellCommand for HelpCommand {
    fn name(&self) -> &'static str {
        "help"
    }

    fn description(&self) -> &'static str {
        "Show usage information"
    }

    async fn execute(&self, args: &[String], _context: &CommandContext) -> Result<CommandResult> {
        // Delegate to existing help logic in mod.rs for now
        crate::commands::help(args);
        Ok(CommandResult::Continue)
    }
}

/// Command to exit the shell.
#[derive(Debug)]
pub struct ExitCommand;

#[async_trait]
impl ShellCommand for ExitCommand {
    fn name(&self) -> &'static str {
        "exit"
    }

    fn description(&self) -> &'static str {
        "Exit the shell"
    }

    async fn execute(&self, _args: &[String], _context: &CommandContext) -> Result<CommandResult> {
        println!("Goodbye!");
        Ok(CommandResult::Quit)
    }
}

/// Command to clear the screen.
#[derive(Debug)]
pub struct ClearCommand;

#[async_trait]
impl ShellCommand for ClearCommand {
    fn name(&self) -> &'static str {
        "clear"
    }

    fn description(&self) -> &'static str {
        "Clear the screen"
    }

    async fn execute(&self, _args: &[String], _context: &CommandContext) -> Result<CommandResult> {
        print!("\x1B[2J\x1B[1;1H");
        Ok(CommandResult::Continue)
    }
}

/// Command to list available models.
#[derive(Debug)]
pub struct ModelsCommand;

#[async_trait]
impl ShellCommand for ModelsCommand {
    fn name(&self) -> &'static str {
        "models"
    }

    fn description(&self) -> &'static str {
        "List available LLM models"
    }

    async fn execute(&self, _args: &[String], _context: &CommandContext) -> Result<CommandResult> {
        crate::commands::list_models();
        Ok(CommandResult::Continue)
    }
}

/// Command to show context.
#[derive(Debug)]
pub struct ContextCommand;

#[async_trait]
impl ShellCommand for ContextCommand {
    fn name(&self) -> &'static str {
        "context"
    }

    fn description(&self) -> &'static str {
        "Show current session context"
    }

    async fn execute(&self, _args: &[String], context: &CommandContext) -> Result<CommandResult> {
        crate::commands::show_context(context.session_id);
        Ok(CommandResult::Continue)
    }
}
