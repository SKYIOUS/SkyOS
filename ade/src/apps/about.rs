//! About SARGA OS — dialog with version and system info.

use crate::render::compositor::Canvas;
use crate::render::snapshot::RenderSnapshot;

pub(crate) struct AboutState {
    pub open: bool,
}

impl AboutState {
    pub fn new() -> Self {
        AboutState { open: false }
    }

    pub fn draw(&self, canvas: &mut Canvas, snap: &RenderSnapshot) {
        if !self.open {
            return;
        }
        let pw = 320u32;
        let ph = 240u32;
        let px = (snap.screen_w - pw) / 2;
        let py = (snap.screen_h - ph) / 3;
        crate::core::dialog::draw_backdrop(canvas, snap.screen_w, snap.screen_h);
        crate::core::dialog::draw_panel(canvas, px, py, pw, ph, "About SARGA OS");

        // Logo area
        let logo_x = px + pw / 2 - 40;
        canvas.draw_rounded_rect(logo_x, py + 32, 80, 32, 6, 0xFF3D5AFE);
        canvas.draw_string(logo_x + 16, py + 38, "SARGA", 0xFFFFFFFF, 0);

        // Info lines
        let mut iy = py + 76;
        let lines = &[
            alloc::format!("SARGA OS v{}", libsarga::version::SKYOS_VERSION),
            alloc::string::String::from("Kernel: SARGA"),
            alloc::string::String::from("Arch: x86_64"),
            alloc::string::String::from("Desktop: ADE"),
            alloc::string::String::new(),
            alloc::string::String::from("Copyright \u{00A9} 2026 SARGA"),
            alloc::string::String::from("A modern OS written in Rust."),
        ];
        for line in lines {
            let cx = px + (pw - line.len() as u32 * 8) / 2;
            canvas.draw_string(cx, iy, line, 0xFFD0D0D0, 0);
            iy += 16;
        }
    }

    pub fn hit_test(&self, mx: i32, my: i32, snap: &RenderSnapshot) -> bool {
        if !self.open {
            return false;
        }
        let pw = 320u32;
        let ph = 240u32;
        let px = (snap.screen_w - pw) / 2;
        let py = (snap.screen_h - ph) / 3;
        mx >= px as i32
            && mx <= (px + pw) as i32
            && my >= py as i32
            && my <= (py + ph) as i32
    }
}
