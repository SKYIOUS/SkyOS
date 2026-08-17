//! Input event types — keyboard, mouse, scroll.

use crate::core::geometry::Point;

// Constructed by the input producers in main.rs.
pub enum Event {
    // A kernel key value: low byte = the character, bits 8..10 = alt/ctrl/
    // shift held (the Phase C packed-key contract — see
    // docs/kernel-gui-modifier-delivery.md, Design A). Today the kernel
    // sends plain bytes, so the high bits are zero; `Desktop::handle_key`
    // decodes via `input::KeyEvent::from_raw`.
    Key(u16),
    MouseClick(Point),
    MouseMiddle(Point),
    MouseRight(Point),
    MouseDrag(Point),
    MouseRelease,
    Scroll(i8),
}
