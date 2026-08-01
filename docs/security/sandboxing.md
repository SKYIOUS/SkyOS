# Process Sandboxing and Isolation

SkyOS provides multiple layers of isolation to contain processes and limit the impact of security compromises. This document describes the sandboxing mechanisms currently implemented and those planned.

## Process Isolation

Every process in SkyOS runs in its own address space with hardware-enforced isolation. The kernel uses x86-64 paging structures with per-process page tables, ensuring that no process can access another process's memory without explicit kernel-mediated sharing. The kernel enforces SMEP (Supervisor Mode Execution Prevention) and SMAP (Supervisor Mode Access Prevention) to prevent execution of userspace code in kernel mode and to prevent the kernel from accessing userspace memory without explicit intent.

## Capability Confinement

Each process holds a credentials set (`Credentials` in `objects/security.rs`) with effective/real/saved uid/gid plus a capability bitmask (`cap_effective`). `has_capability(bit)` in `syscalls/mod.rs` returns true when `euid == 0` **or** the bit is set — so root bypasses capability checks. `capset`/`capget` (syscalls 308/307) manage the bitmask; `capset` is root-only. Denials are recorded via `audit_log()`.

## Driver Isolation

Drivers are built into the kernel and run with kernel privileges; there is no driver sandboxing. The kext framework (`kernel/kernel/src/kext/`) matches PCI/USB/platform nubs to driver families, but memory/DMA access is not isolated per driver.

## Userspace Sandboxing (Planned)

Future releases will introduce additional userspace sandboxing features:

- **Seccomp-style syscall filtering**: Allow processes to restrict which system calls they may invoke.
- **Namespace isolation**: Process-local views of filesystem mounts, process IDs, and IPC objects.
- **Resource limits**: Per-process caps on memory, CPU time, file descriptors, and I/O bandwidth.
- **Network sandboxing**: Per-process firewall rules controlling network access.

These features build on the existing capability infrastructure and extend the principle of least privilege to every aspect of process execution.

## Audit and Monitoring

The kernel records security-relevant events via `audit_log()` (capability denials for mount, swap, chmod/chown, kill, etc.). Future work extends this into a full audit trail: capability checks, denied system calls, and process compartment boundary crossings.
