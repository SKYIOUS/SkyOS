use crate::core::desktop::Desktop;
use crate::core::window::{HoverTarget, VisualFlags, WindowButton, WindowState};
use crate::layout;
use libsarga::io;

pub(crate) fn test_visual_flags() -> bool {
    let flags = VisualFlags::new();
    if !flags.shadow {
        io::print_str("[test] FAIL test_visual_flags: default shadow is false\n");
        return false;
    }
    if flags.opacity != 255 {
        io::print_str("[test] FAIL test_visual_flags: default opacity != 255\n");
        return false;
    }
    if !flags.rounded {
        io::print_str("[test] FAIL test_visual_flags: default rounded is false\n");
        return false;
    }
    if !flags.border {
        io::print_str("[test] FAIL test_visual_flags: default border is false\n");
        return false;
    }
    if !flags.active {
        io::print_str("[test] FAIL test_visual_flags: default active is false\n");
        return false;
    }
    if flags.border_width != 1 {
        io::print_str("[test] FAIL test_visual_flags: default border_width != 1\n");
        return false;
    }
    if flags.blur {
        io::print_str("[test] FAIL test_visual_flags: default blur is true\n");
        return false;
    }
    if flags.transparent {
        io::print_str("[test] FAIL test_visual_flags: default transparent is true\n");
        return false;
    }
    io::print_str("[test] PASS test_visual_flags\n");
    true
}

pub(crate) fn test_window_state() -> bool {
    let normal = WindowState::Normal;
    let minimized = WindowState::Minimized;
    let maximized = WindowState::Maximized;
    let fullscreen = WindowState::Fullscreen;

    // Check PartialEq works between variants
    if normal != WindowState::Normal {
        io::print_str("[test] FAIL test_window_state: Normal != Normal\n");
        return false;
    }
    if minimized == normal {
        io::print_str("[test] FAIL test_window_state: Minimized == Normal\n");
        return false;
    }
    if maximized == minimized {
        io::print_str("[test] FAIL test_window_state: Maximized == Minimized\n");
        return false;
    }
    if fullscreen == maximized {
        io::print_str("[test] FAIL test_window_state: Fullscreen == Maximized\n");
        return false;
    }
    io::print_str("[test] PASS test_window_state\n");
    true
}

/// Hover affordance: the pointer over a window's close/min button must be
/// reported in the render snapshot (via `Desktop::hover_target()`, which
/// mirrors `handle_click`'s topmost-first hit table), and suppressed when a
/// modal overlay swallows the pointer. Runs on a fresh desktop.
pub(crate) fn test_window_hover() -> bool {
    let mut d = Desktop::new(800, 600);
    let wid = d.wm.create(crate::core::window::AppWindow::new(
        100, 100, 400, 300, "HoverWin",
    ));

    // Close button center.
    let close = layout::close_btn_rect(100, 100, 400);
    d.update_mouse(
        close.x + close.w as i32 / 2,
        close.y + close.h as i32 / 2,
        false,
    );
    if d.snapshot().hover
        != Some(HoverTarget::Window {
            win: wid,
            btn: WindowButton::Close,
        })
    {
        io::print_str("[test] FAIL test_window_hover: close button not hovered\n");
        return false;
    }

    // Minimize button center.
    let min = layout::min_btn_rect(100, 100, 400);
    d.update_mouse(min.x + min.w as i32 / 2, min.y + min.h as i32 / 2, false);
    if d.snapshot().hover
        != Some(HoverTarget::Window {
            win: wid,
            btn: WindowButton::Minimize,
        })
    {
        io::print_str("[test] FAIL test_window_hover: minimize button not hovered\n");
        return false;
    }

    // Window content (not a button) → no hover.
    d.update_mouse(300, 300, false);
    if d.snapshot().hover.is_some() {
        io::print_str("[test] FAIL test_window_hover: content hovered as button\n");
        return false;
    }

    // Modal overlay (start menu open) suppresses hover even over the button
    // (the open menu owns the pointer; a click there closes the menu).
    d.start_menu.open = true;
    d.update_mouse(
        close.x + close.w as i32 / 2,
        close.y + close.h as i32 / 2,
        false,
    );
    if d.snapshot().hover.is_some() {
        io::print_str("[test] FAIL test_window_hover: hover leaked under start menu\n");
        return false;
    }

    io::print_str("[test] PASS test_window_hover\n");
    true
}

