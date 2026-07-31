# Phase 1: Boot Reliability — Implementation Plan

> **For agentic workers:** Use subagent-driven-development or executing-plans to implement task-by-task.
> Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make SkyOS reliably boot from UEFI to a running PID 1 in userspace.

**Architecture:** Replace the flat debug-halted `init_os_task()` with a typed boot state machine (`BootState` enum, `BootError` typed errors, `BootContext`/`BootSession` data holders, `BootEvent` trace). Each state is a function that returns `Result<BootState, BootError>`. The state machine loop validates legal transitions. After the `EnterUserspace` state, the kernel calls `setup_user_stack()` + `jump_to_usermode()` (the same path `spawn_userspace_app()` uses today).

**Tech Stack:** Rust nightly, `x86_64` crate, `xmas-elf`, custom `x86_64-sarga` target, bootloader v0.11, QEMU for testing.

## Global Constraints

- `#![no_std]` + `#![no_main]` — no libstd, no test harness
- `#![deny(warnings)]` — all warnings are errors
- `// SAFETY:` comment required on every `unsafe` block
- No `unwrap()`/`expect()` in new boot code; propagate with `BootError`
- Kernel lives at `kernel/` (junction to `../SKYIOUS KERNEL/`); source at `kernel/kernel/src/`
- Build: `python build_disk.py --kernel-only` from workspace root, or `cargo build` from `kernel/kernel/`
- Run: `qemu-system-x86_64 -bios OVMF.fd -drive format=raw,file=skyos_uefi.img -m 512M -smp 2 -nographic`

---
### File Structure

| File | Purpose |
|------|---------|
| `kernel/kernel/src/boot/mod.rs` | `BootState`, `BootError`, `BootWarning`, `BootEvent`, `BootContext`, `BootSession` types + transition validation |
| `kernel/kernel/src/boot/logger.rs` | `BootLogger` with timed `info/warn/error` methods |
| `kernel/kernel/src/boot/state.rs` | State machine runner + one function per `BootState` |
| `kernel/kernel/src/main.rs` | Remove debug halt, call `boot::run()`, update panic handler with trace dump |
| `tests/test_boot.ps1` | QEMU smoke test |

---

### Task 1: Boot types + logger module

**Files:**
- Create: `kernel/kernel/src/boot/mod.rs`
- Create: `kernel/kernel/src/boot/logger.rs`
- Modify: `kernel/kernel/src/main.rs` (add `mod boot;`)

**Interfaces:**
- Produces: `BootState`, `BootError`, `BootWarning`, `BootEvent`, `BootContext`, `BootSession` types; `BootLogger` struct with `info()`, `warn()`, `error()`

- [x] **Step 1: Create `kernel/kernel/src/boot/mod.rs`**

