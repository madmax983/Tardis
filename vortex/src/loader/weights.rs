//! Model weight loading.
//!
//! Handles loading weights from `SafeTensors` and GGUF formats,
//! creating Candle model instances.

use super::config::ModelConfig;
use super::device::DeviceSpec;
use crate::error::{VortexError, VortexResult};
use crate::model::Architecture;
use candle_core::quantized::gguf_file;
use candle_core::{DType, Device};
use candle_nn::VarBuilder;
use candle_transformers::models::llama::{Config as LlamaRuntimeConfig, Llama, LlamaConfig};
use candle_transformers::models::quantized_llama::ModelWeights as QuantizedLlama;
use std::path::Path;
use tracing::{info, instrument};

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
            Self::Llama { config, device, dtype, .. } => {
                f.debug_struct("LoadedModel::Llama")
                    .field("config", config)
                    .field("device", device)
                    .field("dtype", dtype)
                    .finish()
            }
            Self::QuantizedLlama { device, quantization, parameters, .. } => {
                f.debug_struct("LoadedModel::QuantizedLlama")
                    .field("device", device)
                    .field("quantization", quantization)
                    .field("parameters", parameters)
                    .finish()
            }
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
            Self::QuantizedLlama { quantization, parameters, .. } => {
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
fn estimate_bits_from_quantization(quant_type: &str) -> u8 {
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
const fn estimate_params_from_runtime_config(config: &LlamaRuntimeConfig) -> u64 {
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

/// Load model weights from disk.
///
/// Automatically detects the model format (`SafeTensors` or GGUF) and loads
/// accordingly. For GGUF files, the config parameter is optional as metadata
/// is embedded in the file.
///
/// # Arguments
///
/// * `model_path` - Path to model directory or weight file
/// * `config` - Parsed model configuration (used for `SafeTensors`, optional for GGUF)
/// * `device_spec` - Device to load on
/// * `_use_mmap` - Ignored; memory mapping is always used for performance
///
/// # Errors
///
/// Returns an error if weights cannot be loaded.
#[instrument(skip(config))]
#[allow(clippy::used_underscore_binding)] // _use_mmap is intentionally ignored, kept for API compatibility
pub fn load_model_weights(
    model_path: &Path,
    config: &ModelConfig,
    device_spec: &DeviceSpec,
    _use_mmap: bool,
) -> VortexResult<LoadedModel> {
    // Create the device
    let device = super::device::create_device(device_spec)?;

    // Detect format
    let format = ModelFormat::detect_from_directory(model_path)
        .ok_or_else(|| VortexError::LoadFailed(format!(
            "Could not detect model format in {}",
            model_path.display()
        )))?;

    info!(
        "Loading {} model ({:?} format) on {:?}",
        config.architecture, format, device_spec
    );

    match format {
        ModelFormat::Gguf => {
            // Load quantized model from GGUF
            load_gguf(model_path, &device)
        }
        ModelFormat::SafeTensors => {
            // Determine dtype based on device for full-precision models
            let dtype = match &device {
                Device::Cpu => DType::F32, // CPU works best with F32
                _ => DType::F16,           // GPU can use F16
            };

            match config.architecture {
                Architecture::Llama | Architecture::Mistral => {
                    // Mistral uses same architecture as Llama in candle
                    load_llama_safetensors(model_path, config, &device, dtype)
                }
                arch => Err(VortexError::UnsupportedArchitecture(arch.to_string())),
            }
        }
    }
}

/// Load a quantized model from a GGUF file.
///
/// GGUF files contain both weights and metadata, so no separate config is needed.
fn load_gguf(model_path: &Path, device: &Device) -> VortexResult<LoadedModel> {
    // Find GGUF file
    let gguf_path = find_gguf_file(model_path)?;

    info!("Loading GGUF model from {}", gguf_path.display());

    // Open and parse the GGUF file
    let mut file = std::fs::File::open(&gguf_path)?;
    let gguf_content = gguf_file::Content::read(&mut file).map_err(|e| {
        VortexError::LoadFailed(format!("Failed to parse GGUF file: {e}"))
    })?;

    // Extract metadata for logging
    let arch = gguf_content
        .metadata
        .get("general.architecture")
        .and_then(|v| v.to_string().ok())
        .map_or_else(|| "unknown".to_string(), String::from);

    let quant_type = gguf_content
        .metadata
        .get("general.quantization_version")
        .and_then(|v| v.to_string().ok())
        .map(String::from)
        .or_else(|| {
            // Try to infer from file name
            gguf_path
                .file_stem()
                .and_then(|s| s.to_str())
                .map(extract_quantization_from_filename)
        })
        .unwrap_or_else(|| "unknown".to_string());

    info!("GGUF architecture: {}, quantization: {}", arch, quant_type);

    // Estimate parameters from GGUF metadata
    let parameters = estimate_params_from_gguf(&gguf_content)?;

    // Build quantized model
    // Note: The file handle is reused after metadata read. QuantizedLlama::from_gguf
    // expects the file position to be after metadata and seeks internally as needed
    // to read weight tensors.
    let model = QuantizedLlama::from_gguf(gguf_content, &mut file, device).map_err(|e| {
        VortexError::LoadFailed(format!("Failed to build quantized model: {e}"))
    })?;

    info!("Quantized Llama model loaded successfully ({} params)", parameters);

    Ok(LoadedModel::QuantizedLlama {
        model,
        device: device.clone(),
        quantization: quant_type,
        parameters,
    })
}

/// Extract quantization type from filename (e.g., "model-q4\_0" -> "Q4\_0").
///
/// Supports common quantization patterns including K-quant variants.
fn extract_quantization_from_filename(filename: &str) -> String {
    let lower = filename.to_lowercase();

    // Common quantization patterns in order of specificity (more specific first)
    // Include K-quant variants like q4_k_s, q4_k_m, q5_k_s, q5_k_m, etc.
    let patterns = [
        // K-quant variants (check specific sizes first)
        "q2_k_s", "q2_k_m", "q2_k_l", "q2_k",
        "q3_k_s", "q3_k_m", "q3_k_l", "q3_k",
        "q4_k_s", "q4_k_m", "q4_k_l", "q4_k",
        "q5_k_s", "q5_k_m", "q5_k_l", "q5_k",
        "q6_k_s", "q6_k_m", "q6_k_l", "q6_k",
        // Standard quantization
        "q4_0", "q4_1",
        "q5_0", "q5_1",
        "q8_0", "q8_1",
        // Float types
        "f16", "f32", "bf16",
        // IQ quantization (newer formats)
        "iq1_s", "iq1_m",
        "iq2_xxs", "iq2_xs", "iq2_s", "iq2_m",
        "iq3_xxs", "iq3_xs", "iq3_s", "iq3_m",
        "iq4_xs", "iq4_nl",
    ];

    for pattern in patterns {
        if lower.contains(pattern) {
            return pattern.to_uppercase();
        }
    }

    // Fallback: try to extract any q/f pattern dynamically
    if let Some(quant) = extract_dynamic_quant_pattern(&lower) {
        return quant;
    }

    "unknown".to_string()
}

/// Try to extract a quantization pattern dynamically from the filename.
fn extract_dynamic_quant_pattern(filename: &str) -> Option<String> {
    // Look for patterns like q[0-9]_* or f[0-9]+
    for word in filename.split(|c: char| !c.is_alphanumeric() && c != '_') {
        if (word.starts_with('q') || word.starts_with('f') || word.starts_with("iq"))
            && word.len() >= 2
            && word.chars().nth(1).is_some_and(|c| c.is_ascii_digit())
        {
            return Some(word.to_uppercase());
        }
    }
    None
}

/// Estimate parameter count from GGUF metadata.
///
/// # Errors
///
/// Returns an error if required metadata fields are missing from the GGUF file.
fn estimate_params_from_gguf(content: &gguf_file::Content) -> VortexResult<u64> {
    // Extract required dimensions from metadata
    let hidden_size = u64::from(
        content
            .metadata
            .get("llama.embedding_length")
            .and_then(|v| v.to_u32().ok())
            .ok_or_else(|| {
                VortexError::LoadFailed(
                    "GGUF metadata missing 'llama.embedding_length'".to_string(),
                )
            })?,
    );

    let num_layers = u64::from(
        content
            .metadata
            .get("llama.block_count")
            .and_then(|v| v.to_u32().ok())
            .ok_or_else(|| {
                VortexError::LoadFailed("GGUF metadata missing 'llama.block_count'".to_string())
            })?,
    );

    let vocab_size = u64::from(
        content
            .metadata
            .get("llama.vocab_size")
            .and_then(|v| v.to_u32().ok())
            .ok_or_else(|| {
                VortexError::LoadFailed("GGUF metadata missing 'llama.vocab_size'".to_string())
            })?,
    );

    // intermediate_size is optional, default to 4x hidden_size
    let intermediate_size = content
        .metadata
        .get("llama.feed_forward_length")
        .and_then(|v| v.to_u32().ok())
        .map_or(hidden_size * 4, u64::from);

    // Same formula as estimate_params_from_runtime_config
    let embedding = vocab_size * hidden_size;
    let per_layer =
        4 * hidden_size * hidden_size + 3 * hidden_size * intermediate_size + 2 * hidden_size;
    let output = hidden_size * vocab_size;

    Ok(embedding + num_layers * per_layer + output)
}

/// Find a GGUF file in the given path.
///
/// When a directory contains multiple GGUF files, this function sorts them
/// alphabetically and returns the first one for deterministic behavior.
fn find_gguf_file(model_path: &Path) -> VortexResult<std::path::PathBuf> {
    if model_path.is_file() {
        if model_path.extension().is_some_and(|ext| ext == "gguf") {
            return Ok(model_path.to_path_buf());
        }
        return Err(VortexError::LoadFailed(format!(
            "Expected GGUF file, got: {}",
            model_path.display()
        )));
    }

    if model_path.is_dir() {
        // Collect all GGUF files, sort for deterministic behavior
        let mut gguf_files: Vec<_> = std::fs::read_dir(model_path)?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "gguf"))
            .collect();

        if !gguf_files.is_empty() {
            gguf_files.sort();

            if gguf_files.len() > 1 {
                info!(
                    "Multiple GGUF files found, using: {}",
                    gguf_files[0].display()
                );
            }

            return Ok(gguf_files.remove(0));
        }
    }

    Err(VortexError::LoadFailed(format!(
        "No GGUF file found in {}",
        model_path.display()
    )))
}

/// Load a Llama model from `SafeTensors` format.
fn load_llama_safetensors(
    model_path: &Path,
    config: &ModelConfig,
    device: &Device,
    dtype: DType,
) -> VortexResult<LoadedModel> {
    // Build LlamaConfig that matches candle-transformers' expected format
    #[allow(clippy::cast_possible_truncation)]
    let llama_config = LlamaConfig {
        hidden_size: config.hidden_size,
        intermediate_size: config.intermediate_size.unwrap_or(config.hidden_size * 4),
        vocab_size: config.vocab_size,
        num_hidden_layers: config.num_layers,
        num_attention_heads: config.num_heads,
        num_key_value_heads: config.num_kv_heads,
        rms_norm_eps: config.rms_norm_eps,
        rope_theta: config.rope_theta as f32,
        bos_token_id: config.bos_token_id,
        eos_token_id: config.eos_token_id.map(candle_transformers::models::llama::LlamaEosToks::Single),
        rope_scaling: None,
        max_position_embeddings: config.max_seq_len,
        tie_word_embeddings: None,
    };

    // Convert to runtime config
    let runtime_config = llama_config.into_config(false);

    info!(
        "Creating Llama model: {} layers, {} hidden, {} heads",
        runtime_config.num_hidden_layers, runtime_config.hidden_size, runtime_config.num_attention_heads
    );

    // Find weight files
    let weight_files = find_safetensors_files(model_path)?;
    info!("Found {} weight file(s)", weight_files.len());

    // Create VarBuilder from weight files using memory mapping for performance
    info!("Loading weights with memory mapping");

    // SAFETY: Memory-mapped file loading requires the following invariants:
    // 1. Files must not be modified by other processes while mapped - the caller
    //    is responsible for ensuring model files are stable during loading.
    // 2. Files must remain valid for the lifetime of the VarBuilder/model - the
    //    LoadedModel owns the mapping and keeps files valid.
    // 3. The safetensors format includes checksums that candle validates during
    //    loading, protecting against corruption.
    // 4. We only read from the mapped memory, never write.
    #[allow(unsafe_code)]
    let vb = unsafe { VarBuilder::from_mmaped_safetensors(&weight_files, dtype, device)? };

    // Build the model
    info!("Building Llama model...");
    let model = Llama::load(vb, &runtime_config).map_err(|e| {
        VortexError::LoadFailed(format!("Failed to build Llama model: {e}"))
    })?;

    info!("Llama model loaded successfully");

    Ok(LoadedModel::Llama {
        model,
        config: runtime_config,
        device: device.clone(),
        dtype,
    })
}

/// Find `SafeTensors` weight files in a model directory.
///
/// This function uses blocking I/O. For async contexts, wrap the call
/// to `load_model_weights` in `spawn_blocking`.
fn find_safetensors_files(model_path: &Path) -> VortexResult<Vec<std::path::PathBuf>> {
    let mut files = Vec::new();

    if model_path.is_file() {
        // Single file provided
        if model_path.extension().is_some_and(|ext| ext == "safetensors") {
            files.push(model_path.to_path_buf());
        } else {
            return Err(VortexError::LoadFailed(format!(
                "Unsupported file format: {}",
                model_path.display()
            )));
        }
    } else if model_path.is_dir() {
        // Search directory for weight files
        // First, try model.safetensors (single file models)
        let single_file = model_path.join("model.safetensors");
        if single_file.exists() {
            files.push(single_file);
        } else {
            // Look for sharded files: model-00001-of-00002.safetensors
            for entry in std::fs::read_dir(model_path)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().is_some_and(|ext| ext == "safetensors") {
                    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    // Include sharded files or any .safetensors files
                    if name.starts_with("model") || name.contains("safetensors") {
                        files.push(path);
                    }
                }
            }
        }
    } else {
        return Err(VortexError::ModelNotFound {
            path: model_path.display().to_string(),
        });
    }

    if files.is_empty() {
        return Err(VortexError::LoadFailed(format!(
            "No SafeTensors files found in {}",
            model_path.display()
        )));
    }

    // Sort files to ensure consistent ordering for sharded models
    files.sort();

    Ok(files)
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
        assert_eq!(ModelFormat::from_extension(&path), Some(ModelFormat::SafeTensors));
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
        assert_eq!(ModelFormat::from_extension(&path), Some(ModelFormat::SafeTensors));
    }

    #[test]
    fn test_model_format_from_extension_unknown() {
        let path = PathBuf::from("model.bin");
        assert_eq!(ModelFormat::from_extension(&path), None);

        let path = PathBuf::from("model");
        assert_eq!(ModelFormat::from_extension(&path), None);
    }

    // Quantization extraction tests

    #[test]
    fn test_extract_quantization_from_filename_q4_0() {
        assert_eq!(extract_quantization_from_filename("llama-7b-q4_0"), "Q4_0");
    }

    #[test]
    fn test_extract_quantization_from_filename_q8_0() {
        assert_eq!(extract_quantization_from_filename("model-Q8_0-GGUF"), "Q8_0");
    }

    #[test]
    fn test_extract_quantization_from_filename_q4_k_m() {
        // More specific pattern matching now returns the full variant
        assert_eq!(extract_quantization_from_filename("tinyllama-q4_k_m"), "Q4_K_M");
    }

    #[test]
    fn test_extract_quantization_from_filename_q4_k() {
        assert_eq!(extract_quantization_from_filename("model-q4_k"), "Q4_K");
    }

    #[test]
    fn test_extract_quantization_from_filename_unknown() {
        assert_eq!(extract_quantization_from_filename("model"), "unknown");
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

    // Error path tests

    #[test]
    fn test_find_gguf_file_not_found() {
        let result = find_gguf_file(Path::new("/nonexistent/path"));
        assert!(result.is_err());
    }

    #[test]
    fn test_find_safetensors_files_not_found() {
        let result = find_safetensors_files(Path::new("/nonexistent/path"));
        assert!(result.is_err());
    }
}
