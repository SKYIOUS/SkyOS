//! Settings panel — sound, theme, display toggles.

use crate::render::snapshot::RenderSnapshot;
use libsarga::gui::Window;

pub(crate) struct SettingsState {
    pub open: bool,
    pub sound_on: bool,
    pub theme_dark: bool,
    pub hover_idx: i32,
}

impl SettingsState {
    pub fn new() -> Self {
        SettingsState {
            open: false,
            sound_on: true,
            theme_dark: true,
            hover_idx: -1,
        }
    }

    pub fn toggle(&mut self) {
        self.open = !self.open;
        self.hover_idx = -1;
    }

    pub fn draw(&self, win: &mut Window, snap: &RenderSnapshot) {
        if !self.open {
            return;
        }
        let pw = 320u32;
        let ph = 200u32;
        let px = (snap.screen_w - pw) / 2;
        let py = (snap.screen_h - ph) / 3;
        win.draw_rect_alpha(0, 0, snap.screen_w, snap.screen_h, 0x40000000);
        win.draw_rounded_rect(px, py, pw, ph, 8, 0xFF2D2D2D);
        win.draw_rounded_rect_outline(px, py, pw, ph, 8, 0xFF555555);
        win.draw_string(px + 10, py + 6, "Settings", 0xFFFFFFFF, 0);

        let rows = [("Sound", self.sound_on), ("Dark Theme", self.theme_dark)];
        for (i, (label, val)) in rows.iter().enumerate() {
            let iy = py + 36 + i as u32 * 32;
            let hover = self.hover_idx == i as i32;
            let bg = if hover { 0xFF3A3A5C } else { 0xFF2D2D2D };
            win.draw_rounded_rect(px + 8, iy, pw - 16, 28, 4, bg);
            win.draw_string(px + 16, iy + 6, label, 0xFFD0D0D0, 0);
            let toggle_fg = if *val { 0xFF4CAF50 } else { 0xFF555555 };
            win.draw_char(
                px + pw - 40,
                iy + 6,
                if *val { 'Y' } else { 'N' },
                toggle_fg,
                0,
            );
        }

        let close_hover = self.hover_idx == 2;
        let cb = if close_hover { 0xFF5C5C8A } else { 0xFF3D3D5C };
        win.draw_rounded_rect(px + 100, py + ph - 36, 120, 28, 4, cb);
        win.draw_string(px + 140, py + ph - 30, "Close", 0xFFD0D0D0, 0);
    }

    pub fn hit_test(&self, mx: i32, my: i32, snap: &RenderSnapshot) -> Option<usize> {
        if !self.open {
            return None;
        }
        let pw = 320u32;
        let ph = 200u32;
        let px = (snap.screen_w - pw) / 2;
        let py = (snap.screen_h - ph) / 3;
        for i in 0..2 {
            let iy = py + 36 + i as u32 * 32;
            if mx >= px as i32 + 8
                && mx <= (px + pw - 8) as i32
                && my >= iy as i32
                && my <= (iy + 28) as i32
            {
                return Some(i);
            }
        }
        let cy = py + ph - 36;
        if mx >= (px + 100) as i32
            && mx <= (px + 220) as i32
            && my >= cy as i32
            && my <= (cy + 28) as i32
        {
            return Some(2);
        }
        None
    }
}
