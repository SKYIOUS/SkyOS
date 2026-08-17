# Memory Safety Approach in Kernel Space

Ensuring memory safety in kernel space requires strategies beyond what userspace Rust provides.

## The Challenge

Kernel code must manipulate hardware registers, construct page tables, and manage physical memory—all of which requires unsafe Rust. The challenge is to confine unsafe code to minimal, auditable regions.

## Safety Strategy

### 1. Encapsulation

Unsafe operations are wrapped in safe abstractions. Page mapping is exposed through the `x86_64` crate's `OffsetPageTable` plus thin kernel wrappers (`memory/virt.rs`):

```rust
// memory/virt.rs
pub unsafe fn map_contiguous(
    mapper: &mut OffsetPageTable,
    virt_start: VirtAddr,
    phys_start: PhysAddr,
    page_count: u64,
    flags: Flags,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) {
    // for each page: mapper.map_to(page, frame, flags, frame_allocator)
}

pub fn map_device(phys_addr: PhysAddr, _size: u64) -> VirtAddr {
    // returns PHYSICAL_MEMORY_OFFSET + phys_addr
}
```

### 2. Invariant Documentation

Every unsafe function includes a `Safety:` section documenting the caller's responsibilities. These invariants are checked during code review.

### 3. Runtime Checking

Debug builds include runtime assertions for safety invariants. For example, pointer validity is checked before dereference:

```rust
fn read_u32(ptr: *const u32) -> u32 {
    debug_assert!(!ptr.is_null() && (ptr as usize) % 4 == 0);
    unsafe { *ptr }
}
```

### 4. Type System Enforcement

The type system prevents common errors:
- `PhysFrame` vs `VirtAddr`: Different types prevent address space confusion
- `Port<T>` for I/O ports vs raw MMIO access: access methods are type-checked

There is no KASAN. Runtime safety relies on the checks above plus the allocator/guard-page mechanisms described in `docs/security/memory_protection.md`.
