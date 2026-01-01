//! Tokenization services for Vortex.
//!
//! This module provides tokenization support for LLM inference, including:
//! - Loading `HuggingFace` tokenizer.json files
//! - Encoding text to token IDs
//! - Decoding token IDs back to text
//! - Special token handling (BOS, EOS, PAD)
//! - Chat template support for conversation formatting

use crate::error::{VortexError, VortexResult};
use crate::model::ModelHandle;
use std::collections::HashMap;
use std::path::Path;
use std::sync::RwLock;
use tokenizers::Tokenizer;
use tracing::info;

/// Special token IDs for a model.
#[derive(Debug, Clone, Default)]
pub struct SpecialTokens {
    /// Beginning of sequence token ID.
    pub bos_token_id: Option<u32>,
    /// End of sequence token ID.
    pub eos_token_id: Option<u32>,
    /// Padding token ID.
    pub pad_token_id: Option<u32>,
    /// Unknown token ID.
    pub unk_token_id: Option<u32>,
}

/// Chat message role.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatRole {
    /// System message (instructions).
    System,
    /// User message.
    User,
    /// Assistant response.
    Assistant,
}

impl ChatRole {
    /// Get the role name as used in chat templates.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::System => "system",
            Self::User => "user",
            Self::Assistant => "assistant",
        }
    }
}

/// A chat message for template formatting.
#[derive(Debug, Clone)]
pub struct ChatMessage {
    /// The role of the message sender.
    pub role: ChatRole,
    /// The message content.
    pub content: String,
}

impl ChatMessage {
    /// Create a new chat message.
    #[must_use]
    pub fn new(role: ChatRole, content: impl Into<String>) -> Self {
        Self {
            role,
            content: content.into(),
        }
    }

    /// Create a system message.
    #[must_use]
    pub fn system(content: impl Into<String>) -> Self {
        Self::new(ChatRole::System, content)
    }

    /// Create a user message.
    #[must_use]
    pub fn user(content: impl Into<String>) -> Self {
        Self::new(ChatRole::User, content)
    }

