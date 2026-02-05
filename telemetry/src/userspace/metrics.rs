//! Metrics collection utilities.
//!
//! Provides thread-safe counters, gauges, and histograms for recording
//! application metrics.
//!
//! # Usage
//!
//! ```rust,ignore
//! use tardis_telemetry::metrics::{counter, gauge, histogram, Subsystem};
//!
//! // Increment a counter
//! counter!("requests_total").inc();
//! counter!("bytes_sent", Subsystem::Vortex).add(1024);
//!
//! // Set a gauge value
//! gauge!("active_connections").set(42);
//! gauge!("memory_used_bytes").inc();
//!
//! // Record a histogram value
//! histogram!("request_latency_ms").record(150);
//! ```

use crate::meta::Subsystem;
use crate::metrics_types::{MetricSample, MetricValue};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

/// Global metrics registry.
pub static METRICS: std::sync::LazyLock<MetricsRegistry> =
    std::sync::LazyLock::new(MetricsRegistry::new);

/// Thread-safe metrics registry.
///
/// Stores all application metrics and provides methods to register
/// and retrieve counters, gauges, and histograms.
pub struct MetricsRegistry {
    counters: RwLock<HashMap<&'static str, Arc<Counter>>>,
    gauges: RwLock<HashMap<&'static str, Arc<Gauge>>>,
    histograms: RwLock<HashMap<&'static str, Arc<Histogram>>>,
}

impl MetricsRegistry {
    /// Creates a new empty metrics registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            counters: RwLock::new(HashMap::new()),
            gauges: RwLock::new(HashMap::new()),
            histograms: RwLock::new(HashMap::new()),
        }
    }

    /// Gets or creates a counter.
    pub fn counter(&self, name: &'static str, subsystem: Subsystem) -> Arc<Counter> {
        // Fast path: check if already exists
        {
            let counters = self.counters.read();
            if let Some(counter) = counters.get(name) {
                return Arc::clone(counter);
            }
        }

        // Slow path: create new counter
        let mut counters = self.counters.write();
        counters
            .entry(name)
            .or_insert_with(|| Arc::new(Counter::new(name, subsystem)))
            .clone()
    }

    /// Gets or creates a gauge.
    pub fn gauge(&self, name: &'static str, subsystem: Subsystem) -> Arc<Gauge> {
        {
            let gauges = self.gauges.read();
            if let Some(gauge) = gauges.get(name) {
                return Arc::clone(gauge);
            }
        }

        let mut gauges = self.gauges.write();
        gauges
            .entry(name)
            .or_insert_with(|| Arc::new(Gauge::new(name, subsystem)))
            .clone()
    }

    /// Gets or creates a histogram.
    pub fn histogram(&self, name: &'static str, subsystem: Subsystem) -> Arc<Histogram> {
        {
            let histograms = self.histograms.read();
            if let Some(histogram) = histograms.get(name) {
                return Arc::clone(histogram);
            }
        }

        let mut histograms = self.histograms.write();
        histograms
            .entry(name)
            .or_insert_with(|| Arc::new(Histogram::new(name, subsystem)))
            .clone()
    }

    /// Collects all metrics for export.
    #[must_use]
    pub fn collect(&self) -> Vec<MetricSample> {
        // Bolt optimization: Pre-allocate vector to avoid reallocations.
        // We acquire read locks briefly to check size, which is cheaper than reallocating.
        let capacity = self.counters.read().len()
            + self.gauges.read().len()
            + self.histograms.read().len();

        let mut samples = Vec::with_capacity(capacity);
        let timestamp_ns = u64::try_from(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos(),
        )
        .unwrap_or_default();

        // Collect counters
        for counter in self.counters.read().values() {
            samples.push(MetricSample {
                name: counter.name.to_string(),
                subsystem: counter.subsystem,
                timestamp_ns,
                value: MetricValue::Counter(counter.get()),
                labels: Vec::new(),
            });
        }

        // Collect gauges
        for gauge in self.gauges.read().values() {
            samples.push(MetricSample {
                name: gauge.name.to_string(),
                subsystem: gauge.subsystem,
                timestamp_ns,
                value: MetricValue::Gauge(gauge.get()),
                labels: Vec::new(),
            });
        }

        // Collect histograms
        for histogram in self.histograms.read().values() {
            let (sum, count, buckets) = histogram.snapshot();
            samples.push(MetricSample {
                name: histogram.name.to_string(),
                subsystem: histogram.subsystem,
                timestamp_ns,
                value: MetricValue::Histogram {
                    sum,
                    count,
                    buckets,
                },
                labels: Vec::new(),
            });
        }

        samples
    }

    /// Resets all metrics to their initial values.
    pub fn reset(&self) {
        for counter in self.counters.read().values() {
            counter.reset();
        }
        for gauge in self.gauges.read().values() {
            gauge.set(0);
        }
        for histogram in self.histograms.read().values() {
            histogram.reset();
        }
    }
}

