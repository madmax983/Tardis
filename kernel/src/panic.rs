//! Panic handler for the kernel.
//!
//! Provides a panic handler that prints diagnostic information
//! and halts the CPU.

use core::panic::PanicInfo;

/// Kernel panic handler.
///
/// This is called when the kernel panics. It prints the panic message
/// and location, then halts the CPU.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // Log the panic if logging is available
    if let Some(location) = info.location() {
        log::error!(
            "KERNEL PANIC at {}:{}:{}",
            location.file(),
            location.line(),
            location.column()
        );
    } else {
        log::error!("KERNEL PANIC at unknown location");
    }

    if let Some(message) = info.message() {
        log::error!("  Message: {}", message);
    }

    // Halt the CPU
    halt_loop();
}

/// Halt the CPU in an infinite loop.
///
/// This disables interrupts and repeatedly halts, ensuring
/// the CPU doesn't continue executing after a panic.
fn halt_loop() -> ! {
    loop {
        x86_64::instructions::interrupts::disable();
        x86_64::instructions::hlt();
    }
}

/// Kernel assertion macro.
///
/// Like `assert!` but designed for kernel use - provides better
/// diagnostics and guarantees a halt on failure.
#[macro_export]
macro_rules! kernel_assert {
    ($cond:expr) => {
        if !$cond {
            panic!("kernel assertion failed: {}", stringify!($cond));
        }
    };
    ($cond:expr, $($arg:tt)+) => {
        if !$cond {
            panic!("kernel assertion failed: {}: {}", stringify!($cond), format_args!($($arg)+));
        }
    };
}

/// Kernel unreachable marker.
///
/// Marks code that should never be reached. If reached, panics
/// with diagnostic information.
#[macro_export]
macro_rules! kernel_unreachable {
    () => {
        panic!("entered unreachable code")
    };
    ($($arg:tt)+) => {
        panic!("entered unreachable code: {}", format_args!($($arg)+))
    };
}