/// Unified hover: every surface (taskbar buttons, start button, tray
/// entries, start-menu rows, clipboard rows) reports through the single
/// `Desktop::hover_target()` hit test into `snap.hover`, and the enum
/// payloads match the surface under the pointer. Runs on a fresh desktop.
pub(crate) fn test_surface_hover() -> bool {
    let mut d = Desktop::new(800, 600);
    let wid = d.wm.create(crate::core::window::AppWindow::new(
        100, 100, 400, 300, "SurfWin",
    ));
    let ty = d.taskbar_y();

    // Taskbar window button.
    let btn = layout::taskbar_btn_rect(0, ty);
    d.update_mouse(btn.x + btn.w as i32 / 2, btn.y + btn.h as i32 / 2, false);
    if d.snapshot().hover != Some(HoverTarget::TaskbarButton(wid)) {
        io::print_str("[test] FAIL test_surface_hover: taskbar button not hovered\n");
        return false;
    }

    // Start button.
    let start = layout::start_btn_rect(ty);
    d.update_mouse(
        start.x + start.w as i32 / 2,
        start.y + start.h as i32 / 2,
        false,
    );
    if d.snapshot().hover != Some(HoverTarget::StartButton) {
        io::print_str("[test] FAIL test_surface_hover: start button not hovered\n");
        return false;
    }

    // Tray entry.
    let tray_len = d.tray.entries.len() as u32;
    let tr = layout::tray_entry_rect(0, ty, d.screen_w, tray_len);
    d.update_mouse(tr.x + tr.w as i32 / 2, tr.y + tr.h as i32 / 2, false);
    if d.snapshot().hover != Some(HoverTarget::Tray(0)) {
        io::print_str("[test] FAIL test_surface_hover: tray entry not hovered\n");
        return false;
    }

    // Start menu: app row hover once open.
    d.start_menu.open_with(&d.app_reg);
    let menu_r = layout::menu_rect(ty);
    if d.start_menu.filtered.is_empty() {
        io::print_str("[test] FAIL test_surface_hover: empty start menu filter\n");
        return false;
    }
    let item = layout::menu_item_rect(menu_r, 0, 0);
    d.update_mouse(
        item.x + item.w as i32 / 2,
        item.y + item.h as i32 / 2,
        false,
    );
    if d.snapshot().hover != Some(HoverTarget::StartApp(0)) {
        io::print_str("[test] FAIL test_surface_hover: start menu app row not hovered\n");
        return false;
    }
    d.start_menu.open = false;

    // Clipboard row (panel drawn whenever history exists).
    d.services.clipboard.copy("clip", 0);
    let panel = layout::clipboard_panel_rect(d.screen_w, d.screen_h, 1);
    let row = layout::clipboard_row_rect(panel, 0);
    d.update_mouse(row.x + row.w as i32 / 2, row.y + row.h as i32 / 2, false);
    if d.snapshot().hover != Some(HoverTarget::ClipboardRow(0)) {
        io::print_str("[test] FAIL test_surface_hover: clipboard row not hovered\n");
        return false;
    }

    // Notification overlay row (drawn whenever a notification is visible),
    // via the same panel geometry the draw uses.
    d.services.notifications.notify("Notif", "body", 0, 120);
    let nrow = layout::notification_rect(d.screen_w, 0);
    d.update_mouse(
        nrow.x + nrow.w as i32 / 2,
        nrow.y + nrow.h as i32 / 2,
        false,
    );
    if d.snapshot().hover != Some(HoverTarget::Notification(0)) {
        io::print_str("[test] FAIL test_surface_hover: notification row not hovered\n");
        return false;
    }

    io::print_str("[test] PASS test_surface_hover\n");
    true
}

/// Pressed feedback: the snapshot's `mouse_down` tracks the raw primary
/// mouse button, and hovering a window control button while held reports
/// both hover and mouse_down (the draw combines them to render the pressed
/// color). Release clears `mouse_down` but keeps hover.
pub(crate) fn test_window_pressed() -> bool {
    let mut d = Desktop::new(800, 600);
    let wid = d.wm.create(crate::core::window::AppWindow::new(
        100, 100, 400, 300, "PressWin",
    ));

    // Hover the close button WITHOUT pressing: hover only.
    let close = layout::close_btn_rect(100, 100, 400);
    d.update_mouse(
        close.x + close.w as i32 / 2,
        close.y + close.h as i32 / 2,
        false,
    );
    let snap = d.snapshot();
    if snap.hover
        != Some(HoverTarget::Window {
            win: wid,
            btn: WindowButton::Close,
        })
        || snap.mouse_down
    {
        io::print_str("[test] FAIL test_window_pressed: hover-only close reported pressed\n");
        return false;
    }

    // Hold the button down: hover stays, pressed becomes true.
    d.update_mouse(
        close.x + close.w as i32 / 2,
        close.y + close.h as i32 / 2,
        true,
    );
    let snap = d.snapshot();
    if snap.hover
        != Some(HoverTarget::Window {
            win: wid,
            btn: WindowButton::Close,
        })
        || !snap.mouse_down
    {
        io::print_str("[test] FAIL test_window_pressed: held close not pressed\n");
        return false;
    }

    // Release: pressed clears, hover persists.
    d.update_mouse(
        close.x + close.w as i32 / 2,
        close.y + close.h as i32 / 2,
        false,
    );
    let snap = d.snapshot();
    if snap.hover
        != Some(HoverTarget::Window {
            win: wid,
            btn: WindowButton::Close,
        })
        || snap.mouse_down
    {
        io::print_str("[test] FAIL test_window_pressed: released close still pressed\n");
        return false;
    }

    // Pressing a non-button (window content) reports pressed but no hover.
    d.update_mouse(300, 300, true);
    let snap = d.snapshot();
    if snap.hover.is_some() || !snap.mouse_down {
        io::print_str("[test] FAIL test_window_pressed: content press hover/pressed mismatch\n");
        return false;
    }

    io::print_str("[test] PASS test_window_pressed\n");
    true
}

