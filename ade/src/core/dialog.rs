//! Shared dialog helpers — backdrop overlay + rounded panel.

use crate::render::compositor::Canvas;

/// Draw a semi-transparent backdrop covering the full compositor dimensions
pub fn draw_backdrop(canvas: &mut Canvas, screen_w: u32, screen_h: u32) {
    canvas.draw_rect_alpha(0, 0, screen_w, screen_h, 0x80000000);
}

/// Draw a centered rounded panel with title
pub fn draw_panel(canvas: &mut Canvas, px: u32, py: u32, pw: u32, ph: u32, title: &str) {
    canvas.draw_rounded_rect(px, py, pw, ph, 8, 0xFF1E1E2E);
    canvas.draw_rounded_rect_outline(px, py, pw, ph, 8, 0xFF3A3A5C);
    canvas.draw_string(px + 10, py + 8, title, 0xFFFFFFFF, 0);
}
