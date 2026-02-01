//! # Tardis Shell
//!
//! The interactive AI shell for Tardis OS.
//!
//! Provides:
//! - Natural language command interface
//! - Intent routing to appropriate handlers
//! - Built-in commands for memory and time-travel

#![warn(missing_docs)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

pub mod commands;
pub mod repl;
pub mod router;