impl Default for MetricsRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for MetricsRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MetricsRegistry")
            .field("counters", &self.counters.read().len())
            .field("gauges", &self.gauges.read().len())
            .field("histograms", &self.histograms.read().len())
            .finish()
    }
}

/// Monotonically increasing counter.
///
/// Counters can only increase and are typically used for counting
/// events like requests, errors, or bytes transferred.
pub struct Counter {
    /// Metric name.
    pub name: &'static str,
    /// Originating subsystem.
    pub subsystem: Subsystem,
    /// Current value.
    value: AtomicU64,
}

impl Counter {
    /// Creates a new counter.
    #[must_use]
    pub const fn new(name: &'static str, subsystem: Subsystem) -> Self {
        Self {
            name,
            subsystem,
            value: AtomicU64::new(0),
        }
    }

    /// Increments the counter by 1.
    pub fn inc(&self) {
        self.value.fetch_add(1, Ordering::Relaxed);
    }

    /// Adds a value to the counter.
    pub fn add(&self, n: u64) {
        self.value.fetch_add(n, Ordering::Relaxed);
    }

    /// Gets the current value.
    #[must_use]
    pub fn get(&self) -> u64 {
        self.value.load(Ordering::Relaxed)
    }

    /// Resets the counter to zero.
    pub fn reset(&self) {
        self.value.store(0, Ordering::Relaxed);
    }
}

impl std::fmt::Debug for Counter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Counter")
            .field("name", &self.name)
            .field("subsystem", &self.subsystem)
            .field("value", &self.get())
            .finish()
    }
}

/// Gauge that can go up or down.
///
/// Gauges represent a value that can increase or decrease, like
/// current memory usage or active connections.
pub struct Gauge {
    /// Metric name.
    pub name: &'static str,
    /// Originating subsystem.
    pub subsystem: Subsystem,
    /// Current value.
    value: AtomicI64,
}

impl Gauge {
    /// Creates a new gauge.
    #[must_use]
    pub const fn new(name: &'static str, subsystem: Subsystem) -> Self {
        Self {
            name,
            subsystem,
            value: AtomicI64::new(0),
        }
    }

    /// Sets the gauge to a specific value.
    pub fn set(&self, v: i64) {
        self.value.store(v, Ordering::Relaxed);
    }

    /// Increments the gauge by 1.
    pub fn inc(&self) {
        self.value.fetch_add(1, Ordering::Relaxed);
    }

    /// Decrements the gauge by 1.
    pub fn dec(&self) {
        self.value.fetch_sub(1, Ordering::Relaxed);
    }

    /// Adds a value to the gauge.
    pub fn add(&self, n: i64) {
        self.value.fetch_add(n, Ordering::Relaxed);
    }

    /// Subtracts a value from the gauge.
    pub fn sub(&self, n: i64) {
        self.value.fetch_sub(n, Ordering::Relaxed);
    }

    /// Gets the current value.
    #[must_use]
    pub fn get(&self) -> i64 {
        self.value.load(Ordering::Relaxed)
    }
}

impl std::fmt::Debug for Gauge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Gauge")
            .field("name", &self.name)
            .field("subsystem", &self.subsystem)
            .field("value", &self.get())
            .finish()
    }
}

/// Default histogram bucket boundaries (in milliseconds for latency metrics).
pub const DEFAULT_BUCKETS: [f64; 11] = [
    0.5, 1.0, 2.5, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0,
];

/// Histogram for recording distributions of values.
///
/// Histograms track the distribution of values using predefined buckets,
/// useful for latency measurements and other distributions.
pub struct Histogram {
    /// Metric name.
    pub name: &'static str,
    /// Originating subsystem.
    pub subsystem: Subsystem,
    /// Bucket boundaries.
    buckets: Vec<f64>,
    /// Bucket counts.
    counts: Vec<AtomicU64>,
    /// Sum of all recorded values.
    sum: AtomicU64,
    /// Total count of recorded values.
    count: AtomicU64,
}

impl Histogram {
    /// Creates a new histogram with default buckets.
    #[must_use]
    pub fn new(name: &'static str, subsystem: Subsystem) -> Self {
        Self::with_buckets(name, subsystem, &DEFAULT_BUCKETS)
    }

    /// Creates a new histogram with custom buckets.
    #[must_use]
    pub fn with_buckets(name: &'static str, subsystem: Subsystem, buckets: &[f64]) -> Self {
        let counts = buckets.iter().map(|_| AtomicU64::new(0)).collect();

        Self {
            name,
            subsystem,
            buckets: buckets.to_vec(),
            counts,
            sum: AtomicU64::new(0),
            count: AtomicU64::new(0),
        }
    }

