use crate::gui::Window;
use crate::theme::Theme;
use crate::widget::Widget;

pub struct ScrollBar {
    x: i32,
    y: i32,
    vertical: bool,
    length: u32,
    thumb_size: u32,
    thumb_pos: u32,
    content_size: u32,
    view_size: u32,
    dragging: bool,
    drag_offset: i32,
}

impl ScrollBar {
    pub fn vertical(x: i32, y: i32, height: u32) -> Self {
        ScrollBar { x, y, vertical: true, length: height, thumb_size: 30, thumb_pos: 0, content_size: 100, view_size: 100, dragging: false, drag_offset: 0 }
    }

    pub fn horizontal(x: i32, y: i32, width: u32) -> Self {
        ScrollBar { x, y, vertical: false, length: width, thumb_size: 30, thumb_pos: 0, content_size: 100, view_size: 100, dragging: false, drag_offset: 0 }
    }

    pub fn set_content(&mut self, content_size: u32, view_size: u32) {
        self.content_size = content_size.max(1);
        self.view_size = view_size;
        self.thumb_size = ((view_size as f32 / content_size as f32) * self.length as f32) as u32;
        self.thumb_size = self.thumb_size.max(20);
    }

    pub fn scroll_offset(&self) -> u32 {
        let max_scroll = self.content_size.saturating_sub(self.view_size);
        if max_scroll == 0 { }
        (self.thumb_pos as f32 / (self.length as f32 - self.thumb_size as f32) * max_scroll as f32) as u32
    }
}

impl Widget for ScrollBar {
    fn render(&self, win: &mut Window, theme: &Theme) {
        // Track
        if self.vertical {
            win.draw_rect(self.x as u32, self.y as u32, 6, self.length, theme.bg_elevated);
            // Thumb
            win.draw_rounded_rect(self.x as u32, self.y as u32 + self.thumb_pos, 6, self.thumb_size, 3, theme.text_secondary);
        } else {
            win.draw_rect(self.x as u32, self.y as u32, self.length, 6, theme.bg_elevated);
            win.draw_rounded_rect(self.x as u32 + self.thumb_pos, self.y as u32, self.thumb_size, 6, 3, theme.text_secondary);
        }
    }

    fn handle_click(&mut self, x: i32, y: i32, pressed: bool) -> bool {
        if self.vertical {
            if x >= self.x && x < self.x + 6 && y >= self.y && y < self.y + self.length as i32 {
                if pressed {
                    self.dragging = true;
                    self.drag_offset = y - self.y as i32 - self.thumb_pos as i32;
                    return true;
                }
            }
            if self.dragging && pressed {
                let new_pos = (y - self.y as i32 - self.drag_offset).max(0) as u32;
                self.thumb_pos = new_pos.min(self.length.saturating_sub(self.thumb_size));
                return true;
            }
        } else {
            if x >= self.x && x < self.x + self.length as i32 && y >= self.y && y < self.y + 6 {
                if pressed {
                    self.dragging = true;
                    self.drag_offset = x - self.x as i32 - self.thumb_pos as i32;
                    return true;
                }
            }
            if self.dragging && pressed {
                let new_pos = (x - self.x as i32 - self.drag_offset).max(0) as u32;
                self.thumb_pos = new_pos.min(self.length.saturating_sub(self.thumb_size));
                return true;
            }
        }
        if !pressed { self.dragging = false; }
        false
    }

    fn bounds(&self) -> (i32, i32, u32, u32) {
        if self.vertical { (self.x, self.y, 6, self.length) }
        else { (self.x, self.y, self.length, 6) }
    }
    fn set_position(&mut self, x: i32, y: i32) { self.x = x; self.y = y; }
    fn set_size(&mut self, _w: u32, h: u32) { self.length = h; }
}
