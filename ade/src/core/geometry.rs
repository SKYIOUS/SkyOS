//! Geometry primitives — Point, Rect with hit-test and overlap checks.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

impl Rect {
    pub const fn new(x: i32, y: i32, w: u32, h: u32) -> Self {
        Self { x, y, w, h }
    }

    pub fn hit_test(&self, p: Point) -> bool {
        let r = self.x + self.w as i32;
        let b = self.y + self.h as i32;
        p.x >= self.x && p.x < r && p.y >= self.y && p.y < b
    }

    pub fn intersects(&self, other: &Rect) -> bool {
        let ar = self.x + self.w as i32;
        let ab = self.y + self.h as i32;
        let or = other.x + other.w as i32;
        let ob = other.y + other.h as i32;
        self.x < or && ar > other.x && self.y < ob && ab > other.y
    }
}

/// One entry in a context menu: display label plus the action dispatched when
/// clicked. A label of `"---"` is a visual separator row (no action).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MenuItem {
    pub label: &'static str,
    pub action: &'static str,
}

/// An open context menu: anchor position plus its static item list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContextMenu {
    pub x: i32,
    pub y: i32,
    pub items: &'static [MenuItem],
}

/// Rubber-band selection — two dragged corners. The drag rect is derived by
/// normalizing (min of the two corners, absolute delta as size), so drags up
/// or left work identically to down/right.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RubberBand {
    pub x1: i32,
    pub y1: i32,
    pub x2: i32,
    pub y2: i32,
}

impl RubberBand {
    pub const fn new(x: i32, y: i32) -> Self {
        Self {
            x1: x,
            y1: y,
            x2: x,
            y2: y,
        }
    }

    /// Move the drag corner to a new pointer position.
    pub fn drag_to(&mut self, x: i32, y: i32) {
        self.x2 = x;
        self.y2 = y;
    }

    /// Normalized selection rect (handles drags up/left).
    pub fn rect(&self) -> Rect {
        Rect::new(
            self.x1.min(self.x2),
            self.y1.min(self.y2),
            (self.x1 - self.x2).unsigned_abs(),
            (self.y1 - self.y2).unsigned_abs(),
        )
    }
}
