//! Psychic Paper: The Universal Data Interpreter.
//!
//! "It shows you exactly what you want to see."

use crate::error::ChronosResult;
use serde_json::{json, Value};

/// A universal interpreter for unstructured data.
#[derive(Debug, Default)]
pub struct PsychicPaper;

impl PsychicPaper {
    /// Create a new instance of Psychic Paper.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Attempt to interpret the input string based on an optional intent.
    ///
    /// # Errors
    ///
    /// Returns an error if interpretation fails completely.
    pub fn read(&self, input: &str, intent: Option<&str>) -> ChronosResult<Value> {
        let intent = intent.unwrap_or("auto");

        match intent {
            "json" => serde_json::from_str(input).map_err(|e| {
                crate::error::ChronosError::Common(tardis_common::Error::Internal(e.to_string()))
            }),
            "list" => {
                let items: Vec<&str> = input
                    .lines()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .collect();
                Ok(json!(items))
            }
            "kv" => Ok(Self::parse_kv(input)),
            "auto" => Ok(Self::auto_detect(input)),
            _ => Ok(json!({
                "type": "text",
                "content": input
            })),
        }
    }

    fn parse_kv(input: &str) -> Value {
        let mut map = serde_json::Map::new();
        for line in input.lines() {
            if let Some((k, v)) = line.split_once('=') {
                map.insert(k.trim().to_string(), Value::String(v.trim().to_string()));
            }
        }
        Value::Object(map)
    }

    fn auto_detect(input: &str) -> Value {
        // 1. Try JSON
        if let Ok(val) = serde_json::from_str::<Value>(input) {
            return val;
        }

        // 2. Fallback
        json!({
            "type": "text",
            "content": input
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_json_explicit() {
        let paper = PsychicPaper::new();
        let input = r#"{"name": "Rose", "species": "Human"}"#;
        let result = paper.read(input, Some("json")).unwrap();

        assert_eq!(result["name"], "Rose");
        assert_eq!(result["species"], "Human");
    }

    #[test]
    fn test_read_json_implicit() {
        let paper = PsychicPaper::new();
        let input = r#"{"name": "Rose", "species": "Human"}"#;
        let result = paper.read(input, None).unwrap();

        assert_eq!(result["name"], "Rose");
    }

    #[test]
    fn test_read_list() {
        let paper = PsychicPaper::new();
        let input = "Apples\nBananas\nPears";
        let result = paper.read(input, Some("list")).unwrap();

        assert!(result.is_array());
        assert_eq!(result[0], "Apples");
        assert_eq!(result[1], "Bananas");
        assert_eq!(result[2], "Pears");
    }

    #[test]
    fn test_read_kv() {
        let paper = PsychicPaper::new();
        let input = "name=Doctor\nage=900";
        let result = paper.read(input, Some("kv")).unwrap();

        assert_eq!(result["name"], "Doctor");
        assert_eq!(result["age"], "900");
    }

    #[test]
    fn test_read_unknown_text() {
        let paper = PsychicPaper::new();
        let input = "Just some text.";
        // Should default to a wrapped string or text object
        let result = paper.read(input, None).unwrap();

        assert_eq!(result["content"], "Just some text.");
        assert_eq!(result["type"], "text");
    }
}
