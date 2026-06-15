use crate::gui::Window;
use crate::theme::Theme;
use crate::widget::Widget;
use crate::alloc::string::String;

pub struct Label {
    x: i32,
    y: i32,
    text: String,
    color: u32,
    size: u32,
}

impl Label {
    pub fn new(x: i32, y: i32, text: &str) -> Self {
        Label { x, y, text: String::from(text), color: 0xFFFFFFFF, size: 14 }
    }

    pub fn with_color(mut self, color: u32) -> Self { self.color = color; self }
    pub fn with_size(mut self, size: u32) -> Self { self.size = size; self }

    pub fn set_text(&mut self, text: &str) { self.text = String::from(text); }
    pub fn text(&self) -> &str { &self.text }
}

impl Widget for Label {
    fn render(&self, win: &mut Window, _theme: &Theme) {
        win.draw_string(self.x as u32, self.y as u32, &self.text, self.color, 0);
    }

    fn bounds(&self) -> (i32, i32, u32, u32) {
        let w = self.text.len() as u32 * 8;
        (self.x, self.y, w, 16)
    }

    fn set_position(&mut self, x: i32, y: i32) { self.x = x; self.y = y; }
    fn set_size(&mut self, _w: u32, _h: u32) {}
}
