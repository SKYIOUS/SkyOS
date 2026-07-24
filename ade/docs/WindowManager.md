# Window Manager

## Overview

`WindowManager` in `window_manager.rs` orders a `Vec<AppWindow>` and manages focus, drag, resize, minimize, maximize, fullscreen, and snap operations.

## Data Model

```rust
pub struct WindowManager {
    windows: Vec<AppWindow>,
    focused: Option<usize>,
    dragging: Option<usize>,
}
```

Windows are stored in drawing order (last = topmost).

## Window Creation

`WindowManager::create(window: AppWindow) -> WindowId`

1. Push window to end of `windows` vec
2. Set focused index to new window's position
3. Return `WindowId(windows.len() - 1)`

## Window Closing

`WindowManager::close(id: WindowId)`

1. Remove window at `id.0` from vec
2. Set focused to None if no windows remain

`WindowManager::close_by_pid(pid: u64)`

1. Find window with matching PID, remove it

## Focus

- `focus(id)` — set all windows' `focused = false`, then set target to `true`
- `bring_to_front(id)` — remove from vec, push to end, set focused on new last element
- `active()` — return `Some(WindowId(focused))` or `None`
- `focused_mut()` — mutable reference to focused window

## State Transitions

```
Normal ⇄ Maximized     (toggle_maximize)
Normal ⇄ Fullscreen    (toggle_fullscreen)
Normal → Minimized     (minimize)
Minimized → Normal     (restore)
```

- `minimize(id)` — set `state = Minimized`, save previous state in `prev_state`
- `toggle_maximize(id, sw, th)` — toggle between `Maximized` and `Normal`, animate to full width/taskbar height
- `toggle_fullscreen(id, sw, sh)` — toggle between `Fullscreen` and `Normal`, animate to full screen
- `restore(id)` — restore to `prev_state`

## Drag Support

- `begin_drag(id, mx, my)` — set `dragging = true`, store offset from mouse to window origin
- `update_drag(mx, my)` — update window position to `(mx - drag_ox, my - drag_oy)`
- `end_drag()` — set `dragging = false`, clear dragging index

## Resize Support

Handled in `Desktop::handle_drag()` (not in WindowManager directly):
- `ResizeMargin = 4px` from each window edge
- Edge detection via `hit_window_edge()` returns bitmask (1=left, 2=right, 4=bottom)
- Minimum window size: 100×80px

## Tiling Modes

Tiling is handled in `Desktop` (not in `WindowManager`):

### Floating (default)
- Free positioning, no constraints

### Tile
- Vertical split: master window takes 60% width left, remaining windows share right stack
- Applied by `Desktop::apply_tile()`

### Monocle
- All windows maximized to fill screen (overlapping)
- Applied by `Desktop::apply_monocle()`

Before tiling, current geometries are saved to `prev_tiling_geos`. Floating mode restores them.

## Snap Regions

Activated on drag release when window is near screen edges (within `SNAP_MARGIN = 15px`):

| Region | Position |
|--------|----------|
| Left | Left half |
| Right | Right half |
| Top | Top half |
| Bottom | Bottom half |
| TopLeft | Top-left quadrant |
| TopRight | Top-right quadrant |
| BottomLeft | Bottom-left quadrant |
| BottomRight | Bottom-right quadrant |

## Layout Algorithms

### Tile Layout
```
n = number of windows
if n == 1: window fills taskbar area
else:
  master_w = screen_w * 6 / 10  (60%)
  stack_w = screen_w - master_w
  stack_h = taskbar_h / (n - 1)
  window[0] → master area (left 60%, full height)
  window[i>0] → stack area (right 40%, each 1/(n-1) height)
```

### Window Button Layout (in taskbar)
```
start_button:  5px from left, 58px wide
window_buttons: starting at 75px, each 125px apart, 120px wide
```

## Key Constants

```rust
const TITLE_H: i32 = 22;
const BTN_TOP: i32 = 3;
const BTN_BOT: i32 = 19;
const CLOSE_R: i32 = 4;      // close button right margin
const CLOSE_L: i32 = 24;     // close button left from right edge
const MAX_R: i32 = 58;       // maximize button right margin
const MAX_L: i32 = 80;       // maximize button left from right edge
const MIN_R: i32 = 28;       // minimize button right margin
const MIN_L: i32 = 48;       // minimize button left from right edge
const RESIZE_MARGIN: i32 = 4;
const SNAP_MARGIN: i32 = 15;
const MIN_WIN_W: u32 = 100;
const MIN_WIN_H: u32 = 80;
```
