# Tardis OS - Project Context

> A timey-wimey OS built on Rust with local LLM (Candle) and temporal knowledge management (GallifreyDB) at its core.

## Vision

Tardis is a **Personal AI Computer** - a full operating system where AI reasoning and persistent temporal memory are first-class citizens, not afterthoughts. Users interact through natural language, and the system remembers everything with full time-travel capabilities.

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                        USER SPACE                           │
│   AI Shell  │  User Apps  │  Temporal File Manager          │
├─────────────────────────────────────────────────────────────┤
│                     CORE SERVICES                           │
│  CHRONOS (RAG)  │  VORTEX (LLM)  │  GALLIFREY (Temporal DB) │
├─────────────────────────────────────────────────────────────┤
│                    KERNEL LAYER                             │
│  Memory Manager  │  Scheduler  │  IPC  │  HAL (UEFI/CUDA)   │
└─────────────────────────────────────────────────────────────┘
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

## Coding Standards

### Rust Edition
- **Edition**: 2024
- **MSRV**: Latest stable

### Linting
We use `clippy::pedantic` with minimal exceptions. See `Cargo.toml` for workspace-wide lint configuration.

```bash
# Check lints
cargo clippy --all-targets --all-features

# Check formatting
cargo fmt --check
```

### Code Style
- **Formatting**: `rustfmt` with project config (see `rustfmt.toml`)
- **Naming**: Follow Rust API guidelines
- **Documentation**: All public items must have doc comments
- **Error Handling**: Use `thiserror` for library errors, `anyhow` in binaries
- **Unsafe**: Minimize unsafe code; document safety invariants when required

### Commit Messages
```
<type>(<scope>): <subject>

<body>

<footer>
```

Types: `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `chore`
Scopes: `kernel`, `vortex`, `gallifrey`, `chronos`, `shell`, `common`

## Architecture Decision Records

All significant architectural decisions are documented in `docs/adr/`. Use the template at `docs/adr/template.md`.

Key decisions:
- [ADR-001](docs/adr/001-rust-from-scratch-kernel.md): Build kernel from scratch
- [ADR-002](docs/adr/002-uefi-boot.md): Use UEFI boot process
- [ADR-003](docs/adr/003-cuda-primary-gpu.md): CUDA as primary GPU backend
- [ADR-004](docs/adr/004-gallifreydb-temporal-store.md): GallifreyDB for temporal storage
- [ADR-005](docs/adr/005-candle-llm-runtime.md): Candle for LLM inference

## Development Workflow

### Git Worktree Workflow (Parallel Development)

We use git worktrees to enable multiple Claude agents to work on different features simultaneously. Each feature gets its own worktree in `../tardis-worktrees/`.

```powershell
# Create a new feature worktree
.\scripts\worktree-new.ps1 feature/my-feature

# List active worktrees
.\scripts\worktree-list.ps1

# Clean up after PR merge
.\scripts\worktree-cleanup.ps1 feature/my-feature
```

**Workflow:**
1. Create worktree: `.\scripts\worktree-new.ps1 feature/add-metrics`
2. Work in worktree: `cd ..\tardis-worktrees\feature-add-metrics`
3. Commit and push: `git push -u origin feature/add-metrics`
4. Create PR: `gh pr create --base trunk`
5. After merge, cleanup: `.\scripts\worktree-cleanup.ps1 feature/add-metrics`

> **Note:** Bash scripts (`.sh`) are also available for Unix/WSL environments.

**Directory Structure:**
```
~/
├── tardis/                    # Main repo (trunk)
└── tardis-worktrees/          # Feature worktrees
    ├── feature-add-metrics/
    ├── fix-memory-leak/
    └── refactor-vortex/
```

### Building
```bash
# Build all crates
cargo build --workspace

# Build kernel (requires nightly for no_std)
cargo +nightly build -p tardis-kernel

# Run tests
cargo test --workspace
```

### Testing in QEMU
```bash
# Build bootable image
cargo build -p tardis-kernel --release

# Run in QEMU
qemu-system-x86_64 -bios /path/to/OVMF.fd -drive format=raw,file=target/x86_64-tardis/release/bootimage-tardis-kernel.bin
```

## File Organization

```
tardis/
├── CLAUDE.md              # This file
├── Cargo.toml             # Workspace root
├── rustfmt.toml           # Formatting config
├── scripts/               # Development scripts (PowerShell + Bash)
│   ├── worktree-new.ps1   # Create feature worktree
│   ├── worktree-list.ps1  # List active worktrees
│   └── worktree-cleanup.ps1 # Clean up merged worktree
├── docs/
│   ├── architecture/      # Design documents
│   └── adr/               # Architecture Decision Records
├── kernel/                # OS kernel (no_std)
├── vortex/                # LLM inference engine
├── gallifrey/             # Temporal database layer
├── chronos/               # RAG orchestration
├── shell/                 # AI shell interface
├── telemetry/             # Full-stack observability
└── common/                # Shared types
```

## Key Concepts

### Bi-Temporal Data
GallifreyDB tracks two time dimensions:
- **Valid Time**: When the fact was true in the real world
- **Transaction Time**: When the fact was recorded in the system

This enables queries like "What did we know about X as of last Tuesday?"

### Three Temporal Stores
1. **Knowledge Graph**: Facts, entities, relationships with embeddings
2. **Conversation Store**: Chat history with cross-session continuity
3. **System State Journal**: OS state for time-travel debugging

### Syscall Interface
Core services expose syscalls for userspace:
- `sys_vortex_*`: LLM operations (load, infer, embed)
- `sys_gallifrey_*`: Temporal queries and mutations
- `sys_chronos_*`: RAG pipeline operations

## Dependencies Policy

- Prefer well-maintained crates from the Rust ecosystem
- Minimize dependency count in kernel (no_std constraints)
- Pin versions in Cargo.lock for reproducibility
- Security audit dependencies with `cargo audit`

## Performance Targets

- Sub-microsecond kernel syscalls
- LLM first-token latency < 100ms (warm cache)
- Temporal query response < 10ms (hot path)
- Memory-mapped model loading for instant availability
