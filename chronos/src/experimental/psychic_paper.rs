//! Psychic Paper: The Universal Interpreter.
//!
//! "It shows you what you want to see."
//!
//! This module provides robust parsing for unstructured text, designed to handle
//! the messy output of LLMs or user input.

use serde_json::{json, Value};

/// Maximum allowed input text length (1MB) to prevent `DoS`.
pub const MAX_TEXT_LEN: usize = 1024 * 1024;

/// Maximum number of items to parse in a list or map to prevent unbounded allocation.
pub const MAX_ITEMS: usize = 1000;

/// The intent of the interpretation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    /// Try to infer the format using a best-effort heuristic:
    /// 1. Check for JSON start characters `{` or `[`.
    /// 2. Check for Markdown code blocks (` ```json `).
    /// 3. Check for list markers (`-` or `*`).
    /// 4. Check for Key-Value pairs (majority of lines have `:`).
    /// 5. Fallback to raw string.
    Auto,
    /// Expect JSON (strips markdown code blocks).
    Json,
    /// Expect a list (bullets, numbered, or comma-separated).
    List,
    /// Expect Key-Value pairs (e.g., "Name: Doctor").
    KeyValue,
}

/// The Psychic Paper interpreter.
#[derive(Debug, Default)]
pub struct PsychicPaper;

#[allow(
    clippy::unused_self,
    clippy::unnecessary_wraps,
    clippy::redundant_closure_for_method_calls
)]
impl PsychicPaper {
    /// Create a new `PsychicPaper` instance.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Interpret text based on the given intent.
    ///
    /// # Examples
    ///
    /// ```
    /// use tardis_chronos::experimental::psychic_paper::{PsychicPaper, Intent};
    /// use serde_json::json;
    ///
    /// let paper = PsychicPaper::new();
    ///
    /// // A messy LLM response
    /// let text = "Here is the data:\n```json\n{\"answer\": 42}\n```";
    ///
    /// // Auto-detect extracts the JSON inside the code block
    /// let result = paper.interpret(text, Intent::Auto).unwrap();
    /// assert_eq!(result, json!({"answer": 42}));
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error string if parsing fails or input is too large.
    pub fn interpret(&self, text: &str, intent: Intent) -> Result<Value, String> {
        if text.len() > MAX_TEXT_LEN {
            return Err(format!(
                "Input text exceeds maximum length of {MAX_TEXT_LEN} bytes"
            ));
        }

        match intent {
            Intent::Auto => self.interpret_auto(text),
            Intent::Json => self.interpret_json(text),
            Intent::List => self.interpret_list(text),
            Intent::KeyValue => self.interpret_kv(text),
        }
    }

    fn interpret_auto(&self, text: &str) -> Result<Value, String> {
        // Simple heuristics
        let trimmed = text.trim();
        if trimmed.starts_with('{') || trimmed.starts_with('[') {
            // If it looks like JSON, try JSON first
            if let Ok(v) = self.interpret_json(text) {
                return Ok(v);
            }
        }

        // Check for Markdown code block with json
        if trimmed.contains("```json") {
            if let Ok(v) = self.interpret_json(text) {
                return Ok(v);
            }
        }

        if trimmed.contains('\n') && (trimmed.contains("- ") || trimmed.contains("* ")) {
            return self.interpret_list(text);
        }

        if trimmed.contains(':') {
            // Check if it looks like KV lines
            // Heuristic: majority of lines have ':'
            // Limit check to first 100 lines to avoid excessive processing on huge files
            let lines: Vec<&str> = trimmed
                .lines()
                .take(100)
                .filter(|l| !l.trim().is_empty())
                .collect();
            if !lines.is_empty() {
                let colon_count = lines.iter().filter(|l| l.contains(':')).count();
                if colon_count >= lines.len() / 2 {
                    return self.interpret_kv(text);
                }
            }
        }

        // Fallback: just a string
        Ok(Value::String(text.to_string()))
    }

    fn interpret_json(&self, text: &str) -> Result<Value, String> {
        // 1. Try direct parse
        if let Ok(v) = serde_json::from_str(text) {
            return Ok(v);
        }

        // 2. Try to find JSON block
        // Find the outermost braces or brackets
        let start_obj = text.find('{');
        let start_arr = text.find('[');

        let start = match (start_obj, start_arr) {
            (Some(o), Some(a)) => Some(o.min(a)),
            (Some(o), None) => Some(o),
            (None, Some(a)) => Some(a),
            (None, None) => None,
        };

        let end_obj = text.rfind('}');
        let end_arr = text.rfind(']');

        let end = match (end_obj, end_arr) {
            (Some(o), Some(a)) => Some(o.max(a)),
            (Some(o), None) => Some(o),
            (None, Some(a)) => Some(a),
            (None, None) => None,
        };

        if let (Some(s), Some(e)) = (start, end) {
            if s <= e {
                let candidate = &text[s..=e];
                if let Ok(v) = serde_json::from_str(candidate) {
                    return Ok(v);
                }
            }
        }

        Err("Could not find valid JSON".to_string())
    }

