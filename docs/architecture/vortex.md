# Vortex - LLM Inference Engine

## Overview

Vortex is Tardis's LLM inference engine, built on the Candle ML framework. It provides model management, tokenization, and inference with support for flexible model sizes from small (Phi) to large (Llama 70B).

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        VORTEX                               │
├─────────────────────────────────────────────────────────────┤
│  ┌───────────────────────────────────────────────────────┐  │
│  │                 Model Registry                         │  │
│  │  - Model metadata & capabilities                       │  │
│  │  - Hardware requirements mapping                       │  │
│  │  - Version management                                  │  │
│  └───────────────────────────────────────────────────────┘  │
│                            │                                 │
│  ┌───────────────────────────────────────────────────────┐  │
│  │                 Model Loader                           │  │
│  │  - SafeTensors / GGUF / GGML support                  │  │
│  │  - Quantization (Q4_0, Q4_K, Q8_0, FP16, BF16)        │  │
│  │  - Memory-mapped weight loading                        │  │
│  │  - Model sharding for multi-GPU                       │  │
│  └───────────────────────────────────────────────────────┘  │
│                            │                                 │
│  ┌───────────────────────────────────────────────────────┐  │
│  │                Inference Runtime                       │  │
│  │  - Candle compute graph execution                     │  │
│  │  - KV-Cache management (paged attention)              │  │
│  │  - Continuous batching                                │  │
│  │  - Speculative decoding                               │  │
│  └───────────────────────────────────────────────────────┘  │
│                            │                                 │
│  ┌───────────────────────────────────────────────────────┐  │
│  │               Tokenizer Service                        │  │
│  │  - HuggingFace tokenizers                             │  │
│  │  - BPE / SentencePiece / WordPiece                    │  │
│  │  - Cached tokenization                                │  │
│  │  - Streaming detokenization                           │  │
│  └───────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

## Supported Architectures

| Architecture | Models | Context Length | Notes |
|--------------|--------|----------------|-------|
| Llama | Llama 2/3, CodeLlama | 4K-128K | Primary target |
| Mistral | Mistral 7B, Mixtral 8x7B | 32K | MoE support |
| Phi | Phi-2, Phi-3 | 2K-128K | Small & efficient |
| Gemma | Gemma 2B/7B | 8K | Google's models |
| RWKV | RWKV-4/5/6 | Unlimited | Linear attention |
| Qwen | Qwen 2 | 32K-128K | Multilingual |

## Model Registry

```rust
pub struct ModelRegistry {
    models: HashMap<ModelId, ModelMetadata>,
    loaded: HashMap<ModelId, LoadedModel>,
}

pub struct ModelMetadata {
    pub id: ModelId,
    pub name: String,
    pub architecture: Architecture,
    pub parameters: u64,
    pub context_length: usize,
    pub quantization: Quantization,
    pub path: PathBuf,
    pub requirements: HardwareRequirements,
}

pub struct HardwareRequirements {
    pub min_memory_gb: f32,
    pub recommended_memory_gb: f32,
    pub supports_cpu: bool,
    pub supports_cuda: bool,
    pub supports_metal: bool,
}
```

## Model Loader

### Loading Process

```
1. Parse model metadata (config.json)
         │
         ▼
2. Select device (CPU/CUDA) based on availability
         │
         ▼
3. Memory-map weight files (safetensors/GGUF)
         │
         ▼
4. Load tokenizer configuration
         │
         ▼
5. Initialize model architecture with weights
         │
         ▼
6. Warm up KV cache
         │
         ▼
7. Return ModelHandle
```

### Quantization Support

```rust
pub enum Quantization {
    /// Full precision
    F32,
    /// Half precision
    F16,
    /// Brain float
    BF16,
    /// 8-bit integer
    Q8_0,
    /// 4-bit with group size 32
    Q4_0,
    /// 4-bit K-quant (better quality)
    Q4_K_M,
    Q4_K_S,
    /// 5-bit K-quant
    Q5_K_M,
    Q5_K_S,
}

impl Quantization {
    pub fn bits_per_weight(&self) -> f32 {
        match self {
            Self::F32 => 32.0,
            Self::F16 | Self::BF16 => 16.0,
            Self::Q8_0 => 8.0,
            Self::Q4_0 | Self::Q4_K_M | Self::Q4_K_S => 4.0,
            Self::Q5_K_M | Self::Q5_K_S => 5.0,
        }
    }

    pub fn memory_estimate(&self, parameters: u64) -> u64 {
        (parameters as f32 * self.bits_per_weight() / 8.0) as u64
    }
}
```

## Inference Runtime

### Core Types

```rust
pub struct InferenceRequest {
    pub model: ModelHandle,
    pub tokens: Vec<u32>,
    pub params: SamplingParams,
    pub stream: bool,
}

pub struct SamplingParams {
    pub temperature: f32,
    pub top_p: f32,
    pub top_k: usize,
    pub max_tokens: usize,
    pub stop_sequences: Vec<String>,
    pub presence_penalty: f32,
    pub frequency_penalty: f32,
}

pub struct InferenceResponse {
    pub tokens: Vec<u32>,
    pub text: String,
    pub finish_reason: FinishReason,
    pub usage: TokenUsage,
}
```

### KV Cache Management