```rust
//! Boot state machine types and transition validation.

use alloc::string::String;
use alloc::vec::Vec;

/// Phases of the boot state machine.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BootState {
    InitKernel,
    LocateInit,
    ParseElf,
    CreateAddressSpace,
    MapStack,
    CreatePid1,
    SetupConsole,
    EnterUserspace,
    Running,
    Failed,
}

impl BootState {
    /// Valid transitions — any other pair is a programming error.
    pub fn valid_next(&self) -> &[BootState] {
        match self {
            BootState::InitKernel => &[BootState::LocateInit],
            BootState::LocateInit => &[BootState::ParseElf, BootState::Failed],
            BootState::ParseElf => &[BootState::CreateAddressSpace, BootState::Failed],
            BootState::CreateAddressSpace => &[BootState::MapStack, BootState::Failed],
            BootState::MapStack => &[BootState::CreatePid1, BootState::Failed],
            BootState::CreatePid1 => &[BootState::SetupConsole, BootState::Failed],
            BootState::SetupConsole => &[BootState::EnterUserspace, BootState::Running],
            BootState::EnterUserspace => &[BootState::Running, BootState::Failed],
            BootState::Running => &[],
            BootState::Failed => &[],
        }
    }
}

/// Fatal boot errors.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BootError {
    InitNotFound,
    InvalidElf,
    AddressSpaceCreationFailed,
    StackAllocationFailed,
    ConsoleUnavailable,
    UserspaceEntryFailed,
}

/// Recoverable boot warnings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BootWarning {
    ConsoleUnavailable,
    EntropySourceMissing,
}

/// Events recorded in the boot trace.
#[derive(Debug, Clone)]
pub enum BootEvent {
    Enter(BootState),
    Exit(BootState),
    Warning(BootWarning),
    Error(BootError),
}

/// Persistent data live across the entire boot lifetime.
pub struct BootContext {
    pub trace: Vec<BootEvent>,
    pub init_paths_tried: Vec<String>,
    pub boot_start_tick: u64,
}

impl BootContext {
    pub fn new(boot_start_tick: u64) -> Self {
        BootContext {
            trace: Vec::new(),
            init_paths_tried: Vec::new(),
            boot_start_tick,
        }
    }
}

/// Transient objects only needed while launching PID 1.
pub struct BootSession<'a> {
    pub elf_data: Option<&'a [u8]>,
    pub entry_point: u64,
    pub user_rsp: u64,
}
```

- [ ] **Step 2: Create `kernel/kernel/src/boot/logger.rs`**

```rust
//! Boot-time diagnostic logger with millisecond timestamps.

use crate::boot::BootContext;

pub struct BootLogger;

impl BootLogger {
    fn timestamp_ms(context: &BootContext) -> u64 {
        // Use the 100Hz tick counter; divide by 10 for approximate ms
        // ponytail: Ticks at 100Hz = 10ms granularity, good enough for boot diag
        let ticks = crate::interrupts::get_ticks();
        let elapsed = ticks.wrapping_sub(context.boot_start_tick);
        elapsed * 10
    }

    pub fn info(context: &BootContext, msg: &str) {
        let ts = Self::timestamp_ms(context);
        crate::serial_write(&alloc::format!("[{}] BOOT  {}\n", ts, msg));
    }

    pub fn warn(context: &BootContext, msg: &str) {
        let ts = Self::timestamp_ms(context);
        crate::serial_write(&alloc::format!("[{}] BOOT  WARNING {}\n", ts, msg));
    }

    pub fn error(context: &BootContext, msg: &str) {
        let ts = Self::timestamp_ms(context);
        crate::serial_write(&alloc::format!("[{}] BOOT  ERROR {}\n", ts, msg));
    }
}
```

Note: `interrupts::get_ticks()` is a 100Hz counter set up during interrupt init. It only works after `init_idt()` + LAPIC timer setup. If called before that, it returns 0 — which is fine for diagnostics since the boot sequence is linear and the first `BootLogger` call is after scheduler init.

- [ ] **Step 3: Wire module into main.rs**

Add after the existing `mod` block near line 48-102 of `kernel/kernel/src/main.rs`:

```rust
pub mod boot;
```

- [ ] **Step 4: Verify compilation**

From workspace root:
```powershell
python build_disk.py --kernel-only
```
Expected: kernel builds successfully.

---

### Task 2: State machine runner

**Files:**
- Create: `kernel/kernel/src/boot/state.rs`
- Modify: `kernel/kernel/src/boot/mod.rs` (add `pub mod state;`)

**Interfaces:**
- Consumes: `BootState`, `BootError`, `BootWarning`, `BootContext`, `BootSession`, `BootLogger` (from Task 1)
- Produces: `pub fn run_boot() -> !` — the state machine entry point

- [ ] **Step 1: Create `kernel/kernel/src/boot/state.rs`**

