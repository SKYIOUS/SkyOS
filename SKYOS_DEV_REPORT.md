# SkyOS / SARGA OS — Comprehensive Developer Report

> **Audience:** Developers evaluating, contributing to, or debugging this codebase  
> **Date:** July 2026  
> **Type:** Full codebase audit & 20-page action plan  
> **Scope:** Entire userspace repository (kernel is in a separate, external repo)

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

### What Is SkyOS Today

SkyOS (also branded as "SARGA OS") is a from-scratch operating system project written entirely in Rust, split across two repositories:

- **This repository (SkyOS):** The **userspace environment** — 38 workspace crates including libraries, shell, 60+ core utilities, GUI desktop environment, init system, package manager, and graphical applications
- **A separate repository (SKYIOUS KERNEL / Vahi kernel):** The kernel itself — this repo is **not present** in this workspace and is referenced only in build scripts and CI YAML files

**This report covers only the userspace repository.** The kernel was not available for analysis.

### Maturity Level: **PRE-ALPHA / AI-GENERATED PROTOTYPE**

- The userspace **compiles** (Cargo build succeeds with warnings)
- There is **no evidence of a complete boot-to-shell** working system — all QEMU logs provided in the repo show kernel loading messages but no userspace output
- The previous analysis report (SKYOS_DEV_REPORT.md) noted `libc/` — **that directory no longer exists** in this repo
- 38 crates of widely varying quality, many just GUI skeletons with hardcoded window sizes and `unwrap()` calls
- Build scripts reference directories (`kernel/`, `userspace/`) that do not exist
- The E2E plan claims "COMPLETE ✅" for all 13 phases, but this is aspirational

### One-Line Verdict

> **Not usable by end users or developers. A massive AI-generated prototype with broad but shallow coverage, unsound unsafe patterns, aspirational documentation, no working boot path, and an estimated 1.5-2 person-years of work needed to reach a usable state.**

---

## SECTION 2: ARCHITECTURE OVERVIEW

### 2.1 Repository Split

The userspace repo contains 38 crates. The kernel lives in `SKYIOUS KERNEL/` (sibling directory expected by build scripts).

```
┌─ Userspace (this repo) ──────────────────────────────┐
│  libsarga/     — Standard library                     │
│  sash/         — Shell                                │
│  init/         — PID 1 init process                   │
│  ade/          — Desktop environment                  │
│  coreutils/    — 60+ utility binaries                 │
│  login/        — Text login                           │
│  login-manager/— GUI login manager                    │
│  passwd/       — Password change                      │
│  sarga-term/   — Terminal emulator                    │
│  + 28 more app/service/support crates                 │
├─ External (not in repo) ──────────────────────────────┤
│  SKYIOUS KERNEL/ — Vahi kernel (separate repo)        │
│  No local kernel/ directory exists                    │
└───────────────────────────────────────────────────────┘
```

### 2.2 System Call ABI

- **Instruction:** `syscall` / `sysret`
- **Register convention:** `rax` = number, `rdi/rsi/rdx/r10/r8/r9` = args, return in `rax`
- **Error conventions:** negative `rax` = `-errno`

**50 syscall constants defined in `libsarga/src/syscall.rs` (lines 26-79):**
0=READ, 1=WRITE, 2=OPEN, 3=CLOSE, 4=STAT, 5=FSTAT, 8=LSEEK, 9=MMAP, 11=MUNMAP, 12=BRK, 13=RT_SIGACTION, 15=RT_SIGRETURN, 16=IOCTL, 22=PIPE, 23=SELECT, 24=SCHED_YIELD, 33=DUP2, 35=NANOSLEEP, 39=GETPID, 41=SOCKET, 42=CONNECT, 44=SENDTO, 45=RECVFROM, 49=BIND, 50=LISTEN, 56=CLONE, 57=FORK, 59=EXECVE, 60=EXIT, 61=WAIT4, 62=KILL, 63=UNAME, 79=GETCWD, 80=CHDIR, 83=MKDIR, 87=UNLINK, 88=SYMLINK, 89=READLINK, 91=FCHMOD, 92=FCHOWN, 104=BEEP, 158=ARCH_PRCTL, 165=MOUNT, 166=UMOUNT2, 200=RESOLVE, 202=FUTEX, 217=GETDENTS64, 228=CLOCK_GETTIME, 301=GETUID, 302=GETGID, 303=SETUID, 304=SETGID, 305=GETEUID, 306=GETEGID, 7=POLL

