//! Shell command pattern definitions.
//!
//! Defines the `ShellCommand` trait, `CommandRegistry`, and execution context.

use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tardis_chronos::Chronos;
use tardis_common::SessionId;
use tardis_gallifrey::Gallifrey;
use tardis_telemetry::gallifrey::TelemetryStore;

#[cfg(feature = "nova")]
use tardis_chronos::experimental::prophecy::Prophet;

/// Registry for shell commands.
#[derive(Default)]
pub struct CommandRegistry {
    commands: HashMap<String, Box<dyn ShellCommand>>,
}

impl std::fmt::Debug for CommandRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommandRegistry")
            .field("commands", &self.commands.keys())
            .finish()
    }
}

impl CommandRegistry {
    /// Create a new registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            commands: HashMap::new(),
        }
    }

    /// Register a command.
    pub fn register(&mut self, command: Box<dyn ShellCommand>) {
        self.commands.insert(command.name().to_string(), command);
    }

    /// Get a command by name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&dyn ShellCommand> {
        self.commands.get(name).map(std::boxed::Box::as_ref)
    }

    /// Get all command names.
    #[must_use]
    pub fn names(&self) -> Vec<String> {
        self.commands.keys().cloned().collect()
    }

    /// Get all commands (for help listing).
    pub fn commands(&self) -> impl Iterator<Item = &dyn ShellCommand> {
        self.commands.values().map(std::boxed::Box::as_ref)
    }
}

/// Context for command execution.
///
/// Holds references to the system services required by commands.
#[derive(Debug)]
pub struct ShellContext<'a> {
    /// The Gallifrey knowledge store.
    pub gallifrey: Arc<Gallifrey>,
    /// The Chronos RAG engine.
    pub chronos: Arc<Chronos>,
    /// The Telemetry store (optional).
    pub telemetry_store: Option<Arc<TelemetryStore>>,
    /// The Prophet engine (optional, Nova only).
    #[cfg(feature = "nova")]
    pub prophet: Option<Arc<Prophet>>,
    /// The current user session ID.
    pub session_id: SessionId,
    /// Reference to the command registry (for help, etc.).
    pub registry: &'a CommandRegistry,
}

/// Result of a command execution.
#[derive(Debug, PartialEq, Eq)]
pub enum CommandResult {
    /// Command completed successfully.
    Ok,
    /// Command requested to exit the shell.
    Exit,
    /// Command requested to clear the screen.
    ClearScreen,
}

/// A shell command.
#[async_trait]
pub trait ShellCommand: Send + Sync {
    /// The name of the command.
    fn name(&self) -> &str;

    /// A short description of the command.
    fn description(&self) -> &str;

    /// Usage instructions.
    fn usage(&self) -> &str;

    /// Execute the command.
    async fn execute(&self, ctx: &ShellContext<'_>, args: &[String]) -> Result<CommandResult>;
}
