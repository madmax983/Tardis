# Architecture Decision Records

This directory contains Architecture Decision Records (ADRs) for Tardis OS.

## What is an ADR?

An ADR is a document that captures an important architectural decision made along with its context and consequences. ADRs are immutable once accepted - if a decision is changed, a new ADR is created that supersedes the old one.

## ADR Index

| ID | Title | Status | Date |
|----|-------|--------|------|
| [001](001-rust-from-scratch-kernel.md) | Rust From-Scratch Kernel | Accepted | 2024-12-31 |
| [002](002-uefi-boot.md) | UEFI Boot Process | Accepted | 2024-12-31 |
| [003](003-cuda-primary-gpu.md) | CUDA as Primary GPU Backend | Accepted | 2024-12-31 |
| [004](004-gallifreydb-temporal-store.md) | GallifreyDB for Temporal Storage | Accepted | 2024-12-31 |
| [005](005-candle-llm-runtime.md) | Candle for LLM Runtime | Accepted | 2024-12-31 |
| [006](006-monolithic-initial-architecture.md) | Monolithic Initial Architecture | Accepted | 2024-12-31 |
| [007](007-clippy-pedantic.md) | Clippy Pedantic Linting | Accepted | 2024-12-31 |

## Creating a New ADR

1. Copy `template.md` to `NNN-title-with-dashes.md`
2. Fill in all sections
3. Submit for review
4. Update this README with the new ADR

## ADR Lifecycle

- **Proposed**: Under discussion
- **Accepted**: Decision made and in effect
- **Deprecated**: No longer applies (superseded or obsolete)
- **Superseded**: Replaced by a newer ADR (link to successor)

## References

- [ADR GitHub Organization](https://adr.github.io/)
- [Michael Nygard's ADR article](https://cognitect.com/blog/2011/11/15/documenting-architecture-decisions)
