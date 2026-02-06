//! GGUF model loader.

use super::types::LoadedModel;
use crate::error::{VortexError, VortexResult};
use candle_core::quantized::gguf_file;
use candle_core::Device;
use candle_transformers::models::quantized_llama::ModelWeights as QuantizedLlama;
use std::path::Path;
use tracing::info;

/// Load a quantized model from a GGUF file.
///
/// GGUF files contain both weights and metadata, so no separate config is needed.
pub(crate) fn load_gguf(model_path: &Path, device: &Device) -> VortexResult<LoadedModel> {
    // Find GGUF file
    let gguf_path = find_gguf_file(model_path)?;

    info!("Loading GGUF model from {}", gguf_path.display());

    // Open and parse the GGUF file
    let mut file = std::fs::File::open(&gguf_path)?;
    let gguf_content = gguf_file::Content::read(&mut file)
        .map_err(|e| VortexError::LoadFailed(format!("Failed to parse GGUF file: {e}")))?;

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
    let model = QuantizedLlama::from_gguf(gguf_content, &mut file, device)
        .map_err(|e| VortexError::LoadFailed(format!("Failed to build quantized model: {e}")))?;

    info!(
        "Quantized Llama model loaded successfully ({} params)",
        parameters
    );

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
        "q2_k_s", "q2_k_m", "q2_k_l", "q2_k", "q3_k_s", "q3_k_m", "q3_k_l", "q3_k", "q4_k_s",
        "q4_k_m", "q4_k_l", "q4_k", "q5_k_s", "q5_k_m", "q5_k_l", "q5_k", "q6_k_s", "q6_k_m",
        "q6_k_l", "q6_k", // Standard quantization
        "q4_0", "q4_1", "q5_0", "q5_1", "q8_0", "q8_1", // Float types
        "f16", "f32", "bf16", // IQ quantization (newer formats)
        "iq1_s", "iq1_m", "iq2_xxs", "iq2_xs", "iq2_s", "iq2_m", "iq3_xxs", "iq3_xs", "iq3_s",
        "iq3_m", "iq4_xs", "iq4_nl",
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    // Quantization extraction tests

    #[test]
    fn test_extract_quantization_from_filename_q4_0() {
        assert_eq!(extract_quantization_from_filename("llama-7b-q4_0"), "Q4_0");
    }

    #[test]
    fn test_extract_quantization_from_filename_q8_0() {
        assert_eq!(
            extract_quantization_from_filename("model-Q8_0-GGUF"),
            "Q8_0"
        );
    }

    #[test]
    fn test_extract_quantization_from_filename_q4_k_m() {
        // More specific pattern matching now returns the full variant
        assert_eq!(
            extract_quantization_from_filename("tinyllama-q4_k_m"),
            "Q4_K_M"
        );
    }

    #[test]
    fn test_extract_quantization_from_filename_q4_k() {
        assert_eq!(extract_quantization_from_filename("model-q4_k"), "Q4_K");
    }

    #[test]
    fn test_extract_quantization_from_filename_unknown() {
        assert_eq!(extract_quantization_from_filename("model"), "unknown");
    }

    #[test]
    fn test_find_gguf_file_not_found() {
        let result = find_gguf_file(Path::new("/nonexistent/path"));
        assert!(result.is_err());
    }
}
