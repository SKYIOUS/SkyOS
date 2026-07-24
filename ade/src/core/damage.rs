//! Damage tracking — dirty rectangle accumulation for efficient repaints.

use crate::core::geometry::Rect;
use alloc::vec::Vec;
use core::mem;

pub struct DamageTracker {
    rects: Vec<Rect>,
    pub full: bool,
}

impl DamageTracker {
    pub fn new() -> Self {
        Self {
            rects: Vec::new(),
            full: true,
        }
    }

    /// Add a damaged region, merging with any overlapping or touching rects.
    pub fn add(&mut self, r: Rect) {
        if self.full {
            return;
        }
        let mut merged = r;
        let mut i = 0;
        while i < self.rects.len() {
            if rects_touch(&self.rects[i], &merged) {
                merged = rect_union(&self.rects[i], &merged);
                self.rects.swap_remove(i);
            } else {
                i += 1;
            }
        }
        self.rects.push(merged);
    }

    /// Return all accumulated damage rects and clear the tracker.
    pub fn drain(&mut self) -> Vec<Rect> {
        let result = mem::take(&mut self.rects);
        self.full = false;
        result
    }

    pub fn mark_full(&mut self) {
        self.full = true;
        self.rects.clear();
    }

    pub fn is_dirty(&self) -> bool {
        self.full || !self.rects.is_empty()
    }

    pub fn clear(&mut self) {
        self.rects.clear();
        self.full = false;
    }
}

fn rects_touch(a: &Rect, b: &Rect) -> bool {
    let ar = a.x + a.w as i32;
    let ab = a.y + a.h as i32;
    let br = b.x + b.w as i32;
    let bb = b.y + b.h as i32;
    a.x <= br && ar >= b.x && a.y <= bb && ab >= b.y
}

fn rect_union(a: &Rect, b: &Rect) -> Rect {
    let x = a.x.min(b.x);
    let y = a.y.min(b.y);
    let r = (a.x + a.w as i32).max(b.x + b.w as i32);
    let btm = (a.y + a.h as i32).max(b.y + b.h as i32);
    Rect::new(x, y, (r - x) as u32, (btm - y) as u32)
}
