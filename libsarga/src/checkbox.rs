use crate::gui::Window;
use crate::theme::Theme;
use crate::widget::Widget;
use crate::alloc::string::String;

pub struct CheckBox {
    x: i32,
    y: i32,
    label: String,
    checked: bool,
    size: u32,
}

impl CheckBox {
    pub fn new(x: i32, y: i32, label: &str) -> Self {
        CheckBox { x, y, label: String::from(label), checked: false, size: 16 }
    }

    pub fn checked(&self) -> bool { self.checked }
    pub fn set_checked(&mut self, checked: bool) { self.checked = checked; }
}

impl Widget for CheckBox {
    fn render(&self, win: &mut Window, theme: &Theme) {
        let box_y = self.y + 2;

        // Checkbox square
        let bg = if self.checked { theme.accent } else { theme.bg_elevated };
        win.draw_rect(self.x as u32, box_y as u32, self.size, self.size, bg);
        win.draw_line_h(self.x as u32, box_y as u32, self.size, theme.border);
        win.draw_line_h(self.x as u32, box_y as u32 + self.size - 1, self.size, theme.border);
        win.draw_line_v(self.x as u32, box_y as u32, self.size, theme.border);
        win.draw_line_v(self.x as u32 + self.size - 1, box_y as u32, self.size, theme.border);

        // Checkmark
        if self.checked {
            let cx = self.x as u32 + 3;
            let cy = box_y as u32 + self.size / 2;
            // Simple checkmark using lines
            win.draw_line_h(cx + 2, cy, 4, 0xFFFFFFFF);
            win.draw_line_h(cx, cy - 1, 3, 0xFFFFFFFF);
            win.draw_line_h(cx, cy + 1, 3, 0xFFFFFFFF);
        }

        // Label
        win.draw_string(self.x as u32 + self.size + 6, box_y as u32, &self.label, theme.text, 0);
    }

    fn handle_click(&mut self, x: i32, y: i32, _pressed: bool) -> bool {
        if self.contains(x, y) {
            self.checked = !self.checked;
            return true;
        }
        false
    }

    fn bounds(&self) -> (i32, i32, u32, u32) {
        let label_w = self.label.len() as u32 * 8;
        (self.x, self.y, self.size + 6 + label_w, self.size + 4)
    }

    fn set_position(&mut self, x: i32, y: i32) { self.x = x; self.y = y; }
    fn set_size(&mut self, _w: u32, _h: u32) {}
}