/// Pressed feedback on the taskbar: holding the primary button over a
/// taskbar window button, the start button, or a tray entry must report
/// both hover AND pressed in the snapshot — the combination the taskbar
/// draw reads (`theme.pressed` darkens the held surface, mirroring the
/// window control buttons). Hover-only and release must clear `mouse_down`
/// while keeping hover. Mirrors `test_window_pressed` for the taskbar
/// surfaces.
pub(crate) fn test_taskbar_pressed() -> bool {
    let mut d = Desktop::new(800, 600);
    let wid = d.wm.create(crate::core::window::AppWindow::new(
        100,
        100,
        400,
        300,
        "PressTaskbar",
    ));
    let ty = d.taskbar_y();

    // Taskbar window button: hover-only, held, released.
    let btn = layout::taskbar_btn_rect(0, ty);
    let (bx, by) = (btn.x + btn.w as i32 / 2, btn.y + btn.h as i32 / 2);
    d.update_mouse(bx, by, false);
    let snap = d.snapshot();
    if snap.hover != Some(HoverTarget::TaskbarButton(wid)) || snap.mouse_down {
        io::print_str("[test] FAIL test_taskbar_pressed: taskbar hover-only reported pressed\n");
        return false;
    }
    d.update_mouse(bx, by, true);
    let snap = d.snapshot();
    if snap.hover != Some(HoverTarget::TaskbarButton(wid)) || !snap.mouse_down {
        io::print_str("[test] FAIL test_taskbar_pressed: held taskbar button not pressed\n");
        return false;
    }
    d.update_mouse(bx, by, false);
    let snap = d.snapshot();
    if snap.hover != Some(HoverTarget::TaskbarButton(wid)) || snap.mouse_down {
        io::print_str("[test] FAIL test_taskbar_pressed: released taskbar button still pressed\n");
        return false;
    }

    // Start button: hover-only, held, released.
    let start = layout::start_btn_rect(ty);
    let (sx, sy) = (start.x + start.w as i32 / 2, start.y + start.h as i32 / 2);
    d.update_mouse(sx, sy, false);
    let snap = d.snapshot();
    if snap.hover != Some(HoverTarget::StartButton) || snap.mouse_down {
        io::print_str("[test] FAIL test_taskbar_pressed: start hover-only reported pressed\n");
        return false;
    }
    d.update_mouse(sx, sy, true);
    let snap = d.snapshot();
    if snap.hover != Some(HoverTarget::StartButton) || !snap.mouse_down {
        io::print_str("[test] FAIL test_taskbar_pressed: held start button not pressed\n");
        return false;
    }
    d.update_mouse(sx, sy, false);

    // Tray entry: hover-only, held, released. A fresh desktop has the
    // default tray entries, so entry 0 always exists.
    let tray_len = d.tray.entries.len() as u32;
    let tr = layout::tray_entry_rect(0, ty, d.screen_w, tray_len);
    let (tx, tty) = (tr.x + tr.w as i32 / 2, tr.y + tr.h as i32 / 2);
    d.update_mouse(tx, tty, false);
    let snap = d.snapshot();
    if snap.hover != Some(HoverTarget::Tray(0)) || snap.mouse_down {
        io::print_str("[test] FAIL test_taskbar_pressed: tray hover-only reported pressed\n");
        return false;
    }
    d.update_mouse(tx, tty, true);
    let snap = d.snapshot();
    if snap.hover != Some(HoverTarget::Tray(0)) || !snap.mouse_down {
        io::print_str("[test] FAIL test_taskbar_pressed: held tray entry not pressed\n");
        return false;
    }
    d.update_mouse(tx, tty, false);
    let snap = d.snapshot();
    if snap.hover != Some(HoverTarget::Tray(0)) || snap.mouse_down {
        io::print_str("[test] FAIL test_taskbar_pressed: released tray entry still pressed\n");
        return false;
    }

    io::print_str("[test] PASS test_taskbar_pressed\n");
    true
}

