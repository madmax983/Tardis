//! Boot services and initialization.
//!
//! Handles UEFI boot services, memory map acquisition, and
//! transition to kernel control.

use uefi::prelude::*;
use uefi::table::boot::{MemoryDescriptor, MemoryType};

/// Boot configuration collected during UEFI boot phase.
#[derive(Debug)]
pub struct BootConfig {
    /// Total usable memory in bytes.
    pub total_memory: u64,
    /// Memory map from UEFI.
    pub memory_map_size: usize,
    /// Framebuffer address (if available).
    pub framebuffer_addr: Option<u64>,
    /// Framebuffer dimensions.
    pub framebuffer_width: u32,
    pub framebuffer_height: u32,
}

impl BootConfig {
    /// Create a new boot configuration.
    pub const fn new() -> Self {
        Self {
            total_memory: 0,
            memory_map_size: 0,
            framebuffer_addr: None,
            framebuffer_width: 0,
            framebuffer_height: 0,
        }
    }
}

/// Calculate total usable memory from UEFI memory map.
pub fn calculate_usable_memory(descriptors: impl Iterator<Item = &MemoryDescriptor>) -> u64 {
    descriptors
        .filter(|desc| is_usable_memory(desc.ty))
        .map(|desc| desc.page_count * 4096) // 4KB pages
        .sum()
}

/// Check if a memory type is usable for general allocation.
const fn is_usable_memory(ty: MemoryType) -> bool {
    matches!(
        ty,
        MemoryType::CONVENTIONAL
            | MemoryType::BOOT_SERVICES_CODE
            | MemoryType::BOOT_SERVICES_DATA
            | MemoryType::LOADER_CODE
            | MemoryType::LOADER_DATA
    )
}

/// Memory region types for kernel use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryRegionType {
    /// Usable conventional memory.
    Usable,
    /// Reserved by firmware.
    Reserved,
    /// ACPI reclaimable.
    AcpiReclaimable,
    /// ACPI NVS.
    AcpiNvs,
    /// Bad memory.
    Bad,
    /// Kernel code/data.
    Kernel,
    /// Framebuffer.
    Framebuffer,
}

impl From<MemoryType> for MemoryRegionType {
    fn from(ty: MemoryType) -> Self {
        match ty {
            MemoryType::CONVENTIONAL
            | MemoryType::BOOT_SERVICES_CODE
            | MemoryType::BOOT_SERVICES_DATA
            | MemoryType::LOADER_CODE
            | MemoryType::LOADER_DATA => Self::Usable,
            MemoryType::ACPI_RECLAIM => Self::AcpiReclaimable,
            MemoryType::ACPI_NON_VOLATILE => Self::AcpiNvs,
            MemoryType::UNUSABLE => Self::Bad,
            MemoryType::RUNTIME_SERVICES_CODE | MemoryType::RUNTIME_SERVICES_DATA => Self::Reserved,
            _ => Self::Reserved,
        }
    }
}
