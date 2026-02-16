# Tardis Vortex 🌪️

> "The Vortex is a funnel of psychic energy... a door to another dimension."

**Vortex** is the Large Language Model (LLM) inference engine for Tardis OS. It is built on top of [Candle](https://github.com/huggingface/candle), a minimalist ML framework for Rust, enabling high-performance, GPU-accelerated inference directly within the OS.

## ✨ Features

-   **Native Rust Inference**: No Python dependencies or external API calls required.
-   **Multi-Architecture Support**: Supports popular architectures including:
    -   Llama (CodeLlama, Llama 2/3)
    -   Mistral / Mixtral
    -   Phi (2/3)
    -   Gemma
-   **Quantization**: Supports loading quantized models (GGUF, Q4, Q8) for memory efficiency.
-   **State Management**: Manages KV-caches and model states efficiently.
-   **Token Streaming**: Supports real-time token generation for responsive UIs.

## 🚀 Usage

Vortex handles the complexity of loading weights, tokenizing input, and sampling tokens.

```rust,no_run
use tardis_vortex::{Vortex, ModelLoadConfig, InferenceParams};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Initialize the engine
    // This detects available hardware (CUDA, Metal, CPU)
    let vortex = Vortex::new()?;

    // 2. Load a model
    // You can load from a local directory containing .safetensors and config.json
    let handle = vortex.load_model(
        "/path/to/models/phi-3-mini",
        ModelLoadConfig {
            quantized: true, // Use quantization if available
            ..Default::default()
        },
    ).await?;

    // 3. Run inference
    let response = vortex.infer(
        handle,
        "Explain the theory of relativity in 5 words.",
        InferenceParams {
            temperature: 0.7,
            max_tokens: 50,
            ..Default::default()
        },
    ).await?;

    println!("Response: {}", response);
    Ok(())
}
```

## 🏗️ Architecture

Vortex is composed of several modules:

-   **`loader`**: Handles parsing `config.json` and loading SafeTensors/GGUF weights.
-   **`model`**: Defines architecture implementations (e.g., `LlamaModel`, `PhiModel`).
-   **`inference`**: Manages the inference loop, KV-cache updates, and sampling.
-   **`tokenizer`**: Wraps the Hugging Face `tokenizers` library for text encoding/decoding.

## ⚠️ Requirements

Vortex relies on `candle-core`. To enable GPU support:

-   **NVIDIA**: Ensure CUDA toolkit is installed. The `cuda` feature is enabled by default in the workspace.
-   **macOS**: Metal support is automatically enabled on Apple Silicon.