### 2.3 Target Specification Problems

`x86_64-sarga.json` (lines 1-36) has these issues:
- **"os": "none"** — should be a custom OS name
- **"code-model": "kernel"** — wrong for userspace, should be "small"
- **"dynamic-linking": true** — no dynamic linker exists for this target

### 2.4 Build System Flow

```
cargo build (userspace)  →  OK (with warnings)
  ↓
build_initrd.py          →  OK (if binaries exist)
  ↓
(cd kernel && cargo)     →  BROKEN  — kernel/ doesn't exist
  ↓
builder/ crate           →  BROKEN  — in kernel repo
  ↓
make_iso.py              →  BROKEN  — requires kernel steps
```

### 2.5 Init System

`init/src/main.rs` (116 lines):
- Mounts tmpfs, devfs, ctlfs
- Forks two hardcoded services: `/bin/login-manager` and `/bin/svc`
- Loops on `waitpid(-1, 0)` to reap/respawn children
- Does NOT read `/etc/init.toml` (documented but not implemented)

---

## SECTION 3: WHAT WORKS

### 3.1 Compilation
The entire workspace compiles for `x86_64-sarga` with nightly Rust + `build-std`.

### 3.2 Library: libsarga (35 source files)
- **syscall.rs** — 50 syscall constants + inline asm wrappers (0-6 args)
- **io.rs** — open/read/write/close/stat/fstat/mkdir/unlink/getcwd/chdir/getdents64/nanosleep/sync/reboot/fchmod/fchown/mount/umount/select/dup2/clipboard/notify
- **process.rs** — fork/execve/exit/wait4/waitpid/getpid/getppid/getuid/geteuid/getgid/getegid/setuid/setgid/kill/signal
- **mem.rs** — mmap/munmap/brk + GlobalAlloc + memcpy/memset/memcmp/memmove
- **net.rs** — Socket struct + resolve/socket/bind/listen/accept/connect/send/recv + HttpClient
- **thread.rs** — spawn/join + futex + Mutex
- **gui.rs** (733 lines) — Window create/buffer/flush/fill/draw_rect/draw_line/draw_char/glyph cache/TTF outline rasterizer/alpha blend
- **fs.rs** — stat/fstat/statfs/touch/open/read/write/close/mkfs/mount/umount/read_to_string/write_file
- **gpu.rs** — DRM control wrappers (get_display_info/create_dumb/page_flip/etc.)
- **hash.rs** — PBKDF2-SHA256 via syscall 401
- **posix.rs** — 30+ POSIX-compatible C wrappers
- **sync.rs** — RawMutex (futex-based) + Mutex<T> + MutexGuard + TlsKey + init_tls
- **stdio.rs** — FILE struct + fopen/fclose/fread/fwrite/fputs/fprintf/fgetc
- **args.rs** — argc/argv/get (with 32-bit truncation bug)
- **errno.rs** — Error enum (35 variants) + from_i64
- **theme.rs** — Color constants + dark/light themes
- **start.rs** — _start entry point
- **ai.rs + vahiai.rs** — VahiAI query wrappers
- **libskyos.rs** — SysInfo/sysinfo/getcwd/chdir/list_dir/getpid/sleep_ms/hostname
- **14 widget files** — Widget trait + Container + Button/Label/TextBox/CheckBox/ComboBox/Slider/Scrollbar/ProgressBar/TabWidget/MenuBar/Dialog/Layout

### 3.3 Shell: sash (6 source files, ~1500 lines total)
- Command parsing with pipelines and I/O redirection
- Environment variable expansion ($VAR, ${VAR}, $?)
- Job control (bg, fg, jobs)
- Readline with history and tab completion
- Shell scripting (if/while/for/functions)
- Aliases and shell functions
- PATH searching

