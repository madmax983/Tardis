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

/// Command traits and context.
pub mod traits;
/// Command registry.
pub mod registry;
/// System maintenance commands.
pub mod system;
#[cfg(feature = "nova")]
/// Experimental commands (Nova feature).
pub mod experimental;
/// Knowledge management commands.
pub mod knowledge;

use std::sync::Arc;
use tardis_common::SessionId;
use tardis_gallifrey::Gallifrey;

/// Display help information.
///
/// If an argument is provided, displays help for that specific command.
/// Otherwise, displays the general help menu.
///
/// # Example
///
/// ```rust,no_run
/// use tardis_shell::commands;
///
/// commands::help(&[]); // General help
/// commands::help(&["remember".to_string()]); // Help for 'remember'
/// ```
pub fn help(args: &[String]) {
    if args.is_empty() {
        general_help();
    } else {
        command_help(&args[0]);
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
    println!("    > What did we discuss about Rust?");
    println!("    > remember that the API uses JWT tokens");
    println!("    > @last-week show file changes");
    println!("    > !cargo build");
    println!("    > ?Write a poem about time travel");
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
        "models" => {
            println!("models");
            println!();
            println!("List available LLM models and their status.");
            println!("Use 'models load <path>' to load a specific model.");
        }
        "context" => {
            println!("context [subcommand]");
            println!();
            println!("Inspect or modify the current session context.");
            println!();
            println!("Subcommands:");
            println!("  (none)      Show summary");
            println!("  project     Set project context");
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
pub fn history(gallifrey: &Arc<Gallifrey>, session_id: SessionId) {
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
pub fn list_models() {
    println!("Available models:");
    println!();
    println!("  (No models loaded yet)");
    println!();
    println!("Use 'models load <path>' to load a model.");
}

/// Show current session context.
pub fn show_context(session_id: SessionId) {
    println!("Current Context:");
    println!();
    println!("  Session ID: {session_id}");
    println!("  Model: (none loaded)");
    println!("  Project: (none set)");
    println!();
    println!("Use 'context project:<name>' to set project context.");
}
