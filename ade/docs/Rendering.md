# Render Pipeline

## Compositor Layers

Six layers composited in fixed order, defined in `render/layer.rs`:

| Layer | Index | Content | Alpha |
|-------|-------|---------|-------|
| Wallpaper | 0 | Solid color or wallpaper pattern | Opaque |
| Desktop | 1 | Desktop icons, rubber-band selection rect | Opaque |
| Windows | 2 | AppWindow decorations + content | Per-window opacity |
| Popups | 3 | Taskbar, Start menu | Opaque |
| Overlay | 4 | Context menus, notifications, settings panels, tooltips | Alpha |
| Cursor | 5 | (not explicitly drawn — cursor comes from window system) | Opaque |

## Layer Compositing Order

```
Compositor::compose()
  1. Copy Wallpaper layer → window buffer (full copy_from_slice)
  2. For each subsequent layer:
     - Fully transparent pixel (alpha == 0): skip
     - Fully opaque pixel (alpha == 255): overwrite
     - Semi-transparent: alpha_blend(accumulated, src_pixel, alpha)
```

Alpha blending formula:
```
a = src_alpha / 255
inv_a = 1 - a
result = (fg_r * a + bg_r * inv_a, fg_g * a + bg_g * inv_a, fg_b * a + bg_b * inv_a)
```

## Canvas API

`Canvas<'a>` wraps a `&mut [u32]` pixel buffer with dimensions `(w, h)`.

| Method | Description |
|--------|-------------|
| `fill_rect(x, y, w, h, color)` | Fill rectangle with solid color |
| `fill_pixel(x, y, color)` | Set single pixel |
| `draw_rect_alpha(x, y, w, h, color)` | Alpha-blended rectangle |
| `draw_rect_outline(x, y, w, h, color)` | 1px outline |
| `draw_rounded_rect(x, y, w, h, radius, color)` | Rounded fill |
| `draw_rounded_rect_outline(...)` | Rounded outline (Bresenham circle corners) |
| `draw_rect(x, y, w, h, color)` | Alias for fill_rect |
| `draw_gradient_rect(x, y, w, h, c1, c2, vert)` | Vertical/horizontal gradient |
| `draw_line_h(x, y, len, color)` | Horizontal line |
| `draw_line_v(x, y, len, color)` | Vertical line |
| `draw_char(x, y, c, fg, bg)` | 8×8 bitmap glyph |
| `draw_string(x, y, s, fg, bg)` | String of glyphs |
| `draw_string_shadow(...)` | Unused (available) |

All methods clamp to `(w, h)` — out-of-bounds writes are silently discarded.

## Font Rendering

Internal 8×8 bitmap font embedded in `compositor.rs` as a `[[u8; 8]; 95]` array (CP437 glyphs, ASCII 32–126). Characters outside the printable range render as blank (all-zero glyph).

## Dirty Region Tracking

`DamageTracker` in `damage.rs`:

```rust
pub(crate) struct DamageTracker {
    pub full: bool,
    pub dirty_rect: Option<(u32, u32, u32, u32)>,
}
```

- `mark_full()` — invalidate entire screen next frame
- `is_dirty()` — returns true if any damage recorded
- `clear()` — reset after frame render

The render loop in `main.rs`:
```rust
if desktop.damage.is_dirty() {
    let snap = desktop.snapshot();
    render::render(&mut desktop_win, &snap, &clock_str, &mut compositor);
    desktop.damage.clear();
}
```

Currently only full-screen invalidation is used — dirty rectangles are tracked but not yet consumed by the compositor.

## Window Drawing

`window::draw()` renders each `AppWindow`:

1. **Shadow** — 6px offset semi-transparent rect (0x60000000)
2. **Window body** — rounded rect with theme `bg_surface`
3. **Border** — rounded rect outline with focus/border color
4. **Title bar** — gradient rect (accent → accent_dark when focused)
5. **Title text** — white text at x+12, y+7
6. **Always-on-top badge** — `[A]` in orange if `always_on_top`
7. **Close button** — red rounded rect with "x"
8. **Minimize button** — elevated rect with horizontal line
9. **Window content** — text lines at 14px line height
10. **Text cursor** — blinking `_` at insertion point

Title bar metrics (from `desktop.rs` constants):
- Title height: 22px
- Button top/bottom: 3px / 19px
- Close button: right edge at window right - 4px, width 20px
- Max button: right edge at window right - 58px, width 22px
- Min button: right edge at window right - 28px, width 20px

Selection and scroll support:
- `selection: Option<Selection>` — tracks start/end coordinates for text selection (unused in alpha)
- `scroll: u32` — vertical scroll offset for content overflow
