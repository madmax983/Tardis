use crate::commands::traits::ShellCommand;
use std::collections::HashMap;
use std::fmt;

/// Registry for shell commands.
pub struct CommandRegistry {
    commands: HashMap<String, Box<dyn ShellCommand>>,
}

impl CommandRegistry {
    /// Create a new, empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            commands: HashMap::new(),
        }
    }

    /// Register a command.
    pub fn register(&mut self, cmd: Box<dyn ShellCommand>) {
        self.commands.insert(cmd.name().to_string(), cmd);
    }

    /// Get a command by name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&dyn ShellCommand> {
        self.commands.get(name).map(std::convert::AsRef::as_ref)
    }

    /// List all registered command names.
    #[must_use]
    pub fn list_commands(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self
            .commands
            .keys()
            .map(std::string::String::as_str)
            .collect();
        names.sort_unstable();
        names
    }
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for CommandRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CommandRegistry")
            .field("count", &self.commands.len())
            .field("commands", &self.list_commands())
            .finish()
    }
}
