use crate::gui::Window;
use crate::theme::Theme;
use crate::widget::Widget;

pub struct ProgressBar {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    value: f32, // 0.0 to 1.0
    color: u32,
}

impl ProgressBar {
    pub fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        ProgressBar { x, y, width, height, value: 0.0, color: 0xFF0078D4 }
    }

    pub fn set_value(&mut self, value: f32) { self.value = value.clamp(0.0, 1.0); }
    pub fn value(&self) -> f32 { self.value }

    pub fn with_color(mut self, color: u32) -> Self { self.color = color; self }
}

impl Widget for ProgressBar {
    fn render(&self, win: &mut Window, theme: &Theme) {
        // Background track
        win.draw_rounded_rect(self.x as u32, self.y as u32, self.width, self.height, self.height / 2, theme.bg_elevated);

        // Filled portion
        let fill_w = (self.width as f32 * self.value) as u32;
        if fill_w > 0 {
            win.draw_rounded_rect(self.x as u32, self.y as u32, fill_w, self.height, self.height / 2, self.color);
        }
    }

    fn bounds(&self) -> (i32, i32, u32, u32) { (self.x, self.y, self.width, self.height) }
    fn set_position(&mut self, x: i32, y: i32) { self.x = x; self.y = y; }
    fn set_size(&mut self, w: u32, h: u32) { self.width = w; self.height = h; }
}
