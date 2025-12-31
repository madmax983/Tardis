# ADR-003: CUDA as Primary GPU Backend

## Status

Accepted

## Date

2024-12-31

## Context

Tardis's Vortex LLM engine requires GPU acceleration for practical inference speeds. We need to choose a primary GPU compute backend. Options include CUDA (NVIDIA), Metal (Apple), Vulkan Compute (cross-platform), and ROCm (AMD).

Requirements:
- High performance for transformer inference
- Good ecosystem and tooling
- Candle framework support
- Matrix multiplication optimization (tensor cores)

## Decision

We will use CUDA as the primary GPU backend via the `cudarc` crate, with Metal and Vulkan as future secondary targets.

## Consequences

### Positive

- **Best performance**: NVIDIA GPUs with CUDA offer the best LLM inference performance, especially with tensor cores
- **Mature ecosystem**: Extensive libraries (cuBLAS, cuDNN, CUTLASS) for optimized kernels
- **Candle support**: Candle has excellent, well-tested CUDA support
- **Industry standard**: Most LLM tooling assumes CUDA availability
- **cudarc crate**: Safe Rust bindings to CUDA runtime

### Negative

- **Vendor lock-in**: Requires NVIDIA GPU hardware
- **No Apple Silicon**: CUDA doesn't support Apple M-series chips (will need Metal later)
- **Licensing**: CUDA has proprietary licensing considerations
- **Driver dependency**: Requires NVIDIA driver installation

### Neutral

- Will need to implement Metal backend for macOS support in future
- Vulkan compute could provide AMD/Intel support but with more development effort

## Alternatives Considered

### Alternative 1: Metal First (Apple Silicon)

Target Apple Silicon with Metal as primary backend.

**Pros:**
- Excellent Apple Silicon support
- Growing Mac user base among developers
- Unified memory architecture simplifies CPU-GPU transfers

**Cons:**
- Limited to Apple hardware
- Smaller ecosystem for ML
- Less community tooling

**Why not chosen:** While Apple Silicon is compelling, NVIDIA GPUs dominate the AI/ML space with better performance and tooling.

### Alternative 2: Vulkan Compute

Use Vulkan compute shaders for cross-platform GPU support.

**Pros:**
- Works on NVIDIA, AMD, Intel, and even mobile GPUs
- Open standard
- Single codebase for all platforms

**Cons:**
- Lower performance than native CUDA
- More complex programming model
- Less ML-specific optimization
- Candle's Vulkan support is less mature

**Why not chosen:** Performance gap with CUDA is significant for LLM inference. Cross-platform benefits don't outweigh performance cost for primary target.

### Alternative 3: ROCm (AMD)

Use AMD's ROCm platform.

**Pros:**
- Open source
- Competitive AMD GPU hardware
- HIP provides CUDA-like API

**Cons:**
- Less mature ecosystem
- Candle ROCm support is limited
- Fewer users and less community support

**Why not chosen:** ROCm ecosystem is not mature enough for production use. May revisit as secondary target.

### Alternative 4: CPU Only Initially

Start with CPU-only inference, add GPU later.

**Pros:**
- Simpler initial implementation
- Works on any hardware
- No driver dependencies

**Cons:**
- 10-100x slower inference
- Not practical for larger models
- Users expect GPU acceleration

**Why not chosen:** CPU-only is too slow for the "Personal AI Computer" vision. GPU support is essential from early stages.

## References

- [CUDA Toolkit](https://developer.nvidia.com/cuda-toolkit)
- [cudarc crate](https://github.com/coreylowman/cudarc)
- [Candle CUDA backend](https://github.com/huggingface/candle)
- [NVIDIA Tensor Cores](https://www.nvidia.com/en-us/data-center/tensor-cores/)
