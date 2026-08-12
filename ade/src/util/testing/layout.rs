//! Layout parity tests — pin the unified geometry values that draw and
//! hit-testing now share, so the historical divergences (taskbar pitch 120
//! vs 125, titlebar 22 vs 28, misaligned button hit regions) cannot return.
//! These are deliberate pins of concrete values, mirroring how `test_keymap`
//! pins the legacy key bindings.

use crate::core::geometry::{Point, Rect};
use crate::layout;
use crate::layout::WindowHit;
use libsarga::io;

pub(crate) fn test_layout() -> bool {
    // Taskbar buttons: unified 125px pitch, 120px wide (clicks were 120/115).
    if layout::taskbar_btn_x(1) - layout::taskbar_btn_x(0) != layout::TASKBAR_BTN_PITCH {
        io::print_str("[test] FAIL test_layout: taskbar pitch\n");
        return false;
    }
    let b0 = layout::taskbar_btn_rect(0, 564);
    if b0.w != 120 || b0.h != layout::TASKBAR_H - 8 {
        io::print_str("[test] FAIL test_layout: taskbar button rect\n");
        return false;
    }
    // Start button is 58 wide — clicks must not extend past the drawn button.
    let sb = layout::start_btn_rect(564);
    if sb.w != 58 || sb.x != 5 {
        io::print_str("[test] FAIL test_layout: start button rect\n");
        return false;
    }
    // Titlebar: every hit test uses the full 28px visual height (was 22 for
    // the system menu / middle-click).
    if layout::titlebar_rect(100, 200, 400).h != 28 {
        io::print_str("[test] FAIL test_layout: titlebar height\n");
        return false;
    }
    // Window buttons: hit rects == the drawn rects (w-28 / w-54, y+6, 22x18).
    let c = layout::close_btn_rect(100, 200, 400);
    let m = layout::min_btn_rect(100, 200, 400);
    if c.x != 100 + 400 - 28 || c.y != 200 + 6 || c.w != 22 || c.h != 18 {
        io::print_str("[test] FAIL test_layout: close btn rect\n");
        return false;
    }
    if m.x != 100 + 400 - 54 || m.y != 200 + 6 || m.w != 22 || m.h != 18 {
        io::print_str("[test] FAIL test_layout: min btn rect\n");
        return false;
    }
    // Start menu: bottom gap is 4 (the draw's saturating_sub base), not 5.
    let mr = layout::menu_rect(564);
    if mr.y != (564 - layout::MENU_H - layout::MENU_BOTTOM_GAP) as i32 {
        io::print_str("[test] FAIL test_layout: menu origin\n");
        return false;
    }
    // Truncation limits unified.
    if layout::LINE_TRUNCATE_MAX != 55 || layout::TASKBAR_TITLE_MAX != 14 {
        io::print_str("[test] FAIL test_layout: truncation limits\n");
        return false;
    }
    // Sanity: layout rects are well-formed.
    let lr = layout::menu_list_rect(mr);
    if lr.w == 0 || lr.h == 0 || !Rect::new(lr.x, lr.y, lr.w, lr.h).hit_test(mr_center(mr)) {
        io::print_str("[test] FAIL test_layout: list rect sanity\n");
        return false;
    }
    io::print_str("[test] PASS test_layout\n");
    true
}

