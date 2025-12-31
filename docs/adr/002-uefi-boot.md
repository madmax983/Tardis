# ADR-002: UEFI Boot Process

## Status

Accepted

## Date

2024-12-31

## Context

The kernel needs a boot mechanism to load into memory, set up initial page tables, and transfer control to Rust code. We need to choose between UEFI (modern firmware interface) and legacy BIOS boot.

Requirements:
- Support for modern hardware (NVMe, USB 3.0, modern GPUs)
- Access to system memory map
- Early graphics output for debugging
- Secure boot capability (future)

## Decision

We will use UEFI boot with the `uefi-rs` crate for the boot process.

## Consequences

### Positive

- **Modern hardware support**: UEFI provides native support for NVMe, USB 3.0, and modern GPUs during boot
- **Rich boot services**: Memory allocation, file system access, and graphics output available before kernel takes over
- **Memory map access**: UEFI provides accurate physical memory map including reserved regions
- **GOP (Graphics Output Protocol)**: Early graphics without needing VGA/VBE fallbacks
- **Future secure boot**: Can implement secure boot chain for trusted kernel loading
- **64-bit from start**: No need for real mode → protected mode → long mode transitions

### Negative

- **No legacy hardware support**: Systems without UEFI (pre-2010 roughly) cannot boot Tardis
- **UEFI complexity**: UEFI specification is large and complex
- **ExitBootServices transition**: Careful handling required when transitioning from UEFI to kernel control
- **Testing requires OVMF**: Need UEFI firmware image for QEMU testing

### Neutral

- Must implement own drivers after ExitBootServices (UEFI drivers unavailable)
- UEFI runtime services available but limited in usefulness

## Alternatives Considered

### Alternative 1: Legacy BIOS Boot

Use traditional BIOS boot with a custom bootloader or GRUB.

**Pros:**
- Works on older hardware
- Simpler boot process
- Well-documented (decades of resources)

**Cons:**
- Real mode limitations (16-bit, 1MB memory)
- Complex transition to 64-bit long mode
- No native NVMe support
- VGA/VBE for graphics (limited resolution, complexity)

**Why not chosen:** Legacy BIOS adds significant complexity for diminishing returns. Modern AI hardware (GPUs with large VRAM) requires modern systems that have UEFI.

### Alternative 2: Multiboot2 via GRUB

Use GRUB as bootloader with Multiboot2 specification.

**Pros:**
- GRUB handles boot complexity
- Works with both BIOS and UEFI
- Familiar to Linux users

**Cons:**
- Less control over boot process
- GRUB dependency
- Multiboot2 has limitations on memory layout control

**Why not chosen:** We want full control over the boot process for future secure boot and custom memory layout optimizations.

### Alternative 3: Custom Bootloader

Write a completely custom bootloader from scratch.

**Pros:**
- Total control
- No external dependencies

**Cons:**
- Significant additional development effort
- Must handle all hardware variations
- Reinventing well-solved problems

**Why not chosen:** The `uefi-rs` crate provides excellent UEFI support in Rust, making a custom bootloader unnecessary.

## References

- [UEFI Specification](https://uefi.org/specifications)
- [uefi-rs crate](https://github.com/rust-osdev/uefi-rs)
- [OVMF (UEFI for QEMU)](https://github.com/tianocore/tianocore.github.io/wiki/OVMF)
- [Writing an OS in Rust - UEFI](https://os.phil-opp.com/)