```rust
//! Boot state machine runner.
//!
//! Each state is a function that takes `&BootContext` and returns
//! `Result<BootState, BootError>`. The main loop validates transitions
//! and logs every state change.

use crate::boot::{
    BootState, BootError, BootWarning, BootEvent, BootContext, BootSession, logger::BootLogger,
};

/// Run the boot state machine to completion.
/// This function never returns — it either enters userspace or panics.
pub fn run_boot() -> ! {
    use crate::interrupts::get_ticks;
    let mut ctx = BootContext::new(get_ticks());
    let mut session = BootSession {
        elf_data: None,
        entry_point: 0,
        user_rsp: 0,
    };

    let mut state = BootState::InitKernel;
    loop {
        ctx.trace.push(BootEvent::Enter(state));
        BootLogger::info(&ctx, &alloc::format!("{:?}", state));

        let next = match state {
            BootState::InitKernel => state_init_kernel(&ctx),
            BootState::LocateInit => state_locate_init(&mut ctx),
            BootState::ParseElf => state_parse_elf(&mut ctx, &mut session),
            BootState::CreateAddressSpace => state_create_address_space(&mut ctx, &mut session),
            BootState::MapStack => state_map_stack(&mut ctx, &mut session),
            BootState::CreatePid1 => state_create_pid1(&mut ctx, &mut session),
            BootState::SetupConsole => state_setup_console(&mut ctx, &mut session),
            BootState::EnterUserspace => state_enter_userspace(&ctx, &session),
            BootState::Running => {
                BootLogger::info(&ctx, "Boot complete, entering scheduler dispatch");
                // Running is terminal — hand off to scheduler
                unsafe { crate::task::scheduler::schedule(); }
                unreachable!()
            }
            BootState::Failed => {
                // Panic handler will dump trace
                panic!("Boot failed — see trace above");
            }
        };

        match next {
            Ok(next_state) => {
                // Validate transition
                let valid = state.valid_next();
                if !valid.contains(&next_state) && next_state != BootState::Failed {
                    BootLogger::error(&ctx, &alloc::format!(
                        "Invalid boot transition: {:?} → {:?}", state, next_state
                    ));
                    ctx.trace.push(BootEvent::Error(BootError::UserspaceEntryFailed));
                    panic!("Invalid boot state transition");
                }
                ctx.trace.push(BootEvent::Exit(state));
                state = next_state;
            }
            Err(e) => {
                ctx.trace.push(BootEvent::Error(e));
                BootLogger::error(&ctx, &alloc::format!("{:?}", e));
                let err_str = alloc::format!("Boot failed at {:?}: {:?}", state, e);
                ctx.trace.push(BootEvent::Exit(state));
                // Dump trace then panic
                BootLogger::error(&ctx, "Boot trace:");
                for event in &ctx.trace {
                    BootLogger::error(&ctx, &alloc::format!("  {:?}", event));
                }
                panic!("{}", err_str);
            }
        }
    }
}
```

- [ ] **Step 2: Add `pub mod state;` to `boot/mod.rs`**

```rust
pub mod logger;
pub mod state;
```

- [ ] **Step 3: Verify compilation**

