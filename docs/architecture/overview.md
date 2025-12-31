# Tardis OS - System Architecture Overview

## Vision

Tardis is a Rust-based operating system designed as a **Personal AI Computer** with:
- Local LLM inference via Candle
- Temporal knowledge management via GallifreyDB
- RAG (Retrieval-Augmented Generation) as a core OS capability
- Time-travel debugging and knowledge versioning

## High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           TARDIS OS ARCHITECTURE                            │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                         USER SPACE                                   │   │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────────┐   │   │
│  │  │   AI Shell   │  │  User Apps   │  │   Temporal File Manager  │   │   │
│  │  │   (REPL)     │  │              │  │   (time-travel browser)  │   │   │
│  │  └──────────────┘  └──────────────┘  └──────────────────────────┘   │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                    │                                        │
│                         ┌──────────┴──────────┐                            │
│                         │    System Calls     │                            │
│                         └──────────┬──────────┘                            │
│                                    │                                        │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                      CORE SERVICES LAYER                            │   │
│  │  ┌────────────────────────────────────────────────────────────┐    │   │
│  │  │                    CHRONOS (RAG Engine)                     │    │   │
│  │  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │    │   │
│  │  │  │  Retriever  │  │  Augmenter  │  │  Temporal Reasoner  │ │    │   │
│  │  │  └─────────────┘  └─────────────┘  └─────────────────────┘ │    │   │
│  │  └────────────────────────────────────────────────────────────┘    │   │
│  │                         │                    │                      │   │
│  │  ┌──────────────────────┴──┐  ┌─────────────┴────────────────┐    │   │
│  │  │   VORTEX (LLM Engine)   │  │   GALLIFREY (Temporal DB)    │    │   │
│  │  │  ┌───────────────────┐  │  │  ┌────────────────────────┐  │    │   │
│  │  │  │  Candle Runtime   │  │  │  │  GallifreyDB Instance  │  │    │   │
│  │  │  ├───────────────────┤  │  │  ├────────────────────────┤  │    │   │
│  │  │  │  Model Manager    │  │  │  │  Knowledge Graph       │  │    │   │
│  │  │  ├───────────────────┤  │  │  ├────────────────────────┤  │    │   │
│  │  │  │  Tokenizer Pool   │  │  │  │  Conversation Store    │  │    │   │
│  │  │  ├───────────────────┤  │  │  ├────────────────────────┤  │    │   │
│  │  │  │  KV Cache         │  │  │  │  System State Journal  │  │    │   │
│  │  │  └───────────────────┘  │  │  └────────────────────────┘  │    │   │
│  │  └─────────────────────────┘  └──────────────────────────────┘    │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                    │                                        │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                         KERNEL LAYER                                │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌───────────┐  │   │
│  │  │   Memory    │  │   Process   │  │     IPC     │  │  Drivers  │  │   │
│  │  │  Manager    │  │  Scheduler  │  │   (Async)   │  │  (HAL)    │  │   │
│  │  └─────────────┘  └─────────────┘  └─────────────┘  └───────────┘  │   │
│  │  ┌─────────────────────────────────────────────────────────────┐   │   │
│  │  │              Hardware Abstraction Layer (HAL)               │   │   │
│  │  │  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌───────────────┐  │   │   │
│  │  │  │   CPU   │  │   GPU   │  │  Memory │  │   Storage     │  │   │   │
│  │  │  │ (x86_64)│  │ (CUDA)  │  │  (NUMA) │  │   (NVMe)      │  │   │   │
│  │  │  └─────────┘  └─────────┘  └─────────┘  └───────────────┘  │   │   │
│  │  └─────────────────────────────────────────────────────────────┘   │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Subsystem Responsibilities

### Kernel Layer
- **Memory Manager**: Physical/virtual memory, huge pages (2MB/1GB), GPU memory bridge
- **Process Scheduler**: AI-aware scheduling with priority for inference tasks
- **IPC**: Zero-copy channels, async message passing, shared memory for tensors
- **HAL**: Hardware abstraction for CPU, GPU (CUDA), storage (NVMe)

