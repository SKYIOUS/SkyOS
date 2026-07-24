//! File manager — directory listing, navigation history, selection.

use alloc::string::String;
use alloc::vec::Vec;
use libsarga::theme::Theme;
use crate::render::compositor::Canvas;
use crate::core::window::AppWindow;

pub(crate) struct FileManagerState {
    pub path: String,
    pub entries: Vec<String>,
    pub selected: usize,
    pub history: Vec<String>,
    pub history_pos: usize,
}

impl FileManagerState {
    pub fn new() -> Self {
        FileManagerState {
            path: String::from("/home"),
            entries: Vec::new(),
            selected: 0,
            history: Vec::new(),
            history_pos: 0,
        }
    }

    pub fn refresh(&mut self) {
        self.entries.clear();
        let raw = crate::sys::vfs::VfsContext::list_dir_static(&self.path);
        for entry in &raw {
            if entry.is_dir {
                self.entries.push(alloc::format!("[{}]", entry.name));
            } else {
                self.entries.push(entry.name.clone());
            }
        }
    }

    pub fn navigate(&mut self, path: &str) {
        self.history.truncate(self.history_pos + 1);
        self.history.push(self.path.clone());
        self.history_pos = self.history.len() - 1;
        self.path = String::from(path);
        self.selected = 0;
        self.refresh();
    }

    pub fn navigate_up(&mut self) {
        let parent = match self.path.rfind('/') {
            Some(0) => String::from("/"),
            Some(pos) => String::from(&self.path[..pos]),
            None => String::from("/"),
        };
        self.navigate(&parent);
    }

    pub fn go_back(&mut self) {
        if self.history_pos > 0 {
            self.history_pos -= 1;
            self.path = self.history[self.history_pos].clone();
            self.selected = 0;
            self.refresh();
        }
    }

    pub fn go_forward(&mut self) {
        if self.history_pos + 1 < self.history.len() {
            self.history_pos += 1;
            self.path = self.history[self.history_pos].clone();
            self.selected = 0;
            self.refresh();
        }
    }

    pub fn draw(&self, canvas: &mut Canvas, aw: &AppWindow, _theme: &Theme) {
        let path_y = aw.y as u32 + 22;
        canvas.draw_rect(aw.x as u32, path_y, aw.w, 24, 0xFF2D2D40);
        let display = if self.path.len() > 60 {
            &self.path[self.path.len() - 60..]
        } else {
            &self.path
        };
        canvas.draw_string(aw.x as u32 + 4, path_y + 6, display, 0xFFD0D0D0, 0);

        let list_y = path_y + 26;
        let item_h = 20u32;
        let avail = aw.h.saturating_sub(list_y - aw.y as u32 + 4) / item_h;
        for i in 0..avail.min(self.entries.len() as u32) {
            let iy = list_y + i * item_h;
            let idx = i as usize;
            let sel = idx == self.selected;
            let bg = if sel {
                0xFF3D5AFE
            } else if idx % 2 == 0 {
                0xFF1E1E2E
            } else {
                0xFF22223A
            };
            canvas.draw_rect(aw.x as u32, iy, aw.w, item_h, bg);
            let label = &self.entries[idx];
            let display = if label.len() > 50 { &label[..50] } else { label };
            canvas.draw_string(
                aw.x as u32 + 4,
                iy + 4,
                display,
                if sel { 0xFFFFFFFF } else { 0xFFD0D0D0 },
                0,
            );
        }
    }
}
