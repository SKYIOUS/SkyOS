//! Overlay action tests (Phase 3.5 item 1): `hit_test_action` mapping pins
//! plus end-to-end `handle_click` dispatch for the overlays that have hit
//! regions (legacy settings, settings app, task manager). About is
//! dismiss-only — it closes on any click without an action round-trip, so
//! only its end-to-end behavior is pinned. All coordinates below are derived
//! from the app modules' own draw/hit geometry for a fresh 800×600 desktop —
//! changing a layout constant must change these, not the Desktop
//! coordinator.

use crate::apps::settings::SettingsPage;
use crate::core::desktop::Desktop;
use crate::core::window::{AppWindow, HoverTarget};
use libsarga::io;

// ---------------------------------------------------------------------------
// Geometry helpers (mirror the hit-test math in each app module)
// ---------------------------------------------------------------------------

/// Legacy settings panel (core/settings.rs): 320×200 centered on 800×600.
fn legacy_row_center(row: usize) -> (i32, i32) {
    // py = (600 - 200) / 3 = 133; rows at py + 36 + row*32, 28 tall.
    let py = 133;
    (400, py + 36 + row as i32 * 32 + 14)
}

fn legacy_close_center() -> (i32, i32) {
    // cy = py + ph - 36, 28 tall.
    let py = 133;
    (400, py + 200 - 36 + 14)
}

/// Settings app (apps/settings.rs): 560×400 centered on 800×600.
fn app_page_center(row: usize) -> (i32, i32) {
    // px = 120, py = 66; sidebar_x = px + 4, sidebar_y = py + 32;
    // row iy = sidebar_y + 4 + row*28, 24 tall, x in [sidebar_x+4, sidebar_x+136].
    let py = 66;
    let iy = py + 32 + 4 + row as i32 * 28;
    (194, iy + 12)
}

fn app_toggle_center() -> (i32, i32) {
    // toggle_y = py + 32 + 36, 28 tall, x in [px+8, px+544].
    let py = 66;
    (400, py + 32 + 36 + 14)
}

/// Task manager (apps/task_manager.rs): list_y = py + 32 + 22, rows 20 tall.
fn tm_row_center(row: usize) -> (i32, i32) {
    let py = (600 - 360) / 3;
    (400, py + 32 + 22 + row as i32 * 20 + 10)
}

// ---------------------------------------------------------------------------
// hit_test_action mapping pins
// ---------------------------------------------------------------------------

fn pins_legacy_settings() -> bool {
    let d = Desktop::new(800, 600);
    let snap = d.snapshot();
    use crate::apps::AppAction;
    let (rx, ry) = legacy_row_center(0);
    if d.settings.hit_test_action(rx, ry, &snap) != Some(AppAction::ToggleSound) {
        io::print_str("[test] FAIL pins_legacy_settings: row 0 != ToggleSound\n");
        return false;
    }
    let (rx, ry) = legacy_row_center(1);
    // theme_dark defaults true → the toggle computes the *new* state: false.
    if d.settings.hit_test_action(rx, ry, &snap) != Some(AppAction::SetTheme(false)) {
        io::print_str("[test] FAIL pins_legacy_settings: row 1 != SetTheme(false)\n");
        return false;
    }
    let (cx, cy) = legacy_close_center();
    if d.settings.hit_test_action(cx, cy, &snap) != Some(AppAction::Close) {
        io::print_str("[test] FAIL pins_legacy_settings: close != Close\n");
        return false;
    }
    if d.settings.hit_test_action(100, 100, &snap).is_some() {
        io::print_str("[test] FAIL pins_legacy_settings: outside hit != None\n");
        return false;
    }
    io::print_str("[test] PASS pins_legacy_settings\n");
    true
}

