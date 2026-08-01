# Capability and Object Security Model

SkyOS combines a Linux-compatible capability bitmask with per-object security descriptors and
ACLs. The unified check lives in `objects/security.rs` (`access_check`).

## Credentials

Every process carries a snapshot of its credentials used for access checks:

```rust
pub struct Credentials {
    pub uid: u32, pub gid: u32,
    pub euid: u32, pub egid: u32,
    pub fsuid: u32, pub fsgid: u32,
    pub cap_effective: u64,
}
```

## Security Descriptors

Every kernel object carries a `SecurityDescriptor` with an owner, group, mode bits, and an optional
ACL:

```rust
pub struct SecurityDescriptor {
    pub uid: u32,
    pub gid: u32,
    pub mode: u32,          // 0o777 DAC bits
    pub acl: Vec<Ace>,
}

pub struct Ace { pub ace_type: AceType, pub flags: u8, pub access_mask: u32, pub uid: u32 }
pub enum AceType { Allow, Deny }
```

## access_check

`access_check(cred, sec, desired)` grants access when **all** of these pass:

1. **Root bypass**: `euid == 0`
2. **DAC**: owner/group/other mode bits match the effective UID/GID
3. **Capability override**: `CAP_DAC_OVERRIDE` (bypass all), `CAP_DAC_READ_SEARCH` (read/search
   only)
4. **ACL**: first matching `Ace` for the caller's UID decides Allow/Deny (only consulted if the
   requested bits are set in `access_mask`)

There is no separate "ACL check" that runs after DAC is satisfied; the ACL is consulted only when
the DAC bits do not already grant access.

## Capability Bitmask

Beyond the DAC overrides above, capabilities are a single `u64` bitmask with Linux-compatible
positions (`CAP_KILL = 1<<5`, `CAP_NET_RAW = 1<<13`, `CAP_SYS_ADMIN = 1<<21`, `CAP_SYS_BOOT =
1<<22`, etc.). Their use for gating privileged syscalls is documented in
`docs/security/syscall_security.md`.

## LSM / MAC

Rule-based MAC checks hook into `open`, `mkdir`, `socket`, `kill`, `mount`, and `execve` and can
deny operations the DAC would otherwise allow.
