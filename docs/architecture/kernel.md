# Kernel Architecture

## Overview

The Tardis kernel is a from-scratch Rust kernel optimized for AI workloads. It provides memory management, process scheduling, IPC, and hardware abstraction with special consideration for large model weights and GPU compute.

## Boot Process

```
UEFI Firmware
     │
     ▼
┌─────────────┐
│ Bootloader  │  - Uses uefi-rs crate
│             │  - Gets memory map from UEFI
│             │  - Sets up initial page tables
│             │  - Loads kernel ELF
└──────┬──────┘
       │
       ▼
┌─────────────┐
│ Kernel Init │  - Switch to kernel page tables
│             │  - Initialize heap allocator
│             │  - Set up interrupt handlers
│             │  - Initialize scheduler
└──────┬──────┘
       │
       ▼
┌─────────────┐
│ Driver Init │  - PCI enumeration
│             │  - GPU driver (CUDA)
│             │  - Storage driver (NVMe)
│             │  - Console driver
└──────┬──────┘
       │
       ▼
┌─────────────┐
│ Service Init│  - Start Vortex service
│             │  - Start Gallifrey service
│             │  - Start Chronos service
└──────┬──────┘
       │
       ▼
   User Shell
```

## Memory Management

### Physical Memory Manager

```rust
pub struct PhysicalMemoryManager {
    /// Buddy allocator for general allocations
    buddy: BuddyAllocator,
    /// Huge page pool (2MB pages)
    huge_2mb: HugePagePool,
    /// Giant page pool (1GB pages) for large models
    huge_1gb: HugePagePool,
    /// NUMA topology information
    numa: NumaTopology,
}
```

### Allocation Strategy

| Allocation Type | Page Size | Use Case |
|-----------------|-----------|----------|
| Small (< 4KB) | 4KB | General kernel data |
| Medium (4KB-2MB) | 4KB | Process memory |
| Large (2MB-1GB) | 2MB | KV cache, embeddings |
| Huge (> 1GB) | 1GB | Model weights |

### Virtual Memory Layout

```
┌──────────────────────────────────────┐ 0xFFFF_FFFF_FFFF_FFFF
│           Kernel Space               │
│  ├── Kernel Code/Data                │
│  ├── Kernel Heap                     │
│  ├── Physical Memory Map             │
│  └── Device MMIO                     │
├──────────────────────────────────────┤ 0xFFFF_8000_0000_0000
│           Hole (non-canonical)       │
├──────────────────────────────────────┤ 0x0000_7FFF_FFFF_FFFF
│           User Space                 │
│  ├── Stack (grows down)              │
│  ├── mmap Region                     │
│  │   ├── Model Weights               │
│  │   ├── Shared Tensors              │
│  │   └── IPC Buffers                 │
│  ├── Heap (grows up)                 │
│  └── Code/Data                       │
└──────────────────────────────────────┘ 0x0000_0000_0000_0000
```

## Process Management

### Process Structure

```rust
pub struct Process {
    pid: ProcessId,
    /// Page table root
    page_table: PageTable,
    /// Kernel stack
    kernel_stack: VirtAddr,
    /// User stack
    user_stack: VirtAddr,
    /// Process state
    state: ProcessState,
    /// Priority class
    priority: Priority,
    /// Capabilities
    capabilities: CapabilitySet,
    /// Open handles (models, DB connections)
    handles: HandleTable,
}

pub enum Priority {
    /// Real-time inference tasks
    Inference,
    /// Interactive shell/UI
    Interactive,
    /// Background tasks
    Background,
    /// System services
    System,
}
```

### Scheduler

The scheduler is AI-aware with special handling for inference workloads:

```rust
impl Scheduler {
    pub fn schedule(&mut self) -> Option<ProcessId> {
        // 1. Check for inference tasks (highest priority)
        if let Some(pid) = self.inference_queue.pop() {
            return Some(pid);
        }

        // 2. Round-robin interactive tasks
        if let Some(pid) = self.interactive_queue.next() {
            return Some(pid);
        }

        // 3. Background tasks when idle
        self.background_queue.pop()
    }
}
```

## Inter-Process Communication

### Zero-Copy Channels

```rust
pub struct Channel<T> {
    /// Ring buffer in shared memory
    ring: SharedRingBuffer<T>,
    /// Sender notification
    sender_event: Event,
    /// Receiver notification
    receiver_event: Event,
}

impl<T> Channel<T> {
    /// Send without copying - data written directly to shared buffer
    pub fn send(&self, data: T) -> Result<(), ChannelError>;

    /// Receive reference to data in buffer
    pub fn recv(&self) -> Result<&T, ChannelError>;
}
```

### Shared Memory for Tensors

```rust
pub struct SharedTensor {
    /// Physical pages backing the tensor
    pages: Vec<PhysFrame>,
    /// Shape and dtype
    metadata: TensorMetadata,
    /// Reference count
    refcount: AtomicUsize,
}
```

