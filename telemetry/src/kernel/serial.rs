//! Serial port output for kernel telemetry.
//!
//! # Overview
//!
//! This module provides a direct interface to the PC serial port (COM1),
//! primarily used for:
//! 1.  **Early Boot Logging**: Emitting messages before the memory allocator or ring buffer are initialized.
//! 2.  **Panic Handling**: Reliability dumping panic information when the system state is corrupted.
//! 3.  **Fallback Logging**: Providing an output channel when the telemetry ring buffer is full.
//!
//! # Safety & Implementation
//!
//! This module interacts directly with hardware I/O ports (specifically `0x3F8` for COM1).
//!
//! -   **Unsafe Operations**: All functions are marked `unsafe` because they perform raw I/O
//!     and rely on the hardware being present and correctly mapped.
//! -   **Kernel Context**: These functions should *only* be called when running in kernel mode (ring 0).
//! -   **Synchronization**: The implementation assumes it is the sole owner of the serial port.
//!     While `init` has a basic atomic guard, concurrent writes from multiple cores are NOT locked
//!     (to prevent deadlocks in panic handlers) and may result in interleaved output.
//!
//! # Configuration
//!
//! The `init()` function configures the UART to:
//! -   **Baud Rate**: 115200
//! -   **Data Bits**: 8
//! -   **Parity**: None
//! -   **Stop Bits**: 1
//! -   **FIFO**: Enabled (14-byte threshold)

use core::sync::atomic::{AtomicBool, Ordering};

/// COM1 base I/O port address.
#[cfg(target_arch = "x86_64")]
#[allow(dead_code)]
const COM1_PORT: u16 = 0x3F8;

/// Whether serial port has been initialized.
static SERIAL_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Initializes the serial port.
///
/// Configures COM1 for 115200 baud, 8N1.
///
/// # Safety
///
/// This function performs raw port I/O and should only be called
/// during kernel initialization.
///
/// It checks `SERIAL_INITIALIZED` to prevent double-initialization,
/// but callers must ensure no other code is accessing the serial port hardware.
#[cfg(target_arch = "x86_64")]
#[allow(clippy::needless_return)]
pub unsafe fn init() {
    if SERIAL_INITIALIZED.swap(true, Ordering::SeqCst) {
        return; // Already initialized
    }

    // In actual kernel, we would use x86_64 port I/O here
    // For compilation in userspace tests, we skip the actual I/O

    #[cfg(all(feature = "kernel", not(test)))]
    // SAFETY: We are initializing the standard PC serial port (COM1).
    // This requires direct I/O port access which is unsafe.
    // This is safe to call during kernel initialization as we have
    // exclusive access to the hardware.
    unsafe {
        use x86_64::instructions::port::Port;

        let mut port = Port::new(COM1_PORT);

        // Disable interrupts
        let mut ier: Port<u8> = Port::new(COM1_PORT + 1);
        ier.write(0x00);

        // Enable DLAB (set baud rate divisor)
        let mut lcr: Port<u8> = Port::new(COM1_PORT + 3);
        lcr.write(0x80);

        // Set divisor to 1 (115200 baud)
        // Divisor = 115200 / 115200 = 1
        port.write(0x01u8);
        ier.write(0x00);

        // 8 bits, no parity, one stop bit (8N1)
        lcr.write(0x03);

        // Enable FIFO, clear them, with 14-byte threshold
        let mut fcr: Port<u8> = Port::new(COM1_PORT + 2);
        fcr.write(0xC7);

        // IRQs enabled, RTS/DSR set
        let mut mcr: Port<u8> = Port::new(COM1_PORT + 4);
        mcr.write(0x0B);

        // Enable interrupts
        ier.write(0x01);
    }
}

/// Initializes the serial port (non-x86_64 stub).
///
/// # Safety
///
/// Safe on non-x86_64 targets as it is a no-op, but marked unsafe for API consistency.
#[cfg(not(target_arch = "x86_64"))]
pub unsafe fn init() {
    SERIAL_INITIALIZED.store(true, Ordering::SeqCst);
}

