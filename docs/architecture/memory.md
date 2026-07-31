# Memory Management Architecture

SkyOS implements a multi-layered memory management system built on x86_64 hardware paging.

## Paging

The kernel uses 4-level page tables (PML4, PDPT, PD, PT) with 4 KiB page sizes. The `AddressSpace` struct (`memory/paging.rs`) wraps `pml4: PhysFrame` plus `virt_offset` and uses the x86_64 crate's `OffsetPageTable`; `clone_cow()` implements copy-on-write for fork. Page tables are NOT accessed recursively.

```rust
pub struct AddressSpace {
    pml4: PhysFrame,
    virt_offset: u64,
    // OffsetPageTable<'static> mapper over PHYSICAL_MEMORY_OFFSET
}
```

## Physical Memory Manager

A **buddy allocator** (`memory/buddy.rs`) tracks physical page usage, backed by `memory/phys.rs` and `memory/frame_info.rs`.

## Virtual Memory Allocator

Kernel objects use a **slab allocator** (`memory/slab.rs`, `FixedSizeBlockAllocator`) with block sizes 8..4096 bytes; larger allocations fall back to `linked_list_allocator`. The slab is NOT NUMA-aware (no per-CPU caches).

## Kernel Heap

The kernel heap sits at a fixed virtual address `0xFFFF_C000_0000_0000`, **128 MiB** (`HEAP_SIZE`), managed by the global `Locked<FixedSizeBlockAllocator>` (`allocator.rs`).

## Userspace Memory

Each process has a dedicated address space (`AddressSpace`) with:
- A stack region allocated per-thread with a **guard page** at `stack_bottom - 4096` (`memory/stack.rs`)
- Heap region managed through `mmap()`/`munmap()` + `brk`
- Code and data segments loaded by the ELF loader
- Guard pages to detect stack overflow
