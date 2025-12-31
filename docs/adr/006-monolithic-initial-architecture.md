# ADR-006: Monolithic Initial Architecture

## Status

Accepted

## Date

2024-12-31

## Context

We need to decide on the kernel architecture for Tardis. The two main approaches are:
- **Monolithic**: All kernel services (memory, scheduling, drivers, AI services) run in kernel space
- **Microkernel**: Minimal kernel with services running as userspace processes communicating via IPC

This decision affects development velocity, performance characteristics, and long-term maintainability.

## Decision

We will start with a monolithic architecture, but design clean interfaces between subsystems to enable potential future microkernel extraction.

## Consequences

### Positive

- **Faster development**: No IPC overhead to design and implement initially
- **Simpler debugging**: All code in same address space, easier to trace
- **Better initial performance**: No context switch overhead for service calls
- **LLM latency**: Direct function calls for inference requests instead of IPC
- **Tensor sharing**: Easy to share large tensor buffers without IPC complexity
- **Proven approach**: Linux-style monolithic kernels are well-understood

### Negative

- **Less isolation**: Bug in one subsystem can crash entire kernel
- **Larger trusted computing base**: More code runs in privileged mode
- **Harder to test in isolation**: Subsystems more coupled
- **Future migration effort**: If we decide to go microkernel later, significant refactoring needed

### Neutral

- Will design subsystem interfaces as if they were IPC boundaries
- Each subsystem (Vortex, Gallifrey, Chronos) will have clean API surfaces
- Can incrementally move services to userspace if needed

## Alternatives Considered

### Alternative 1: Microkernel from Start

Build as a microkernel like seL4 or Redox.

**Pros:**
- Better isolation and fault tolerance
- Easier to formally verify small kernel
- More modular, easier to update individual services
- Security benefits from isolation

**Cons:**
- IPC overhead for every service call
- Complex tensor sharing across process boundaries
- Longer time to initial functionality
- More infrastructure needed (IPC, service discovery)

**Why not chosen:** The IPC overhead is particularly problematic for LLM inference where we're passing large tensors and need sub-millisecond latencies. We can always migrate to microkernel later with clean interfaces.

### Alternative 2: Hybrid Kernel

Run some services in kernel (performance-critical) and others in userspace.

**Pros:**
- Balance of performance and isolation
- Critical paths fast, less critical paths isolated

**Cons:**
- Complexity of managing both modes
- Decision of what goes where is arbitrary initially
- Still need IPC infrastructure

**Why not chosen:** Adds complexity without clear benefit at this stage. Better to start simple and add complexity as needed.

## Design Constraints

To preserve future microkernel option, we will:

1. **Define clear interfaces**: Each subsystem exposes a trait-based API
2. **No direct memory access**: Subsystems interact through defined interfaces, not shared mutable state
3. **Async-ready**: Design APIs to be async-compatible for future IPC
4. **Capability-based**: Access control designed as if subsystems were separate processes

```rust
// Example: Vortex interface designed for potential IPC
pub trait VortexService: Send + Sync {
    async fn load_model(&self, path: &Path, config: LoadConfig) -> Result<ModelHandle>;
    async fn infer(&self, handle: ModelHandle, request: InferenceRequest) -> Result<TokenStream>;
    async fn embed(&self, handle: ModelHandle, tokens: &[u32]) -> Result<Vec<f32>>;
}

// Initially: Direct implementation
impl VortexService for VortexDirect { ... }

// Future: IPC implementation
impl VortexService for VortexIpc { ... }
```

## References

- [Linus Torvalds on Microkernels](https://www.oreilly.com/openbook/opensources/book/linus.html)
- [seL4 Microkernel](https://sel4.systems/)
- [Redox OS Microkernel](https://www.redox-os.org/)
- [Hybrid Kernel Approach](https://en.wikipedia.org/wiki/Hybrid_kernel)
