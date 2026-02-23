//! Intent routing for the Tardis shell.
//!
//! The router is responsible for classifying user input into actionable intents.
//! It uses a priority-based system to distinguish between:
//!
//! 1.  **Shell Commands** (prefixed with `!`): Executed directly by the OS.
//! 2.  **Time Travel** (prefixed with `@`): Queries directed at a specific point in time.
//! 3.  **Direct Queries** (prefixed with `?`): Questions for the LLM without RAG context.
//! 4.  **Built-in Commands**: Known commands like `help` or `history`.
//! 5.  **Chronos Queries**: Everything else is treated as a natural language query for the RAG engine.

use std::collections::HashSet;

const PREFIX_SHELL: char = '!';
const PREFIX_TIME_TRAVEL: char = '@';
const PREFIX_DIRECT_QUERY: char = '?';
const TEMPORAL_PATTERNS: [&str; 8] = [
    "yesterday",
    "last week",
    "last month",
    "today",
    "this morning",
    "earlier",
    "before",
    "after",
];

/// Classified intent of user input.
#[derive(Debug, Clone)]
pub enum Intent {
    /// Built-in shell command (e.g., `help`, `history`).
    BuiltinCommand {
        /// Command name (e.g., "help").
        command: String,
        /// Command arguments.
        args: Vec<String>,
    },

    /// External shell command (prefixed with `!`).
    ShellCommand {
        /// Full command string (e.g., "ls -la").
        command: String,
    },

    /// RAG query to Chronos (default behavior).
    ChronosQuery {
        /// Query text.
        query: String,
        /// Temporal context (if detected).
        #[allow(dead_code)]
        temporal_context: Option<String>,
    },

    /// Time-travel query (prefixed with `@`).
    TimeTravel {
        /// Timestamp or temporal reference (e.g., "yesterday").
        timestamp: String,
        /// Query to execute at that time.
        query: String,
    },

    /// Direct LLM query without RAG (prefixed with `?`).
    DirectQuery {
        /// Query text.
        query: String,
    },
}

/// Router for classifying user input intent.
///
/// The router maintains a list of built-in commands and uses prefix matching
/// to determine the intent of the user's input.
#[derive(Debug)]
pub struct Router {
    /// Built-in command names.
    builtins: HashSet<String>,
}

impl Router {
    /// Create a new router with the given built-in commands.
    #[must_use]
    pub fn new(builtins: impl IntoIterator<Item = String>) -> Self {
        Self {
            builtins: builtins.into_iter().collect(),
        }
    }

    /// Route user input to an intent.
    ///
    /// # Examples
    ///
    /// ```
    /// use tardis_shell::router::{Router, Intent};
    ///
    /// let router = Router::new(vec!["help".to_string()]);
    ///
    /// // Built-in command
    /// let intent = router.route("help me");
    /// matches!(intent, Intent::BuiltinCommand { command, .. } if command == "help");
    ///
    /// // Shell command
    /// let intent = router.route("!ls -la");
    /// matches!(intent, Intent::ShellCommand { command } if command == "ls -la");
    ///
    /// // Time travel
    /// let intent = router.route("@yesterday what happened?");
    /// matches!(intent, Intent::TimeTravel { timestamp, .. } if timestamp == "yesterday");
    ///
    /// // Chronos query (default)
    /// let intent = router.route("What is the meaning of life?");
    /// matches!(intent, Intent::ChronosQuery { .. });
    /// ```
    #[must_use]
    pub fn route(&self, input: &str) -> Intent {
        let input = input.trim();

        if let Some(intent) = Self::try_shell_command(input) {
            return intent;
        }

        if let Some(intent) = Self::try_time_travel(input) {
            return intent;
        }

        if let Some(intent) = Self::try_direct_query(input) {
            return intent;
        }

        if let Some(intent) = self.try_builtin_command(input) {
            return intent;
        }

        // Default: Chronos query
        Intent::ChronosQuery {
            query: input.to_string(),
            temporal_context: Self::detect_temporal_context(input),
        }
    }

    /// Parse a time-travel command.
    fn parse_time_travel(input: &str) -> Intent {
        // Expected format: @<timestamp> <query>
        // e.g., "@yesterday what did we discuss"
        // e.g., "@2024-03-15 show system state"

        let parts: Vec<&str> = input.splitn(2, ' ').collect();

        let timestamp = parts.first().unwrap_or(&"").to_string();
        let query = parts.get(1).unwrap_or(&"").to_string();

        Intent::TimeTravel { timestamp, query }
    }

