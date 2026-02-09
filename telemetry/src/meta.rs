//! Telemetry metadata and categorization.
//!
//! Provides enums for subsystem identification and event classification.
//! `no_std` compatible.

use core::fmt;
use serde::{Deserialize, Serialize};

/// Subsystem identifier for routing and filtering.
///
/// Identifies which component of Tardis OS generated the telemetry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u16)]
pub enum Subsystem {
    // Kernel subsystems (0-99)
    /// Core kernel functionality.
    Kernel = 0,
    /// Memory management.
    Memory = 1,
    /// Process scheduler.
    Scheduler = 2,
    /// Interrupt handling.
    Interrupt = 3,
    /// Syscall interface.
    Syscall = 4,
    /// Boot sequence.
    Boot = 5,

    // Userspace services (100-199)
    /// Vortex LLM inference engine.
    Vortex = 100,
    /// Gallifrey temporal database.
    Gallifrey = 101,
    /// Chronos RAG orchestration.
    Chronos = 102,
    /// Shell user interface.
    Shell = 103,
    /// Telemetry subsystem itself.
    Telemetry = 104,

    // External/Unknown (200+)
    /// Unknown or external subsystem.
    Unknown = 255,
}

/// Helper for case-insensitive substring check without allocation.
fn contains_ignore_case(haystack: &str, needle: &str) -> bool {
    let n_len = needle.len();
    if n_len == 0 {
        return true;
    }
    let h_len = haystack.len();
    if h_len < n_len {
        return false;
    }

    let haystack_bytes = haystack.as_bytes();
    let needle_bytes = needle.as_bytes();

    // Bolt: Manual loop optimization to avoid iterator overhead in hot path.
    // This is O(N*M) worst case but faster than iterator chain for small strings.
    let first_needle_byte = needle_bytes[0];
    for i in 0..=(h_len - n_len) {
        // Fast check for first byte to avoid inner loop setup
        if !haystack_bytes[i].eq_ignore_ascii_case(&first_needle_byte) {
            continue;
        }

        let mut match_found = true;
        for j in 1..n_len {
            if !haystack_bytes[i + j].eq_ignore_ascii_case(&needle_bytes[j]) {
                match_found = false;
                break;
            }
        }
        if match_found {
            return true;
        }
    }
    false
}

impl Subsystem {
    /// Returns the subsystem as a static string.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Kernel => "kernel",
            Self::Memory => "memory",
            Self::Scheduler => "scheduler",
            Self::Interrupt => "interrupt",
            Self::Syscall => "syscall",
            Self::Boot => "boot",
            Self::Vortex => "vortex",
            Self::Gallifrey => "gallifrey",
            Self::Chronos => "chronos",
            Self::Shell => "shell",
            Self::Telemetry => "telemetry",
            Self::Unknown => "unknown",
        }
    }

    /// Parses a subsystem from a target string.
    #[must_use]
    pub fn from_target(target: &str) -> Self {
        const MAPPINGS: &[(&str, Subsystem)] = &[
            ("kernel", Subsystem::Kernel),
            ("memory", Subsystem::Memory),
            ("heap", Subsystem::Memory),
            ("scheduler", Subsystem::Scheduler),
            ("process", Subsystem::Scheduler),
            ("interrupt", Subsystem::Interrupt),
            ("irq", Subsystem::Interrupt),
            ("syscall", Subsystem::Syscall),
            ("boot", Subsystem::Boot),
            ("vortex", Subsystem::Vortex),
            ("gallifrey", Subsystem::Gallifrey),
            ("chronos", Subsystem::Chronos),
            ("shell", Subsystem::Shell),
            ("tardis", Subsystem::Shell),
            ("telemetry", Subsystem::Telemetry),
        ];

        // Warden: Zero-allocation case-insensitive search.
        // Critical for safety in restricted contexts (interrupts, no_std).
        for (key, subsystem) in MAPPINGS {
            if contains_ignore_case(target, key) {
                return *subsystem;
            }
        }

        Self::Unknown
    }
}

