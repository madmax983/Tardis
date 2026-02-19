//! REPL (Read-Eval-Print Loop) for the Tardis shell.

use crate::commands;
use crate::commands::registry::CommandRegistry;
use crate::commands::traits::{CommandContext, CommandResult};
use crate::router::{Intent, Router};
use anyhow::Result;
use rustyline::error::ReadlineError;
use rustyline::{DefaultEditor, Result as RlResult};
use std::sync::Arc;
use tardis_chronos::{Chronos, RagConfig};
use tardis_common::SessionId;
use tardis_gallifrey::Gallifrey;
use tardis_telemetry::gallifrey::TelemetryStore;
use tracing::{error, info};

#[cfg(feature = "nova")]
use tardis_chronos::experimental::prophecy::Prophet;

/// The main REPL for Tardis shell.
#[derive(Debug)]
pub struct Repl {
    /// Readline editor.
    editor: DefaultEditor,
    /// Router for intent classification.
    router: Router,
    /// Command registry.
    registry: CommandRegistry,
    /// Chronos RAG engine.
    chronos: Arc<Chronos>,
    /// Gallifrey database.
    gallifrey: Arc<Gallifrey>,
    /// Telemetry store (optional).
    #[allow(dead_code)]
    telemetry_store: Option<Arc<TelemetryStore>>,
    /// Prophet engine (optional, Nova only).
    #[cfg(feature = "nova")]
    prophet: Option<Arc<Prophet>>,
    /// Current session ID.
    session_id: SessionId,
    /// Whether to continue running.
    running: bool,
}

impl Repl {
    /// Create a new REPL.
    ///
    /// # Errors
    ///
    /// Returns an error if initialization fails.
    pub fn new(
        chronos: Arc<Chronos>,
        gallifrey: Arc<Gallifrey>,
        telemetry_store: Option<Arc<TelemetryStore>>,
        #[cfg(feature = "nova")] prophet: Option<Arc<Prophet>>,
    ) -> Result<Self> {
        let editor = DefaultEditor::new()?;

        // Create a new session
        let session_id = gallifrey.conversation().create_session()?;
        info!("Created session: {}", session_id);

        let mut registry = CommandRegistry::new();
        Self::register_commands(&mut registry);

        Ok(Self {
            editor,
            router: Router::new(),
            registry,
            chronos,
            gallifrey,
            telemetry_store,
            #[cfg(feature = "nova")]
            prophet,
            session_id,
            running: true,
        })
    }

    fn register_commands(registry: &mut CommandRegistry) {
        // System
        registry.register(Box::new(commands::system::HelpCommand));
        registry.register(Box::new(commands::system::ExitCommand));
        registry.register(Box::new(commands::system::ClearCommand));
        registry.register(Box::new(commands::system::ModelsCommand));
        registry.register(Box::new(commands::system::ContextCommand));

        // Knowledge
        registry.register(Box::new(commands::knowledge::HistoryCommand));
        registry.register(Box::new(commands::knowledge::RememberCommand));
        registry.register(Box::new(commands::knowledge::RecallCommand));
        registry.register(Box::new(commands::knowledge::SnapshotCommand));
        registry.register(Box::new(commands::knowledge::RestoreCommand));

        // Experimental
        #[cfg(feature = "nova")]
        {
            registry.register(Box::new(commands::experimental::SonicCommand));
            registry.register(Box::new(commands::experimental::FixCommand));
            registry.register(Box::new(commands::experimental::DashboardCommand));
            registry.register(Box::new(commands::experimental::TimelineCommand));
            registry.register(Box::new(commands::experimental::MapCommand));
            registry.register(Box::new(commands::experimental::HeatmapCommand));
            registry.register(Box::new(commands::experimental::BiographerCommand));
            registry.register(Box::new(commands::experimental::CuriosityCommand));
            registry.register(Box::new(commands::experimental::CapsuleCommand));
            registry.register(Box::new(commands::experimental::DreamCommand));
        }
    }

    /// Run the REPL loop.
    ///
    /// # Errors
    ///
    /// Returns an error if the REPL encounters a fatal error.
    pub async fn run(&mut self) -> Result<()> {
        while self.running {
            match self.read_line() {
                Ok(line) => {
                    if !line.trim().is_empty() {
                        self.process_input(&line).await;
                    }
                }
                Err(ReadlineError::Interrupted) => {
                    println!("^C");
                }
                Err(ReadlineError::Eof) => {
                    println!("Goodbye!");
                    break;
                }
                Err(err) => {
                    error!("Readline error: {}", err);
                    break;
                }
            }
        }

        // End session
        self.gallifrey.conversation().end_session(self.session_id)?;

        Ok(())
    }

    /// Read a line from the user.
    fn read_line(&mut self) -> RlResult<String> {
        self.editor.readline("tardis> ")
    }

    /// Process user input.
    async fn process_input(&mut self, input: &str) {
        // Add to history
        let _ = self.editor.add_history_entry(input);

        // Route the input
        let intent = self.router.route(input);

        match intent {
            Intent::BuiltinCommand { command, args } => {
                self.handle_builtin(&command, &args).await;
            }
            Intent::ShellCommand { command } => {
                self.handle_shell_command(&command).await;
            }
            Intent::ChronosQuery { query, .. } => {
                self.handle_chronos_query(&query).await;
            }
            Intent::TimeTravel { timestamp, query } => {
                self.handle_time_travel(&timestamp, &query).await;
            }
            Intent::DirectQuery { query } => {
                self.handle_direct_query(&query).await;
            }
        }
    }

    /// Handle a built-in command.
    async fn handle_builtin(&mut self, command: &str, args: &[String]) {
        if let Some(cmd) = self.registry.get(command) {
            let context = CommandContext {
                gallifrey: self.gallifrey.clone(),
                chronos: self.chronos.clone(),
                telemetry: self.telemetry_store.clone(),
                #[cfg(feature = "nova")]
                prophet: self.prophet.clone(),
                session_id: self.session_id,
            };

            match cmd.execute(args, &context).await {
                Ok(CommandResult::Continue) => {}
                Ok(CommandResult::Quit) => {
                    self.running = false;
                }
                Err(e) => println!("Command failed: {e}"),
            }
        } else {
            println!("Unknown command: {command}. Type 'help' for available commands.");
        }
    }

    /// Handle a shell command (prefixed with !).
    #[allow(clippy::unused_async)]
    async fn handle_shell_command(&self, command: &str) {
        println!("[Shell command: {command}]");
        println!("Shell commands not yet implemented.");
    }

    /// Handle a Chronos RAG query.
    async fn handle_chronos_query(&self, query: &str) {
        let config = RagConfig {
            session_id: Some(self.session_id),
            ..RagConfig::default()
        };

        match self.chronos.query(query, config).await {
            Ok(response) => {
                println!();
                println!("{}", response.text);
                if !response.sources.is_empty() {
                    println!();
                    println!("[Sources: {} items]", response.sources.len());
                }
                println!();
            }
            Err(e) => {
                println!("Query failed: {e}");
            }
        }
    }

    /// Handle a time-travel query (prefixed with @).
    #[allow(clippy::unused_async)]
    async fn handle_time_travel(&self, timestamp: &str, query: &str) {
        println!("[Time travel to {timestamp} with query: {query}]");
        println!("Time travel not yet fully implemented.");
    }

    /// Handle a direct LLM query (prefixed with ?).
    #[allow(clippy::unused_async)]
    async fn handle_direct_query(&self, query: &str) {
        println!("[Direct query: {query}]");
        println!("Direct queries not yet implemented.");
    }
}
