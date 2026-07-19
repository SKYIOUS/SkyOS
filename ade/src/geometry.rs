//! Geometry primitives — Point, Size, Rect with hit-test and transforms.

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

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Size {
    pub w: u32,
    pub h: u32,
}

#[allow(dead_code)]
impl Size {
    pub const fn new(w: u32, h: u32) -> Self {
        Self { w, h }
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

    #[allow(dead_code)]
    pub fn contains(&self, other: &Rect) -> bool {
        self.x <= other.x
            && self.y <= other.y
            && self.x + self.w as i32 >= other.x + other.w as i32
            && self.y + self.h as i32 >= other.y + other.h as i32
    }

    pub fn intersects(&self, other: &Rect) -> bool {
        let ar = self.x + self.w as i32;
        let ab = self.y + self.h as i32;
        let or = other.x + other.w as i32;
        let ob = other.y + other.h as i32;
        self.x < or && ar > other.x && self.y < ob && ab > other.y
    }

    #[allow(dead_code)]
    pub fn center(&self) -> Point {
        Point::new(self.x + self.w as i32 / 2, self.y + self.h as i32 / 2)
    }

    #[allow(dead_code)]
    pub fn translate(&self, dx: i32, dy: i32) -> Rect {
        Rect::new(self.x + dx, self.y + dy, self.w, self.h)
    }

    #[allow(dead_code)]
    pub fn inflate(&self, dx: u32, dy: u32) -> Rect {
        let x = self.x - dx as i32;
        let y = self.y - dy as i32;
        Rect::new(x, y, self.w + dx * 2, self.h + dy * 2)
    }
}
