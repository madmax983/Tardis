//! Model configuration parsing.
//!
//! Parses config.json files from `HuggingFace` model directories.

use crate::error::{VortexError, VortexResult};
use crate::model::Architecture;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::task;

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
    #[serde(
        alias = "max_position_embeddings",
        alias = "n_positions",
        alias = "max_seq_len"
    )]
    pub max_seq_len: usize,
    /// RMS norm epsilon.
    #[serde(
        alias = "rms_norm_eps",
        alias = "layer_norm_epsilon",
        default = "default_rms_norm_eps"
    )]
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
    ///
    /// Uses checked arithmetic to prevent overflow with malicious configs.
    /// Returns `u64::MAX` if any calculation would overflow.
    #[must_use]
    pub fn estimate_parameters(&self) -> u64 {
        // Convert to u64 early to avoid overflow in intermediate calculations
        let hidden = self.hidden_size as u64;
        let layers = self.num_layers as u64;
        let vocab = self.vocab_size as u64;
        let intermediate = self
            .intermediate_size
            .map_or_else(|| hidden.saturating_mul(4), |i| i as u64);

        // Use checked arithmetic to detect overflow
        let result = (|| -> Option<u64> {
            // Embedding: vocab_size * hidden_size
            let embedding = vocab.checked_mul(hidden)?;

            // Per layer:
            // - Attention: 4 * hidden_size^2 (Q, K, V, O projections)
            let attention = 4u64.checked_mul(hidden)?.checked_mul(hidden)?;
            // - FFN: 3 * hidden_size * intermediate (gate, up, down)
            let ffn = 3u64.checked_mul(hidden)?.checked_mul(intermediate)?;
            // - Norms: 2 * hidden_size
            let norms = 2u64.checked_mul(hidden)?;

            let per_layer = attention.checked_add(ffn)?.checked_add(norms)?;

            // Output: hidden_size * vocab_size (often tied with embedding)
            let output = hidden.checked_mul(vocab)?;

            // Total
            embedding
                .checked_add(layers.checked_mul(per_layer)?)?
                .checked_add(output)
        })();

        result.unwrap_or(u64::MAX)
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
pub async fn parse_model_config(model_path: &Path) -> VortexResult<ModelConfig> {
    // Find config.json using shared helper
    let config_path = super::find_model_file(model_path, "config.json");

    if !config_path.exists() {
        return Err(VortexError::ConfigError(format!(
            "config.json not found at {}",
            config_path.display()
        )));
    }

    // Read file asynchronously
    let config_str = tokio::fs::read_to_string(&config_path)
        .await
        .map_err(|e| VortexError::ConfigError(format!("Failed to read config.json: {e}")))?;

    // Parse JSON in a blocking task to avoid stalling the executor
    let config = task::spawn_blocking(move || -> VortexResult<ModelConfig> {
        let raw_config: serde_json::Value = serde_json::from_str(&config_str)?;

        // Detect architecture from raw config
        let architecture = Architecture::detect(&raw_config);

        // Parse the rest of the config
        let mut config: ModelConfig = serde_json::from_value(raw_config)?;
        config.architecture = architecture;

        Ok(config)
    })
    .await
    .map_err(|e| VortexError::ConfigError(format!("Config parsing task failed: {e}")))??;

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

    #[test]
    fn test_estimate_parameters_overflow_protection() {
        // Create a config with values that would overflow if not using checked arithmetic
        let config = ModelConfig {
            architecture: Architecture::Llama,
            num_layers: usize::MAX,
            hidden_size: usize::MAX,
            intermediate_size: Some(usize::MAX),
            num_heads: 32,
            num_kv_heads: Some(32),
            vocab_size: usize::MAX,
            max_seq_len: 4096,
            rms_norm_eps: 1e-5,
            rope_theta: 10000.0,
            bos_token_id: Some(1),
            eos_token_id: Some(2),
            model_type: Some("llama".to_string()),
            architectures: vec![],
        };

        let params = config.estimate_parameters();
        // Should return MAX on overflow, not panic
        assert_eq!(params, u64::MAX);
    }

    #[tokio::test]
    async fn test_parse_model_config_missing_file() {
        let result = parse_model_config(Path::new("/nonexistent/path")).await;
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(matches!(err, crate::error::VortexError::ConfigError(_)));
    }
}
