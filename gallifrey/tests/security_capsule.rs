#![cfg(feature = "nova")]
#![allow(missing_docs)]

use std::fs::File;
use std::io::Write;
use tardis_gallifrey::experimental::time_capsule::TimeCapsule;

#[test]
fn test_path_traversal_save() {
    let capsule = TimeCapsule::new();
    // Attempt to save with ".." in path
    // This should fail with "Path traversal detected" or similar error
    let result = capsule.save_to_file("../traversal_test.json");

    match result {
        Ok(_) => {
            let _ = std::fs::remove_file("../traversal_test.json");
            panic!("TimeCapsule::save_to_file should reject paths with '..'");
        }
        Err(e) => {
            let msg = e.to_string();
            assert!(
                msg.contains("Path traversal"),
                "Expected 'Path traversal' error, got: '{}'",
                msg
            );
        }
    }
}

#[test]
fn test_overwrite_protection() {
    let path = "overwrite_test.json";

    // Create an existing file
    {
        let mut file = File::create(path).unwrap();
        writeln!(file, "original content").unwrap();
    }

    let capsule = TimeCapsule::new();
    // Attempt to overwrite
    let result = capsule.save_to_file(path);

    // Verify original content is still there
    let content = std::fs::read_to_string(path).unwrap();
    let _ = std::fs::remove_file(path);

    if result.is_ok() {
        panic!("TimeCapsule::save_to_file should fail if file exists");
    }

    assert_eq!(content.trim(), "original content", "File content was overwritten!");
}

#[test]
fn test_path_traversal_load() {
    // Attempt to load from ".." path
    // We don't need the file to exist, the path validation should happen first
    let result = TimeCapsule::load_from_file("../passwd");

    if let Err(e) = result {
        let msg = e.to_string();
        assert!(
            msg.contains("Path traversal"),
            "Expected 'Path traversal' error, got: '{}'",
            msg
        );
    } else {
        panic!("TimeCapsule::load_from_file should reject paths with '..'");
    }
}
