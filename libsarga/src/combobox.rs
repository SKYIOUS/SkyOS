use crate::gui::Window;
use crate::theme::Theme;
use crate::widget::Widget;
use alloc::string::String;
use alloc::vec::Vec;

pub struct ComboBox {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    items: Vec<String>,
    selected: Option<usize>,
    open: bool,
    focused: bool,
}

impl ComboBox {
    pub fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        ComboBox { x, y, width, height, items: Vec::new(), selected: None, open: false, focused: false }
    }

    pub fn add_item(&mut self, item: &str) { self.items.push(String::from(item)); }
    pub fn selected(&self) -> Option<usize> { self.selected }
    pub fn selected_text(&self) -> Option<&str> {
        self.selected.and_then(|i| self.items.get(i).map(|s| s.as_str()))
    }
    pub fn set_selected(&mut self, index: usize) { if index < self.items.len() { self.selected = Some(index); } }
}

impl Widget for ComboBox {
    fn render(&self, win: &mut Window, theme: &Theme) {
        // Main box
        let bg = if self.focused { theme.bg_elevated } else { theme.bg_surface };
        win.draw_rounded_rect(self.x as u32, self.y as u32, self.width, self.height, 4, bg);
        let border = if self.focused { theme.accent } else { theme.border };
        win.draw_line_h(self.x as u32, self.y as u32, self.width, border);
        win.draw_line_h(self.x as u32, self.y as u32 + self.height - 1, self.width, border);
        win.draw_line_v(self.x as u32, self.y as u32, self.height, border);
        win.draw_line_v(self.x as u32 + self.width - 1, self.y as u32, self.height, border);

        // Selected text
        let text = self.selected_text().unwrap_or("");
        win.draw_string(self.x as u32 + 8, self.y as u32 + (self.height - 14) / 2, text, theme.text, 0);

        // Arrow
        let ax = self.x as u32 + self.width - 16;
        let ay = self.y as u32 + self.height / 2;
        win.draw_string(ax, ay - 4, "v", theme.text_secondary, 0);

        // Dropdown
        if self.open {
            let item_h: u32 = 24;
            let dy = self.y as u32 + self.height;
            win.draw_rect(self.x as u32, dy, self.width, item_h * self.items.len() as u32, theme.bg_elevated);
            win.draw_line_h(self.x as u32, dy, self.width, theme.border);

            for (i, item) in self.items.iter().enumerate() {
                let iy = dy + i as u32 * item_h;
                let is_selected = self.selected == Some(i);
                let item_bg = if is_selected { theme.accent } else { theme.bg_elevated };
                win.draw_rect(self.x as u32, iy, self.width, item_h, item_bg);
                win.draw_string(self.x as u32 + 8, iy + 4, item, theme.text, 0);
            }
        }
    }

    fn handle_click(&mut self, x: i32, y: i32, _pressed: bool) -> bool {
        // Click on main box
        if y >= self.y && y < self.y + self.height as i32 && x >= self.x && x < self.x + self.width as i32 {
            self.open = !self.open;
            self.focused = true;
            return true;
        }

        // Click on dropdown
        if self.open {
            let item_h = 24i32;
            let dy = self.y + self.height as i32;
            if x >= self.x && x < self.x + self.width as i32 && y >= dy {
                let idx = ((y - dy) / item_h) as usize;
                if idx < self.items.len() {
                    self.selected = Some(idx);
                    self.open = false;
                    return true;
                }
            }
            self.open = false;
        }
        self.focused = false;
        false
    }

    fn bounds(&self) -> (i32, i32, u32, u32) { (self.x, self.y, self.width, self.height) }
    fn set_position(&mut self, x: i32, y: i32) { self.x = x; self.y = y; }
    fn set_size(&mut self, w: u32, h: u32) { self.width = w; self.height = h; }
    fn is_focused(&self) -> bool { self.focused }
    fn set_focus(&mut self, focused: bool) { self.focused = focused; }
}
