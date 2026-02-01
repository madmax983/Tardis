//! # Tardis Kernel
//!
//! The core of Tardis OS - an AI-native operating system.
//!
//! This kernel provides:
//! - UEFI boot with memory management
//! - AI-aware process scheduling
//! - Syscall interface for Vortex, Gallifrey, and Chronos
//! - Hardware abstraction for CPU, GPU, and storage

#![no_std]
#![no_main]
#![feature(abi_efiapi)]
#![warn(missing_docs)]

extern crate alloc;

mod boot;
mod memory;
mod panic;

use core::fmt::Write;
use uefi::prelude::*;

/// Kernel entry point.
///
/// Called by UEFI firmware after loading the kernel image.
#[entry]
fn efi_main(image_handle: Handle, mut system_table: SystemTable<Boot>) -> Status {
    // Initialize UEFI services
    uefi::helpers::init(&mut system_table).expect("Failed to initialize UEFI helpers");

    // Clear screen and print banner
    system_table.stdout().clear().ok();
    print_banner(&mut system_table);

    // Get boot services
    let boot_services = system_table.boot_services();

    // Initialize memory subsystem
    log::info!("Initializing memory subsystem...");
    if let Err(e) = memory::init(boot_services) {
        log::error!("Failed to initialize memory: {:?}", e);
        return Status::ABORTED;
    }

    // Print memory map info
    print_memory_info(&mut system_table);

    // TODO: Initialize other subsystems
    // - Process scheduler
    // - IPC system
    // - Hardware abstraction layer
    // - Vortex service
    // - Gallifrey service
    // - Chronos service

    log::info!("Tardis kernel initialized successfully");
    log::info!("Entering idle loop...");

    // For now, just loop forever
    // In a real implementation, we would:
    // 1. Exit boot services
    // 2. Set up our own page tables
    // 3. Initialize the scheduler
    // 4. Start the init process (shell)
    loop {
        x86_64::instructions::hlt();
    }
}

/// Print the Tardis banner.
fn print_banner(st: &mut SystemTable<Boot>) {
    let stdout = st.stdout();

    let _ = writeln!(stdout);
    let _ = writeln!(
        stdout,
        "╔════════════════════════════════════════════════════════════╗"
    );
    let _ = writeln!(
        stdout,
        "║                                                            ║"
    );
    let _ = writeln!(
        stdout,
        "║              ████████╗ █████╗ ██████╗ ██████╗ ██╗███████╗  ║"
    );
    let _ = writeln!(
        stdout,
        "║              ╚══██╔══╝██╔══██╗██╔══██╗██╔══██╗██║██╔════╝  ║"
    );
    let _ = writeln!(
        stdout,
        "║                 ██║   ███████║██████╔╝██║  ██║██║███████╗  ║"
    );
    let _ = writeln!(
        stdout,
        "║                 ██║   ██╔══██║██╔══██╗██║  ██║██║╚════██║  ║"
    );
    let _ = writeln!(
        stdout,
        "║                 ██║   ██║  ██║██║  ██║██████╔╝██║███████║  ║"
    );
    let _ = writeln!(
        stdout,
        "║                 ╚═╝   ╚═╝  ╚═╝╚═╝  ╚═╝╚═════╝ ╚═╝╚══════╝  ║"
    );
    let _ = writeln!(
        stdout,
        "║                                                            ║"
    );
    let _ = writeln!(
        stdout,
        "║           AI-Native Operating System v0.1.0                ║"
    );
    let _ = writeln!(
        stdout,
        "║           Time And Relative Dimension In Space             ║"
    );
    let _ = writeln!(
        stdout,
        "║                                                            ║"
    );
    let _ = writeln!(
        stdout,
        "╚════════════════════════════════════════════════════════════╝"
    );
    let _ = writeln!(stdout);
}

/// Print memory information.
fn print_memory_info(st: &mut SystemTable<Boot>) {
    let stdout = st.stdout();

    let _ = writeln!(stdout, "Memory Information:");
    let _ = writeln!(stdout, "  Heap initialized: yes");
    let _ = writeln!(stdout, "  Allocator: linked list");
    let _ = writeln!(stdout);
}
