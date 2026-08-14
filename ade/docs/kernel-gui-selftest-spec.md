# Kernel selftest spec — Option 1 promotion (map the heap-content fallback)

**Status:** spec only, for the kernel rewrite's selftest framework — no kernel
code touched (kernel is mid-major-change). Written Aug 10, 2026 against
`kernel/kernel/src` at that date; treat function/syscall names and the TAP
framework as the stable anchors. Companion to
`kernel-gui-window-fix.md` **Patch Option 1** (promote a fallback window in
`sys_gui_map_buffer` instead of returning 0).

## What is under test

Option 1 changes `sys_gui_map_buffer` so a window whose `phys_addr` is `None`
(a heap-content fallback created under memory pressure) is promoted to a real
shared buffer before the mapping step. The selftest must prove three things
about that behavior:

1. **The fallback can be forced**: under deterministic memory pressure, window
   creation lands in the `win.content = Some(...)` branch (`phys_addr == None`).
2. **Promotion works**: after promotion, `phys_addr` is `Some`, `content` is
   dropped, and `sys_gui_map_buffer` returns a non-zero user pointer.
3. **The promoted window is renderable**: the compositor draws the window's
   content pixels into the backbuffer exactly as a natively-allocated window
   would.

The test must run inside the kernel's existing TAP selftest framework
(`kernel/src/selftest.rs`: `register(name, fn() -> Result<(), &'static str>)`,
`run_all()` prints `ok N - name` / `not ok N - name # msg` to serial and
panics on any failure — the CI gate already greps it).

## Framework integration (exact wiring)

Add `kernel/src/tests/gui_tests.rs` mirroring `memory_tests.rs`:

```rust
// kernel/src/tests/gui_tests.rs
use crate::memory::buddy::BUDDY_ALLOCATOR;
use crate::syscalls;

pub fn test_option1_fallback_forced() -> Result<(), &'static str> { ... }
pub fn test_option1_promotion_maps() -> Result<(), &'static str> { ... }
pub fn test_option1_renderable() -> Result<(), &'static str> { ... }

pub fn register() {
    crate::selftest::register("gui::option1_fallback_forced", test_option1_fallback_forced);
    crate::selftest::register("gui::option1_promotion_maps", test_option1_promotion_maps);
    crate::selftest::register("gui::option1_renderable", test_option1_renderable);
}
```

Wire it into `kernel/src/tests/mod.rs`:

```rust
mod gui_tests;
```

and call `gui_tests::register()` from the existing `register_all` aggregator
alongside the other modules (the exact aggregator fn name/placement is up to
the rewrite; `memory_tests.rs` uses a per-module `pub fn register()`, and
`new_features.rs` shows a flat `register_all()` — either shape works, the
anchor is `selftest::register("gui::option1_*", ...)`).

## Visibility prerequisite (one-word kernel change)

`sys_gui_create_window` is currently a **private** `fn`
(`kernel/src/syscalls/mod.rs:4656`) while `sys_gui_map_buffer` /
`sys_gui_flush` are already `pub(crate)` (`:4709`, `:4762`). The test module
is a sibling of `syscalls`, so `sys_gui_create_window` must be marked
`pub(crate)` for the forced-fallback test to call it. That is the only kernel
signature change the selftest requires.

## Timing prerequisite (critical)

Selftests run from `main.rs` **before** `gui::init()` and **before any
process exists** (`CURRENT_PROCESS` is `Mutex<Option<Arc<Process>>> =
Mutex::new(None)`, `kernel/src/task/process.rs:13`). `sys_gui_map_buffer`
returns 0 when `CURRENT_PROCESS` is `None` — so the syscall-level map
assertion CANNOT run at boot-selftest time as-is.

Two options, pick per rewrite preference:

- **(a) Seed a minimal process in the test** — set `CURRENT_PROCESS` to a
  bare `Process` (default fields, an initialized `address_space` with a
  mapper) for the duration of the map test, then restore `None`. This keeps
  all three tests in the boot selftest, matching the existing `memory_tests`
  pattern of driving kernel state directly. Most selftests in the tree touch
  globals directly, so this is consistent.
- **(b) Move only the map assertion to a userspace test binary** — run the
  create/map/flush through the real syscall path from a small userspace
  program (like `futex_test`) that runs after process init; the boot selftest
  keeps the forced-fallback + renderable tests (which need no process).

The spec below writes option (a) — it keeps everything in one place and is
what `memory_tests.rs` already does for the buddy allocator. If the rewrite
prefers (b), drop `test_option1_promotion_maps` from the boot suite and
recreate it as a userspace binary; the fallback/render tests are unchanged.

