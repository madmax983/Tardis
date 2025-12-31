//! Model management for Vortex.
//!
//! Provides:
//! - Model registry for tracking available and loaded models
//! - Model loading from various formats
//! - Model metadata and capabilities

mod registry;

pub use registry::{ModelHandle, ModelInfo, ModelRegistry};

use serde::{Deserialize, Serialize};

/// Supported model architectures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Architecture {
    /// Meta's LLaMA family (1, 2, 3).
    Llama,
    /// Mistral AI's Mistral.
    Mistral,
    /// Mistral AI's Mixtral (MoE).
    Mixtral,
    /// Microsoft's Phi family.
    Phi,
    /// Google's Gemma.
    Gemma,
    /// RWKV (linear attention).
    Rwkv,
    /// Alibaba's Qwen.
    Qwen,
    /// Unknown/unsupported.
    Unknown,
}

impl Architecture {
    /// Detect architecture from model config.
    #[must_use]
    pub fn detect(config: &serde_json::Value) -> Self {
        // Check architectures field
        if let Some(archs) = config.get("architectures").and_then(|v| v.as_array()) {
            for arch in archs {
                if let Some(name) = arch.as_str() {
                    let lower = name.to_lowercase();
                    if lower.contains("llama") {
                        return Self::Llama;
                    }
                    if lower.contains("mistral") && !lower.contains("mixtral") {
                        return Self::Mistral;
                    }
                    if lower.contains("mixtral") {
                        return Self::Mixtral;
                    }
                    if lower.contains("phi") {
                        return Self::Phi;
                    }
                    if lower.contains("gemma") {
                        return Self::Gemma;
                    }
                    if lower.contains("rwkv") {
                        return Self::Rwkv;
                    }
                    if lower.contains("qwen") {
                        return Self::Qwen;
                    }
                }
            }
        }

        // Check model_type field
        if let Some(model_type) = config.get("model_type").and_then(|v| v.as_str()) {
            let lower = model_type.to_lowercase();
            if lower.contains("llama") {
                return Self::Llama;
            }
            if lower == "mistral" {
                return Self::Mistral;
            }
            if lower == "mixtral" {
                return Self::Mixtral;
            }
            if lower.contains("phi") {
                return Self::Phi;
            }
        }

        Self::Unknown
    }
}

impl std::fmt::Display for Architecture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Llama => write!(f, "llama"),
            Self::Mistral => write!(f, "mistral"),
            Self::Mixtral => write!(f, "mixtral"),
            Self::Phi => write!(f, "phi"),
            Self::Gemma => write!(f, "gemma"),
            Self::Rwkv => write!(f, "rwkv"),
            Self::Qwen => write!(f, "qwen"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

/// Quantization types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Quantization {
    /// Full 32-bit precision.
    F32,
    /// Half precision (16-bit float).
    F16,
    /// Brain float 16.
    BF16,
    /// 8-bit quantization.
    Q8_0,
    /// 4-bit quantization.
    Q4_0,
    /// 4-bit K-quant (medium).
    Q4_K_M,
    /// 4-bit K-quant (small).
    Q4_K_S,
    /// 5-bit K-quant (medium).
    Q5_K_M,
    /// 5-bit K-quant (small).
    Q5_K_S,
}

impl Quantization {
    /// Bits per weight for this quantization.
    #[must_use]
    pub const fn bits_per_weight(&self) -> f32 {
        match self {
            Self::F32 => 32.0,
            Self::F16 | Self::BF16 => 16.0,
            Self::Q8_0 => 8.0,
            Self::Q4_0 | Self::Q4_K_M | Self::Q4_K_S => 4.0,
            Self::Q5_K_M | Self::Q5_K_S => 5.0,
        }
    }

    /// Estimate memory usage for a given parameter count.
    #[must_use]
    pub fn memory_bytes(&self, parameters: u64) -> u64 {
        #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        let bytes = (parameters as f64 * f64::from(self.bits_per_weight()) / 8.0) as u64;
        bytes
    }
}

impl Default for Quantization {
    fn default() -> Self {
        Self::F16
    }
}
