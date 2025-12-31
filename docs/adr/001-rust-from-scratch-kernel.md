# ADR-001: Rust From-Scratch Kernel

## Status

Accepted

## Date

2024-12-31

## Context

Tardis OS requires a kernel that can efficiently support AI workloads - specifically large language model inference and temporal database operations. We need to decide whether to build our kernel from scratch, fork an existing Rust OS (like Redox), or use a different approach entirely.

Key requirements:
- Support for huge pages (2MB/1GB) for model weights
- GPU memory management integration
- AI-aware process scheduling
- Custom syscall interface for LLM/RAG operations
- NUMA-aware memory allocation

## Decision

We will build the Tardis kernel from scratch using Rust's `no_std` environment.

## Consequences

### Positive

- **Full control over memory layout**: Can optimize specifically for LLM workloads with huge pages, GPU memory bridging, and model weight mmap
- **Custom syscall design**: Can create AI-native syscalls (`sys_vortex_*`, `sys_gallifrey_*`, `sys_chronos_*`) without retrofitting
- **AI-aware scheduling**: Can implement priority classes specifically for inference tasks
- **Learning opportunity**: Deep understanding of OS internals enables better optimization decisions
- **No legacy baggage**: Can make clean design decisions without compatibility constraints

### Negative

- **Significant development effort**: Building a kernel from scratch requires implementing many low-level components
- **Longer time to usable state**: Will take longer before we have a bootable system with basic functionality
- **Hardware support burden**: Must write or port drivers for all supported hardware
- **Debugging complexity**: Kernel bugs are notoriously difficult to debug

### Neutral

- Will need to implement standard kernel components (memory manager, scheduler, IPC) before AI-specific features
- May eventually need to port or implement POSIX-like APIs for user application compatibility

## Alternatives Considered

### Alternative 1: Fork Redox OS

Fork the Redox OS microkernel and add AI capabilities.

**Pros:**
- Mature Rust codebase with working drivers
- Faster path to bootable system
- Existing filesystem, networking, GUI

**Cons:**
- Microkernel architecture may add IPC overhead for LLM operations
- Would need significant modification for huge page support
- Existing design decisions may conflict with our requirements

**Why not chosen:** The architectural changes needed for AI-native operation would require such significant modifications that forking provides limited benefit over starting fresh.

### Alternative 2: Linux Kernel Module / eBPF

Implement Tardis as Linux kernel modules or eBPF programs.

**Pros:**
- Leverage existing Linux hardware support
- Easier development and debugging
- Can use existing GPU drivers (NVIDIA, AMD)

**Cons:**
- Limited by Linux's existing architecture
- Cannot deeply integrate temporal awareness
- Not a true "AI-native OS" - just extensions

**Why not chosen:** This approach would not achieve the vision of an AI-native operating system with temporal awareness at its core.

### Alternative 3: Unikernel Approach

Build a unikernel that combines application and kernel into a single address space.

**Pros:**
- Minimal overhead
- Fast boot times
- Simple security model (single application)

**Cons:**
- Limited to single application
- Not suitable for general-purpose "Personal AI Computer" vision
- Would need separate unikernel for each service

**Why not chosen:** The vision of Tardis as a personal AI computer requires multi-application support, which unikernels don't provide.

## References

- [Writing an OS in Rust](https://os.phil-opp.com/)
- [Redox OS](https://www.redox-os.org/)
- [The UEFI-rs crate](https://github.com/rust-osdev/uefi-rs)