fn pins_settings_app() -> bool {
    let d = Desktop::new(800, 600);
    let snap = d.snapshot();
    use crate::apps::AppAction;
    let (px0, py0) = app_page_center(0);
    if d.settings_app.hit_test_action(px0, py0, &snap)
        != Some(AppAction::SelectPage(SettingsPage::Appearance))
    {
        io::print_str("[test] FAIL pins_settings_app: row 0 != SelectPage(Appearance)\n");
        return false;
    }
    let (px1, py1) = app_page_center(1);
    if d.settings_app.hit_test_action(px1, py1, &snap)
        != Some(AppAction::SelectPage(SettingsPage::Desktop))
    {
        io::print_str("[test] FAIL pins_settings_app: row 1 != SelectPage(Desktop)\n");
        return false;
    }
    let (tx, ty) = app_toggle_center();
    // app flag defaults true → the toggle computes the new state: false.
    if d.settings_app.hit_test_action(tx, ty, &snap) != Some(AppAction::SetTheme(false)) {
        io::print_str("[test] FAIL pins_settings_app: theme toggle != SetTheme(false)\n");
        return false;
    }
    if d.settings_app.hit_test_action(700, 500, &snap).is_some() {
        io::print_str("[test] FAIL pins_settings_app: outside hit != None\n");
        return false;
    }
    io::print_str("[test] PASS pins_settings_app\n");
    true
}

fn pins_task_manager() -> bool {
    let mut d = Desktop::new(800, 600);
    // A row target requires at least one window in the snapshot.
    d.wm.create(AppWindow::new(100, 100, 300, 200, "ProcA"));
    let snap = d.snapshot();
    use crate::apps::AppAction;
    let (rx, ry) = tm_row_center(0);
    if d.task_manager.hit_test_action(rx, ry, &snap) != Some(AppAction::FocusWindow(0)) {
        io::print_str("[test] FAIL pins_task_manager: row 0 != FocusWindow(0)\n");
        return false;
    }
    if d.task_manager.hit_test_action(700, 500, &snap).is_some() {
        io::print_str("[test] FAIL pins_task_manager: outside hit != None\n");
        return false;
    }
    io::print_str("[test] PASS pins_task_manager\n");
    true
} // ---------------------------------------------------------------------------
  // End-to-end handle_click dispatch
  // ---------------------------------------------------------------------------

fn e2e_legacy_settings() -> bool {
    let mut d = Desktop::new(800, 600);
    d.settings.open = true;

    // Sound row: flips and keeps the panel open.
    let (rx, ry) = legacy_row_center(0);
    d.handle_click(rx, ry);
    if d.settings.sound_on {
        io::print_str("[test] FAIL e2e_legacy_settings: sound not toggled off\n");
        return false;
    }
    if !d.settings.open {
        io::print_str("[test] FAIL e2e_legacy_settings: row click closed panel\n");
        return false;
    }

    // Theme row: flips the flag AND the live theme (via toggle_theme).
    let (rx, ry) = legacy_row_center(1);
    d.handle_click(rx, ry);
    if d.settings.theme_dark {
        io::print_str("[test] FAIL e2e_legacy_settings: theme flag not flipped\n");
        return false;
    }
    if d.theme_svc.current().bg_primary != libsarga::theme::Theme::light().bg_primary {
        io::print_str("[test] FAIL e2e_legacy_settings: theme service not switched to light\n");
        return false;
    }

    // Close button: closes the panel (and clears any context menu).
    d.context_menu = Some(crate::core::geometry::ContextMenu {
        x: 0,
        y: 0,
        items: &[],
    });
    let (cx, cy) = legacy_close_center();
    d.handle_click(cx, cy);
    if d.settings.open || d.context_menu.is_some() {
        io::print_str("[test] FAIL e2e_legacy_settings: close did not close panel\n");
        return false;
    }

    // Click-outside closes the panel.
    d.settings.open = true;
    d.handle_click(100, 100);
    if d.settings.open {
        io::print_str("[test] FAIL e2e_legacy_settings: outside click did not close panel\n");
        return false;
    }
    io::print_str("[test] PASS e2e_legacy_settings\n");
    true
}

