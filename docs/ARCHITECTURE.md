# SkyOS Kernel Architecture

This document provides a high-level overview of the Vahi kernel's architecture, design principles, and major components.

## 1. Core Principles

Vahi is a monolithic kernel with a modular design, written from scratch in Rust. It aims for POSIX compatibility where feasible while exploring modern OS concepts. Key principles include:

- **Safety:** Leverage Rust's safety features to minimize `unsafe` code. All `unsafe` blocks must be justified with a `// SAFETY:` comment.
- **Modularity:** Subsystems (memory, scheduler, VFS) are designed as independent modules with clear APIs.
- **Hardware Abstraction:** All architecture-specific code is isolated, primarily through the `x86_64` crate and dedicated modules for hardware interaction (e.g., `apic`, `pci`).

## 2. Boot Process

1.  **UEFI Firmware:** The system starts by executing the UEFI firmware (e.g., OVMF in QEMU, or the motherboard's firmware).
2.  **Bootloader:** The UEFI firmware loads the `bootloader` crate's EFI application from the disk's EFI System Partition (ESP).
3.  **Kernel Loading:** The bootloader loads the Vahi kernel ELF file, sets up a higher-half memory map, maps the framebuffer, and hands off control.
4.  **`kernel_main`:** The bootloader jumps to the `kernel_main` entry point, passing a `BootInfo` structure containing the memory map, framebuffer details, and physical memory offset.

## 3. Major Subsystems

### 3.1 Memory Management

- **Physical Memory:** A **Buddy Allocator** (`memory/buddy.rs`) manages physical frames, tracking allocations in power-of-two blocks. It is initialized from the UEFI memory map.
- **Virtual Memory:** Each process has its own Level-4 Page Table (`PML4`). The kernel is mapped into the higher half of every address space, ensuring it is always accessible. `OffsetPageTable` from the `x86_64` crate is used for translation.
- **Heap:** A **Slab Allocator** (`memory/slab.rs`) manages small, fixed-size kernel allocations. Larger allocations fall back to a linked-list allocator. The heap is located at `0xFFFF_C000_0000_0000`.
- **Stacks:** Kernel thread stacks are dynamically allocated with guard pages to protect against stack overflows.

### 3.2 Interrupts & Exceptions

- **APIC:** The kernel uses the **LAPIC** (Local APIC) and **IOAPIC** for interrupt management, replacing the legacy PIC. The LAPIC timer drives the preemptive scheduler.
- **IDT:** An `InterruptDescriptorTable` is set up with handlers for hardware interrupts (timer, keyboard, mouse), exceptions (Page Fault, Double Fault), and syscalls.

### 3.3 Scheduling & Concurrency

- **Preemptive Scheduler:** A per-CPU **stride scheduler** (`task/scheduler.rs`) manages `Thread`s using tickets/`pass` proportional-share (see SCHEDULER.md). New threads spawn into a global `pending_queue`; idle CPUs steal work.
- **SMP:** The kernel boots Application Processors (APs), each of which enters the scheduler loop to begin executing threads.
- **Async Executor:** A cooperative async/await executor (`task/executor.rs`) runs tasks like the kernel shell. It is run within a dedicated kernel thread.

### 3.4 Virtual Filesystem (VFS)

- **Trait-based:** `VfsNode` and `FileSystem` traits provide a unified interface for files, directories, and devices.
- **Filesystems:** ramfs (/tmp), devfs (/dev), ctlfs (/ctl), pipe, tarfs (initrd root unless a block device filesystem is found), skyfs, ext2 (**read-write**), ext4 (**read-only**, feature-gated), and FAT32 (via `fatfs` crate).
- **Mounting:** A `VfsManager` (`pub static VFS: SchedLock<VfsManager>`) handles mounting filesystems at different paths.

### 3.5 Syscalls

- **Interface:** Uses the `SYSCALL`/`SYSRET` instructions with an assembly trampoline.
- **Compatibility:** Aims for POSIX compatibility, providing syscalls for processes, files, memory, and networking.

### 3.6 Drivers

- **PCI:** The PCI bus is enumerated at boot to discover devices.
- **Storage:** A block device trait abstracts storage controllers: AHCI, NVMe, PATA, and VirtIO block.
- **Network:** E1000 and VirtIO network drivers are supported, integrated with the `smoltcp` networking stack.
- **USB:** UHCI, XHCI, and HID drivers.
- **Audio:** HDA and PC speaker.
- **Graphics:** VirtIO GPU plus BGA framebuffer with a graphical console (UEFI GOP).
