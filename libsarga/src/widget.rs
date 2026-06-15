use crate::gui::Window;
use crate::theme::Theme;
use alloc::boxed::Box;
use alloc::vec::Vec;

/// Base trait for all GUI widgets
pub trait Widget {
    fn render(&self, win: &mut Window, theme: &Theme);
    fn handle_click(&mut self, _x: i32, _y: i32, _pressed: bool) -> bool { false }
    fn handle_key(&mut self, _key: u8) -> bool { false }
    fn bounds(&self) -> (i32, i32, u32, u32);
    fn set_position(&mut self, x: i32, y: i32);
    fn set_size(&mut self, w: u32, h: u32);
    fn contains(&self, x: i32, y: i32) -> bool {
        let (bx, by, bw, bh) = self.bounds();
        x >= bx && x < bx + bw as i32 && y >= by && y < by + bh as i32
    }
    fn is_focused(&self) -> bool { false }
    fn set_focus(&mut self, _focused: bool) {}
}

pub struct Container {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    pub children: Vec<Box<dyn Widget>>,
}

impl Container {
    pub fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Container { x, y, width, height, children: Vec::new() }
    }
    pub fn add(&mut self, widget: Box<dyn Widget>) {
        self.children.push(widget);
    }
}

impl Widget for Container {
    fn render(&self, win: &mut Window, theme: &Theme) {
        for child in &self.children { child.render(win, theme); }
    }
    fn handle_click(&mut self, x: i32, y: i32, pressed: bool) -> bool {
        for child in self.children.iter_mut().rev() {
            if child.handle_click(x, y, pressed) { return true; }
        }
        false
    }
    fn handle_key(&mut self, key: u8) -> bool {
        for child in self.children.iter_mut() {
            if child.is_focused() && child.handle_key(key) { return true; }
        }
        false
    }
    fn bounds(&self) -> (i32, i32, u32, u32) { (self.x, self.y, self.width, self.height) }
    fn set_position(&mut self, x: i32, y: i32) { self.x = x; self.y = y; }
    fn set_size(&mut self, w: u32, h: u32) { self.width = w; self.height = h; }
}
