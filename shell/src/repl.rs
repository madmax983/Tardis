//! REPL (Read-Eval-Print Loop) for the Tardis shell.

use crate::commands::CommandHandler;
use crate::router::{Intent, Router};
use anyhow::Result;
use rustyline::error::ReadlineError;
use rustyline::{DefaultEditor, Result as RlResult};
use std::sync::Arc;
use tardis_chronos::{Chronos, RagConfig};
use tardis_common::SessionId;
use tardis_gallifrey::Gallifrey;
use tracing::{error, info};

/// The main REPL for Tardis shell.
pub struct Repl {
    /// Readline editor.
    editor: DefaultEditor,
    /// Router for intent classification.
    router: Router,
    /// Command handler for built-in commands.
    commands: CommandHandler,
    /// Chronos RAG engine.
    chronos: Arc<Chronos>,
    /// Gallifrey database.
    gallifrey: Arc<Gallifrey>,
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
    pub fn new(chronos: Arc<Chronos>, gallifrey: Arc<Gallifrey>) -> Result<Self> {
        let editor = DefaultEditor::new()?;

        // Create a new session
        let session_id = gallifrey.conversation().create_session()?;
        info!("Created session: {}", session_id);

        Ok(Self {
            editor,
            router: Router::new(),
            commands: CommandHandler::new(),
            chronos,
            gallifrey,
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
                    continue;
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
        match command {
            "help" => self.commands.help(args),
            "exit" | "quit" => {
                println!("Goodbye!");
                self.running = false;
            }
            "history" => self.commands.history(&self.gallifrey, self.session_id),
            "remember" => {
                let content = args.join(" ");
                match self.chronos.remember(&content, tardis_chronos::MemoryCategory::Knowledge).await {
                    Ok(id) => println!("Remembered: {}", id),
                    Err(e) => println!("Failed to remember: {}", e),
                }
            }
            "recall" => {
                let query = args.join(" ");
                match self.chronos.recall(&query, 5).await {
                    Ok(results) => {
                        for result in results {
                            println!("- {}", result.content);
                        }
                    }
                    Err(e) => println!("Failed to recall: {}", e),
                }
            }
            "models" => self.commands.list_models(),
            "context" => self.commands.show_context(self.session_id),
            "clear" => {
                print!("\x1B[2J\x1B[1;1H");
            }
            _ => println!("Unknown command: {}. Type 'help' for available commands.", command),
        }
    }

    /// Handle a shell command (prefixed with !).
    async fn handle_shell_command(&self, command: &str) {
        println!("[Shell command: {}]", command);
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
                println!("Query failed: {}", e);
            }
        }
    }

    /// Handle a time-travel query (prefixed with @).
    async fn handle_time_travel(&self, timestamp: &str, query: &str) {
        println!("[Time travel to {} with query: {}]", timestamp, query);
        println!("Time travel not yet fully implemented.");
    }

    /// Handle a direct LLM query (prefixed with ?).
    async fn handle_direct_query(&self, query: &str) {
        println!("[Direct query: {}]", query);
        println!("Direct queries not yet implemented.");
    }
}
