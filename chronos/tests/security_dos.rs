#[cfg(test)]
mod tests {
    use tardis_chronos::experimental::psychic_paper::{Intent, PsychicPaper};
    use std::time::Instant;

    #[test]
    fn test_zip_bomb_dos() {
        let paper = PsychicPaper::new();

        // Construct a "zip bomb" - many newlines with some content
        // 2MB of "a\n" - exceeds 1MB limit
        let mut bomb = String::with_capacity(2 * 1024 * 1024);
        // "a\n" is 2 bytes. 1M iterations = 2MB.
        for _ in 0..1_000_000 {
            bomb.push_str("a\n");
        }

        let start = Instant::now();
        // Intent::List triggers the line splitting logic
        let result = paper.interpret(&bomb, Intent::List);
        let duration = start.elapsed();

        println!("Time taken: {:?}", duration);

        // It should return an error immediately because input > MAX_TEXT_LEN
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("exceeds maximum length"));

        // Ensure it failed fast
        assert!(duration.as_millis() < 100);
    }

    #[test]
    fn test_item_limit_dos() {
        let paper = PsychicPaper::new();

        // Construct a list with many items but small total size
        // 2000 items of "a\n" = 4000 bytes. < 1MB.
        let mut list_text = String::with_capacity(4096);
        for i in 0..2000 {
            list_text.push_str(&format!("- Item {}\n", i));
        }

        let start = Instant::now();
        let result = paper.interpret(&list_text, Intent::List);
        let duration = start.elapsed();

        println!("Time taken: {:?}", duration);

        assert!(result.is_ok());
        let json = result.unwrap();
        let array = json.as_array().expect("Should be array");

        // Should be truncated to MAX_ITEMS (1000)
        assert_eq!(array.len(), 1000);

        // Ensure it processed reasonably fast
        assert!(duration.as_millis() < 100);
    }
}
