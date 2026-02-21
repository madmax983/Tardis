//! Experimental features for Chronos.
//!
//! These features are unstable and hidden behind the `nova` feature flag.

#[cfg(feature = "nova")]
pub mod psychic_paper;

#[cfg(feature = "nova")]
pub mod prophecy;

#[cfg(feature = "nova")]
pub mod doctor;

#[cfg(feature = "nova")]
pub mod curiosity;

#[cfg(feature = "nova")]
pub mod fugue;

#[cfg(feature = "nova")]
pub mod dreamer;

#[cfg(feature = "nova")]
pub mod weaver;

#[cfg(feature = "nova")]
pub mod medium;