fn e2e_settings_app() -> bool {
    let mut d = Desktop::new(800, 600);
    d.settings_app.open = true;

    // Theme toggle (Appearance page is the default): flips flag + live theme.
    let (tx, ty) = app_toggle_center();
    d.handle_click(tx, ty);
    if d.settings_app.app {
        io::print_str("[test] FAIL e2e_settings_app: theme flag not flipped\n");
        return false;
    }
    if d.theme_svc.current().bg_primary != libsarga::theme::Theme::light().bg_primary {
        io::print_str("[test] FAIL e2e_settings_app: theme service not switched to light\n");
        return false;
    }

    // Sidebar row 1 → Desktop page (still open).
    let (px1, py1) = app_page_center(1);
    d.handle_click(px1, py1);
    if d.settings_app.current_page != SettingsPage::Desktop {
        io::print_str("[test] FAIL e2e_settings_app: sidebar did not switch page\n");
        return false;
    }
    if !d.settings_app.open {
        io::print_str("[test] FAIL e2e_settings_app: page click closed overlay\n");
        return false;
    }

    // Click-outside closes.
    d.handle_click(700, 500);
    if d.settings_app.open {
        io::print_str("[test] FAIL e2e_settings_app: outside click did not close overlay\n");
        return false;
    }
    io::print_str("[test] PASS e2e_settings_app\n");
    true
}

fn e2e_task_manager() -> bool {
    let mut d = Desktop::new(800, 600);
    let id_a = d.wm.create(AppWindow::new(100, 100, 300, 200, "ProcA"));
    let id_b = d.wm.create(AppWindow::new(200, 200, 300, 200, "ProcB"));
    if d.wm.active() != Some(id_b) {
        io::print_str("[test] FAIL e2e_task_manager: window B should be active\n");
        return false;
    }
    d.task_manager.open = true;

    // Row 0: selects window A and brings it to front.
    let (rx, ry) = tm_row_center(0);
    d.handle_click(rx, ry);
    if d.task_manager.selected != 0 {
        io::print_str("[test] FAIL e2e_task_manager: selection not updated\n");
        return false;
    }
    if d.wm.active() != Some(id_a) {
        io::print_str("[test] FAIL e2e_task_manager: row click did not focus window A\n");
        return false;
    }

    // Click-outside closes.
    d.handle_click(700, 500);
    if d.task_manager.open {
        io::print_str("[test] FAIL e2e_task_manager: outside click did not close overlay\n");
        return false;
    }
    io::print_str("[test] PASS e2e_task_manager\n");
    true
}

