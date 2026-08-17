# Kernel exit → reap chain — unblock design for init's respawn accounting

**Status:** SPEC ONLY — no kernel code changed. The kernel is mid-major-change.
Anchors are verified against the current tree (`SKYIOUS KERNEL/kernel/src/`,
Aug 12, 2026); function names + syscall numbers are the stable anchors, line
numbers drift. **Intended for:** the kernel rewrite to pick up verbatim (same
convention as `kernel-gui-modifier-delivery.md`, `kernel-tcsets-echo.md`).

**Purpose:** make a child's `exit_code` observable to the parent's
`waitpid` scan, so `init`'s respawn accounting (`[init] service X exited`,
`giving up on X after too many crashes`) works on real hardware — the
precondition for the boundedness harnesses (`tests/qemu_giveup_boot.exp`,
`tests/test_init_golden_trace.py::SVC_EXIT_BOOT`). Today the chain is 90%
present in the *working tree* but the OOM-kill path removes the child from
the process table before the parent can reap it, and the whole chain is
uncommitted, so a live boot still shows services dying with **no**
`[init] service X exited` (KERNEL-GATED in the give-up harness).

---

## 1. The chain today (verified, not assumed)

| Hop | Site (kernel/src/) | What happens |
|---|---|---|
| Normal exit | `syscalls/mod.rs:1855` `sys_exit` | sets `*process.exit_code.lock() = Some(status as i32)` (`:1859`), raises `SIGCHLD` on the parent (`:1876-1881`), marks the thread `Exited`, `schedule()`s away, then hlt-loops. **Process stays in `PROCESS_TABLE`** — reaping is left to the parent. |
| exit_group | `syscalls/mod.rs:1907` `sys_exit_group` | `println!` then calls `sys_exit(status)` — same bookkeeping. |
| SIGSEGV death | `interrupts.rs:516-555` page-fault handler | prints `[SIGSEGV] pid=… (killing process)`, sets `exit_code = Some(139)` (128+11, `:543`), raises SIGCHLD, marks thread Exited, schedules away. **Process stays in the table** (same reap-by-parent contract). |
| OOM / kill | `task/process.rs:778` `kill_process` (caller `main.rs:146`) | sets `exit_code = Some(-1)` (`:781`), raises SIGCHLD — **then `table.remove(&pid)` (`:786`)**. ❌ The child is gone before the parent can reap it. |
| Reap | `syscalls/mod.rs:2879` `sys_wait4` | scans `parent.children`, looks each child up in `PROCESS_TABLE`, reads `child.exit_code`; on `Some(status)` writes it through `status_ptr`, removes the child from `children` + the table, returns the pid. No `Some` → WNOHANG returns 0, otherwise `check_signal_interrupt()` (→ EINTR) or sleep-one-tick and re-scan. |
| Dispatch | `syscalls/mod.rs:657` | `SYS_WAIT4 (61) => sys_wait4(...)` — live. |
| Userspace | `libsarga/src/process.rs:95` `waitpid` | `syscall4(61, pid, &mut status, options, 0)` — already correct. |

So the happy path already works **in the working tree**: a child that calls
`sys_exit`/`sys_exit_group` or dies by SIGSEGV sets `exit_code` and stays
visible; `sys_wait4` finds it and reaps. The userspace half needs no change.

## 2. The gaps the rewrite must close

### Gap 1 (blocking): `kill_process` removes before reap — `task/process.rs:786`

```rust
pub fn kill_process(pid: u64) {
    let mut table = PROCESS_TABLE.lock();
    if let Some(proc) = table.get(&pid) {
        *proc.exit_code.lock() = Some(-1);
        crate::println!("[OOM] Killed process pid={}", pid);
        if let Some(parent) = proc.parent_id.and_then(|ppid| table.get(&ppid)) {
            parent.signals.lock().raise(crate::syscalls::signal::Signal::SIGCHLD);
        }
        table.remove(&pid);   // ❌ child vanishes before the parent can reap
    }
}
```

The `table.remove(&pid)` at `:786` means the parent's `sys_wait4` scan
(`process_table.get(&child_pid)` → `None`) can never see this child, and the
parent's `children` vec keeps the pid forever (a **stale entry** — the reap
never removes it). Consequences for init: an OOM-killed svc/vahid gets no
`[init] service X exited`, no crash accounting, no give-up. The boundedness
claim silently stops being exercised.

Scope note: this gap affects **OOM-killed children specifically**. The
stock-boot boundedness path (svc's `sys_exit(1)`, login-manager's SIGSEGV)
already keeps the child in the table — what blocks *that* path today is the
uncommitted reap itself (see §4). R1 is the fix for the OOM-kill leg of the
chain; it does not change the stock-boot path.