```rust
pub struct KVCache {
    /// Key cache: [num_layers, batch_size, num_heads, seq_len, head_dim]
    key_cache: Tensor,
    /// Value cache: [num_layers, batch_size, num_heads, seq_len, head_dim]
    value_cache: Tensor,
    /// Current sequence length
    seq_len: usize,
    /// Maximum sequence length
    max_seq_len: usize,
}

impl KVCache {
    pub fn new(config: &ModelConfig, max_seq_len: usize, device: &Device) -> Self {
        let shape = (
            config.num_layers,
            1, // batch_size
            config.num_kv_heads,
            max_seq_len,
            config.head_dim,
        );

        Self {
            key_cache: Tensor::zeros(shape, DType::F16, device).unwrap(),
            value_cache: Tensor::zeros(shape, DType::F16, device).unwrap(),
            seq_len: 0,
            max_seq_len,
        }
    }

    pub fn append(&mut self, layer: usize, key: &Tensor, value: &Tensor) {
        // Append new KV to cache
    }

    pub fn clear(&mut self) {
        self.seq_len = 0;
    }
}
```

### Continuous Batching

```rust
pub struct BatchScheduler {
    /// Pending requests
    pending: VecDeque<InferenceRequest>,
    /// Currently running requests
    running: Vec<RunningRequest>,
    /// Maximum batch size
    max_batch_size: usize,
}

impl BatchScheduler {
    pub fn schedule(&mut self) -> Option<Batch> {
        // Collect requests that can fit in a batch
        let mut batch_tokens = 0;
        let mut batch_requests = Vec::new();

        while let Some(req) = self.pending.front() {
            if batch_requests.len() >= self.max_batch_size {
                break;
            }
            if batch_tokens + req.tokens.len() > MAX_BATCH_TOKENS {
                break;
            }

            let req = self.pending.pop_front().unwrap();
            batch_tokens += req.tokens.len();
            batch_requests.push(req);
        }

        if batch_requests.is_empty() {
            None
        } else {
            Some(Batch::new(batch_requests))
        }
    }
}
```

## Tokenizer Service

```rust
pub struct TokenizerService {
    tokenizers: HashMap<ModelId, Tokenizer>,
    cache: LruCache<(ModelId, String), Vec<u32>>,
}

impl TokenizerService {
    pub fn encode(&mut self, model: ModelId, text: &str) -> Vec<u32> {
        // Check cache first
        if let Some(tokens) = self.cache.get(&(model, text.to_string())) {
            return tokens.clone();
        }

        let tokenizer = self.tokenizers.get(&model).unwrap();
        let encoding = tokenizer.encode(text, true).unwrap();
        let tokens = encoding.get_ids().to_vec();

        // Cache result
        self.cache.put((model, text.to_string()), tokens.clone());

        tokens
    }

    pub fn decode(&self, model: ModelId, tokens: &[u32]) -> String {
        let tokenizer = self.tokenizers.get(&model).unwrap();
        tokenizer.decode(tokens, true).unwrap()
    }

    pub fn decode_stream(&self, model: ModelId) -> TokenDecoder {
        let tokenizer = self.tokenizers.get(&model).unwrap();
        TokenDecoder::new(tokenizer.clone())
    }
}
```

## Syscall Interface

```rust
/// Load a model from disk
/// Returns: ModelHandle on success, negative error code on failure
pub fn sys_vortex_load_model(
    path: *const u8,
    path_len: usize,
    config: *const LoadConfig,
) -> isize;

/// Unload a model
pub fn sys_vortex_unload_model(handle: ModelHandle) -> isize;

/// Run inference
/// Returns: Number of tokens generated, streams to output buffer
pub fn sys_vortex_infer(
    handle: ModelHandle,
    input_tokens: *const u32,
    input_len: usize,
    params: *const SamplingParams,
    output_tokens: *mut u32,
    output_capacity: usize,
) -> isize;

/// Generate embeddings
/// Returns: Embedding dimension
pub fn sys_vortex_embed(
    handle: ModelHandle,
    tokens: *const u32,
    token_len: usize,
    output: *mut f32,
    output_capacity: usize,
) -> isize;

/// List available models
pub fn sys_vortex_list_models(
    output: *mut ModelInfo,
    capacity: usize,
) -> isize;

/// Get model status
pub fn sys_vortex_model_status(
    handle: ModelHandle,
    status: *mut ModelStatus,
) -> isize;
```

## Directory Structure

```
vortex/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── model/
    │   ├── mod.rs
    │   ├── registry.rs       # Model registry
    │   ├── loader.rs         # Model loading
    │   ├── quantization.rs   # Quantization support
    │   └── architectures/    # Model architectures
    │       ├── mod.rs
    │       ├── llama.rs
    │       ├── mistral.rs
    │       ├── phi.rs
    │       └── rwkv.rs
    ├── inference/
    │   ├── mod.rs
    │   ├── runtime.rs        # Inference runtime
    │   ├── kv_cache.rs       # KV cache management
    │   ├── sampling.rs       # Token sampling
    │   └── batch.rs          # Continuous batching
    └── tokenizer/
        ├── mod.rs
        └── service.rs        # Tokenizer service
```

## Performance Considerations

### Memory Optimization
- Memory-mapped weights avoid copying on load
- KV cache reuse across requests
- Huge pages for model weights (2MB/1GB)

### Compute Optimization
- Flash Attention for efficient attention
- Fused kernels for layer norms
- INT8/INT4 matrix multiplication

### Latency Targets
- First token: < 100ms (warm cache)
- Per-token: < 20ms (7B model on RTX 4090)
- Model load: < 5s (memory-mapped)
