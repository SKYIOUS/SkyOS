//! Task manager — process list overlay with focus/close actions.

use crate::render::compositor::Canvas;
use crate::render::snapshot::RenderSnapshot;
use crate::core::window::AppWindow;

pub(crate) struct TaskManagerState {
    pub open: bool,
    pub selected: usize,
    pub scroll: u32,
}

impl TaskManagerState {
    pub fn new() -> Self {
        TaskManagerState {
            open: false,
            selected: 0,
            scroll: 0,
        }
    }

    pub fn draw(&self, canvas: &mut Canvas, windows: &[AppWindow], _theme: &libsarga::theme::Theme) {
        if !self.open {
            return;
        }
        let pw = 560u32;
        let ph = 360u32;
        let px = (canvas.w - pw) / 2;
        let py = (canvas.h - ph) / 3;
        crate::core::dialog::draw_backdrop(canvas, canvas.w, canvas.h);
        crate::core::dialog::draw_panel(canvas, px, py, pw, ph, "Task Manager");

        // Column headers
        let header_y = py + 32;
        canvas.draw_rect(px + 4, header_y, pw - 8, 20, 0xFF2D2D40);
        canvas.draw_string(px + 10, header_y + 4, "PID", 0xFF888888, 0);
        canvas.draw_string(px + 60, header_y + 4, "Name", 0xFF888888, 0);
        canvas.draw_string(px + 220, header_y + 4, "State", 0xFF888888, 0);
        canvas.draw_string(px + 310, header_y + 4, "Memory", 0xFF888888, 0);
        canvas.draw_string(px + 400, header_y + 4, "CPU", 0xFF888888, 0);

        // Process list
        let item_h = 20u32;
        let list_y = header_y + 22;
        let max_visible = (ph.saturating_sub(list_y - py + 4) / item_h) as usize;
        let count = max_visible.min(windows.len());
        for i in 0..count {
            let iy = list_y + i as u32 * item_h;
            let sel = i == self.selected;
            let bg = if sel {
                0xFF3D5AFE
            } else if i % 2 == 0 {
                0xFF22223A
            } else {
                0xFF1E1E2E
            };
            canvas.draw_rect(px + 4, iy, pw - 8, item_h, bg);
            let w = &windows[i];
            let pid_str = match w.pid {
                Some(p) => alloc::format!("{}", p),
                None => alloc::string::String::new(),
            };
            let state_str = match w.state {
                crate::core::window::WindowState::Normal => "Running",
                crate::core::window::WindowState::Minimized => "Minimized",
                crate::core::window::WindowState::Maximized => "Maximized",
                crate::core::window::WindowState::Fullscreen => "Fullscreen",
            };
            let fg = if sel { 0xFFFFFFFF } else { 0xFFD0D0D0 };
            canvas.draw_string(px + 10, iy + 4, &pid_str, fg, 0);
            let title = if w.title.len() > 22 {
                &w.title[..22]
            } else {
                &w.title
            };
            canvas.draw_string(px + 60, iy + 4, title, fg, 0);
            canvas.draw_string(px + 220, iy + 4, state_str, fg, 0);
            canvas.draw_string(px + 310, iy + 4, "\u{2014}", fg, 0);
            canvas.draw_string(px + 400, iy + 4, "\u{2014}", fg, 0);
        }
    }

    pub fn hit_test(&self, mx: i32, my: i32, snap: &RenderSnapshot) -> Option<(usize, &'static str)> {
        if !self.open {
            return None;
        }
        let pw = 560u32;
        let ph = 360u32;
        let px = (snap.screen_w - pw) / 2;
        let py = (snap.screen_h - ph) / 3;
        let header_y = py + 32;
        let list_y = header_y + 22;
        let item_h = 20u32;
        let rel = (my as u32).saturating_sub(list_y);
        let idx = (rel / item_h) as usize;
        if idx < snap.windows.len()
            && mx >= px as i32 + 4
            && mx <= (px + pw - 4) as i32
            && (my as u32) >= list_y
            && (my as u32) < list_y + snap.windows.len() as u32 * item_h
        {
            return Some((idx, "focus"));
        }
        None
    }
}