impl fmt::Display for Subsystem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<u16> for Subsystem {
    fn from(value: u16) -> Self {
        match value {
            0 => Self::Kernel,
            1 => Self::Memory,
            2 => Self::Scheduler,
            3 => Self::Interrupt,
            4 => Self::Syscall,
            5 => Self::Boot,
            100 => Self::Vortex,
            101 => Self::Gallifrey,
            102 => Self::Chronos,
            103 => Self::Shell,
            104 => Self::Telemetry,
            _ => Self::Unknown,
        }
    }
}

/// Event type within a subsystem.
///
/// Provides fine-grained categorization of telemetry events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u16)]
pub enum EventType {
    // Lifecycle events (0-99)
    /// Span started.
    SpanStart = 0,
    /// Span ended.
    SpanEnd = 1,
    /// Log message.
    Log = 2,
    /// Metric sample.
    Metric = 3,

    // Kernel events (100-199)
    /// Boot sequence event.
    Boot = 100,
    /// Kernel panic.
    Panic = 101,
    /// Syscall entry.
    SyscallEntry = 102,
    /// Syscall exit.
    SyscallExit = 103,
    /// Page fault.
    PageFault = 104,
    /// Interrupt received.
    InterruptReceived = 105,
    /// Context switch.
    ContextSwitch = 106,

    // Vortex events (200-299)
    /// Model loading started.
    ModelLoad = 200,
    /// Model unloaded.
    ModelUnload = 201,
    /// Inference started.
    InferenceStart = 202,
    /// Inference completed.
    InferenceEnd = 203,
    /// Token generated.
    TokenGenerated = 204,
    /// Embedding computed.
    EmbeddingComputed = 205,

    // Gallifrey events (300-399)
    /// Query started.
    QueryStart = 300,
    /// Query completed.
    QueryEnd = 301,
    /// Entity inserted.
    EntityInsert = 302,
    /// Entity updated.
    EntityUpdate = 303,
    /// Time travel query.
    TimeTravel = 304,
    /// Snapshot taken.
    SnapshotTaken = 305,

    // Chronos events (400-499)
    /// RAG query started.
    RagQuery = 400,
    /// Retrieval operation.
    Retrieval = 401,
    /// Context augmentation.
    Augmentation = 402,
    /// Memory consolidation.
    Consolidation = 403,

    // Shell events (500-599)
    /// User input received.
    UserInput = 500,
    /// Intent classified.
    IntentClassified = 501,
    /// Response generated.
    ResponseGenerated = 502,

    // Unknown
    /// Unknown event type.
    Unknown = 65535,
}

impl EventType {
    /// Returns the event type as a static string.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::SpanStart => "span_start",
            Self::SpanEnd => "span_end",
            Self::Log => "log",
            Self::Metric => "metric",
            Self::Boot => "boot",
            Self::Panic => "panic",
            Self::SyscallEntry => "syscall_entry",
            Self::SyscallExit => "syscall_exit",
            Self::PageFault => "page_fault",
            Self::InterruptReceived => "interrupt",
            Self::ContextSwitch => "context_switch",
            Self::ModelLoad => "model_load",
            Self::ModelUnload => "model_unload",
            Self::InferenceStart => "inference_start",
            Self::InferenceEnd => "inference_end",
            Self::TokenGenerated => "token_generated",
            Self::EmbeddingComputed => "embedding_computed",
            Self::QueryStart => "query_start",
            Self::QueryEnd => "query_end",
            Self::EntityInsert => "entity_insert",
            Self::EntityUpdate => "entity_update",
            Self::TimeTravel => "time_travel",
            Self::SnapshotTaken => "snapshot_taken",
            Self::RagQuery => "rag_query",
            Self::Retrieval => "retrieval",
            Self::Augmentation => "augmentation",
            Self::Consolidation => "consolidation",
            Self::UserInput => "user_input",
            Self::IntentClassified => "intent_classified",
            Self::ResponseGenerated => "response_generated",
            Self::Unknown => "unknown",
        }
    }
}

