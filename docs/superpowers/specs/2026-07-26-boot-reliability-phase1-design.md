# Phase 1: Boot Reliability — Design Doc

**Date:** 2026-07-26
**Status:** Draft (revised)
**Goal:** Make SkyOS reliably boot from UEFI to a userspace shell prompt.

## Problem Statement

The Vahi kernel loads the `init` ELF binary, creates an address space, maps a stack, activates the page tables, then **deliberately halts** at `kernel/src/main.rs:466`:

```rust
crate::serial_write("[INIT] HALTING (debug)\n");
loop { crate::arch::CurrentArch::halt(); }
```

No userspace process ever executes. Every other subsystem (shell, desktop, networking, compositor, drivers) is untestable because the system never transitions out of kernel mode into a running userspace.

## Scope

**In scope (must-have for Phase 1):**
1. Replace the debug halt with a proper transition to userspace (PID 1)
2. Implement a boot state machine with structured diagnostics and typed errors
3. Verify console I/O (`/dev/tty0` or serial fallback) works for init
4. Ensure the scheduler continues running after entering userspace
5. Add an automated QEMU boot smoke test that verifies userspace execution

**Out of scope (explicitly deferred):**
- ASLR / KASLR
- Dynamic linker (`ld.so`)
- Swap
- Signal handling edge cases (beyond basic SIGCHLD)
- Security hardening (beyond what already exists)
- Performance optimization
- Writable root filesystem (tmpfs root is fine for now)
- Desktop Environment boot (separate phase)

## Architecture: Boot State Machine

Replace the flat `init_os_task()` with an explicit state machine using typed states, a shared context, and structured error types.

### Types

```rust
// ── State machine ──

enum BootState {
    InitKernel,          // After scheduler init
    LocateInit,          // Searching /bin/init, /init, /sbin/init
    ParseElf,            // Validating and loading ELF
    CreateAddressSpace,  // Allocating page table hierarchy
    MapStack,            // Setting up user stack with guard page
    CreatePid1,          // Registering process, assigning PID 1
    SetupConsole,        // Opening stdin/stdout/stderr
    EnterUserspace,      // Final transition to user mode
    Running,             // PID 1 executing
    Failed(BootError),   // Terminal failure
}

// Valid transitions: InitKernel → LocateInit → ParseElf →
// CreateAddressSpace → MapStack → CreatePid1 → SetupConsole →
// EnterUserspace → Running. Any other transition is a bug.

impl BootState {
    /// Returns the set of states this state may validly transition to.
    fn valid_next(&self) -> &[BootState];
}

// ── Errors and warnings ──

enum BootError {
    InitNotFound,
    InvalidElf,
    AddressSpaceCreationFailed,
    StackAllocationFailed,
    ConsoleUnavailable,
    UserspaceEntryFailed,
}

enum BootWarning {
    ConsoleUnavailable,   // /dev/tty0 missing, falling back to serial
    EntropySourceMissing, // /dev/random not available
}

// ── Context: split persistent vs transient ──

/// Persistent data live across the entire boot lifetime.
/// After PID 1 starts, this is archived in case a panic
/// trace is needed.
struct BootContext {
    trace: Vec<BootEvent>,
    init_paths_tried: Vec<String>,
    boot_start_tick: u64,
}

/// Transient objects only needed while launching PID 1.
struct BootSession {
    elf_data: Vec<u8>,
    process: Option<Arc<Process>>,
    address_space: Option<AddressSpace>,
    entry_point: u64,
    user_rsp: u64,
}

// ── Event log ──

enum BootEvent {
    Enter(BootState),
    Exit(BootState),
    Warning(BootWarning),
    Error(BootError),
}
```

### Flow

```
InitKernel
  ↓
LocateInit  ── InitNotFound ──→ Failed(InitNotFound)
  ↓ (found)
ParseElf  ── InvalidElf ──→ Failed(InvalidElf)
  ↓ (valid)
CreateAddressSpace  ── failure ──→ Failed(AddressSpaceCreationFailed)
  ↓
MapStack  ── failure ──→ Failed(StackAllocationFailed)
  ↓
CreatePid1
  ↓
SetupConsole  ── ConsoleUnavailable ──→ warning, continue with serial
  ↓
EnterUserspace  ── failure ──→ Failed(UserspaceEntryFailed)
  ↓
Running
```

### Transition Behavior

The transition to userspace reuses the existing architecture-specific entry mechanism (the same path used by `spawn_userspace_app()`). The design describes the required behavior — perform an atomic, correct transition to user mode with the new address space active and the scheduler able to preempt the new process — not a specific function name.

The state machine itself drives transitions:
```rust
loop {
    // Validate transition legality
    debug_assert!(state.valid_next().contains(&next));
    let next = state.run(&mut context, &mut session)?;
    state = next;
}
```
The current state is owned by the machine, not stored in `BootContext`.
Illegal transitions (e.g. `LocateInit` → `Running`) are caught by `valid_next()`.

### Diagnostics

Timestamps in milliseconds since boot:
```
[0.013] BOOT  InitKernel
[0.015] BOOT  LocateInit /bin/init
[0.016] BOOT  ParseElf entry=0x400000
[0.019] BOOT  CreateAddressSpace pml4=0x1A2B000
[0.021] BOOT  MapStack 0x7FFFFF000
[0.022] BOOT  CreatePid1 pid=100
[0.023] BOOT  SetupConsole /dev/tty0
[0.024] BOOT  EnterUserspace
[0.025] BOOT  Running
```

