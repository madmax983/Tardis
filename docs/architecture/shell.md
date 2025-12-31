# AI Shell - User Interface

## Overview

The Tardis AI Shell is the primary user interface - an AI-native REPL that combines natural language interaction with temporal awareness and traditional shell capabilities.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                       AI SHELL                              │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌───────────────────────────────────────────────────────┐  │
│  │                    Input Layer                         │  │
│  │  ┌─────────────┐  ┌─────────────┐  ┌───────────────┐  │  │
│  │  │    REPL     │  │   History   │  │   Tab-Complete│  │  │
│  │  │   Prompt    │  │   Search    │  │   (AI-powered)│  │  │
│  │  └─────────────┘  └─────────────┘  └───────────────┘  │  │
│  └────────────────────────┬──────────────────────────────┘  │
│                           │                                  │
│  ┌────────────────────────▼──────────────────────────────┐  │
│  │                  Intent Router                         │  │
│  │                                                        │  │
│  │   "find files about rust"  → Semantic File Search      │  │
│  │   "ls -la"                 → Traditional Command       │  │
│  │   "what did we discuss..." → Chronos Query             │  │
│  │   "remember that..."       → Memory Storage            │  │
│  │   "show state at 3pm"      → Time Travel               │  │
│  │                                                        │  │
│  └────────────────────────┬──────────────────────────────┘  │
│                           │                                  │
│          ┌────────────────┼────────────────┐                │
│          ▼                ▼                ▼                │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐         │
│  │  Built-in   │  │   Chronos   │  │   System    │         │
│  │  Commands   │  │   Bridge    │  │   Bridge    │         │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘         │
│         │                │                │                 │
│         └────────────────┼────────────────┘                 │
│                          ▼                                  │
│  ┌───────────────────────────────────────────────────────┐  │
│  │                   Output Layer                         │  │
│  │  ┌─────────────┐  ┌─────────────┐  ┌───────────────┐  │  │
│  │  │  Streaming  │  │  Markdown   │  │   Syntax      │  │  │
│  │  │   Display   │  │  Rendering  │  │   Highlight   │  │  │
│  │  └─────────────┘  └─────────────┘  └───────────────┘  │  │
│  └───────────────────────────────────────────────────────┘  │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

## Input Modes

### Natural Language Mode (Default)

```
tardis> find all rust files I edited last week
```

The shell interprets natural language queries and routes them appropriately.

### Command Mode (Prefix: !)

```
tardis> !ls -la
tardis> !cargo build
```

Prefix with `!` for traditional shell command execution.

### Time-Travel Mode (Prefix: @)

```
tardis> @3pm show file changes
tardis> @2024-03-15 what did I know about async
```

Prefix with `@` followed by temporal reference for time-aware queries.

### Direct Chronos Mode (Prefix: ?)

```
tardis> ?what patterns do you see in my coding style
```

Prefix with `?` for direct LLM conversation without retrieval.

## Intent Router

```rust
pub enum Intent {
    /// Natural language query routed to Chronos
    ChronosQuery {
        query: String,
        temporal_context: Option<TemporalReference>,
    },

    /// Traditional shell command
    ShellCommand {
        command: String,
        args: Vec<String>,
    },

    /// Built-in shell command
    BuiltinCommand {
        command: BuiltinCmd,
        args: Vec<String>,
    },

    /// Time-travel query
    TimeTravel {
        timestamp: DateTime<Utc>,
        query: String,
    },

    /// Memory operation
    Memory {
        operation: MemoryOp,
        data: String,
    },

    /// Semantic file search
    SemanticSearch {
        query: String,
        scope: SearchScope,
    },
}

impl IntentRouter {
    pub fn route(&self, input: &str) -> Intent {
        // Check for explicit mode prefixes
        if input.starts_with('!') {
            return self.parse_shell_command(&input[1..]);
        }
        if input.starts_with('@') {
            return self.parse_time_travel(&input[1..]);
        }
        if input.starts_with('?') {
            return self.parse_direct_query(&input[1..]);
        }

        // Check for built-in commands
        if let Some(builtin) = self.match_builtin(input) {
            return builtin;
        }

        // Default: natural language to Chronos
        Intent::ChronosQuery {
            query: input.to_string(),
            temporal_context: self.extract_temporal_context(input),
        }
    }
}
```

## Built-in Commands

| Command | Description | Example |
|---------|-------------|---------|
| `help` | Show help | `help`, `help remember` |
| `history` | Show conversation history | `history`, `history search async` |
| `remember` | Store information | `remember that the API uses JWT tokens` |
| `recall` | Retrieve memories | `recall what I know about authentication` |
| `forget` | Remove memory | `forget the old API endpoint` |
| `timeline` | Show entity history | `timeline authentication-module` |
| `snapshot` | Save system state | `snapshot before-refactor` |
| `restore` | Restore system state | `restore before-refactor` |
| `models` | List loaded LLM models | `models`, `models load phi-3` |
| `context` | Show/set conversation context | `context clear`, `context project:web-app` |
| `export` | Export conversation | `export markdown today.md` |

## AI-Powered Completion

