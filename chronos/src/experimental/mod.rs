//! Experimental features for Chronos.
//!
//! These features are unstable and hidden behind the `nova` feature flag.

#[cfg(feature = "nova")]
pub mod psychic_paper;

#[cfg(feature = "nova")]
pub mod doctor;

#[cfg(feature = "nova")]
pub mod curiosity;