### 3.4 Core Utilities (56 binaries, all compile)
cat, ls, echo, mkdir, rm, cp, mv, grep, find, head, wc, sort, env, kill, ps, top, df, du, uname, date, sleep, true, false, hexdump, od, dd, lspci, ping, mkfs_sargafs, chmod, chown, id, whoami, su, ln, readlink, tee, which, basename, dirname, xargs, sync, hostname, uptime, free, stat, touch, cut, tr, uniq, diff, tac, nl, sed, awk, patch, tar, gzip, mount, umount

### 3.5 GUI Applications (all compile)
sarga-term, skyedit, skyfiles, skyview, calculator, calendar, clock, notes, paint, tasks, search, archive, sysinfo, sysmon, installer, skystore, sargasettings, login-manager

### 3.6 GitHub CI
- `build-userspace.yml` — builds on push/PR (works)
- `build-release-iso.yml` — manual dispatch (untested, needs kernel repo)
- `system-updates.yml` — manual dispatch (untested)

---

## SECTION 4: WHAT'S BROKEN OR MISSING

### 4.1 Critical Structural Issues (6 HIGH severity)

| ID | Issue | File:Line | Detail |
|----|-------|-----------|--------|
| C1 | **No kernel in this repo** | `build_disk.py:18` | `kernel_dir = os.path.join(root_dir, "kernel")` — directory doesn't exist |
| C2 | **No working boot path** | All QEMU logs | No log shows userspace output. System never reaches shell. |
| C3 | **4 different error conventions** | `io.rs` vs `fs.rs` vs `posix.rs` vs `syscall.rs` | io.rs uses `Result<_, Error>`, fs.rs uses `Result<_, i64>`, posix.rs returns raw i64, syscall.rs returns i64 |
| C4 | **libc crate missing** | Previous report references it | `libc/` directory does not exist. Either deleted or never created. |
| C5 | **Zero TODO/FIXME/HACK/XXX** | All .rs files | Grep returns zero results. A 38-crate codebase with zero acknowledged issues is suspicious. |
| C6 | **64-bit pointer truncated to 32 bits** | `libsarga/src/args.rs:9` | `ARGV.store(stack as i32 + 8, ...)` — on x86_64 this truncates stack pointer to 32 bits |

### 4.2 All 21 unwrap()/expect() Calls

**HIGH severity (10):**
- `libsarga/src/thread.rs:24` — Layout::from_size_align unwrap
- `libsarga/src/libskyos.rs:67-68` — try_into unwrap on directory entries
- `installer/src/main.rs:9` — Window::create unwrap
- `calendar/src/main.rs:8` — Window::create unwrap
- `clock/src/main.rs:7` — Window::create unwrap
- `paint/src/main.rs:7` — Window::create unwrap
- `notes/src/main.rs:9` — Window::create unwrap
- `search/src/main.rs:8` — Window::create unwrap
- `tasks/src/main.rs:7` — Window::create unwrap
- `archive/src/main.rs:7` — Window::create unwrap
- `skystore/src/main.rs:15` — Window::create unwrap
- `sysinfo/src/main.rs:7` — Window::create unwrap
- `sysmon/src/main.rs:7` — Window::create unwrap

**MEDIUM severity (6):**
- `sash/src/readline.rs:66,90,127` — CString unwrap
- `ade/src/main.rs:251` — windows.last_mut unwrap
- `coreutils/src/cp.rs:91` — files.pop expect
- `coreutils/src/lspci.rs:16` — fd unwrap
- `nettools/src/ifconfig.rs:9` — fd unwrap

**LOW severity (2):**
- `sash/src/main.rs:174` — wait unwrap_or (has fallback)

### 4.3 unsafe Without SAFETY Justification

100+ unsafe blocks across the codebase. Only 38 have "SAFETY:" comments (all in `io.rs`, `net.rs`, `hash.rs`, `vahiai.rs`). Every single one says `// SAFETY: <syscall> syscall is safe here` which is inadequate.

