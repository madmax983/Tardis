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
#![allow(clippy::unused_self)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::needless_continue)]
#![allow(clippy::unused_async)]
#![allow(missing_debug_implementations)]

pub mod commands;
pub mod repl;
pub mod router;
