//! Input event types — keyboard, mouse, scroll.

pub enum Event {
    Key(u8),
    MouseClick(i32, i32),
    MouseMiddle(i32, i32),
    MouseRight(i32, i32),
    MouseDrag(i32, i32),
    MouseRelease,
    Scroll(i8),
}
