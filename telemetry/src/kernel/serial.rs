//! Serial port output for kernel telemetry.
//!
//! Provides direct serial port (COM1) output for:
//! - Early boot messages before ring buffer is available
//! - Panic messages when ring buffer may be corrupted
//! - Debug output when configured
//!
//! # Hardware
//!
//! Uses the standard PC COM1 port at I/O address 0x3F8 with 8N1 configuration.
//!
//! # Safety
//!
//! This module uses port I/O which requires kernel-level access.
//! In userspace builds, these functions are no-ops.

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
/// This function performs port I/O and should only be called
/// during kernel initialization.
#[cfg(target_arch = "x86_64")]
#[allow(clippy::needless_return)]
pub fn init() {
    if SERIAL_INITIALIZED.swap(true, Ordering::SeqCst) {
        return; // Already initialized
    }

    // In actual kernel, we would use x86_64 port I/O here
    // For compilation in userspace tests, we skip the actual I/O

    #[cfg(all(feature = "kernel", not(test)))]
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
#[cfg(not(target_arch = "x86_64"))]
pub fn init() {
    SERIAL_INITIALIZED.store(true, Ordering::SeqCst);
}

/// Writes a byte to the serial port.
#[cfg(all(target_arch = "x86_64", feature = "kernel"))]
#[allow(clippy::needless_return)]
pub fn write_byte(byte: u8) {
    // Suppress unused variable warning for tests where the unsafe block is cfg-gated out
    #[cfg(test)]
    let _ = byte;

    if !SERIAL_INITIALIZED.load(Ordering::Acquire) {
        return;
    }

    #[cfg(not(test))]
    unsafe {
        use x86_64::instructions::port::Port;

        // Wait for transmit buffer to be empty
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
#[cfg(not(all(target_arch = "x86_64", feature = "kernel")))]
pub fn write_byte(_byte: u8) {
    // No-op in non-kernel builds
}

/// Writes a string to the serial port.
pub fn write_str(s: &str) {
    for byte in s.bytes() {
        if byte == b'\n' {
            write_byte(b'\r'); // CR before LF for terminal compatibility
        }
        write_byte(byte);
    }
}

/// Writes bytes to the serial port.
pub fn write_bytes(bytes: &[u8]) {
    for &byte in bytes {
        write_byte(byte);
    }
}

/// Writes a formatted message to the serial port.
///
/// This is useful for early boot messages before the full logging
/// infrastructure is available.
pub fn write_line(prefix: &str, message: &str) {
    write_str(prefix);
    write_str(": ");
    write_str(message);
    write_str("\n");
}

/// Writes a panic message to the serial port.
///
/// This formats the panic information for serial output and should
/// be called from the panic handler.
pub fn write_panic(message: &str, file: &str, line: u32) {
    write_str("\n!!! KERNEL PANIC !!!\n");
    write_str("Message: ");
    write_str(message);
    write_str("\nLocation: ");
    write_str(file);
    write_str(":");
    write_u32(line);
    write_str("\n\n");
}

/// Writes an unsigned 32-bit integer to serial.
fn write_u32(mut value: u32) {
    if value == 0 {
        write_byte(b'0');
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
        write_byte(buf[i]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serial_init_idempotent() {
        // Multiple init calls should be safe
        init();
        init();
        assert!(SERIAL_INITIALIZED.load(Ordering::Relaxed));
    }
}