    /// Create an assistant message.
    #[must_use]
    pub fn assistant(content: impl Into<String>) -> Self {
        Self::new(ChatRole::Assistant, content)
    }
}

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

        let mut tokenizers = self.tokenizers.write().map_err(|_| {
            VortexError::TokenizationError("failed to acquire tokenizer lock".to_string())
        })?;

        tokenizers.insert(handle, loaded);
        Ok(())
    }

    /// Load special tokens and chat template from `tokenizer_config.json`.
    fn load_tokenizer_config(
        config_path: &Path,
    ) -> VortexResult<(SpecialTokens, Option<String>)> {
        let config_str = std::fs::read_to_string(config_path).map_err(|e| {
            VortexError::TokenizationError(format!("failed to read tokenizer config: {e}"))
        })?;

        let config: serde_json::Value = serde_json::from_str(&config_str).map_err(|e| {
            VortexError::TokenizationError(format!("failed to parse tokenizer config: {e}"))
        })?;

        // Extract special token IDs
        let special_tokens = SpecialTokens {
            bos_token_id: config
                .get("bos_token_id")
                .and_then(serde_json::Value::as_u64)
                .and_then(|v| u32::try_from(v).ok()),
            eos_token_id: config
                .get("eos_token_id")
                .and_then(serde_json::Value::as_u64)
                .and_then(|v| u32::try_from(v).ok()),
            pad_token_id: config
                .get("pad_token_id")
                .and_then(serde_json::Value::as_u64)
                .and_then(|v| u32::try_from(v).ok()),
            unk_token_id: config
                .get("unk_token_id")
                .and_then(serde_json::Value::as_u64)
                .and_then(|v| u32::try_from(v).ok()),
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
            bos_token_id: vocab.get("<s>").or_else(|| vocab.get("<|begin_of_text|>")).copied(),
            eos_token_id: vocab.get("</s>").or_else(|| vocab.get("<|end_of_text|>")).copied(),
            pad_token_id: vocab.get("<pad>").or_else(|| vocab.get("<|padding|>")).copied(),
            unk_token_id: vocab.get("<unk>").copied(),
        }
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
    /// # Arguments
    ///
    /// * `handle` - Model handle
    /// * `text` - Text to encode
    ///
    /// # Errors
    ///
    /// Returns an error if encoding fails.
    pub fn encode(&self, handle: ModelHandle, text: &str) -> VortexResult<Vec<u32>> {
        let tokenizers = self.tokenizers.read().map_err(|_| {
            VortexError::TokenizationError("failed to acquire tokenizer lock".to_string())
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

    /// Encode text with optional BOS token prepended.
    ///
    /// # Errors
    ///
    /// Returns an error if encoding fails.
    pub fn encode_with_bos(&self, handle: ModelHandle, text: &str) -> VortexResult<Vec<u32>> {
        let mut tokens = self.encode(handle, text)?;

        if let Some(bos) = self.get_special_tokens(handle).and_then(|s| s.bos_token_id) {
            // Only prepend if not already present
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
        let tokenizers = self.tokenizers.read().map_err(|_| {
            VortexError::TokenizationError("failed to acquire tokenizer lock".to_string())
        })?;

        let loaded = tokenizers.get(&handle).ok_or_else(|| {
            VortexError::TokenizationError(format!("no tokenizer for handle {handle}"))
        })?;

        loaded
            .tokenizer
            .decode(tokens, true)
            .map_err(|e| VortexError::TokenizationError(format!("decoding failed: {e}")))
    }

    /// Decode tokens, skipping special tokens.
    ///
    /// # Errors
    ///
    /// Returns an error if decoding fails.
    pub fn decode_skip_special(&self, handle: ModelHandle, tokens: &[u32]) -> VortexResult<String> {
        let tokenizers = self.tokenizers.read().map_err(|_| {
            VortexError::TokenizationError("failed to acquire tokenizer lock".to_string())
        })?;

        let loaded = tokenizers.get(&handle).ok_or_else(|| {
            VortexError::TokenizationError(format!("no tokenizer for handle {handle}"))
        })?;

        loaded
            .tokenizer
            .decode(tokens, false) // skip_special_tokens = false means DON'T clean up
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
        let tokenizers = self.tokenizers.read().map_err(|_| {
            VortexError::TokenizationError("failed to acquire tokenizer lock".to_string())
        })?;

        let loaded = tokenizers.get(&handle).ok_or_else(|| {
            VortexError::TokenizationError(format!("no tokenizer for handle {handle}"))
        })?;

        if let Some(template) = &loaded.chat_template {
            // Use Jinja-like template (simplified implementation)
            Ok(Self::apply_jinja_template(template, messages, add_generation_prompt))
        } else {
            // Fall back to simple `ChatML`-like format
            Ok(Self::apply_simple_template(messages, add_generation_prompt))
        }
    }

    /// Apply a Jinja-like chat template.
    ///
    /// This is a simplified implementation that handles common patterns.
    fn apply_jinja_template(
        template: &str,
        messages: &[ChatMessage],
        add_generation_prompt: bool,
    ) -> String {
        // Check for common template patterns and use appropriate formatter
        if template.contains("<|im_start|>") {
            // `ChatML` format (used by many models)
            Self::apply_chatml_template(messages, add_generation_prompt)
        } else if template.contains("[INST]") {
            // Llama 2 format
            Self::apply_llama2_template(messages, add_generation_prompt)
        } else if template.contains("<|start_header_id|>") {
            // Llama 3 format
            Self::apply_llama3_template(messages, add_generation_prompt)
        } else {
            // Default to simple format
            Self::apply_simple_template(messages, add_generation_prompt)
        }
    }

    /// Apply `ChatML` template format.
    fn apply_chatml_template(messages: &[ChatMessage], add_generation_prompt: bool) -> String {
        let mut result = String::new();

        for msg in messages {
            result.push_str("<|im_start|>");
            result.push_str(msg.role.as_str());
            result.push('\n');
            result.push_str(&msg.content);
            result.push_str("<|im_end|>\n");
        }

        if add_generation_prompt {
            result.push_str("<|im_start|>assistant\n");
        }

        result
    }

    /// Apply Llama 2 template format.
    fn apply_llama2_template(messages: &[ChatMessage], add_generation_prompt: bool) -> String {
        let mut result = String::new();
        let mut system_msg = None;

        // Extract system message if present
        for msg in messages {
            if msg.role == ChatRole::System {
                system_msg = Some(&msg.content);
                break;
            }
        }

        for msg in messages {
            match msg.role {
                ChatRole::System => {
                    // System message is included with first user message
                }
                ChatRole::User => {
                    result.push_str("[INST] ");
                    if let Some(sys) = system_msg.take() {
                        result.push_str("<<SYS>>\n");
                        result.push_str(sys);
                        result.push_str("\n<</SYS>>\n\n");
                    }
                    result.push_str(&msg.content);
                    result.push_str(" [/INST]");
                }
                ChatRole::Assistant => {
                    result.push(' ');
                    result.push_str(&msg.content);
                    result.push_str(" </s><s>");
                }
            }
        }

        if add_generation_prompt && !result.ends_with("[/INST]") {
            // Already ends with prompt
        }

        result
    }

    /// Apply Llama 3 template format.
    fn apply_llama3_template(messages: &[ChatMessage], add_generation_prompt: bool) -> String {
        let mut result = String::from("<|begin_of_text|>");

        for msg in messages {
            result.push_str("<|start_header_id|>");
            result.push_str(msg.role.as_str());
            result.push_str("<|end_header_id|>\n\n");
            result.push_str(&msg.content);
            result.push_str("<|eot_id|>");
        }

        if add_generation_prompt {
            result.push_str("<|start_header_id|>assistant<|end_header_id|>\n\n");
        }

        result
    }

    /// Apply simple template format (fallback).
    fn apply_simple_template(messages: &[ChatMessage], add_generation_prompt: bool) -> String {
        let mut result = String::new();

        for msg in messages {
            result.push_str(msg.role.as_str());
            result.push_str(": ");
            result.push_str(&msg.content);
            result.push('\n');
        }

        if add_generation_prompt {
            result.push_str("assistant: ");
        }

        result
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
    fn test_chat_role_as_str() {
        assert_eq!(ChatRole::System.as_str(), "system");
        assert_eq!(ChatRole::User.as_str(), "user");
        assert_eq!(ChatRole::Assistant.as_str(), "assistant");
    }

    #[test]
    fn test_chat_message_constructors() {
        let system = ChatMessage::system("You are helpful");
        assert_eq!(system.role, ChatRole::System);
        assert_eq!(system.content, "You are helpful");

        let user = ChatMessage::user("Hello");
        assert_eq!(user.role, ChatRole::User);
        assert_eq!(user.content, "Hello");

        let assistant = ChatMessage::assistant("Hi there!");
        assert_eq!(assistant.role, ChatRole::Assistant);
        assert_eq!(assistant.content, "Hi there!");
    }

    #[test]
    fn test_apply_chatml_template() {
        let messages = vec![
            ChatMessage::system("You are helpful"),
            ChatMessage::user("Hello"),
        ];

        let result = TokenizerService::apply_chatml_template(&messages, true);
        assert!(result.contains("<|im_start|>system"));
        assert!(result.contains("You are helpful"));
        assert!(result.contains("<|im_start|>user"));
        assert!(result.contains("Hello"));
        assert!(result.ends_with("<|im_start|>assistant\n"));
    }

    #[test]
    fn test_apply_llama3_template() {
        let messages = vec![
            ChatMessage::user("What is 2+2?"),
        ];

        let result = TokenizerService::apply_llama3_template(&messages, true);
        assert!(result.starts_with("<|begin_of_text|>"));
        assert!(result.contains("<|start_header_id|>user<|end_header_id|>"));
        assert!(result.contains("What is 2+2?"));
        assert!(result.ends_with("<|start_header_id|>assistant<|end_header_id|>\n\n"));
    }

    #[test]
    fn test_apply_simple_template() {
        let messages = vec![
            ChatMessage::user("Hello"),
            ChatMessage::assistant("Hi!"),
        ];

        let result = TokenizerService::apply_simple_template(&messages, true);
        assert!(result.contains("user: Hello"));
        assert!(result.contains("assistant: Hi!"));
        assert!(result.ends_with("assistant: "));
    }

    #[test]
    fn test_apply_simple_template_no_generation_prompt() {
        let messages = vec![ChatMessage::user("Hello")];

        let result = TokenizerService::apply_simple_template(&messages, false);
        assert!(!result.ends_with("assistant: "));
        assert!(result.ends_with("Hello\n"));
    }

    #[test]
    fn test_special_tokens_default() {
        let tokens = SpecialTokens::default();
        assert!(tokens.bos_token_id.is_none());
        assert!(tokens.eos_token_id.is_none());
        assert!(tokens.pad_token_id.is_none());
        assert!(tokens.unk_token_id.is_none());
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
}
