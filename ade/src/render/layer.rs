//! Layer enumeration for the compositor.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Layer {
    Wallpaper = 0,
    Desktop,
    Windows,
    Popups,
    Overlay,
    Cursor,
}

pub(crate) const LAYER_COUNT: usize = 6;
