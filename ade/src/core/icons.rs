//! Desktop icons — app shortcuts drawn on wallpaper.

use libsarga::gui::Window;
use libsarga::theme::Theme;

#[allow(dead_code)]
pub(crate) fn draw(win: &mut Window, theme: &Theme, name: &str, x: u32, y: u32) {
    win.draw_rounded_rect(x, y, 40, 40, 6, theme.bg_elevated);
    let letter = name.as_bytes()[0] as char;
    win.draw_char(x + 15, y + 12, letter, theme.accent, theme.bg_elevated);
    let tw = name.len() as u32 * 8;
    win.draw_string(x + 20 - tw / 2, y + 44, name, theme.text, 0);
}
