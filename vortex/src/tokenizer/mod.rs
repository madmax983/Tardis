//! Tokenization services for Vortex.
//!
//! This module provides tokenization support for LLM inference, including:
//! - Loading `HuggingFace` tokenizer.json files
//! - Encoding text to token IDs
//! - Decoding token IDs back to text
//! - Special token handling (BOS, EOS, PAD)
//! - Chat template support for conversation formatting

pub(crate) mod template;
mod types;

pub use types::*;

use crate::error::{VortexError, VortexResult};
use crate::model::ModelHandle;
use std::collections::HashMap;
use std::path::Path;
use std::sync::RwLock;
use tokenizers::Tokenizer;
use tracing::info;

/// Loaded tokenizer with metadata.
struct LoadedTokenizer {
    /// The tokenizer itself.
    tokenizer: Tokenizer,
    /// Special token IDs.
    special_tokens: SpecialTokens,
    /// Chat template (if available).
    chat_template: Option<String>,
}

/// Service for managing tokenizers.
pub struct TokenizerService {
    /// Tokenizers by model handle.
    tokenizers: RwLock<HashMap<ModelHandle, LoadedTokenizer>>,
}

impl std::fmt::Debug for TokenizerService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let count = self.tokenizers.read().map(|t| t.len()).unwrap_or(0);
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
    /// This loads the tokenizer from `tokenizer.json` and optionally reads
    /// special tokens and chat template from `tokenizer_config.json`.
    ///
    /// # Errors
    ///
    /// Returns an error if the tokenizer cannot be loaded.
    pub fn load(&self, handle: ModelHandle, tokenizer_path: &Path) -> VortexResult<()> {
        let tokenizer = Tokenizer::from_file(tokenizer_path).map_err(|e| {
            VortexError::TokenizationError(format!("failed to load tokenizer: {e}"))
        })?;

        // Try to load tokenizer_config.json for special tokens and chat template
        let config_path = tokenizer_path.with_file_name("tokenizer_config.json");
        let (special_tokens, chat_template) = if config_path.exists() {
            Self::load_tokenizer_config(&config_path)?
        } else {
            // Fall back to extracting from tokenizer
            (Self::extract_special_tokens(&tokenizer), None)
        };

        info!(
            "Loaded tokenizer: vocab_size={}, bos={:?}, eos={:?}, chat_template={}",
            tokenizer.get_vocab_size(true),
            special_tokens.bos_token_id,
            special_tokens.eos_token_id,
            chat_template.is_some()
        );

        let loaded = LoadedTokenizer {
            tokenizer,
            special_tokens,
            chat_template,
        };

        let mut tokenizers = self
            .tokenizers
            .write()
            .map_err(|_| VortexError::LockPoisoned {
                context: "tokenizer write lock",
            })?;

        tokenizers.insert(handle, loaded);
        Ok(())
    }

    /// Load special tokens and chat template from `tokenizer_config.json`.
    fn load_tokenizer_config(config_path: &Path) -> VortexResult<(SpecialTokens, Option<String>)> {
        let config_str = std::fs::read_to_string(config_path).map_err(|e| {
            VortexError::TokenizationError(format!("failed to read tokenizer config: {e}"))
        })?;

        let config: serde_json::Value = serde_json::from_str(&config_str).map_err(|e| {
            VortexError::TokenizationError(format!("failed to parse tokenizer config: {e}"))
        })?;

        // Extract special token IDs
        let get_token_id = |key| {
            config
                .get(key)
                .and_then(serde_json::Value::as_u64)
                .and_then(|v| u32::try_from(v).ok())
        };

        let special_tokens = SpecialTokens {
            bos_token_id: get_token_id("bos_token_id"),
            eos_token_id: get_token_id("eos_token_id"),
            pad_token_id: get_token_id("pad_token_id"),
            unk_token_id: get_token_id("unk_token_id"),
        };

        // Extract chat template
        let chat_template = config
            .get("chat_template")
            .and_then(|v| v.as_str())
            .map(String::from);

        Ok((special_tokens, chat_template))
    }

    /// Extract special tokens from the tokenizer itself.
    fn extract_special_tokens(tokenizer: &Tokenizer) -> SpecialTokens {
        let vocab = tokenizer.get_vocab(true);

        SpecialTokens {
            bos_token_id: vocab
                .get("<s>")
                .or_else(|| vocab.get("<|begin_of_text|>"))
                .copied(),
            eos_token_id: vocab
                .get("</s>")
                .or_else(|| vocab.get("<|end_of_text|>"))
                .copied(),
            pad_token_id: vocab
                .get("<pad>")
                .or_else(|| vocab.get("<|padding|>"))
                .copied(),
            unk_token_id: vocab.get("<unk>").copied(),
        }
    }

    /// Unload a tokenizer.
    ///
    /// # Errors
    ///
    /// Returns an error if the tokenizer lock is poisoned.
    pub fn unload(&self, handle: ModelHandle) -> VortexResult<()> {
        let mut tokenizers = self
            .tokenizers
            .write()
            .map_err(|_| VortexError::LockPoisoned {
                context: "tokenizer write lock",
            })?;

        tokenizers.remove(&handle);
        Ok(())
    }

    /// Encode text to tokens.
    ///
    /// # Arguments
    ///
    /// * `handle` - Model handle
    /// * `text` - Text to encode
    ///
    /// # Errors
    ///
    /// Returns an error if encoding fails.
    pub fn encode(&self, handle: ModelHandle, text: &str) -> VortexResult<Vec<u32>> {
        let tokenizers = self
            .tokenizers
            .read()
            .map_err(|_| VortexError::LockPoisoned {
                context: "tokenizer read lock",
            })?;

        let loaded = tokenizers.get(&handle).ok_or_else(|| {
            VortexError::TokenizationError(format!("no tokenizer for handle {handle}"))
        })?;

        let encoding = loaded
            .tokenizer
            .encode(text, true)
            .map_err(|e| VortexError::TokenizationError(format!("encoding failed: {e}")))?;

        Ok(encoding.get_ids().to_vec())
    }

    /// Encode text, ensuring a BOS token is present at the start.
    ///
    /// This method calls [`Self::encode`] and then ensures the BOS token is at the
    /// start of the sequence. If the tokenizer already added a BOS token
    /// (based on its configuration), no duplicate is added.
    ///
    /// Use this when you need to guarantee a BOS token regardless of the
    /// tokenizer's default configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if encoding fails.
    pub fn encode_with_bos(&self, handle: ModelHandle, text: &str) -> VortexResult<Vec<u32>> {
        let mut tokens = self.encode(handle, text)?;

        if let Some(bos) = self.get_special_tokens(handle).and_then(|s| s.bos_token_id) {
            // Only prepend if not already present (tokenizer may have added it)
            if tokens.first() != Some(&bos) {
                tokens.insert(0, bos);
            }
        }

        Ok(tokens)
    }

    /// Decode tokens to text.
    ///
    /// # Arguments
    ///
    /// * `handle` - Model handle
    /// * `tokens` - Token IDs to decode
    ///
    /// # Errors
    ///
    /// Returns an error if decoding fails.
    pub fn decode(&self, handle: ModelHandle, tokens: &[u32]) -> VortexResult<String> {
        let tokenizers = self
            .tokenizers
            .read()
            .map_err(|_| VortexError::LockPoisoned {
                context: "tokenizer read lock",
            })?;

        let loaded = tokenizers.get(&handle).ok_or_else(|| {
            VortexError::TokenizationError(format!("no tokenizer for handle {handle}"))
        })?;

        loaded
            .tokenizer
            .decode(tokens, true)
            .map_err(|e| VortexError::TokenizationError(format!("decoding failed: {e}")))
    }

    /// Decode tokens, including special tokens in the output.
    ///
    /// Unlike [`Self::decode`], this preserves special tokens (BOS, EOS, etc.) in the
    /// output string. Useful for debugging or when you need to see the raw
    /// token sequence.
    ///
    /// # Errors
    ///
    /// Returns an error if decoding fails.
    pub fn decode_with_special_tokens(
        &self,
        handle: ModelHandle,
        tokens: &[u32],
    ) -> VortexResult<String> {
        let tokenizers = self
            .tokenizers
            .read()
            .map_err(|_| VortexError::LockPoisoned {
                context: "tokenizer read lock",
            })?;

        let loaded = tokenizers.get(&handle).ok_or_else(|| {
            VortexError::TokenizationError(format!("no tokenizer for handle {handle}"))
        })?;

        loaded
            .tokenizer
            .decode(tokens, false) // skip_special_tokens = false: include special tokens
            .map_err(|e| VortexError::TokenizationError(format!("decoding failed: {e}")))
    }

    /// Get vocabulary size.
    #[must_use]
    pub fn vocab_size(&self, handle: ModelHandle) -> Option<usize> {
        self.tokenizers
            .read()
            .ok()?
            .get(&handle)
            .map(|t| t.tokenizer.get_vocab_size(true))
    }

    /// Get special tokens for a model.
    #[must_use]
    pub fn get_special_tokens(&self, handle: ModelHandle) -> Option<SpecialTokens> {
        self.tokenizers
            .read()
            .ok()?
            .get(&handle)
            .map(|t| t.special_tokens.clone())
    }

    /// Check if a model has a chat template.
    #[must_use]
    pub fn has_chat_template(&self, handle: ModelHandle) -> bool {
        self.tokenizers
            .read()
            .ok()
            .and_then(|t| t.get(&handle).map(|l| l.chat_template.is_some()))
            .unwrap_or(false)
    }

    /// Apply chat template to messages.
    ///
    /// This formats a conversation using the model's chat template (if available)
    /// or falls back to a simple format.
    ///
    /// # Arguments
    ///
    /// * `handle` - Model handle
    /// * `messages` - Chat messages to format
    /// * `add_generation_prompt` - Whether to add the assistant prompt prefix
    ///
    /// # Errors
    ///
    /// Returns an error if the tokenizer is not found.
    pub fn apply_chat_template(
        &self,
        handle: ModelHandle,
        messages: &[ChatMessage],
        add_generation_prompt: bool,
    ) -> VortexResult<String> {
        let tokenizers = self
            .tokenizers
            .read()
            .map_err(|_| VortexError::LockPoisoned {
                context: "tokenizer read lock",
            })?;

        let loaded = tokenizers.get(&handle).ok_or_else(|| {
            VortexError::TokenizationError(format!("no tokenizer for handle {handle}"))
        })?;

        if let Some(template) = &loaded.chat_template {
            // Use Jinja-like template (simplified implementation)
            Ok(template::apply_jinja_template(
                template,
                messages,
                add_generation_prompt,
            ))
        } else {
            // Fall back to simple `ChatML`-like format
            Ok(template::apply_simple_template(
                messages,
                add_generation_prompt,
            ))
        }
    }

    /// Check if a tokenizer is loaded for a handle.
    #[must_use]
    pub fn is_loaded(&self, handle: ModelHandle) -> bool {
        self.tokenizers
            .read()
            .ok()
            .is_some_and(|t| t.contains_key(&handle))
    }
}

