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

        // Check for prefix commands
        if let Some(cmd) = input.strip_prefix('!') {
            return Intent::ShellCommand {
                command: cmd.trim().to_string(),
            };
        }

        if let Some(rest) = input.strip_prefix('@') {
            return Self::parse_time_travel(rest.trim());
        }

        if let Some(query) = input.strip_prefix('?') {
            return Intent::DirectQuery {
                query: query.trim().to_string(),
            };
        }

        // Check for built-in commands
        // Use tokenize to handle quoted arguments
        let tokens_with_offsets = Self::tokenize(input);
        let tokens: Vec<String> = tokens_with_offsets.iter().map(|(t, _)| t.clone()).collect();
        let first_word = tokens.first().map(|s| s.to_lowercase());

        if let Some(ref cmd) = first_word {
            if self.builtins.contains(cmd.as_str()) {
                let args = if tokens.len() > 1 {
                    tokens[1..].to_vec()
                } else {
                    Vec::new()
                };

                return Intent::BuiltinCommand {
                    command: cmd.clone(),
                    args,
                };
            }
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

        let tokens = Self::tokenize(input);

        if let Some((timestamp, end_offset)) = tokens.first() {
            let query = input[*end_offset..].trim().to_string();
            Intent::TimeTravel {
                timestamp: timestamp.clone(),
                query,
            }
        } else {
            Intent::TimeTravel {
                timestamp: String::new(),
                query: String::new(),
            }
        }
    }

    /// Tokenize input string respecting quotes.
    /// Returns a vector of (token, end_offset) tuples.
    fn tokenize(input: &str) -> Vec<(String, usize)> {
        let mut tokens = Vec::new();
        let mut current_token = String::new();
        let mut in_quote = None; // None, Some('"'), Some('\'')
        let mut escape = false;
        let mut last_char_end = 0;

        for (i, c) in input.char_indices() {
            let char_len = c.len_utf8();
            let current_char_end = i + char_len;

            if escape {
                current_token.push(c);
                escape = false;
            } else if c == '\\' {
                escape = true;
            } else if let Some(quote) = in_quote {
                if c == quote {
                    in_quote = None;
                } else {
                    current_token.push(c);
                }
            } else if c == '"' || c == '\'' {
                in_quote = Some(c);
            } else if c.is_whitespace() {
                if !current_token.is_empty() {
                    tokens.push((current_token.clone(), i));
                    current_token.clear();
                }
            } else {
                current_token.push(c);
            }
            last_char_end = current_char_end;
        }

        if !current_token.is_empty() {
            tokens.push((current_token, last_char_end));
        }

        tokens
    }

    /// Detect temporal context in a query.
    fn detect_temporal_context(query: &str) -> Option<String> {
        let lower = query.to_lowercase();

        let temporal_patterns = [
            "yesterday",
            "last week",
            "last month",
            "today",
            "this morning",
            "earlier",
            "before",
            "after",
        ];

        for pattern in temporal_patterns {
            if lower.contains(pattern) {
                return Some(pattern.to_string());
            }
        }

        None
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

    #[test]
    fn test_time_travel_quoted_timestamp() {
        let router = test_router();
        // The current implementation splits on space, so "last week" becomes timestamp="last" query="week query"
        // This test documents the DESIRED behavior (which currently fails)
        let intent = router.route("@ \"last week\" what happened");

        match intent {
            Intent::TimeTravel { timestamp, query } => {
                assert_eq!(timestamp, "last week");
                assert_eq!(query, "what happened");
            }
            _ => panic!("Expected TimeTravel"),
        }
    }

    #[test]
    fn test_builtin_quoted_args() {
        let router = Router::new(vec!["navigate".to_string()]);
        // The current implementation splits on whitespace, so "The Doctor" becomes two args
        let intent = router.route("navigate \"The Doctor\" Dalek");

        match intent {
            Intent::BuiltinCommand { command, args } => {
                assert_eq!(command, "navigate");
                assert_eq!(args.len(), 2);
                assert_eq!(args[0], "The Doctor");
                assert_eq!(args[1], "Dalek");
            }
            _ => panic!("Expected BuiltinCommand"),
        }
    }

    #[test]
    fn test_time_travel_preserves_query_formatting() {
        let router = test_router();
        // Regression test: ensure query part preserves whitespace and quotes
        let intent = router.route("@ \"last week\"   println!(\"hello\")  ");

        match intent {
            Intent::TimeTravel { timestamp, query } => {
                assert_eq!(timestamp, "last week");
                // The query should be trimmed of leading/trailing whitespace, but preserve internal spacing and quotes
                assert_eq!(query, "println!(\"hello\")");
            }
            _ => panic!("Expected TimeTravel"),
        }
    }
}