```rust
pub struct AICompleter {
    chronos: ChronosBridge,
    history: CommandHistory,
    context: SessionContext,
}

impl AICompleter {
    pub async fn complete(&self, partial: &str) -> Vec<Completion> {
        let mut completions = Vec::new();

        // 1. Check command history
        completions.extend(self.history.prefix_matches(partial));

        // 2. Check file system
        if partial.contains('/') || partial.contains('\\') {
            completions.extend(self.complete_path(partial));
        }

        // 3. AI-powered semantic completion
        if partial.len() > 3 {
            let semantic = self.chronos.complete(partial, &self.context).await?;
            completions.extend(semantic);
        }

        // 4. Built-in command completion
        completions.extend(self.complete_builtins(partial));

        completions
    }
}
```

## Streaming Output

```rust
pub struct OutputRenderer {
    terminal: Terminal,
    syntax_highlighter: SyntaxHighlighter,
    markdown_renderer: MarkdownRenderer,
}

impl OutputRenderer {
    /// Render streaming LLM response with live updates
    pub async fn render_stream(&mut self, stream: TokenStream) {
        let mut buffer = String::new();

        while let Some(token) = stream.next().await {
            buffer.push_str(&token);

            // Detect code blocks for syntax highlighting
            if let Some(code_block) = self.detect_code_block(&buffer) {
                self.syntax_highlighter.highlight_incremental(code_block);
            } else {
                // Render as plain text
                self.terminal.write(&token);
            }
        }

        // Final render with full markdown processing
        let rendered = self.markdown_renderer.render(&buffer);
        self.terminal.replace_content(&rendered);
    }
}
```

## Session Management

```rust
pub struct Session {
    pub id: SessionId,
    pub started_at: DateTime<Utc>,
    pub context: SessionContext,
    pub conversation: Vec<Message>,
    pub working_directory: PathBuf,
    pub environment: HashMap<String, String>,
}

pub struct SessionContext {
    /// Active project/domain
    pub project: Option<String>,
    /// Active model
    pub model: ModelHandle,
    /// Retrieval configuration
    pub retrieval_config: RetrievalConfig,
    /// Personality/tone settings
    pub personality: PersonalityConfig,
}

impl Session {
    /// Save session to Gallifrey for later resumption
    pub async fn save(&self, gallifrey: &GallifreyBridge) -> Result<()> {
        gallifrey.conversation().save_session(self).await
    }

    /// Resume previous session
    pub async fn resume(gallifrey: &GallifreyBridge, id: SessionId) -> Result<Self> {
        gallifrey.conversation().load_session(id).await
    }
}
```

## Example Sessions

### Natural Conversation

```
tardis> I'm working on the authentication module

I'll set the context to focus on authentication-related information.
Context set: project=authentication

What would you like to work on?

tardis> What patterns have we established for token validation?

Based on our previous discussions and the codebase:

1. **JWT Validation**: We use `jsonwebtoken` crate with RS256 algorithm
2. **Token Structure**: {user_id, roles, exp, iat, jti}
3. **Middleware Pattern**: `validate_token` middleware on protected routes
4. **Error Handling**: Custom `AuthError` type with specific variants

Would you like me to show the relevant code, or discuss any of these patterns?
```

### Time-Travel Query

```
tardis> @last-monday what was the API response format for /users

[Retrieving knowledge state from 2024-03-18]

As of last Monday, the /users endpoint returned:

```json
{
  "users": [...],
  "pagination": { "page": 1, "total": 100 }
}
```

Note: This changed on Wednesday (2024-03-20) when we added the
`meta` wrapper. Current format includes additional fields.
```

### System State Debug

```
tardis> snapshot before-refactor

✓ System state snapshot saved: before-refactor
  - 47 files tracked
  - 3 processes recorded
  - Configuration preserved

tardis> [... make changes ...]

tardis> @before-refactor compare with now

Changes since snapshot 'before-refactor' (15 minutes ago):

Files modified (12):
  - src/auth/token.rs: +45 -12 lines
  - src/auth/middleware.rs: +23 -8 lines
  ...

Configuration changes:
  - Added AUTH_TIMEOUT=30s

Would you like to see the diff for any specific file?
```

## Directory Structure

```
shell/
├── Cargo.toml
└── src/
    ├── main.rs              # Entry point
    ├── repl.rs              # REPL loop
    ├── input/               # Input handling
    │   ├── mod.rs
    │   ├── parser.rs        # Input parsing
    │   ├── history.rs       # Command history
    │   └── completion.rs    # AI completion
    ├── router/              # Intent routing
    │   ├── mod.rs
    │   └── intent.rs        # Intent classification
    ├── commands/            # Built-in commands
    │   ├── mod.rs
    │   ├── help.rs
    │   ├── memory.rs
    │   ├── timeline.rs
    │   └── snapshot.rs
    ├── output/              # Output rendering
    │   ├── mod.rs
    │   ├── stream.rs        # Streaming display
    │   ├── markdown.rs      # Markdown rendering
    │   └── syntax.rs        # Syntax highlighting
    ├── bridges/             # Service bridges
    │   ├── mod.rs
    │   ├── chronos.rs       # Chronos integration
    │   └── system.rs        # System command execution
    └── session/             # Session management
        ├── mod.rs
        └── context.rs       # Session context
```