    /// Records a value in the histogram.
    pub fn record(&self, value: u64) {
        self.count.fetch_add(1, Ordering::Relaxed);
        self.sum.fetch_add(value, Ordering::Relaxed);

        // Precision loss is acceptable for histogram buckets
        #[allow(clippy::cast_precision_loss)]
        let value_f64 = value as f64;
        for (i, boundary) in self.buckets.iter().enumerate() {
            if value_f64 <= *boundary {
                // Bolt optimization: Only increment the first matching bucket (non-cumulative).
                // This reduces atomic operations from O(N) to O(1).
                // The snapshot() method will reconstruct cumulative counts.
                self.counts[i].fetch_add(1, Ordering::Relaxed);
                return;
            }
        }
    }

    /// Records a floating-point value.
    pub fn record_f64(&self, value: f64) {
        // Truncation/sign loss acceptable for mapping float to integer histogram
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        self.record(value as u64);
    }

    /// Gets a snapshot of the histogram.
    #[must_use]
    pub fn snapshot(&self) -> (f64, u64, Vec<u64>) {
        #[allow(clippy::cast_precision_loss)]
        let sum = self.sum.load(Ordering::Relaxed) as f64;
        let count = self.count.load(Ordering::Relaxed);

        // Bolt optimization: Accumulate counts to restore cumulative buckets.
        // This ensures monotonic snapshots even under concurrent updates.
        let mut accumulated = 0;
        let buckets: Vec<u64> = self
            .counts
            .iter()
            .map(|c| {
                accumulated += c.load(Ordering::Relaxed);
                accumulated
            })
            .collect();
        (sum, count, buckets)
    }

    /// Resets the histogram.
    pub fn reset(&self) {
        self.sum.store(0, Ordering::Relaxed);
        self.count.store(0, Ordering::Relaxed);
        for count in &self.counts {
            count.store(0, Ordering::Relaxed);
        }
    }

    /// Gets the bucket boundaries.
    #[must_use]
    pub fn bucket_boundaries(&self) -> &[f64] {
        &self.buckets
    }
}

impl std::fmt::Debug for Histogram {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (sum, count, _) = self.snapshot();
        f.debug_struct("Histogram")
            .field("name", &self.name)
            .field("subsystem", &self.subsystem)
            .field("sum", &sum)
            .field("count", &count)
            .finish_non_exhaustive()
    }
}

/// Creates or retrieves a counter from the global registry.
#[macro_export]
macro_rules! counter {
    ($name:expr_2021) => {
        $crate::metrics::METRICS.counter($name, $crate::Subsystem::Unknown)
    };
    ($name:expr_2021, $subsystem:expr_2021) => {
        $crate::metrics::METRICS.counter($name, $subsystem)
    };
}

/// Creates or retrieves a gauge from the global registry.
#[macro_export]
macro_rules! gauge {
    ($name:expr_2021) => {
        $crate::metrics::METRICS.gauge($name, $crate::Subsystem::Unknown)
    };
    ($name:expr_2021, $subsystem:expr_2021) => {
        $crate::metrics::METRICS.gauge($name, $subsystem)
    };
}

/// Creates or retrieves a histogram from the global registry.
#[macro_export]
macro_rules! histogram {
    ($name:expr_2021) => {
        $crate::metrics::METRICS.histogram($name, $crate::Subsystem::Unknown)
    };
    ($name:expr_2021, $subsystem:expr_2021) => {
        $crate::metrics::METRICS.histogram($name, $subsystem)
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counter_operations() {
        let counter = Counter::new("test_counter", Subsystem::Kernel);
        assert_eq!(counter.get(), 0);

        counter.inc();
        assert_eq!(counter.get(), 1);

        counter.add(10);
        assert_eq!(counter.get(), 11);

        counter.reset();
        assert_eq!(counter.get(), 0);
    }

    #[test]
    fn gauge_operations() {
        let gauge = Gauge::new("test_gauge", Subsystem::Kernel);
        assert_eq!(gauge.get(), 0);

        gauge.set(42);
        assert_eq!(gauge.get(), 42);

        gauge.inc();
        assert_eq!(gauge.get(), 43);

        gauge.dec();
        assert_eq!(gauge.get(), 42);

        gauge.add(10);
        assert_eq!(gauge.get(), 52);

        gauge.sub(5);
        assert_eq!(gauge.get(), 47);
    }

    #[test]
    fn histogram_operations() {
        let histogram =
            Histogram::with_buckets("test_histogram", Subsystem::Kernel, &[1.0, 5.0, 10.0]);

        histogram.record(2);
        histogram.record(7);
        histogram.record(3);

        let (sum, count, buckets) = histogram.snapshot();
        assert_eq!(count, 3);
        assert!((sum - 12.0).abs() < f64::EPSILON);
        assert_eq!(buckets, vec![0, 2, 3]); // 0 <= 1, 2 <= 5, 3 <= 10
    }

    #[test]
    fn registry_operations() {
        let registry = MetricsRegistry::new();

        let c1 = registry.counter("requests", Subsystem::Vortex);
        let c2 = registry.counter("requests", Subsystem::Vortex);

        // Should return the same counter
        c1.inc();
        assert_eq!(c2.get(), 1);

        let samples = registry.collect();
        assert!(!samples.is_empty());
    }
}
