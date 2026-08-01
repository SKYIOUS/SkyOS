# ELF Binary Loading Design

The kernel's ELF loader transforms executable files into running processes. It is implemented in `task/process.rs` (`Process::load_elf`) using the `xmas_elf` crate.

## Loading Process

`Process::load_elf(elf_data, address_space)`:

1. **Parse** the ELF with `xmas_elf::ElfFile`.
2. **Static load** (`load_elf_static`): for each `PT_LOAD` program header, a VMA is created (`virt_start`, file/mem size, offset). Page flags are `PRESENT | USER_ACCESSIBLE`, plus `WRITABLE` when the segment is writable and `NO_EXECUTE` when it is not executable.
3. **Dynamic detection**: if the binary has a `PT_DYNAMIC` header, `elf_dyn::load_dynamic_binary` performs dynamic linking (below).
4. **Process creation**: VMAs are registered (with merge of adjacent/overlapping segments) and the initial `brk` is set to the page-aligned end of the highest VMA.
5. **Entry point**: the process's `entry_point` is set from the ELF header; `sys_execve` sets up the user stack (`setup_user_stack(argv)`) and jumps to usermode via `jump_to_usermode(entry, rsp)`.

`sys_execve` (`syscalls/mod.rs`) does the pre-load work: copies the path/argv from user space, runs the LSM exec hook, resolves the VFS node, checks execute permission, applies setuid/setgid (with `hook_setuid_exec`), and copies the old fd table into the new process.

## Emulation

After loading, `emulation::set_emulation(&process, &elf_data)` inspects the ELF header to detect Linux binaries (`EmulationMode::Linux`), enabling the Linux-compatible syscall surface where present.

## Dynamic Linking (`elf_dyn.rs`)

Dynamically linked executables (`ET_DYN`):
- `load_dynamic_binary` walks `PT_DYNAMIC` entries (via `parse_dt_entries`/`get_dt`) to find DT_NEEDED, DT_SYMTAB, DT_STRTAB, DT_RELA.
- `load_library` reads each shared library as a VFS node (`node.read(usize::MAX)`), verifies it is an `ET_DYN` shared object, and maps its `PT_LOAD` segments at a load base.
- `apply_rela` performs RELA relocations (including symbol resolution against loaded libraries via `resolve_sym`).

## Linux Emulation

When a loaded binary is detected as Linux (`EmulationMode::Linux`), its syscalls route through `emulation.rs` `dispatch_linux_syscall` instead of the native table. A subset of Linux syscall numbers are handled natively (`rt_sigaction`=13, `rt_sigreturn`=15, `fork`=57, `uname`=63, `arch_prctl`=158); anything else is translated with `map_linux_to_vahi` and served by the native handler.
