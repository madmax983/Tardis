//! Metric data types.
//!
//! Provides structures for metric values and samples.
//! `no_std` compatible.

use crate::meta::Subsystem;
use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

/// Metric value types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MetricValue {
    /// Monotonically increasing counter.
    Counter(u64),
    /// Value that can go up or down.
    Gauge(i64),
    /// Distribution of values with buckets.
    Histogram {
        /// Sum of all recorded values.
        sum: f64,
        /// Count of recorded values.
        count: u64,
        /// Bucket counts (index corresponds to predefined bucket boundaries).
        buckets: Vec<u64>,
    },
}

/// A recorded metric sample.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricSample {
    /// Metric name.
    pub name: String,
    /// Originating subsystem.
    pub subsystem: Subsystem,
    /// Timestamp in nanoseconds.
    pub timestamp_ns: u64,
    /// Metric value.
    pub value: MetricValue,
    /// Labels as key-value pairs.
    pub labels: Vec<(String, String)>,
}
