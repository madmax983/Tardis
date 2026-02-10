//! # Tardis Chronos ⚡
//!
//! > "Time is not a straight line. It's more of a ball of wibbly-wobbly, timey-wimey... stuff."
//!
//! **Chronos** is the RAG (Retrieval-Augmented Generation) orchestration engine for Tardis OS.
//! It serves as the bridge between the raw intelligence of **[Vortex](tardis_vortex::Vortex)** (LLM)
//! and the structured memory of **[Gallifrey](tardis_gallifrey::Gallifrey)** (Temporal Knowledge Graph).
//!
//! Unlike traditional RAG systems that just "search and stuff", Chronos understands **Time**.
//! It knows that "yesterday" means `Now - 1 Day`, and it retrieves context based on both
//! semantic similarity and temporal relevance.
//!
//! ## The Pipeline
//!
//! 1. **Analyze**: The user's query is analyzed for intent and temporal references (e.g., "What did I do last week?").
//! 2. **Retrieve**: Chronos queries Gallifrey for memories that match the semantic *and* temporal context.
//! 3. **Augment**: A prompt is constructed, enriching the user's query with the retrieved knowledge.
//! 4. **Infer**: The augmented prompt is sent to Vortex for generation.
//!
//! ## Hero's Journey: Your First Query
//!
//! ```rust,no_run
//! use std::sync::Arc;
//! use tardis_chronos::{Chronos, RagConfig};
//! use tardis_gallifrey::Gallifrey;
//! use tardis_vortex::Vortex;
//!
//! # async fn example() -> anyhow::Result<()> {
//! // 1. Initialize the Core Components
//! //    - Vortex: The LLM brain
//! //    - Gallifrey: The temporal memory
//! let vortex = Arc::new(Vortex::new()?);
//! let gallifrey = Arc::new(Gallifrey::new());
//!
//! // 2. Summon Chronos
//! //    The Time Lord of RAG, bridging memory and intelligence.
//! let chronos = Chronos::new(vortex, gallifrey);
//!
//! // 3. Ask a Question
//! //    "What happened yesterday?" - Chronos will resolve "yesterday"
//! //    to the correct date range and search for memories.
//! let config = RagConfig::default();
//! let response = chronos.query(
//!     "Summarize what we discussed yesterday about the Daleks.",
//!     config
//! ).await?;
//!
//! println!("🤖 Chronos says: {}", response.text);
//! println!("📚 Sources used: {}", response.sources.len());
//! # Ok(())
//! # }
//! ```
//!
//! ## Features
//!
//! - **Temporal RAG**: Resolves extraction like "last Tuesday" or "during the Time War".
//! - **Multi-Source Retrieval**: Pulls from Knowledge Graph, Conversation History, and System State.
//! - **Psychic Paper**: (Experimental) Cleans up messy LLM outputs into structured JSON.

#![warn(missing_docs)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

pub mod error;
pub mod memory;
pub mod pipeline;

// Re-export main types
pub use error::{ChronosError, ChronosResult};
pub use pipeline::{Chronos, MemoryCategory, RagConfig, RagResponse};

#[cfg(feature = "nova")]
pub mod experimental;
