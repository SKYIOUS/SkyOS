use crate::gui::Window;
use crate::theme::Theme;
use crate::widget::Widget;

pub struct Slider {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    value: f32,
    min: f32,
    max: f32,
    dragging: bool,
    thumb_w: u32,
}

impl Slider {
    pub fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Slider { x, y, width, height, value: 0.5, min: 0.0, max: 1.0, dragging: false, thumb_w: 12 }
    }

    pub fn with_range(mut self, min: f32, max: f32) -> Self { self.min = min; self.max = max; self }
    pub fn with_value(mut self, value: f32) -> Self { self.value = value.clamp(self.min, self.max); self }

    pub fn set_value(&mut self, value: f32) { self.value = value.clamp(self.min, self.max); }
    pub fn value(&self) -> f32 { self.value }

    fn value_to_x(&self) -> f32 {
        let t = (self.value - self.min) / (self.max - self.min);
        self.x as f32 + t * (self.width as f32 - self.thumb_w as f32)
    }

    fn x_to_value(&self, px: f32) -> f32 {
        let t = (px - self.x as f32) / (self.width as f32 - self.thumb_w as f32);
        self.min + t.clamp(0.0, 1.0) * (self.max - self.min)
    }
}

impl Widget for Slider {
    fn render(&self, win: &mut Window, theme: &Theme) {
        let track_y = self.y as u32 + self.height / 2 - 2;
        let track_h: u32 = 4;

        // Track background
        win.draw_rect(self.x as u32, track_y, self.width, track_h, theme.bg_elevated);

        // Filled portion
        let fill_w = ((self.value - self.min) / (self.max - self.min) * self.width as f32) as u32;
        win.draw_rect(self.x as u32, track_y, fill_w, track_h, theme.accent);

        // Thumb
        let thumb_x = self.value_to_x() as u32;
        let thumb_y = self.y as u32;
        let thumb_h = self.height;
        win.draw_rounded_rect(thumb_x, thumb_y, self.thumb_w, thumb_h, self.thumb_w / 2, theme.accent);
    }

    fn handle_click(&mut self, x: i32, y: i32, pressed: bool) -> bool {
        if y >= self.y && y < self.y + self.height as i32 {
            if pressed {
                self.dragging = true;
                self.value = self.x_to_value(x as f32);
                return true;
            }
        }
        if !pressed { self.dragging = false; }
        if self.dragging {
            self.value = self.x_to_value(x as f32);
            return true;
        }
        false
    }

    fn bounds(&self) -> (i32, i32, u32, u32) { (self.x, self.y, self.width, self.height) }
    fn set_position(&mut self, x: i32, y: i32) { self.x = x; self.y = y; }
    fn set_size(&mut self, w: u32, h: u32) { self.width = w; self.height = h; }
}
