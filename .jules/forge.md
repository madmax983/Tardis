**God Function in Vortex Runtime**
**Learning:** The `Vortex::load_model` function had accumulated too many responsibilities: I/O, config parsing, weight loading (blocking), state management, and tokenizer loading. This made it hard to read and test.
**Action:** Extract distinct responsibilities into private helper methods (`load_weights_blocking`, `register_and_store_model`, `load_tokenizer`) to keep the main orchestration function linear and high-level.