**Render timing is a separate constraint:** `test_option1_renderable` calls
`Compositor::render`, which ends in the hardware framebuffer commit
(`gui/mod.rs:821`). Selftests run before `gui::init()` (`main.rs:346` vs
`:350`), so the renderable test must be registered for a **post-init** phase
— e.g. a second `selftest::run_all()` call after `gui::init()` in `main.rs`
registering only the render test, or the rewrite verifies the framebuffer
commit is a safe no-op before init. Test 1 and Test 2 (fallback + promotion
state) do not render and are fine pre-init.

## The three tests, in detail

Common setup helper (local to `gui_tests.rs`):

```rust
/// Allocate every contiguous block of `order` the buddy currently has,
/// returning them so the test can release the pressure later. Draining the
/// free list is what forces the create_window fallback deterministically:
/// after this, allocate_contiguous(order) returns None.
fn drain_order(order: usize) -> alloc::vec::Vec<(u64, usize)> {
    let mut held = alloc::vec::Vec::new();
    loop {
        let addr = BUDDY_ALLOCATOR.lock().allocate_contiguous(order);
        match addr {
            Some(pa) => held.push((pa.as_u64(), order)),
            None => break,
        }
    }
    held
}

fn release(drained: alloc::vec::Vec<(u64, usize)>) {
    let mut buddy = BUDDY_ALLOCATOR.lock();
    for (addr, order) in drained {
        buddy.deallocate_contiguous(
            x86_64::PhysAddr::new(addr),
            order,
        );
    }
}
```

### Test 1 — `gui::option1_fallback_forced`

Proves the fallback branch is reachable under pressure (the precondition the
respawn loop is built on).

```rust
fn test_option1_fallback_forced() -> Result<(), &'static str> {
    // 800x600 window: create_window computes content_len = width * height
    // (the ORIGINAL dims, NOT width-2 x height-22), so content_len = 480,000
    // px = 1,920,000 bytes -> order 9 (2 MB block). Same order loop as
    // create_window: order grows while (4096 << order) < size_bytes &&
    // order < MAX_ORDER.
    let order = 9usize;
    let title = alloc::string::String::from("selftest").into_boxed_str();
    let free_before = BUDDY_ALLOCATOR.lock().count_free_pages();
    let drained = drain_order(order);
    if drained.is_empty() {
        // No order-9 block available at boot is itself a valid OOM
        // precondition, but the test's drain would be a no-op and the
        // fallback may not fire — fail loudly so the harness stays honest.
        return Err("no order-9 blocks to drain; cannot force fallback");
    }

    let handle = syscalls::sys_gui_create_window(
        title.as_ptr(),
        800,
        600,
    );

    // Always release the drain: the window itself holds the fallback state.
    release(drained);

    let comp = crate::gui::COMPOSITOR.lock();
    let win = &comp.windows[handle as usize];
    if win.phys_addr.is_some() {
        return Err("expected content fallback, got a physical buffer");
    }
    if win.content.is_none() {
        return Err("fallback window has neither phys_addr nor content");
    }
    // Fallback is vec![0; width * height] = 480,000 u32 (create_window's
    // content_len uses the original width/height, matching map_buffer's
    // win.width-2 / win.height-22 = 800x600).
    let expected_len = 800 * 600;
    if win.content.as_ref().map(|c| c.len()) != Some(expected_len) {
        return Err("fallback content length mismatch");
    }
    drop(comp);
    // Cleanup: remove the window (drops the content Box) and verify the
    // drain+release left the buddy free count restored (memory_tests.rs
    // discipline).
    crate::gui::COMPOSITOR.lock().windows.remove(handle as usize);
    if BUDDY_ALLOCATOR.lock().count_free_pages() != free_before {
        return Err("buddy free count not restored after drain/release");
    }
    Ok(())
}
```

Notes:
- `order = 9` must be kept in lockstep with `create_window`'s own order loop.
  If the rewrite extracts the `order_for_size` helper the fix doc suggests,
  this test should call it instead of hardcoding 9 (the helper becomes the
  single source of truth).
- Draining order 9 specifically: `allocate_contiguous(9)` walks the free
  lists and splits higher orders, so draining order 9 until `None` also
  exhausts every higher-order block — the create then deterministically
  falls back.
- **Fragility guard:** if boot state already has almost no free memory, the
  drain may return zero blocks and `sys_gui_create_window`'s allocation may
  succeed anyway. The `drained.is_empty()` early-return keeps the test from
  silently passing on a non-fallback path.

