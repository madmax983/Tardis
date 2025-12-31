//! Inference runtime for Vortex.

mod runtime;

pub use runtime::Vortex;

use candle_core::Tensor;
use serde::{Deserialize, Serialize};

/// KV cache for a single layer.
#[derive(Debug)]
pub struct LayerKVCache {
    /// Key cache tensor.
    pub key: Tensor,
    /// Value cache tensor.
    pub value: Tensor,
    /// Current sequence length.
    pub seq_len: usize,
}

/// KV cache for the entire model.
#[derive(Debug)]
pub struct KVCache {
    /// Per-layer caches.
    pub layers: Vec<LayerKVCache>,
    /// Maximum sequence length.
    pub max_seq_len: usize,
}

impl KVCache {
    /// Create a new empty KV cache.
    #[must_use]
    pub fn new(num_layers: usize, max_seq_len: usize) -> Self {
        Self {
            layers: Vec::with_capacity(num_layers),
            max_seq_len,
        }
    }

    /// Clear the cache.
    pub fn clear(&mut self) {
        self.layers.clear();
    }

    /// Current sequence length.
    #[must_use]
    pub fn seq_len(&self) -> usize {
        self.layers.first().map(|l| l.seq_len).unwrap_or(0)
    }
}

/// Token usage statistics.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Number of prompt tokens.
    pub prompt_tokens: usize,
    /// Number of generated tokens.
    pub completion_tokens: usize,
}

impl TokenUsage {
    /// Total tokens used.
    #[must_use]
    pub const fn total(&self) -> usize {
        self.prompt_tokens + self.completion_tokens
    }
}

/// Reason for generation completion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FinishReason {
    /// Hit max tokens limit.
    Length,
    /// Hit a stop sequence.
    Stop,
    /// End of sequence token.
    EndOfSequence,
}
