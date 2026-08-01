# Memory Protection Mechanisms

SkyOS uses hardware and software mechanisms to protect memory.

## Hardware Paging

The kernel uses 4-level x86_64 paging via the `x86_64` crate's `PageTable` types, with page-table entries controlling `PRESENT`, `WRITABLE`, `USER_ACCESSIBLE`, `NO_EXECUTE`, etc.

## NX Bit (No-Execute)

`PageTableFlags::NO_EXECUTE` (bit 63) marks memory pages non-executable:
- Data pages (stack, heap, BSS) are mapped non-executable
- ELF `PT_LOAD` segments set `NO_EXECUTE` when the segment is not executable (see `Process::load_elf_static`)
- Guard pages have no permissions (prevent overflow)

## SMEP (Supervisor Mode Execution Prevention)

SMEP prevents the kernel from executing code in userspace pages. It is enabled conditionally at boot in `arch/arch_x86_64.rs` when CPUID reports support (CR4 bit 20 / `0x100000`).

## SMAP (Supervisor Mode Access Prevention)

SMAP prevents the kernel from reading or writing userspace memory directly. It is enabled conditionally when CPUID reports support (CR4 bit 11 / `0x800`). Userspace accesses go through the explicit copy helpers in `syscalls/user_access.rs` (`copy_from_user`/`copy_to_user`/`read_user_string`).

## Guard Pages

Guard pages (unmapped) are placed at:
- The bottom of every thread stack (`memory/stack.rs` — `stack_bottom - 4096`)
- Kernel stacks have guard pages per thread

There is no KPTI (kernel page tables are shared, higher-half mapped), no KASAN, and no heap-region guard pages.
