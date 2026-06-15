use crate::gui::Window;
use crate::theme::Theme;
use crate::widget::Widget;
use alloc::string::String;
use alloc::vec::Vec;

pub struct MenuBar {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    menus: Vec<(String, Vec<String>)>, // (menu_name, items)
    open_menu: Option<usize>,
}

impl MenuBar {
    pub fn new(x: i32, y: i32, width: u32) -> Self {
        MenuBar { x, y, width, height: 24, menus: Vec::new(), open_menu: None }
    }

    pub fn add_menu(&mut self, name: &str, items: &[&str]) {
        let menu_items: Vec<String> = items.iter().map(|s| String::from(*s)).collect();
        self.menus.push((String::from(name), menu_items));
    }

    pub fn active_menu(&self) -> Option<usize> { self.open_menu }
}

impl Widget for MenuBar {
    fn render(&self, win: &mut Window, theme: &Theme) {
        // Menu bar background
        win.draw_rect(self.x as u32, self.y as u32, self.width, self.height, theme.bg_surface);

        let mut cx = self.x as u32;
        for (i, (name, _)) in self.menus.iter().enumerate() {
            let w = name.len() as u32 * 8 + 16;
            let active = self.open_menu == Some(i);
            let bg = if active { theme.accent } else { theme.bg_surface };
            win.draw_rect(cx, self.y as u32, w, self.height, bg);
            win.draw_string(cx + 8, self.y as u32 + 4, name, theme.text, 0);
            cx += w;
        }

        // Draw open dropdown
        if let Some(menu_idx) = self.open_menu {
            if let Some((_, items)) = self.menus.get(menu_idx) {
                let mut dx = self.x as u32;
                for i in 0..menu_idx {
                    dx += self.menus[i].0.len() as u32 * 8 + 16;
                }
                let item_h: u32 = 24;
                let dy = self.y as u32 + self.height;
                let max_w: u32 = 160;

                // Dropdown background
                win.draw_rect(dx, dy, max_w, item_h * items.len() as u32, theme.bg_elevated);
                win.draw_line_h(dx, dy, max_w, theme.border);
                win.draw_line_v(dx, dy, item_h * items.len() as u32, theme.border);
                win.draw_line_v(dx + max_w - 1, dy, item_h * items.len() as u32, theme.border);

                for (j, item) in items.iter().enumerate() {
                    let iy = dy + j as u32 * item_h;
                    if item == "---" {
                        win.draw_line_h(dx + 4, iy + item_h / 2, max_w - 8, theme.separator);
                    } else {
                        win.draw_string(dx + 8, iy + 4, item, theme.text, 0);
                    }
                }
            }
        }
    }

    fn handle_click(&mut self, x: i32, y: i32, _pressed: bool) -> bool {
        // Check menu bar
        if y >= self.y && y < self.y + self.height as i32 {
            let mut cx = self.x;
            for (i, (name, _)) in self.menus.iter().enumerate() {
                let w = name.len() as i32 * 8 + 16;
                if x >= cx && x < cx + w {
                    self.open_menu = if self.open_menu == Some(i) { None } else { Some(i) };
                    return true;
                }
                cx += w;
            }
        }

        // Check dropdown items
        if let Some(menu_idx) = self.open_menu {
            let mut dx = self.x;
            for i in 0..menu_idx {
                dx += self.menus[i].0.len() as i32 * 8 + 16;
            }
            let item_h = 24i32;
            let dy = self.y + self.height as i32;
            if x >= dx && x < dx + 160 {
                let idx = (y - dy) / item_h;
                if idx >= 0 && idx < self.menus[menu_idx].1.len() as i32 {
                    self.open_menu = None;
                    return true;
                }
            }
            self.open_menu = None;
        }
        false
    }

    fn bounds(&self) -> (i32, i32, u32, u32) { (self.x, self.y, self.width, self.height) }
    fn set_position(&mut self, x: i32, y: i32) { self.x = x; self.y = y; }
    fn set_size(&mut self, w: u32, _h: u32) { self.width = w; }
}
