# Tardis Chronos ⚡

> "Time is the Fire in which we burn." — *Generations*

**Chronos** is the Retrieval-Augmented Generation (RAG) orchestration engine for Tardis OS. It acts as the bridge between the semantic reasoning of **Vortex** (the LLM) and the bi-temporal memory of **Gallifrey**.

## 🏗️ Architecture

Chronos executes queries through a four-stage pipeline designed to ground AI responses in both facts and time.

```text
User Query ──► [ 1. Analysis ] ───► [ 2. Retrieval ] ───► [ 3. Augmentation ] ───► [ 4. Inference ] ──► Response
                     │                    │                      │                       │
           ┌─────────▼─────────┐  ┌───────▼────────┐   ┌─────────▼─────────┐   ┌─────────▼─────────┐
           │ Intent Classifier │  │ Gallifrey Store│   │ Prompt Engineer   │   │ Vortex Engine     │
           │ Temporal Extractor│  │ (Vector DB)    │   │ (System Context)  │   │ (LLM)             │
           └───────────────────┘  └────────────────┘   └───────────────────┘   └───────────────────┘
```

## 🧩 Pipeline Stages

1.  **Analysis (`QueryAnalyzer`)**:
    *   **Intent Classification**: Determines if the user is asking a question (`Recall`), stating a fact (`Remember`), or commanding the system (`SystemQuery`).
    *   **Temporal Extraction**: Resolves relative time references like "yesterday" or "last week" into absolute UTC timestamps using `chrono`.

2.  **Retrieval (`Retriever`)**:
    *   Fetches relevant entities from the **Knowledge Graph** using vector similarity.
    *   Retrieves past interactions from the **Conversation History**.
    *   Filters results based on the temporal window identified in the analysis phase.

3.  **Augmentation (`ContextAugmenter`)**:
    *   Constructs a structured prompt that includes the user's query, retrieved context, and system instructions.
    *   injects current system time and temporal context to ensure the model understands "now".

4.  **Inference**:
    *   Sends the augmented prompt to **Vortex** for final text generation.

## 🚀 Usage

Chronos is typically used by the `tardis-shell` or other user-facing interfaces. It requires an initialized LLM service (via `Vortex`) and storage services (via `Gallifrey`).

```rust,no_run
use std::sync::Arc;
use tardis_chronos::{Chronos, RagConfig};
use tardis_vortex::{Vortex, VortexLlmService, ModelLoadConfig};
use tardis_gallifrey::Gallifrey;

# async fn example() -> anyhow::Result<()> {
// 1. Initialize Vortex (LLM Engine)
let vortex = Arc::new(Vortex::new()?);
let model_handle = vortex.load_model(
    "/path/to/model.safetensors",
    ModelLoadConfig::default()
).await?;

// Create the service adapter
let llm_service = Arc::new(VortexLlmService::new(vortex, model_handle));

// 2. Initialize Gallifrey (Temporal Knowledge Store)
let gallifrey = Gallifrey::new();

// 3. Initialize Chronos with specific services
let chronos = Chronos::new(
    llm_service,              // LlmService
    gallifrey.knowledge(),    // KnowledgeService
    gallifrey.conversation(), // ConversationService
    gallifrey.system_state()  // SystemStateService
);

// 4. Run a query
let response = chronos.query(
    "What did I work on yesterday?",
    RagConfig::default()
).await?;

println!("Response: {}", response.text);
println!("Sources used: {}", response.sources.len());
# Ok(())
# }
```

## 🧠 Memory Categories

Chronos can also store new memories:

- **Knowledge**: General facts about the world.
- **Preference**: User preferences (e.g., "I like dark mode").
- **Fact**: Specific events or data points.

```rust,ignore
use tardis_chronos::MemoryCategory;

chronos.remember(
    "The project deadline is next Friday.",
    MemoryCategory::Fact
).await?;
```