### Test 2 — `gui::option1_promotion_maps`

Proves the promotion + map returns a usable pointer. Requires the option (a)
minimal-process seed (see Timing prerequisite).

```rust
fn test_option1_promotion_maps() -> Result<(), &'static str> {
    // Seed a minimal CURRENT_PROCESS so sys_gui_map_buffer's VMA/map path
    // has a process to attach to. Exact construction depends on Process's
    // fields in the rewrite; the anchor is: CURRENT_PROCESS must be Some
    // with a working address_space.mapper(). Restore None on all exits.
    let proc = crate::task::process::Process::default(); // or the rewrite's equivalent
    *crate::task::process::CURRENT_PROCESS.lock() = Some(alloc::sync::Arc::new(proc));

    let title = alloc::string::String::from("selftest").into_boxed_str();
    let drained = drain_order(9);
    let handle = syscalls::sys_gui_create_window(title.as_ptr(), 800, 600);

    // Promotion must NOT be allowed to inherit the drain (Option 1 allocates
    // its own order-9 block). Release BEFORE map so the promotion can win.
    release(drained);

    let mapped = syscalls::sys_gui_map_buffer(handle);
    *crate::task::process::CURRENT_PROCESS.lock() = None;

    if mapped == 0 {
        return Err("map_buffer returned 0 after forced fallback (promotion failed)");
    }

    // Postcondition: the window is now a real shared buffer.
    let comp = crate::gui::COMPOSITOR.lock();
    let win = &comp.windows[handle as usize];
    if win.phys_addr.is_none() {
        return Err("promotion did not set phys_addr");
    }
    if win.content.is_some() {
        return Err("promotion did not drop the content fallback");
    }
    // Cleanup: the promoted window owns an order-9 block that Window's drop
    // does NOT free (Window has no Drop impl deallocating phys_addr — the
    // compositor's remove path leaks it today). Free it explicitly so the
    // suite restores the buddy free count, then drop the window.
    let pa = win.phys_addr.expect("phys_addr confirmed above");
    drop(comp);
    {
        let mut buddy = BUDDY_ALLOCATOR.lock();
        buddy.deallocate_contiguous(x86_64::PhysAddr::new(pa), 9);
    }
    crate::gui::COMPOSITOR.lock().windows.remove(handle as usize);
    Ok(())
}
```

Notes:
- The drain-then-release ordering is the heart of the test: the drain forces
  create to fall back, and the release gives Option 1's promotion allocation
  room to succeed — modeling the transient-pressure assumption exactly
  (pressure at create, cleared by map time). This is the same transient/persistent
  distinction the boot-time memory marker (`[login] mem free=N pages`, see
  kernel-gui-window-fix.md Evidence probe) measures on real boots.
- If the rewrite lands Option 1 as an extracted helper
  (`fn promote_content_window(&mut Window) -> bool`), this test can call the
  helper on the fallback window directly and skip the process seeding — but
  asserting the full `sys_gui_map_buffer` return keeps the syscall contract
  covered, which is the point.

### Test 3 — `gui::option1_renderable`

Proves the promoted window renders like a native one. No process needed —
renders through `Compositor::render` into the backbuffer and reads pixels
back. This is the strongest proof that the "fully functional fallback" claim
holds after promotion.

