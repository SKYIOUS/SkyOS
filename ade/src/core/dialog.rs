//! Shared dialog helpers — backdrop overlay + rounded panel.

use crate::render::compositor::Canvas;

/// Draw a semi-transparent backdrop covering the full compositor dimensions
pub fn draw_backdrop(
    canvas: &mut Canvas,
    screen_w: u32,
    screen_h: u32,
    theme: &libsarga::theme::Theme,
) {
    canvas.draw_rect_alpha(0, 0, screen_w, screen_h, theme.shadow);
}

/// Draw a centered rounded panel with title
pub fn draw_panel(
    canvas: &mut Canvas,
    px: u32,
    py: u32,
    pw: u32,
    ph: u32,
    title: &str,
    theme: &libsarga::theme::Theme,
) {
    canvas.draw_rounded_rect(px, py, pw, ph, 8, theme.bg_surface);
    canvas.draw_rounded_rect_outline(px, py, pw, ph, 8, theme.border);
    canvas.draw_string(px + 10, py + 8, title, theme.text, 0);
}