**Fix (minimal, mirrors `sys_exit`'s contract):** drop the `table.remove`
line. The exited child stays in `PROCESS_TABLE` with `exit_code = Some(-1)`
until the parent's `waitpid` reaps it — exactly like `sys_exit` does today.
Remove the `#[allow(dead_code)]` if it's now live, and keep the OOM
diagnostic. Optionally mark the process `Exited` (thread state) as the other
paths do, though a killed process has no running thread to mark.

### Gap 2 (latency, non-blocking): blocking `sys_wait4` is tick-polled

`sys_wait4`'s no-child-exited path does `sleep_until = ticks + 1` and
re-scans each tick (`:2934-2940`). SIGCHLD is raised but does **not** wake
the waiter directly, so a blocked parent reaps with up to 1 tick (10ms)
latency. Acceptable for init's respawn loop; noted so the rewrite doesn't
"optimize" the raise away or add a direct wake that races the scan.

### Gap 3 (convention): exit status is RAW, not POSIX-encoded

`sys_wait4` writes the raw `exit_code` (`1`, `139`, `-1`, …) through
`status_ptr`. POSIX would encode `WEXITSTATUS(status) = (status >> 8) & 0xff`
with the signal in the low byte. **The userspace stack already consumes raw
statuses deliberately** — `init` checks `status == 0` vs non-zero, and
`ade/src/service/session.rs` `exit_class` classifies `0` = Clean,
`128+sig` = Signal, negative = Killed. Keep RAW; document it in the syscall
comment so a future POSIX-encoded caller doesn't double-shift.

### Gap 4 (quirk): the `status != 42` suppress in `sys_exit`

```rust
if status != 42 {
    crate::println!("[PROCESS] Pid {} exited with status {}", process.id, status);
}
```

`42` is a magic sentinel with no in-tree meaning: `grep -rn "\b42\b"
init/ login/ login-manager/ vahid/ svc/ libsarga/` → no hits, and
`SYS_EXIT (60)` has no documented 42 convention. Drop the suppression and
always print — a respawn-loop service exiting with 42 today silently loses
its exit line, which would confuse the give-up greps. (Safe either way: the
guard only *suppresses* a log line, so removing it adds output.)

---

## 3. The reviewable change set (what the rewrite must land)

One behavioral change (Gap 1) + two hygiene items (Gap 3 comment, Gap 4).

| # | File | Change |
|---|---|---|
| R1 | `kernel/src/task/process.rs:786` | **Delete `table.remove(&pid);`** in `kill_process` — the child must stay in `PROCESS_TABLE` until the parent reaps it (exit_code already `Some(-1)`). |
| R2 | `kernel/src/syscalls/mod.rs:2896-2923` | Keep the `sys_wait4` scan as-is (reads `exit_code`, writes status, removes child + entry). Optionally add a comment: status is RAW (`0` / `128+sig` / negative), not POSIX-encoded — see Gap 3. |
| R3 | `kernel/src/syscalls/mod.rs:1860` | Remove the `status != 42` guard so every exit prints its `[PROCESS]` line. |
| R4 | (already present) `sys_exit` `exit_code` write + SIGCHLD raise + stay-in-table; SIGSEGV path `:543` | Preserve unchanged — they are the observable-to-parent half. |

**Landing condition (the gate flips from KERNEL-GATED to hard):**
`tests/qemu_giveup_boot.exp` observes `[init] service svc exited` followed by
`giving up on svc after too many crashes` on a real boot (no more
`KERNEL-GATED:` deferral), AND `tests/test_init_golden_trace.py`'s
`SVC_EXIT_BOOT` — the wire-faithful exit-side trace — replays through
`RespawnAccounting` to exactly `respawn × MAX_RESPAWNS` then `gave_up`.

**Zombie lifetime (standard):** after R1, an OOM-killed child is a table
zombie (entry + `exit_code`) until the parent reaps it — exactly like a
normal `sys_exit`. This is fine because init always blocks in
`waitpid(-1, 0)`, which removes the entry at reap; a process whose parent
never calls waitpid would leak the entry, same as Linux's unreaped-zombie
behavior. No new reaper is needed.

**Not changed by this spec:** `sys_wait4`'s WNOHANG semantics, the
`pid == -1` any-child scan, EINTR-on-SIGCHLD behavior (a pending SIGCHLD can
still EINTR a blocking wait — init's `Err(_)` arm sleeps 100ms and re-loops,
so it is tolerated), and `exit_code` reset on clean exit (init clears
`crashes = 0` userside on `status == 0`).

---

## 4. Why init sees nothing today (evidence)

Live boot log (Aug 10, 2026, captured in
`tests/test_init_golden_trace.py::REAL_BOOT`): the four services spawn, then
`Usage: svc …`, then `[SIGSEGV] pid=106 … (killing process)` + `[KILL3] mark
exited` — **and no `[init] service … exited` line follows**, so the
accounting never observes a single exit. Two causes combine:

1. The working-tree reap (Gap 1 + the `sys_wait4` scan) is **uncommitted**
   (`git status` shows `kernel/src/syscalls/mod.rs` modified, +103 lines), so
   the committed kernel a boot uses lacks the observability half entirely.
2. Even with the working tree, `kill_process` (Gap 1) would make OOM-killed
   children unreapable.

The golden trace's exit side (`SVC_EXIT_BOOT`) pins the *userspace* contract
for this chain now, so when the rewrite lands R1–R4, the harness flips from
KERNEL-GATED to hard PASS without any userspace edit.
