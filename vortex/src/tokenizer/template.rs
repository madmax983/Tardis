//! Chat template application logic.

use super::types::{ChatMessage, ChatRole};

/// Apply a Jinja-like chat template.
///
/// This is a simplified implementation that handles common patterns.
pub fn apply_jinja_template(
    template: &str,
    messages: &[ChatMessage],
    add_generation_prompt: bool,
) -> String {
    // Check for common template patterns and use appropriate formatter
    if template.contains("<|im_start|>") {
        // `ChatML` format (used by many models)
        apply_chatml_template(messages, add_generation_prompt)
    } else if template.contains("[INST]") {
        // Llama 2 format
        apply_llama2_template(messages, add_generation_prompt)
    } else if template.contains("<|start_header_id|>") {
        // Llama 3 format
        apply_llama3_template(messages, add_generation_prompt)
    } else {
        // Default to simple format
        apply_simple_template(messages, add_generation_prompt)
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
///
/// Note: Only the first system message is used; subsequent system messages
/// are ignored per Llama 2 chat format conventions.
fn apply_llama2_template(messages: &[ChatMessage], add_generation_prompt: bool) -> String {
    let mut result = String::new();
    let mut system_msg = None;

    // Extract first system message if present (Llama 2 only uses one)
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
                // If there's a pending system message, flush it before assistant response
                if let Some(sys) = system_msg.take() {
                    result.push_str("[INST] <<SYS>>\n");
                    result.push_str(sys);
                    result.push_str("\n<</SYS>>\n\n [/INST]");
                }

                result.push(' ');
                result.push_str(&msg.content);
                result.push_str(" </s><s>");
            }
        }
    }

    // Flush pending system message if no User/Assistant message consumed it
    if let Some(sys) = system_msg {
        result.push_str("[INST] <<SYS>>\n");
        result.push_str(sys);
        result.push_str("\n<</SYS>>\n\n [/INST]");
    }

    // Add space after [/INST] for assistant to generate
    if add_generation_prompt && result.ends_with(" [/INST]") {
        result.push(' ');
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
pub fn apply_simple_template(messages: &[ChatMessage], add_generation_prompt: bool) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_chatml_template() {
        let messages = vec![
            ChatMessage::system("You are helpful"),
            ChatMessage::user("Hello"),
        ];

        let result = apply_chatml_template(&messages, true);
        assert!(result.contains("<|im_start|>system"));
        assert!(result.contains("You are helpful"));
        assert!(result.contains("<|im_start|>user"));
        assert!(result.contains("Hello"));
        assert!(result.ends_with("<|im_start|>assistant\n"));
    }

    #[test]
    fn test_apply_llama3_template() {
        let messages = vec![ChatMessage::user("What is 2+2?")];

        let result = apply_llama3_template(&messages, true);
        assert!(result.starts_with("<|begin_of_text|>"));
        assert!(result.contains("<|start_header_id|>user<|end_header_id|>"));
        assert!(result.contains("What is 2+2?"));
        assert!(result.ends_with("<|start_header_id|>assistant<|end_header_id|>\n\n"));
    }

    #[test]
    fn test_apply_simple_template() {
        let messages = vec![ChatMessage::user("Hello"), ChatMessage::assistant("Hi!")];

        let result = apply_simple_template(&messages, true);
        assert!(result.contains("user: Hello"));
        assert!(result.contains("assistant: Hi!"));
        assert!(result.ends_with("assistant: "));
    }

    #[test]
    fn test_apply_simple_template_no_generation_prompt() {
        let messages = vec![ChatMessage::user("Hello")];

        let result = apply_simple_template(&messages, false);
        assert!(!result.ends_with("assistant: "));
        assert!(result.ends_with("Hello\n"));
    }

    #[test]
    fn test_apply_llama2_template_with_generation_prompt() {
        let messages = vec![
            ChatMessage::system("You are helpful"),
            ChatMessage::user("Hello"),
        ];

        let result = apply_llama2_template(&messages, true);
        assert!(result.contains("[INST]"));
        assert!(result.contains("<<SYS>>"));
        assert!(result.contains("You are helpful"));
        assert!(result.contains("Hello"));
        // Should end with space after [/INST] for generation
        assert!(result.ends_with(" [/INST] "));
    }

    #[test]
    fn test_apply_llama2_template_multi_turn() {
        let messages = vec![
            ChatMessage::user("Hi"),
            ChatMessage::assistant("Hello!"),
            ChatMessage::user("How are you?"),
        ];

        let result = apply_llama2_template(&messages, true);
        assert!(result.contains("[INST] Hi [/INST]"));
        assert!(result.contains(" Hello! </s><s>"));
        assert!(result.contains("[INST] How are you? [/INST]"));
        assert!(result.ends_with(" [/INST] "));
    }

    #[test]
    fn test_apply_llama2_template_system_only() {
        let messages = vec![ChatMessage::system("You are a poet")];

        let result = apply_llama2_template(&messages, true);
        assert!(
            result.contains("You are a poet"),
            "Should contain system message"
        );
        assert!(result.starts_with("[INST]"), "Should start with [INST]");
        assert!(
            result.ends_with(" [/INST] "),
            "Should end with space for generation"
        );
    }

    #[test]
    fn test_apply_llama2_template_system_assistant() {
        let messages = vec![ChatMessage::system("Sys"), ChatMessage::assistant("Hi")];

        let result = apply_llama2_template(&messages, true);
        assert!(result.contains("Sys"), "Should contain system message");
        assert!(result.contains("Hi"), "Should contain assistant message");
        // Structure: [INST] <<SYS>>\nSys\n<</SYS>>\n\n [/INST] Hi </s><s>
        assert!(result.contains("[INST]"), "Should contain INST block");
        assert!(result.contains(" [/INST]"), "Should contain closing INST");
        assert!(
            result.ends_with(" Hi </s><s>"),
            "Should end with assistant response"
        );
    }
}

