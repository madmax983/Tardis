//! Temporal primitives for bi-temporal data.
//!
//! Provides types for working with bi-temporal data, which tracks both:
//! - **Valid time**: When the fact was/is true in the real world
//! - **Transaction time**: When the fact was recorded in the system
//!
//! This enables queries like "What did we know about X at time T?"

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A range of time with optional end (open-ended if None).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeRange {
    /// Start of the time range (inclusive).
    pub start: DateTime<Utc>,
    /// End of the time range (exclusive). None means "forever" / current.
    pub end: Option<DateTime<Utc>>,
}

impl TimeRange {
    /// Create a time range starting now with no end.
    #[must_use]
    pub fn from_now() -> Self {
        Self {
            start: Utc::now(),
            end: None,
        }
    }

    /// Create a time range with specific start and no end.
    #[must_use]
    pub const fn starting_at(start: DateTime<Utc>) -> Self {
        Self { start, end: None }
    }

    /// Create a bounded time range.
    #[must_use]
    pub const fn bounded(start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        Self {
            start,
            end: Some(end),
        }
    }

    /// Check if a timestamp falls within this range.
    #[must_use]
    pub fn contains(&self, timestamp: DateTime<Utc>) -> bool {
        if timestamp < self.start {
            return false;
        }
        match self.end {
            Some(end) => timestamp < end,
            None => true,
        }
    }

    /// Check if this range is currently active (no end or end is in future).
    #[must_use]
    pub fn is_current(&self) -> bool {
        match self.end {
            Some(end) => end > Utc::now(),
            None => true,
        }
    }

    /// Close this range at the current time.
    #[must_use]
    pub fn close_now(&self) -> Self {
        Self {
            start: self.start,
            end: Some(Utc::now()),
        }
    }
}

impl Default for TimeRange {
    fn default() -> Self {
        Self::from_now()
    }
}

/// Bi-temporal interval tracking both valid and transaction time.
///
/// This is the core temporal primitive used throughout Gallifrey. It allows answering two types of questions:
/// 1. **History**: "What was true at 10 AM yesterday?" (Valid Time)
/// 2. **Audit**: "What did we *believe* was true at 10 AM yesterday?" (Transaction Time)
///
/// # Examples
///
/// Correcting a mistake in the past:
///
/// ```
/// use tardis_common::temporal::BiTemporalInterval;
///
/// // 1. We record a fact (Transaction Time starts now)
/// //    Belief: "It is raining" (Valid Time starts now)
/// let original_fact = BiTemporalInterval::now();
///
/// // 2. Later, we realize it actually started raining an hour ago.
/// //    We "supersede" the old fact (close its Transaction Time).
/// let old_version = original_fact.supersede();
///
/// // 3. We create a new version with the corrected Valid Time.
/// //    Transaction Time starts NOW (we just learned this).
/// //    Valid Time starts 1 hour ago (when it actually happened).
/// //    The system now knows that "it was raining an hour ago",
/// //    but also remembers that "we didn't know that until now".
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BiTemporalInterval {
    /// When the fact was/is true in the real world.
    pub valid_time: TimeRange,
    /// When the fact was recorded in the system.
    pub transaction_time: TimeRange,
}

impl BiTemporalInterval {
    /// Create a new bi-temporal interval starting now.
    #[must_use]
    pub fn now() -> Self {
        let now = Utc::now();
        Self {
            valid_time: TimeRange::starting_at(now),
            transaction_time: TimeRange::starting_at(now),
        }
    }

    /// Create a bi-temporal interval with specific valid time, transaction time starting now.
    #[must_use]
    pub fn with_valid_time(valid_time: TimeRange) -> Self {
        Self {
            valid_time,
            transaction_time: TimeRange::from_now(),
        }
    }

    /// Check if this interval is current in both dimensions.
    #[must_use]
    pub fn is_current(&self) -> bool {
        self.valid_time.is_current() && self.transaction_time.is_current()
    }

    /// Close the transaction time (mark as superseded).
    #[must_use]
    pub fn supersede(&self) -> Self {
        Self {
            valid_time: self.valid_time,
            transaction_time: self.transaction_time.close_now(),
        }
    }

    /// Check if this interval was active at a specific valid time.
    #[must_use]
    pub fn valid_at(&self, timestamp: DateTime<Utc>) -> bool {
        self.valid_time.contains(timestamp)
    }

