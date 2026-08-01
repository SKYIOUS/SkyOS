# Syscall Permission Model

System calls are subject to permission checks before execution. SkyOS combines Linux-compatible
POSIX credentials and capabilities with a rule-based LSM (`objects/security.rs`).

## Capabilities

Capabilities are a `u64` bitmask with **Linux-compatible bit positions** (defined in
`syscalls/mod.rs`). `has_capability(bit)` returns true when the caller's effective UID is root or
the bit is set in `cap_effective`:

```rust
pub const CAP_CHOWN: u64 = 1 << 0;
pub const CAP_DAC_OVERRIDE: u64 = 1 << 1;
pub const CAP_DAC_READ_SEARCH: u64 = 1 << 2;
pub const CAP_FOWNER: u64 = 1 << 3;
pub const CAP_FSETID: u64 = 1 << 4;        // defined, unused
pub const CAP_KILL: u64 = 1 << 5;
pub const CAP_SETUID: u64 = 1 << 6;
pub const CAP_SETGID: u64 = 1 << 7;
pub const CAP_SETPCAP: u64 = 1 << 8;       // defined, unused
pub const CAP_NET_BIND_SERVICE: u64 = 1 << 10;   // defined, unused
pub const CAP_NET_ADMIN: u64 = 1 << 12;    // defined, unused
pub const CAP_NET_RAW: u64 = 1 << 13;
pub const CAP_SYS_ADMIN: u64 = 1 << 21;
pub const CAP_SYS_BOOT: u64 = 1 << 22;
```

(`CAP_SETUID`/`CAP_SETGID` are deliberately swapped vs. Linux positions 6/7; see the comment in
`syscalls/mod.rs`.)

## DAC Checks

File access uses `check_file_permission()` (`syscalls/mod.rs`): owner/group/other mode bits are
matched against the caller's effective UID/GID, with `CAP_DAC_OVERRIDE` bypassing DAC entirely and
`CAP_DAC_READ_SEARCH` bypassing read/search only. `CAP_FOWNER` bypasses ownership checks for
permission-changing operations.

## Privileged Operations

These operations are gated (each denial is `audit_log`'d):

| Operation | Gate |
|-----------|------|
| `mount` / `umount2` | `CAP_SYS_ADMIN` |
| `mkfs` | `CAP_SYS_ADMIN` |
| `swapon` / `swapoff` | `CAP_SYS_ADMIN` |
| process control / wait of non-child | `CAP_SYS_ADMIN` |
| `reboot` | `CAP_SYS_BOOT` |
| `kill` of another user's process | `CAP_KILL` |
| `socket(SOCK_RAW)` | `CAP_NET_RAW` (raw is backed by an ICMP socket) |
| `capset` | root / `CAP_SETPCAP` |

## LSM / MAC

`objects/security.rs` implements a rule-based mandatory access control layer with hooks in
`open`, `mkdir`, `socket`, `kill`, `mount`, and `execve`. Rules can deny an operation even when the
DAC check would allow it.

## Signals

Signals are not a gate; every process carries a pending/blocked bitmask and up to 32 handlers.
`sigprocmask` (syscall 309) changes the mask, and delivery happens in the syscall postamble.