#[cfg(test)]
mod sentry_tests {
    use super::*;

    #[test]
    fn test_apply_jinja_template_dispatch() {
        let messages = vec![ChatMessage::user("Hi")];

        // ChatML pattern
        let res_chatml = apply_jinja_template("... <|im_start|> ...", &messages, true);
        assert!(
            res_chatml.contains("<|im_start|>user"),
            "Should dispatch to ChatML"
        );

        // Llama 2 pattern
        let res_llama2 = apply_jinja_template("... [INST] ...", &messages, true);
        assert!(res_llama2.contains("[INST]"), "Should dispatch to Llama 2");

        // Llama 3 pattern
        let res_llama3 = apply_jinja_template("... <|start_header_id|> ...", &messages, true);
        assert!(
            res_llama3.contains("<|start_header_id|>"),
            "Should dispatch to Llama 3"
        );

        // Fallback
        let res_simple = apply_jinja_template("unknown template", &messages, true);
        assert!(res_simple.contains("user: Hi"), "Should fallback to simple");
    }

    #[test]
    fn test_apply_llama2_template_multiple_systems() {
        let messages = vec![
            ChatMessage::system("Sys1"),
            ChatMessage::user("Hi"),
            ChatMessage::system("Sys2"), // Should be ignored
            ChatMessage::user("Bye"),
        ];

        let result = apply_llama2_template(&messages, true);

        assert!(
            result.contains("Sys1"),
            "Should contain first system message"
        );
        assert!(
            !result.contains("Sys2"),
            "Should ignore subsequent system messages"
        );
    }

    #[test]
    fn test_apply_llama2_template_system_only_no_user() {
        // This is similar to existing test_apply_llama2_template_system_only but verifies logic explicitly
        let messages = vec![ChatMessage::system("Sys1")];
        let result = apply_llama2_template(&messages, true);

        assert!(result.contains("Sys1"));
        assert!(result.contains("<<SYS>>"));
        assert!(
            result.ends_with(" [/INST] "),
            "Should end ready for assistant generation"
        );
    }
}
