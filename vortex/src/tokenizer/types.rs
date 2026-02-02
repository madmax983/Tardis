//! Tokenizer types.

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

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_special_tokens_default() {
        let tokens = SpecialTokens::default();
        assert!(tokens.bos_token_id.is_none());
        assert!(tokens.eos_token_id.is_none());
        assert!(tokens.pad_token_id.is_none());
        assert!(tokens.unk_token_id.is_none());
    }
}
