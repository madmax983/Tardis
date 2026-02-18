# Tardis OS 🎻

> "All of time and space; everything that ever happened or ever will - where do you want to start?"

**Tardis OS** is an experimental Operating System designed with Artificial Intelligence at its core. Unlike traditional OSs where AI is an application layer, Tardis embeds Large Language Models (LLMs) and Vector Databases deep into the system services, enabling semantic understanding of data, temporal queries, and natural language interaction.

## 📚 Documentation

- [**Official Documentation**](./docs/README.md): Architecture, ADRs, and guides.
- [**API Docs**](https://madmax983.github.io/tardis/): Generated Rust documentation.

## 🚀 The Stack

Tardis is built as a Rust workspace containing several specialized crates:

| Crate | Role | Description |
|-------|------|-------------|
| **`kernel`** | 🧠 Core | UEFI bootloader and `no_std` kernel (requires Nightly). |
| **`vortex`** | 🌪️ Inference | LLM inference engine using [Candle](https://github.com/huggingface/candle). |
| **`gallifrey`**| 🕰️ Memory | Temporal knowledge store with bi-temporal queries. |
| **`chronos`** | ⚡ RAG | Orchestration layer bridging Vortex and Gallifrey. |
| **`shell`** | 🐚 UI | Natural language shell with intent routing. |
| **`telemetry`**| 📡 Observability | Full-stack tracing from Kernel to User space. |
| **`common`** | 🔧 Utils | Shared types and error handling. |

### ✨ Features

Tardis uses feature flags to enable experimental capabilities:

- **`nova`**: Enables experimental modules ("Nova" features) such as:
  - **Biographer**: Narrative generation from history.
  - **Sonic Screwdriver**: System diagnosis and repair tools.
  - **Curiosity**: Active learning engine.
  - **TUI Dashboard**: Rich terminal interface.

Enable it by running:
```bash
cargo run --features nova --bin tardis
```

## 🛠️ Getting Started

### Prerequisites

- **Rust:** Stable (1.85+) for userspace, Nightly for kernel.
- **Tools:** `qemu-system-x86_64` (for kernel testing).

### Build & Run

1. **Build Userspace Components:**
   ```bash
   cargo build
   ```

2. **Run the AI Shell:**
   ```bash
   cargo run --bin tardis
   ```

3. **Run Tests:**
   ```bash
   cargo test
   ```

4. **Build Documentation:**
   ```bash
   cargo doc --open
   ```

## 🤝 Contributing

We welcome contributions!

> **For Developers & AI Agents:** Please read [**CLAUDE.md**](./CLAUDE.md) first. It contains critical information about the workspace structure, coding standards, and the "worktree workflow" we use.

1. Fork the repo.
2. Create your feature branch (`git checkout -b feature/amazing-feature`).
3. Commit your changes.
4. Push to the branch.
5. Open a Pull Request.

## 📜 License

MIT OR Apache-2.0
