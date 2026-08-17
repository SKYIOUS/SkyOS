# Planned Security Features

This page documents security features planned for future implementation.

## ASLR (Address Space Layout Randomization)

Partially implemented. Stack ASLR is real: `setup_user_stack()` in `task/process.rs` randomizes the stack base within a 64 MiB window below `0x7FFF_F000_0000` using RDTSC-based entropy.

Still planned:
- Heap base randomization
- mmap base randomization
- Executable base (PIE) randomization
- vDSO randomization

## KASLR (Kernel ASLR)

KASLR randomizes the kernel's base virtual address at boot time. The kernel image is decompressed at a random offset within a 1 GiB range. Page table structures are also randomized.

## Stack Canaries

Already implemented — the kernel builds with `-Z stack-protector=strong` (both profiles), with `__stack_chk_guard`/`__stack_chk_fail` provided by the kernel. The compiler inserts canary checks:

```rust
// Compiler generates:
fn vulnerable_function() {
    let canary = __stack_chk_guard;
    let buf: [u8; 64] = [0; 64];
    // ... function body ...
    if canary != __stack_chk_guard {
        __stack_chk_fail();
    }
}
```

## Control Flow Integrity (CFI)

Forward-edge CFI validates indirect function calls against a valid target list. Backward-edge CFI uses shadow stacks to protect return addresses. The kernel will support both compiler-based CFI and hardware-assisted CET (Control-flow Enforcement Technology).

## Mandatory Access Control (MAC)

A rule-based MAC (LSM) framework is already implemented — `security.rs` hooks file/socket/exec/mount/kill decisions. Future work extends it toward a SELinux-style policy engine:
- Type enforcement for process-to-resource access
- Role-based access control (RBAC)
- Multi-level security (MLS)
- Policy loading at boot time

## Audit Subsystem

A basic audit log already exists — `audit_log()` in `syscalls/mod.rs` records capability denials (mount, swapon/swapoff, chmod, kill, etc.). Future work adds comprehensive event logging:
- Syscall audit trail (configurable per syscall)
- File access monitoring
- Process creation and termination tracking
- Network connection logging
- Secure log storage (tamper-evident)

## Signed Kernel Modules

Kernel modules must be cryptographically signed:
- Kernel contains the public key
- Module signatures are verified before loading
- Unsigned modules are rejected (configurable)
- Hardware-backed key storage (TPM)
