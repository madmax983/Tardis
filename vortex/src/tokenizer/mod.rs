//! Tokenization services for Vortex.

use crate::error::{VortexError, VortexResult};
use crate::model::ModelHandle;
use std::collections::HashMap;
use std::path::Path;
use std::sync::RwLock;
use tokenizers::Tokenizer;

/// Service for managing tokenizers.
pub struct TokenizerService {
    /// Tokenizers by model handle.
    tokenizers: RwLock<HashMap<ModelHandle, Tokenizer>>,
}

impl std::fmt::Debug for TokenizerService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let count = self
            .tokenizers
            .read()
            .map(|t| t.len())
            .unwrap_or(0);
        f.debug_struct("TokenizerService")
            .field("loaded_count", &count)
            .finish()
    }
}

impl TokenizerService {
    /// Create a new tokenizer service.
    #[must_use]
    pub fn new() -> Self {
        Self {
            tokenizers: RwLock::new(HashMap::new()),
        }
    }

    /// Load a tokenizer for a model.
    ///
    /// # Errors
    ///
    /// Returns an error if the tokenizer cannot be loaded.
    pub fn load(&self, handle: ModelHandle, tokenizer_path: &Path) -> VortexResult<()> {
        let tokenizer = Tokenizer::from_file(tokenizer_path).map_err(|e| {
            VortexError::TokenizationError(format!("failed to load tokenizer: {e}"))
        })?;

        let mut tokenizers = self.tokenizers.write().map_err(|_| {
            VortexError::TokenizationError("failed to acquire tokenizer lock".to_string())
        })?;

        tokenizers.insert(handle, tokenizer);
        Ok(())
    }

    /// Unload a tokenizer.
    ///
    /// # Errors
    ///
    /// Returns an error if the tokenizer lock is poisoned.
    pub fn unload(&self, handle: ModelHandle) -> VortexResult<()> {
        let mut tokenizers = self.tokenizers.write().map_err(|_| {
            VortexError::TokenizationError("failed to acquire tokenizer lock".to_string())
        })?;

        tokenizers.remove(&handle);
        Ok(())
    }

    /// Encode text to tokens.
    ///
    /// # Errors
    ///
    /// Returns an error if encoding fails.
    pub fn encode(&self, handle: ModelHandle, text: &str) -> VortexResult<Vec<u32>> {
        let tokenizers = self.tokenizers.read().map_err(|_| {
            VortexError::TokenizationError("failed to acquire tokenizer lock".to_string())
        })?;

        let tokenizer = tokenizers.get(&handle).ok_or_else(|| {
            VortexError::TokenizationError(format!("no tokenizer for handle {handle}"))
        })?;

        let encoding = tokenizer
            .encode(text, true)
            .map_err(|e| VortexError::TokenizationError(format!("encoding failed: {e}")))?;

        Ok(encoding.get_ids().to_vec())
    }

    /// Decode tokens to text.
    ///
    /// # Errors
    ///
    /// Returns an error if decoding fails.
    pub fn decode(&self, handle: ModelHandle, tokens: &[u32]) -> VortexResult<String> {
        let tokenizers = self.tokenizers.read().map_err(|_| {
            VortexError::TokenizationError("failed to acquire tokenizer lock".to_string())
        })?;

        let tokenizer = tokenizers.get(&handle).ok_or_else(|| {
            VortexError::TokenizationError(format!("no tokenizer for handle {handle}"))
        })?;

        tokenizer
            .decode(tokens, true)
            .map_err(|e| VortexError::TokenizationError(format!("decoding failed: {e}")))
    }

    /// Get vocabulary size.
    pub fn vocab_size(&self, handle: ModelHandle) -> Option<usize> {
        self.tokenizers
            .read()
            .ok()?
            .get(&handle)
            .map(|t| t.get_vocab_size(true))
    }
}

impl Default for TokenizerService {
    fn default() -> Self {
        Self::new()
    }
}
