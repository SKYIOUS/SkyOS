# Performance Review

## Targets

- 60 FPS idle ✓ (frame < 16ms)
- 0 allocs per frame ✓ (Vec reuse)
- Window launch < 100ms
- Focus switch < 2ms

## Bottlenecks

1. **Software compositor**: full-frame blit + per-pixel alpha blending on every dirty region. For 1024×768 × 5 layers = ~3.9M pixel operations each frame.
2. **Full-screen damage**: `damage.mark_full()` called aggressively (on every click, every frame cursor blink, etc.). No dirty-rect tracking used by compositor yet.
3. **Gradient rendering**: uses `f32` interpolation per pixel. Minor cost but avoidable.
4. **Notification render**: `visible_notifications()` iterates all 64 slots, even when few are active.
5. **Clipboard history**: `copy()` does retain + push which re-scans all entries. Small but unnecessary on every copy.
6. **String allocation**: window creation allocates title and content strings. Frequent window create/close causes allocation churn.
7. **A11y tree rebuild**: `build_a11y_tree()` clears and rebuilds the full tree every tick.

## Per-Frame Allocation Analysis

| Code Path | Allocations | Notes |
|-----------|-------------|-------|
| `Desktop::tick()` | 0 | No Vec operations |
| `handle_click()` | 0 | No alloc (snapshot is borrow) |
| `handle_drag()` | 0 | No alloc |
| `snapshot()` | 0 | Borrows only |
| `render()` | 0 | Compositor uses pre-allocated buffers |
| `compose()` | 0 | In-place pixel ops |
| `services.tick()` | 0 | No Vec ops in steady state |

## Recommendations

1. **Dirty-rect tracking**: Track per-rectangle dirty regions per layer. Only composite affected regions. (post-alpha)
2. **Notification compaction**: Skip empty slots in render loop. Track `visible_count` directly.
3. **A11y tree**: Incremental update instead of full rebuild (or reduce rebuild frequency).
4. **Cursor blink**: Use timer-based redraw rather than full damage every tick.
5. **Allocation pools**: Pre-allocate common string sizes for window titles.
6. **Remove float gradients**: Replace `f32` interpolation in `draw_gradient_rect` with fixed-point arithmetic.