```rust
fn test_option1_renderable() -> Result<(), &'static str> {
    let order = 9usize;
    let title = alloc::string::String::from("selftest").into_boxed_str();
    let drained = drain_order(order);
    let handle = syscalls::sys_gui_create_window(title.as_ptr(), 800, 600);
    release(drained);

    // Seed CURRENT_PROCESS BEFORE the map call — same as Test 2. Option 1's
    // promotion runs at the TOP of sys_gui_map_buffer, BEFORE the
    // CURRENT_PROCESS check, so with no process the promotion would succeed
    // but map would still return 0 (the check after the size computation).
    // Seeding keeps the guard below meaningful and exercises the full path.
    let proc = crate::task::process::Process::default();
    *crate::task::process::CURRENT_PROCESS.lock() = Some(alloc::sync::Arc::new(proc));
    let mapped = syscalls::sys_gui_map_buffer(handle);
    *crate::task::process::CURRENT_PROCESS.lock() = None;
    if mapped == 0 {
        return Err("map_buffer returned 0; nothing to render");
    }
    // Write the pattern through the PHYSICAL address (win.phys_addr +
    // physical_memory_offset) — the exact pointer the zero-copy flush and
    // render paths read — so no user mapping needs to be active. `mapped`
    // (a VA in the seeded process) is deliberately unused for the write.
    let comp = crate::gui::COMPOSITOR.lock();
    let win = &comp.windows[handle as usize];
    let pa = win.phys_addr.expect("promotion must set phys_addr");
    let k_ptr = (crate::memory::physical_memory_offset() + pa) as *mut u32;
    let pattern = 0xFFAABBCCu32;
    unsafe { core::ptr::write(k_ptr, pattern); } // first content pixel
    drop(comp);

    // Render into the compositor backbuffer, then read the window's content
    // origin back. Window is at (0,0), content area starts at (1, 21):
    // (content_x, content_y) = (x+1, y+21) = (1, 21).
    // TIMING: this render MUST run after gui::init() (render ends in the
    // hardware framebuffer commit, gui/mod.rs:821). If the suite runs
    // pre-init, register this one test for a post-init phase (a second
    // selftest::run_all() after gui::init in main.rs) or verify the commit
    // is a safe no-op pre-init.
    let mut comp = crate::gui::COMPOSITOR.lock();
    comp.render(0, 0);
    let got = comp.backbuffer[21 * crate::gui::SCREEN_WIDTH + 1];
    if got != pattern {
        return Err("promoted window content not rendered to backbuffer");
    }
    // Free the promoted block explicitly (Window drop does not), then remove.
    let mut buddy = BUDDY_ALLOCATOR.lock();
    buddy.deallocate_contiguous(x86_64::PhysAddr::new(pa), 9);
    drop(buddy);
    comp.windows.remove(handle as usize);
    Ok(())
}
```

Notes:
- Writing through the physical address (not the user-mapped VA) removes the
  CURRENT_PROCESS dependency for the render check entirely — the render path
  reads `physical_memory_offset + phys_addr` (the same pointer the zero-copy
  flush path uses), so this is exactly what production rendering sees.
- `pattern` at content origin (1,21): verify against the rewrite's
  `Window::render` — if `render` draws the title bar over that pixel or the
  window is offset, read a pixel from the middle of the content area instead
  (e.g. `(content_w/2, content_h/2)`). The assertion is the contract: a
  promoted window's pixels reach the backbuffer.
- The render path draws the fallback `content` branch **before** the
  `phys_addr` branch (`window.rs` content/phys ordering), so after promotion
  (`content = None`) only the phys path can satisfy the pixel — the test is
  guaranteed to exercise the promoted path, not the old fallback.

## Assertion checklist (what the rewrite must verify)

