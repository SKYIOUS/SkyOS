# SkyOS / SARGA OS Developer Report

> **Audience:** Developers evaluating, contributing to, or debugging this codebase
> **Date:** July 2026
> **Report type:** Comprehensive project assessment

---

## Table of Contents

1. [EXECUTIVE SUMMARY](#section-1-executive-summary)
2. [ARCHITECTURE OVERVIEW](#section-2-architecture-overview)
3. [WHAT WORKS](#section-3-what-works)
4. [WHAT'S BROKEN OR MISSING](#section-4-whats-broken-or-missing)
5. [DEV EXPERIENCE ASSESSMENT](#section-5-dev-experience-assessment)
6. [20-PAGE ACTION PLAN](#section-6-20-page-action-plan)
7. [VERDICT & RECOMMENDATION](#section-7-verdict--recommendation)

---

## SECTION 1: EXECUTIVE SUMMARY

### What is SkyOS Today

SkyOS (also branded as "SARGA OS") is a from-scratch operating system project written entirely in Rust. The project is split into two repositories:

- **This repository (SkyOS):** The **userspace environment** — libraries, shell, utilities, GUI desktop, package manager, init system, and all user-facing applications (40+ workspace crates)
- **A separate repository (SKYIOUS KERNEL / Vahi kernel):** The kernel itself — memory management, scheduler, VFS, drivers, syscall handlers

This report covers **the userspace repository only**. The kernel is in a separate repo (SKYIOUS KERNEL) that was not available for this analysis.

### Maturity Level: **PRE-ALPHA / TECHNOLOGY DEMONSTRATION**

This is not a usable operating system. It is an ambitious AI-generated codebase that:

- **Builds successfully** (with warnings) — the userspace compiles
- **Has no evidence of complete boot-to-shell operation** — QEMU logs show the kernel loading but stop before reaching userspace
- **Has 40+ crates** of widely varying quality
- **Has comprehensive aspirational documentation** that describes features not yet implemented
- **Has no test suite** — the tests/ directory contains 3 PowerShell scripts that test boot/login/panic via expect-like QEMU automation

### One-Line Verdict

> **Not developer-ready. A massive, ambitious, AI-generated prototype with broad but shallow implementation, a confusing dual-architecture design (microkernel claimed, monolithic implemented), and no path to running on real hardware today.**

---

## SECTION 2: ARCHITECTURE OVERVIEW

### 2.1 Project Split

This is a userspace-only repository. The kernel (Vahi/SKYIOUS KERNEL) lives in an external, undocumented sibling repository. The complete architecture is:

- Kernel repo: Memory management, scheduler, VFS, drivers, syscall dispatching, bootloader (via ootloader crate v0.11)
- This repo: All userspace — libsarga (standard library), sash (shell), coreutils (62 utilities), ade (desktop environment), init (PID 1), GUI apps, package manager, build scripts

### 2.2 System Call ABI

- **Instruction:** syscall/sysret
- **Register convention:** ax=number, di/si/dx/10/8/9=args, return in ax
- **Error convention:** negative ax = -errno
- **Syscall numbers:** Follow Linux x86_64 numbering where applicable
- **Documented as "FROZEN for v1.0"** — but the syscall table is aspirational, many are not implemented

### 2.3 Userspace Runtime (libsarga)

Two parallel libraries exist:

**libsarga** (primary, 17 modules):
- Pure Rust syscall wrappers via inline assembly
- Full widget toolkit: Button, Label, TextBox, CheckBox, ComboBox, Slider, Scrollbar, ProgressBar, TabWidget, MenuBar, Dialog
- TTF font rendering via 	tf-parser + custom rasterizer
- PNG decoding via miniz_oxide
- Networking: socket, connect, bind, listen, accept, sendto, recvfrom, DNS resolve, HTTP client
- GUI: window creation, buffer flush, key/mouse input
- POSIX compatibility layer with C-compatible function signatures

**libc (skyos-libc)** (secondary, partial):
- C-compatible syscall wrappers (returns u64, not Result)
- Heap allocator with malloc/free/realloc/calloc
- stdio: fopen/fclose/fread/fwrite/printf
- pthread: futex-based Mutex, TLS management via arch_prctl
- errno via TLS
- crt0: _start entry point

Having two separate libc-like implementations with different error handling conventions targeting the same ABI is a source of confusion and potential bugs.

### 2.4 Build System

Build pipeline: Userspace (cargo build) - Kernel (separate repo) - initrd (Python) - UEFI bootimage (builder crate) - ISO (xorriso)

### 2.5 Target Specification Issues

The x86_64-sarga.json target spec has problematic settings:
- "os": "none" — should be a custom OS value
- "code-model": "kernel" — inappropriate for userspace binaries
- "dynamic-linking": true — but no dynamic linker exists

### 2.6 Init System

/bin/init reads /etc/init.toml, mounts filesystems, forks services, monitors with wait4, supports respawn.

### 2.7 Desktop Environment (ADE)

Compositing window manager with taskbar, desktop icons, notifications, drag/move/resize, theming.

### 2.8 Network Stack

Userspace has socket/connect/DNS/HTTP wrappers. Kernel (separate repo) has smoltcp with E1000/VirtIO. Known broken: TCP connect is a stub, no ICMP, static IP only.

---

## SECTION 3: WHAT WORKS

### 3.1 Build System
- Userspace cargo build succeeds (with warnings)
- initrd creation via build_initrd.py
- ISO creation via scripts/make_iso.py (requires xorriso/WSL)
- Cargo workspace of 40+ crates

### 3.2 Library: libsarga
- syscall.rs: Raw syscall wrappers (inline asm for 0-6 args)
- io.rs: open/read/write/close/stat/mount/umount/sync/mkdir/rename/unlink/rmdir/chdir/getcwd/pipe/dup2/nanosleep
- process.rs: fork/execve/exit/wait/kill/getpid/getppid/uid/gid
- mem.rs: mmap/munmap/brk/GlobalAlloc/memcpy/memset/memcmp/memmove
- net.rs: socket/connect/bind/listen/accept/sendto/recvfrom/DNS resolve/HTTP client
- gui.rs: 733-line GUI framework with window management, TTF rendering, shape drawing, alpha blending, text rendering
- thread.rs: Thread spawn via clone + futex-based join, futex Mutex
- posix.rs: 30+ POSIX-compatible C wrappers

### 3.3 Core Utilities (55+ all compile)
cat, ls, echo, mkdir, rm, cp, mv, grep, find, head, wc, sort, env, kill, ps, top, df, du, uname, date, sleep, true, false, hexdump, od, dd, lspci, ping, mkfs_sargafs, login, passwd, chmod, chown, id, whoami, su, ln, readlink, tee, which, basename, dirname, xargs, sync, hostname, uptime, free, stat, touch, cut, tr, uniq, diff, tac, nl, sed, awk, patch, tar, gzip, mount, umount

### 3.4 Shell (sash)
Command parsing, pipelines, I/O redirection, env var expansion, job control (partial), history/readline, aliases, scripting (conditionals/loops/functions), tab completion, PATH searching

### 3.5 GUI Applications (all compile)
sarga-term, skyedit, skyfiles, skyview, calculator, calendar, clock, notes, paint, tasks, skysettings, sysinfo, sysmon, login-manager, skystore, installer

### 3.6 GitHub CI
Build-userspace.yml (automated), build-release-iso.yml (manual dispatch), system-updates.yml

---

## SECTION 4: WHAT'S BROKEN OR MISSING

### 4.1 Critical Structural Issues

| Issue | Severity | Details |
|-------|----------|---------|
| No kernel code in this repo | HIGH | build_disk.py, make_bootimage.ps1 reference non-existent kernel/ directory |
| No evidence of complete boot | HIGH | QEMU logs stop at kernel entry point, no userspace output |
| Dual libc implementations | HIGH | libsarga and libc/skyos-libc overlap, different conventions |
| Zero SAFETY comments | HIGH | Project guidelines require them, grep returns zero results |

### 4.2 49 unwrap()/expect() Calls

Spread across: notes, clock, calendar, archive, search, tasks, paint, sysmon, sysinfo, installer, skystore, login, passwd, sargash, sash/readline, vahid, svc, skybuild, nettools, coreutils (cp, lspci), libc/heap, libsarga/thread, libskyos, ade, setup

Each will panic on error. None have meaningful error messages.

### 4.3 unsafe Without Justification

Every userspace crate abuses unsafe:
- libsarga/syscall.rs: Unsafe inline asm (no SAFETY)
- libsarga/io.rs: 40+ unsafe syscall calls (no SAFETY)
- libsarga/gui.rs: Unsafe lifetime transmute of ttf_parser::Face (line 207)
- libsarga/thread.rs: Raw alloc + clone syscall
- libsarga/mem.rs: GlobalAlloc + memory intrinsics
- init/src/main.rs: 20+ unsafe calls
- sash/src/main.rs: 6 UnsafeCell statics for global state

### 4.4 Unsound Patterns

| Pattern | Location | Issue |
|---------|----------|-------|
| UnsafeCell globals | sash/src/main.rs lines 18,23,32,196,208,233 | No synchronization, used incorrectly |
| Lifetime transmute | libsarga/src/gui.rs:207 | transmute ttf_parser::Face<'a> to 'static |
| Unnecessary unsafe | skybuild/src/main.rs:39, login/src/main.rs:134 | Wrapping safe code |
| unreachable!() | sargaview/src/main.rs:218 | Could panic |

### 4.5 Build Warnings
- Unused imports (libskyos/net.rs)
- Unused variables (libskyos/lib.rs)
- Unnecessary unsafe blocks (skybuild, login)
- Dead code constants (skypkg)
- Useless comparisons (skypkg — u64 always >= 0)
- Output filename collisions (login, passwd — duplicate names)

### 4.6 Missing Feature Details

**Networking:**
- TCP connect is kernel stub — no data transfer
- No ICMP (ping uses UDP hack)
- Static IP only (10.0.2.15/24)
- No loopback interface
- No poll/select/epoll (no non-blocking I/O)

**Filesystem:**
- No statfs syscall (df/du/stat broken)
- No /proc or /sys
- No poll/select

**Security:**
- Password hashing uses simple djb2, not real crypt()
- No capability system (documented but not implemented)

**SMP:**
- Causes INVALID OPCODE — must use -smp 1

### 4.7 Documentation Issues
- docs/index.md references many non-existent files
- Architecture docs describe microkernel; AGENTS.md says monolithic
- Security docs describe unimplemented capability system
- CHANGELOG.md reads like a spec, not actual progress
- Design docs are aspirational, not reflective of code

### 4.8 Test Infrastructure
- tests/ directory has 3 PowerShell scripts with hardcoded paths to C:\Users\nanda
- No Rust #[test] functions anywhere
- No CI test execution
- No integration test framework

---

## SECTION 5: DEV EXPERIENCE ASSESSMENT

### Building: ⚠️ Partial
- Userspace cargo build: ✅ Works (with warnings)
- Bootable image: ❌ Requires kernel repo + external dependencies
- ISO: ❌ Requires WSL + xorriso on Windows

### Running: ❌ Not Proven
- No evidence of boot to shell prompt
- QEMU logs stop at kernel entry point
- SMP is broken

### Writing Programs: ⚠️ Partial
- libsarga API exists but undocumented, lacks SAFETY comments
- Two libc implementations create confusion
- Most apps use unwrap, not error handling

### Debugging: ❌ Minimal
- Serial console works
- No GDB integration
- No debugger setup documented
- No test framework

### Onboarding: ❌ Painful
- Kernel repo not referenced in Cargo.toml
- Build scripts reference non-existent directories
- Two libc implementations with no explanation
- Target spec has unusual settings
- Aspirational docs give false impression of maturity

---

## SECTION 6: 20-PAGE ACTION PLAN

### PHASE 0: STABILIZE BUILD (Sprint 1) — 2 person-weeks

P0.1: Fix output filename collisions (login, passwd) - rename or deduplicate
P0.2: Fix build warnings — unused imports, variables, unnecessary unsafe, dead code
P0.3: Add #![deny(warnings)] to workspace
P0.4: Document kernel repo dependency in README
P0.5: Make build scripts work (fix references to kernel/)

### PHASE 1: KERNEL-USERSPACE BRIDGE (Sprints 2-3) — 6 person-weeks

P1.1: Verify syscall contract between kernel and libsarga
P1.2: Test boot end-to-end (kernel to shell prompt)
P1.3: Create integration test suite (QEMU-based)
P1.4: Fix SMP (diagnose INVALID OPCODE)

### PHASE 2: SAFETY & CORRECTNESS (Sprints 3-4) — 8 person-weeks

P2.1: Document ALL unsafe blocks with // SAFETY:
P2.2: Eliminate ALL unwrap/expect() — replace with proper error handling
P2.3: Replace UnsafeCell in sash with proper synchronization
P2.4: Add alloc_error_handler consistency

### PHASE 3: LIBSARGA COMPLETENESS (Sprints 4-5) — 6 person-weeks

P3.1: Unify libc and libsarga — decide which to keep
P3.2: Add doc comments to ALL public API
P3.3: Standardize error handling (Error type, no raw i64)
P3.4: Add missing syscall wrappers (ioctl, select, sched_yield, fcntl, clock_gettime, rt_sigaction)

### PHASE 4: FILESYSTEM & STORAGE (Sprints 5-6) — 6 person-weeks

P4.1: Fix initrd loading (debug boot logs)
P4.2: Add statfs syscall
P4.3: Implement /proc and /sys (or ctlfs)
P4.4: Add blocking I/O support (poll/select/epoll)
P4.5: Verify ext2 write support

### PHASE 5: NETWORKING (Sprints 6-7) — 6 person-weeks

P5.1: Fix TCP implementation
P5.2: Add poll/select/epoll
P5.3: Add DHCP client
P5.4: Add loopback interface
P5.5: Add ICMP support

### PHASE 6: GUI & DESKTOP (Sprints 7-8) — 6 person-weeks

P6.1: Fix window compositor
P6.2: Add input method/keymap support
P6.3: Add GPU acceleration
P6.4: Complete widget toolkit

### PHASE 7: INIT & SERVICES (Sprint 8) — 4 person-weeks

P7.1: Complete init system (dependency resolution, logging, timeout)
P7.2: Add login/session management
P7.3: Complete device manager (vahid)

### PHASE 8: TOOLING & UTILITIES (Sprint 9) — 4 person-weeks

P8.1: Complete package manager (spkg/skypkg)
P8.2: Complete complex coreutils (gzip, tar, sed, awk, diff, patch)
P8.3: Add compiler toolchain

### PHASE 9: SECURITY (Sprints 9-10) — 4 person-weeks

P9.1: Fix password authentication (real hashing)
P9.2: Verify process isolation
P9.3: Capability system (deferrable)

### PHASE 10: TESTING & QA (Sprints 10-11) — 8 person-weeks

P10.1: Create QEMU-based integration test framework
P10.2: Add in-kernel self-tests
P10.3: Stress testing (memory, scheduler, filesystem, network)

### PHASE 11: PERFORMANCE (Sprint 11) — 4 person-weeks

P11.1: Profile and optimize boot time
P11.2: Optimize memory allocator
P11.3: Optimize GUI rendering

### PHASE 12: DOCUMENTATION (Sprint 12) — 4 person-weeks

P12.1: Right-size documentation to match codebase reality
P12.2: Write getting-started guide
P12.3: Create architecture decision records

### PHASE 13: CI/CD & DEVOPS (Ongoing) — 4 person-weeks

P13.1: Add clippy, fmt checks to CI
P13.2: Add automated release pipeline
P13.3: Add code quality dashboard

### PHASE 14: PORTABILITY (Backlog) — 8+ person-weeks

P14.1: Support aarch64 userspace
P14.2: Support RISC-V
P14.3: Implement dynamic linking

### Total Estimated Effort: ~72-80 person-weeks (1.5-2 person-years)

---

## SECTION 7: VERDICT & RECOMMENDATION

### Should a Developer Use This Today?

**No. Absolutely not.**

Reasons:
1. Cannot build a bootable system (kernel is in separate undocumented repo)
2. Cannot reach a shell prompt (QEMU logs show no userspace boot)
3. Cannot write safe programs (zero SAFETY comments, 49 unwrap calls, unsound patterns)
4. Cannot trust documentation (aspirational, contradictory, mismatched with code)
5. Cannot contribute effectively (two libcs, no tests, no CI passing)
6. Cannot run on real hardware (no evidence of hardware compatibility)

### Most Critical Thing to Fix First

**Phase 0 + Phase 1: Make the system actually boot to a shell.** Without this, nothing else matters.

### Is This Project Viable Long-Term?

**Possibly, but it needs a major reset.** The codebase demonstrates breadth (40+ crates, GUI desktop, 70+ documented syscalls, 62 utilities) but lacks depth, safety, and correctness. 1-2 person-years of human engineering effort is needed.

### What Would Make It Worth Using

1. The kernel is real, boots, and provides the documented syscall surface
2. Safety issues are addressed
3. The dual-libc confusion is resolved
4. AI-generated code is reviewed and hardened
5. A real test suite exists and passes
6. Documentation reflects reality

Developers should treat this as a **learning resource** and **starting point** for their own OS project, not as a production-ready system.

---

*Report generated July 2026 by systematic codebase analysis.*
*Kernel source was not available for review — the Vahi kernel is in a separate repository.*
