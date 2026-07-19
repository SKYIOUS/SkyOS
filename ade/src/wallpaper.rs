use libsarga::gui::Window;
use crate::desktop::Desktop;

pub(crate) fn draw(win: &mut Window, desktop: &Desktop) {
    // Draw a nice gradient wallpaper
    win.draw_gradient_rect(
        0,
        0,
        desktop.screen_w,
        desktop.screen_h,
        0xFF1A1A2E,
        0xFF0F0F1A,
        true,
    );
    // Add some "abstract" shapes
    win.draw_rounded_rect(
        desktop.screen_w / 2,
        desktop.screen_h / 4,
        300,
        300,
        150,
        0x103D5AFE,
    );
    win.draw_rounded_rect(
        desktop.screen_w / 4,
        desktop.screen_h / 2,
        200,
        200,
        100,
        0x103D5AFE,
    );
}