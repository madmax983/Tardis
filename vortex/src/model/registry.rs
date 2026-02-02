//! Model registry for tracking available and loaded models.

use super::{Architecture, Quantization};
use crate::error::{VortexError, VortexResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::RwLock;
use std::sync::atomic::{AtomicU64, Ordering};

/// A handle to a loaded model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ModelHandle(u64);

impl ModelHandle {
    /// Create a new model handle from a raw value.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Get the raw handle value.
    #[must_use]
    pub const fn raw(&self) -> u64 {
        self.0
    }
}

impl std::fmt::Display for ModelHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "model:{}", self.0)
    }
}

/// Information about a model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    /// Model name.
    pub name: String,
    /// Path to model files.
    pub path: PathBuf,
    /// Model architecture.
    pub architecture: Architecture,
    /// Number of parameters.
    pub parameters: u64,
    /// Maximum context length.
    pub context_length: usize,
    /// Quantization type.
    pub quantization: Quantization,
    /// Whether the model is currently loaded.
    pub loaded: bool,
    /// Memory usage in bytes (if loaded).
    pub memory_bytes: Option<u64>,
    /// Number of layers.
    pub num_layers: usize,
    /// Hidden size.
    pub hidden_size: usize,
    /// Number of attention heads.
    pub num_heads: usize,
    /// Vocabulary size.
    pub vocab_size: usize,
}

/// Registry for managing available and loaded models.
pub struct ModelRegistry {
    /// Next handle to assign.
    next_handle: AtomicU64,
    /// Registered models by path.
    models: RwLock<HashMap<PathBuf, ModelInfo>>,
    /// Loaded models by handle.
    loaded: RwLock<HashMap<ModelHandle, PathBuf>>,
}

impl std::fmt::Debug for ModelRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let registered = self.models.read().map(|m| m.len()).unwrap_or(0);
        let loaded = self.loaded.read().map(|l| l.len()).unwrap_or(0);
        f.debug_struct("ModelRegistry")
            .field("next_handle", &self.next_handle.load(Ordering::SeqCst))
            .field("registered_count", &registered)
            .field("loaded_count", &loaded)
            .finish()
    }
}

impl ModelRegistry {
    /// Create a new model registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            next_handle: AtomicU64::new(1),
            models: RwLock::new(HashMap::new()),
            loaded: RwLock::new(HashMap::new()),
        }
    }

    /// Register a model from its path.
    ///
    /// # Errors
    ///
    /// Returns an error if the model metadata cannot be read.
    pub fn register(&self, path: PathBuf, info: ModelInfo) -> VortexResult<()> {
        let mut models = self
            .models
            .write()
            .map_err(|_| VortexError::ConfigError("failed to acquire registry lock".to_string()))?;

        models.insert(path, info);
        Ok(())
    }

    /// Get information about a registered model.
    pub fn get_info(&self, path: &PathBuf) -> Option<ModelInfo> {
        self.models.read().ok()?.get(path).cloned()
    }

    /// Mark a model as loaded and return a handle.
    ///
    /// # Errors
    ///
    /// Returns an error if the registry lock is poisoned.
    pub fn mark_loaded(&self, path: &PathBuf, memory_bytes: u64) -> VortexResult<ModelHandle> {
        let handle = ModelHandle(self.next_handle.fetch_add(1, Ordering::SeqCst));

        // Update model info
        {
            let mut models = self.models.write().map_err(|_| {
                VortexError::ConfigError("failed to acquire registry lock".to_string())
            })?;

            if let Some(info) = models.get_mut(path) {
                info.loaded = true;
                info.memory_bytes = Some(memory_bytes);
            }
        }

        // Track loaded model
        {
            let mut loaded = self.loaded.write().map_err(|_| {
                VortexError::ConfigError("failed to acquire loaded lock".to_string())
            })?;

            loaded.insert(handle, path.clone());
        }

        Ok(handle)
    }

    /// Mark a model as unloaded.
    ///
    /// # Errors
    ///
    /// Returns an error if the registry lock is poisoned.
    pub fn mark_unloaded(&self, handle: ModelHandle) -> VortexResult<()> {
        let path = {
            let mut loaded = self.loaded.write().map_err(|_| {
                VortexError::ConfigError("failed to acquire loaded lock".to_string())
            })?;

            loaded.remove(&handle)
        };

        if let Some(path) = path {
            let mut models = self.models.write().map_err(|_| {
                VortexError::ConfigError("failed to acquire registry lock".to_string())
            })?;

            if let Some(info) = models.get_mut(&path) {
                info.loaded = false;
                info.memory_bytes = None;
            }
        }

        Ok(())
    }

    /// Get the path for a loaded model handle.
    pub fn get_path(&self, handle: ModelHandle) -> Option<PathBuf> {
        self.loaded.read().ok()?.get(&handle).cloned()
    }

    /// Check if a handle is valid.
    pub fn is_valid(&self, handle: ModelHandle) -> bool {
        self.loaded
            .read()
            .map(|l| l.contains_key(&handle))
            .unwrap_or(false)
    }

    /// List all registered models.
    pub fn list(&self) -> Vec<ModelInfo> {
        self.models
            .read()
            .map(|m| m.values().cloned().collect())
            .unwrap_or_default()
    }

    /// List loaded models.
    pub fn list_loaded(&self) -> Vec<(ModelHandle, ModelInfo)> {
        let Ok(loaded) = self.loaded.read() else {
            return Vec::new();
        };

        let Ok(models) = self.models.read() else {
            return Vec::new();
        };

        loaded
            .iter()
            .filter_map(|(handle, path)| models.get(path).map(|info| (*handle, info.clone())))
            .collect()
    }
}

impl Default for ModelRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_lifecycle() {
        let registry = ModelRegistry::new();

        let path = PathBuf::from("/models/test.safetensors");
        let info = ModelInfo {
            name: "test".to_string(),
            path: path.clone(),
            architecture: Architecture::Llama,
            parameters: 7_000_000_000,
            context_length: 4096,
            quantization: Quantization::F16,
            loaded: false,
            memory_bytes: None,
            num_layers: 32,
            hidden_size: 4096,
            num_heads: 32,
            vocab_size: 32000,
        };

        // Register
        registry.register(path.clone(), info).unwrap();

        // Check registered
        let retrieved = registry.get_info(&path).unwrap();
        assert_eq!(retrieved.name, "test");
        assert!(!retrieved.loaded);

        // Mark loaded
        let handle = registry.mark_loaded(&path, 14_000_000_000).unwrap();
        assert!(registry.is_valid(handle));

        // Check loaded
        let retrieved = registry.get_info(&path).unwrap();
        assert!(retrieved.loaded);
        assert_eq!(retrieved.memory_bytes, Some(14_000_000_000));

        // Unload
        registry.mark_unloaded(handle).unwrap();
        assert!(!registry.is_valid(handle));
    }
}