impl fmt::Display for EventType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<u16> for EventType {
    fn from(value: u16) -> Self {
        match value {
            0 => Self::SpanStart,
            1 => Self::SpanEnd,
            2 => Self::Log,
            3 => Self::Metric,
            100 => Self::Boot,
            101 => Self::Panic,
            102 => Self::SyscallEntry,
            103 => Self::SyscallExit,
            104 => Self::PageFault,
            105 => Self::InterruptReceived,
            106 => Self::ContextSwitch,
            200 => Self::ModelLoad,
            201 => Self::ModelUnload,
            202 => Self::InferenceStart,
            203 => Self::InferenceEnd,
            204 => Self::TokenGenerated,
            205 => Self::EmbeddingComputed,
            300 => Self::QueryStart,
            301 => Self::QueryEnd,
            302 => Self::EntityInsert,
            303 => Self::EntityUpdate,
            304 => Self::TimeTravel,
            305 => Self::SnapshotTaken,
            400 => Self::RagQuery,
            401 => Self::Retrieval,
            402 => Self::Augmentation,
            403 => Self::Consolidation,
            500 => Self::UserInput,
            501 => Self::IntentClassified,
            502 => Self::ResponseGenerated,
            _ => Self::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subsystem_from_target() {
        assert_eq!(
            Subsystem::from_target("tardis_vortex::inference"),
            Subsystem::Vortex
        );
        assert_eq!(
            Subsystem::from_target("tardis_gallifrey::stores"),
            Subsystem::Gallifrey
        );
        assert_eq!(
            Subsystem::from_target("memory::allocator"),
            Subsystem::Memory
        );
        assert_eq!(
            Subsystem::from_target("tardis_kernel::core"),
            Subsystem::Kernel
        );
        assert_eq!(Subsystem::from_target("random_crate"), Subsystem::Unknown);
    }

    #[test]
    fn subsystem_from_target_comprehensive() {
        let cases = [
            ("kernel_panic", Subsystem::Kernel),
            ("memory_alloc", Subsystem::Memory),
            ("heap_allocator", Subsystem::Memory),
            ("scheduler_tick", Subsystem::Scheduler),
            ("process_manager", Subsystem::Scheduler),
            ("interrupt_handler", Subsystem::Interrupt),
            ("irq_controller", Subsystem::Interrupt),
            ("syscall_dispatch", Subsystem::Syscall),
            ("boot_loader", Subsystem::Boot),
            ("vortex_inference", Subsystem::Vortex),
            ("gallifrey_store", Subsystem::Gallifrey),
            ("chronos_rag", Subsystem::Chronos),
            ("shell_repl", Subsystem::Shell),
            ("tardis_cli", Subsystem::Shell),
            ("telemetry_drainer", Subsystem::Telemetry),
            ("unknown_module", Subsystem::Unknown),
        ];

        for (input, expected) in cases {
            assert_eq!(
                Subsystem::from_target(input),
                expected,
                "Failed for input: {input}"
            );
        }
    }

    #[test]
    fn subsystem_from_target_case_insensitive() {
        assert_eq!(Subsystem::from_target("KERNEL"), Subsystem::Kernel);
        assert_eq!(Subsystem::from_target("MeMoRy"), Subsystem::Memory);
        assert_eq!(Subsystem::from_target("VORtex"), Subsystem::Vortex);
    }

    #[test]
    fn subsystem_precedence() {
        // "kernel" is checked before "memory"
        assert_eq!(Subsystem::from_target("kernel_memory"), Subsystem::Kernel);
        // "telemetry" is checked last (before Unknown)
        assert_eq!(
            Subsystem::from_target("telemetry_kernel"),
            Subsystem::Kernel
        );
    }
}
