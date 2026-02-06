//! Shared types for model loading.

use candle_core::{DType, Device};
use candle_transformers::models::llama::{Config as LlamaRuntimeConfig, Llama};
use candle_transformers::models::quantized_llama::ModelWeights as QuantizedLlama;
use std::path::Path;

/// Model weight file format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelFormat {
    /// `SafeTensors` format (full precision F16/F32).
    SafeTensors,
    /// GGUF format (quantized models: Q4, Q8, etc.).
    Gguf,
}

impl ModelFormat {
    /// Detect model format from file extension.
    ///
    /// Returns `None` if the extension is not recognized.
    #[must_use]
    pub fn from_extension(path: &Path) -> Option<Self> {
        path.extension()
            .and_then(|ext| ext.to_str())
            .and_then(|ext| match ext.to_lowercase().as_str() {
                "safetensors" => Some(Self::SafeTensors),
                "gguf" => Some(Self::Gguf),
                _ => None,
            })
    }

    /// Detect model format from a directory by scanning for weight files.
    ///
    /// Prefers GGUF if both formats are present (usually smaller/faster).
    #[must_use]
    pub fn detect_from_directory(dir: &Path) -> Option<Self> {
        if !dir.is_dir() {
            return Self::from_extension(dir);
        }

        let mut has_safetensors = false;
        let mut has_gguf = false;

        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    match ext.to_lowercase().as_str() {
                        "safetensors" => has_safetensors = true,
                        "gguf" => has_gguf = true,
                        _ => {}
                    }
                }
            }
        }

        // Prefer GGUF (quantized) over SafeTensors (full precision) when both available
        if has_gguf {
            Some(Self::Gguf)
        } else if has_safetensors {
            Some(Self::SafeTensors)
        } else {
            None
        }
    }
}

/// A loaded model, ready for inference.
///
/// This enum wraps different model architectures in a common interface.
pub enum LoadedModel {
    /// Llama family model (Llama, Llama2, Llama3) in full precision.
    Llama {
        /// The Candle Llama model.
        model: Llama,
        /// Model configuration.
        config: LlamaRuntimeConfig,
        /// Device the model is loaded on.
        device: Device,
        /// Data type used.
        dtype: DType,
    },
    /// Quantized Llama model (GGUF format).
    QuantizedLlama {
        /// The quantized Llama model.
        model: QuantizedLlama,
        /// Device the model is loaded on.
        device: Device,
        /// Quantization type (e.g., "Q4\_0", "Q8\_0").
        quantization: String,
        /// Estimated parameter count.
        parameters: u64,
    },
    // Future: Add more architectures
    // Mistral { model: Mistral, config: MistralConfig, device: Device, dtype: DType },
    // Phi { model: Phi, config: PhiConfig, device: Device, dtype: DType },
}

// Manual Debug implementation since model types don't implement Debug
impl std::fmt::Debug for LoadedModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Llama {
                config,
                device,
                dtype,
                ..
            } => f
                .debug_struct("LoadedModel::Llama")
                .field("config", config)
                .field("device", device)
                .field("dtype", dtype)
                .finish(),
            Self::QuantizedLlama {
                device,
                quantization,
                parameters,
                ..
            } => f
                .debug_struct("LoadedModel::QuantizedLlama")
                .field("device", device)
                .field("quantization", quantization)
                .field("parameters", parameters)
                .finish(),
        }
    }
}

impl LoadedModel {
    /// Get the device this model is loaded on.
    #[must_use]
    pub const fn device(&self) -> &Device {
        match self {
            Self::Llama { device, .. } | Self::QuantizedLlama { device, .. } => device,
        }
    }

    /// Get the data type used by this model.
    ///
    /// For quantized models, returns the effective dtype for computations.
    #[must_use]
    pub const fn dtype(&self) -> DType {
        match self {
            Self::Llama { dtype, .. } => *dtype,
            // Quantized models typically compute in F32 but store in lower precision
            Self::QuantizedLlama { .. } => DType::F32,
        }
    }

    /// Check if this is a quantized model.
    #[must_use]
    pub const fn is_quantized(&self) -> bool {
        matches!(self, Self::QuantizedLlama { .. })
    }

    /// Get the quantization type if this is a quantized model.
    #[must_use]
    pub fn quantization_type(&self) -> Option<&str> {
        match self {
            Self::Llama { .. } => None,
            Self::QuantizedLlama { quantization, .. } => Some(quantization.as_str()),
        }
    }

    /// Get the model format.
    #[must_use]
    pub const fn format(&self) -> ModelFormat {
        match self {
            Self::Llama { .. } => ModelFormat::SafeTensors,
            Self::QuantizedLlama { .. } => ModelFormat::Gguf,
        }
    }

