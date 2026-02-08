# Tardis Shell 🐚

> "Hello, Sweetie."

The **Tardis Shell** is the AI-native command-line interface for Tardis OS. It serves as the primary bridge between human intent and the system's temporal intelligence.

Unlike traditional shells that expect strict syntax, Tardis Shell understands natural language, context, and time.

## 🚀 Getting Started

To launch the shell:

```bash
cargo run --bin tardis
```

You will be greeted by the REPL (Read-Eval-Print Loop):

```text
tardis>
```

## ✨ Features

- **Natural Language Understanding**: Ask questions like "What is the system status?" or "Why did the build fail?".
- **Temporal Awareness**: Use `@time` syntax to query the past or future.
- **Context Retention**: The shell remembers your conversation history and current task.
- **RAG Integration**: Seamlessly retrieves knowledge from **Gallifrey** (the memory store) to answer queries.
- **Built-in Commands**: Traditional commands for system management.

## 🎮 Usage

### 1. Natural Language Queries
Just type your question. The system will use **Chronos** to retrieve relevant context and generate an answer.

```text
tardis> How do I add a new user?
[Chronos] To add a new user, you can use the `useradd` command...
```

### 2. Time Travel (`@time`)
Prefix your query with `@` to shift the temporal context.

```text
tardis> @yesterday Who logged in?
[Time Travel] Querying history for 2023-10-26...
```

### 3. Direct LLM (`?`)
Bypass RAG and talk directly to the model.

```text
tardis> ?Write a poem about Rust.
[Vortex] Rust, the iron oxide red...
```

### 4. System Commands (`!`)
Execute standard shell commands (not fully implemented yet).

```text
tardis> !ls -la
```

## 🛠️ Built-in Commands

The shell includes several built-in commands for managing the AI system.

| Command | Description | Example |
|---------|-------------|---------|
| `help` | Show available commands. | `help` |
| `clear` | Clear the screen. | `clear` |
| `history` | Show conversation history. | `history` |
| `remember` | Store a fact in long-term memory. | `remember "I prefer dark mode"` |
| `recall` | Search long-term memory. | `recall "preferences"` |
| `models` | List available LLM models. | `models` |
| `context` | Show current session context. | `context` |
| `snapshot` | Save system state snapshot. | `snapshot "pre-deploy"` |
| `restore` | Restore a saved snapshot. | `restore "pre-deploy"` |
| `timeline` | Show history of an entity. | `timeline "config.toml"` |
| `forget` | Remove a memory. | `forget "old-password"` |
| `export` | Export session data. | `export "session.json"` |
| `exit` / `quit` | Exit the shell. | `exit` |

## 🏗️ Architecture

The shell is built on three main components:

1.  **Repl (`repl.rs`)**: Handles the input loop and history.
2.  **Router (`router.rs`)**: Classifies input intent (Command vs. Query vs. Time Travel).
3.  **Handlers**: Dispatches actions to **Chronos** (RAG), **Gallifrey** (Memory), or internal logic.
