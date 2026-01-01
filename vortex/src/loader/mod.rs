//! Model loading infrastructure for Vortex.
//!
//! This module handles:
//! - Parsing model configuration (config.json)
//! - Device selection (CPU/CUDA/Metal)
//! - Loading model weights from SafeTensors/GGUF formats
//! - Creating Candle model instances
//! - Downloading models from `HuggingFace` Hub

mod config;
mod device;
mod hub;
mod weights;

pub use config::{ModelConfig, parse_model_config};
pub use device::{create_device, DeviceSpec};
pub use hub::{
    download_model, download_preset, get_cached_model, is_preset_cached, ModelPreset,
};
pub use weights::{LoadedModel, load_model_weights};
