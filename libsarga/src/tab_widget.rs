use crate::gui::Window;
use crate::theme::Theme;
use crate::widget::Widget;
use alloc::string::String;
use alloc::vec::Vec;

pub struct TabWidget {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    tabs: Vec<(String, bool)>, // (title, closeable)
    active: usize,
}

impl TabWidget {
    pub fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        TabWidget { x, y, width, height, tabs: Vec::new(), active: 0 }
    }

    pub fn add_tab(&mut self, title: &str) {
        self.tabs.push((String::from(title), true));
    }

    pub fn remove_tab(&mut self, index: usize) {
        if index < self.tabs.len() {
            self.tabs.remove(index);
            if self.active >= self.tabs.len() && self.active > 0 {
                self.active -= 1;
            }
        }
    }

    pub fn active_tab(&self) -> usize { self.active }
    pub fn tab_count(&self) -> usize { self.tabs.len() }

    pub fn set_active(&mut self, index: usize) {
        if index < self.tabs.len() { self.active = index; }
    }
}

impl Widget for TabWidget {
    fn render(&self, win: &mut Window, theme: &Theme) {
        let tab_h: u32 = 28;
        let tab_w: u32 = 120;

        // Tab bar background
        win.draw_rect(self.x as u32, self.y as u32, self.width, tab_h, theme.bg_surface);

        for (i, (title, _)) in self.tabs.iter().enumerate() {
            let tx = self.x as u32 + i as u32 * (tab_w + 2);
            let active = i == self.active;

            // Tab background
            let bg = if active { theme.bg_elevated } else { theme.bg_surface };
            win.draw_rect(tx, self.y as u32, tab_w, tab_h, bg);

            // Active indicator
            if active {
                win.draw_line_h(tx, self.y as u32 + tab_h - 2, tab_w, theme.accent);
            }

            // Tab title (truncated)
            let display_title = if title.len() > 12 { &title[..12] } else { title };
            let text_color = if active { theme.text } else { theme.text_secondary };
            win.draw_string(tx + 8, self.y as u32 + 6, display_title, text_color, 0);

            // Close button (X)
            if self.tabs[i].1 {
                let cx = tx + tab_w - 16;
                let cy = self.y as u32 + 8;
                win.draw_string(cx, cy, "x", theme.text_disabled, 0);
            }
        }

        // Content area border
        let content_y = self.y as u32 + tab_h;
        win.draw_line_h(self.x as u32, content_y, self.width, theme.border);
    }

    fn handle_click(&mut self, x: i32, y: i32, _pressed: bool) -> bool {
        let tab_h = 28i32;
        let tab_w = 120i32;
        if y >= self.y && y < self.y + tab_h {
            let idx = ((x - self.x) / (tab_w + 2)) as usize;
            if idx < self.tabs.len() {
                self.active = idx;
                return true;
            }
        }
        false
    }

    fn bounds(&self) -> (i32, i32, u32, u32) { (self.x, self.y, self.width, self.height) }
    fn set_position(&mut self, x: i32, y: i32) { self.x = x; self.y = y; }
    fn set_size(&mut self, w: u32, h: u32) { self.width = w; self.height = h; }
}