### Core Services Layer
- **Vortex**: LLM inference runtime built on Candle with model management
- **Gallifrey**: Temporal storage with three stores (knowledge, conversation, system state)
- **Chronos**: RAG orchestration connecting retrieval, augmentation, and reasoning

### User Space
- **AI Shell**: Natural language REPL with time-travel capabilities
- **User Apps**: Standard applications with access to AI syscalls
- **Temporal File Manager**: Browse filesystem history, compare versions

## Data Flow

```
User Query
    │
    ▼
┌─────────┐     ┌──────────┐     ┌───────────┐
│  Shell  │────▶│ Chronos  │────▶│  Vortex   │
└─────────┘     │  (RAG)   │     │  (LLM)    │
                └────┬─────┘     └───────────┘
                     │
                     ▼
              ┌───────────┐
              │ Gallifrey │
              │(Temporal) │
              └───────────┘
```

1. User submits query to Shell
2. Shell forwards to Chronos
3. Chronos analyzes query, extracts temporal references
4. Chronos retrieves context from Gallifrey (knowledge + conversation + state)
5. Chronos augments prompt with retrieved context
6. Chronos sends augmented prompt to Vortex
7. Vortex performs inference, streams response
8. Chronos stores new conversation in Gallifrey
9. Shell displays response to user

## Memory Architecture

```
┌────────────────────────────────────────────────────────────┐
│                    Physical Memory                          │
├────────────────────────────────────────────────────────────┤
│  ┌─────────────────┐  ┌─────────────────────────────────┐  │
│  │  Kernel Space   │  │         User Space              │  │
│  │  (Identity Map) │  │                                 │  │
│  ├─────────────────┤  ├─────────────────────────────────┤  │
│  │ Kernel Code/Data│  │ Process Memory                  │  │
│  │ Page Tables     │  │ ├── Code/Data                   │  │
│  │ IPC Buffers     │  │ ├── Heap                        │  │
│  └─────────────────┘  │ └── Stack                       │  │
│                       ├─────────────────────────────────┤  │
│  ┌─────────────────┐  │ Shared Memory Regions           │  │
│  │   Huge Pages    │  │ ├── Model Weights (mmap)        │  │
│  │   (2MB/1GB)     │  │ ├── KV Cache                    │  │
│  │                 │  │ └── Tensor Buffers              │  │
│  │ Model Weights   │  ├─────────────────────────────────┤  │
│  │ KV Cache        │  │ GPU Memory (CUDA Managed)       │  │
│  │ Embeddings      │  │ ├── Model on GPU                │  │
│  └─────────────────┘  │ └── Compute Buffers             │  │
│                       └─────────────────────────────────┘  │
└────────────────────────────────────────────────────────────┘
```

## Syscall Interface

### Vortex Syscalls
```rust
sys_vortex_load_model(path, config) -> ModelHandle
sys_vortex_unload_model(handle)
sys_vortex_infer(handle, tokens, params) -> TokenStream
sys_vortex_embed(handle, tokens) -> Vec<f32>
sys_vortex_list_models() -> Vec<ModelInfo>
```

### Gallifrey Syscalls
```rust
sys_gallifrey_query(query, temporal) -> ResultSet
sys_gallifrey_insert(data, valid_time) -> NodeId
sys_gallifrey_time_travel(timestamp) -> Snapshot
sys_gallifrey_get_history(entity) -> Timeline
```

### Chronos Syscalls
```rust
sys_chronos_query(prompt, config) -> Response
sys_chronos_remember(data, category, temporal)
sys_chronos_recall(query, constraints) -> Memory
```

## Boot Sequence

1. **UEFI Firmware** loads bootloader
2. **Bootloader** sets up page tables, loads kernel
3. **Kernel Init**: Memory manager, scheduler, basic drivers
4. **Service Init**: Start Vortex, Gallifrey, Chronos
5. **Model Loading**: Memory-map default LLM weights
6. **Shell Launch**: Start AI shell for user interaction

## Security Model

- **Process Isolation**: Each process has isolated address space
- **Capability-Based Access**: Syscalls require explicit capabilities
- **Model Sandboxing**: LLM inference isolated from system state
- **Temporal Audit Log**: All system changes recorded for forensics
