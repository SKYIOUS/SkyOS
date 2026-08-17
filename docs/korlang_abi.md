# Korlang ABI Contract

This document defines the interface between Korlang programs and the Vahi kernel.

## 1. Runtime Environment

- **Format:** Korlang programs are compiled to standard 64-bit ELF files.
- **Entry Point:** The entry point is defined in the ELF header.
- **Stack:** The kernel provides a stack for each process.

## 2. Calling Convention

Korlang uses the standard SkyOS syscall ABI (see `SYSCALL_ABI.md`) for all interactions with the
kernel.

## 3. Specialized Syscall: `SYS_KORLANG` (201)

`SYS_KORLANG` provides specialized runtime support for the Korlang interpreter/JIT. The kernel
implementation (`sys_korlang` in `syscalls/mod.rs`) dispatches on the sub-command ID in `rdi`:

| ID | Purpose |
|----|---------|
| 1 | `korlang_alloc(size, alignment)` → allocate memory, returns pointer |
| 2 | `korlang_free(ptr, size, alignment)` |
| 10 | `stdout_write(buf, len)` |
| 11 | `stdout_write(buf, len)` + newline |
| 20 | `file_open(path)` → file handle |
| 99 | `panic(msg)` — logs the message and exits the process |

## 4. Standard Library

Korlang programs run against the `libsarga` userspace runtime (the same runtime used by the rest of
the SkyOS userspace); Korlang-specific operations beyond the POSIX syscalls go through
`SYS_KORLANG` above.
