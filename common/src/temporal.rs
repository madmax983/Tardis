//! Temporal primitives for bi-temporal data.
//!
//! # What is Bi-Temporality?
//!
//! Bi-temporal data tracks two dimensions of time for every fact:
//!
//! 1. **Valid Time**: The time range when the fact was, is, or will be true in the real world.
//!    *Example: "John lived in NY from 2020 to 2022."*
//! 2. **Transaction Time**: The time range when the system knew this fact to be true.
//!    *Example: "We learned this on Jan 1st, 2023, and it is currently the accepted truth."*
//!
//! # Why is this useful?
//!
//! It allows for **retroactive corrections** without losing history.
//!
//! *Scenario:*
//! - **Day 1**: System records "Price is $10" (Valid: [Day 1, ∞), Transaction: [Day 1, ∞)).
//! - **Day 2**: We realize the price was actually $12 starting Day 1.
//! - **Correction**: We close the Transaction Time of the first record (making it historical)
//!   and insert a new record "Price is $12" (Valid: [Day 1, ∞), Transaction: [Day 2, ∞)).
//!
//! Now we can ask:
//! - "What is the price?" -> $12
//! - "What did we *think* the price was yesterday?" -> $10

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A range of time with optional end (open-ended if None).
///
/// Represents an interval `[start, end)`. If `end` is `None`, it represents `[start, ∞)`.
///
/// # Examples
///
/// ```
/// use tardis_common::temporal::TimeRange;
/// use chrono::{Utc, Duration};
///
/// let now = Utc::now();
/// let range = TimeRange::starting_at(now);
/// assert!(range.contains(now + Duration::days(1)));
/// ```
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
    ///
    /// Returns true if `start <= timestamp < end` (or `end` is None).
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
    ///
    /// Returns a new `TimeRange` with `end` set to `Utc::now()`.
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
/// This is the core temporal primitive used throughout Gallifrey.
/// It combines two [`TimeRange`]s to fully describe the temporal state of a fact.
///
/// # Examples
///
/// ```
/// use tardis_common::temporal::BiTemporalInterval;
///
/// let interval = BiTemporalInterval::now();
/// assert!(interval.is_current());
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
    ///
    /// Both valid and transaction time start at `Utc::now()` and are open-ended.
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
    ///
    /// This effectively "deletes" the record from the current system state,
    /// turning it into a historical artifact.
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
///
/// Allows specifying exactly which slice of time you want to query.
///
/// # Examples
///
/// ```
/// use tardis_common::temporal::TemporalQuery;
/// use chrono::Utc;
///
/// // Query the current state (default)
/// let query = TemporalQuery::current();
///
/// // Query the state as it was known yesterday
/// let yesterday = Utc::now() - chrono::Duration::days(1);
/// let historical = TemporalQuery::as_of_transaction(yesterday);
/// ```
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