/// Pressed feedback on the start menu: holding the primary button over a
/// menu row reports both hover AND pressed in the snapshot (the draw
/// darkens with `theme.pressed`), matching the window-button and taskbar
/// affordances. Hover-only and release clear `mouse_down` while keeping
/// hover. Covers all four row types (app, category, power, and a seeded
/// recent tile).
pub(crate) fn test_start_menu_pressed() -> bool {
    let mut d = Desktop::new(800, 600);
    let ty = d.taskbar_y();
    let menu_r = layout::menu_rect(ty);
    d.start_menu.open_with(&d.app_reg);

    // App row: hover-only, held, released. Row 0 of the All list always
    // exists on a fresh catalog.
    if d.start_menu.filtered.is_empty() {
        io::print_str("[test] FAIL test_start_menu_pressed: empty app list\n");
        return false;
    }
    let row = layout::menu_item_rect(menu_r, 0, 0);
    let (rx, ry) = (row.x + row.w as i32 / 2, row.y + row.h as i32 / 2);
    d.update_mouse(rx, ry, false);
    let snap = d.snapshot();
    if snap.hover != Some(HoverTarget::StartApp(0)) || snap.mouse_down {
        io::print_str("[test] FAIL test_start_menu_pressed: app row hover-only reported pressed\n");
        return false;
    }
    d.update_mouse(rx, ry, true);
    let snap = d.snapshot();
    if snap.hover != Some(HoverTarget::StartApp(0)) || !snap.mouse_down {
        io::print_str("[test] FAIL test_start_menu_pressed: held app row not pressed\n");
        return false;
    }
    d.update_mouse(rx, ry, false);
    let snap = d.snapshot();
    if snap.hover != Some(HoverTarget::StartApp(0)) || snap.mouse_down {
        io::print_str("[test] FAIL test_start_menu_pressed: released app row still pressed\n");
        return false;
    }

    // Category row: held (a category click keeps the menu open, so the
    // pressed frame is the live case).
    let cat = layout::menu_category_rect(menu_r, 1);
    let (cx, cy) = (cat.x + cat.w as i32 / 2, cat.y + cat.h as i32 / 2);
    d.update_mouse(cx, cy, true);
    let snap = d.snapshot();
    if snap.hover != Some(HoverTarget::StartCategory(1)) || !snap.mouse_down {
        io::print_str("[test] FAIL test_start_menu_pressed: held category row not pressed\n");
        return false;
    }
    d.update_mouse(cx, cy, false);

    // Power row: held (no click action — the menu stays open, so this is
    // the clearest visible pressed case).
    let pow = layout::menu_power_rect(menu_r, 0);
    let (px, py) = (pow.x + pow.w as i32 / 2, pow.y + pow.h as i32 / 2);
    d.update_mouse(px, py, true);
    let snap = d.snapshot();
    if snap.hover != Some(HoverTarget::StartPower(0)) || !snap.mouse_down {
        io::print_str("[test] FAIL test_start_menu_pressed: held power row not pressed\n");
        return false;
    }
    d.update_mouse(px, py, false);

    // Recent tile: seed the single launch-history owner (the strip reads it
    // live — no menu reopen needed) and hold the first tile.
    d.app_reg.record_launch(crate::util::app_catalog::AppId(0));
    let rx0 = layout::menu_recent_x0(menu_r);
    let tile = layout::menu_recent_rect(menu_r, rx0);
    let (qx, qy) = (tile.x + tile.w as i32 / 2, tile.y + tile.h as i32 / 2);
    d.update_mouse(qx, qy, false);
    let snap = d.snapshot();
    if snap.hover != Some(HoverTarget::StartRecent(0)) || snap.mouse_down {
        io::print_str("[test] FAIL test_start_menu_pressed: recent hover-only reported pressed\n");
        return false;
    }
    d.update_mouse(qx, qy, true);
    let snap = d.snapshot();
    if snap.hover != Some(HoverTarget::StartRecent(0)) || !snap.mouse_down {
        io::print_str("[test] FAIL test_start_menu_pressed: held recent tile not pressed\n");
        return false;
    }
    d.update_mouse(qx, qy, false);

    io::print_str("[test] PASS test_start_menu_pressed\n");
    true
}
