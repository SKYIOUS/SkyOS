use alloc::string::String;
use crate::render::compositor::Canvas;

pub(crate) struct Tooltip {
    pub text: String,
    pub x: i32,
    pub y: i32,
    pub timeout: u32,
    pub visible: bool,
}

pub(crate) struct TooltipManager {
    pub active: Option<Tooltip>,
    delay: u32,
}

impl TooltipManager {
    pub fn new() -> Self {
        TooltipManager {
            active: None,
            delay: 30,
        }
    }

    pub fn show(&mut self, text: &str, x: i32, y: i32, timeout: u32) {
        self.active = Some(Tooltip {
            text: text.into(),
            x,
            y,
            timeout,
            visible: true,
        });
    }

    pub fn hide(&mut self) {
        self.active = None;
    }

    pub fn tick(&mut self) {
        if let Some(ref mut t) = self.active {
            if t.timeout > 0 {
                t.timeout -= 1;
            }
            if t.timeout == 0 {
                self.active = None;
            }
        }
    }

    pub fn draw(&self, canvas: &mut Canvas, screen_w: u32, screen_h: u32) {
        if let Some(ref t) = self.active {
            if !t.visible {
                return;
            }
            let tw = (t.text.len() as u32 * 8 + 16).min(screen_w.saturating_sub(8));
            let mut tx = t.x;
            let mut ty = t.y.saturating_sub(26);
            if tx + tw as i32 > screen_w as i32 {
                tx = (screen_w as i32).saturating_sub(tw as i32 + 4);
            }
            if tx < 2 {
                tx = 2;
            }
            if ty < 2 {
                ty = t.y + 20;
            }
            canvas.draw_rounded_rect(tx as u32, ty as u32, tw, 22, 4, 0xFF2D2D2D);
            canvas.draw_rounded_rect_outline(tx as u32, ty as u32, tw, 22, 4, 0xFF555555);
            canvas.draw_string(tx as u32 + 8, ty as u32 + 5, &t.text, 0xFFFFFFFF, 0);
        }
    }
}
