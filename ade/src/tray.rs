//! System tray — background service indicators in the taskbar.

use libsarga::gui::Window;

pub(crate) struct TrayEntry {
    pub icon: char,
    #[allow(dead_code)]
    pub tooltip: &'static str,
}

pub(crate) struct SystemTray {
    pub entries: &'static [TrayEntry],
}

impl SystemTray {
    pub fn new() -> Self {
        SystemTray {
            entries: &[
                TrayEntry { icon: 'N', tooltip: "Network" },
                TrayEntry { icon: 'S', tooltip: "Sound" },
                TrayEntry { icon: 'B', tooltip: "Battery" },
            ],
        }
    }
}

pub(crate) const TRAY_ICON_W: u32 = 28;
pub(crate) const TRAY_ICON_H: u32 = 28;

#[allow(dead_code)]
pub(crate) fn draw_tray(win: &mut Window, _theme: &libsarga::theme::Theme, tray_x: u32, tray_y: u32, tray: &[TrayEntry]) {
    for (i, entry) in tray.iter().enumerate() {
        let ix = tray_x + i as u32 * TRAY_ICON_W;
        let iy = tray_y;
        win.draw_rounded_rect(ix, iy, TRAY_ICON_W - 2, TRAY_ICON_H, 4, 0xFF2D2D40);
        win.draw_char(ix + 8, iy + 6, entry.icon, 0xFFB0B0B0, 0);
    }
}