**Key unsafe blocks missing SAFETY:**
- `libsarga/src/syscall.rs:5-17` — Core inline asm (0 SAFETY)
- `libsarga/src/syscall.rs:81-94` — All unsafe wrappers
- `libsarga/src/mem.rs:5-75` — mmap/munmap/GlobalAlloc/memcpy/memset
- `libsarga/src/process.rs:7-104` — All process functions
- `libsarga/src/thread.rs:9-50` — futex + spawn
- `libsarga/src/sync.rs:21-117` — 10 unsafe blocks
- `libsarga/src/gui.rs:206-210` — Lifetime transmute to 'static
- `libsarga/src/gui.rs:367-376` — Window::create
- `init/src/main.rs:28-48` — fork/execve
- `sash/src/main.rs:18-234` — 50+ UnsafeCell accesses
- `sash/src/executor.rs:71-336` — 100+ syscall calls
- `coreutils/src/cp.rs:21-117` — 96 lines of raw syscall

### 4.4 Unsound Patterns (10 identified)

| ID | Pattern | File:Line | Issue |
|----|---------|-----------|-------|
| U1 | **Lifetime transmute** | `libsarga/src/gui.rs:206-210` | `transmute<Face<'_>, Face<'static>>` — dangling pointer risk |
| U2 | **UnsafeCell without locks** | `sash/src/main.rs:18-33` | ShellEnv/AliasTable/JobTable use UnsafeCell + Sync without synchronization |
| U3 | **Mutable statics** | `libsarga/src/stdio.rs:21-23` | `pub static mut STDIN: FILE` — unsound, any access is UB |
| U4 | **Pointer truncation** | `libsarga/src/args.rs:9` | Stack pointer cast to i32 loses upper 32 bits |
| U5 | **Unsound Sync impl** | `sash/src/main.rs:19` | `unsafe impl Sync for ShellEnv` — Vec<String> is not Sync |
| U6 | **Unsound Sync impl** | `libsarga/src/sync.rs:38-39` | `Sync for Mutex<T>` where T: Send — should require T: Send + Sync |
| U7 | **Memory leak** | `libsarga/src/stdio.rs:49` | `Box::leak(f)` — fopen never frees |
| U8 | **unreachable!()** | `sargaview/src/main.rs:218` | Match arm will panic on unexpected input |
| U9 | **Duplicate HttpClient** | `libsarga/src/net.rs:195-202 + 216-254` | Same struct defined twice in same file |
| U10 | **Wrong syscall number** | `libsarga/src/fs.rs:120` vs `syscall.rs:68` | fs.rs uses 167, syscall.rs defines 166 for SYS_UMOUNT2 |

### 4.5 Build Issues (10 identified)

| ID | Issue | File:Line | Severity |
|----|-------|-----------|----------|
| B1 | Missing .json in target | `build.ps1:6` | HIGH |
| B2 | userspace/ dir doesn't exist | `build_userspace.ps1:9` | HIGH |
| B3 | kernel/ dir doesn't exist | `build_disk.py:18` | HIGH |
| B4 | Hardcoded user paths | `rebuild_initrd.ps1:5-6` | HIGH |
| B5 | Hardcoded kernel path | `run.ps1:4` | HIGH |
| B6 | Fragile kernel lookup | `build_image.py:18-28` | MEDIUM |
| B7 | Hardcoded kernel path | `Makefile:13` | HIGH |
| B8 | Nightly-only build-std | `.cargo/config.toml:2` | LOW |
| B9 | Duplicate shells/pkg mgrs | Multiple Cargo.toml | MEDIUM |
| B10 | Dev profile without panic=abort | `Cargo.toml` | LOW |

### 4.6 Missing Features (20 documented but not implemented)

- TCP connect (kernel stub, no data)
- ICMP (ping uses UDP hack)
- poll/select/epoll (select wrapper exists, kernel may not support)
- statfs (wrapper exists, kernel may not implement)
- Loopback interface
- DHCP (static IP only)
- Non-blocking I/O
- SMP stability (INVALID OPCODE)
- Proper password hashing (djb2 used)
- io_uring (wrappers documented but no code)
- bpf/eBPF (defined in ABI, no code)
- fcntl (defined in ABI, no code)
- Signal handling (rt_sigaction call uses sig=0, wrong)
- Dynamic linking (target spec says true, no linker exists)
- SkyFS (custom filesystem, only mkfs_sargafs.rs exists)
- /proc and /sys filesystems
- Access syscall
- Dup syscall
- Lseek syscall
- Rename syscall

