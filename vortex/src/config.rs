//! Configuration types for Vortex.

use serde::{Deserialize, Serialize};

/// Configuration for loading a model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelLoadConfig {
    /// Device to load on ("cpu", "cuda:0", "metal").
    pub device: String,
    /// Quantization type (none, `q4_0`, `q4_k`, `q8_0`, f16).
    pub quantization: Option<String>,
    /// Maximum context length (overrides model default).
    pub max_context_length: Option<usize>,
    /// Use memory mapping for weights.
    pub use_mmap: bool,
    /// Tensor parallelism across GPUs.
    pub tensor_parallel: usize,
}

impl Default for ModelLoadConfig {
    fn default() -> Self {
        Self {
            device: "cpu".to_string(),
            quantization: None,
            max_context_length: None,
            use_mmap: true,
            tensor_parallel: 1,
        }
    }
}

impl ModelLoadConfig {
    /// Create configuration for CPU inference.
    #[must_use]
    pub fn cpu() -> Self {
        Self::default()
    }

    /// Create configuration for CUDA GPU inference.
    #[must_use]
    pub fn cuda(device_id: usize) -> Self {
        Self {
            device: format!("cuda:{device_id}"),
            ..Self::default()
        }
    }

    /// Create configuration for Metal GPU inference (macOS).
    #[must_use]
    pub fn metal() -> Self {
        Self {
            device: "metal".to_string(),
            ..Self::default()
        }
    }

    /// Set quantization type.
    #[must_use]
    pub fn with_quantization(mut self, quant: &str) -> Self {
        self.quantization = Some(quant.to_string());
        self
    }

    /// Set maximum context length.
    #[must_use]
    pub const fn with_context_length(mut self, length: usize) -> Self {
        self.max_context_length = Some(length);
        self
    }
}

/// Parameters for inference.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceParams {
    /// Sampling temperature (0.0 = deterministic, higher = more random).
    pub temperature: f32,
    /// Top-p (nucleus) sampling threshold.
    pub top_p: f32,
    /// Top-k sampling (0 = disabled).
    pub top_k: usize,
    /// Maximum tokens to generate.
    pub max_tokens: usize,
    /// Stop sequences.
    pub stop_sequences: Vec<String>,
    /// Repetition penalty.
    pub repetition_penalty: f32,
    /// Seed for reproducibility (None = random).
    pub seed: Option<u64>,
}

impl Default for InferenceParams {
    fn default() -> Self {
        Self {
            temperature: 0.7,
            top_p: 0.9,
            top_k: 40,
            max_tokens: 2048,
            stop_sequences: Vec::new(),
            repetition_penalty: 1.1,
            seed: None,
        }
    }
}

impl InferenceParams {
    /// Create deterministic parameters (temperature = 0).
    #[must_use]
    pub fn deterministic() -> Self {
        Self {
            temperature: 0.0,
            top_p: 1.0,
            top_k: 0,
            ..Self::default()
        }
    }

    /// Create creative parameters (higher temperature).
    #[must_use]
    pub fn creative() -> Self {
        Self {
            temperature: 1.0,
            top_p: 0.95,
            top_k: 50,
            ..Self::default()
        }
    }

    /// Set temperature.
    #[must_use]
    pub const fn with_temperature(mut self, temp: f32) -> Self {
        self.temperature = temp;
        self
    }

    /// Set maximum tokens.
    #[must_use]
    pub const fn with_max_tokens(mut self, tokens: usize) -> Self {
        self.max_tokens = tokens;
        self
    }

    /// Add a stop sequence.
    #[must_use]
    pub fn with_stop(mut self, sequence: &str) -> Self {
        self.stop_sequences.push(sequence.to_string());
        self
    }
}
