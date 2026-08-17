//! Wallpaper — desktop background gradient.

use crate::render::compositor::Canvas;
use crate::render::snapshot::RenderSnapshot;

pub(crate) fn draw(canvas: &mut Canvas, snap: &RenderSnapshot) {
    let accent_soft = (snap.theme.accent & 0x00FF_FFFF) | 0x10_000000;
    canvas.draw_gradient_rect(
        0,
        0,
        snap.screen_w,
        snap.screen_h,
        snap.theme.bg_surface,
        snap.theme.bg_primary,
        true,
    );
    canvas.draw_rounded_rect(
        snap.screen_w / 2,
        snap.screen_h / 4,
        300,
        300,
        150,
        accent_soft,
    );
    canvas.draw_rounded_rect(
        snap.screen_w / 4,
        snap.screen_h / 2,
        200,
        200,
        100,
        accent_soft,
    );
}
