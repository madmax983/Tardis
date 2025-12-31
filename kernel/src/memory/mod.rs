//! Memory management subsystem.
//!
//! Provides:
//! - Physical memory allocation (buddy allocator)
//! - Virtual memory management (page tables)
//! - Kernel heap allocation
//! - Huge page support for AI workloads

mod heap;

use linked_list_allocator::LockedHeap;
use uefi::table::boot::BootServices;

/// Kernel heap size: 16 MB initially
const KERNEL_HEAP_SIZE: usize = 16 * 1024 * 1024;

/// Global kernel heap allocator.
#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

/// Initialize the memory subsystem.
///
/// # Errors
///
/// Returns an error if memory allocation fails.
pub fn init(boot_services: &BootServices) -> Result<(), MemoryError> {
    // Allocate pages for kernel heap
    let heap_pages = (KERNEL_HEAP_SIZE + 4095) / 4096;

    let heap_start = boot_services
        .allocate_pages(
            uefi::table::boot::AllocateType::AnyPages,
            uefi::table::boot::MemoryType::LOADER_DATA,
            heap_pages,
        )
        .map_err(|_| MemoryError::AllocationFailed)?;

    // Initialize the heap allocator
    unsafe {
        ALLOCATOR
            .lock()
            .init(heap_start as *mut u8, KERNEL_HEAP_SIZE);
    }

    log::info!(
        "Kernel heap initialized: {} MB at {:#x}",
        KERNEL_HEAP_SIZE / 1024 / 1024,
        heap_start
    );

    Ok(())
}

/// Memory subsystem errors.
#[derive(Debug, Clone, Copy)]
pub enum MemoryError {
    /// Memory allocation failed.
    AllocationFailed,
    /// Invalid memory address.
    InvalidAddress,
    /// Out of memory.
    OutOfMemory,
    /// Page table error.
    PageTableError,
}

/// Physical memory frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhysFrame {
    /// Physical address of the frame.
    pub addr: u64,
}

impl PhysFrame {
    /// Standard page size: 4 KB
    pub const SIZE_4K: u64 = 4096;
    /// Huge page size: 2 MB
    pub const SIZE_2M: u64 = 2 * 1024 * 1024;
    /// Giant page size: 1 GB
    pub const SIZE_1G: u64 = 1024 * 1024 * 1024;

    /// Create a new physical frame at the given address.
    ///
    /// # Safety
    ///
    /// The address must be page-aligned and valid.
    pub const unsafe fn new(addr: u64) -> Self {
        Self { addr }
    }

    /// Check if this is a valid frame address.
    pub const fn is_aligned(&self, size: u64) -> bool {
        self.addr % size == 0
    }
}

/// Virtual address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VirtAddr(u64);

impl VirtAddr {
    /// Create a new virtual address.
    pub const fn new(addr: u64) -> Self {
        Self(addr)
    }

    /// Get the raw address value.
    pub const fn as_u64(&self) -> u64 {
        self.0
    }

    /// Get the page offset (lower 12 bits).
    pub const fn page_offset(&self) -> u64 {
        self.0 & 0xFFF
    }

    /// Get the P4 index (bits 39-47).
    pub const fn p4_index(&self) -> usize {
        ((self.0 >> 39) & 0x1FF) as usize
    }

    /// Get the P3 index (bits 30-38).
    pub const fn p3_index(&self) -> usize {
        ((self.0 >> 30) & 0x1FF) as usize
    }

    /// Get the P2 index (bits 21-29).
    pub const fn p2_index(&self) -> usize {
        ((self.0 >> 21) & 0x1FF) as usize
    }

    /// Get the P1 index (bits 12-20).
    pub const fn p1_index(&self) -> usize {
        ((self.0 >> 12) & 0x1FF) as usize
    }
}
