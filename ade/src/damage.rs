//! Damage tracking — dirty rectangle accumulation for efficient repaints.

use crate::geometry::Rect;
use alloc::vec::Vec;

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

    #[allow(dead_code)]
    pub fn add(&mut self, r: Rect) {
        if self.full {
            return;
        }
        if self.rects.len() >= 16 {
            self.full = true;
            self.rects.clear();
            return;
        }
        for existing in &self.rects {
            if existing.contains(&r) {
                return;
            }
        }
        self.rects.push(r);
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
