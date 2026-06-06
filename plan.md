# SkyOS — Production Readiness Plan

> **Vision:** Turn Vahi Kernel + SkyOS userland into a production-quality operating system.

---

## Phase 1 — Stabilize (fix what's broken)

These are **blockers** — code that won't compile or is fundamentally broken.

### P0 — Must fix immediately

| Task | File(s) | Issue |
|------|---------|-------|
| Fix `libsky` → `libsarga` imports | `aicli/src/main.rs:4`, `ade/src/main.rs:4`, `sash/src/builtins.rs:3`, `sash/src/executor.rs:2`, `sash/src/kor_bridge.rs:2` | Import nonexistent crate `libsky` |
| Fix `println!` double-format bug | `libsarga/src/io.rs:57` | `println!` evaluates `format!` twice |
| Implement `SYS_CONNECT` | kernel `syscalls/mod.rs` | Stub returning 0 |
| Wire socket read/write | kernel `syscalls/mod.rs` | Returns `ENOSYS` for socket fds |
| Implement `SYS_RENAME` | kernel `syscalls/numbers.rs` + `mod.rs` | Missing entirely |
| Fix `$MOuck` typo | `disk/create_disk.sh:40` | `$MOuck` → `$MOUNT` |

### P1 — Should fix soon

| Task | File(s) | Issue |
|------|---------|-------|
| Fix build target | `build.sh:21` | Uses `x86_64-unknown-none` instead of `x86_64-sarga` |
| Add user pointer validation | `kernel/src/syscalls/mod.rs` | Several syscalls bypass SMAP/`copy_from_user` |
| Fix `nettools` return types | `nettools/src/*.rs` | `fn user_main() -> i32` vs `sarga_main!` expecting `fn()` |
| Remove stale docs | `README.md` | References "Aethos OS" and "Velox Kernel" |

---

## Phase 2 — Core Userland

The kernel works. The userland is where users live.

### 2A — Core utilities (12+ real implementations)

Current: `cat` works. Everything else is a stub.

| Utility | Priority | What it needs |
|---------|----------|---------------|
| `ls` | P0 | Directory listing via `getdents64`, format output, flags `-l -a -h` |
| `mkdir` | P0 | `-p` (parents), error handling |
| `rm` | P0 | `-r` (recursive), `-f` (force) |
| `cp` | P0 | File copy, `-r` for dirs, preserve mode |
| `mv` | P0 | Rename via `SYS_RENAME`, cross-fs copy+delete |
| `echo` | P0 | Print args, `-n` flag |
| `grep` | P1 | Line matching, `-r`, `-i`, `-v` |
| `head`/`tail` | P1 | Line count, `-n`, `-f` (tail follow) |
| `wc` | P1 | Line/word/byte count |
| `sleep` | P1 | `sys_nanosleep` wrapper |
| `kill` | P1 | Send signals by PID/name |
| `ps` | P2 | Process list via `sys_sysinfo` or `/proc` |
| `df` | P2 | Filesystem usage via `statvfs` |
| `date` | P2 | `clock_gettime` + formatting |

### 2B — Shell (`sargash` / `sash`)

Current: reads chars, splits on whitespace, fork+exec. No pipes, no quoting, no PATH search.

| Feature | Priority | Description |
|---------|----------|-------------|
| Pipes (`|`) | P0 | Chain commands via pipe fds |
| Redirections (`>`, `>>`, `<`) | P0 | Stdout/stdin redirect to files |
| Environment variables | P0 | `$VAR` expansion, `export`, `unset` |
| PATH search | P0 | Search `$PATH` for executables |
| Quoting | P0 | Single/double quotes, escape chars |
| Job control | P1 | `&`, `fg`, `bg`, `jobs`, SIGTSTP |
| Tab completion | P1 | File/command name completion |
| History | P2 | Up-arrow recall, `~/.history` |
| Scripting | P2 | `if`, `for`, `while`, functions |

### 2C — Init system

Current: reads `/etc/init.cfg`, spawns `login` binary, respawns on exit.

| Feature | Priority | Description |
|---------|----------|-------------|
| Signal handling | P0 | SIGTERM → reboot, SIGINT → Ctrl-C passthrough |
| Runlevels/targets | P1 | Multi-user vs single-user vs rescue |
| Service spawning | P1 | Daemon management, pid tracking |
| Orphan reaping | P1 | Adopt orphaned children |
| `/etc/inittab` | P2 | Configurable init table |
| Shutdown/reboot | P2 | `sys_reboot` syscall + init coordination |

### 2D — Package manager (`spkg`)

Current: prints "usage" and exits.

