use crate::gui::Window;
use crate::theme::Theme;
use crate::widget::Widget;
use alloc::string::String;
use alloc::vec::Vec;

pub struct Dialog {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    title: String,
    message: String,
    buttons: Vec<String>,
    pub result: Option<usize>,
    pub open: bool,
}

impl Dialog {
    pub fn alert(title: &str, message: &str) -> Self {
        Dialog {
            x: 100, y: 100, width: 400, height: 200,
            title: String::from(title),
            message: String::from(message),
            buttons: alloc::vec![String::from("OK")],
            result: None, open: true,
        }
    }

    pub fn confirm(title: &str, message: &str) -> Self {
        Dialog {
            x: 100, y: 100, width: 400, height: 200,
            title: String::from(title),
            message: String::from(message),
            buttons: alloc::vec![String::from("OK"), String::from("Cancel")],
            result: None, open: true,
        }
    }

    pub fn set_position(&mut self, x: i32, y: i32) { self.x = x; self.y = y; }
}

impl Widget for Dialog {
    fn render(&self, win: &mut Window, theme: &Theme) {
        if !self.open { return; }

        // Overlay (semi-transparent)
        win.draw_rect(0, 0, win.width, win.height, 0x80000000);

        // Dialog box
        win.draw_rounded_rect(self.x as u32, self.y as u32, self.width, self.height, 8, theme.bg_elevated);

        // Title bar
        win.draw_rect(self.x as u32, self.y as u32, self.width, 32, theme.accent);
        win.draw_string(self.x as u32 + 12, self.y as u32 + 8, &self.title, 0xFFFFFFFF, 0);

        // Message
        win.draw_string(self.x as u32 + 16, self.y as u32 + 48, &self.message, theme.text, 0);

        // Buttons
        let btn_h: u32 = 28;
        let btn_w: u32 = 80;
        let spacing: u32 = 8;
        let total_btn_w = self.buttons.len() as u32 * btn_w + (self.buttons.len() as u32 - 1) * spacing;
        let btn_x = self.x as u32 + (self.width - total_btn_w) / 2;
        let btn_y = self.y as u32 + self.height - btn_h - 16;

        for (i, label) in self.buttons.iter().enumerate() {
            let bx = btn_x + i as u32 * (btn_w + spacing);
            let color = if i == 0 { theme.accent } else { theme.bg_surface };
            win.draw_rounded_rect(bx, btn_y, btn_w, btn_h, 4, color);
            let tw = label.len() as u32 * 8;
            win.draw_string(bx + (btn_w - tw) / 2, btn_y + 6, label, 0xFFFFFFFF, 0);
        }
    }

    fn handle_click(&mut self, x: i32, y: i32, _pressed: bool) -> bool {
        if !self.open { return false; }

        let btn_h = 28i32;
        let btn_w = 80i32;
        let spacing = 8i32;
        let total_btn_w = self.buttons.len() as i32 * btn_w + (self.buttons.len() as i32 - 1) * spacing;
        let btn_x = self.x + (self.width as i32 - total_btn_w) / 2;
        let btn_y = self.y + self.height as i32 - btn_h - 16;

        if y >= btn_y && y < btn_y + btn_h {
            for (i, _) in self.buttons.iter().enumerate() {
                let bx = btn_x + i as i32 * (btn_w + spacing);
                if x >= bx && x < bx + btn_w {
                    self.result = Some(i);
                    self.open = false;
                    return true;
                }
            }
        }
        false
    }

    fn bounds(&self) -> (i32, i32, u32, u32) { (self.x, self.y, self.width, self.height) }
    fn set_position(&mut self, x: i32, y: i32) { self.x = x; self.y = y; }
    fn set_size(&mut self, w: u32, h: u32) { self.width = w; self.height = h; }
}
