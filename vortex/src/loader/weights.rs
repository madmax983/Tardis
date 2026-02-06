//! Model weight loading facade.
//!
//! Handles loading weights from `SafeTensors` and GGUF formats by delegating
//! to specific loaders.

use super::config::ModelConfig;
use super::device::DeviceSpec;
use super::gguf_loader::load_gguf;
use super::safetensors_loader::load_llama_safetensors;
use super::types::{LoadedModel, ModelFormat};
use crate::error::{VortexError, VortexResult};
use crate::model::Architecture;
use candle_core::{DType, Device};
use std::path::Path;
use tracing::{info, instrument};

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
    let format = ModelFormat::detect_from_directory(model_path).ok_or_else(|| {
        VortexError::LoadFailed(format!(
            "Could not detect model format in {}",
            model_path.display()
        ))
    })?;

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
