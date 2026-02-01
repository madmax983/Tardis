**[God Function Detected]
**Learning:** `Vortex::load_model` was accumulating too many responsibilities: I/O, parsing, concurrent weight loading, registry management, and tokenizer setup.
**Action:** Extracted `load_weights_task`, `register_loaded_model`, and `setup_tokenizer` to keep the main flow readable. Future additions to model loading should follow this pattern of small, dedicated helpers.
