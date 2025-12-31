# Tardis OS Documentation

Welcome to Tardis OS - a Rust-based operating system with AI at its core.

## Quick Links

- [Architecture Overview](architecture/overview.md)
- [Architecture Decision Records](adr/README.md)
- [API Documentation](https://madmax983.github.io/tardis/) (rustdoc)

## Crates

| Crate | Description |
|-------|-------------|
| `tardis-common` | Shared types, error handling, and service traits |
| `tardis-vortex` | LLM inference engine using Candle |
| `tardis-gallifrey` | Temporal knowledge store with bi-temporal queries |
| `tardis-chronos` | RAG orchestration bridging Vortex and Gallifrey |
| `tardis-shell` | Interactive AI shell with intent routing |
| `tardis-kernel` | UEFI boot kernel (requires nightly Rust) |

## Getting Started

### Prerequisites

- Rust 1.85+ (stable)
- Rust nightly (for kernel development only)

### Building

```bash
# Build all default crates (excludes kernel)
cargo build

# Run tests
cargo test

# Build documentation
cargo doc --open

# Run benchmarks
cargo bench
```

### Running the Shell

```bash
cargo run --bin tardis
```

## Development

- See [CLAUDE.md](../CLAUDE.md) for coding standards and contribution guidelines
- See [ADR-007](adr/007-clippy-pedantic.md) for linting configuration rationale

## CI/CD

This project uses GitHub Actions for:
- Multi-platform testing (Linux, Windows, macOS)
- Code coverage via Codecov (90% target)
- Clippy and rustfmt checks
- Security audits via cargo-deny
- Benchmark tracking
- Automated documentation publishing
