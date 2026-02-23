//! Experimental features for Chronos.
//!
//! These features are unstable and hidden behind the `nova` feature flag.

/// Psychic Paper: The Universal Interpreter.
///
/// Robust parsing for unstructured text, designed to handle the messy output of LLMs or user input.
#[cfg(feature = "nova")]
pub mod psychic_paper;

/// Prophecy: The Future Forecast Engine.
///
/// Utilizes the Vortex LLM to predict likely future system states based on current context.
#[cfg(feature = "nova")]
pub mod prophecy;

/// The System Doctor 🩺.
///
/// A self-diagnostic tool that correlates telemetry data (symptoms) with system state changes (causes) to prescribe fixes.
#[cfg(feature = "nova")]
pub mod doctor;

/// The Curiosity Engine: Active Learning for Tardis.
///
/// Scans the knowledge graph for sparse or ambiguous entities and generates questions to ask the user.
#[cfg(feature = "nova")]
pub mod curiosity;

/// The Temporal Fugue: An Alternative Timeline Simulator.
///
/// Allows exploring counterfactual scenarios by branching from a specific point in the past.
#[cfg(feature = "nova")]
pub mod fugue;

/// The Dreamer: Memory Consolidation Engine.
///
/// Processes conversation history (short-term memory) and consolidates it into the Knowledge Graph (long-term memory).
#[cfg(feature = "nova")]
pub mod dreamer;

/// The Weaver 🕸️.
///
/// A narrative engine that connects seemingly unrelated entities through creative storytelling.
#[cfg(feature = "nova")]
pub mod weaver;

/// The Medium: Seance Engine.
///
/// Allows querying the system state at a specific point in the past.
#[cfg(feature = "nova")]
pub mod medium;

/// The Astrolabe: Semantic Pathfinder.
///
/// Implements A* search over the knowledge graph to find semantic paths between entities.
#[cfg(feature = "nova")]
pub mod astrolabe;

/// The Prism: Multi-Perspective Analysis Engine.
///
/// Breaks down a query into its spectral components, analyzing it from multiple viewpoints (personas).
#[cfg(feature = "nova")]
pub mod prism;