    /// Estimate memory usage in bytes.
    ///
    /// For quantized models, this includes an estimated 10% overhead for GGUF
    /// block metadata, padding, and other file structure elements.
    #[must_use]
    pub fn memory_bytes(&self) -> u64 {
        match self {
            Self::Llama { config, dtype, .. } => {
                let params = estimate_params_from_runtime_config(config);
                let bytes_per_param = match dtype {
                    DType::F32 => 4,
                    // All other types (F16, BF16, quantized) use 2 bytes
                    _ => 2,
                };
                params * bytes_per_param
            }
            Self::QuantizedLlama {
                quantization,
                parameters,
                ..
            } => {
                // Estimate bytes based on quantization type
                let bits_per_weight = estimate_bits_from_quantization(quantization);
                // Convert bits to bytes
                let base_bytes = parameters * u64::from(bits_per_weight) / 8;
                // Add ~10% overhead for GGUF block metadata, padding, etc.
                base_bytes.saturating_add(base_bytes / 10)
            }
        }
    }
}

/// Estimate bits per weight from quantization type string.
#[must_use]
pub fn estimate_bits_from_quantization(quant_type: &str) -> u8 {
    let quant_upper = quant_type.to_uppercase();
    if quant_upper.contains("Q2") {
        2
    } else if quant_upper.contains("Q3") {
        3
    } else if quant_upper.contains("Q4") {
        4
    } else if quant_upper.contains("Q5") {
        5
    } else if quant_upper.contains("Q6") {
        6
    } else if quant_upper.contains("Q8") {
        8
    } else if quant_upper.contains("F16") {
        16
    } else {
        // Default to 4-bit for unknown
        4
    }
}

/// Estimate parameter count from a Llama runtime config.
///
/// This uses the same formula as `ModelConfig::estimate_parameters` but works
/// with the Candle runtime config type.
#[must_use]
pub const fn estimate_params_from_runtime_config(config: &LlamaRuntimeConfig) -> u64 {
    let hidden = config.hidden_size as u64;
    let layers = config.num_hidden_layers as u64;
    let vocab = config.vocab_size as u64;
    let intermediate = config.intermediate_size as u64;

    // Embedding: vocab_size * hidden_size
    let embedding = vocab * hidden;

    // Per layer:
    // - Attention: 4 * hidden_size^2 (Q, K, V, O projections)
    // - FFN: 3 * hidden_size * intermediate (gate, up, down)
    // - Norms: 2 * hidden_size
    let per_layer = 4 * hidden * hidden + 3 * hidden * intermediate + 2 * hidden;

    // Output: hidden_size * vocab_size (often tied with embedding)
    let output = hidden * vocab;

    embedding + layers * per_layer + output
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_estimate_params_from_runtime_config() {
        let config = LlamaRuntimeConfig {
            hidden_size: 4096,
            intermediate_size: 11008,
            vocab_size: 32000,
            num_hidden_layers: 32,
            num_attention_heads: 32,
            num_key_value_heads: 32,
            rms_norm_eps: 1e-5,
            rope_theta: 10000.0,
            use_flash_attn: false,
            bos_token_id: Some(1),
            eos_token_id: None,
            rope_scaling: None,
            max_position_embeddings: 4096,
            tie_word_embeddings: false,
        };

        let params = estimate_params_from_runtime_config(&config);
        // Should be around 7B
        assert!(params > 6_000_000_000);
        assert!(params < 8_000_000_000);
    }

    // ModelFormat tests

    #[test]
    fn test_model_format_from_extension_safetensors() {
        let path = PathBuf::from("model.safetensors");
        assert_eq!(
            ModelFormat::from_extension(&path),
            Some(ModelFormat::SafeTensors)
        );
    }

    #[test]
    fn test_model_format_from_extension_gguf() {
        let path = PathBuf::from("model.gguf");
        assert_eq!(ModelFormat::from_extension(&path), Some(ModelFormat::Gguf));
    }

    #[test]
    fn test_model_format_from_extension_case_insensitive() {
        let path = PathBuf::from("model.GGUF");
        assert_eq!(ModelFormat::from_extension(&path), Some(ModelFormat::Gguf));

        let path = PathBuf::from("model.SafeTensors");
        assert_eq!(
            ModelFormat::from_extension(&path),
            Some(ModelFormat::SafeTensors)
        );
    }

    #[test]
    fn test_model_format_from_extension_unknown() {
        let path = PathBuf::from("model.bin");
        assert_eq!(ModelFormat::from_extension(&path), None);

        let path = PathBuf::from("model");
        assert_eq!(ModelFormat::from_extension(&path), None);
    }

    // Bits per weight estimation tests

    #[test]
    fn test_estimate_bits_from_quantization() {
        assert_eq!(estimate_bits_from_quantization("Q4_0"), 4);
        assert_eq!(estimate_bits_from_quantization("Q4_K_M"), 4);
        assert_eq!(estimate_bits_from_quantization("Q8_0"), 8);
        assert_eq!(estimate_bits_from_quantization("Q2_K"), 2);
        assert_eq!(estimate_bits_from_quantization("F16"), 16);
        assert_eq!(estimate_bits_from_quantization("unknown"), 4); // default
    }
}
