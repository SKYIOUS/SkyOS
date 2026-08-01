# Design Philosophy and Goals

SkyOS is built on a set of core design principles that guide every architectural decision.

## Core Principles

### 1. Safety First

Memory safety is the #1 priority. By using Rust as the implementation language, we eliminate buffer overflows, use-after-free, and many other classes of vulnerabilities at compile time. Unsafe code is minimized, reviewed extensively, and documented with `// SAFETY:` invariants (enforced as a convention in AGENTS.md).

### 2. Monolithic, Higher-Half Kernel

SkyOS is a monolithic kernel: scheduling, memory, IPC, filesystems, network stacks, and drivers all run in kernel space at the higher half (`0xFFFFFFFF80000000`). This favors low overhead over isolation — a driver bug can crash the kernel. See `docs/architecture/overview.md`.

### 3. Minimalism

The kernel provides the essentials: scheduling (stride, per-CPU with work-stealing), memory management (buddy + slab), a trait-based VFS, syscalls, and security hooks. Subsystems are trait-based (`VfsNode`, `BlockDevice`) to keep the core generic without moving drivers out of kernel space.

### 4. POSIX Credentials + Capabilities

Security follows the POSIX model (uid/gid/euid/egid + DAC permission bits) layered with Linux-style capability bits (`CAP_NET_RAW`, `CAP_SYS_ADMIN`, ...). `has_capability` returns true for `euid == 0`, so root bypasses capability checks — this is the classic Unix model, not capability-based security. A rule-based MAC (LSM) in `security.rs` adds mandatory hooks on top. See `docs/security/`.

### 5. Performance without Compromise

The per-CPU stride scheduler with work-stealing, zero-cost Rust abstractions, and a fixed physical-memory offset keep overhead low.

## Design Goals

- **Correctness**: Race-free execution via disciplined locking (`spin::Mutex`, `SchedLock`)
- **Performance**: Competitive with Linux for common workloads
- **Simplicity**: Clean, well-documented code that is easy to understand
- **Extensibility**: Easy to add new drivers, filesystems, and syscalls (see `docs/guide/adding_syscall.md`)
- **Security**: DAC + capabilities + LSM MAC with audit logging
