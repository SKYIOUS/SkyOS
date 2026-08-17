# SkyOS Architecture Analysis

## Complete Architectural Overview

### System Architecture

SkyOS is a monolithic kernel operating system with a modular userspace desktop environment.

**Kernel Layer** (external repo):
- Monolithic kernel written in Rust with `#![no_std]` - all FS, drivers, and network stack run in-kernel
- Higher-half mapped at `0xFFFFFFFF80000000`
- Memory: Buddy allocator (physical), Slab allocator (kernel heap at `0xFFFF_C000_0000_0000`, 128 MiB)
- Scheduler: Per-CPU, preemptive, **stride scheduling** (tickets/pass proportional-share) - NOT priority round-robin
- VFS: Trait-based (`VfsNode`, `FileSystem`); 9 filesystems: ramfs, devfs, ctlfs, pipe, tarfs, skyfs, ext2 (read-write), ext4 (feature-gated), fat
- Syscalls: x86_64 SYSCALL instruction, POSIX-compatible ABI (~170 handlers, 171 `SYS_*` constants)
- Drivers: storage (ahci, nvme, pata, virtio_block), net (e1000, virtio), usb (uhci, xhci, hid), audio (hda, pcspeaker), gpu (virtio_gpu), framebuffer (bga)

**Userspace Layer** (this repo):
- Entry point: `_start()` in libsarga, parses argc/argv from stack
- Standard library: libsarga provides syscall wrappers, I/O, process management
- Init system: PID 1 spawns services (login-manager, svc) with **MAX_RESPAWNS = 5** crash limit
- Desktop environment: ADE with compositor, window manager, IPC

### Boot Flow

1. **UEFI → Bootloader → Kernel**: UEFI loads bootloader, which loads kernel ELF and sets up memory map
2. **Kernel Boot State Machine**: Locates init binary, parses ELF, creates address space, enters userspace
3. **Init (PID 1)**: Mounts /tmp (tmpfs), /dev (devfs), /ctl (ctlfs); spawns login-manager and svc
4. **Login Manager**: GUI authentication via /etc/shadow, execs /bin/ade on success
5. **ADE Desktop**: Creates window, initializes compositor, enters 60 FPS event loop

### Desktop Environment Architecture

**ADE Components**:
- **Main Event Loop**: Polls keyboard/mouse, ticks desktop state, renders on damage, sleeps to 60 FPS
- **Desktop Coordinator**: Central state manager, handles events, coordinates subsystems
- **Window Manager**: Vec-based ordered window list with z-order, focus, drag, minimize, close animations
- **Compositor**: 6-layer rendering (Wallpaper, Desktop, Windows, Popups, Overlay, Cursor) with alpha blending
- **IPC Transport**: AF_UNIX socketpair per app, non-blocking poll, request/response framing
- **Portal/Services**: Permission-gated service dispatch (Clipboard, Notification, Settings)
- **Lifecycle Manager**: Tracks process states (running, terminated, crashed)
- **Permission Manager**: Bitmask-based permission grants per PID
- **Accessibility Tree**: Per-frame rebuild of a11y nodes for screen readers
- **Service Manager**: Notification timeout handling, power management

## Inconsistencies Between Docs and Implementation

### 1. Kernel Architecture Codemap Missing
**Issue**: The SkyOS Kernel Architecture codemap file is a placeholder with no content.
**Impact**: Cannot verify kernel implementation against documented design.
**Status**: Kernel code is in external repo not included in this workspace.

### 2. Dual Permission Systems (CONFIRMED)
**Doc**: Breakage-prone codemap identifies "dual permission systems" as a risk.
**Implementation**: Both systems are active:
- `PermissionManager` grants bitmask permissions at launch (`desktop.rs:123`)
- Service registry has `required_permissions` field checked at dispatch (`desktop.rs:1755-1764`)
**Inconsistency**: Docs flag this as risk but implementation has both systems active without unification.

