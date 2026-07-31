# Skyious Kernel Memory Map

This document defines the virtual address space layout for the Skyious kernel.

## Virtual Address Regions

| Start Address           | End Address             | Size      | Description                |
|-------------------------|-------------------------|-----------|----------------------------|
| `0xFFFF_8000_0000_0000` | `0xFFFF_8FFF_FFFF_FFFF` | 1 TB      | Physical Memory Mapping    |
| `0xFFFF_C000_0000_0000` | `0xFFFF_C000_07FF_FFFF` | 128 MB    | Kernel Heap                |
| `0xFFFF_D000_0000_0000` | `0xFFFF_DFFF_FFFF_FFFF` | 1 TB      | Kernel Stacks (dynamic)    |
| `0xFFFF_E000_0000_0000` | `0xFFFF_EFFF_FFFF_FFFF` | 1 TB      | VMALLOC Region             |

## Current Implementation Details

- **Physical Memory Mapping**: Uses the offset provided by the bootloader at initialization (`PHYSICAL_MEMORY_OFFSET`).
- **Kernel Heap**: `0xFFFF_C000_0000_0000`, **128 MiB** (`HEAP_SIZE = 128 * 1024 * 1024` in `allocator.rs`).
- **Kernel Stacks**: Allocated dynamically per thread with a guard page (`memory/stack.rs`) — not a fixed pre-mapped region.
- **Dynamic Allocations**: Handled by the Slab Allocator (`FixedSizeBlockAllocator`, `memory/slab.rs`) for small objects and the Buddy Allocator (`memory/buddy.rs`) for physical frames; large objects fall back to `linked_list_allocator`.

## Design Goals

1. **Higher-Half Kernel**: Map the kernel and all its data structures in the upper half of the virtual address space (canonical addresses starting with `0xFFFF...`).
2. **Standardization**: Follow established patterns for region naming and placement to ease future migrations (e.g., to Limine).
3. **Protection**: Implement guard pages between kernel stacks (Phase I3).