    fn interpret_list(&self, text: &str) -> Result<Value, String> {
        let items: Vec<String> = text
            .lines()
            .take(MAX_ITEMS)
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .map(|line| {
                // Strip bullets
                if let Some(stripped) = line.strip_prefix("- ") {
                    return stripped.to_string();
                }
                if let Some(stripped) = line.strip_prefix("* ") {
                    return stripped.to_string();
                }
                // Strip numbers "1. "
                if let Some(idx) = line.find(". ") {
                    if idx > 0 && line[..idx].chars().all(|c| c.is_numeric()) {
                        return line[idx + 2..].to_string();
                    }
                }
                line.to_string()
            })
            .collect();

        // Fallback: Try comma separation if single line and no bullets were found/stripped
        // "No bullets found" means we have exactly one item and it matches the original trimmed text.
        // Also respect MAX_ITEMS
        if items.len() == 1 && items[0] == text.trim() && !text.contains('\n') && text.contains(',')
        {
            let items: Vec<String> = text
                .split(',')
                .take(MAX_ITEMS)
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            return Ok(json!(items));
        }

        Ok(json!(items))
    }

    fn interpret_kv(&self, text: &str) -> Result<Value, String> {
        let mut map = serde_json::Map::new();

        for line in text.lines().take(MAX_ITEMS) {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim().to_string();
                let value = value.trim();

                // Try to parse value as JSON (number, boolean, null), else string
                let json_val = if let Ok(v) = serde_json::from_str(value) {
                    v
                } else {
                    Value::String(value.to_string())
                };

                map.insert(key, json_val);
            }
        }

        Ok(Value::Object(map))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_interpret_json_clean() {
        let text = r#"{"name": "Doctor", "age": 900}"#;
        let paper = PsychicPaper::new();
        let result = paper.interpret(text, Intent::Json).unwrap();
        assert_eq!(result, json!({"name": "Doctor", "age": 900}));
    }

    #[test]
    fn test_interpret_json_markdown() {
        let text = r#"Here is the JSON:
```json
{
  "name": "Master",
  "plan": "Conquest"
}
```
Hope that helps."#;
        let paper = PsychicPaper::new();
        let result = paper.interpret(text, Intent::Json).unwrap();
        assert_eq!(result, json!({"name": "Master", "plan": "Conquest"}));
    }

    #[test]
    fn test_interpret_list_bullets() {
        let text = "- Sonic Screwdriver\n- TARDIS Key\n- Psychic Paper";
        let paper = PsychicPaper::new();
        let result = paper.interpret(text, Intent::List).unwrap();
        assert_eq!(
            result,
            json!(["Sonic Screwdriver", "TARDIS Key", "Psychic Paper"])
        );
    }

    #[test]
    fn test_interpret_list_numbered() {
        let text = "1. Run\n2. Hide\n3. Save the world";
        let paper = PsychicPaper::new();
        let result = paper.interpret(text, Intent::List).unwrap();
        assert_eq!(result, json!(["Run", "Hide", "Save the world"]));
    }

    #[test]
    fn test_interpret_kv() {
        let text = "Species: Time Lord\nOrigin: Gallifrey\nRegenerations: 12";
        let paper = PsychicPaper::new();
        let result = paper.interpret(text, Intent::KeyValue).unwrap();
        assert_eq!(
            result,
            json!({
                "Species": "Time Lord",
                "Origin": "Gallifrey",
                "Regenerations": 12
            })
        );
    }

    #[test]
    fn test_interpret_auto_json() {
        let text = r#"{"type": "TARDIS"}"#;
        let paper = PsychicPaper::new();
        let result = paper.interpret(text, Intent::Auto).unwrap();
        assert_eq!(result, json!({"type": "TARDIS"}));
    }

    #[test]
    fn test_interpret_auto_list() {
        let text = "* Dalek\n* Cyberman";
        let paper = PsychicPaper::new();
        let result = paper.interpret(text, Intent::Auto).unwrap();
        assert_eq!(result, json!(["Dalek", "Cyberman"]));
    }

    #[test]
    fn test_interpret_auto_kv() {
        let text = "Enemy: Weeping Angel\nDon't: Blink";
        let paper = PsychicPaper::new();
        let result = paper.interpret(text, Intent::Auto).unwrap();
        assert_eq!(result, json!({"Enemy": "Weeping Angel", "Don't": "Blink"}));
    }

    #[test]
    fn test_interpret_list_comma_separated() {
        let text = "red, green, blue";
        let paper = PsychicPaper::new();
        let result = paper.interpret(text, Intent::List).unwrap();
        assert_eq!(result, json!(["red", "green", "blue"]));
    }

    #[test]
    fn test_interpret_max_len() {
        let paper = PsychicPaper::new();
        // Create 1MB + 1 byte string
        let text = "a".repeat(MAX_TEXT_LEN + 1);
        let result = paper.interpret(&text, Intent::Auto);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("exceeds maximum length"));
    }

    #[test]
    fn test_interpret_max_items_list() {
        let paper = PsychicPaper::new();
        // Create 1500 lines
        let mut text = String::new();
        for i in 0..1500 {
            text.push_str(&format!("- Item {}\n", i));
        }

        // This won't fail size check (1500 * ~10 bytes = 15KB)
        let result = paper.interpret(&text, Intent::List).unwrap();
        let array = result.as_array().unwrap();
        assert_eq!(array.len(), MAX_ITEMS);
    }
}