### 3. Damage Regions Exist but Are Wired to Full Recomposite (CONFIRMED)
**Doc**: Breakage-prone codemap identifies full-frame recomposite as performance risk.
**Implementation**: 
- `core/damage.rs` has a full `DamageTracker` (Rect add/merge/drain)
- `compositor.rs:781` `compose(win, damage_rects: Option<&[Rect]>)` supports partial recomposite
- But every call site uses `damage.mark_full()` (60+ sites) and `render/mod.rs:208` calls `compose(win, None)` → still full-frame every time
**Inconsistency**: Regional tracking infrastructure is implemented but unused; only dirty-flag semantics are exercised.

### 4. Unwrap() Panic Risks (FIXED)
**Doc**: Breakage-prone codemap identifies unwrap() calls as panic risk.
**Status**: All unwrap() calls in desktop.rs/launcher.rs have been removed. The last one (desktop.rs:1433 `explorer_id.unwrap()`) was replaced with `if let` in a refactor pass.

### 5. Init Service Respawn Without Crash Count Limit (FIXED)
**Doc**: Breakage-prone codemap identifies infinite respawn loop risk.
**Status**: [init/src/main.rs](cci:7://file:///c:/Users/nanda/Desktop/Github/SkyOS/init/src/main.rs:0:0-0:0) now has `MAX_RESPAWNS = 5` - services that crash more than 5 times are disabled with a `[init] giving up on <name>` message.

### 6. Window Manager Vec Index Staleness (FIXED)
**Doc**: Breakage-prone codemap identifies stale indices after Vec::remove.
**Implementation**: 
- `window_manager.rs:79` removes window during iteration
- `window_manager.rs:64` removes by PID
- No index invalidation mechanism
- `WindowId` is just `WindowId(usize)` - index into Vec
**Inconsistency**: Docs identify risk but WindowId remains as direct Vec index, susceptible to staleness.
**Fixed**: `WindowId` is now `WindowId(u64)` — a monotonically increasing stable id assigned at `create()`. All WM lookups (`lookup`, `close`, `focus`, `bring_to_front`, `minimize`, `snap_to_region`, `restore`, `begin_drag`, focus cycling) resolve ids via `find_index()` scan (window count is tiny; no HashMap). `focused`/`dragging` store ids, not positions. `close_by_pid`/`process_closing` clear `focused`/`dragging` if the removed window was referenced. Desktop call sites (hit-test, taskbar, system menu, task manager, switcher, middle-click, resize) use `wm.id_at(pos)` to obtain stable ids instead of `WindowId(pos)`. Also fixes the live wrong-window-close bug where `process_closing()` returned indices and `close()` re-targeted shifted windows — `desktop.rs` tick now only marks full damage.

### 7. Compositor OOM Risk (FIXED)
**Doc**: Breakage-prone codemap identifies OOM on large screens (75MB at 1920x1080).
**Implementation**: 
- `compositor.rs:677` allocates 6 LayerBuffers of width*height*4 bytes each
- No fallback or error handling
**Inconsistency**: Docs identify risk but no mitigation implemented.
**Fixed**: `LayerBuffer` now uses `try_reserve_exact` + `resize` and reports `Err(())` instead of panicking through the global alloc error handler. `Compositor::new` returns `Option<Self>` (all-or-nothing — every layer is full-screen, so a partial allocation can't be composed); `ade/src/main.rs` and the test/bench call sites fail cleanly (log + exit / FAIL) instead of aborting.

### 8. Accessibility Tree Full Rebuild Per Frame (CONFIRMED)
**Doc**: ADE codemap shows [build_a11y_tree()](cci:1://file:///c:/Users/nanda/Desktop/Github/SkyOS/ade/src/core/desktop.rs:347:4-449:5) called every tick.
**Implementation**: 
- `desktop.rs:275` calls [build_a11y_tree()](cci:1://file:///c:/Users/nanda/Desktop/Github/SkyOS/ade/src/core/desktop.rs:347:4-449:5) every frame
- `desktop.rs:321` clears entire tree before rebuild
**Inconsistency**: No incremental updates - full rebuild every 16ms.

### 9. Scheduler Documentation Inconsistency
**Doc**: Old codemaps mentioned "priority-based round-robin" scheduling.
**Actual Implementation**: Stride scheduling (proportional-share with tickets/pass) as documented in SCHEDULER.md and architecture/scheduling.md.
**Inconsistency**: Old codemaps had stale scheduler information; current docs are correct.

## Highest-Impact Refactoring Phase

### Phase 1: Critical Stability Fixes (Immediate Priority)

**1.1 Replace unwrap() with proper error handling** - DONE. All unwrap() calls removed.

**1.2 Add crash count limit to init respawn** - DONE. `MAX_RESPAWNS = 5` implemented.

**1.3 Implement WindowId as stable identifier** - DONE. `WindowId` is now a stable `u64` id (monotonic counter in `WindowManager`), resolved to list positions via linear scan on lookup. Removes the index-staleness crash class and the live wrong-window-close bug.

### Phase 2: Performance Optimization (High Priority)

**2.1 Implement regional damage tracking**
- **Impact**: Reduces CPU usage from full-frame recomposite to partial updates
- **File**: [ade/src/core/desktop.rs](cci:7://file:///c:/Users/nanda/Desktop/Github/SkyOS/ade/src/core/desktop.rs:0:0-0:0) (damage tracker), `ade/src/render/mod.rs`
- **Approach**: Replace boolean dirty flag with Rect-based damage regions, only recomposite damaged areas
- **Priority**: HIGH - Infrastructure exists, just needs wiring

**2.2 Implement incremental a11y tree updates**
- **Impact**: Reduces per-frame overhead from full tree rebuild
- **File**: `ade/src/core/desktop.rs:321-449`
- **Approach**: Track dirty nodes, only rebuild changed subtrees, use diff-based updates
- **Priority**: MEDIUM - Performance improvement but not critical

**2.3 Add compositor memory fallback**
- **Impact**: Prevents OOM on high-resolution displays
- **File**: `ade/src/render/compositor.rs:674-690`
- **Approach**: Try allocation, fallback to lower resolution or tiled rendering on failure
- **Priority**: MEDIUM - Only affects high-res displays

### Phase 3: Architecture Cleanup (Medium Priority)

**3.1 Consolidate dual permission systems**
- **Impact**: Reduces complexity, eliminates permission check redundancy
- **Files**: `ade/src/core/desktop.rs:1755-1764`, `ade/src/sec/perms.rs`
- **Approach**: Unify PermissionManager bitmask with service registry required_permissions into single check
- **Priority**: MEDIUM - Code complexity reduction

**3.2 Implement proper event queue**
- **Impact**: Eliminates input polling inefficiency, enables event batching
- **File**: `ade/src/main.rs:48-76`
- **Approach**: Replace direct dispatch with event queue, batch input events per frame
- **Priority**: LOW - Current polling works but is inefficient

### Phase 4: Documentation Alignment (Low Priority)

**4.1 Complete kernel architecture codemap**
- **Impact**: Provides complete system documentation
- **Approach**: Obtain kernel codemap content or generate from kernel repo

**4.2 Verify boot state machine implementation**
- **Impact**: Ensures documented design matches implementation
- **Approach**: Audit kernel repo for boot state machine implementation status

## Recommended Starting Point

**Begin with Phase 1.3 (WindowId as stable identifier)** - This is the highest stability risk with moderate architectural changes. The Vec-based WindowId is a direct crash vector when windows are removed during iteration or by PID, as subsequent index lookups can access invalid memory. Replacing with a generation-based HashMap lookup provides immediate stability improvements without requiring significant refactoring of the rendering or IPC subsystems.

**Second priority: Phase 2.1 (Regional damage tracking)** - The infrastructure already exists (DamageTracker with Rect support, compositor accepts damage_rects parameter). This is a high-impact performance win with relatively low implementation cost - simply replace the 60+ `damage.mark_full()` calls with region-specific damage updates and wire the damage rects through to the compositor.