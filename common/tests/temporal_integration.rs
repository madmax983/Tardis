//! Integration tests for temporal primitives.

use tardis_common::temporal::*;
use chrono::{Utc, Duration};

#[test]
fn time_range_is_current_with_future_start() {
    let now = Utc::now();
    let future_start = now + Duration::days(1);

    // Create a range starting in the future
    let range = TimeRange::starting_at(future_start);

    // Verify that is_current() returns true even if start is in future
    // This documents existing behavior: "current" means "not ended", not "active now".
    assert!(range.is_current(), "Future start time should still be considered current (not ended)");

    // Verify contains(now) correctly returns false
    assert!(!range.contains(now), "Future start time should NOT contain now");
}

#[test]
fn time_range_empty() {
    let now = Utc::now();
    let range = TimeRange::bounded(now, now); // Empty range [now, now)

    assert!(!range.contains(now), "Empty range should not contain start time");

    // is_current checks end > now.
    // Since end == now, is_current should be false.
    assert!(!range.is_current(), "Empty range ending now should not be current");
}

#[test]
fn bi_temporal_future_valid_time() {
    // Construct times explicitly to avoid race conditions
    let now = Utc::now();
    let future_start = now + Duration::days(1);
    let valid_range = TimeRange::starting_at(future_start);

    // Manually construct interval to control transaction time
    let interval = BiTemporalInterval {
        valid_time: valid_range,
        transaction_time: TimeRange::starting_at(now),
    };

    // valid_time starts in future.
    // transaction_time starts at 'now'.

    // is_current() checks if both ends are > now (or None).
    // valid_time.end is None -> true.
    // transaction_time.end is None -> true.
    assert!(interval.is_current(), "Interval with future valid time should be current (not superseded)");

    // But it should not be valid_at(now)
    assert!(!interval.valid_at(now), "Interval should not be valid at now");

    // It should be known_at(now)
    // Note: range.contains(start) is true.
    assert!(interval.known_at(now), "Interval should be known at now");
}
