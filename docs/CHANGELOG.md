# SkyOS Changelog

## [0.6.0] - 2026-07

### Build System
- Unified `build_disk.py` with cleaner error handling and Path-based paths
- Consolidated from 4 build workflows to 2 (ci.yml, release-iso.yml)
- Removed stale workflows: build-userspace, build-release-iso, system-updates
- `make_iso.py` now auto-falls back from xorriso to pycdlib

### CI/CD
- CI: matrix builds (debug + release), cargo caching, `cargo-deny` checks
- Dependabot config for weekly Rust + monthly Actions updates
- Stale issue/PR management (90d stale, 30d close)
- Labeler auto-tags PRs by changed paths
- Release ISO build streamlined with direct artifact upload

### Developer Experience
- Added `.gitattributes` for cross-platform line endings
- Added `SECURITY.md` and `CODEOWNERS`
- GitHub issue forms (structured YAML) replacing free-form markdown
- `.gitignore` cleanup: removed stale entries, added new artifact patterns

### Documentation
- BUILD.md: consolidated build instructions, removed legacy script references
- README.md: updated badge URLs, CI workflow table, project structure

## [0.1.0] - In Development

### Phase A: Foundation Hardening
- Upgraded `bootloader` to v0.11 for UEFI support.
- Buddy Allocator for physical frame management.
- Slab Allocator for efficient kernel object allocation.
- Kernel virtual address space layout defined and documented.
- LAPIC and IOAPIC for modern interrupt handling.

### Phase B: Process & Memory Model
- Per-process user page tables with shared higher-half kernel mapping.
- Page Fault handler with demand paging.
- Copy-on-Write (CoW) for efficient `fork`.
- Process model with global process table.
- ELF loader for user-space programs.
- `sys_fork`, `sys_execve`, `sys_exit`, `sys_wait4`.

### Phase C: Scheduler Evolution
- Priority-based, preemptive round-robin scheduler.
- `sys_nanosleep` for thread blocking.
- SMP support for multi-core scheduling.

### Phase D: Syscall Expansion
- POSIX-compatible syscalls for processes, files, memory, and IPC.

### Phase E: Filesystem Layer
- Trait-based VFS: Tmpfs, Ext2 (ro), FAT32, Pipe.

### Phase F: Networking Stack
- smoltcp with E1000 and VirtIO drivers + socket syscalls.

### Phase G: Graphical System
- Framebuffer driver (UEFI GOP) + graphical console.

### Phase H: Korlang Integration
- `sys_korlang` syscall for runtime support.

### Phase I: Security
- SMEP/SMAP, kernel stack guard pages, syscall input validation.

### Phase J: ADE Desktop Environment
- Compositing window manager, taskbar, notifications, theming.
- GUI applications: terminal, text editor, file manager, calculator, paint.

### Phase K: IPC Transport
- AF_UNIX socketpair IPC, service codec, ADE dispatch.

### Phase L: Userspace Completion
- POSIX syscall wrappers across init, sash, coreutils, daemons.
- Network tools, package manager (spkg), AI integration (aicli).

## Documentation
- Architecture, syscalls, VFS, scheduler, build system docs.
- Design philosophy, async model, memory safety, GUI architecture.
- Security architecture, testing methodology, API reference.