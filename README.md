# Tardis OS 🎻

[![Rust](https://img.shields.io/badge/rust-1.85%2B-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)

> "People assume that time is a strict progression of cause to effect, but actually from a non-linear, non-subjective viewpoint - it's more like a big ball of wibbly wobbly... time-y wimey... stuff."

**Tardis OS** is a Personal AI Computer - a full operating system where AI reasoning and persistent temporal memory are first-class citizens, not afterthoughts. Users interact through natural language, and the system remembers everything with full time-travel capabilities.

## 🏗 Architecture

The system is built on a bi-temporal foundation, allowing you to query not just *what* is true, but *when* it was true and *when* the system learned about it.

```mermaid
graph TD
    User[User Space] --> Shell[AI Shell]
    Shell --> Chronos[Chronos (RAG)]
    Chronos --> Vortex[Vortex (LLM)]
    Chronos --> Gallifrey[Gallifrey (Temporal DB)]

    subgraph Core Services
        Chronos
        Vortex
        Gallifrey
    end

    subgraph Kernel Layer
        HAL[HAL (UEFI/CUDA)]
        Memory[Memory Manager]
        Scheduler
    end
```

### Core Subsystems

| Crate | Purpose | Key Tech |
|-------|---------|----------|
| `kernel/` | OS kernel - memory, processes, syscalls | no_std, UEFI, x86_64 |
| `vortex/` | LLM inference engine | Candle, CUDA, tokenizers |
| `gallifrey/` | Temporal knowledge store | GallifreyDB |
| `chronos/` | RAG orchestration | Multi-source retrieval |
| `shell/` | AI-native user interface | REPL, NLP commands |
| `common/` | Shared types and utilities | serde, error types |

## 🚀 Getting Started

### Prerequisites

- Rust 1.85+ (Nightly required for kernel)
- QEMU (for running the OS image)
- CUDA Toolkit (optional, for GPU acceleration)

### Building

```bash
# Build all crates
cargo build --workspace

# Build kernel (requires nightly for no_std)
cargo +nightly build -p tardis-kernel
```

### Running in QEMU

```bash
# Build bootable image
cargo build -p tardis-kernel --release

# Run in QEMU
qemu-system-x86_64 -bios /path/to/OVMF.fd -drive format=raw,file=target/x86_64-tardis/release/bootimage-tardis-kernel.bin
```

## 🧠 Key Concepts

### Bi-Temporal Data
GallifreyDB tracks two time dimensions:
- **Valid Time**: When the fact was true in the real world
- **Transaction Time**: When the fact was recorded in the system

This enables queries like "What did we know about X as of last Tuesday?"

### Three Temporal Stores
1. **Knowledge Graph**: Facts, entities, relationships with embeddings
2. **Conversation Store**: Chat history with cross-session continuity
3. **System State Journal**: OS state for time-travel debugging

## 🤝 Contributing

We welcome contributions! Please check `CLAUDE.md` for development workflows and architectural decision records.

1. Create a feature branch (or worktree)
2. Make your changes
3. Run `cargo test` and `cargo clippy`
4. Submit a PR

## 📜 License

This project is licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
