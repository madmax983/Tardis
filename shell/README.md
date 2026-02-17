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
- **Context Retention**: The shell remembers your conversation history and current task.
- **RAG Integration**: Seamlessly retrieves knowledge from **Gallifrey** (the memory store) to answer queries.
- **Built-in Commands**: Traditional commands for system management.

## ⚠️ Experimental Features

The following features are currently in active development or partially implemented:

- **Temporal Awareness (`@time`)**: Syntax for querying the past is parsed but not fully integrated.
- **Direct LLM (`?`)**: Bypassing RAG to talk to the raw model is a work in progress.
- **System Commands (`!`)**: Execution of host OS commands is currently disabled for safety.

## 🎮 Usage

### 1. Natural Language Queries
Just type your question. The system will use **Chronos** to retrieve relevant context and generate an answer.

```text
tardis> How do I add a new user?
[Chronos] To add a new user, you can use the `useradd` command...
```

### 2. Built-in Commands
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
| `exit` / `quit` | Exit the shell. | `exit` |

## 🌟 Nova Features (Experimental)

When compiled with the `nova` feature, the shell gains advanced TUI visualization and system repair tools.

```bash
cargo run --bin tardis --features nova
```

| Command | Description | Example |
|---------|-------------|---------|
| `dashboard` | Interactive system monitor TUI. | `dashboard` |
| `timeline` | Visualize an entity's history. | `timeline "config.toml"` |
| `map` | View knowledge graph relationships. | `map "User" 2` |
| `heatmap` | Bi-temporal activity visualization. | `heatmap` |
| `sonic` | "Sonic Screwdriver" - Inspect/Fix files. | `sonic fix config.json` |
| `capsule` | Import/Export time capsules. | `capsule capture "ProjectX" backup.json` |

## 🏗️ Architecture

The shell is built on three main components:

1.  **Repl (`repl.rs`)**: Handles the input loop and history.
2.  **Router (`router.rs`)**: Classifies input intent (Command vs. Query vs. Time Travel).
3.  **Handlers**: Dispatches actions to **Chronos** (RAG), **Gallifrey** (Memory), or internal logic.
