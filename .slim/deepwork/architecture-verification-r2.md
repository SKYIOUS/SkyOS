# Architecture Verification Round 2 — docs/arch.md vs. code (re-verify after Phase 1.1/1.2)

Re-verified every claim in the CURRENT `docs/arch.md` against source. All line numbers below are
as of this session.

## arch.md claims — verified

1. **Kernel codemap placeholder (TRUE, still stale)** — `docs/codemaps/skyos-kernel-architecture-*.md`
   is a 17-line placeholder and still describes "priority-based round-robin", which the scheduler
   has not been since the stride scheduler landed. Fixing it = regenerate from kernel repo (deferred,
   kernel is external).
2. **Dual permission systems (CONFIRMED)** — `desktop.rs:137` `permissions: PermissionManager`;
   `process_ipc()` at `desktop.rs:1742-1776` gates each request on `required_permissions` via the
   service registry. Both active. (Line 123 in doc is `recovery`; doc line drifted.)
3. **Damage regions exist but unused (CONFIRMED)** — `core/damage.rs` DamageTracker (Rect add/merge/
   drain); `compositor.rs:781` `compose(win, damage_rects: Option<&[Rect]>)` partial path at :810;
   ALL call sites `mark_full()`; `render/mod.rs:208` calls `compose(win, None)`. Regional infra
   dead-wired.
4. **Unwraps (FIXED, confirmed)** — no `unwrap()` in desktop.rs/launcher.rs anymore.
5. **Init respawn limit (FIXED, confirmed)** — `init/src/main.rs` `MAX_RESPAWNS = 5`, `crashes: u32`.
6. **WindowId stale index (FIXED, was CONFIRMED + a LIVE bug)** — `WindowId` was `WindowId(usize)`
   at `window.rs:7`; assigned `WindowId(len-1)` at `window_manager.rs:47`. **Live bug was**:
   `desktop.rs:296-300` — `process_closing()` (window_manager.rs:72-86) REMOVED finished windows and
   returned their indices, then desktop called `wm.close(cid)` on each; `close()` did `get_mut(id.0)`
   and if the returned index was in range it could mark a *different* window closing → wrong window
   closes. Also `resize_win: Option<WindowId>` (desktop.rs:90) stored a handle across frames with no
   invalidation on window removal.
   **FIXED in Phase 1.3**: `WindowId(pub(crate) u64)` (window.rs:7); `AppWindow` gains `id: u64`
   (window.rs:78); `WindowManager` assigns a monotonic `next_id` counter in `create()` and resolves
   every id via `find_index()` linear scan. `focused`/`dragging` now store stable ids
   (`Option<u64>`), `close_by_pid`/`process_closing` call `clear_refs()` on removal, and the tick
   loop only marks full damage (no re-close). Desktop call sites use `wm.id_at(pos)` / stable ids.
7. **Compositor OOM (FIXED, was CONFIRMED)** — `compositor.rs:677-684` was 6× `LayerBuffer::new(pixels)`
   where `new` did `vec![0u32; pixels]` (:652). No fallback. 800×600 = 6×1.92MB ≈ 11.5MB; 1920×1080 ≈ 49.7MB.
   **FIXED in Phase 2.3**: `LayerBuffer::alloc` uses `try_reserve_exact` + `resize`, returning `Err(())`
   on OOM; `Compositor::new` returns `Option<Self>`; callers (main.rs, benchmark, testing) fail cleanly
   instead of panicking via the alloc error handler.
8. **a11y full rebuild per frame (CONFIRMED)** — `desktop.rs:303` calls `build_a11y_tree()` (def at
   :348) every tick; tree cleared each time.
9. **Scheduler docs (CURRENT docs correct)** — SCHEDULER.md + architecture/scheduling.md now describe
   stride scheduling; matches `task/scheduler.rs` (PassOrd heap, tickets default 20, STRIDE_MAX
   1<<20, work-stealing ≤3 CPUs, pending_queue).

## Extra findings (not in doc)

- `render/mod.rs:208` is the ONLY `compose()` call site and passes `None` always.
- WM `focused`/`dragging` WERE `Option<usize>` (window_manager.rs:29-30) — stale after
  `close_by_pid`/`process_closing` if the removed window was focused/dragged. NOW `Option<u64>`
  stable ids with `clear_refs()` on removal.
- `system_menu_for` WAS `Option<usize>` (desktop.rs:131) — NOW `Option<WindowId>` (stable id
  captured at right-click; no bounds guard needed since lookups scan).
- ADE `#[test]` modules exist (sys/network.rs:326+, input.rs, display.rs, audio.rs, util/*) but CANNOT
  run: `cargo test --target x86_64-sarga.json` fails with "duplicate lang item in crate `core`:
  `sized`" (pre-existing target/test-harness incompatibility, not caused by us).

## Corrected overview (deltas vs. doc)

- Doc's "direct crash vector" framing for WindowId was wrong: every access was via `get_mut`/bounds
  check → it is a *wrong-target logic bug*, not memory-unsafety. Still worth fixing (wrong window
  closes/restores/moves), but the severity is "logic correctness", not "crash". **Now FIXED** — stable
  u64 ids.
- Tiling/monocle loops (desktop.rs:459-524) iterate `0..len` and `lookup_mut(WindowId(i))` — SAFE
  because they never mutate the window list inside the loop. Not a bug; now migrated to
  `wm.id_at(i)` + `lookup_mut(wid)`.

## Proposed improvements (ranked)

1. ~~**Fix live wrong-close bug + harden stale handles (Phase 1.3, scoped)**~~ — DONE. WindowId is now
   a stable u64 assigned at create; WM looks up by scanning for `w.id == id` (window count is tiny,
   ≤ ~6). Kills the whole stale-index class (focused, dragging, resize_win, system_menu_for) at its
   root. Redundant `close(cid)` in the tick loop removed. No HashMap needed.
2. Wire regional damage (Phase 2.1) — infra exists; needs damage.add() at small-update sites + pass
   rects through render(). Can't visually verify here; defer.
3. ~~**Compositor OOM fallback (Phase 2.3)**~~ — DONE. `try_reserve_exact` + all-or-nothing
   `Option<Self>` from `Compositor::new`; OOM degrades to a clean failure instead of a panic.
4. a11y incremental rebuild (Phase 2.2) — medium, defer.

## Selected phase

**Phase 1.3 (WindowId → stable u64)** — the doc's top recommendation, fixes a live wrong-window bug,
and eliminates the entire stale-index class. Implemented with linear-scan lookup (no HashMap) since
window counts are tiny. Every step keeps the build green. **DONE** — ade release + full workspace
release + kernel build all green.

**Phase 2.3 (Compositor OOM fallback)** — follows: allocation failure now fails cleanly instead of
panicking. **DONE** — ade release + workspace release + kernel build all green.