    /// Check if this interval was known at a specific transaction time.
    #[must_use]
    pub fn known_at(&self, timestamp: DateTime<Utc>) -> bool {
        self.transaction_time.contains(timestamp)
    }

    /// Check if this interval was active at both valid and transaction times.
    #[must_use]
    pub fn active_at(&self, valid: DateTime<Utc>, transaction: DateTime<Utc>) -> bool {
        self.valid_at(valid) && self.known_at(transaction)
    }
}

impl Default for BiTemporalInterval {
    fn default() -> Self {
        Self::now()
    }
}

/// Parameters for temporal queries.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TemporalQuery {
    /// Point-in-time for valid time queries (AS OF VALID TIME).
    pub valid_at: Option<DateTime<Utc>>,
    /// Point-in-time for transaction time queries (AS OF SYSTEM TIME).
    pub transaction_at: Option<DateTime<Utc>>,
    /// Time range for valid time (BETWEEN).
    pub valid_between: Option<(DateTime<Utc>, DateTime<Utc>)>,
    /// Whether to include historical versions.
    pub include_history: bool,
}

impl TemporalQuery {
    /// Create a query for current state (default).
    #[must_use]
    pub const fn current() -> Self {
        Self {
            valid_at: None,
            transaction_at: None,
            valid_between: None,
            include_history: false,
        }
    }

    /// Create a query for state as of a specific valid time.
    #[must_use]
    pub const fn as_of_valid(timestamp: DateTime<Utc>) -> Self {
        Self {
            valid_at: Some(timestamp),
            transaction_at: None,
            valid_between: None,
            include_history: false,
        }
    }

    /// Create a query for state as of a specific transaction time.
    #[must_use]
    pub const fn as_of_transaction(timestamp: DateTime<Utc>) -> Self {
        Self {
            valid_at: None,
            transaction_at: Some(timestamp),
            valid_between: None,
            include_history: false,
        }
    }

    /// Create a query for state at both valid and transaction times.
    #[must_use]
    pub const fn as_of_both(valid: DateTime<Utc>, transaction: DateTime<Utc>) -> Self {
        Self {
            valid_at: Some(valid),
            transaction_at: Some(transaction),
            valid_between: None,
            include_history: false,
        }
    }

    /// Include historical versions in the result.
    #[must_use]
    pub const fn with_history(mut self) -> Self {
        self.include_history = true;
        self
    }
}

/// A temporal reference extracted from natural language.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TemporalReference {
    /// Relative reference like "yesterday", "last week".
    Relative {
        /// The original text
        text: String,
        /// Resolved timestamp
        resolved: DateTime<Utc>,
    },
    /// Absolute reference like "March 15, 2024".
    Absolute(DateTime<Utc>),
    /// Event-based reference like "before the update".
    EventBased {
        /// The event description
        event: String,
        /// Resolved timestamp (if found)
        resolved: Option<DateTime<Utc>>,
    },
    /// No temporal reference (use current time).
    Implicit,
}

impl TemporalReference {
    /// Get the resolved timestamp, if available.
    #[must_use]
    pub fn resolved(&self) -> Option<DateTime<Utc>> {
        match self {
            Self::Relative { resolved, .. } => Some(*resolved),
            Self::Absolute(ts) => Some(*ts),
            Self::EventBased { resolved, .. } => *resolved,
            Self::Implicit => Some(Utc::now()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn time_range_contains() {
        let now = Utc::now();
        let range = TimeRange::starting_at(now - Duration::hours(1));

        assert!(!range.contains(now - Duration::hours(2)));
        assert!(range.contains(now - Duration::minutes(30)));
        assert!(range.contains(now));
    }

    #[test]
    fn time_range_bounded() {
        let now = Utc::now();
        let range = TimeRange::bounded(now - Duration::hours(2), now - Duration::hours(1));

        assert!(!range.contains(now - Duration::hours(3)));
        assert!(range.contains(now - Duration::hours(1) - Duration::minutes(30)));
        assert!(!range.contains(now));
    }

    #[test]
    fn bi_temporal_current() {
        let interval = BiTemporalInterval::now();
        assert!(interval.is_current());
    }

    #[test]
    fn bi_temporal_supersede() {
        let interval = BiTemporalInterval::now();
        let superseded = interval.supersede();

        assert!(interval.is_current());
        assert!(!superseded.transaction_time.is_current());
    }
}