### 4.7 Documentation Issues (22 identified)

- docs/index.md references 100+ files that don't exist
- CHANGELOG.md is a spec/roadmap, not a changelog
- SYSCALL_ABI.md claims frozen v1.0 but 20+ syscalls have no code
- SKYOS_E2E_PLAN.md marks all phases COMPLETE but lists known limitations that contradict
- architecture docs describe microkernel; AGENTS.md says monolithic
- security docs describe non-existent capability system
- All design, future, testing docs are aspirational
- README claims numbers that don't match code (90+ syscalls, 12+ drivers, 7 filesystems)
- SARGA_OS_DESIGN_QA.md is 100+ Q&A about a system that doesn't exist

### 4.8 Test Infrastructure (9 issues)

- Only 3 PowerShell test scripts exist
- All have hardcoded absolute paths (C:\Users\nanda\...)
- Zero Rust #[test] functions anywhere
- Tests require external kernel binary
- Tests require QEMU
- No unit tests for libsarga
- No CI test execution
- test_login.ps1 requires `expect` utility
- No test framework

### 4.9 Code Duplication (6 instances)

| Duplicate | Files | Problem |
|-----------|-------|---------|
| HttpClient | `libsarga/src/net.rs:195 + 216` | Same struct twice in same file |
| Password verify | `login/` + `login-manager/` | Identical PBKDF2 logic duplicated |
| I/O wrappers | `io.rs` vs `fs.rs` | Both provide open/read/write/close with different errors |
| Shells | `sash/` + `sargash/` | Two shell crates in workspace |
| Package managers | `spkg/` + `skypkg/` | Two crate names for package manager |
| AI wrappers | `vahiai.rs` + `ai.rs` | Both wrap SYS_VAHIAI=300 |

---

## SECTION 5: DEV EXPERIENCE ASSESSMENT

| Dimension | Score | Notes |
|-----------|-------|-------|
| **Building** | ⚠️ Partial | Cargo build works. Build scripts are broken. Bootable image requires external repo. |
| **Running** | ❌ Not Proven | No evidence of boot to shell. SMP broken. Network is stub. |
| **Writing Programs** | ⚠️ Partial | libsarga API exists but is inconsistent (4 error conventions). Unsafe undocumented. Widget toolkit untested. |
| **Debugging** | ❌ Minimal | Serial output works. No GDB, no stack traces, no structured logging. |
| **Onboarding** | ❌ Painful | Aspirational docs don't match reality. Two shells/pkg mgrs confuse. Kernel repo unknown. |

---

## SECTION 6: 20-PAGE ACTION PLAN

### Phase 0: Emergency Stabilization (Sprint 1) — 2 pw

| Task | Effort | Target |
|------|--------|--------|
| P0.1 Fix args.rs 32-bit truncation | 2h | `libsarga/src/args.rs:9` |
| P0.2 Fix duplicate HttpClient | 1h | `libsarga/src/net.rs:195-202` |
| P0.3 Fix SYS_UMOUNT2 mismatch | 1h | `libsarga/src/fs.rs:120` |
| P0.4 Audit 21 unwrap calls | 2d | All unwrap sites |
| P0.5 Fix build scripts | 1d | build.ps1, build_disk.py, run.ps1 |
| P0.6 Remove libc references | 1h | README, previous report |
| P0.7 Fix build_userspace.ps1 | 1d | build_userspace.ps1 |

### Phase 1: Kernel-Userspace Bridge (Sprints 2-3) — 6 pw

| Task | Effort | Target |
|------|--------|--------|
| P1.1 Find kernel repo | 2h | README |
| P1.2 Verify syscall contract | 1w | Both repos |
| P1.3 End-to-end CI build | 1w | .github/workflows |
| P1.4 QEMU integration test | 2d | tests/ |
| P1.5 Fix SMP | 2w | Kernel |
| P1.6 Document split | 1d | README, AGENTS.md |

### Phase 2: Safety & Correctness (Sprints 3-4) — 8 pw

