#[cfg(feature = "nova")]
#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::Write;
    use std::path::PathBuf;
    use tardis_shell::experimental::sonic::SonicScrewdriver;

    fn create_large_file(path: &PathBuf, size_mb: usize) -> std::io::Result<()> {
        let mut file = File::create(path)?;
        let chunk = vec![0u8; 1024 * 1024]; // 1MB chunk
        for _ in 0..size_mb {
            file.write_all(&chunk)?;
        }
        Ok(())
    }

    #[test]
    fn test_inspect_rejects_large_file() {
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("large_file.txt");

        // Create 11MB file
        create_large_file(&file_path, 11).expect("Failed to create large file");

        let sonic = SonicScrewdriver::new(None, None, None, None);
        let result = sonic.inspect(&file_path);

        // Cleanup first to avoid disk usage
        let _ = std::fs::remove_file(&file_path);

        assert!(result.is_err(), "Inspect should fail for large file");
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("File too large") || err.to_string().contains("exceeds limit"),
            "Error should be about file size, got: {}",
            err
        );
    }

    #[test]
    fn test_inspect_rejects_directory() {
        let temp_dir = std::env::temp_dir();
        // Use temp_dir itself as the directory to test
        let sonic = SonicScrewdriver::new(None, None, None, None);
        let result = sonic.inspect(&temp_dir);

        assert!(result.is_err(), "Inspect should fail for directory");
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("Not a regular file"),
            "Error should be about file type, got: {}",
            err
        );
    }

    #[test]
    fn test_inspect_accepts_small_file() {
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("small_file.txt");
        let mut file = File::create(&file_path).expect("Failed to create file");
        writeln!(file, r#"{{"key": "value"}}"#).expect("Failed to write file");

        let sonic = SonicScrewdriver::new(None, None, None, None);
        let result = sonic.inspect(&file_path);

        let _ = std::fs::remove_file(&file_path);

        assert!(result.is_ok(), "Inspect should pass for small file");
    }

    #[test]
    fn test_repair_rejects_large_file() {
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("large_file_repair.txt");

        // Create 11MB file
        create_large_file(&file_path, 11).expect("Failed to create large file");

        let sonic = SonicScrewdriver::new(None, None, None, None);
        let result = sonic.repair(&file_path);

        // Cleanup
        let _ = std::fs::remove_file(&file_path);

        assert!(result.is_err(), "Repair should fail for large file");
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("File too large") || err.to_string().contains("exceeds limit"),
            "Error should be about file size, got: {}",
            err
        );
    }
}
