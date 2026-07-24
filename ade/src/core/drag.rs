//! Drag and drop — unified drag context for windows, icons, rubber band.

use crate::core::window::WindowId;

pub(crate) enum DragOp {
    None,
    WindowMove(WindowId),
    WindowResize(WindowId, u8, (i32, i32, u32, u32)),
    IconMove,
    RubberBand(i32, i32),
}