```powershell
python build_disk.py --kernel-only
```
Expected: compiles (state machine entry exists but isn't called yet).

---

### Task 3: Implement each boot state function

**Files:**
- Modify: `kernel/kernel/src/boot/state.rs` (implement the 8 state functions)
- Create: No new files

**Interfaces:**
- Each state fn returns `Result<BootState, BootError>`
- Consumes `&BootContext` for logging, `&mut BootSession` for state

- [ ] **Step 1: Append state functions to `state.rs`**

```rust
// ── State implementations ──

fn state_init_kernel(_ctx: &BootContext) -> Result<BootState, BootError> {
    // InitKernel: after scheduler init, nothing to do but advance
    Ok(BootState::LocateInit)
}

fn state_locate_init(ctx: &mut BootContext) -> Result<BootState, BootError> {
    let search_paths = ["/bin/init", "/init", "/sbin/init"];
    let vfs_mgr = crate::vfs::VFS.lock();
    for path in &search_paths {
        ctx.init_paths_tried.push(alloc::string::String::from(*path));
        BootLogger::info(ctx, &alloc::format!("Looking for {}", path));
        if let Some(node) = vfs_mgr.resolve_path(path) {
            if let Ok(data) = node.read(usize::MAX) {
                ctx.elf_data = Some(data);
                drop(vfs_mgr);
                BootLogger::info(ctx, &alloc::format!("Found init at {}", path));
                return Ok(BootState::ParseElf);
            }
        }
    }
    drop(vfs_mgr);
    Err(BootError::InitNotFound)
}

fn state_parse_elf(ctx: &BootContext, session: &mut BootSession) -> Result<BootState, BootError> {
    let elf_data = session.elf_data.as_ref().ok_or(BootError::InitNotFound)?;
    // Validate ELF header minimally before attempting load
    if elf_data.len() < 64 || &elf_data[..4] != b"\x7fELF" {
        return Err(BootError::InvalidElf);
    }
    // Full load happens in the next state; here we just validate the header
    BootLogger::info(ctx, "ELF header valid");
    Ok(BootState::CreateAddressSpace)
}

fn state_create_address_space(ctx: &BootContext, session: &mut BootSession) -> Result<BootState, BootError> {
    use crate::memory::buddy::BuddyFrameAllocator;
    let mut frame_allocator = BuddyFrameAllocator;
    let elf_data = session.elf_data.as_ref().ok_or(BootError::InitNotFound)?;
    let address_space = crate::memory::paging::AddressSpace::new(&mut frame_allocator)
        .ok_or(BootError::AddressSpaceCreationFailed)?;
    let process = crate::task::process::Process::load_elf(elf_data, address_space)
        .map_err(|_| BootError::InvalidElf)?;
    session.entry_point = process.entry_point;
    // Store process in a static so we can access it from later states
    // without threading Arc through BootSession
    unsafe { BOOT_PROCESS = Some(alloc::sync::Arc::new(process)); }
    BootLogger::info(ctx, &alloc::format!("PID 1 ELF loaded, entry=0x{:x}", session.entry_point));
    Ok(BootState::MapStack)
}

// We need a static to hold the Process Arc between states
use spin::Mutex;
static BOOT_PROCESS: Mutex<Option<alloc::sync::Arc<crate::task::process::Process>>> = Mutex::new(None);

fn state_map_stack(ctx: &BootContext, session: &mut BootSession) -> Result<BootState, BootError> {
    let process_guard = BOOT_PROCESS.lock();
    let process = process_guard.as_ref().ok_or(BootError::StackAllocationFailed)?;
    // Use the executable path as argv[0]
    let argv = alloc::vec!["/bin/init".to_string()];
    let user_rsp = process.setup_user_stack(&argv)
        .map_err(|_| BootError::StackAllocationFailed)?;
    session.user_rsp = user_rsp;
    BootLogger::info(ctx, &alloc::format!("User stack at 0x{:x}", user_rsp));
    Ok(BootState::CreatePid1)
}

fn state_create_pid1(ctx: &BootContext, _session: &BootSession) -> Result<BootState, BootError> {
    let mut process_guard = BOOT_PROCESS.lock();
    let process = process_guard.take().ok_or(BootError::StackAllocationFailed)?;
    let pid = process.id;
    debug_assert_eq!(pid, 100, "Phase 1 assumes first userspace process gets PID 100+");

    let process_arc = alloc::sync::Arc::new(process);
    // Unwrap the Arc — was created from Box leak in load_elf
    let raw = alloc::sync::Arc::into_raw(process_arc.clone());
    unsafe { (*raw).id = pid; } // ensure PID is set
    crate::task::process::Process::register(process_arc.clone());
    // Restore in static for subsequent states
    *process_guard = Some(process_arc);
    drop(process_guard);

    BootLogger::info(ctx, &alloc::format!("PID 1 registered (pid={})", pid));
    Ok(BootState::SetupConsole)
}

fn state_setup_console(ctx: &BootContext, session: &BootSession) -> Result<BootState, BootError> {
    let process_guard = BOOT_PROCESS.lock();
    let process = process_guard.as_ref().ok_or(BootError::ConsoleUnavailable)?;

    let tty_node = crate::vfs::VFS.lock().resolve_path("/dev/tty0");
    match tty_node {
        Some(tty) => {
            use crate::task::process::FileDescriptor;
            let mut fd_table = process.fd_table.lock();
            fd_table.resize(3, None);
            fd_table[0] = Some(FileDescriptor::File { node: tty.clone(), offset: spin::Mutex::new(0) });
            fd_table[1] = Some(FileDescriptor::File { node: tty.clone(), offset: spin::Mutex::new(0) });
            fd_table[2] = Some(FileDescriptor::File { node: tty, offset: spin::Mutex::new(0) });
            drop(fd_table);
            BootLogger::info(ctx, "stdin/stdout/stderr -> /dev/tty0");
        }
        None => {
            ctx.trace.push(BootEvent::Warning(BootWarning::ConsoleUnavailable));
            BootLogger::warn(ctx, "/dev/tty0 not found — init runs with no stdin/stdout/stderr");
            // Continue — init can still use serial or fail gracefully
        }
    }

    // Set thread.process — must happen AFTER tty setup (see main.rs NOTE about timer ISR)
    crate::task::scheduler::with_current_thread(|thread| {
        thread.process = process.clone().into();
    });
    BootLogger::info(ctx, "Thread process assigned");
    Ok(BootState::EnterUserspace)
}

fn state_enter_userspace(ctx: &BootContext, session: &BootSession) -> Result<BootState, BootError> {
    let process_guard = BOOT_PROCESS.lock();
    let process = process_guard.as_ref().ok_or(BootError::UserspaceEntryFailed)?;

    // Set CURRENT_PROCESS for syscall dispatch
    {
        let mut cur = crate::task::process::CURRENT_PROCESS.lock();
        *cur = Some(process.clone());
    }

    // Activate the process address space
    BootLogger::info(ctx, "Activating address space");
    unsafe { process.address_space.activate(); }
    debug_assert!(process.address_space.is_active(), "Address space must be active before user entry");

    BootLogger::info(ctx, &alloc::format!("Jumping to userspace entry=0x{:x} rsp=0x{:x}", session.entry_point, session.user_rsp));
    // SAFETY: All prerequisite setup is complete — valid ELF, mapped stack, active AS, registered process
    unsafe {
        crate::task::thread::jump_to_usermode(session.entry_point, session.user_rsp);
    }
    // unreachable
    Err(BootError::UserspaceEntryFailed)
}
```

Key design note: We use a `static BOOT_PROCESS: Mutex<Option<Arc<Process>>>` to carry the process across state functions, rather than putting it in `BootSession`. `BootSession` is consumed during setup (it holds raw data and primitives). The `Arc<Process>` needs to live across states where we take it out for registration and put it back — a static is the simplest way.

- [ ] **Step 2: Verify compilation**

```powershell
python build_disk.py --kernel-only
```
Expected: compiles with no errors.

---

### Task 4: Wire state machine into main.rs, remove debug halt

**Files:**
- Modify: `kernel/kernel/src/main.rs`

- [ ] **Step 1: Read the current `init_os_task()` at line 355-482**

Locate the function — it's the long block starting at `extern "C" fn init_os_task() -> ! {`.
It searches for init, loads ELF, creates AS, maps, registers, sets up tty, then HALTS at line 466-467.

- [ ] **Step 2: Replace the function body**

Replace the entire `init_os_task()` function:

```rust
extern "C" fn init_os_task() -> ! {
    crate::boot::state::run_boot()
}
```

- [ ] **Step 3: Remove unused code**

The old code at line 355-482 is gone. The `spawn_userspace_app()` at line 669-727 should remain (it's a separate public API for launching apps later).

Also remove the `#[allow(dead_code)]` on `threading_demo()` at line 484 if it was only used by the old init.

- [ ] **Step 4: Update panic handler to dump boot trace**

Replace the existing panic handler (lines 733-752):

```rust
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    crate::serial_write("\n=== KERNEL PANIC ===\n");
    crate::serial_write("[PANIC] ");
    let msg = info.message();
    let panic_str = alloc::format!("{:?}", msg);
    crate::serial_write(&panic_str);
    crate::serial_write("\n");
    if let Some(loc) = info.location() {
        crate::serial_write("[PANIC] at ");
        crate::serial_write(loc.file());
        crate::serial_write(":");
        let line_str = alloc::format!("{}", loc.line());
        crate::serial_write(&line_str);
        crate::serial_write("\n");
    }
    // Dump boot trace if available
    crate::serial_write("[PANIC] Boot trace:\n");
    // The boot context is only accessible via the panic if we store it.
    // For now, just dump registers and stack.
    crate::debug::print_stack_trace();

    // Dump key registers
    #[cfg(target_arch = "x86_64")]
    unsafe {
        let cr2: u64;
        core::arch::asm!("mov {}, cr2", out(reg) cr2);
        crate::serial_write(&alloc::format!("[PANIC] CR2 (page fault addr): 0x{:x}\n", cr2));
    }

    loop { crate::arch::CurrentArch::halt(); }
}
```

- [ ] **Step 5: Add `boot_start_tick` initialization**

In `kernel_main()`, right before spawning `init_os_task`, add the boot context. Actually, since the state machine creates its own `BootContext` on entry, we don't need to init it here. But we should record the boot start tick somewhere accessible. Add near the top of `kernel_main()`:

```rust
// After scheduler init, before spawning tasks
crate::boot::BOOT_START_TICK.store(crate::interrupts::get_ticks(), core::sync::atomic::Ordering::Relaxed);
```

This requires adding to `boot/mod.rs`:
```rust
use core::sync::atomic::AtomicU64;
pub static BOOT_START_TICK: AtomicU64 = AtomicU64::new(0);
```

And updating `BootLogger` to fall back to this if `context.boot_start_tick == 0`.

Actually, let's simplify: the `BootLogger` already takes `&BootContext` which has `boot_start_tick`. The context is created inside `run_boot()` using `get_ticks()` at call time. No global needed.

- [ ] **Step 6: Verify compilation**

```powershell
python build_disk.py --kernel-only
```
Expected: compiles cleanly.

---

### Task 5: Update BootLogger to handle early-boot calls before timer init

**Files:**
- Modify: `kernel/kernel/src/boot/logger.rs`
- Modify: `kernel/kernel/src/boot/state.rs`

The `BootLogger` uses `interrupts::get_ticks()` which is only valid after LAPIC timer init (which happens during `interrupts::init_idt()` in `kernel_main`). However, the boot state machine starts after scheduler init, which is after timer init. So this should work.

But let's add a safety guard just in case:

- [ ] **Step 1: Guard `timestamp_ms` against missing timer**

In `logger.rs`, add a fallback:

```rust
fn timestamp_ms(context: &BootContext) -> alloc::string::String {
    let tick = crate::interrupts::get_ticks();
    if tick == 0 && context.boot_start_tick == 0 {
        // Timer not yet initialized — use raw tick count
        alloc::format!("?")
    } else {
        let elapsed = tick.wrapping_sub(context.boot_start_tick);
        alloc::format!("{}", elapsed * 10)
    }
}
```

- [ ] **Step 2: Verify compilation**

```powershell
python build_disk.py --kernel-only
```

---

### Task 6: Add boot trace persistence for panic dump

**Files:**
- Modify: `kernel/kernel/src/boot/mod.rs` (add global trace reference)

- [ ] **Step 1: Make boot trace accessible from panic handler**

Add to `boot/mod.rs`:

```rust
use spin::Mutex;

/// Global reference to the last boot trace, so the panic handler can dump it.
static BOOT_TRACE: Mutex<Option<Vec<BootEvent>>> = Mutex::new(None);
static BOOT_INIT_PATHS: Mutex<Option<Vec<String>>> = Mutex::new(None);

pub fn store_trace(trace: Vec<BootEvent>, init_paths: Vec<String>) {
    *BOOT_TRACE.lock() = Some(trace);
    *BOOT_INIT_PATHS.lock() = Some(init_paths);
}
```

- [ ] **Step 2: Call `store_trace` before userspace entry**

In `state.rs`, at the top of `state_enter_userspace`, store the trace:

```rust
fn state_enter_userspace(ctx: &BootContext, session: &BootSession) -> Result<BootState, BootError> {
    // Store boot trace for panic handler before potentially irrecoverable transition
    crate::boot::store_trace(ctx.trace.clone(), ctx.init_paths_tried.clone());
    // ... rest of function
}
```

- [ ] **Step 3: Update panic handler to dump stored trace**

In `main.rs` panic handler, add between the location print and the stack trace:

```rust
// Dump boot trace
{
    let trace_guard = crate::boot::BOOT_TRACE.lock();
    if let Some(trace) = trace_guard.as_ref() {
        crate::serial_write("[PANIC] Boot trace:\n");
        for event in trace.iter() {
            crate::serial_write(&alloc::format!("  {:?}\n", event));
        }
    }
    let paths_guard = crate::boot::BOOT_INIT_PATHS.lock();
    if let Some(paths) = paths_guard.as_ref() {
        crate::serial_write("[PANIC] Init paths searched:\n");
        for p in paths.iter() {
            crate::serial_write(&alloc::format!("  {}\n", p));
        }
    }
}
```

- [ ] **Step 4: Verify compilation**

```powershell
python build_disk.py --kernel-only
```

---

### Task 7: QEMU boot smoke test

**Files:**
- Create: `tests/test_boot.ps1`

- [ ] **Step 1: Create the test script**

```powershell
# tests/test_boot.ps1 — QEMU boot smoke test
# Verifies kernel boots and PID 1 enters userspace.

param(
    [string]$Image = "skyos_uefi.img",
    [int]$TimeoutSeconds = 30
)

$scriptDir = Split-Path -Parent $PSCommandPath
$projectRoot = Resolve-Path "$scriptDir/.."
$qemu = "qemu-system-x86_64"
$bios = "$projectRoot/OVMF.fd"
$imagePath = "$projectRoot/$Image"

if (-not (Test-Path $bios)) {
    Write-Error "OVMF.fd not found at $bios"
    exit 1
}

if (-not (Test-Path $imagePath)) {
    Write-Error "Boot image not found at $imagePath. Run 'python build_disk.py --kernel-only' first."
    exit 1
}

Write-Host "[TEST] Booting $imagePath in QEMU (timeout: ${TimeoutSeconds}s)..."
Write-Host "[TEST] Looking for boot completion marker..."

$process = Start-Process -NoNewWindow -FilePath $qemu -ArgumentList @(
    "-bios", $bios,
    "-drive", "format=raw,file=$imagePath",
    "-m", "512M",
    "-smp", "2",
    "-nographic",
    "-serial", "stdio",
    "-device", "isa-debugcon,iobase=0xE9,chardev=dbg",
    "-chardev", "file,id=dbg,path=$projectRoot/serial_test.log"
) -PassThru

$elapsed = 0
$found = $false
while ($elapsed -lt $TimeoutSeconds) {
    Start-Sleep -Seconds 1
    $elapsed++
    if (Test-Path "$projectRoot/serial_test.log") {
        $content = Get-Content "$projectRoot/serial_test.log" -Raw
        # Look for successful userspace execution marker
        if ($content -match "Boot complete" -or
            $content -match "init] SARGA init starting" -or
            $content -match "Userland init running") {
            $found = $true
            Write-Host "[TEST] SUCCESS: Userspace execution verified"
            break
        }
        if ($content -match "KERNEL PANIC" -or $content -match "PANIC") {
            Write-Host "[TEST] FAIL: Kernel panic detected"
            Write-Host "[TEST] Last 20 lines of serial output:"
            $content -split "`n" | Select-Object -Last 20
            break
        }
    }
    Write-Host "[TEST] ... still waiting (${elapsed}s)"
}

