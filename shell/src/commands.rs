//! Built-in command handlers for the Tardis shell.
//!
//! This module implements the "standard library" of shell commands. These commands
//! are executed locally by the shell, rather than being sent to the RAG engine.
//!
//! # Supported Commands
//!
//! - `help`: Show usage information.
//! - `history`: Display conversation history.
//! - `remember` / `recall`: Manual memory management.
//! - `models`: List available LLMs.
//! - `context`: Inspect session state.

use std::sync::Arc;
use tardis_chronos::Chronos;
use tardis_common::SessionId;
use tardis_gallifrey::Gallifrey;

/// Handler for built-in shell commands.
///
/// The command handler is stateless but receives the application state (like `Gallifrey` instances)
/// during method calls to perform its duties.
#[derive(Debug)]
pub struct CommandHandler {
    // Configuration
}

impl CommandHandler {
    /// Create a new command handler.
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }

    /// Display help information.
    ///
    /// If an argument is provided, displays help for that specific command.
    /// Otherwise, displays the general help menu.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use tardis_shell::commands::CommandHandler;
    ///
    /// let handler = CommandHandler::new();
    /// handler.help(&[]); // General help
    /// handler.help(&["remember".to_string()]); // Help for 'remember'
    /// ```
    #[allow(clippy::unused_self)]
    pub fn help(&self, args: &[String]) {
        if args.is_empty() {
            Self::general_help();
        } else {
            Self::command_help(&args[0]);
        }
    }

    /// Display general help.
    fn general_help() {
        println!("Tardis Shell Commands:");
        println!();
        println!("  BUILT-IN COMMANDS:");
        println!("    help [command]    Show help (for a specific command)");
        println!("    history           Show conversation history");
        println!("    remember <text>   Store information for later recall");
        println!("    recall <query>    Retrieve stored memories");
        println!("    models            List available LLM models");
        println!("    context           Show current session context");
        println!("    snapshot <name>   Save system state snapshot");
        println!("    restore <name>    Restore a saved snapshot");
        println!("    timeline <entity> Show history of an entity");
        println!("    clear             Clear the screen");
        println!("    exit / quit       Exit the shell");
        println!();
        println!("  INPUT MODES:");
        println!("    <query>           Natural language (default) -> RAG response");
        println!("    !<command>        Shell command (e.g., !ls -la)");
        println!("    @<time> <query>   Time-travel query (e.g., @yesterday what did we discuss)");
        println!("    ?<query>          Direct LLM query (no RAG)");
        println!();
        println!("  EXAMPLES:");
        println!("    What did we discuss about Rust?");
        println!("    remember that the API uses JWT tokens");
        println!("    @last-week show file changes");
        println!("    !cargo build");
        println!();
    }

    /// Display help for a specific command.
    fn command_help(command: &str) {
        match command {
            "remember" => {
                println!("remember <text>");
                println!();
                println!("Store information in the knowledge graph for later recall.");
                println!();
                println!("Examples:");
                println!("  remember that the API uses JWT tokens");
                println!("  remember project deadline is March 15");
            }
            "recall" => {
                println!("recall <query>");
                println!();
                println!("Retrieve stored memories matching the query.");
                println!();
                println!("Examples:");
                println!("  recall what I know about authentication");
                println!("  recall project deadlines");
            }
            "snapshot" => {
                println!("snapshot <name>");
                println!();
                println!("Save a snapshot of the current system state.");
                println!("Snapshots can be used for time-travel debugging.");
                println!();
                println!("Examples:");
                println!("  snapshot before-refactor");
                println!("  snapshot working-state");
            }
            "timeline" => {
                println!("timeline <entity>");
                println!();
                println!("Show the history of changes to an entity.");
                println!();
                println!("Examples:");
                println!("  timeline authentication-module");
                println!("  timeline config.toml");
            }
            _ => {
                println!("No help available for '{command}'");
            }
        }
    }

    /// Show conversation history.
    ///
    /// Fetches and displays recent messages from the current session.
    #[allow(clippy::unused_self)]
    pub fn history(&self, gallifrey: &Arc<Gallifrey>, session_id: SessionId) {
        match gallifrey.conversation().get_messages(session_id) {
            Ok(messages) => {
                if messages.is_empty() {
                    println!("No messages in current session.");
                } else {
                    println!("Session history ({} messages):", messages.len());
                    println!();
                    for msg in messages.iter().take(20) {
                        let role = match msg.role {
                            tardis_gallifrey::stores::Role::User => "You",
                            tardis_gallifrey::stores::Role::Assistant => "Tardis",
                            tardis_gallifrey::stores::Role::System => "System",
                        };
                        let content = if msg.content.len() > 80 {
                            format!("{}...", &msg.content[..77])
                        } else {
                            msg.content.clone()
                        };
                        println!(
                            "  [{}] {}: {}",
                            msg.timestamp.format("%H:%M"),
                            role,
                            content
                        );
                    }
                    if messages.len() > 20 {
                        println!("  ... and {} more", messages.len() - 20);
                    }
                }
            }
            Err(e) => {
                println!("Failed to get history: {e}");
            }
        }
    }

    /// List available models.
    #[allow(clippy::unused_self)]
    pub fn list_models(&self) {
        println!("Available models:");
        println!();
        println!("  (No models loaded yet)");
        println!();
        println!("Use 'models load <path>' to load a model.");
    }

    /// Show current session context.
    #[allow(clippy::unused_self)]
    pub fn show_context(&self, session_id: SessionId) {
        println!("Current Context:");
        println!();
        println!("  Session ID: {session_id}");
        println!("  Model: (none loaded)");
        println!("  Project: (none set)");
        println!();
        println!("Use 'context project:<name>' to set project context.");
    }

    /// Remember information for later recall.
    #[allow(clippy::unused_self)]
    pub async fn remember(&self, chronos: &Arc<Chronos>, args: &[String]) {
        let content = args.join(" ");
        match chronos
            .remember(&content, tardis_chronos::MemoryCategory::Knowledge)
            .await
        {
            Ok(id) => println!("Remembered: {id}"),
            Err(e) => println!("Failed to remember: {e}"),
        }
    }

    /// Recall stored memories.
    #[allow(clippy::unused_self)]
    pub async fn recall(&self, chronos: &Arc<Chronos>, args: &[String]) {
        let query = args.join(" ");
        match chronos.recall(&query, 5).await {
            Ok(results) => {
                for result in results {
                    println!("- {}", result.content);
                }
            }
            Err(e) => println!("Failed to recall: {e}"),
        }
    }
}

impl Default for CommandHandler {
    fn default() -> Self {
        Self::new()
    }
}
