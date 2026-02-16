//! Simulacrum: A manager for divergent timelines.
//!
//! "Realities are just possibilities waiting to be observed."
//!
//! This module manages multiple `Timeline` instances, allowing users to create,
//! switch between, and experiment with different versions of the knowledge graph.

use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tardis_gallifrey::experimental::timeline::Timeline;
use tardis_gallifrey::Gallifrey;

/// The Simulation Manager.
#[derive(Debug)]
pub struct Simulacrum {
    /// The base reality (Gallifrey instance).
    base: Arc<Gallifrey>,
    /// Active forks (Timelines).
    forks: HashMap<String, Arc<Timeline>>,
}

impl Simulacrum {
    /// Create a new Simulacrum attached to a base Gallifrey instance.
    #[must_use]
    pub fn new(base: Arc<Gallifrey>) -> Self {
        Self {
            base,
            forks: HashMap::new(),
        }
    }

    /// Create a new fork (timeline).
    ///
    /// # Errors
    ///
    /// Returns an error if a fork with the same name already exists.
    pub fn fork(&mut self, name: &str) -> Result<()> {
        if self.forks.contains_key(name) {
            return Err(anyhow!("Timeline '{name}' already exists"));
        }

        let timeline = Arc::new(Timeline::new(self.base.clone()));
        self.forks.insert(name.to_string(), timeline);
        Ok(())
    }

    /// Get a timeline by name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<Arc<Timeline>> {
        self.forks.get(name).cloned()
    }

    /// List available timelines.
    #[must_use]
    pub fn list(&self) -> Vec<String> {
        let mut names: Vec<_> = self.forks.keys().cloned().collect();
        names.sort();
        names
    }

    /// Remove a timeline.
    ///
    /// # Errors
    ///
    /// Returns an error if the timeline does not exist.
    pub fn remove(&mut self, name: &str) -> Result<()> {
        if self.forks.remove(name).is_some() {
            Ok(())
        } else {
            Err(anyhow!("Timeline '{name}' not found"))
        }
    }
}
