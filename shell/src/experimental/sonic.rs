use std::path::Path;
use std::fs;
use serde_json::Value;
use tardis_chronos::experimental::psychic_paper::{PsychicPaper, Intent};

/// The Sonic Screwdriver: A universal file repair tool.
#[derive(Debug, Default)]
pub struct SonicScrewdriver;

impl SonicScrewdriver {
    /// Create a new Sonic Screwdriver.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Fix a file by attempting to parse and pretty-print it.
    ///
    /// # Errors
    ///
    /// Returns an error if file reading or parsing fails.
    pub fn fix<P: AsRef<Path>>(&self, path: P) -> Result<String, String> {
        let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;

        let paper = PsychicPaper::new();
        // Use Auto intent to handle messy inputs
        let value = paper.interpret(&content, Intent::Auto)?;

        // Pretty print the result
        serde_json::to_string_pretty(&value).map_err(|e| e.to_string())
    }

    /// Scan a file and report its structure.
    ///
    /// # Errors
    ///
    /// Returns an error if file reading or parsing fails.
    pub fn scan<P: AsRef<Path>>(&self, path: P) -> Result<String, String> {
        let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;

        let paper = PsychicPaper::new();
        let value = paper.interpret(&content, Intent::Auto)?;

        match value {
            Value::Object(map) => {
                let keys: Vec<&String> = map.keys().collect();
                Ok(format!("Found object with {} keys: {:?}", map.len(), keys))
            }
            Value::Array(arr) => {
                Ok(format!("Found array with {} items", arr.len()))
            }
            _ => Ok(format!("Found primitive value: {:?}", value)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn temp_file(name: &str, content: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!("tardis_test_{}_{}", name, std::process::id()));
        fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn test_sonic_fix_json() {
        let path = temp_file("fix_json", r#"{ "key": "value" }"#);
        let sonic = SonicScrewdriver::new();
        let result = sonic.fix(&path);
        let _ = fs::remove_file(&path);

        assert!(result.is_ok());
        let json = result.unwrap();
        assert!(json.contains("\"key\": \"value\""));
    }

    #[test]
    fn test_sonic_fix_messy() {
        // Messy JSON-like content (PsychicPaper heuristic handles this)
        let path = temp_file("fix_messy", "key: value\nlist: [1, 2]");
        let sonic = SonicScrewdriver::new();
        let result = sonic.fix(&path);
        let _ = fs::remove_file(&path);

        assert!(result.is_ok());
        let json = result.unwrap();
        // Should parse as object
        assert!(json.contains("\"key\": \"value\""));
        // Check for array formatting (pretty print puts elements on new lines)
        assert!(json.contains("\"list\": ["));
        assert!(json.contains("1"));
        assert!(json.contains("2"));
    }

    #[test]
    fn test_sonic_scan() {
        let path = temp_file("scan", r#"{ "a": 1, "b": 2 }"#);
        let sonic = SonicScrewdriver::new();
        let result = sonic.scan(&path);
        let _ = fs::remove_file(&path);

        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("Found object with 2 keys"));
        assert!(output.contains("\"a\""));
        assert!(output.contains("\"b\""));
    }
}
