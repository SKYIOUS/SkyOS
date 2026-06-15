use crate::gui::Window;
use crate::theme::Theme;
use crate::widget::Widget;
use crate::alloc::string::String;

pub struct TextBox {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    text: String,
    cursor_pos: usize,
    focused: bool,
    placeholder: String,
}

impl TextBox {
    pub fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        TextBox {
            x, y, width, height,
            text: String::new(),
            cursor_pos: 0,
            focused: false,
            placeholder: String::new(),
        }
    }

    pub fn with_placeholder(mut self, placeholder: &str) -> Self {
        self.placeholder = String::from(placeholder);
        self
    }

    pub fn text(&self) -> &str { &self.text }
    pub fn set_text(&mut self, text: &str) { self.text = String::from(text); self.cursor_pos = self.text.len(); }
}

impl Widget for TextBox {
    fn render(&self, win: &mut Window, theme: &Theme) {
        // Background
        win.draw_rounded_rect(self.x as u32, self.y as u32, self.width, self.height, 4, theme.bg_elevated);

        // Border
        let border_color = if self.focused { theme.accent } else { theme.border };
        win.draw_line_h(self.x as u32, self.y as u32, self.width, border_color);
        win.draw_line_h(self.x as u32, self.y as u32 + self.height - 1, self.width, border_color);
        win.draw_line_v(self.x as u32, self.y as u32, self.height, border_color);
        win.draw_line_v(self.x as u32 + self.width - 1, self.y as u32, self.height, border_color);

        // Text or placeholder
        let text_x = self.x as u32 + 8;
        let text_y = self.y as u32 + (self.height - 16) / 2;
        if self.text.is_empty() {
            win.draw_string(text_x, text_y, &self.placeholder, theme.text_disabled, 0);
        } else {
            win.draw_string(text_x, text_y, &self.text, theme.text, 0);
        }

        // Cursor
        if self.focused {
            let cursor_x = text_x + self.cursor_pos as u32 * 8;
            win.draw_line_v(cursor_x, self.y as u32 + 4, self.height - 8, theme.accent);
        }
    }

    fn handle_click(&mut self, x: i32, y: i32, _pressed: bool) -> bool {
        self.focused = self.contains(x, y);
        self.focused
    }

    fn handle_key(&mut self, key: u8) -> bool {
        if !self.focused { return false; }
        match key {
            0x08 => { // Backspace
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                    self.text.remove(self.cursor_pos);
                }
                true
            }
            0x0D => true, // Enter — consume but don't insert
            0x1B => true, // Escape
            0x20..=0x7E => { // Printable ASCII
                self.text.insert(self.cursor_pos, key as char);
                self.cursor_pos += 1;
                true
            }
            _ => false,
        }
    }

    fn bounds(&self) -> (i32, i32, u32, u32) {
        (self.x, self.y, self.width, self.height)
    }

    fn set_position(&mut self, x: i32, y: i32) { self.x = x; self.y = y; }
    fn set_size(&mut self, w: u32, h: u32) { self.width = w; self.height = h; }
    fn is_focused(&self) -> bool { self.focused }
    fn set_focus(&mut self, focused: bool) { self.focused = focused; }
}
