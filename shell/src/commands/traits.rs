use anyhow::Result;
use async_trait::async_trait;
use std::fmt;
use std::sync::Arc;
use tardis_chronos::Chronos;
use tardis_common::SessionId;
use tardis_gallifrey::Gallifrey;
use tardis_telemetry::gallifrey::TelemetryStore;

#[cfg(feature = "nova")]
use tardis_chronos::experimental::prophecy::Prophet;

/// Context provided to every shell command.
pub struct CommandContext {
    /// Access to the Knowledge Graph.
    pub gallifrey: Arc<Gallifrey>,
    /// Access to the RAG Engine.
    pub chronos: Arc<Chronos>,
    /// Access to Telemetry (optional).
    pub telemetry: Option<Arc<TelemetryStore>>,
    /// Access to the Prophet Engine (optional, Nova only).
    #[cfg(feature = "nova")]
    pub prophet: Option<Arc<Prophet>>,
    /// The current session ID.
    pub session_id: SessionId,
}

impl fmt::Debug for CommandContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut d = f.debug_struct("CommandContext");
        d.field("gallifrey", &"Arc<Gallifrey>")
            .field("chronos", &"Arc<Chronos>")
            .field("telemetry", &self.telemetry.is_some());

        #[cfg(feature = "nova")]
        d.field("prophet", &self.prophet.is_some());

        d.field("session_id", &self.session_id).finish()
    }
}

/// Result of a command execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandResult {
    /// Continue the shell loop.
    Continue,
    /// Exit the shell loop.
    Quit,
}

/// A shell command.
#[async_trait]
pub trait ShellCommand: Send + Sync {
    /// The name of the command (e.g., "help", "history").
    fn name(&self) -> &'static str;

    /// A short description of the command.
    fn description(&self) -> &'static str;

    /// Execute the command.
    async fn execute(&self, args: &[String], context: &CommandContext) -> Result<CommandResult>;
}
