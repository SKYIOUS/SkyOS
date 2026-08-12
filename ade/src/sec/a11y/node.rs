use crate::core::geometry::Rect;
use crate::core::window::WindowId;
use alloc::string::String;
use alloc::vec::Vec;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
// Trimmed from 18 → 7 during the P1 dead-code sweep: only the roles actually
// constructed by `build_tree` remain. Re-add roles (Dialog, TextInput,
// ListItem, Slider, …) as widgets grow — add the match arms with them.
pub(crate) enum A11yRole {
    Desktop,
    Taskbar,
    StartMenu,
    Window,
    Button,
    Icon,
    Notification,
    /// The tray panel (entries + clock) — owner-stamped with the
    /// `TRAY_PANEL_OWNER` sentinel so tooltip resolution can name it, and
    /// non-focusable (a status surface, not a keyboard control).
    TrayPanel,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct A11yState {
    pub focused: bool,
    pub visible: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct A11yNode {
    pub id: u32,
    pub role: A11yRole,
    pub label: String,
    pub bounds: Rect,
    pub state: A11yState,
    pub focusable: bool,
    pub parent: Option<u32>,
    pub children: Vec<u32>,
    /// WindowId of the window this node belongs to (windows and their
    /// control buttons). Lets activation route a control back to its window.
    pub owner: Option<WindowId>,
}
