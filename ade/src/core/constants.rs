//! Shared constants — layout sizes, menu definitions.

pub const TASKBAR_H: u32 = 36;

// Window title bar metrics
pub const TITLE_H: i32 = 28;
pub const BTN_TOP: i32 = 3;
pub const BTN_BOT: i32 = 19;
pub const CLOSE_R: i32 = 4;
pub const CLOSE_L: i32 = 24;
pub const MAX_R: i32 = 58;
pub const MAX_L: i32 = 80;
pub const MIN_R: i32 = 28;
pub const MIN_L: i32 = 48;

// Window resize/snap
pub const RESIZE_MARGIN: i32 = 4;
pub const SNAP_MARGIN: i32 = 15;
pub const MIN_WIN_W: u32 = 100;
pub const MIN_WIN_H: u32 = 80;

#[allow(dead_code)] // snap preview alpha, paired with SNAP_PREVIEW_COLOR (used by render)
pub const SNAP_PREVIEW_ALPHA: u32 = 0x40;
pub const SNAP_PREVIEW_COLOR: u32 = 0x403D5AFE;