| Task | Effort | Target |
|------|--------|--------|
| P2.1 SAFETY comments on all unsafe | 2w | 100+ blocks |
| P2.2 Fix lifetime transmute | 1d | `gui.rs:206` |
| P2.3 Fix sash UnsafeCell | 2d | `sash/src/main.rs` |
| P2.4 Fix mutable statics | 1d | `stdio.rs:21-23` |
| P2.5 Audit Sync/Send impls | 2d | Multiple |
| P2.6 Fix Box::leak leak | 1d | `stdio.rs:49` |
| P2.7 Fix unreachable!() | 1h | `sargaview.rs:218` |

### Phase 3: Library Unification (Sprints 4-5) — 6 pw

| Task | Effort | Target |
|------|--------|--------|
| P3.1 Merge io.rs + fs.rs | 1w | libsarga |
| P3.2 Audit posix.rs | 2d | libsarga |
| P3.3 Doc comments on all public API | 2w | libsarga |
| P3.4 Standardize error handling | 1w | libsarga |
| P3.5 Decide sash vs sargash | 1d | Both |
| P3.6 Decide spkg vs skypkg | 1d | Both |

### Phase 4: Filesystem (Sprints 5-6) — 6 pw

| Task | Effort | Target |
|------|--------|--------|
| P4.1 Verify initrd loading | 1w | Kernel |
| P4.2 Implement statfs | 3d | Kernel |
| P4.3 Implement /proc or ctlfs | 2w | Kernel |
| P4.4 Implement poll/select | 2w | Kernel |
| P4.5 Fix pipe buffer | 2d | Kernel |

### Phase 5: Networking (Sprints 6-7) — 6 pw

| Task | Effort | Target |
|------|--------|--------|
| P5.1 Fix TCP | 2w | Kernel |
| P5.2 Socket poll/select | 1w | Kernel |
| P5.3 DHCP client | 1w | Userspace+kernel |
| P5.4 Loopback | 3d | Kernel |
| P5.5 ICMP | 1w | Kernel |
| P5.6 Verify HttpClient | 1d | nettools |

### Phase 6: GUI (Sprints 7-8) — 6 pw

| Task | Effort | Target |
|------|--------|--------|
| P6.1 Fix compositor | 1w | Kernel |
| P6.2 Keyboard input | 1w | Kernel+libsarga |
| P6.3 Mouse input | 1w | Kernel+libsarga |
| P6.4 Fix TTF rendering | 3d | gui.rs:206 |
| P6.5 Complete widgets | 2w | libsarga |
| P6.6 Eliminate GUI unwraps | 2d | 12 GUI apps |

### Phase 7: Init & Services (Sprint 8) — 4 pw

| Task | Effort | Target |
|------|--------|--------|
| P7.1 Read /etc/init.toml | 1w | init |
| P7.2 Service dependencies | 2d | init |
| P7.3 Logging | 3d | init+syslog |
| P7.4 Shutdown support | 2d | init |
| P7.5 Verify login flow | 3d | login-manager |
| P7.6 Complete vahid | 1w | vahid |

### Phase 8: Tooling (Sprint 9) — 4 pw

| Task | Effort | Target |
|------|--------|--------|
| P8.1 Complete package mgr | 1w | spkg/skypkg |
| P8.2 Complete complex utils | 2w | gzip/tar/sed/awk/diff/patch |
| P8.3 Tar/gzip interop | 2d | coreutils |
| P8.4 Complete skybuild | 2d | skybuild |

### Phase 9: Security (Sprints 9-10) — 4 pw

| Task | Effort | Target |
|------|--------|--------|
| P9.1 Verify password hashing | 3d | login+passwd |
| P9.2 Proper salt generation | 1d | passwd.rs:52 |
| P9.3 Fix password echo | 1d | login.rs:149 |
| P9.4 Capability system | 2w | Kernel+libsarga |
| P9.5 Process isolation | 1w | Kernel |
| P9.6 File permissions | 1w | Kernel |

### Phase 10: Testing (Sprints 10-11) — 8 pw

| Task | Effort | Target |
|------|--------|--------|
| P10.1 Unit tests for libsarga | 2w | libsarga |
| P10.2 QEMU integration tests | 2w | tests/ |
| P10.3 Kernel self-tests | 2w | Kernel |
| P10.4 Memory stress | 1w | Kernel+libsarga |
| P10.5 Scheduler stress | 1w | Kernel |
| P10.6 Network tests | 1w | nettools |
| P10.7 FS stress | 1w | Kernel |
| P10.8 Regression suite | 1w | All |

