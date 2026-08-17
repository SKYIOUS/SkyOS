# SMP Support Design

SkyOS supports symmetric multiprocessing (SMP) across all available CPU cores, with architecture-specific initialization on x86_64.

## BSP and AP Startup

The bootstrap processor (BSP) initializes the kernel and wakes application processors (APs) using the SIPI (Startup IPI) protocol. Each AP executes a small trampoline routine loaded at `0x8000` (code; per-CPU data at `0x7000`) — below 1 MiB — that transitions it to protected mode, then long mode, and finally jumps to the Rust kernel entry point.

```rust
// smp.rs: TRAMPOLINE_PHYS = 0x8000 (code), DATA_PHYS = 0x7000 (per-AP data)
pub fn wake_aps(bsp: &mut Cpu) {
    // copy smp_trampoline_start..end (global_asm!) to TRAMPOLINE_PHYS
    for ap_id in 1..num_cpus() {
        send_sipi(ap_id, TRAMPOLINE_PHYS);
        wait_for_ap_ready(ap_id);
    }
}
```

## Per-CPU Data

Each CPU has a per-CPU data region accessible via the `GS` segment base. This region contains:
- CPU ID and feature flags
- Local APIC registers
- Per-CPU allocator cache
- Current task pointer
- Interrupt stack pointers

## CPU Hotplug

CPU hotplug via ACPI is **not implemented**. APs are brought up once at boot with SIPI; no hot-add/remove path exists.

## Load Balancing

The scheduler uses work-stealing to balance load across CPUs. When a CPU's stride heap is empty, it steals from other CPUs' heaps (up to 3 attempts) and from the global `pending_queue` of newly spawned threads.
