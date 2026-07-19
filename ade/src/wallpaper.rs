//! Wallpaper — desktop background gradient.

use libsarga::gui::Window;
use crate::render::snapshot::RenderSnapshot;

pub(crate) fn draw(win: &mut Window, snap: &RenderSnapshot) {
    win.draw_gradient_rect(0, 0, snap.screen_w, snap.screen_h, 0xFF1A1A2E, 0xFF0F0F1A, true);
    win.draw_rounded_rect(snap.screen_w / 2, snap.screen_h / 4, 300, 300, 150, 0x103D5AFE);
    win.draw_rounded_rect(snap.screen_w / 4, snap.screen_h / 2, 200, 200, 100, 0x103D5AFE);
}