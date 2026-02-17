//! REPL (Read-Eval-Print Loop) for the Tardis shell.

use crate::command_pattern::{CommandRegistry, CommandResult, ShellContext};
use crate::commands;
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
    /// Command registry.
    registry: CommandRegistry,
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

        // Register built-ins
        registry.register(Box::new(commands::help::HelpCommand));
        registry.register(Box::new(commands::history::HistoryCommand));
        registry.register(Box::new(commands::memory::RememberCommand));
        registry.register(Box::new(commands::memory::RecallCommand));
        registry.register(Box::new(commands::models::ModelsCommand));
        registry.register(Box::new(commands::context::ContextCommand));
        registry.register(Box::new(commands::system::ExitCommand));
        registry.register(Box::new(commands::system::QuitCommand));
        registry.register(Box::new(commands::system::ClearCommand));

        // Register experimental
        #[cfg(feature = "nova")]
        {
            use crate::experimental::commands::*;
            registry.register(Box::new(dashboard::DashboardCommand));
            registry.register(Box::new(timeline::TimelineCommand));
            registry.register(Box::new(map::MapCommand));
            registry.register(Box::new(heatmap::HeatmapCommand));
            registry.register(Box::new(biography::BiographyCommand));
            registry.register(Box::new(sonic::SonicCommand));
            registry.register(Box::new(sonic::FixCommand));
            registry.register(Box::new(ask::AskCommand));
            registry.register(Box::new(capsule::CapsuleCommand));
        }

        // Initialize Router with registered commands
        let router = Router::with_builtins(registry.names());

        Ok(Self {
            editor,
            router,
            chronos,
            gallifrey,
            telemetry_store,
            #[cfg(feature = "nova")]
            prophet,
            registry,
            session_id,
            running: true,
        })
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
    async fn handle_builtin(&mut self, command_name: &str, args: &[String]) {
        if let Some(command) = self.registry.get(command_name) {
            let ctx = ShellContext {
                gallifrey: Arc::clone(&self.gallifrey),
                chronos: Arc::clone(&self.chronos),
                telemetry_store: self.telemetry_store.clone(),
                #[cfg(feature = "nova")]
                prophet: self.prophet.clone(),
                session_id: self.session_id,
                registry: &self.registry,
            };

            match command.execute(&ctx, args).await {
                Ok(result) => match result {
                    CommandResult::Ok => {}
                    CommandResult::Exit => {
                        self.running = false;
                    }
                    CommandResult::ClearScreen => {
                        // ClearCommand prints escape codes itself.
                    }
                },
                Err(e) => {
                    println!("Error executing command '{}': {}", command_name, e);
                }
            }
        } else {
             println!("Unknown command: {command_name}. Type 'help' for available commands.");
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