/// Writes a byte to the serial port.
///
/// # Safety
///
/// This function performs port I/O and should only be called in kernel context.
/// It assumes the port has been initialized via `init()`.
#[cfg(all(target_arch = "x86_64", feature = "kernel"))]
#[allow(clippy::needless_return)]
pub unsafe fn write_byte(byte: u8) {
    // Suppress unused variable warning for tests where the unsafe block is cfg-gated out
    #[cfg(test)]
    let _ = byte;

    if !SERIAL_INITIALIZED.load(Ordering::Acquire) {
        return;
    }

    #[cfg(not(test))]
    // SAFETY: We are writing to the standard PC serial port (COM1).
    // This requires direct I/O port access which is unsafe.
    // We check the Line Status Register (LSR) to ensure the transmit
    // buffer is empty before writing, preventing data corruption.
    unsafe {
        use x86_64::instructions::port::Port;

        // Wait for transmit buffer to be empty
        // Bit 5 (0x20) of LSR indicates "Transmitter Holding Register Empty"
        let mut lsr: Port<u8> = Port::new(COM1_PORT + 5);
        while (lsr.read() & 0x20) == 0 {
            core::hint::spin_loop();
        }

        // Write the byte
        let mut data: Port<u8> = Port::new(COM1_PORT);
        data.write(byte);
    }
}

/// Writes a byte to the serial port (stub for non-kernel or non-x86_64).
///
/// # Safety
///
/// Safe as it is a no-op, but marked unsafe for API consistency.
#[cfg(not(all(target_arch = "x86_64", feature = "kernel")))]
pub unsafe fn write_byte(_byte: u8) {
    // No-op in non-kernel builds
}

/// Writes a string to the serial port.
///
/// # Safety
///
/// This function calls `write_byte` which performs port I/O.
pub unsafe fn write_str(s: &str) {
    for byte in s.bytes() {
        if byte == b'\n' {
            unsafe { write_byte(b'\r') }; // CR before LF for terminal compatibility
        }
        unsafe { write_byte(byte) };
    }
}

/// Writes bytes to the serial port.
///
/// # Safety
///
/// This function calls `write_byte` which performs port I/O.
pub unsafe fn write_bytes(bytes: &[u8]) {
    for &byte in bytes {
        unsafe { write_byte(byte) };
    }
}

/// Writes a formatted message to the serial port.
///
/// This is useful for early boot messages before the full logging
/// infrastructure is available.
///
/// # Safety
///
/// This function calls `write_str` which performs port I/O.
pub unsafe fn write_line(prefix: &str, message: &str) {
    unsafe {
        write_str(prefix);
        write_str(": ");
        write_str(message);
        write_str("\n");
    }
}

/// Writes a panic message to the serial port.
///
/// This formats the panic information for serial output and should
/// be called from the panic handler.
///
/// # Safety
///
/// This function calls `write_str` which performs port I/O.
/// It is designed to be minimal and robust for use in crash conditions.
pub unsafe fn write_panic(message: &str, file: &str, line: u32) {
    unsafe {
        write_str("\n!!! KERNEL PANIC !!!\n");
        write_str("Message: ");
        write_str(message);
        write_str("\nLocation: ");
        write_str(file);
        write_str(":");
        write_u32(line);
        write_str("\n\n");
    }
}

/// Writes an unsigned 32-bit integer to serial.
///
/// # Safety
///
/// This function calls `write_byte` which performs port I/O.
unsafe fn write_u32(mut value: u32) {
    if value == 0 {
        unsafe { write_byte(b'0') };
        return;
    }

    let mut buf = [0u8; 10];
    let mut i = 0;

    while value > 0 {
        buf[i] = b'0' + (value % 10) as u8;
        value /= 10;
        i += 1;
    }

    while i > 0 {
        i -= 1;
        unsafe { write_byte(buf[i]) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serial_init_idempotent() {
        // Multiple init calls should be safe
        unsafe {
            init();
            init();
        }
        assert!(SERIAL_INITIALIZED.load(Ordering::Relaxed));
    }
}
