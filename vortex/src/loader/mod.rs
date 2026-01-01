//! Model loading infrastructure for Vortex.
//!
//! This module handles:
//! - Parsing model configuration (config.json)
//! - Device selection (CPU/CUDA/Metal)
//! - Loading model weights from SafeTensors/GGUF formats
//! - Creating Candle model instances

mod config;
mod device;
mod weights;

pub use config::{ModelConfig, parse_model_config};
pub use device::{create_device, DeviceSpec};
pub use weights::{LoadedModel, load_model_weights};
