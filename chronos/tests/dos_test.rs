use std::sync::Arc;
use tardis_chronos::{Chronos, ChronosError, MemoryCategory, RagConfig};
use tardis_gallifrey::Gallifrey;
use tardis_vortex::Vortex;

#[tokio::test]
async fn test_large_input_rejection() {
    // Setup
    let vortex = Arc::new(Vortex::new().unwrap());
    let gallifrey = Arc::new(Gallifrey::new());
    let chronos = Chronos::new(vortex, gallifrey);

    // Create a large input (20KB)
    let large_input = "a".repeat(20_000);

    // Test query
    let result = chronos.query(&large_input, RagConfig::default()).await;
    match result {
        Err(ChronosError::InputTooLarge(_)) => {
            // Success
        }
        Err(e) => panic!("Expected InputTooLarge, got: {:?}", e),
        Ok(_) => panic!("Expected InputTooLarge, got Ok"),
    }

    // Test remember
    let result = chronos.remember(&large_input, MemoryCategory::Fact).await;
    match result {
        Err(ChronosError::InputTooLarge(_)) => {
            // Success
        }
        Err(e) => panic!("Expected InputTooLarge, got: {:?}", e),
        Ok(_) => panic!("Expected InputTooLarge, got Ok"),
    }
}