On failure:
```
[0.016] BOOT  ERROR InitNotFound
[0.016] BOOT  Trace: InitKernel → LocateInit → Failed(InitNotFound)
```

### Assertions

After every major state transition, insert debug assertions:
- After `CreateAddressSpace`: `assert!(address_space.is_some())`
- After `CreatePid1`: `assert_eq!(process.pid, 1)` (Phase 1 assumes PID 1 is the first userspace process; this may change with future kernel helper threads)
- After `EnterUserspace`: verify execution has entered user mode according to the current architecture, and that the stack pointer is non-zero

These catch corruption immediately and are stripped in release builds.

### Boot Trace

The `BootContext.trace` vector records every state transition and warning. On panic, the panic handler performs this sequence:
1. Dump `BootContext.trace` (the boot event log)
2. Dump CPU registers (rax, rbx, cr2 if page fault, etc.)
3. Dump page-fault info (if applicable: faulting address, error code)
4. Dump stack trace
5. Halt CPU

This turns intermittent boot failures from "it hung" into "it failed at EnterUserspace".

## Error Handling Policy

| Failure | Severity | Action |
|---------|----------|--------|
| Init not found | Fatal | Panic with `BootError::InitNotFound` |
| Invalid ELF | Fatal | Panic with `BootError::InvalidElf` |
| Address space creation | Fatal | Panic with `BootError::AddressSpaceCreationFailed` |
| Stack allocation | Fatal | Panic with `BootError::StackAllocationFailed` |
| `/dev/tty0` missing | Recoverable | Print warning, continue with serial |
| `/dev/random` missing | Recoverable | Print warning, continue |
| Userspace entry | Fatal | Panic with `BootError::UserspaceEntryFailed` |

Recoverable failures produce a timestamped `[BOOT] WARNING` line but do not halt.

## Code Changes

### 1. `kernel/src/main.rs` — `init_os_task()`

Introduce `BootState`, `BootError`, `BootContext` types and refactor the init logic into a `run_boot_state_machine()` function that drives the states above. Each state is a separate function (`boot_locate_init()`, `boot_parse_elf()`, etc.) that takes `&mut BootContext` and returns `Result<BootState, BootError>`.

The existing ELF loading, address space creation, and stack setup code is reused. The key structural change: remove the debug halt and wire the state machine's terminal transitions.

### 2. Boot diagnostics

A `BootLogger` struct with `info()`, `warn()`, `error()` methods that prepend `[{ms}] BOOT` timestamps from the RTC/timer tick count. Internally it writes to serial (preferred) or the active console. The interface is independent of the output backend — later it can also log to the framebuffer, kernel ring buffer, or a remote debugger without changing callers.

### 3. `tests/` — QEMU smoke test

A test script that:
1. Builds kernel + initrd
2. Boots in QEMU with `-nographic`
3. Verifies PID 1 entered userspace and executed its first syscall (detected via serial output or QEMU's ISA debug console)
4. Timeouts after 30 seconds → FAIL
5. Exits 0 on success

This verifies actual userspace execution (a userspace process running and making a syscall), not just a kernel log line.

### 4. Regression tests (future, design for now)

| Test | Purpose | Phase |
|------|---------|-------|
| `BootSuccess` | PID 1 starts, first syscall succeeds | 1 |
| `MissingInit` | Proper `BootError::InitNotFound` | 2 |
| `InvalidElf` | `BootError::InvalidElf` on corrupted binary | 2 |
| `BrokenStack` | `BootError::StackAllocationFailed` on OOM | 2 |
| `MissingTTY` | Falls back to serial, init runs | 2 |
| `SchedulerAlive` | Timer continues after entering userspace | 2 |

## Implementation Order

| Step | Change | Verification |
|------|--------|-------------|
| 1 | Add `BootState`, `BootError`, `BootWarning`, `BootContext`, `BootSession`, `BootEvent` types | Compiles |
| 2 | Add `BootLogger` with timed `info/warn/error` methods | Compiles |
| 3 | Refactor `init_os_task()` into state machine with transition validation | Compiles |
| 4 | Wire `EnterUserspace` transition (reuse arch entry mechanism) | Boots to serial output |
| 5 | Fix any boot-time crashes | Init process runs, PID == 1 |
| 6 | Add console I/O fallback if `/dev/tty0` missing | Init runs on serial |
| 7 | Write QEMU smoke test (syscall-based PID 1 verification) | `./test_boot.sh` passes 3x |

## Risks

- **ELF loading bug:** The ELF loader may not handle the init binary correctly. Mitigation: `BootError::InvalidElf` with detailed reason.
- **Page fault on context switch:** The address space switch or stack setup may be wrong. Mitigation: boot assertions catch this immediately; the existing `spawn_userspace_app()` code path serves as a reference implementation.
- **init binary missing from initrd:** The boot state machine searches multiple paths and reports exactly which paths were tried via `BootError::InitNotFound`.
- **Timer IRQ during transition:** Handled by the existing architecture-specific entry mechanism. The design does not prescribe interrupt masking — the implementation uses whatever mechanism the arch layer provides for an atomic, correct transition.

## Success Criteria

1. Kernel boots, prints timed `[BOOT]` diagnostics through all states, transitions to `Running`
2. PID 1 executes (verified via sentinel, not just log output)
3. QEMU smoke test passes: boots, detects sentinel within 30s, exits cleanly
4. The same test passes on 3 consecutive runs
5. Boot trace is available in panic output for debugging failures
