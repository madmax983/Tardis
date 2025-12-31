//! Intent routing for the Tardis shell.

/// Classified intent of user input.
#[derive(Debug, Clone)]
pub enum Intent {
    /// Built-in shell command.
    BuiltinCommand {
        /// Command name.
        command: String,
        /// Command arguments.
        args: Vec<String>,
    },

    /// External shell command (prefixed with !).
    ShellCommand {
        /// Full command string.
        command: String,
    },

    /// RAG query to Chronos.
    ChronosQuery {
        /// Query text.
        query: String,
        /// Temporal context (if detected).
        temporal_context: Option<String>,
    },

    /// Time-travel query (prefixed with @).
    TimeTravel {
        /// Timestamp or temporal reference.
        timestamp: String,
        /// Query to execute at that time.
        query: String,
    },

    /// Direct LLM query without RAG (prefixed with ?).
    DirectQuery {
        /// Query text.
        query: String,
    },
}

/// Router for classifying user input intent.
pub struct Router {
    /// Built-in command names.
    builtins: Vec<&'static str>,
}

impl Router {
    /// Create a new router.
    #[must_use]
    pub fn new() -> Self {
        Self {
            builtins: vec![
                "help", "exit", "quit", "history", "remember", "recall",
                "models", "context", "clear", "snapshot", "restore",
                "timeline", "forget", "export",
            ],
        }
    }

    /// Route user input to an intent.
    #[must_use]
    pub fn route(&self, input: &str) -> Intent {
        let input = input.trim();

        // Check for prefix commands
        if let Some(cmd) = input.strip_prefix('!') {
            return Intent::ShellCommand {
                command: cmd.to_string(),
            };
        }

        if let Some(rest) = input.strip_prefix('@') {
            return self.parse_time_travel(rest);
        }

        if let Some(query) = input.strip_prefix('?') {
            return Intent::DirectQuery {
                query: query.trim().to_string(),
            };
        }

        // Check for built-in commands
        let parts: Vec<&str> = input.splitn(2, ' ').collect();
        let first_word = parts.first().map(|s| s.to_lowercase());

        if let Some(ref cmd) = first_word {
            if self.builtins.contains(&cmd.as_str()) {
                let args = if parts.len() > 1 {
                    parts[1].split_whitespace().map(String::from).collect()
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
            temporal_context: self.detect_temporal_context(input),
        }
    }

    /// Parse a time-travel command.
    fn parse_time_travel(&self, input: &str) -> Intent {
        // Expected format: @<timestamp> <query>
        // e.g., "@yesterday what did we discuss"
        // e.g., "@2024-03-15 show system state"

        let parts: Vec<&str> = input.splitn(2, ' ').collect();

        let timestamp = parts.first().unwrap_or(&"").to_string();
        let query = parts.get(1).unwrap_or(&"").to_string();

        Intent::TimeTravel { timestamp, query }
    }

    /// Detect temporal context in a query.
    fn detect_temporal_context(&self, query: &str) -> Option<String> {
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

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_command() {
        let router = Router::new();
        let intent = router.route("!ls -la");

        match intent {
            Intent::ShellCommand { command } => assert_eq!(command, "ls -la"),
            _ => panic!("Expected ShellCommand"),
        }
    }

    #[test]
    fn test_builtin_command() {
        let router = Router::new();
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
        let router = Router::new();
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
        let router = Router::new();
        let intent = router.route("@yesterday what did we discuss");

        match intent {
            Intent::TimeTravel { timestamp, query } => {
                assert_eq!(timestamp, "yesterday");
                assert_eq!(query, "what did we discuss");
            }
            _ => panic!("Expected TimeTravel"),
        }
    }
}
