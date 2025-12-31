# ADR-005: Candle for LLM Runtime

## Status

Accepted

## Date

2024-12-31

## Context

Tardis's Vortex engine needs an ML framework for LLM inference. Requirements:
- Rust-native for integration with kernel/services
- Support for transformer architectures (Llama, Mistral, Phi, etc.)
- GPU acceleration (CUDA)
- Quantization support (Q4, Q8, FP16)
- Reasonable performance for interactive use

## Decision

We will use Hugging Face's Candle framework as the ML runtime for Vortex.

## Consequences

### Positive

- **Rust-native**: Written entirely in Rust, no Python or C++ FFI complexity
- **Lightweight**: Minimal dependencies compared to PyTorch/TensorFlow
- **Model support**: Llama, Mistral, Phi, Gemma, and more already implemented
- **Quantization**: GGUF/GGML quantized model loading
- **CUDA support**: Well-tested CUDA backend via cudarc
- **Active development**: Backed by Hugging Face with regular updates
- **Embeddable**: Can be embedded in kernel/services without Python runtime

### Negative

- **Smaller ecosystem**: Fewer pre-built models and examples than PyTorch
- **Less optimization**: May not match llama.cpp performance in all cases
- **Breaking changes**: API still evolving, may require updates
- **Documentation gaps**: Less comprehensive than mature frameworks

### Neutral

- May need to contribute model implementations for less common architectures
- Performance optimization opportunities through kernel fusion
- Could potentially contribute improvements back to Candle

## Alternatives Considered

### Alternative 1: llama.cpp with Rust Bindings

Use llama.cpp via FFI bindings (llama-cpp-rs).

**Pros:**
- Excellent performance (highly optimized)
- Broad model format support
- Very active development
- GGUF is the standard for quantized models

**Cons:**
- C++ codebase with Rust bindings (FFI complexity)
- Memory management across FFI boundary
- Harder to deeply integrate with Tardis internals
- Build complexity (CMake, CUDA toolkit)

**Why not chosen:** While llama.cpp has excellent performance, the FFI boundary adds complexity. Candle's pure Rust implementation integrates more cleanly.

### Alternative 2: PyTorch via tch-rs

Use PyTorch through Rust bindings.

**Pros:**
- Full PyTorch ecosystem access
- Extensive model zoo
- Mature and well-documented

**Cons:**
- Heavy dependency (libtorch, Python optional)
- Complex build process
- Large binary size
- Not embeddable in no_std kernel context

**Why not chosen:** PyTorch's weight makes it unsuitable for deep OS integration. The dependency complexity conflicts with Tardis's architecture.

### Alternative 3: ONNX Runtime

Use ONNX Runtime for inference.

**Pros:**
- Standard model format
- Good performance
- Cross-platform

**Cons:**
- Requires model conversion to ONNX
- Rust bindings less mature
- Transformer-specific optimizations limited

**Why not chosen:** ONNX's generic nature means less transformer-specific optimization. Model conversion adds friction.

### Alternative 4: Custom Inference Engine

Build a custom transformer inference engine.

**Pros:**
- Complete control
- Optimized for Tardis use cases
- No external dependencies

**Cons:**
- Massive development effort
- Would need to reimplement attention, layer norms, etc.
- Performance optimization is a deep specialization

**Why not chosen:** Building a competitive inference engine from scratch would take years and distract from Tardis's core mission.

## References

- [Candle Repository](https://github.com/huggingface/candle)
- [Candle Examples](https://github.com/huggingface/candle/tree/main/candle-examples)
- [Candle Transformers](https://github.com/huggingface/candle/tree/main/candle-transformers)