| # | Assertion | Failure means |
|---|---|---|
| 1 | Fallback window has `phys_addr == None`, `content == Some(len 800*600 = 480,000)` (create_window uses original width*height) | create_window's fallback unreachable / test drain wrong |
| 2 | `map_buffer` returns non-zero after release | Option 1 promotion not implemented or its alloc failed |
| 3 | `phys_addr == Some` and `content == None` after map | promotion half-applied |
| 4 | Backbuffer pixel at content origin == written pattern after `render` | promoted window not renderable |
| 5 | Suite leaves compositor empty and buddy free count restored (explicit `deallocate_contiguous` of any promoted block — Window's drop does not free `phys_addr`) | leaked test state corrupts later tests |

Also assert the **pre-fix failure** once for honesty: with Option 1 NOT
applied, test 2 must fail (`map_buffer` returns 0) — if it passes, the test
isn't exercising the fallback (see the `drained.is_empty()` guard).

## Second user-visible path — forced-failure boot leg (drain hook + serial markers)

The three TAP tests above prove Option 1's mechanism **synthetically** — the
buddy drained inside the kernel, before any process exists. This section
specs the **real-hardware leg**: a test-only kernel boot flag that drains the
buddy at boot and HOLDS the blocks, so login-manager's own 800x600
`Window::create` runs under genuine memory pressure and the boot's serial
stream carries the two markers the QEMU gates assert. The **first**
user-visible path — the healthy boot (`[login] window created`, GUI gate
PASS, fix doc Verification plan item 1) — is covered there; this is the
second, the forced-failure boot. The leg is the bridge between the selftest
(drain helper, synthetic) and the give-up harness (mem-series capture, real
boot): the same drain mechanism, one level up the stack.

### Drain hook contract (kernel, test-only)

Mirror vahid's userspace `--force-fail` test hook, kernel-side. When the
rewrite lands a working buddy allocator, add a test-only boot flag
(`SKYOS_DRAIN_BUDDY=<order>`, default unset) honored in `kernel/src/main.rs`
after memory init and before the first userspace process is spawned:

- Parse the flag; when set to order N, run the selftest's `drain_order(N)`
  loop — allocate every contiguous N-order block the buddy holds — and
  **hold** the blocks (keep the returned Vec alive in a `static`, or
  `core::mem::forget` it; never release), so `count_free_pages()` stays
  near zero for the whole boot.
- **Hold, not release:** the selftest drains then releases to model
  *transient* pressure (Option 1's promotion has room after release). The
  boot leg models the *persistent* row of the fix doc's evidence table:
  free stays low across every login-manager respawn, so the give-up
  harness's per-respawn series is flat and classifies Option 2. Both rows
  of the table now have a deterministic driver.
- Order 9 for login-manager's 800x600 window (create_window computes
  content_len = 800*600*4 bytes -> order 9 — the same constant the tests
  above hardcode; share the `order_for_size` helper the fix doc suggests).
- No production behavior change: the flag is absent on every normal boot.
  The exact delivery (boot arg vs early env) is the rewrite's call — the
  flag NAME is the stable contract, like the `gui::option1_*` TAP names.

### Serial markers the leg asserts

With the drain hook set, login-manager's real create runs under pressure and
the boot must show, in order:

1. `[login] mem free=N pages` with N **near zero** (the fix doc's `< 2k
   pages` ≈ `< 8 MB` line) — login-manager/src/main.rs:56 (ctlFS
   `/ctl/sys/mem/free`), printed before every `Window::create`.
2. `[login] failed to create window` — login-manager/src/main.rs:66
   (`Window::create` Err: the fallback's `phys_addr == None` makes
   `sys_gui_map_buffer` return 0, so libsarga yields `Err(5)` and this
   print runs).
3. `[login] window created` stays ABSENT on this boot.

No new serial print is needed: the drain is proven by the marker's magnitude
(near zero) and the failure by the existing `failed to create window` line.

### Evidence-table mapping

| Boot-leg observation (drain hook set) | Selftest equivalent | Verdict | Kernel fix |
|---|---|---|---|
| `mem free=N` near zero + `failed to create window`, series flat across respawns | `gui::option1_fallback_forced` (drain, no release) | persistent OOM | Option 2 + 2b |
| `mem free=N` recovers + `window created` | `fallback_forced` + release + `promotion_maps` | transient | Option 1 |

### Bridge to selftest + gate

- The drain hook reuses the selftest's `drain_order` helper — one source of
  truth for "drain until `allocate_contiguous` returns None"; the hook is
  the same loop called at boot with a hold-leak instead of a returned Vec.
- The give-up harness (`qemu_giveup_boot.exp`, audit row 6) already captures
  the per-respawn `mem_readings` series live. When the hook lands, its
  forced-failure boot additionally greps `[login] failed to create window`
  and asserts the first reading is near zero — turning the persistent
  verdict from a NOTE/PASS series classification into a hard requirement on
  a boot where pressure is guaranteed.
- Division of labor: the selftest proves the mechanism pre-userspace (no
  process exists, so no serial login-manager can run); the boot leg proves
  the same pressure conditions through the real syscall path and real init
  respawn accounting. The selftest alone cannot produce the serial markers,
  and the boot leg alone cannot isolate the drain (the mem marker is the
  only probe) — hence both.

## CI

The TAP lines print to serial (`ok 2 - gui::option1_promotion_maps`), so the
existing kernel-selftest CI gate that greps `not ok` catches a regression
automatically. If the rewrite wants a narrower tripwire, grep for
`gui::option1_` lines specifically.

## Files touched by this spec (when the rewrite lands)

1. `kernel/src/tests/gui_tests.rs` — NEW, the three tests + helpers.
2. `kernel/src/tests/mod.rs` — declare `mod gui_tests;` + call its `register()`.
3. `kernel/src/syscalls/mod.rs` — `fn sys_gui_create_window` → `pub(crate) fn`
   (visibility only, no behavior change).
4. Optionally extract `order_for_size()` shared by create_window + the test
   (the fix doc's existing suggestion).
5. `kernel/src/main.rs` — the `SKYOS_DRAIN_BUDDY` boot-flag parse + the
   drain-hold call (test-only; the second user-visible path above).
6. `tests/qemu_giveup_boot.exp` — GATED on the hook landing: the
   `[login] failed to create window` grep + near-zero first-reading
   assertion on the forced-failure boot (see the leg section).
