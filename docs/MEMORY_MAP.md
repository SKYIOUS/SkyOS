# Vahi Kernel Memory Map

This document defines the virtual address space layout for the Vahi kernel.

## Virtual Address Regions

| Start Address           | End Address             | Size      | Description                |
|-------------------------|-------------------------|-----------|----------------------------|
| `0xFFFF_8000_0000_0000` | `0xFFFF_8FFF_FFFF_FFFF` | 1 TB      | Physical Memory Mapping    |
| `0xFFFF_C000_0000_0000` | `0xFFFF_C000_07FF_FFFF` | 128 MB    | Kernel Heap                |
| `0xFFFF_E000_0000_0000` | (grows down)            | dynamic   | Kernel Stacks (dynamic)    |

## Current Implementation Details

- **Physical Memory Mapping**: Uses the offset provided by the bootloader at initialization (`PHYSICAL_MEMORY_OFFSET`, mapped at `0xFFFF_8000_0000_0000`).
- **Kernel Heap**: `0xFFFF_C000_0000_0000`, **128 MiB** (`HEAP_SIZE = 128 * 1024 * 1024` in `allocator.rs`).
- **Kernel Stacks**: Allocated dynamically per thread with a guard page (`memory/stack.rs`). New stacks are carved out of a region that grows downward from `NEXT_STACK_TOP = 0xFFFF_E000_0000_0000` — not a fixed pre-mapped region. There is no separate VMALLOC region.
- **Dynamic Allocations**: Handled by the Slab Allocator (`FixedSizeBlockAllocator`, `memory/slab.rs`) for small objects and the Buddy Allocator (`memory/buddy.rs`) for physical frames; large objects fall back to `linked_list_allocator`.

## Design Goals

1. **Higher-Half Kernel**: Map the kernel and all its data structures in the upper half of the virtual address space (canonical addresses starting with `0xFFFF...`).
2. **Standardization**: Follow established patterns for region naming and placement to ease future migrations (e.g., to Limine).
3. **Protection**: Guard pages under each kernel stack are implemented (`stack_bottom - 4096`).
