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
//! 1. **REPL**: The [`Repl`](crate::repl::Repl) runs the read-eval-print loop.
//! 2. **Router**: The [`Router`](crate::router::Router) classifies input intent (e.g., "Time Travel" vs "Question").
//! 3. **Handlers**:
//!    - **Chronos**: RAG queries are sent to the Chronos engine.
//!    - **Commands**: Built-in commands (`help`, `clear`, etc.) are handled locally.
//!
//! ## Example
//!
//! ```bash
//! $ cargo run --bin tardis
//! tardis> What is the system status?
//! [Chronos] RAG response...
//!
//! tardis> @yesterday Who logged in?
//! [Time Travel] Querying history...
//! ```

#![warn(missing_docs)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

pub mod commands;
#[cfg(feature = "nova")]
/// The TUI dashboard.
pub mod dashboard;
pub mod repl;
pub mod router;
