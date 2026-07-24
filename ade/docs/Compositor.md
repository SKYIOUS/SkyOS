# Compositor

## Architecture

The compositor in `render/compositor.rs` manages six screen-sized pixel buffers and composites them onto the window framebuffer each frame.

## Layer Buffer Allocation

All six layer buffers are allocated once at compositor creation and reused every frame:

```rust
pub(crate) struct Compositor {
    layers: [LayerBuffer; LAYER_COUNT],  // LAYER_COUNT = 6
    w: u32,
    h: u32,
}
```

Each `LayerBuffer` contains a `Vec<u32>` of `w * h` pixels. For a 1024×768 display, that's 786,432 pixels × 4 bytes = ~3MB per layer, ~18MB total.

## Compositing Algorithm

`Compositor::compose(win)`:

1. Copy wallpaper layer (index 0) directly to framebuffer via `copy_from_slice`
2. For layers 1 through 5:
   - Each pixel tested for alpha:
     - `0xAARRGGBB` with alpha == 0 → skip (fully transparent)
     - alpha == 255 → overwrite (fully opaque)
     - else → `alpha_blend(accumulated, src_pixel, alpha)`

Alpha blend formula:
```
alpha_blend(bg, fg, alpha):
    if alpha == 0: return bg
    if alpha == 255: return fg
    a = alpha
    inv_a = 255 - alpha
    r = (fg_red * a + bg_red * inv_a) / 255
    g = (fg_green * a + bg_green * inv_a) / 255
    b = (fg_blue * a + bg_blue * inv_a) / 255
    return (r << 16) | (g << 8) | b
```

## Canvas Drawing Primitives

`Canvas<'a>` wraps a `&mut [u32]` buffer with `w` and `h` bounds:

| Primitive | Description |
|-----------|-------------|
| `fill_rect` | Solid fill, clamped to bounds |
| `fill_pixel` | Single pixel |
| `draw_rect_alpha` | Alpha-blended rectangle |
| `draw_rect_outline` | 1-pixel border |
| `draw_rounded_rect` | Rounded fill (Bresenham corners) |
| `draw_rounded_rect_outline` | Rounded outline |
| `draw_gradient_rect` | Horizontal/vertical gradient |
| `draw_line_h` / `draw_line_v` | Axis-aligned lines |
| `draw_char` | 8×8 bitmap glyph |
| `draw_string` | Glyph string at 8px/char |

## Font Rendering

Embedded 8×8 bitmap font (CP437, characters 32–126) stored as a `[[u8; 8]; 95]` constant. Each glyph is 8 bytes, one byte per row, MSB = leftmost pixel.

Character `' '` (space) at index 0 is all zeros. Characters outside ASCII 32–126 render as space.

## Layer Clear

`clear_all()` iterates all 6 layer buffers and zeroes every pixel. `clear_layer(layer)` clears a single layer.

## Performance Characteristics

- Full frame recomposite: 1024×768 × 5 blend layers = ~3.9M pixel operations
- No dirty-rect optimization yet — every pixel re-evaluated each frame
- All operations CPU-bound with no GPU acceleration
- Gradients use `f32` arithmetic (acceptable in alpha but could be integer-optimized)
