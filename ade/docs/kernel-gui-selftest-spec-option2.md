# Kernel selftest spec — Option 2 honest `-ENOMEM` (fail create under pressure)

**Status:** spec only, for the kernel rewrite's selftest framework — no kernel
code touched (kernel is mid-major-change). Written Aug 13, 2026 against
`kernel/kernel/src` at that date; treat function/syscall names and the TAP
framework as the stable anchors. Companion to `kernel-gui-window-fix.md`
**Patch Option 2** (fail `SYS_GUI_CREATE_WINDOW` with `-ENOMEM` instead of the
silent heap fallback) and its userspace half **Option 2b** (login-manager
reports `Out of memory` and exits non-zero). Sibling of
`kernel-gui-selftest-spec.md` (the Option 1 selftest).

> **Mutual exclusion:** Option 1 and Option 2 cannot coexist at the
> `create_window` fallback site (fix doc: "the two kernel options are
> mutually exclusive at the `create_window` fallback level"), so the rewrite
> registers ONE family of `gui::option1_*` / `gui::option2_*` TAP tests.
> Pick by the memory-pressure evidence (fix doc Evidence probe: persistent
> OOM → Option 2 + this spec; transient → Option 1).

## What is under test

Option 2 changes `sys_gui_create_window` so that when the contiguous
allocation fails (memory pressure) it returns `-ENOMEM`
(`errno::Errno::ENOMEM as u64` = `0xFFFF_FFFF_FFFF_FFF4`, read back as i64 by
libsarga's `syscall3` = `-12`) INSTEAD of silently falling back to a
heap-content window. The selftest must prove three things:

1. **The `-ENOMEM` branch is reachable**: under deterministic memory pressure
   (the same buddy drain), `create_window` returns the error, not a handle.
2. **No half-window leaks**: the failing create adds nothing to the
   compositor (window count unchanged) — the `-ENOMEM` return fires before
   `comp.add_window(win)`.
3. **The happy path is intact**: with memory available, create still returns a
   valid handle with `phys_addr == Some` — the error return must not leak
   into normal boots.

The kernel TAP framework runs pre-process, so login-manager's response to the
new errno is NOT a kernel test: it is the **userspace half** — Option 2b's
`Err(12)` arm prints `[login] window create failed: Out of memory` and exits
non-zero (`EXIT_WINDOW_CREATE_FAILED = 1`) so init's accounting bounds the
respawn loop. That contract is host-pinned as a source contract (see the
userspace section below), not a TAP test.

## Framework integration (exact wiring)

Same `kernel/src/tests/gui_tests.rs` module as Option 1, but the `register()`
calls REPLACE the `option1_*` ones (mutual exclusion):

```rust
// kernel/src/tests/gui_tests.rs (Option 2 variant)
use crate::memory::buddy::BUDDY_ALLOCATOR;
use crate::syscalls;

pub fn test_option2_enomem_forced() -> Result<(), &'static str> { ... }
pub fn test_option2_create_succeeds_when_room() -> Result<(), &'static str> { ... }

pub fn register() {
    crate::selftest::register("gui::option2_enomem_forced", test_option2_enomem_forced);
    crate::selftest::register("gui::option2_create_succeeds_when_room", test_option2_create_succeeds_when_room);
}
```

Wire it into `kernel/src/tests/mod.rs` exactly as Option 1: `mod gui_tests;`
plus `gui_tests::register()` from the aggregator (either the per-module or
flat `register_all` shape — the anchor is `selftest::register("gui::option2_*",
...)` in the TAP framework `kernel/src/selftest.rs`, the same
`register(name: &'static str, func: TestFn)` signature the Option 1 spec
quotes).

## Visibility prerequisite (one-word kernel change)

Identical to Option 1: `sys_gui_create_window` is currently a **private** `fn`
(`kernel/src/syscalls/mod.rs:4656`). The test module is a sibling of
`syscalls`, so it must be marked `pub(crate)` for the forced-`-ENOMEM` test to
call it. That is the only kernel signature change the selftest requires.

## Timing prerequisite

Both Option 2 tests are **pre-init safe** — no process, no render, no
`CURRENT_PROCESS`:

- `enomem_forced` asserts a return VALUE plus the compositor window count; no
  mapping or rendering is involved.
- `create_succeeds_when_room` creates a real phys-backed window but must NOT
  call `sys_gui_map_buffer` (which needs `CURRENT_PROCESS` and a VMA seed,
  see the Option 1 spec's timing note) — it only verifies the create returned
  a valid handle with a real buffer.

So unlike Option 1, there is no post-init phase and no process seeding.

## The two TAP tests, in detail

Common setup helper: **identical** to Option 1's `drain_order`/`release`
(kernel-gui-selftest-spec.md) — draining every contiguous N-order block until
`allocate_contiguous(N)` returns `None` is what forces the failure branch
deterministically. One source of truth in `gui_tests.rs`; the rewrite's
extracted `order_for_size` helper feeds both specs.

### Test 1 — `gui::option2_enomem_forced`

Proves the `-ENOMEM` branch is reachable and leaves no half-window.

```rust
fn test_option2_enomem_forced() -> Result<(), &'static str> {
    // 800x600: create_window computes content_len = width * height (the
    // ORIGINAL dims) = 480,000 px = 1,920,000 bytes -> order 9 (2 MB).
    let order = 9usize;
    let title = alloc::string::String::from("selftest").into_boxed_str();
    let free_before = BUDDY_ALLOCATOR.lock().count_free_pages();
    let len_before = crate::gui::COMPOSITOR.lock().windows.len();
    let drained = drain_order(order);
    if drained.is_empty() {
        // No order-9 block at boot is a valid low-memory precondition, but
        // the drain would be a no-op and ENOMEM might fire for the wrong
        // reason (or not at all) — fail loudly so the harness stays honest.
        return Err("no order-9 blocks to drain; cannot force ENOMEM");
    }

    let ret = syscalls::sys_gui_create_window(title.as_ptr(), 800, 600);

    // The contract: the syscall returns -ENOMEM as u64, NOT a handle. The
    // assertion MUST use the same expression the Option 2 hunk returns
    // (`return errno::Errno::ENOMEM as u64;`) so a sign/width slip
    // (e.g. `12 as u64` instead of `-12 as u64`) fails the test.
    if ret != errno::Errno::ENOMEM as u64 {
        return Err("create_window did not return -ENOMEM under pressure");
    }
    // No half-window: the failing create must not reach comp.add_window.
    let comp = crate::gui::COMPOSITOR.lock();
    if comp.windows.len() != len_before {
        return Err("ENOMEM path left a window in the compositor");
    }
    drop(comp);
    release(drained);
    if BUDDY_ALLOCATOR.lock().count_free_pages() != free_before {
        return Err("buddy free count not restored after drain/release");
    }
    Ok(())
}
```

Notes:

- `errno::Errno::ENOMEM = -12` (`syscalls/errno.rs:17`); `-12 as u64` wraps to
  `0xFFFF_FFFF_FFFF_FFF4`, and libsarga's `syscall3` reads the return back as
  i64 = `-12` → `Window::create` yields `Err(12)` (libsarga `errno::ENOMEM =
  12`). Adjust the `errno::` import path to the rewrite's errno module
  location — the EXPRESSION is the stable anchor.
- `len_before` (not a hard `0`) keeps the test robust if earlier selftests
  left the compositor non-empty.
- **Pre-fix honesty:** with Option 2 NOT applied (silent fallback still
  present), this test FAILS — create returns a handle (>= 0), not ENOMEM —
  proving the test is exercising the `-ENOMEM` branch (mirrors Option 1's
  `drained.is_empty()` guard).

### Test 2 — `gui::option2_create_succeeds_when_room`

Regression guard: the error return must not leak into the normal path.

```rust
fn test_option2_create_succeeds_when_room() -> Result<(), &'static str> {
    // NO drain — the buddy must be able to satisfy a normal 800x600 create.
    let title = alloc::string::String::from("selftest").into_boxed_str();
    let free_before = BUDDY_ALLOCATOR.lock().count_free_pages();
    let handle = syscalls::sys_gui_create_window(title.as_ptr(), 800, 600);

    let comp = crate::gui::COMPOSITOR.lock();
    if handle as usize >= comp.windows.len() {
        return Err("create returned an invalid handle on the happy path");
    }
    let win = &comp.windows[handle as usize];
    if win.phys_addr.is_none() {
        return Err("happy-path window has no phys_addr (fallback leaked?)");
    }
    // Cleanup: the phys block is NOT freed by Window's drop (the compositor
    // remove path leaks it today) — free explicitly, then remove the window
    // and verify the free count is restored (memory_tests.rs discipline).
    let pa = win.phys_addr.expect("phys_addr confirmed above");
    drop(comp);
    {
        let mut buddy = BUDDY_ALLOCATOR.lock();
        buddy.deallocate_contiguous(x86_64::PhysAddr::new(pa), 9);
    }
    crate::gui::COMPOSITOR.lock().windows.remove(handle as usize);
    if BUDDY_ALLOCATOR.lock().count_free_pages() != free_before {
        return Err("buddy free count not restored after window removal");
    }
    Ok(())
}
```

Notes:

- If the boot is so memory-poor that even an undrained create fails, this
  test fails loudly — which is honest: it means normal boots cannot create
  windows either, exactly the condition the `[login] mem free=N pages`
  evidence marker would show as persistent OOM.
- `deallocate_contiguous(pa, 9)` mirrors the Option 1 cleanup (the
  `order_for_size` helper, when extracted, feeds both the create and the
  explicit free).

## Assertion checklist (what the rewrite must verify)

| # | Assertion | Failure means |
|---|---|---|
| 1 | Under drain, `create_window` returns `errno::Errno::ENOMEM as u64` (not a handle) | Option 2 not applied, or sign/width slip (12 vs -12) |
| 2 | Compositor window count unchanged after the `-ENOMEM` return (no half-window) | `-ENOMEM` path adds a window before returning |
| 3 | Undrained create returns a valid handle with `phys_addr == Some` | happy path broken / silent fallback leaked into normal boots |
| 4 | Suite leaves compositor empty and buddy free count restored (explicit `deallocate_contiguous` of the happy-path window) | leaked test state corrupts later tests |

Also assert the **pre-fix failure** once for honesty: with Option 2 NOT
applied, test 1 must fail (`create_window` returns a handle) — if it passes,
the test isn't exercising the `-ENOMEM` branch (see the `drained.is_empty()`
guard).

## Userspace half — Option 2b `Err(12)` contract (host-pinned, not TAP)

The kernel TAP framework runs pre-process, so login-manager's response to the
new `-ENOMEM` cannot be a kernel selftest. It is a SOURCE contract, host-pinned
by `tests/test_selftest_spec_contract.py` (this spec's companion pin) and by
the Option 2b apply-check in `test_login_flow.py` (the drafted 2b hunk must
keep applying cleanly to `login-manager/src/main.rs`):

- When `Window::create` yields `Err(12)` (libsarga errno ENOMEM, from the
  kernel's `-12`), the Option 2b `Err(e)` arm prints
  `` [login] window create failed: Out of memory `` —
  the marker the give-up harness will grep on a forced-low-memory boot.
- Any OTHER `Err(e)` prints `` [login] window create failed: errno {e} `` —
  equally fatal.
- EITHER failure arm returns `EXIT_WINDOW_CREATE_FAILED` (= 1, non-zero), so
  init's crash accounting counts the failure and gives up after
  MAX_RESPAWNS — `[init] giving up on login-manager` (bounded), the K1-alt
  landing condition.
- **Without Option 2b, `Err(5)` → `Err(12)` alone changes nothing
  observable:** the current arm swallows the error value (`Err(_)`) and still
  returns 0 (clean exit → crash counter reset → unbounded respawn). Option 2b
  is MANDATORY for any observable effect — this is why the fix doc's Option 2
  section carries the mandatory-follow-up warning.

The exact contract tokens this spec pins:

```
[login] window create failed: Out of memory
EXIT_WINDOW_CREATE_FAILED: i32 = 1
return EXIT_WINDOW_CREATE_FAILED
```

## CI

The TAP lines print to serial (`ok N - gui::option2_enomem_forced`), so the
existing kernel-selftest CI gate that greps `not ok` catches a regression
automatically; for a narrower tripwire grep `gui::option2_` lines
specifically. The userspace half needs no QEMU boot — the host-tests job
(`test_selftest_spec_contract.py`) pins the contract, and the give-up harness
(`qemu_giveup_boot.exp`) asserts the runtime `Out of memory` →
`giving up on .*login-manager` sequence once the kernel lands (the
unbounded-absence grep flips to a positive requirement, per K1-alt).

## Files touched by this spec (when the rewrite lands)

1. `kernel/src/tests/gui_tests.rs` — the two `option2_*` tests + shared
   `drain_order`/`release` helpers (REPLACING the `option1_*` registrations —
   the two options are mutually exclusive).
2. `kernel/src/tests/mod.rs` — `mod gui_tests;` + `gui_tests::register()`.
3. `kernel/src/syscalls/mod.rs` — `pub(crate)` on `sys_gui_create_window` +
   the Option 2 `-ENOMEM` hunk (kernel-gui-window-fix.md).
4. `login-manager/src/main.rs` — the Option 2b `Err(12)` arm (the drafted 2b
   hunk in kernel-gui-window-fix.md).
5. `tests/qemu_giveup_boot.exp` — GATED on the kernel landing: the bounded
   `giving up on .*login-manager` becomes a POSITIVE requirement (K1-alt).
