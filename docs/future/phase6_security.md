# Phase 6: Security Features

Phase 6 adds comprehensive security mechanisms to SkyOS.

## Already Implemented

Some items in the goals below already exist (see `docs/security/`):

- **SMEP/SMAP** — enabled conditionally at boot (see `docs/security/memory_protection.md`)
- **Stack canaries** — `-Z stack-protector=strong` on the kernel
- **MAC** — rule-based LSM (`security.rs`) hooks file/socket/exec/mount/kill
- **Audit** — `audit_log()` records capability denials
- **Capabilities** — Linux-style capability bits + `capset`/`capget`

## Goals

- Address Space Layout Randomization (ASLR)
- Kernel ASLR (KASLR)
- Stack canaries for buffer overflow detection
- Control Flow Integrity (CFI)
- Signed kernel modules
- Audit subsystem
- Mandatory Access Control (MAC)

## Key Milestones

1. **ASLR**: Randomize userspace memory layout (stack, heap, mmap base, executable base)
2. **KASLR**: Randomize kernel base address at boot time
3. **Stack canaries**: Compiler-inserted canary values to detect stack corruption
4. **CFI**: Forward-edge and backward-edge control flow integrity
5. **Module signing**: Cryptographic verification of kernel modules
6. **Audit**: Comprehensive security event logging
7. **MAC**: Implement a SELinux-like mandatory access control framework

## Threat Model

The security architecture defends against:
- Local privilege escalation
- Code injection
- Return-oriented programming (ROP) attacks
- Kernel module tampering
- Side-channel attacks (future)

## Security Hardening

Additional hardening measures (KPTI is not planned — the kernel uses a shared higher-half mapping, so page-table isolation is not a Meltdown mitigation for this layout):
- W^X enforcement for kernel memory

## Expected Timeline

3-4 months (ongoing, security is a continuous effort).
