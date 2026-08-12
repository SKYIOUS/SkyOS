//! Damage tracking — full-screen invalidation for repaints.
//!
//! Rect-level tracking was removed (never used); the compositor repaints
//! the full surface when `is_dirty()` is true. Phase 4 will re-introduce
//! region tracking alongside incremental compose.

pub struct DamageTracker {
    pub full: bool,
}

impl DamageTracker {
    pub fn new() -> Self {
        Self { full: true }
    }

    pub fn mark_full(&mut self) {
        self.full = true;
    }

    pub fn is_dirty(&self) -> bool {
        self.full
    }

    pub fn clear(&mut self) {
        self.full = false;
    }
}