### Phase 11: Performance (Sprint 11) — 4 pw

| Task | Effort | Target |
|------|--------|--------|
| P11.1 Boot time | 1w | Full stack |
| P11.2 Memory allocator | 1w | Kernel |
| P11.3 GUI rendering | 1w | gui.rs |
| P11.4 Benchmark harness | 1w | tests/ |

### Phase 12: Documentation (Sprint 12) — 4 pw

| Task | Effort | Target |
|------|--------|--------|
| P12.1 Right-size docs | 1w | docs/ + README |
| P12.2 Getting started guide | 3d | docs/guide |
| P12.3 Architecture decisions | 2d | New file |
| P12.4 Syscall contract doc | 2d | docs/syscalls |
| P12.5 CI/CD docs | 1d | docs/build |
| P12.6 Module doc comments | 3d | All .rs files |

### Phase 13: CI/CD (Ongoing) — 4 pw

| Task | Effort | Target |
|------|--------|--------|
| P13.1 Clippy in CI | 1d | workflows |
| P13.2 Rustfmt in CI | 1d | workflows |
| P13.3 Release pipeline | 2w | workflows |
| P13.4 Code quality dashboard | 1w | CI |
| P13.5 Dependabot | 1d | .github |

### Phase 14: Portability (Backlog) — 8+ pw

| Task | Effort | Target |
|------|--------|--------|
| P14.1 aarch64 syscall wrappers | 2w | libsarga/syscall.rs |
| P14.2 aarch64 linker fixes | 1w | aarch64-sarga.ld |
| P14.3 Verify aarch64 builds | 1w | CI |
| P14.4 Dynamic linking | 2w | New crate |
| P14.5 RISC-V target | 4w | New target spec |

**Total: ~72-80 person-weeks (1.5-2 person-years)**

---

## SECTION 7: VERDICT & RECOMMENDATION

### Should a Developer Use This Today?

**No. Absolutely not.**

1. Cannot build a bootable system (kernel is in undocumented external repo)
2. Cannot reach a shell prompt (no QEMU log shows userspace running)
3. Cannot write safe programs (100+ undocumented unsafe, 21 unwrap panics)
4. Cannot trust documentation (aspirational, contradictory)
5. Cannot contribute effectively (two shells, two pkg mgrs, zero tests)
6. Cannot run on real hardware (SMP broken, only VBE/QEMU tested)

### Most Critical Thing to Fix

**Phase 0 + Phase 1: Make the system boot to a shell.**

1. Find the kernel repo and document its location
2. Verify the syscall contract between kernel and libsarga
3. Get a bootable image that reaches a login prompt
4. Fix the immediate safety issues (pointer truncation, unsafe documentation)

### Is This Project Viable Long-Term?

**Possibly, but it needs a realistic reset.** The project demonstrates impressive breadth but lacks depth in every dimension. 1.5-2 person-years of skilled Rust systems engineering is needed. Realistically, this is 2-3 years of part-time work by a single developer to reach "usable hobby OS" status.

### What Would Make It Worth Using

1. The kernel exists, boots, and provides the documented syscall surface
2. Safety issues are addressed (unsafe documented, unwrap eliminated)
3. The dual-everything confusion is resolved (one shell, one pkg mgr, one error convention)
4. AI-generated code is human-reviewed and hardened
5. A real test suite exists and passes in CI
6. Documentation reflects reality, not aspirations
7. The build pipeline works from a single checkout

### Final Verdict

> **Treat this as a learning resource and starting point, not a usable system. The concepts are sound, the breadth is impressive, but the depth, safety, and correctness work is 90% unfinished. Any developer considering using or contributing to this project should start with Phase 0 and Phase 1 — and be prepared for a multi-year journey.**

---

*Report generated July 2026 by systematic codebase analysis.*
*Kernel source was not available for review — the Vahi kernel is in a separate, external repository.*
*38 crates, 200+ source files, 100+ unsafe blocks, 21 unwrap calls, 0 TODO comments, 10 unsound patterns audited.*