if (-not $found) {
    Write-Host "[TEST] FAIL: Timeout - boot marker not found within ${TimeoutSeconds}s"
    if (Test-Path "$projectRoot/serial_test.log") {
        Write-Host "[TEST] Last 30 lines of serial output:"
        Get-Content "$projectRoot/serial_test.log" | Select-Object -Last 30
    }
    Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
    exit 1
}

Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
Remove-Item "$projectRoot/serial_test.log" -Force -ErrorAction SilentlyContinue
exit 0
```

- [ ] **Step 2: Create a Linux/Makefile-compatible version**

Create `tests/test_boot.sh`:

```bash
#!/bin/bash
# QEMU boot smoke test — verifies kernel boots to userspace
set -euo pipefail

IMAGE="${1:-skyos_uefi.img}"
TIMEOUT="${2:-30}"
DIR="$(cd "$(dirname "$0")/.." && pwd)"
SERIAL_LOG=$(mktemp /tmp/skyos-boot-test.XXXXXX)
trap 'rm -f "$SERIAL_LOG"' EXIT

if [ ! -f "$DIR/OVMF.fd" ]; then echo "OVMF.fd not found"; exit 1; fi
if [ ! -f "$DIR/$IMAGE" ]; then echo "$IMAGE not found — run 'python build_disk.py --kernel-only'"; exit 1; fi