| Feature | Priority | Description |
|---------|----------|-------------|
| Package format | P1 | Define `.spkg` tar-based format |
| Install | P1 | Extract to filesystem, register metadata |
| Remove | P1 | Uninstall + dependency check |
| List | P1 | List installed packages |
| Dependency resolution | P2 | Satisfy deps from repo |
| Repository support | P2 | Remote package repos |

---

## Phase 3 — Network Stack

### 3A — Kernel network improvements

| Task | Priority | Description |
|------|----------|-------------|
| TCP connect | P0 | Implement `sys_connect` for TCP via smoltcp |
| TCP recv | P0 | Wire socket read path for TCP |
| `sys_listen` / `sys_accept` | P1 | TCP server support |
| DHCP | P1 | Replace hardcoded 10.0.2.15 |
| DNS caching | P2 | Cache resolved names to avoid repeated queries |
| ARP management | P2 | Expose ARP table |
| IPv6 | P3 | Dual-stack support |

### 3B — Userspace networking (`nettools` + `libsarga`)

Current: `ifconfig`, `curl`, `nc` are stubs. `libsarga` has no socket wrappers.

| Task | Priority | Description |
|------|----------|-------------|
| `libsarga` socket wrappers | P0 | `connect`, `send`, `recv`, `bind`, `listen`, `accept` |
| `ping` | P1 | ICMP echo via raw socket |
| `ifconfig` | P1 | Display NIC info, set IP |
| `curl` | P2 | HTTP/HTTPS client |
| `nc` | P2 | Netcat (TCP/UDP) |

---

## Phase 4 — Drivers & Hardware

### 4A — Complete existing drivers

| Driver | Priority | What's needed |
|--------|----------|---------------|
| AHCI | P1 | Verify/complete DMA command path, test on real disks |
| XHCI | P1 | Complete device enumeration (keyboard, mouse, storage) |
| HDA audio | P2 | PCM playback via Intel HDA |
| RTC (CMOS) | P1 | Read real-time clock for `clock_gettime` |

### 4B — New drivers

| Driver | Priority | Rationale |
|--------|----------|-----------|
| NVMe | P2 | Common on modern hardware, not just QEMU |
| RTL8139 | P2 | QEMU default NIC; E1000 must be manually selected |
| PS/2 → USB HCI | P3 | USB keyboard/mouse support |
| MSI/MSI-X | P2 | Replace line-based IRQs for performance |

---

## Phase 5 — Filesystem & Storage

| Task | Priority | Description |
|------|----------|-------------|
| `rename` syscall | P0 | Missing entirely |
| Ext2 journaling | P2 | Basic journal recovery for crash safety |
| Real `/proc` | P2 | Replace AI-generated content with kernel state |
| `sys_select`/`poll` | P1 | I/O multiplexing for servers |
| File-backed mmap | P1 | Required for dynamic linking |
| `inotify` | P3 | File event monitoring |

---

## Phase 6 — Security & Hardening

| Feature | Priority | Description |
|---------|----------|-------------|
| User accounts | P1 | `/etc/passwd`, `/etc/shadow`, login, uid/gid syscalls |
| Permission checks | P1 | Enforce file permissions in VFS |
| KASLR | P2 | Randomize kernel base at boot |
| Heap hardening | P2 | Guard pages, free list poisoning |
| Stack canaries | P2 | `-fstack-protector` for kernel |
| OOM handling | P2 | Kill offending process instead of panic |
| Capabilities | P3 | POSIX capabilities for fine-grained privileges |

---

## Phase 7 — Polish & Production

| Feature | Priority | Description |
|---------|----------|-------------|
| Dynamic linker | P2 | Userspace `ld.so` for shared libraries |
| PIE testing | P2 | Verify PIE executables work end-to-end |
| `futex` hashing | P2 | O(1) wake instead of linear scan |
| Per-CPU memory pools | P2 | Reduce allocator contention on SMP |
| Comprehensive tests | P1 | Unit tests + integration tests for every syscall |
| CI/CD | P1 | GitHub Actions: build + QEMU boot test on every commit |
| GDB/kgdb stub | P3 | Kernel debugging support |
| Performance profiling | P3 | `perf` equivalent |
| Documentation | P2 | Man pages for all utilities |

---

## Legend

| Priority | Meaning |
|----------|---------|
| **P0** | Blocks compilation or core functionality — must fix now |
| **P1** | Needed for a usable system — high priority |
| **P2** | Important for production-readiness |
| **P3** | Nice-to-have / stretch goal |

## Current state snapshot

- **Kernel:** ~25,000 lines of Rust, 45+ syscalls, boots to userspace ✅
- **Userland:** ~4,000 lines of Rust, init + shell + 25 utilities
- **Working on real hardware:** Not yet (QEMU only)
- **Build:** Rust nightly, x86_64, UEFI boot