## Hardware Abstraction Layer

### CPU Abstraction

```rust
pub trait Cpu {
    fn id(&self) -> CpuId;
    fn enable_interrupts(&self);
    fn disable_interrupts(&self);
    fn halt(&self);
}

// x86_64 implementation
pub struct X86_64Cpu { ... }
```

### GPU Abstraction

```rust
pub trait GpuDevice {
    fn name(&self) -> &str;
    fn memory_total(&self) -> usize;
    fn memory_free(&self) -> usize;
    fn allocate(&self, size: usize) -> GpuBuffer;
    fn copy_to_device(&self, src: &[u8], dst: &GpuBuffer);
    fn copy_from_device(&self, src: &GpuBuffer, dst: &mut [u8]);
    fn synchronize(&self);
}

// CUDA implementation
pub struct CudaDevice { ... }
```

### Storage Abstraction

```rust
pub trait BlockDevice {
    fn read_blocks(&self, start: u64, blocks: &mut [Block]) -> Result<()>;
    fn write_blocks(&self, start: u64, blocks: &[Block]) -> Result<()>;
    fn block_size(&self) -> usize;
    fn block_count(&self) -> u64;
}

// NVMe implementation
pub struct NvmeDevice { ... }
```

## Syscall Interface

### Syscall Table

| Number | Name | Description |
|--------|------|-------------|
| 0 | `sys_exit` | Terminate process |
| 1 | `sys_read` | Read from file descriptor |
| 2 | `sys_write` | Write to file descriptor |
| 3 | `sys_mmap` | Map memory |
| 4 | `sys_munmap` | Unmap memory |
| ... | ... | ... |
| 100 | `sys_vortex_load_model` | Load LLM model |
| 101 | `sys_vortex_infer` | Run inference |
| 102 | `sys_vortex_embed` | Generate embeddings |
| ... | ... | ... |
| 200 | `sys_gallifrey_query` | Query temporal DB |
| 201 | `sys_gallifrey_insert` | Insert into DB |
| ... | ... | ... |
| 300 | `sys_chronos_query` | RAG query |
| 301 | `sys_chronos_remember` | Store memory |

### Syscall Handler

```rust
pub fn syscall_handler(
    syscall_num: usize,
    arg1: usize,
    arg2: usize,
    arg3: usize,
    arg4: usize,
) -> isize {
    match syscall_num {
        0 => sys_exit(arg1 as i32),
        1 => sys_read(arg1, arg2 as *mut u8, arg3),
        // ... standard syscalls ...
        100 => sys_vortex_load_model(arg1 as *const u8, arg2),
        101 => sys_vortex_infer(arg1, arg2 as *const u32, arg3, arg4),
        // ... AI syscalls ...
        _ => -1, // ENOSYS
    }
}
```

## Interrupt Handling

### Interrupt Descriptor Table

```rust
lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.divide_error.set_handler_fn(divide_error_handler);
        idt.page_fault.set_handler_fn(page_fault_handler);
        idt.general_protection_fault.set_handler_fn(gpf_handler);
        idt[32].set_handler_fn(timer_interrupt_handler);
        idt[33].set_handler_fn(keyboard_interrupt_handler);
        // GPU interrupts, NVMe interrupts, etc.
        idt
    };
}
```

## Directory Structure

```
kernel/
├── Cargo.toml
└── src/
    ├── main.rs              # Entry point
    ├── boot/                # Boot code
    │   ├── mod.rs
    │   └── uefi.rs          # UEFI boot services
    ├── memory/              # Memory management
    │   ├── mod.rs
    │   ├── physical.rs      # Physical memory allocator
    │   ├── virtual.rs       # Virtual memory / paging
    │   ├── heap.rs          # Kernel heap
    │   └── huge_pages.rs    # Huge page support
    ├── process/             # Process management
    │   ├── mod.rs
    │   ├── process.rs       # Process structure
    │   ├── scheduler.rs     # AI-aware scheduler
    │   └── context.rs       # Context switching
    ├── ipc/                 # Inter-process communication
    │   ├── mod.rs
    │   ├── channel.rs       # Zero-copy channels
    │   └── shared_mem.rs    # Shared memory
    ├── hal/                 # Hardware abstraction
    │   ├── mod.rs
    │   ├── cpu/
    │   │   ├── mod.rs
    │   │   └── x86_64.rs
    │   ├── gpu/
    │   │   ├── mod.rs
    │   │   └── cuda.rs
    │   └── storage/
    │       ├── mod.rs
    │       └── nvme.rs
    ├── syscall/             # System calls
    │   ├── mod.rs
    │   └── handlers.rs
    └── interrupts/          # Interrupt handling
        ├── mod.rs
        └── idt.rs
```
