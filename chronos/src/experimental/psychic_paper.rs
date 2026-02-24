//! Psychic Paper: The Universal Interpreter.
//!
//! "It shows you what you want to see."
//!
//! This module provides robust parsing for unstructured text, designed to handle
//! the messy output of LLMs or user input.

use serde_json::{json, Value};

/// Extract JSON from a string, handling markdown blocks and surrounding text.
///
/// # Errors
///
/// Returns an error if no valid JSON object or array can be found.
pub fn extract_json(text: &str) -> Result<Value, String> {
    // 1. Try direct parse
    if let Ok(v) = serde_json::from_str(text) {
        return Ok(v);
    }

    // 2. Try to find JSON block
    let start = text.find(['{', '[']);
    let end = text.rfind(['}', ']']);

    if let (Some(s), Some(e)) = (start, end) {
        if s <= e {
            if let Ok(v) = serde_json::from_str(&text[s..=e]) {
                return Ok(v);
            }
        }
    }

    Err("Could not find valid JSON".to_string())
}

/// Parse a list from text (bullets, numbered, or comma-separated).
///
/// # Errors
///
/// Returns an error if parsing fails (currently infallible, returns string array).
pub fn parse_list(text: &str) -> Result<Value, String> {
    let items: Vec<String> = text
        .lines()
        .map(str::trim)
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
                if idx > 0 && line[..idx].chars().all(char::is_numeric) {
                    return line[idx + 2..].to_string();
                }
            }
            line.to_string()
        })
        .collect();

    // Fallback: Try comma separation if single line and no bullets were found
    if items.len() == 1 && items[0] == text.trim() && !text.contains('\n') && text.contains(',') {
        let items: Vec<String> = text
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        return Ok(json!(items));
    }

    Ok(json!(items))
}

/// Parse Key-Value pairs (e.g., "Name: Doctor").
///
/// # Errors
///
/// Returns an error if parsing fails.
pub fn parse_kv(text: &str) -> Result<Value, String> {
    let mut map = serde_json::Map::new();

    for line in text.lines() {
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

/// Attempt to infer structure and parse it.
///
/// Tries JSON -> List -> `KeyValue` -> String.
///
/// # Errors
///
/// Returns an error if the structure cannot be repaired or fallback fails (unlikely).
pub fn repair_structure(text: &str) -> Result<Value, String> {
    if let Ok(v) = extract_json(text) {
        return Ok(v);
    }

    // Heuristic: If it has bullets or is a list, try list
    if text.contains("- ") || text.contains("* ") || (text.contains(',') && !text.contains('\n')) {
        if let Ok(v) = parse_list(text) {
            if let Some(arr) = v.as_array() {
                if !arr.is_empty() {
                    return Ok(v);
                }
            }
        }
    }

    // Heuristic: If it has colons, try KV
    if text.contains(':') {
        if let Ok(v) = parse_kv(text) {
            if let Some(obj) = v.as_object() {
                if !obj.is_empty() {
                    return Ok(v);
                }
            }
        }
    }

    // Fallback
    Ok(Value::String(text.to_string()))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_extract_json() {
        assert_eq!(extract_json(r#"{"a":1}"#).unwrap(), json!({"a": 1}));
        assert_eq!(
            extract_json(r#"Prefix {"a":1} Suffix"#).unwrap(),
            json!({"a": 1})
        );
        assert_eq!(
            extract_json(r#"```json {"a":1} ```"#).unwrap(),
            json!({"a": 1})
        );
        assert!(extract_json("Invalid").is_err());
    }

    #[test]
    fn test_parse_list() {
        assert_eq!(parse_list("- A\n- B").unwrap(), json!(["A", "B"]));
        assert_eq!(parse_list("1. A\n2. B").unwrap(), json!(["A", "B"]));
        assert_eq!(parse_list("A, B, C").unwrap(), json!(["A", "B", "C"]));
    }

    #[test]
    fn test_parse_kv() {
        assert_eq!(
            parse_kv("Name: Doctor\nAge: 900").unwrap(),
            json!({"Name": "Doctor", "Age": 900})
        );
    }
}