impl Default for TokenizerService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenizer_service_new() {
        let service = TokenizerService::new();
        assert!(!service.is_loaded(ModelHandle::new(1)));
    }

    #[test]
    fn test_encode_without_tokenizer() {
        let service = TokenizerService::new();
        let result = service.encode(ModelHandle::new(999), "hello");
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_without_tokenizer() {
        let service = TokenizerService::new();
        let result = service.decode(ModelHandle::new(999), &[1, 2, 3]);
        assert!(result.is_err());
    }

    #[test]
    fn test_vocab_size_without_tokenizer() {
        let service = TokenizerService::new();
        assert!(service.vocab_size(ModelHandle::new(999)).is_none());
    }

    #[test]
    fn test_get_special_tokens_without_tokenizer() {
        let service = TokenizerService::new();
        assert!(service.get_special_tokens(ModelHandle::new(999)).is_none());
    }

    #[test]
    fn test_has_chat_template_without_tokenizer() {
        let service = TokenizerService::new();
        assert!(!service.has_chat_template(ModelHandle::new(999)));
    }

    #[test]
    fn test_apply_chat_template_without_tokenizer() {
        let service = TokenizerService::new();
        let messages = vec![ChatMessage::user("Hello")];
        let result = service.apply_chat_template(ModelHandle::new(999), &messages, true);
        assert!(result.is_err());
    }

    #[test]
    fn test_unload_nonexistent_tokenizer() {
        let service = TokenizerService::new();
        // Should not error when unloading non-existent tokenizer
        let result = service.unload(ModelHandle::new(999));
        assert!(result.is_ok());
    }

    #[test]
    fn test_decode_with_special_tokens_without_tokenizer() {
        let service = TokenizerService::new();
        let result = service.decode_with_special_tokens(ModelHandle::new(999), &[1, 2, 3]);
        assert!(result.is_err());
    }

    // --- Sentry Integration Tests ---

    fn create_dummy_tokenizer_files(dir: &Path) -> std::path::PathBuf {
        use tokenizers::models::wordlevel::WordLevel;
        use tokenizers::AddedToken;

        std::fs::create_dir_all(dir).expect("failed to create temp dir");

        // Create a simple WordLevel tokenizer
        // ID 0: <s>, 1: </s>, 2: <pad>, 3: <unk>
        // ID 4: hello, 5: world, 6: !
        let vocab = HashMap::from([
            ("<s>".to_string(), 0),
            ("</s>".to_string(), 1),
            ("<pad>".to_string(), 2),
            ("<unk>".to_string(), 3),
            ("hello".to_string(), 4),
            ("world".to_string(), 5),
            ("!".to_string(), 6),
        ]);

        let vocab_path = dir.join("vocab.json");
        let vocab_str = serde_json::to_string(&vocab).expect("failed to serialize vocab");
        std::fs::write(&vocab_path, vocab_str).expect("failed to write vocab file");

        let model = WordLevel::from_file(vocab_path.to_str().unwrap(), "<unk>".to_string())
            .expect("failed to build model");

        let mut tokenizer = tokenizers::Tokenizer::new(model);
        tokenizer.with_pre_tokenizer(Some(tokenizers::pre_tokenizers::whitespace::Whitespace::default()));

        tokenizer.add_special_tokens(&[
            AddedToken::from("<s>", true),
            AddedToken::from("</s>", true),
            AddedToken::from("<pad>", true),
            AddedToken::from("<unk>", true),
        ]);

        let tokenizer_path = dir.join("tokenizer.json");
        tokenizer
            .save(&tokenizer_path, true)
            .expect("failed to save tokenizer");

        // Create config
        let config = serde_json::json!({
            "bos_token_id": 0,
            "eos_token_id": 1,
            "pad_token_id": 2,
            "unk_token_id": 3,
            "chat_template": "TEST_TEMPLATE"
        });

        let config_path = dir.join("tokenizer_config.json");
        std::fs::write(&config_path, config.to_string()).expect("failed to save config");

        tokenizer_path
    }

    #[test]
    fn test_tokenizer_integration_flow() {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_dir = std::env::temp_dir().join(format!("tardis_test_{}_{}", std::process::id(), timestamp));
        let tokenizer_path = create_dummy_tokenizer_files(&temp_dir);
        let service = TokenizerService::new();
        let handle = ModelHandle::new(42);

        // 1. Test Load
        service
            .load(handle, &tokenizer_path)
            .expect("failed to load tokenizer");
        assert!(service.is_loaded(handle));

        // 2. Test Special Tokens
        let special = service.get_special_tokens(handle).unwrap();
        assert_eq!(special.bos_token_id, Some(0));
        assert_eq!(special.eos_token_id, Some(1));

        // 3. Test Vocab Size
        assert_eq!(service.vocab_size(handle), Some(7));

        // 4. Test Encode
        // Note: WordLevel splits by space.
        let encoded = service
            .encode(handle, "hello world !")
            .expect("failed to encode");
        assert_eq!(encoded, vec![4, 5, 6]);

        // 5. Test Encode with BOS
        // Case A: BOS missing
        let with_bos = service
            .encode_with_bos(handle, "hello world !")
            .expect("failed to encode with bos");
        assert_eq!(with_bos, vec![0, 4, 5, 6]);

        // Case B: BOS already present in input
        // If we manually include the BOS token in the input string, the tokenizer
        // (with special tokens enabled) should encode it as BOS.
        // Then encode_with_bos should NOT add another one.
        let with_bos_manual = service
            .encode_with_bos(handle, "<s> hello")
            .expect("failed to encode with manual bos");
        assert_eq!(with_bos_manual, vec![0, 4]);

        // 6. Test Decode
        let decoded = service
            .decode(handle, &[4, 5, 6])
            .expect("failed to decode");
        assert_eq!(decoded, "hello world !");

        // 7. Test Decode with Special Tokens
        let decoded_special = service
            .decode_with_special_tokens(handle, &[0, 4, 5, 1])
            .expect("failed to decode special");
        assert_eq!(decoded_special, "<s> hello world </s>");

        // 8. Test Chat Template
        assert!(service.has_chat_template(handle));
        // Our dummy template logic checks for "TEST_TEMPLATE" string?
        // Wait, `apply_jinja_template` checks for specific substrings like "<|im_start|>" etc.
        // If it doesn't match any known ones, it falls back to simple.
        // My config put "TEST_TEMPLATE". `apply_jinja_template` will likely fall through to default.
        // I should check `template.rs` again.
        // Ah, `apply_chat_template` calls `template::apply_jinja_template`.
        // `apply_jinja_template` checks `template.contains(...)`.
        // "TEST_TEMPLATE" won't match anything, so it calls `apply_simple_template`.

        let msgs = vec![ChatMessage::user("hello")];
        let prompt = service
            .apply_chat_template(handle, &msgs, false)
            .expect("failed to apply template");
        // Simple template: "user: hello\n"
        assert!(prompt.contains("user: hello"));

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
