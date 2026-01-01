//! Model configuration parsing.
//!
//! Parses config.json files from `HuggingFace` model directories.

use crate::error::{VortexError, VortexResult};
use crate::model::Architecture;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Parsed model configuration from config.json.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    /// Model architecture type.
    #[serde(default)]
    pub architecture: Architecture,
    /// Number of hidden layers.
    #[serde(alias = "num_hidden_layers", alias = "n_layer")]
    pub num_layers: usize,
    /// Hidden size (embedding dimension).
    #[serde(alias = "hidden_size", alias = "n_embd", alias = "dim")]
    pub hidden_size: usize,
    /// Intermediate size (FFN dimension).
    #[serde(alias = "intermediate_size", alias = "n_inner")]
    pub intermediate_size: Option<usize>,
    /// Number of attention heads.
    #[serde(alias = "num_attention_heads", alias = "n_head")]
    pub num_heads: usize,
    /// Number of key-value heads (for GQA).
    #[serde(alias = "num_key_value_heads")]
    pub num_kv_heads: Option<usize>,
    /// Vocabulary size.
    #[serde(alias = "vocab_size")]
    pub vocab_size: usize,
    /// Maximum sequence length.
    #[serde(alias = "max_position_embeddings", alias = "n_positions", alias = "max_seq_len")]
    pub max_seq_len: usize,
    /// RMS norm epsilon.
    #[serde(alias = "rms_norm_eps", alias = "layer_norm_epsilon", default = "default_rms_norm_eps")]
    pub rms_norm_eps: f64,
    /// Rope theta (for rotary embeddings).
    #[serde(alias = "rope_theta", default = "default_rope_theta")]
    pub rope_theta: f64,
    /// Beginning of sequence token ID.
    #[serde(alias = "bos_token_id")]
    pub bos_token_id: Option<u32>,
    /// End of sequence token ID.
    #[serde(alias = "eos_token_id")]
    pub eos_token_id: Option<u32>,
    /// Model type string from config.
    #[serde(alias = "model_type")]
    pub model_type: Option<String>,
    /// Architectures list from config.
    #[serde(default)]
    pub architectures: Vec<String>,
}

const fn default_rms_norm_eps() -> f64 {
    1e-5
}

const fn default_rope_theta() -> f64 {
    10000.0
}

impl ModelConfig {
    /// Estimate the number of parameters.
    #[must_use]
    pub fn estimate_parameters(&self) -> u64 {
        let intermediate = self.intermediate_size.unwrap_or(self.hidden_size * 4);

        // Embedding: vocab_size * hidden_size
        let embedding = self.vocab_size * self.hidden_size;

        // Per layer:
        // - Attention: 4 * hidden_size^2 (Q, K, V, O projections)
        // - FFN: 3 * hidden_size * intermediate (gate, up, down)
        // - Norms: 2 * hidden_size
        let per_layer = 4 * self.hidden_size * self.hidden_size
            + 3 * self.hidden_size * intermediate
            + 2 * self.hidden_size;

        // Output: hidden_size * vocab_size (often tied with embedding)
        let output = self.hidden_size * self.vocab_size;

        (embedding + self.num_layers * per_layer + output) as u64
    }

    /// Get effective number of KV heads (defaults to `num_heads` if not specified).
    #[must_use]
    pub fn effective_kv_heads(&self) -> usize {
        self.num_kv_heads.unwrap_or(self.num_heads)
    }
}

/// Parse model configuration from a directory containing config.json.
///
/// # Errors
///
/// Returns an error if config.json cannot be read or parsed.
pub fn parse_model_config(model_path: &Path) -> VortexResult<ModelConfig> {
    // Find config.json - could be in the path itself or alongside model files
    let config_path = if model_path.is_dir() {
        model_path.join("config.json")
    } else {
        // Model path is a file, look for config.json in same directory
        model_path
            .parent().map_or_else(|| model_path.with_file_name("config.json"), |p| p.join("config.json"))
    };

    if !config_path.exists() {
        return Err(VortexError::ConfigError(format!(
            "config.json not found at {}",
            config_path.display()
        )));
    }

    let config_str = std::fs::read_to_string(&config_path)?;
    let raw_config: serde_json::Value = serde_json::from_str(&config_str)?;

    // Detect architecture from raw config
    let architecture = Architecture::detect(&raw_config);

    // Parse the rest of the config
    let mut config: ModelConfig = serde_json::from_value(raw_config)?;
    config.architecture = architecture;

    tracing::info!(
        "Parsed model config: {} layers, {} hidden, {} heads, {} vocab",
        config.num_layers,
        config.hidden_size,
        config.num_heads,
        config.vocab_size
    );

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_llama_config() {
        let json = r#"{
            "architectures": ["LlamaForCausalLM"],
            "model_type": "llama",
            "num_hidden_layers": 32,
            "hidden_size": 4096,
            "intermediate_size": 11008,
            "num_attention_heads": 32,
            "num_key_value_heads": 32,
            "vocab_size": 32000,
            "max_position_embeddings": 4096,
            "rms_norm_eps": 1e-5,
            "rope_theta": 10000.0,
            "bos_token_id": 1,
            "eos_token_id": 2
        }"#;

        let raw: serde_json::Value = serde_json::from_str(json).unwrap();
        let architecture = Architecture::detect(&raw);
        assert_eq!(architecture, Architecture::Llama);

        let config: ModelConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.num_layers, 32);
        assert_eq!(config.hidden_size, 4096);
        assert_eq!(config.num_heads, 32);
        assert_eq!(config.vocab_size, 32000);
    }

    #[test]
    fn test_estimate_parameters() {
        let config = ModelConfig {
            architecture: Architecture::Llama,
            num_layers: 32,
            hidden_size: 4096,
            intermediate_size: Some(11008),
            num_heads: 32,
            num_kv_heads: Some(32),
            vocab_size: 32000,
            max_seq_len: 4096,
            rms_norm_eps: 1e-5,
            rope_theta: 10000.0,
            bos_token_id: Some(1),
            eos_token_id: Some(2),
            model_type: Some("llama".to_string()),
            architectures: vec!["LlamaForCausalLM".to_string()],
        };

        let params = config.estimate_parameters();
        // Llama 7B should be around 7 billion parameters
        assert!(params > 6_000_000_000);
        assert!(params < 8_000_000_000);
    }
}