echo "[TEST] Booting $IMAGE (timeout: ${TIMEOUT}s)..."
timeout "$TIMEOUT" qemu-system-x86_64 \
    -bios "$DIR/OVMF.fd" \
    -drive "format=raw,file=$DIR/$IMAGE" \
    -m 512M -smp 2 -nographic \
    -serial stdio 2>&1 | tee "$SERIAL_LOG" &
QEMU_PID=$!

# Wait for boot completion marker
if grep -q -e "Boot complete" -e "SARGA init starting" -e "Userland init running" <(timeout "$TIMEOUT" tail -f "$SERIAL_LOG" 2>/dev/null); then
    echo "[TEST] SUCCESS: Userspace execution verified"
    kill "$QEMU_PID" 2>/dev/null || true
    exit 0
fi

echo "[TEST] FAIL: Boot marker not found"
kill "$QEMU_PID" 2>/dev/null || true
exit 1
```

- [ ] **Step 3: Run the smoke test**

```powershell
# First build
python build_disk.py --kernel-only; if ($?) { tests/test_boot.ps1 }
```

- [ ] **Step 4: Run 3 times to verify stability**

```powershell
1..3 | ForEach-Object { Write-Host "=== Run $_ ==="; tests/test_boot.ps1; if (-not $?) { exit 1 } }
```

Expected: all 3 runs pass.

---

### Post-implementation checklist

- [ ] Kernel compiles with `#![deny(warnings)]`
- [ ] `init_os_task()` calls `boot::state::run_boot()`
- [ ] Every `BootState` has a corresponding function
- [ ] Transition validation catches illegal transitions
- [ ] `BootLogger` emits timestamped `[{ms}] BOOT` lines for each state
- [ ] On success: `[Boot complete, entering scheduler dispatch]`
- [ ] On failure: boot trace + error type dumped before panic
- [ ] Panic handler dumps boot trace + CR2 (if page fault)
- [ ] Console fallback: missing `/dev/tty0` produces warning, not fatal
- [ ] QEMU smoke test passes 3 consecutive runs
- [ ] The old `[INIT] HALTING (debug)` line is gone