/// Pins the window chrome hit-testing table: the left-click priority order
/// `handle_click` uses (close → minimize → titlebar strip → resize edges →
/// content), the edge flag encoding, and the release-drag snap-region
/// decision. Right/middle-click deliberately do NOT use this table (they
/// keep the whole 28px strip as the system-menu/close target). If this test
/// fails, a click-routing change was made, not just a refactor.
pub(crate) fn test_hit_window() -> bool {
    // Window under test: (100, 200) 400x300.
    // Titlebar strip away from the buttons → Titlebar (drag).
    if !matches!(
        layout::hit_window(100, 200, 400, 300, Point::new(150, 210)),
        WindowHit::Titlebar
    ) {
        io::print_str("[test] FAIL test_hit_window: titlebar strip\n");
        return false;
    }
    // Control buttons win over the titlebar they are drawn on.
    if !matches!(
        layout::hit_window(100, 200, 400, 300, Point::new(483, 215)),
        WindowHit::Close
    ) {
        io::print_str("[test] FAIL test_hit_window: close button\n");
        return false;
    }
    if !matches!(
        layout::hit_window(100, 200, 400, 300, Point::new(457, 215)),
        WindowHit::Minimize
    ) {
        io::print_str("[test] FAIL test_hit_window: minimize button\n");
        return false;
    }
    // The strip between the buttons still drags (no dead zone appeared).
    if !matches!(
        layout::hit_window(100, 200, 400, 300, Point::new(469, 215)),
        WindowHit::Titlebar
    ) {
        io::print_str("[test] FAIL test_hit_window: gap between buttons is titlebar\n");
        return false;
    }
    // Resize edges (1 = left, 2 = right, 4 = bottom), only below the titlebar.
    if !matches!(
        layout::hit_window(100, 200, 400, 300, Point::new(102, 240)),
        WindowHit::ResizeEdge(1)
    ) {
        io::print_str("[test] FAIL test_hit_window: left edge\n");
        return false;
    }
    if !matches!(
        layout::hit_window(100, 200, 400, 300, Point::new(498, 240)),
        WindowHit::ResizeEdge(2)
    ) {
        io::print_str("[test] FAIL test_hit_window: right edge\n");
        return false;
    }
    if !matches!(
        layout::hit_window(100, 200, 400, 300, Point::new(300, 498)),
        WindowHit::ResizeEdge(4)
    ) {
        io::print_str("[test] FAIL test_hit_window: bottom edge\n");
        return false;
    }
    if !matches!(
        layout::hit_window(100, 200, 400, 300, Point::new(102, 498)),
        WindowHit::ResizeEdge(5)
    ) {
        io::print_str("[test] FAIL test_hit_window: corner edges combine\n");
        return false;
    }
    // Content and outside.
    if !matches!(
        layout::hit_window(100, 200, 400, 300, Point::new(300, 300)),
        WindowHit::Content
    ) {
        io::print_str("[test] FAIL test_hit_window: content\n");
        return false;
    }
    if !matches!(
        layout::hit_window(100, 200, 400, 300, Point::new(600, 100)),
        WindowHit::Outside
    ) {
        io::print_str("[test] FAIL test_hit_window: outside\n");
        return false;
    }
    // Edge flag encoding, direct.
    let e = |x, y| layout::hit_window_edge(100, 200, 400, 300, Point::new(x, y));
    if e(102, 240) != 1 || e(498, 240) != 2 || e(300, 498) != 4 || e(300, 300) != 0 {
        io::print_str("[test] FAIL test_hit_window: edge flags\n");
        return false;
    }
    // Release-drag snap regions: corners first, then edges, then none.
    let snap = |x, y| layout::snap_region_at(x, y, 800, 564);
    if snap(2, 2) != Some(layout::SnapRegion::TopLeft)
        || snap(795, 2) != Some(layout::SnapRegion::TopRight)
        || snap(2, 560) != Some(layout::SnapRegion::BottomLeft)
        || snap(795, 560) != Some(layout::SnapRegion::BottomRight)
    {
        io::print_str("[test] FAIL test_hit_window: snap corners\n");
        return false;
    }
    if snap(2, 300) != Some(layout::SnapRegion::Left)
        || snap(795, 300) != Some(layout::SnapRegion::Right)
        || snap(400, 2) != Some(layout::SnapRegion::Top)
        || snap(400, 560) != Some(layout::SnapRegion::Bottom)
    {
        io::print_str("[test] FAIL test_hit_window: snap edges\n");
        return false;
    }
    if snap(400, 300).is_some() {
        io::print_str("[test] FAIL test_hit_window: snap none\n");
        return false;
    }
    io::print_str("[test] PASS test_hit_window\n");
    true
}

fn mr_center(m: Rect) -> crate::core::geometry::Point {
    crate::core::geometry::Point::new(m.x + m.w as i32 / 2, m.y + m.h as i32 / 2)
}