    /// Detect temporal context in a query.
    fn detect_temporal_context(query: &str) -> Option<String> {
        let lower = query.to_lowercase();

        for pattern in TEMPORAL_PATTERNS {
            if lower.contains(pattern) {
                return Some(pattern.to_string());
            }
        }

        None
    }

    /// Try to parse as a shell command.
    fn try_shell_command(input: &str) -> Option<Intent> {
        input
            .strip_prefix(PREFIX_SHELL)
            .map(|cmd| Intent::ShellCommand {
                command: cmd.trim().to_string(),
            })
    }

    /// Try to parse as a time-travel command.
    fn try_time_travel(input: &str) -> Option<Intent> {
        input
            .strip_prefix(PREFIX_TIME_TRAVEL)
            .map(|rest| Self::parse_time_travel(rest.trim()))
    }

    /// Try to parse as a direct query.
    fn try_direct_query(input: &str) -> Option<Intent> {
        input
            .strip_prefix(PREFIX_DIRECT_QUERY)
            .map(|query| Intent::DirectQuery {
                query: query.trim().to_string(),
            })
    }

    /// Try to parse as a built-in command.
    fn try_builtin_command(&self, input: &str) -> Option<Intent> {
        let parts: Vec<&str> = input.splitn(2, ' ').collect();
        // first() returns Option<&&str>, map gives Option<String>
        let cmd = parts.first().map(|s| s.to_lowercase())?;

        self.builtins.contains(&cmd).then(|| {
            let args = parts
                .get(1)
                .map(|s| s.split_whitespace().map(String::from).collect())
                .unwrap_or_default();
            Intent::BuiltinCommand { command: cmd, args }
        })
    }
}

#[cfg(test)]
#[allow(clippy::panic)]
mod tests {
    use super::*;

    fn test_router() -> Router {
        Router::new(vec!["help".to_string(), "history".to_string()])
    }

    #[test]
    fn test_shell_command() {
        let router = test_router();
        let intent = router.route("!ls -la");

        match intent {
            Intent::ShellCommand { command } => assert_eq!(command, "ls -la"),
            _ => panic!("Expected ShellCommand"),
        }
    }

    #[test]
    fn test_builtin_command() {
        let router = test_router();
        let intent = router.route("help");

        match intent {
            Intent::BuiltinCommand { command, args } => {
                assert_eq!(command, "help");
                assert!(args.is_empty());
            }
            _ => panic!("Expected BuiltinCommand"),
        }
    }

    #[test]
    fn test_chronos_query() {
        let router = test_router();
        let intent = router.route("what is rust?");

        match intent {
            Intent::ChronosQuery { query, .. } => {
                assert_eq!(query, "what is rust?");
            }
            _ => panic!("Expected ChronosQuery"),
        }
    }

    #[test]
    fn test_time_travel() {
        let router = test_router();
        let intent = router.route("@yesterday what did we discuss");

        match intent {
            Intent::TimeTravel { timestamp, query } => {
                assert_eq!(timestamp, "yesterday");
                assert_eq!(query, "what did we discuss");
            }
            _ => panic!("Expected TimeTravel"),
        }
    }

    #[test]
    fn test_time_travel_with_space() {
        let router = test_router();
        let intent = router.route("@ yesterday query");

        match intent {
            Intent::TimeTravel { timestamp, query } => {
                assert_eq!(timestamp, "yesterday");
                assert_eq!(query, "query");
            }
            _ => panic!("Expected TimeTravel"),
        }
    }

    #[test]
    fn test_shell_command_with_space() {
        let router = test_router();
        let intent = router.route("! ls");

        match intent {
            Intent::ShellCommand { command } => assert_eq!(command, "ls"),
            _ => panic!("Expected ShellCommand"),
        }
    }

    #[test]
    fn test_shell_command_trailing_space() {
        let router = test_router();
        let intent = router.route("!ls ");

        match intent {
            Intent::ShellCommand { command } => assert_eq!(command, "ls"),
            _ => panic!("Expected ShellCommand"),
        }
    }
}
