# Tardis OS Architecture Documentation

This directory contains the detailed architectural design documents for Tardis OS.

## Documents

| Document | Description |
|----------|-------------|
| [overview.md](overview.md) | High-level system architecture |
| [kernel.md](kernel.md) | Kernel subsystem design |
| [vortex.md](vortex.md) | LLM inference engine design |
| [gallifrey.md](gallifrey.md) | Temporal database integration |
| [chronos.md](chronos.md) | RAG orchestration engine |
| [shell.md](shell.md) | AI shell interface |

## Architecture Principles

1. **AI-Native**: LLM and RAG are core OS services, not applications
2. **Temporal First**: All data has bi-temporal semantics by default
3. **Memory Efficient**: Huge pages, mmap, zero-copy IPC for large tensors
4. **Rust Safety**: Leverage Rust's type system to prevent bugs at compile time
5. **Modular Design**: Clean interfaces between subsystems for future microkernel extraction

## Related

- [Architecture Decision Records](../adr/README.md) - Key design decisions and rationale
- [CLAUDE.md](../../CLAUDE.md) - Project overview and coding standards