/// The unified hover state drives the modal panels too: hovering a legacy
/// settings row / settings-app toggle / task-manager row reports the panel
/// variant through `Desktop::hover_target` (computed once per frame, not by
/// the panels hit-testing the mouse themselves), with the same pressed
/// combos as the window buttons. Also pins the guard rails: a pointer on an
/// open panel's non-row area is None (the overlay-open guard then silences
/// everything beneath), and the settings-app toggle stops reporting once the
/// page no longer draws it.
fn pins_overlay_hover() -> bool {
    // Legacy settings rows + Close.
    let mut d = Desktop::new(800, 600);
    d.settings.open = true;
    let (rx, ry) = legacy_row_center(0);
    d.update_mouse(rx, ry, false);
    let snap = d.snapshot();
    if snap.hover != Some(HoverTarget::SettingsRow(0)) || snap.mouse_down {
        io::print_str("[test] FAIL pins_overlay_hover: legacy row 0 hover\n");
        return false;
    }
    d.update_mouse(rx, ry, true); // hold: hover + pressed
    let snap = d.snapshot();
    if snap.hover != Some(HoverTarget::SettingsRow(0)) || !snap.mouse_down {
        io::print_str("[test] FAIL pins_overlay_hover: legacy row 0 pressed\n");
        return false;
    }
    d.update_mouse(rx, ry, false); // release: hover stays, pressed clears
    let snap = d.snapshot();
    if snap.hover != Some(HoverTarget::SettingsRow(0)) || snap.mouse_down {
        io::print_str("[test] FAIL pins_overlay_hover: legacy row 0 release\n");
        return false;
    }
    let (cx, cy) = legacy_close_center();
    d.update_mouse(cx, cy, false);
    if d.snapshot().hover != Some(HoverTarget::SettingsRow(2)) {
        io::print_str("[test] FAIL pins_overlay_hover: legacy Close row hover\n");
        return false;
    }
    // Panel background (inside the panel, off any row): None, and nothing
    // beneath leaks through while the panel is open.
    d.update_mouse(400, 240, false);
    if d.snapshot().hover.is_some() {
        io::print_str("[test] FAIL pins_overlay_hover: panel background hovered\n");
        return false;
    }
    // Settings-app toggle (Appearance page is the default) + pressed.
    let mut d = Desktop::new(800, 600);
    d.settings_app.open = true;
    let (tx, ty) = app_toggle_center();
    d.update_mouse(tx, ty, false);
    if d.snapshot().hover != Some(HoverTarget::SettingsAppRow(0)) {
        io::print_str("[test] FAIL pins_overlay_hover: settings-app toggle hover\n");
        return false;
    }
    d.update_mouse(tx, ty, true);
    let snap = d.snapshot();
    if snap.hover != Some(HoverTarget::SettingsAppRow(0)) || !snap.mouse_down {
        io::print_str("[test] FAIL pins_overlay_hover: settings-app toggle pressed\n");
        return false;
    }
    // Non-Appearance page: the toggle is not drawn, so its rect is dead space.
    d.settings_app.current_page = SettingsPage::About;
    d.update_mouse(tx, ty, false);
    if d.snapshot().hover.is_some() {
        io::print_str("[test] FAIL pins_overlay_hover: toggle hovered off Appearance\n");
        return false;
    }
    // Task-manager rows + pressed.
    let mut d = Desktop::new(800, 600);
    d.wm.create(AppWindow::new(100, 100, 300, 200, "ProcA"));
    d.task_manager.open = true;
    let (rx, ry) = tm_row_center(0);
    d.update_mouse(rx, ry, false);
    let snap = d.snapshot();
    if snap.hover != Some(HoverTarget::TaskManagerRow(0)) || snap.mouse_down {
        io::print_str("[test] FAIL pins_overlay_hover: task-manager row 0 hover\n");
        return false;
    }
    d.update_mouse(rx, ry, true);
    let snap = d.snapshot();
    if snap.hover != Some(HoverTarget::TaskManagerRow(0)) || !snap.mouse_down {
        io::print_str("[test] FAIL pins_overlay_hover: task-manager row 0 pressed\n");
        return false;
    }
    io::print_str("[test] PASS pins_overlay_hover\n");
    true
}

fn e2e_about() -> bool {
    let mut d = Desktop::new(800, 600);
    d.about_state.open = true;
    d.handle_click(400, 300);
    if d.about_state.open {
        io::print_str("[test] FAIL e2e_about: click did not close about\n");
        return false;
    }
    io::print_str("[test] PASS e2e_about\n");
    true
}

pub(crate) fn test_overlay_actions(_desktop: &mut Desktop) -> bool {
    let mut ok = true;
    ok &= pins_legacy_settings();
    ok &= pins_settings_app();
    ok &= pins_task_manager();
    ok &= pins_overlay_hover();
    ok &= e2e_legacy_settings();
    ok &= e2e_settings_app();
    ok &= e2e_task_manager();
    ok &= e2e_about();
    io::print_str(if ok {
        "[test] PASS test_overlay_actions\n"
    } else {
        "[test] FAIL test_overlay_actions\n"
    });
    ok
}
