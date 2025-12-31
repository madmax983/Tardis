//! Kernel heap management.
//!
//! Provides dynamic memory allocation for the kernel using a linked list allocator.
//! This is a simple allocator suitable for early boot; may be replaced with a more
//! sophisticated allocator (e.g., slab, buddy) later.

use alloc::alloc::{GlobalAlloc, Layout};
use core::ptr::NonNull;

/// Align a value up to the given alignment.
pub const fn align_up(value: usize, align: usize) -> usize {
    (value + align - 1) & !(align - 1)
}

/// Heap statistics for debugging.
#[derive(Debug, Clone, Copy, Default)]
pub struct HeapStats {
    /// Total heap size in bytes.
    pub total_size: usize,
    /// Currently allocated bytes.
    pub allocated: usize,
    /// Number of allocations.
    pub allocation_count: usize,
    /// Number of deallocations.
    pub deallocation_count: usize,
}

/// A simple bump allocator for very early boot.
///
/// This allocator cannot free memory - it only bumps a pointer.
/// Used only during the earliest boot phase before the real heap is set up.
pub struct BumpAllocator {
    heap_start: usize,
    heap_end: usize,
    next: usize,
    allocations: usize,
}

impl BumpAllocator {
    /// Create a new bump allocator.
    ///
    /// # Safety
    ///
    /// The caller must ensure the memory region is valid and not used elsewhere.
    pub const unsafe fn new(heap_start: usize, heap_size: usize) -> Self {
        Self {
            heap_start,
            heap_end: heap_start + heap_size,
            next: heap_start,
            allocations: 0,
        }
    }

    /// Allocate memory with the given layout.
    pub fn allocate(&mut self, layout: Layout) -> Option<NonNull<u8>> {
        let alloc_start = align_up(self.next, layout.align());
        let alloc_end = alloc_start.checked_add(layout.size())?;

        if alloc_end > self.heap_end {
            return None; // Out of memory
        }

        self.next = alloc_end;
        self.allocations += 1;

        Some(unsafe { NonNull::new_unchecked(alloc_start as *mut u8) })
    }

    /// Get the number of allocations.
    pub const fn allocations(&self) -> usize {
        self.allocations
    }

    /// Get the remaining space.
    pub const fn remaining(&self) -> usize {
        self.heap_end - self.next
    }
}

unsafe impl GlobalAlloc for BumpAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // This is a bit of a hack - we need interior mutability
        // In practice, the bump allocator is only used in single-threaded early boot
        let this = self as *const Self as *mut Self;
        (*this)
            .allocate(layout)
            .map_or(core::ptr::null_mut(), |p| p.as_ptr())
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // Bump allocator cannot free - this is intentional
        // Memory is reclaimed when we switch to the real allocator
    }
}
