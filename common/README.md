# Tardis Common 🔧

> "It's the sonic screwdriver of the operating system. It does... things."

**Common** is the shared utility crate for Tardis OS. It provides the core types, traits, and error handling mechanisms used by all other subsystems (`chronos`, `gallifrey`, `vortex`, `shell`).

## 📦 Modules

-   **`error`**: Defines the unified `Error` type and `Result` alias for cross-crate compatibility.
-   **`id`**: Strongly-typed identifiers for system entities:
    -   `EntityId`: Unique ID for knowledge graph nodes (UUID v4).
    -   `SessionId`: ID for user sessions.
    -   `SnapshotId`: ID for system state snapshots.
    -   `ModelHandle`: Lightweight handle for loaded LLM models.
-   **`temporal`**: Primitives for bi-temporal data management.

## ⏳ Bi-Temporal Primitives

The crown jewel of this crate is the **`BiTemporalInterval`**, which enables `Gallifrey` to track history accurately.

```rust
use tardis_common::temporal::{BiTemporalInterval, TimeRange};
use chrono::{Utc, TimeZone};

// Create an interval representing "Now" in both dimensions
let interval = BiTemporalInterval::now();

// Create a historical interval
// Valid: 2020-2022
// Transaction: Recorded today
let historical = BiTemporalInterval {
    valid_time: TimeRange::closed(
        Utc.ymd(2020, 1, 1).and_hms(0, 0, 0),
        Utc.ymd(2022, 1, 1).and_hms(0, 0, 0),
    ),
    transaction_time: TimeRange::open_from(Utc::now()),
};
```

## 🤝 Usage

This crate is a workspace dependency.

```toml
[dependencies]
tardis-common = { path = "../common" }
```
