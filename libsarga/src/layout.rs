use crate::widget::Widget;
use alloc::boxed::Box;

pub enum LayoutDirection {
    Vertical,
    Horizontal,
}

pub struct LinearLayout {
    direction: LayoutDirection,
    spacing: i32,
    padding: i32,
}

impl LinearLayout {
    pub fn vertical(spacing: i32, padding: i32) -> Self {
        LinearLayout { direction: LayoutDirection::Vertical, spacing, padding }
    }
    pub fn horizontal(spacing: i32, padding: i32) -> Self {
        LinearLayout { direction: LayoutDirection::Horizontal, spacing, padding }
    }

    pub fn apply(&self, children: &mut [Box<dyn Widget>], x: i32, y: i32, max_w: u32, max_h: u32) {
        let mut offset = self.padding;
        match self.direction {
            LayoutDirection::Vertical => {
                for child in children.iter_mut() {
                    let (_, _, _w, h) = child.bounds();
                    child.set_position(x + self.padding, y + offset);
                    child.set_size(max_w.saturating_sub(self.padding as u32 * 2), h);
                    offset += h as i32 + self.spacing;
                }
            }
            LayoutDirection::Horizontal => {
                for child in children.iter_mut() {
                    let (_, _, w, _h) = child.bounds();
                    child.set_position(x + offset, y + self.padding);
                    child.set_size(w, max_h.saturating_sub(self.padding as u32 * 2));
                    offset += w as i32 + self.spacing;
                }
            }
        }
    }
}

pub struct GridLayout {
    cols: usize,
    spacing: i32,
    padding: i32,
}

impl GridLayout {
    pub fn new(cols: usize, spacing: i32, padding: i32) -> Self {
        GridLayout { cols, spacing, padding }
    }

    pub fn apply(&self, children: &mut [Box<dyn Widget>], x: i32, y: i32, total_w: u32) {
        let cell_w = (total_w as i32 - self.padding * 2 - self.spacing * (self.cols as i32 - 1)) / self.cols as i32;
        for (i, child) in children.iter_mut().enumerate() {
            let col = i % self.cols;
            let row = i / self.cols;
            let cx = x + self.padding + col as i32 * (cell_w + self.spacing);
            let cy = y + self.padding + row as i32 * (cell_w + self.spacing);
            child.set_position(cx, cy);
            child.set_size(cell_w as u32, cell_w as u32);
        }
    }
}
