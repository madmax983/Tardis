//! # Tardis Shell
//!
//! The interactive AI shell for Tardis OS.
//!
//! The Shell serves as the primary user interface, accepting natural language input
//! and routing it to the appropriate subsystem (Chronos for RAG, Gallifrey for queries,
//! or internal commands).
//!
//! ## Architecture
//!
//! The shell is designed as a modular pipeline:
//!
//! 1.  **REPL Loop**: The [`Repl`](crate::repl::Repl) manages the user session, history, and terminal I/O using `rustyline`.
//! 2.  **Intent Classification**: The [`Router`](crate::router::Router) analyzes input to determine if it is:
//!     -   A **Built-in Command** (e.g., `help`, `history`) -> Handled by [`CommandHandler`](crate::commands::CommandHandler).
//!     -   A **Chronos Query** (e.g., "What is the status?") -> Sent to the RAG engine.
//!     -   A **Time Travel Request** (e.g., `@yesterday ...`) -> Modifies temporal context.
//!     -   A **Direct LLM Query** (e.g., `?Write a poem`) -> Bypasses RAG.
//! 3.  **Execution**: The appropriate subsystem executes the request and returns a response, which is then formatted for the user.
//!
//! ## Example Session
//!
//! ```text
//! $ cargo run --bin tardis
//!
//! tardis> What is the system status?
//! [Chronos] The system is running normally. CPU usage is at 15%.
//!
//! tardis> @yesterday Who logged in?
//! [Time Travel] Querying history for 2023-10-26...
//! ```
//!
//! ## Modules
//!
//! - [`repl`]: The main read-eval-print loop.
//! - [`router`]: Input classification and routing logic.
//! - [`commands`]: Built-in command handlers.
//! - `dashboard`: (Feature `nova`) TUI dashboard.

#![warn(missing_docs)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

pub mod commands;
#[cfg(feature = "nova")]
/// The TUI dashboard.
pub mod dashboard;
#[cfg(feature = "nova")]
/// Experimental features (Nova).
pub mod experimental;
pub mod repl;
pub mod router;
