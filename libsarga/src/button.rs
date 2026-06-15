use crate::gui::{Window, alpha_blend};
use crate::theme::Theme;
use crate::widget::Widget;
use crate::alloc::string::String;

pub struct Button {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    label: String,
    pressed: bool,
    hovered: bool,
    enabled: bool,
}

impl Button {
    pub fn new(x: i32, y: i32, width: u32, height: u32, label: &str) -> Self {
        Button {
            x, y, width, height,
            label: String::from(label),
            pressed: false, hovered: false, enabled: true,
        }
    }

    pub fn set_label(&mut self, label: &str) {
        self.label = String::from(label);
    }
}

impl Widget for Button {
    fn render(&self, win: &mut Window, theme: &Theme) {
        let color = if !self.enabled {
            theme.text_disabled
        } else if self.pressed {
            theme.accent_dark
        } else if self.hovered {
            theme.accent_light
        } else {
            theme.accent
        };

        // Button background with rounded corners
        win.draw_rounded_rect(self.x as u32, self.y as u32, self.width, self.height, theme.border_radius, color);

        // Button border
        win.draw_line_h(self.x as u32, self.y as u32, self.width, alpha_blend(color, 0xFFFFFFFF, 30));
        win.draw_line_h(self.x as u32, self.y as u32 + self.height - 1, self.width, alpha_blend(color, 0x00000000, 60));

        // Centered text
        let text_color = if self.enabled { 0xFFFFFFFF } else { theme.text_disabled };
        let text_w = win.measure_text(&self.label);
        let text_x = self.x as u32 + (self.width.saturating_sub(text_w)) / 2;
        let text_y = self.y as u32 + (self.height - theme.font_size) / 2;
        win.draw_string(text_x, text_y, &self.label, text_color, 0);
    }

    fn handle_click(&mut self, x: i32, y: i32, pressed: bool) -> bool {
        if self.contains(x, y) && self.enabled {
            self.pressed = pressed;
            true
        } else {
            self.pressed = false;
            false
        }
    }

    fn bounds(&self) -> (i32, i32, u32, u32) {
        (self.x, self.y, self.width, self.height)
    }

    fn set_position(&mut self, x: i32, y: i32) {
        self.x = x;
        self.y = y;
    }

    fn set_size(&mut self, w: u32, h: u32) {
        self.width = w;
        self.height = h;
    }
}
