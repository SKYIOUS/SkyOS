//! A11y tree activation tests — the Close button must close the window that
//! owns it (keyboard/a11y users close windows like the mouse can), the Start
//! button (Button role with the sentinel owner) toggles the start menu,
//! activating a Window node brings it to front via its owner stamp
//! (replacing the dead title-as-index parse), and taskbar window buttons
//! bring their window to front (restoring it first if minimized) — mirroring
//! a taskbar mouse click, distinguished from Close by tree structure
//! (parent role), not label.
//!
//! Each sub-case runs on its own fresh `Desktop` so the tree and focus state
//! stay deterministic and can't leak into the shared desktop from `run_all`.

use crate::apps::tooltip::{TooltipManager, TooltipOwner};
use crate::core::desktop::Desktop;
use crate::core::event::Event;
use crate::core::geometry::{ContextMenu, Rect};
use crate::core::window::{
    AppWindow, HoverTarget, WindowButton, WindowId, WindowState, START_BUTTON_OWNER,
    TRAY_PANEL_OWNER,
};
use crate::input::keys;
use crate::layout;
use crate::render::snapshot::RenderSnapshot;
use crate::sec::a11y::{A11yRole, A11yTree, FocusManager};
use crate::util::app_catalog::AppId;
use alloc::vec::Vec;
use libsarga::io;

/// Log one node-activation sub-case's result to serial. The aggregate
/// `[test] PASS/FAIL <name>` line is per test; these per-step lines let a
/// QEMU serial log pinpoint exactly which activation succeeded before a
/// failure (or a panic) later in the test.
fn a11y_log_pass(test: &str, step: &str) {
    io::print_str(&alloc::format!("[a11y] {} {}: PASS\n", test, step));
}

pub(crate) fn test_a11y_close_button() -> bool {
    // Create a window, build the tree, and pin the Close node's owner.
    let mut d = Desktop::new(800, 600);
    let before = d.wm.len();
    let wid = d.wm.create(AppWindow::new(100, 100, 400, 300, "CloseMe"));
    d.tick(); // build_a11y_tree runs inside tick

    let close = d
        .a11y_tree
        .nodes
        .iter()
        .find(|n| n.role == A11yRole::Button && n.label == "Close");
    let Some(close) = close else {
        io::print_str("[test] FAIL test_a11y_close_button: no Close node in tree\n");
        return false;
    };
    if close.owner != Some(wid) {
        io::print_str("[test] FAIL test_a11y_close_button: Close node owner wrong\n");
        return false;
    }

    // Focus + activate (Enter) must close the owning window. `wm.close` is
    // animated, so drain it before counting (settle idiom).
    d.focus.focus(close.id);
    d.handle_event(Event::Key(keys::SCAN_ENTER as u16));
    for _ in 0..60 {
        d.tick();
    }
    if d.wm.len() != before {
        io::print_str("[test] FAIL test_a11y_close_button: Close did not remove window\n");
        return false;
    }
    a11y_log_pass("test_a11y_close_button", "Close closes");

    // Minimize control: a Button child of the Window node labeled
    // "Minimize", owner-stamped. Enter minimizes the owning window (state
    // leaves Normal for Minimized) and never closes it — mirroring the
    // mouse min button, the same way Close mirrors the close button.
    let mut d = Desktop::new(800, 600);
    let wid3 = d.wm.create(AppWindow::new(100, 100, 400, 300, "MinMe"));
    d.tick();
    let min = d
        .a11y_tree
        .nodes
        .iter()
        .find(|n| n.role == A11yRole::Button && n.label == "Minimize" && n.owner == Some(wid3));
    let Some(min) = min else {
        io::print_str("[test] FAIL test_a11y_close_button: no Minimize node in tree\n");
        return false;
    };
    if !min
        .parent
        .and_then(|p| d.a11y_tree.nodes.iter().find(|m| m.id == p))
        .is_some_and(|p| p.role == A11yRole::Window)
    {
        io::print_str("[test] FAIL test_a11y_close_button: Minimize node not Window child\n");
        return false;
    }
    let min_id = min.id; // copy: `min` borrows the tree, and the next
                         // `handle_event` needs `&mut d`.
    d.focus.focus(min_id);
    d.handle_event(Event::Key(keys::SCAN_ENTER as u16));
    if !d
        .wm
        .lookup(wid3)
        .is_some_and(|w| w.state == WindowState::Minimized)
    {
        io::print_str("[test] FAIL test_a11y_close_button: Minimize did not minimize\n");
        return false;
    }
    if d.wm.lookup(wid3).is_none() {
        io::print_str("[test] FAIL test_a11y_close_button: Minimize closed the window\n");
        return false;
    }
    a11y_log_pass("test_a11y_close_button", "Minimize minimizes");

    // Re-activating the Minimize control of an already-minimized window is
    // a no-op: state stays Minimized (never a toggle, never a close), and
    // the ring survives — the window stays in the wm, so its chrome node
    // persists in the rebuilt tree and the focused id stays valid.
    d.handle_event(Event::Key(keys::SCAN_ENTER as u16));
    if !d
        .wm
        .lookup(wid3)
        .is_some_and(|w| w.state == WindowState::Minimized)
    {
        io::print_str("[test] FAIL test_a11y_close_button: re-minimize toggled state\n");
        return false;
    }
    if !d
        .a11y_tree
        .nodes
        .iter()
        .any(|n| n.id == min_id && n.focusable && n.state.visible)
    {
        io::print_str("[test] FAIL test_a11y_close_button: ring died after minimize\n");
        return false;
    }
    a11y_log_pass("test_a11y_close_button", "re-minimize no-op, ring survives");

    // A window literally titled "Minimize" keeps the two semantics apart by
    // tree structure, not label (the Close-titled mirror of
    // test_a11y_taskbar_button): its taskbar button (Button child of the
    // Taskbar) brings it to front without minimizing, while its Window-child
    // Minimize control still minimizes it.
    let mut d = Desktop::new(800, 600);
    let a = d.wm.create(AppWindow::new(50, 50, 300, 200, "Minimize"));
    let _b = d.wm.create(AppWindow::new(200, 200, 300, 200, "WinB"));
    d.tick();
    let taskbar_btn = d.a11y_tree.nodes.iter().find(|n| {
        n.role == A11yRole::Button
            && n.label == "Minimize"
            && n.owner == Some(a)
            && n.parent
                .and_then(|p| d.a11y_tree.nodes.iter().find(|m| m.id == p))
                .is_some_and(|p| p.role == A11yRole::Taskbar)
    });
    let Some(taskbar_btn) = taskbar_btn else {
        io::print_str("[test] FAIL test_a11y_close_button: no 'Minimize'-titled taskbar node\n");
        return false;
    };
    d.focus.focus(taskbar_btn.id);
    d.handle_event(Event::Key(keys::SCAN_ENTER as u16));
    if d.wm
        .lookup(a)
        .is_none_or(|w| w.state == WindowState::Minimized)
    {
        io::print_str(
            "[test] FAIL test_a11y_close_button: 'Minimize'-titled taskbar minimized A\n",
        );
        return false;
    }
    if d.wm.active() != Some(a) {
        io::print_str(
            "[test] FAIL test_a11y_close_button: 'Minimize'-titled taskbar did not focus A\n",
        );
        return false;
    }
    // Its Window-child Minimize control still minimizes it.
    let min_btn = d.a11y_tree.nodes.iter().find(|n| {
        n.role == A11yRole::Button
            && n.label == "Minimize"
            && n.owner == Some(a)
            && n.parent
                .and_then(|p| d.a11y_tree.nodes.iter().find(|m| m.id == p))
                .is_some_and(|p| p.role == A11yRole::Window)
    });
    let Some(min_btn) = min_btn else {
        io::print_str("[test] FAIL test_a11y_close_button: no Minimize control for A\n");
        return false;
    };
    d.focus.focus(min_btn.id);
    d.handle_event(Event::Key(keys::SCAN_ENTER as u16));
    if !d
        .wm
        .lookup(a)
        .is_some_and(|w| w.state == WindowState::Minimized)
    {
        io::print_str("[test] FAIL test_a11y_close_button: Minimize control did not minimize A\n");
        return false;
    }
    a11y_log_pass("test_a11y_close_button", "Minimize-titled discrimination");

    // Start button: Button-role with the sentinel owner. Activating it with
    // Enter toggles the start menu open (the same action closes it again),
    // mirroring a mouse click on the button — and never touches a window.
    let mut d = Desktop::new(800, 600);
    let wid2 = d.wm.create(AppWindow::new(100, 100, 400, 300, "KeepMe"));
    d.tick();
    let start = d
        .a11y_tree
        .nodes
        .iter()
        .find(|n| n.role == A11yRole::Button && n.label == "Start");
    let Some(start) = start else {
        io::print_str("[test] FAIL test_a11y_close_button: no Start node in tree\n");
        return false;
    };
    if start.owner != Some(START_BUTTON_OWNER) {
        io::print_str("[test] FAIL test_a11y_close_button: Start button owner stamp wrong\n");
        return false;
    }
    if d.start_menu.open {
        io::print_str("[test] FAIL test_a11y_close_button: start menu already open\n");
        return false;
    }
    // Enter opens the menu…
    d.focus.focus(start.id);
    d.handle_event(Event::Key(keys::SCAN_ENTER as u16));
    if !d.start_menu.open {
        io::print_str("[test] FAIL test_a11y_close_button: Enter did not open start menu\n");
        return false;
    }
    a11y_log_pass("test_a11y_close_button", "Start opens menu");
    // …and the same action closes it (toggle).
    d.handle_event(Event::Key(keys::SCAN_ENTER as u16));
    if d.start_menu.open {
        io::print_str("[test] FAIL test_a11y_close_button: Enter did not close start menu\n");
        return false;
    }
    a11y_log_pass("test_a11y_close_button", "Start toggles closed");
    // …and neither activation ever closes a window.
    for _ in 0..60 {
        d.tick();
    }
    if d.wm.lookup(wid2).is_none() {
        io::print_str("[test] FAIL test_a11y_close_button: Start button closed a window\n");
        return false;
    }

    // Window-node activation brings the owning window to front (the owner
    // stamp replaces the old title-as-index parse, which always no-oped on
    // real titles). After creating A then B, B is active; activating A's
    // Window node must make A active again.
    let mut d = Desktop::new(800, 600);
    let a = d.wm.create(AppWindow::new(50, 50, 300, 200, "WinA"));
    let _b = d.wm.create(AppWindow::new(200, 200, 300, 200, "WinB"));
    d.tick();
    let win_a = d
        .a11y_tree
        .nodes
        .iter()
        .find(|n| n.role == A11yRole::Window && n.owner == Some(a));
    let Some(win_a) = win_a else {
        io::print_str("[test] FAIL test_a11y_close_button: no Window node for A\n");
        return false;
    };
    d.focus.focus(win_a.id);
    d.handle_event(Event::Key(keys::SCAN_ENTER as u16));
    if d.wm.active() != Some(a) {
        io::print_str("[test] FAIL test_a11y_close_button: Window activation did not focus A\n");
        return false;
    }
    a11y_log_pass("test_a11y_close_button", "Window brings to front");

    io::print_str("[test] PASS test_a11y_close_button\n");
    true
}

/// The a11y tree models the start menu's app rows: one focusable Button
/// child of the StartMenu node per VISIBLE row (same scroll-aware range and
/// `menu_item_rect` bounds as the draw and `menu_hover_at`), and activating
/// a row with Enter launches its app — closing the menu like a mouse click
/// on the row. The About row is the deterministic launch target (no fork:
/// launching it opens the about overlay instead of spawning a window).
pub(crate) fn test_a11y_start_menu_rows() -> bool {
    let mut d = Desktop::new(800, 600);
    d.start_menu.open_with(&d.app_reg);
    d.tick();

    // Contract: exactly one focusable Button row per visible filtered app,
    // parented to the StartMenu node, with the shared row-rect bounds and
    // the truncated app name as the label.
    let menu_r = layout::menu_rect(d.taskbar_y());
    let list_r = layout::menu_list_rect(menu_r);
    let avail = (list_r.h / layout::MENU_ITEM_H) as usize;
    let start = d.start_menu.scroll as usize;
    let end = (start + avail).min(d.start_menu.filtered.len());
    let sm = d
        .a11y_tree
        .nodes
        .iter()
        .find(|n| n.role == A11yRole::StartMenu);
    let Some(sm) = sm else {
        io::print_str("[test] FAIL test_a11y_start_menu_rows: no StartMenu node\n");
        return false;
    };
    let sm_id = sm.id;
    let rows: Vec<u32> = d
        .a11y_tree
        .nodes
        .iter()
        .filter(|n| n.parent == Some(sm_id))
        .map(|n| n.id)
        .collect();
    if rows.len() != end - start {
        io::print_str(&alloc::format!(
            "[test] FAIL test_a11y_start_menu_rows: {} row nodes != {} visible apps\n",
            rows.len(),
            end - start
        ));
        return false;
    }
    for (k, &row_id) in rows.iter().enumerate() {
        let i = start + k;
        let n = match d.a11y_tree.nodes.iter().find(|n| n.id == row_id) {
            Some(n) => n,
            None => {
                io::print_str("[test] FAIL test_a11y_start_menu_rows: row node missing\n");
                return false;
            }
        };
        let app_id = d.start_menu.filtered[i];
        let expected_name = d
            .app_reg
            .get(app_id)
            .map(|app| layout::trunc(app.name, layout::MENU_APP_NAME_MAX))
            .unwrap_or("?");
        if !n.focusable || n.role != A11yRole::Button {
            io::print_str("[test] FAIL test_a11y_start_menu_rows: row not a focusable Button\n");
            return false;
        }
        if n.label != expected_name {
            io::print_str("[test] FAIL test_a11y_start_menu_rows: row label wrong\n");
            return false;
        }
        if n.bounds != layout::menu_item_rect(menu_r, i, start) {
            io::print_str("[test] FAIL test_a11y_start_menu_rows: row bounds wrong\n");
            return false;
        }
    }
    a11y_log_pass("test_a11y_start_menu_rows", "rows modeled per visible app");

    // Activating a row launches its app: focus the About row and press
    // Enter. The menu closes and the about overlay opens — exactly what a
    // mouse click on that row does. About SARGA is the deterministic
    // no-fork target, so the launch is pinned without spawning a window.
    // The menu is search-filtered to "about" first: the default list (26
    // apps, ~11 visible rows at MENU_H=460 / 32px pitch) scrolls catalog
    // entry 16 (About SARGA) out of view, so the row is only guaranteed
    // visible as the sole filtered match.
    let about_id = d
        .app_reg
        .apps
        .iter()
        .position(|a| a.name == "About SARGA")
        .map(AppId);
    let Some(about_id) = about_id else {
        io::print_str("[test] FAIL test_a11y_start_menu_rows: no About SARGA in catalog\n");
        return false;
    };
    d.start_menu.search = b"about".to_vec();
    d.start_menu.rebuild_filter(&d.app_reg);
    d.tick();
    let about_row_i = d.start_menu.filtered.iter().position(|&id| id == about_id);
    let Some(about_row_i) = about_row_i else {
        io::print_str("[test] FAIL test_a11y_start_menu_rows: About not filtered by search\n");
        return false;
    };
    let menu_r = layout::menu_rect(d.taskbar_y());
    let sm = d
        .a11y_tree
        .nodes
        .iter()
        .find(|n| n.role == A11yRole::StartMenu);
    let Some(sm) = sm else {
        io::print_str("[test] FAIL test_a11y_start_menu_rows: no StartMenu after search\n");
        return false;
    };
    let about_row = d.a11y_tree.nodes.iter().find(|n| {
        n.parent == Some(sm.id) && n.bounds == layout::menu_item_rect(menu_r, about_row_i, 0)
    });
    let Some(about_row) = about_row else {
        io::print_str("[test] FAIL test_a11y_start_menu_rows: About row node missing\n");
        return false;
    };
    d.focus.focus(about_row.id);
    d.handle_event(Event::Key(keys::SCAN_ENTER as u16));
    if !d.about_state.open {
        io::print_str("[test] FAIL test_a11y_start_menu_rows: Enter did not launch About\n");
        return false;
    }
    if d.start_menu.open {
        io::print_str("[test] FAIL test_a11y_start_menu_rows: menu stayed open after launch\n");
        return false;
    }
    if !d.wm.is_empty() {
        io::print_str("[test] FAIL test_a11y_start_menu_rows: About spawned a window\n");
        return false;
    }
    a11y_log_pass("test_a11y_start_menu_rows", "Enter launches focused row");

    // The menu closed, so the next tick rebuilds the tree WITHOUT the rows:
    // the focused row id must re-sync to a live node (the central validate
    // path), and the ring keeps bounds to draw.
    for _ in 0..60 {
        d.tick();
    }
    let fid = match d.focus.focused() {
        Some(f) => f,
        None => {
            io::print_str("[test] FAIL test_a11y_start_menu_rows: focus lost after launch\n");
            return false;
        }
    };
    if !d
        .a11y_tree
        .nodes
        .iter()
        .any(|n| n.id == fid && n.focusable && n.state.visible)
    {
        io::print_str("[test] FAIL test_a11y_start_menu_rows: focus stale after menu closed\n");
        return false;
    }
    a11y_log_pass("test_a11y_start_menu_rows", "ring survives menu close");

    io::print_str("[test] PASS test_a11y_start_menu_rows\n");
    true
}

pub(crate) fn test_a11y_taskbar_button() -> bool {
    // Taskbar window buttons carry their window's owner. Activating one
    // brings the window to front (like a taskbar mouse click), never closes
    // it.
    let mut d = Desktop::new(800, 600);
    let a = d.wm.create(AppWindow::new(50, 50, 300, 200, "WinA"));
    let _b = d.wm.create(AppWindow::new(200, 200, 300, 200, "WinB"));
    d.tick();
    let btn_a = d
        .a11y_tree
        .nodes
        .iter()
        .find(|n| n.role == A11yRole::Button && n.label == "WinA");
    let Some(btn_a) = btn_a else {
        io::print_str("[test] FAIL test_a11y_taskbar_button: no taskbar node for A\n");
        return false;
    };
    if btn_a.owner != Some(a) {
        io::print_str("[test] FAIL test_a11y_taskbar_button: taskbar node owner wrong\n");
        return false;
    }
    d.focus.focus(btn_a.id);
    d.handle_event(Event::Key(keys::SCAN_ENTER as u16));
    if d.wm.active() != Some(a) {
        io::print_str("[test] FAIL test_a11y_taskbar_button: activation did not focus A\n");
        return false;
    }
    if d.wm.lookup(a).is_none() {
        io::print_str("[test] FAIL test_a11y_taskbar_button: activation closed A\n");
        return false;
    }
    a11y_log_pass("test_a11y_taskbar_button", "taskbar brings to front");

    // A minimized window is restored (state leaves Minimized) and brought to
    // front, exactly like a taskbar click.
    let mut d = Desktop::new(800, 600);
    let a = d.wm.create(AppWindow::new(50, 50, 300, 200, "WinA"));
    d.wm.minimize(a, d.screen_w, d.taskbar_y());
    d.tick();
    if !d
        .wm
        .lookup(a)
        .is_some_and(|w| w.state == WindowState::Minimized)
    {
        io::print_str("[test] FAIL test_a11y_taskbar_button: minimize did not stick\n");
        return false;
    }
    let btn_a = d
        .a11y_tree
        .nodes
        .iter()
        .find(|n| n.role == A11yRole::Button && n.label == "WinA");
    let Some(btn_a) = btn_a else {
        io::print_str("[test] FAIL test_a11y_taskbar_button: no taskbar node (minimized)\n");
        return false;
    };
    d.focus.focus(btn_a.id);
    d.handle_event(Event::Key(keys::SCAN_ENTER as u16));
    if d.wm
        .lookup(a)
        .is_some_and(|w| w.state == WindowState::Minimized)
    {
        io::print_str("[test] FAIL test_a11y_taskbar_button: activation did not restore A\n");
        return false;
    }
    if d.wm.active() != Some(a) {
        io::print_str("[test] FAIL test_a11y_taskbar_button: restored A not focused\n");
        return false;
    }
    a11y_log_pass("test_a11y_taskbar_button", "taskbar restores minimized");

    // A window literally titled "Close" must still behave as a taskbar button
    // (bring to front, not close) because the guard is structural (parent
    // role), not label-based — while the window's own Close control still
    // closes it.
    let mut d = Desktop::new(800, 600);
    let a = d.wm.create(AppWindow::new(50, 50, 300, 200, "Close"));
    d.tick();
    let taskbar_btn = d.a11y_tree.nodes.iter().find(|n| {
        n.role == A11yRole::Button
            && n.owner == Some(a)
            && n.parent
                .and_then(|p| d.a11y_tree.nodes.iter().find(|m| m.id == p))
                .is_some_and(|p| p.role == A11yRole::Taskbar)
    });
    let Some(taskbar_btn) = taskbar_btn else {
        io::print_str("[test] FAIL test_a11y_taskbar_button: no taskbar node titled Close\n");
        return false;
    };
    d.focus.focus(taskbar_btn.id);
    d.handle_event(Event::Key(keys::SCAN_ENTER as u16));
    if d.wm.lookup(a).is_none() {
        io::print_str("[test] FAIL test_a11y_taskbar_button: 'Close'-titled taskbar closed A\n");
        return false;
    }
    if d.wm.active() != Some(a) {
        io::print_str(
            "[test] FAIL test_a11y_taskbar_button: 'Close'-titled taskbar did not focus A\n",
        );
        return false;
    }
    // The window's own Close control (Button child of the Window node) still
    // closes it.
    let close_btn = d.a11y_tree.nodes.iter().find(|n| {
        n.role == A11yRole::Button
            && n.owner == Some(a)
            && n.parent
                .and_then(|p| d.a11y_tree.nodes.iter().find(|m| m.id == p))
                .is_some_and(|p| p.role == A11yRole::Window)
    });
    let Some(close_btn) = close_btn else {
        io::print_str("[test] FAIL test_a11y_taskbar_button: no Close control for A\n");
        return false;
    };
    d.focus.focus(close_btn.id);
    d.handle_event(Event::Key(keys::SCAN_ENTER as u16));
    for _ in 0..60 {
        d.tick();
    }
    if d.wm.lookup(a).is_some() {
        io::print_str("[test] FAIL test_a11y_taskbar_button: Close control did not close A\n");
        return false;
    }
    a11y_log_pass(
        "test_a11y_taskbar_button",
        "Close-titled taskbar + Close control",
    );

    io::print_str("[test] PASS test_a11y_taskbar_button\n");
    true
}

/// Owner stamps drive tooltip labels too: hovering a window's Close button
/// for the tooltip delay shows "Close <title>" and hovering Minimize shows
/// "Minimize <title>" — driven by the unified hover state (the a11y tree
/// only models a Close node, so the Minimize button's identity comes from
/// `Desktop::hover_target`); hovering a taskbar button shows the title via
/// the owner path.
pub(crate) fn test_tooltip_owner_label() -> bool {
    let mut d = Desktop::new(800, 600);
    let _wid =
        d.wm.create(AppWindow::new(100, 100, 400, 300, "SettingsWin"));
    d.tick();
    let close = layout::close_btn_rect(100, 100, 400);
    d.update_mouse(
        close.x + close.w as i32 / 2,
        close.y + close.h as i32 / 2,
        false,
    );
    for _ in 0..40 {
        d.tick();
    }
    let tip = match &d.tooltips.active {
        Some(t) => t,
        None => {
            io::print_str("[test] FAIL test_tooltip_owner_label: no tooltip on Close hover\n");
            return false;
        }
    };
    if tip.text != "Close SettingsWin" {
        io::print_str(&alloc::format!(
            "[test] FAIL test_tooltip_owner_label: Close tooltip '{}' != 'Close <title>'\n",
            tip.text
        ));
        return false;
    }
    a11y_log_pass(
        "test_tooltip_owner_label",
        "Close hover shows 'Close <title>'",
    );

    // Minimize button: the label comes from the unified hover state (the
    // tooltip formatter names the button), not from the tree node's label.
    let mut d = Desktop::new(800, 600);
    let _wid =
        d.wm.create(AppWindow::new(100, 100, 400, 300, "SettingsWin"));
    d.tick();
    let min = layout::min_btn_rect(100, 100, 400);
    d.update_mouse(min.x + min.w as i32 / 2, min.y + min.h as i32 / 2, false);
    for _ in 0..40 {
        d.tick();
    }
    let tip = match &d.tooltips.active {
        Some(t) => t,
        None => {
            io::print_str("[test] FAIL test_tooltip_owner_label: no tooltip on Minimize hover\n");
            return false;
        }
    };
    if tip.text != "Minimize SettingsWin" {
        io::print_str(&alloc::format!(
            "[test] FAIL test_tooltip_owner_label: Minimize tooltip '{}' != 'Minimize <title>'\n",
            tip.text
        ));
        return false;
    }
    a11y_log_pass(
        "test_tooltip_owner_label",
        "Minimize hover shows 'Minimize <title>'",
    );

    // Taskbar button hover: the owner stamp resolves the same title with a
    // "Switch to" action prefix (single formatter in the a11y tree builder).
    let mut d = Desktop::new(800, 600);
    let _wid2 =
        d.wm.create(AppWindow::new(100, 100, 400, 300, "TaskbarWin"));
    d.tick();
    let ty = d.taskbar_y() as i32;
    let btn = layout::taskbar_btn_rect(0, ty as u32);
    d.update_mouse(btn.x + btn.w as i32 / 2, btn.y + btn.h as i32 / 2, false);
    for _ in 0..40 {
        d.tick();
    }
    let tip = match &d.tooltips.active {
        Some(t) => t,
        None => {
            io::print_str("[test] FAIL test_tooltip_owner_label: no tooltip on taskbar hover\n");
            return false;
        }
    };
    if tip.text != "Switch to TaskbarWin" {
        io::print_str(&alloc::format!(
            "[test] FAIL test_tooltip_owner_label: taskbar tooltip '{}' != 'Switch to <title>'\n",
            tip.text
        ));
        return false;
    }
    a11y_log_pass(
        "test_tooltip_owner_label",
        "taskbar hover shows 'Switch to <title>'",
    );

    // Minimized window's taskbar button: the action is "Restore" (a
    // taskbar click restores it, not just switches to it) — the minimized
    // state drives the label prefix through the injected lookup.
    let mut d = Desktop::new(800, 600);
    let wid = d.wm.create(AppWindow::new(100, 100, 400, 300, "MinWin"));
    d.wm.minimize(wid, d.screen_w, d.taskbar_y());
    d.tick();
    let ty = d.taskbar_y() as i32;
    let btn = layout::taskbar_btn_rect(0, ty as u32);
    d.update_mouse(btn.x + btn.w as i32 / 2, btn.y + btn.h as i32 / 2, false);
    for _ in 0..40 {
        d.tick();
    }
    let tip = match &d.tooltips.active {
        Some(t) => t,
        None => {
            io::print_str("[test] FAIL test_tooltip_owner_label: no tooltip on minimized hover\n");
            return false;
        }
    };
    if tip.text != "Restore MinWin" {
        io::print_str(&alloc::format!(
            "[test] FAIL test_tooltip_owner_label: minimized tooltip '{}' != 'Restore <title>'\n",
            tip.text
        ));
        return false;
    }
    a11y_log_pass(
        "test_tooltip_owner_label",
        "minimized hover shows 'Restore <title>'",
    );

    io::print_str("[test] PASS test_tooltip_owner_label\n");
    true
}

/// Role-aware tooltip labels all come from the single formatter in the a11y
/// tree builder (`tooltip_label`): the Start button names the action it
/// performs, and hovering a start-menu app row shows the app's description
/// (the catalog now carries one). Pins the start-menu arms the owner-label
/// test doesn't reach.
pub(crate) fn test_tooltip_role_labels() -> bool {
    // No catalog entry may carry an empty description: a placeholder `desc:
    // ""` would render as a silent empty tooltip on every row that app
    // drives. Cheap sweep over the live catalog pins the data contract.
    let d = Desktop::new(800, 600);
    for app in &d.app_reg.apps {
        if app.description.is_empty() {
            io::print_str(&alloc::format!(
                "[test] FAIL test_tooltip_role_labels: empty description for '{}'\n",
                app.name
            ));
            return false;
        }
    }
    a11y_log_pass("test_tooltip_role_labels", "all descriptions non-empty");

    // Start button: "Open Start menu" (the sentinel owner never resolves via
    // the WM, so the action-name arm must fire).
    let mut d = Desktop::new(800, 600);
    d.tick();
    let ty = d.taskbar_y() as i32;
    let sb = layout::start_btn_rect(ty as u32);
    d.update_mouse(sb.x + sb.w as i32 / 2, sb.y + sb.h as i32 / 2, false);
    for _ in 0..40 {
        d.tick();
    }
    let tip = match &d.tooltips.active {
        Some(t) => t,
        None => {
            io::print_str("[test] FAIL test_tooltip_role_labels: no tooltip on Start hover\n");
            return false;
        }
    };
    if tip.text != "Open Start menu" {
        io::print_str(&alloc::format!(
            "[test] FAIL test_tooltip_role_labels: Start tooltip '{}' != 'Open Start menu'\n",
            tip.text
        ));
        return false;
    }
    a11y_log_pass("test_tooltip_role_labels", "Start shows 'Open Start menu'");

    // Start-menu app row: hover the first visible row and expect the app's
    // description (the catalog's new `desc` field, not the app name).
    let mut d = Desktop::new(800, 600);
    d.start_menu.open_with(&d.app_reg);
    d.tick();
    let first = *d.start_menu.filtered.first().unwrap();
    let desc = alloc::string::String::from(d.app_reg.get(first).unwrap().description);
    let row = layout::menu_item_rect(layout::menu_rect(d.taskbar_y()), 0, 0);
    d.update_mouse(row.x + row.w as i32 / 2, row.y + row.h as i32 / 2, false);
    for _ in 0..40 {
        d.tick();
    }
    let tip = match &d.tooltips.active {
        Some(t) => t,
        None => {
            io::print_str("[test] FAIL test_tooltip_role_labels: no tooltip on app row\n");
            return false;
        }
    };
    if tip.text != desc {
        io::print_str(&alloc::format!(
            "[test] FAIL test_tooltip_role_labels: app tooltip '{}' != desc '{}'\n",
            tip.text,
            desc
        ));
        return false;
    }
    a11y_log_pass("test_tooltip_role_labels", "app row shows description");

    io::print_str("[test] PASS test_tooltip_role_labels\n");
    true
}

/// Closing via a11y must not leave stale focus: after activating a window's
/// Close button, `activate_a11y_node` re-syncs the FocusManager to the next
/// visible focusable node (excluding the closing window's own nodes, which
/// stay in the tree until the close animation settles), so the ring never
/// points at a node that is about to vanish.
pub(crate) fn test_a11y_close_resyncs_focus() -> bool {
    // Single window: close via its Close button, focus must move off the
    // closing node immediately and stay valid after the settle.
    let mut d = Desktop::new(800, 600);
    d.wm.create(AppWindow::new(100, 100, 400, 300, "CloseMe"));
    d.tick();
    let close = d
        .a11y_tree
        .nodes
        .iter()
        .find(|n| n.role == A11yRole::Button && n.label == "Close");
    let Some(close) = close else {
        io::print_str("[test] FAIL test_a11y_close_resyncs_focus: no Close node\n");
        return false;
    };
    let close_id = close.id;
    d.focus.focus(close_id);
    d.handle_event(Event::Key(keys::SCAN_ENTER as u16)); // activates Close -> closes
    if d.focus.focused() == Some(close_id) {
        io::print_str("[test] FAIL test_a11y_close_resyncs_focus: focus still on closing node\n");
        return false;
    }
    for _ in 0..60 {
        d.tick();
    }
    if !d.wm.is_empty() {
        io::print_str("[test] FAIL test_a11y_close_resyncs_focus: window not removed\n");
        return false;
    }
    let fid = match d.focus.focused() {
        Some(f) => f,
        None => {
            io::print_str("[test] FAIL test_a11y_close_resyncs_focus: focus lost after close\n");
            return false;
        }
    };
    if !d
        .a11y_tree
        .nodes
        .iter()
        .any(|n| n.id == fid && n.focusable && n.state.visible)
    {
        io::print_str(
            "[test] FAIL test_a11y_close_resyncs_focus: focused id not visible in rebuilt tree\n",
        );
        return false;
    }
    a11y_log_pass("test_a11y_close_resyncs_focus", "single-window re-sync");

    // Two windows: closing A must land on the SIBLING window's taskbar
    // button (B's Button child of the Taskbar — the deterministic target,
    // since the taskbar is emitted before the windows and A's nodes are
    // excluded while the animation settles), not the empty taskbar and not
    // A's own surfaces.
    let mut d = Desktop::new(800, 600);
    let a = d.wm.create(AppWindow::new(50, 50, 300, 200, "WinA"));
    let b = d.wm.create(AppWindow::new(200, 200, 300, 200, "WinB"));
    d.tick();
    let close_a = d
        .a11y_tree
        .nodes
        .iter()
        .find(|n| n.role == A11yRole::Button && n.label == "Close" && n.owner == Some(a));
    let Some(close_a) = close_a else {
        io::print_str("[test] FAIL test_a11y_close_resyncs_focus: no Close node for A\n");
        return false;
    };
    d.focus.focus(close_a.id);
    d.handle_event(Event::Key(keys::SCAN_ENTER as u16));
    let fid = match d.focus.focused() {
        Some(f) => f,
        None => {
            io::print_str("[test] FAIL test_a11y_close_resyncs_focus: focus lost (two windows)\n");
            return false;
        }
    };
    let node = d.a11y_tree.nodes.iter().find(|n| n.id == fid);
    let Some(node) = node else {
        io::print_str("[test] FAIL test_a11y_close_resyncs_focus: focused node missing\n");
        return false;
    };
    let parent_is_taskbar = node
        .parent
        .and_then(|p| d.a11y_tree.nodes.iter().find(|m| m.id == p))
        .is_some_and(|p| p.role == A11yRole::Taskbar);
    if !node.focusable || !node.state.visible || node.owner != Some(b) || !parent_is_taskbar {
        io::print_str(
            "[test] FAIL test_a11y_close_resyncs_focus: did not land on sibling taskbar button\n",
        );
        return false;
    }
    for _ in 0..60 {
        d.tick();
    }
    if d.wm.lookup(a).is_some() {
        io::print_str("[test] FAIL test_a11y_close_resyncs_focus: A not removed\n");
        return false;
    }
    if !d
        .a11y_tree
        .nodes
        .iter()
        .any(|n| n.id == fid && n.focusable && n.state.visible)
    {
        io::print_str("[test] FAIL test_a11y_close_resyncs_focus: focus stale after A removed\n");
        return false;
    }
    a11y_log_pass("test_a11y_close_resyncs_focus", "two-window re-sync");

    io::print_str("[test] PASS test_a11y_close_resyncs_focus\n");
    true
}

/// Direct pin of the owner-exclusion in `resync_after_close`. The end-to-end
/// cases can't exercise it — `build_tree` always emits the Taskbar before
/// any owner-stamped node, so the naive first-candidate is never a closing
/// window's node — so a future build-order change would otherwise land the
/// ring on a vanishing node without this guard being tested.
pub(crate) fn test_a11y_resync_exclusion() -> bool {
    // A focusable Close node owned by window 7 precedes an unrelated Start
    // node: resync must skip the Close and land on Start.
    let mut tree = A11yTree::new();
    let close = tree.add_node(A11yRole::Button, "Close", Rect::new(0, 0, 20, 20), true);
    tree.set_owner(close, WindowId(7));
    let start = tree.add_node(A11yRole::Button, "Start", Rect::new(0, 0, 50, 30), true);
    let mut f = FocusManager::new();
    f.resync_after_close(&tree, WindowId(7));
    if f.focused() != Some(start) {
        io::print_str(
            "[test] FAIL test_a11y_resync_exclusion: did not skip closed window's node\n",
        );
        return false;
    }
    a11y_log_pass("test_a11y_resync_exclusion", "skips owned node");

    // Everything owned by the closed window: blur.
    let mut tree = A11yTree::new();
    let a = tree.add_node(A11yRole::Button, "Close", Rect::new(0, 0, 20, 20), true);
    tree.set_owner(a, WindowId(7));
    let b = tree.add_node(A11yRole::Button, "Close", Rect::new(30, 0, 20, 20), true);
    tree.set_owner(b, WindowId(7));
    let mut f = FocusManager::new();
    f.resync_after_close(&tree, WindowId(7));
    if f.focused().is_some() {
        io::print_str("[test] FAIL test_a11y_resync_exclusion: expected blur\n");
        return false;
    }
    a11y_log_pass("test_a11y_resync_exclusion", "blurs when all owned");

    io::print_str("[test] PASS test_a11y_resync_exclusion\n");
    true
}

/// Central focus-lifecycle safety net (`FocusManager::validate`, run by
/// `build_tree`'s focus-sync step every frame): a focused id whose window
/// closed on ANY non-a11y path (mouse Close click, Ctrl+W, reap) either no
/// longer exists in the freshly rebuilt tree OR — because node ids are
/// positional — survives while now naming a DIFFERENT node; in both cases
/// the ring must re-sync to a live node (preferably the sibling window's
/// taskbar button) instead of silently dying or parking on an
/// arbitrary-but-valid surface. The identity half (id-reuse) is pinned by
/// the fingerprint unit leg; direct unit cases pin the re-sync/blur
/// semantics; end-to-end cases drive the two real close paths that never
/// call `resync_after_close`.
pub(crate) fn test_focus_validate_central() -> bool {
    // Unit: stale id re-syncs to the first visible focusable node. The
    // non-focusable "Gone" node proves the re-sync skips unfocusable
    // leftovers; with no owner-stamped nodes, the Pass-2 fallback lands on
    // Start — the first focusable node in tree order. (A focusable "Gone"
    // here would be the landing target, which is why it is non-focusable.)
    let mut tree = A11yTree::new();
    tree.add_node(A11yRole::Button, "Gone", Rect::new(0, 0, 20, 20), false);
    let start = tree.add_node(A11yRole::Button, "Start", Rect::new(0, 0, 50, 30), true);
    let mut f = FocusManager::new();
    f.focus(999); // node id that the rebuilt tree never emits
    f.validate(&tree, None); // unit callers pass no previous fingerprint
    if f.focused() != Some(start) {
        io::print_str("[test] FAIL test_focus_validate_central: stale id not re-synced\n");
        return false;
    }
    a11y_log_pass("test_focus_validate_central", "stale id re-syncs");

    // Unit: stale id with nothing focusable left -> blur.
    let mut tree = A11yTree::new();
    tree.add_node(
        A11yRole::Desktop,
        "Desktop",
        Rect::new(0, 0, 800, 560),
        false,
    );
    let mut f = FocusManager::new();
    f.focus(7);
    f.validate(&tree, None); // unit callers pass no previous fingerprint
    if f.focused().is_some() {
        io::print_str("[test] FAIL test_focus_validate_central: expected blur\n");
        return false;
    }
    a11y_log_pass(
        "test_focus_validate_central",
        "blurs when nothing focusable",
    );

    // Unit: a live id is left untouched.
    let mut tree = A11yTree::new();
    let live = tree.add_node(A11yRole::Button, "Keep", Rect::new(0, 0, 50, 30), true);
    let mut f = FocusManager::new();
    f.focus(live);
    f.validate(&tree, None); // unit callers pass no previous fingerprint
    if f.focused() != Some(live) {
        io::print_str("[test] FAIL test_focus_validate_central: live id disturbed\n");
        return false;
    }
    a11y_log_pass("test_focus_validate_central", "live id untouched");

    // Unit: id-reuse is detected. Node ids are POSITIONAL — assigned in
    // rebuild order — so when a window closes and a NEW tree maps the SAME
    // id to a DIFFERENT node, the previous fingerprint (owner, role, parent
    // role) no longer matches and the ring must re-sync to the sibling
    // instead of parking on whatever node now owns the id. Without this
    // check, focus would silently jump from A's taskbar button to B's
    // button (or B's chrome) with no key press.
    let mut old = A11yTree::new();
    let taskbar = old.add_node(
        A11yRole::Taskbar,
        "Taskbar",
        Rect::new(0, 540, 800, 40),
        true,
    );
    let a_btn = old.add_node(A11yRole::Button, "A", Rect::new(60, 544, 120, 32), true);
    old.set_owner(a_btn, WindowId(7));
    old.add_child(taskbar, a_btn);
    old.add_node(A11yRole::Button, "B", Rect::new(190, 544, 120, 32), true);
    let prev_fp = old
        .nodes
        .iter()
        .find(|n| n.id == a_btn)
        .map(|n| crate::sec::a11y::focus::node_fingerprint(&old, n));
    // New tree: A is gone. Slot 1 in the rebuilt tree is now the Start
    // SENTINEL (owner u64::MAX) — if the stale id were kept, the ring would
    // park on Start, which is wrong. B's button lives at id 2, deliberately
    // DIFFERENT from the stale id 1, so the assertion is on node identity
    // (re-sync must land on B's button), not on id coincidence.
    let mut new = A11yTree::new();
    let taskbar = new.add_node(
        A11yRole::Taskbar,
        "Taskbar",
        Rect::new(0, 540, 800, 40),
        true,
    );
    let start = new.add_node(A11yRole::Button, "Start", Rect::new(0, 544, 50, 32), true);
    new.set_owner(start, START_BUTTON_OWNER);
    new.add_child(taskbar, start);
    let b_btn = new.add_node(A11yRole::Button, "B", Rect::new(60, 544, 120, 32), true);
    new.set_owner(b_btn, WindowId(8));
    new.add_child(taskbar, b_btn);
    let mut f = FocusManager::new();
    f.focus(a_btn); // the OLD tree's id
    f.validate(&new, prev_fp);
    if f.focused() != Some(b_btn) {
        io::print_str(
            "[test] FAIL test_focus_validate_central: id-reuse not detected / wrong landing\n",
        );
        return false;
    }
    a11y_log_pass("test_focus_validate_central", "id-reuse detected");

    // Unit: the fingerprint-MATCH path keeps focus — an unchanged node at
    // the same id (the same owner, role, and parent role in the rebuilt
    // tree) is left untouched. The trees carry a LEADING sibling button
    // (X at id 1, owner 10) ahead of the focused button (A at id 2, owner
    // 9) so the leg discriminates: if a regression ever treated every
    // fingerprint as a mismatch, `preferred_target` would re-sync to X@1
    // (the first taskbar button) and the assertion on A@2 would fail. With
    // a single focusable node the two behaviors coincide (both land on id
    // 1) and the leg would pass either way.
    let mut old = A11yTree::new();
    let taskbar = old.add_node(
        A11yRole::Taskbar,
        "Taskbar",
        Rect::new(0, 540, 800, 40),
        true,
    );
    let x_btn = old.add_node(A11yRole::Button, "X", Rect::new(60, 544, 120, 32), true);
    old.set_owner(x_btn, WindowId(10));
    old.add_child(taskbar, x_btn);
    let a_btn = old.add_node(A11yRole::Button, "A", Rect::new(190, 544, 120, 32), true);
    old.set_owner(a_btn, WindowId(9));
    old.add_child(taskbar, a_btn);
    let prev_fp = old
        .nodes
        .iter()
        .find(|n| n.id == a_btn)
        .map(|n| crate::sec::a11y::focus::node_fingerprint(&old, n));
    let mut new = A11yTree::new();
    let taskbar = new.add_node(
        A11yRole::Taskbar,
        "Taskbar",
        Rect::new(0, 540, 800, 40),
        true,
    );
    let x_btn2 = new.add_node(A11yRole::Button, "X", Rect::new(60, 544, 120, 32), true);
    new.set_owner(x_btn2, WindowId(10));
    new.add_child(taskbar, x_btn2);
    let a_btn2 = new.add_node(A11yRole::Button, "A", Rect::new(190, 544, 120, 32), true);
    new.set_owner(a_btn2, WindowId(9));
    new.add_child(taskbar, a_btn2);
    let mut f = FocusManager::new();
    f.focus(a_btn); // A at id 2
    f.validate(&new, prev_fp);
    if f.focused() != Some(a_btn2) {
        io::print_str(
            "[test] FAIL test_focus_validate_central: fingerprint match lost focus
",
        );
        return false;
    }
    a11y_log_pass(
        "test_focus_validate_central",
        "fingerprint match keeps focus",
    );

    // End-to-end, mouse close path: focus a window's Close node, then close
    // the window via `wm.close` directly (exactly what the mouse Close click
    // handler does — NO a11y activation, so `resync_after_close` never runs).
    // After the close animation settles and `process_closing` removes the
    // window, the focused id must be re-synced to a live node, and the
    // render snapshot must still have bounds to draw the ring on.
    let mut d = Desktop::new(800, 600);
    let a = d.wm.create(AppWindow::new(50, 50, 300, 200, "WinA"));
    let b = d.wm.create(AppWindow::new(200, 200, 300, 200, "WinB"));
    d.tick();
    let close_a = d
        .a11y_tree
        .nodes
        .iter()
        .find(|n| n.role == A11yRole::Button && n.label == "Close" && n.owner == Some(a));
    let Some(close_a) = close_a else {
        io::print_str("[test] FAIL test_focus_validate_central: no Close node for A\n");
        return false;
    };
    let stale_id = close_a.id;
    d.focus.focus(stale_id);
    d.wm.close(a); // mouse Close click path
    for _ in 0..60 {
        d.tick();
    }
    if d.wm.lookup(a).is_some() {
        io::print_str("[test] FAIL test_focus_validate_central: mouse close did not remove A\n");
        return false;
    }
    let fid = match d.focus.focused() {
        Some(id) => id,
        None => {
            io::print_str("[test] FAIL test_focus_validate_central: mouse close lost focus\n");
            return false;
        }
    };
    // The ring must land on the sibling window's surface (B's taskbar
    // button, owner == Some(b)) — not on the empty taskbar, which is the
    // Pass-2 fallback reserved for when no sibling window remains.
    if !d
        .a11y_tree
        .nodes
        .iter()
        .any(|n| n.id == fid && n.focusable && n.state.visible && n.owner == Some(b))
    {
        io::print_str(
            "[test] FAIL test_focus_validate_central: mouse close did not land on sibling window\n",
        );
        return false;
    }
    if RenderSnapshot::from(&d).focused_bounds.is_none() {
        io::print_str("[test] FAIL test_focus_validate_central: mouse close ring has no bounds\n");
        return false;
    }
    a11y_log_pass("test_focus_validate_central", "mouse close re-syncs");

    // End-to-end, Ctrl+W path: focus the window's own node, then close the
    // active window via the CloseFocused key action (Ctrl+W). Same contract:
    // the ring survives on a live node.
    let mut d = Desktop::new(800, 600);
    let a = d.wm.create(AppWindow::new(50, 50, 300, 200, "WinA"));
    d.tick();
    let win_a = d
        .a11y_tree
        .nodes
        .iter()
        .find(|n| n.role == A11yRole::Window && n.owner == Some(a));
    let Some(win_a) = win_a else {
        io::print_str("[test] FAIL test_focus_validate_central: no Window node for A\n");
        return false;
    };
    d.focus.focus(win_a.id);
    d.handle_key_event(crate::input::KeyEvent::from_byte(23)); // Ctrl+W
    for _ in 0..60 {
        d.tick();
    }
    if !d.wm.is_empty() {
        io::print_str("[test] FAIL test_focus_validate_central: Ctrl+W did not close A\n");
        return false;
    }
    let fid = match d.focus.focused() {
        Some(id) => id,
        None => {
            io::print_str("[test] FAIL test_focus_validate_central: Ctrl+W lost focus\n");
            return false;
        }
    };
    if !d
        .a11y_tree
        .nodes
        .iter()
        .any(|n| n.id == fid && n.focusable && n.state.visible && n.owner != Some(a))
    {
        io::print_str("[test] FAIL test_focus_validate_central: Ctrl+W left stale focus\n");
        return false;
    }
    if RenderSnapshot::from(&d).focused_bounds.is_none() {
        io::print_str("[test] FAIL test_focus_validate_central: Ctrl+W ring has no bounds\n");
        return false;
    }
    a11y_log_pass("test_focus_validate_central", "Ctrl+W re-syncs");

    io::print_str("[test] PASS test_focus_validate_central\n");
    true
}

/// An open overlay consumes a11y activation first — mouse-click semantics.
/// `handle_click` checks the overlays before the taskbar and windows, so a
/// click on a taskbar button or Close control with a modal up only dismisses
/// the modal; keyboard activation mirrors that: the first Enter closes the
/// overlay and is consumed, the next Enter acts on the still-focused node.
pub(crate) fn test_a11y_activation_dismisses_overlays() -> bool {
    // Legacy settings panel up + Close focused: Enter closes settings, the
    // window behind it stays (activation never acts through a modal).
    let mut d = Desktop::new(800, 600);
    let wid = d.wm.create(AppWindow::new(100, 100, 400, 300, "KeepMe"));
    d.settings.open = true;
    d.tick();
    let close = d
        .a11y_tree
        .nodes
        .iter()
        .find(|n| n.role == A11yRole::Button && n.label == "Close");
    let Some(close) = close else {
        io::print_str("[test] FAIL test_a11y_activation_dismisses_overlays: no Close node\n");
        return false;
    };
    d.focus.focus(close.id);
    d.handle_event(Event::Key(keys::SCAN_ENTER as u16));
    if d.settings.open {
        io::print_str(
            "[test] FAIL test_a11y_activation_dismisses_overlays: settings not dismissed\n",
        );
        return false;
    }
    if d.wm.lookup(wid).is_none() {
        io::print_str(
            "[test] FAIL test_a11y_activation_dismisses_overlays: Close acted through settings\n",
        );
        return false;
    }
    a11y_log_pass(
        "test_a11y_activation_dismisses_overlays",
        "settings dismissed",
    );

    // Settings app up + taskbar button focused: Enter closes it, the window
    // is not brought to front.
    let mut d = Desktop::new(800, 600);
    let a = d.wm.create(AppWindow::new(50, 50, 300, 200, "WinA"));
    let _b = d.wm.create(AppWindow::new(200, 200, 300, 200, "WinB"));
    d.settings_app.open = true;
    d.tick();
    let btn_a = d
        .a11y_tree
        .nodes
        .iter()
        .find(|n| n.role == A11yRole::Button && n.label == "WinA");
    let Some(btn_a) = btn_a else {
        io::print_str(
            "[test] FAIL test_a11y_activation_dismisses_overlays: no taskbar node for A\n",
        );
        return false;
    };
    d.focus.focus(btn_a.id);
    d.handle_event(Event::Key(keys::SCAN_ENTER as u16));
    if d.settings_app.open {
        io::print_str(
            "[test] FAIL test_a11y_activation_dismisses_overlays: settings app not dismissed\n",
        );
        return false;
    }
    if d.wm.active() == Some(a) {
        io::print_str(
            "[test] FAIL test_a11y_activation_dismisses_overlays: taskbar acted through modal\n",
        );
        return false;
    }
    a11y_log_pass(
        "test_a11y_activation_dismisses_overlays",
        "settings app dismissed",
    );

    // About up + taskbar button: Enter closes about, window not focused.
    let mut d = Desktop::new(800, 600);
    let a = d.wm.create(AppWindow::new(50, 50, 300, 200, "WinA"));
    let _b = d.wm.create(AppWindow::new(200, 200, 300, 200, "WinB"));
    d.about_state.open = true;
    d.tick();
    let btn_a = d
        .a11y_tree
        .nodes
        .iter()
        .find(|n| n.role == A11yRole::Button && n.label == "WinA");
    let Some(btn_a) = btn_a else {
        io::print_str(
            "[test] FAIL test_a11y_activation_dismisses_overlays: no taskbar node (about)\n",
        );
        return false;
    };
    d.focus.focus(btn_a.id);
    d.handle_event(Event::Key(keys::SCAN_ENTER as u16));
    if d.about_state.open {
        io::print_str("[test] FAIL test_a11y_activation_dismisses_overlays: about not dismissed\n");
        return false;
    }
    if d.wm.active() == Some(a) {
        io::print_str(
            "[test] FAIL test_a11y_activation_dismisses_overlays: taskbar acted through about\n",
        );
        return false;
    }
    a11y_log_pass("test_a11y_activation_dismisses_overlays", "about dismissed");

    // Start menu up + Close focused: Enter closes the menu, window stays.
    let mut d = Desktop::new(800, 600);
    let wid = d.wm.create(AppWindow::new(100, 100, 400, 300, "KeepMe"));
    d.start_menu.open_with(&d.app_reg);
    d.tick();
    let close = d
        .a11y_tree
        .nodes
        .iter()
        .find(|n| n.role == A11yRole::Button && n.label == "Close");
    let Some(close) = close else {
        io::print_str(
            "[test] FAIL test_a11y_activation_dismisses_overlays: no Close node (menu)\n",
        );
        return false;
    };
    d.focus.focus(close.id);
    d.handle_event(Event::Key(keys::SCAN_ENTER as u16));
    if d.start_menu.open {
        io::print_str(
            "[test] FAIL test_a11y_activation_dismisses_overlays: start menu not dismissed\n",
        );
        return false;
    }
    if d.wm.lookup(wid).is_none() {
        io::print_str(
            "[test] FAIL test_a11y_activation_dismisses_overlays: Close acted through menu\n",
        );
        return false;
    }
    a11y_log_pass(
        "test_a11y_activation_dismisses_overlays",
        "start menu dismissed",
    );

    // Context menu up: Enter dismisses it (no focus needed — the overlay
    // consumes the activation regardless of what is beneath).
    let mut d = Desktop::new(800, 600);
    d.context_menu = Some(ContextMenu {
        x: 0,
        y: 0,
        items: &[],
    });
    d.tick();
    d.handle_event(Event::Key(keys::SCAN_ENTER as u16));
    if d.context_menu.is_some() {
        io::print_str(
            "[test] FAIL test_a11y_activation_dismisses_overlays: context menu not dismissed\n",
        );
        return false;
    }
    a11y_log_pass(
        "test_a11y_activation_dismisses_overlays",
        "context menu dismissed",
    );

    // Task manager up: Enter dismisses it.
    let mut d = Desktop::new(800, 600);
    d.task_manager.open = true;
    d.tick();
    d.handle_event(Event::Key(keys::SCAN_ENTER as u16));
    if d.task_manager.open {
        io::print_str(
            "[test] FAIL test_a11y_activation_dismisses_overlays: task manager not dismissed\n",
        );
        return false;
    }
    a11y_log_pass(
        "test_a11y_activation_dismisses_overlays",
        "task manager dismissed",
    );

    // Two-step contract: after dismissal, the NEXT Enter acts on the
    // still-focused node (nothing moved the focus), so Close closes — the
    // "dismiss before acting" half of the mouse-click semantics.
    let mut d = Desktop::new(800, 600);
    let before = d.wm.len();
    let wid = d.wm.create(AppWindow::new(100, 100, 400, 300, "CloseMe"));
    d.settings.open = true;
    d.tick();
    let close = d
        .a11y_tree
        .nodes
        .iter()
        .find(|n| n.role == A11yRole::Button && n.label == "Close" && n.owner == Some(wid));
    let Some(close) = close else {
        io::print_str(
            "[test] FAIL test_a11y_activation_dismisses_overlays: no Close node (2-step)\n",
        );
        return false;
    };
    d.focus.focus(close.id);
    d.handle_event(Event::Key(keys::SCAN_ENTER as u16)); // 1st: dismisses settings
    if d.settings.open {
        io::print_str(
            "[test] FAIL test_a11y_activation_dismisses_overlays: 2-step did not dismiss\n",
        );
        return false;
    }
    d.handle_event(Event::Key(keys::SCAN_ENTER as u16)); // 2nd: Close acts
    for _ in 0..60 {
        d.tick();
    }
    if d.wm.len() != before {
        io::print_str(
            "[test] FAIL test_a11y_activation_dismisses_overlays: 2-step Close did not act\n",
        );
        return false;
    }
    a11y_log_pass(
        "test_a11y_activation_dismisses_overlays",
        "second Enter acts",
    );

    // Negative control: with no overlay up, activation still acts normally
    // (Close closes its window) — the guard must not swallow real actions.
    let mut d = Desktop::new(800, 600);
    let before = d.wm.len();
    let wid = d.wm.create(AppWindow::new(100, 100, 400, 300, "CloseMe"));
    d.tick();
    let close = d
        .a11y_tree
        .nodes
        .iter()
        .find(|n| n.role == A11yRole::Button && n.label == "Close" && n.owner == Some(wid));
    let Some(close) = close else {
        io::print_str(
            "[test] FAIL test_a11y_activation_dismisses_overlays: no Close node (control)\n",
        );
        return false;
    };
    d.focus.focus(close.id);
    d.handle_event(Event::Key(keys::SCAN_ENTER as u16));
    for _ in 0..60 {
        d.tick();
    }
    if d.wm.len() != before {
        io::print_str(
            "[test] FAIL test_a11y_activation_dismisses_overlays: Close no-op without overlay\n",
        );
        return false;
    }
    a11y_log_pass(
        "test_a11y_activation_dismisses_overlays",
        "no overlay, Close acts",
    );

    io::print_str("[test] PASS test_a11y_activation_dismisses_overlays\n");
    true
}

/// Mouse/keyboard modal parity: for EVERY overlay flag, a mouse click on a
/// taskbar button position (`handle_click`) and an a11y Enter on that
/// taskbar node (`handle_a11y_key`) agree — both dismiss the overlay and
/// NEITHER acts on the window beneath (the active window stays the other
/// one). This pins the contract `test_a11y_activation_dismisses_overlays`
/// documents ("handle_click checks the overlays before the taskbar and
/// windows, so a click on a taskbar button with a modal up only dismisses
/// the modal") against regression: if an overlay is ever dropped from the
/// pre-taskbar checks, or the context menu sneaks back below the taskbar
/// (where it historically acted through the menu), the two legs diverge
/// and this test fails.
pub(crate) fn test_a11y_overlay_mouse_keyboard_parity() -> bool {
    // The six overlay flags. Each sub-case runs both legs on its own fresh
    // Desktop with two windows (B active after creation); the click/Enter
    // targets A's taskbar button, so "not acted on" means wm.active()
    // stays B.
    let names: [&str; 6] = [
        "settings",
        "settings_app",
        "task_manager",
        "about",
        "start_menu",
        "context_menu",
    ];
    for &name in &names {
        // Mouse leg: click A's taskbar button position (index 0).
        let mut d = Desktop::new(800, 600);
        let _a = d.wm.create(AppWindow::new(50, 50, 300, 200, "WinA"));
        let b = d.wm.create(AppWindow::new(200, 200, 300, 200, "WinB"));
        if !open_overlay(&mut d, name) {
            io::print_str(&alloc::format!(
                "[test] FAIL test_a11y_overlay_mouse_keyboard_parity: unknown overlay '{}'\n",
                name
            ));
            return false;
        }
        d.tick();
        let ty = d.taskbar_y() as i32;
        let r = layout::taskbar_btn_rect(0, ty as u32);
        d.handle_click(r.x + r.w as i32 / 2, r.y + r.h as i32 / 2);
        if overlay_flag_open(&d, name) {
            io::print_str(&alloc::format!(
                "[test] FAIL test_a11y_overlay_mouse_keyboard_parity: mouse did not dismiss {}\n",
                name
            ));
            return false;
        }
        if d.wm.active() != Some(b) {
            io::print_str(&alloc::format!(
                "[test] FAIL test_a11y_overlay_mouse_keyboard_parity: mouse acted through {}\n",
                name
            ));
            return false;
        }
        a11y_log_pass(
            "test_a11y_overlay_mouse_keyboard_parity",
            &alloc::format!("mouse dismisses {}", name),
        );

        // Keyboard leg: Enter on A's taskbar node with the same overlay up.
        let mut d = Desktop::new(800, 600);
        let a = d.wm.create(AppWindow::new(50, 50, 300, 200, "WinA"));
        let b = d.wm.create(AppWindow::new(200, 200, 300, 200, "WinB"));
        if !open_overlay(&mut d, name) {
            io::print_str(&alloc::format!(
                "[test] FAIL test_a11y_overlay_mouse_keyboard_parity: unknown overlay '{}'\n",
                name
            ));
            return false;
        }
        d.tick();
        let btn_a = d.a11y_tree.nodes.iter().find(|n| {
            n.role == A11yRole::Button
                && n.owner == Some(a)
                && n.parent
                    .and_then(|p| d.a11y_tree.nodes.iter().find(|m| m.id == p))
                    .is_some_and(|p| p.role == A11yRole::Taskbar)
        });
        let Some(btn_a) = btn_a else {
            io::print_str(&alloc::format!(
                "[test] FAIL test_a11y_overlay_mouse_keyboard_parity: no taskbar node ({})\n",
                name
            ));
            return false;
        };
        d.focus.focus(btn_a.id);
        d.handle_event(Event::Key(keys::SCAN_ENTER as u16));
        if overlay_flag_open(&d, name) {
            io::print_str(&alloc::format!(
                "[test] FAIL test_a11y_overlay_mouse_keyboard_parity: Enter did not dismiss {}\n",
                name
            ));
            return false;
        }
        if d.wm.active() != Some(b) {
            io::print_str(&alloc::format!(
                "[test] FAIL test_a11y_overlay_mouse_keyboard_parity: Enter acted through {}\n",
                name
            ));
            return false;
        }
        a11y_log_pass(
            "test_a11y_overlay_mouse_keyboard_parity",
            &alloc::format!("Enter dismisses {}", name),
        );
    }

    io::print_str("[test] PASS test_a11y_overlay_mouse_keyboard_parity\n");
    true
}

/// Open a named overlay flag on a fresh desktop — the setup side of the
/// parity legs (the mouse leg and the keyboard leg must start from the
/// same state). Returns false for an unknown name so a typo in the
/// caller's name list fails the test instead of opening nothing and
/// passing both legs vacuously.
fn open_overlay(d: &mut Desktop, name: &str) -> bool {
    match name {
        "settings" => {
            d.settings.open = true;
        }
        "settings_app" => {
            d.settings_app.open = true;
        }
        "task_manager" => {
            d.task_manager.open = true;
        }
        "about" => {
            d.about_state.open = true;
        }
        "start_menu" => d.start_menu.open_with(&d.app_reg),
        "context_menu" => {
            d.context_menu = Some(ContextMenu {
                x: 0,
                y: 0,
                items: &[],
            });
        }
        _ => return false,
    }
    true
}

/// Whether a named overlay flag is currently open — the single predicate
/// the parity test uses to assert both legs dismissed the same thing.
fn overlay_flag_open(d: &Desktop, name: &str) -> bool {
    match name {
        "settings" => d.settings.open,
        "settings_app" => d.settings_app.open,
        "task_manager" => d.task_manager.open,
        "about" => d.about_state.open,
        "start_menu" => d.start_menu.open,
        "context_menu" => d.context_menu.is_some(),
        _ => false,
    }
}

/// Keyboard focus lights the taskbar button under the ring, distinct from
/// mouse hover. The snapshot's `focused` must carry the focused taskbar
/// button's `TaskbarButton` target (or `StartButton` for the Start button)
/// and must be None when the mouse is in charge. The exclusion cases (a
/// Close control never lights a taskbar button, etc.) are pinned directly
/// on the resolver in `test_a11y_focused_target`.
pub(crate) fn test_a11y_taskbar_focus_feedback() -> bool {
    // Focused taskbar button lights up: the snapshot carries its
    // TaskbarButton target, the same value the draw compares for hover.
    let mut d = Desktop::new(800, 600);
    let a = d.wm.create(AppWindow::new(50, 50, 300, 200, "WinA"));
    let _b = d.wm.create(AppWindow::new(200, 200, 300, 200, "WinB"));
    d.tick();
    let btn_a = d
        .a11y_tree
        .nodes
        .iter()
        .find(|n| n.role == A11yRole::Button && n.label == "WinA");
    let Some(btn_a) = btn_a else {
        io::print_str("[test] FAIL test_a11y_taskbar_focus_feedback: no taskbar node for A\n");
        return false;
    };
    d.focus.focus(btn_a.id);
    d.focus_visible = true;
    let snap = RenderSnapshot::from(&d);
    if snap.focused != Some(HoverTarget::TaskbarButton(a)) {
        io::print_str(
            "[test] FAIL test_a11y_taskbar_focus_feedback: focused taskbar button not lit\n",
        );
        return false;
    }
    a11y_log_pass("test_a11y_taskbar_focus_feedback", "taskbar button lit");

    // The Start button resolves to its own target (sentinel owner), not a
    // window id.
    let mut d = Desktop::new(800, 600);
    d.tick();
    let start = d
        .a11y_tree
        .nodes
        .iter()
        .find(|n| n.role == A11yRole::Button && n.label == "Start");
    let Some(start) = start else {
        io::print_str("[test] FAIL test_a11y_taskbar_focus_feedback: no Start node\n");
        return false;
    };
    d.focus.focus(start.id);
    d.focus_visible = true;
    let snap = RenderSnapshot::from(&d);
    if snap.focused != Some(HoverTarget::StartButton) {
        io::print_str("[test] FAIL test_a11y_taskbar_focus_feedback: Start button not lit\n");
        return false;
    }
    a11y_log_pass("test_a11y_taskbar_focus_feedback", "Start button lit");

    // Mouse in charge (focus_visible false): nothing lights, even with a
    // focused id still set — the ring is the a11y mode's signal.
    let mut d = Desktop::new(800, 600);
    d.wm.create(AppWindow::new(50, 50, 300, 200, "WinA"));
    d.tick();
    let btn_a = d
        .a11y_tree
        .nodes
        .iter()
        .find(|n| n.role == A11yRole::Button && n.label == "WinA");
    let Some(btn_a) = btn_a else {
        io::print_str("[test] FAIL test_a11y_taskbar_focus_feedback: no taskbar node (mouse)\n");
        return false;
    };
    d.focus.focus(btn_a.id);
    d.focus_visible = false;
    let snap = RenderSnapshot::from(&d);
    if snap.focused.is_some() {
        io::print_str("[test] FAIL test_a11y_taskbar_focus_feedback: lit without focus_visible\n");
        return false;
    }
    a11y_log_pass("test_a11y_taskbar_focus_feedback", "mouse mode not lit");

    // Focus must be visually DISTINCT from hover: the focused fill is the
    // accent_light blue, hover keeps the indigo `th.hover`, focus wins
    // when both apply (the ring is the active mode), and pressed (hover
    // held) still beats everything. Pinned via the pure fill helpers so
    // the exact color choice is a contract, not a pixel test — a change
    // that collapses the two affordances back to one fill fails here.
    {
        let th = d.theme_svc.current();
        let fill = crate::core::taskbar::window_button_fill;
        if fill(true, false, false, false, false, th) != th.accent_light {
            io::print_str(
                "[test] FAIL test_a11y_taskbar_focus_feedback: focused fill != accent_light\n",
            );
            return false;
        }
        if fill(false, true, false, false, false, th) != th.hover {
            io::print_str("[test] FAIL test_a11y_taskbar_focus_feedback: hover fill != th.hover\n");
            return false;
        }
        if fill(true, true, false, false, false, th) != th.accent_light {
            io::print_str(
                "[test] FAIL test_a11y_taskbar_focus_feedback: focus did not beat hover\n",
            );
            return false;
        }
        if fill(true, true, true, false, false, th) != th.pressed {
            io::print_str(
                "[test] FAIL test_a11y_taskbar_focus_feedback: pressed did not beat focus\n",
            );
            return false;
        }
        // Lower rungs of the priority ladder: a hovered MINIMIZED button
        // still lights via th.hover (the old `hover || focused` union
        // widened hover over the is_min arm — the split must preserve it),
        // and the resting fills stay stable.
        if fill(false, true, false, true, false, th) != th.hover {
            io::print_str(
                "[test] FAIL test_a11y_taskbar_focus_feedback: hovered minimized not lit\n",
            );
            return false;
        }
        if fill(false, false, false, true, false, th) != th.bg_surface {
            io::print_str(
                "[test] FAIL test_a11y_taskbar_focus_feedback: minimized fill regressed\n",
            );
            return false;
        }
        if fill(false, false, false, false, true, th) != th.bg_elevated {
            io::print_str("[test] FAIL test_a11y_taskbar_focus_feedback: top fill regressed\n");
            return false;
        }
        let sfill = crate::core::taskbar::start_button_fill;
        if sfill(true, false, false, th) != th.accent_light {
            io::print_str(
                "[test] FAIL test_a11y_taskbar_focus_feedback: Start focused != accent_light\n",
            );
            return false;
        }
        if sfill(false, true, false, th) != th.hover {
            io::print_str(
                "[test] FAIL test_a11y_taskbar_focus_feedback: Start hover != th.hover\n",
            );
            return false;
        }
        a11y_log_pass(
            "test_a11y_taskbar_focus_feedback",
            "focus fill distinct from hover",
        );
    }

    // Overflow lockstep: with more windows than the taskbar fits
    // (TASKBAR_MAX_BTNS), the draw caps the buttons and the a11y tree must
    // cap identically. An overflow window's taskbar button is never drawn,
    // so no node may exist for it — otherwise the ring could land on an
    // undrawn button and the focused light would silently miss (and the
    // ring would float over the overflow/tray region). The overflow
    // window stays reachable via its Window node.
    {
        let mut d = Desktop::new(800, 600);
        let mut overflow_win = None;
        for i in 0..=crate::layout::TASKBAR_MAX_BTNS {
            let wid = d.wm.create(AppWindow::new(20, 20, 200, 150, "OverflowWin"));
            if i == crate::layout::TASKBAR_MAX_BTNS {
                overflow_win = Some(wid);
            }
        }
        d.tick();
        let taskbar_id = d
            .a11y_tree
            .nodes
            .iter()
            .find(|n| n.role == A11yRole::Taskbar)
            .map(|n| n.id);
        let Some(taskbar_id) = taskbar_id else {
            io::print_str("[test] FAIL test_a11y_taskbar_focus_feedback: no Taskbar node\n");
            return false;
        };
        let btn_count = d
            .a11y_tree
            .nodes
            .iter()
            .filter(|n| {
                n.role == A11yRole::Button
                    && n.parent == Some(taskbar_id)
                    && n.owner != Some(START_BUTTON_OWNER)
            })
            .count();
        if btn_count != crate::layout::TASKBAR_MAX_BTNS {
            io::print_str(&alloc::format!(
                "[test] FAIL test_a11y_taskbar_focus_feedback: {} taskbar nodes != cap {}\n",
                btn_count,
                crate::layout::TASKBAR_MAX_BTNS
            ));
            return false;
        }
        if let Some(wid) = overflow_win {
            // Taskbar-child Button only — the overflow window's CHROME
            // Close/Minimize nodes (Window children) legitimately carry its
            // owner and must not trip this.
            if d.a11y_tree.nodes.iter().any(|n| {
                n.role == A11yRole::Button && n.parent == Some(taskbar_id) && n.owner == Some(wid)
            }) {
                io::print_str(
                    "[test] FAIL test_a11y_taskbar_focus_feedback: overflow window has a taskbar node\n",
                );
                return false;
            }
        }
        a11y_log_pass("test_a11y_taskbar_focus_feedback", "overflow cap lockstep");
    }

    io::print_str("[test] PASS test_a11y_taskbar_focus_feedback\n");
    true
}

/// Keyboard focus lights a window's Close/Minimize control under the ring,
/// like the hover affordance. The snapshot's `focused` must carry the
/// focused control's `HoverTarget::Window` (window id + which button,
/// discriminated by the tree's own chrome labels) and must be None when
/// the mouse is in charge. The exclusion cases (a taskbar button or menu
/// row never lights a control) are pinned directly on the resolver in
/// `test_a11y_focused_target`.
pub(crate) fn test_a11y_window_btn_focus_feedback() -> bool {
    // Focused Close control lights up: the snapshot carries its Window
    // target (win + Close), the same value `window::draw` compares for
    // hover to brighten the fill.
    let mut d = Desktop::new(800, 600);
    let a = d.wm.create(AppWindow::new(100, 100, 400, 300, "CloseMe"));
    d.tick();
    let close = d
        .a11y_tree
        .nodes
        .iter()
        .find(|n| n.role == A11yRole::Button && n.label == "Close" && n.owner == Some(a));
    let Some(close) = close else {
        io::print_str("[test] FAIL test_a11y_window_btn_focus_feedback: no Close node\n");
        return false;
    };
    d.focus.focus(close.id);
    d.focus_visible = true;
    let snap = RenderSnapshot::from(&d);
    let expected_close = Some(HoverTarget::Window {
        win: a,
        btn: WindowButton::Close,
    });
    if snap.focused != expected_close {
        io::print_str(&alloc::format!(
            "[test] FAIL test_a11y_window_btn_focus_feedback: Close not lit: {:?}\n",
            snap.focused
        ));
        return false;
    }
    a11y_log_pass("test_a11y_window_btn_focus_feedback", "Close lit");

    // Focused Minimize control lights up with its own button discriminant —
    // the label check, not position, tells the two chrome controls apart.
    let mut d = Desktop::new(800, 600);
    let a = d.wm.create(AppWindow::new(100, 100, 400, 300, "MinMe"));
    d.tick();
    let min = d
        .a11y_tree
        .nodes
        .iter()
        .find(|n| n.role == A11yRole::Button && n.label == "Minimize" && n.owner == Some(a));
    let Some(min) = min else {
        io::print_str("[test] FAIL test_a11y_window_btn_focus_feedback: no Minimize node\n");
        return false;
    };
    d.focus.focus(min.id);
    d.focus_visible = true;
    let snap = RenderSnapshot::from(&d);
    let expected_min = Some(HoverTarget::Window {
        win: a,
        btn: WindowButton::Minimize,
    });
    if snap.focused != expected_min {
        io::print_str(&alloc::format!(
            "[test] FAIL test_a11y_window_btn_focus_feedback: Minimize not lit: {:?}\n",
            snap.focused
        ));
        return false;
    }
    a11y_log_pass("test_a11y_window_btn_focus_feedback", "Minimize lit");

    // Mouse in charge (focus_visible false): nothing lights, even with a
    // focused Close id still set — the ring is the a11y mode's signal.
    let mut d = Desktop::new(800, 600);
    let a = d.wm.create(AppWindow::new(100, 100, 400, 300, "CloseMe"));
    d.tick();
    let close = d
        .a11y_tree
        .nodes
        .iter()
        .find(|n| n.role == A11yRole::Button && n.label == "Close" && n.owner == Some(a));
    let Some(close) = close else {
        io::print_str("[test] FAIL test_a11y_window_btn_focus_feedback: no Close node (mouse)\n");
        return false;
    };
    d.focus.focus(close.id);
    d.focus_visible = false;
    let snap = RenderSnapshot::from(&d);
    if snap.focused.is_some() {
        io::print_str(
            "[test] FAIL test_a11y_window_btn_focus_feedback: lit without focus_visible\n",
        );
        return false;
    }
    a11y_log_pass("test_a11y_window_btn_focus_feedback", "mouse mode not lit");

    io::print_str("[test] PASS test_a11y_window_btn_focus_feedback\n");
    true
}

/// Direct pin of the single focus resolver (`Desktop::focused_target`): a
/// focused Button resolves by parent role to the Start button (sentinel
/// owner), a taskbar window button (owner), a start-menu app row (bounds
/// equality), or a window Close/Minimize control (chrome label) — and
/// everything else (Window nodes, the StartMenu container, the Taskbar
/// container, icons, the tray) resolves to None. This is where the
/// cross-surface exclusions live now that the snapshot carries one
/// `focused` value instead of three per-surface fields: the old "Close
/// focus lit a taskbar button" style legs all reduce to these arms.
pub(crate) fn test_a11y_focused_target() -> bool {
    // Start button -> StartButton (sentinel owner, never a window id).
    let mut d = Desktop::new(800, 600);
    d.tick();
    let start = d
        .a11y_tree
        .nodes
        .iter()
        .find(|n| n.role == A11yRole::Button && n.label == "Start");
    let Some(start) = start else {
        io::print_str("[test] FAIL test_a11y_focused_target: no Start node\n");
        return false;
    };
    if d.focused_target(start.id) != Some(HoverTarget::StartButton) {
        io::print_str("[test] FAIL test_a11y_focused_target: Start not StartButton\n");
        return false;
    }
    a11y_log_pass("test_a11y_focused_target", "Start button");

    // Taskbar window button -> TaskbarButton(owner). A window titled
    // "Close" proves the discrimination is structural (parent role), not
    // label-based: its taskbar button must resolve to TaskbarButton, never
    // to a Window control.
    let mut d = Desktop::new(800, 600);
    let a = d.wm.create(AppWindow::new(50, 50, 300, 200, "Close"));
    d.tick();
    let taskbar_btn = d.a11y_tree.nodes.iter().find(|n| {
        n.role == A11yRole::Button
            && n.owner == Some(a)
            && n.parent
                .and_then(|p| d.a11y_tree.nodes.iter().find(|m| m.id == p))
                .is_some_and(|p| p.role == A11yRole::Taskbar)
    });
    let Some(taskbar_btn) = taskbar_btn else {
        io::print_str("[test] FAIL test_a11y_focused_target: no taskbar node\n");
        return false;
    };
    if d.focused_target(taskbar_btn.id) != Some(HoverTarget::TaskbarButton(a)) {
        io::print_str("[test] FAIL test_a11y_focused_target: taskbar btn misresolved\n");
        return false;
    }
    a11y_log_pass("test_a11y_focused_target", "taskbar button (titled Close)");

    // Start-menu app row -> StartApp(i) via bounds equality.
    let mut d = Desktop::new(800, 600);
    d.start_menu.open_with(&d.app_reg);
    d.tick();
    let menu_r = layout::menu_rect(d.taskbar_y());
    let sm = d
        .a11y_tree
        .nodes
        .iter()
        .find(|n| n.role == A11yRole::StartMenu);
    let Some(sm) = sm else {
        io::print_str("[test] FAIL test_a11y_focused_target: no StartMenu node\n");
        return false;
    };
    let row0 = d
        .a11y_tree
        .nodes
        .iter()
        .find(|n| n.parent == Some(sm.id) && n.bounds == layout::menu_item_rect(menu_r, 0, 0));
    let Some(row0) = row0 else {
        io::print_str("[test] FAIL test_a11y_focused_target: no row 0 node\n");
        return false;
    };
    if d.focused_target(row0.id) != Some(HoverTarget::StartApp(0)) {
        io::print_str("[test] FAIL test_a11y_focused_target: row misresolved\n");
        return false;
    }
    a11y_log_pass("test_a11y_focused_target", "menu row");

    // Window Close control -> Window{win, Close}.
    let mut d = Desktop::new(800, 600);
    let a = d.wm.create(AppWindow::new(100, 100, 400, 300, "CloseMe"));
    d.tick();
    let close = d
        .a11y_tree
        .nodes
        .iter()
        .find(|n| n.role == A11yRole::Button && n.label == "Close" && n.owner == Some(a));
    let Some(close) = close else {
        io::print_str("[test] FAIL test_a11y_focused_target: no Close node\n");
        return false;
    };
    let expected_close = Some(HoverTarget::Window {
        win: a,
        btn: WindowButton::Close,
    });
    if d.focused_target(close.id) != expected_close {
        io::print_str("[test] FAIL test_a11y_focused_target: Close misresolved\n");
        return false;
    }
    // A taskbar button's owner must NOT resolve to a Window control even
    // when the window is titled "Close" — same structural guard.
    let taskbar_btn = d.a11y_tree.nodes.iter().find(|n| {
        n.role == A11yRole::Button
            && n.owner == Some(a)
            && n.parent
                .and_then(|p| d.a11y_tree.nodes.iter().find(|m| m.id == p))
                .is_some_and(|p| p.role == A11yRole::Taskbar)
    });
    // Hard assert: a regression that drops taskbar nodes must fail here,
    // not pass by skipping the body (the find above returning None would
    // otherwise make this leg silently green).
    let Some(btn) = taskbar_btn else {
        io::print_str("[test] FAIL test_a11y_focused_target: no Close taskbar node\n");
        return false;
    };
    if d.focused_target(btn.id) != Some(HoverTarget::TaskbarButton(a)) {
        io::print_str("[test] FAIL test_a11y_focused_target: Close taskbar misresolved\n");
        return false;
    }
    a11y_log_pass("test_a11y_focused_target", "Close control");

    // Window Minimize control -> Window{win, Minimize}.
    let mut d = Desktop::new(800, 600);
    let a = d.wm.create(AppWindow::new(100, 100, 400, 300, "MinMe"));
    d.tick();
    let min = d
        .a11y_tree
        .nodes
        .iter()
        .find(|n| n.role == A11yRole::Button && n.label == "Minimize" && n.owner == Some(a));
    let Some(min) = min else {
        io::print_str("[test] FAIL test_a11y_focused_target: no Minimize node\n");
        return false;
    };
    if d.focused_target(min.id)
        != Some(HoverTarget::Window {
            win: a,
            btn: WindowButton::Minimize,
        })
    {
        io::print_str("[test] FAIL test_a11y_focused_target: Minimize misresolved\n");
        return false;
    }
    a11y_log_pass("test_a11y_focused_target", "Minimize control");

    // Non-interactive containers and roles resolve to None: the Window
    // node, the StartMenu container, the Taskbar container, the Desktop
    // root, and the tray panel all carry focus (the ring draws) but no
    // surface light. Role-based finds — labels collide with taskbar
    // buttons and are display text only.
    let mut d = Desktop::new(800, 600);
    d.wm.create(AppWindow::new(50, 50, 300, 200, "WinA"));
    d.start_menu.open_with(&d.app_reg);
    d.tick();
    let container_roles = [
        A11yRole::Window,
        A11yRole::StartMenu,
        A11yRole::Taskbar,
        A11yRole::Desktop,
        A11yRole::TrayPanel,
    ];
    for role in container_roles {
        let node = d.a11y_tree.nodes.iter().find(|n| n.role == role);
        let Some(node) = node else {
            io::print_str(&alloc::format!(
                "[test] FAIL test_a11y_focused_target: no {:?} node\n",
                role
            ));
            return false;
        };
        if d.focused_target(node.id).is_some() {
            io::print_str(&alloc::format!(
                "[test] FAIL test_a11y_focused_target: {:?} lit a surface\n",
                role
            ));
            return false;
        }
    }
    a11y_log_pass("test_a11y_focused_target", "containers resolve to None");

    io::print_str("[test] PASS test_a11y_focused_target\n");
    true
}

/// Keyboard focus lights the start-menu app row under the ring, like the
/// StartApp hover target. The snapshot's `focused` must carry
/// `HoverTarget::StartApp(i)` for the focused row (resolved by the same
/// bounds-equality Enter-launch uses) and must be None when the mouse is
/// in charge. The exclusion cases (the StartMenu container and window
/// controls never light a row) are pinned directly on the resolver in
/// `test_a11y_focused_target`.
pub(crate) fn test_a11y_start_menu_focus_feedback() -> bool {
    // Focused row lights up: the snapshot carries its StartApp target.
    let mut d = Desktop::new(800, 600);
    d.start_menu.open_with(&d.app_reg);
    d.tick();
    let menu_r = layout::menu_rect(d.taskbar_y());
    let sm = d
        .a11y_tree
        .nodes
        .iter()
        .find(|n| n.role == A11yRole::StartMenu);
    let Some(sm) = sm else {
        io::print_str("[test] FAIL test_a11y_start_menu_focus_feedback: no StartMenu node\n");
        return false;
    };
    let row0 = d
        .a11y_tree
        .nodes
        .iter()
        .find(|n| n.parent == Some(sm.id) && n.bounds == layout::menu_item_rect(menu_r, 0, 0));
    let Some(row0) = row0 else {
        io::print_str("[test] FAIL test_a11y_start_menu_focus_feedback: no row 0 node\n");
        return false;
    };
    d.focus.focus(row0.id);
    d.focus_visible = true;
    let snap = RenderSnapshot::from(&d);
    if snap.focused != Some(HoverTarget::StartApp(0)) {
        io::print_str(&alloc::format!(
            "[test] FAIL test_a11y_start_menu_focus_feedback: focused row not lit: {:?}\n",
            snap.focused
        ));
        return false;
    }
    a11y_log_pass("test_a11y_start_menu_focus_feedback", "app row lit");

    // A scrolled row resolves to its filtered index: focus the LAST visible
    // row (scroll = 3, so its bounds are `menu_item_rect(menu_r, 3, 3)` and
    // the resolved index is 3, not 0) — proving the resolver uses bounds
    // equality, not the position in the node list.
    let mut d = Desktop::new(800, 600);
    d.start_menu.open_with(&d.app_reg);
    d.start_menu.scroll = 3;
    d.tick();
    let menu_r = layout::menu_rect(d.taskbar_y());
    let sm = d
        .a11y_tree
        .nodes
        .iter()
        .find(|n| n.role == A11yRole::StartMenu);
    let Some(sm) = sm else {
        io::print_str("[test] FAIL test_a11y_start_menu_focus_feedback: no StartMenu (scrolled)\n");
        return false;
    };
    let row = d
        .a11y_tree
        .nodes
        .iter()
        .find(|n| n.parent == Some(sm.id) && n.bounds == layout::menu_item_rect(menu_r, 3, 3));
    let Some(row) = row else {
        io::print_str("[test] FAIL test_a11y_start_menu_focus_feedback: no scrolled row node\n");
        return false;
    };
    d.focus.focus(row.id);
    d.focus_visible = true;
    let snap = RenderSnapshot::from(&d);
    if snap.focused != Some(HoverTarget::StartApp(3)) {
        io::print_str(&alloc::format!(
            "[test] FAIL test_a11y_start_menu_focus_feedback: scrolled row index wrong: {:?}\n",
            snap.focused
        ));
        return false;
    }
    a11y_log_pass("test_a11y_start_menu_focus_feedback", "scrolled row index");

    // Mouse in charge (focus_visible false): nothing lights, even with a
    // focused id still set — the ring is the a11y mode's signal.
    let mut d = Desktop::new(800, 600);
    d.start_menu.open_with(&d.app_reg);
    d.tick();
    let menu_r = layout::menu_rect(d.taskbar_y());
    let sm = d
        .a11y_tree
        .nodes
        .iter()
        .find(|n| n.role == A11yRole::StartMenu);
    let Some(sm) = sm else {
        io::print_str("[test] FAIL test_a11y_start_menu_focus_feedback: no StartMenu (mouse)\n");
        return false;
    };
    let row0 = d
        .a11y_tree
        .nodes
        .iter()
        .find(|n| n.parent == Some(sm.id) && n.bounds == layout::menu_item_rect(menu_r, 0, 0));
    let Some(row0) = row0 else {
        io::print_str("[test] FAIL test_a11y_start_menu_focus_feedback: no row 0 node (mouse)\n");
        return false;
    };
    d.focus.focus(row0.id);
    d.focus_visible = false;
    let snap = RenderSnapshot::from(&d);
    if snap.focused.is_some() {
        io::print_str(
            "[test] FAIL test_a11y_start_menu_focus_feedback: lit without focus_visible\n",
        );
        return false;
    }
    a11y_log_pass("test_a11y_start_menu_focus_feedback", "mouse mode not lit");

    io::print_str("[test] PASS test_a11y_start_menu_focus_feedback\n");
    true
}

/// Tooltip hardening pins: the manager shows one tooltip at a time with an
/// explicit focus/pointer owner, only the owner may dismiss it, dismissal is
/// a delayed fade-out (so a quick return cancels it), fade-in ramps alpha
/// from transparent, and the desktop-level hover path keeps a tooltip alive
/// while hovering (no show→timeout→re-show flicker). Long titles truncate
/// through the layout helper before sizing.
pub(crate) fn test_tooltip_hardening() -> bool {
    // Owner discipline: show replaces (one at a time) and a foreign owner
    // cannot hide another owner's tooltip.
    let mut m = TooltipManager::new();
    m.show(TooltipOwner::Pointer(1), "A", 0, 0, 120);
    m.show(TooltipOwner::Pointer(2), "B", 0, 0, 120);
    if m.active.as_ref().is_none_or(|t| t.text != "B") {
        io::print_str("[test] FAIL test_tooltip_hardening: show did not replace\n");
        return false;
    }
    if m.active.as_ref().map(|t| t.alpha) != Some(0) {
        io::print_str("[test] FAIL test_tooltip_hardening: show not transparent\n");
        return false;
    }
    m.hide(TooltipOwner::Pointer(1)); // stale owner
    if m.active.is_none() {
        io::print_str("[test] FAIL test_tooltip_hardening: stale owner hid tooltip\n");
        return false;
    }
    a11y_log_pass("test_tooltip_hardening", "owner-scoped hide");

    // Delayed dismiss: hide starts a fade-out; the tooltip lingers a few
    // ticks then vanishes.
    let mut m = TooltipManager::new();
    m.show(TooltipOwner::Pointer(1), "A", 0, 0, 120);
    m.hide(TooltipOwner::Pointer(1));
    m.tick();
    if m.active.is_none() {
        io::print_str("[test] FAIL test_tooltip_hardening: hide vanished instantly\n");
        return false;
    }
    for _ in 0..16 {
        m.tick();
    }
    if m.active.is_some() {
        io::print_str("[test] FAIL test_tooltip_hardening: fade-out never finished\n");
        return false;
    }
    a11y_log_pass("test_tooltip_hardening", "delayed fade-out");

    // Keep-alive cancels an in-progress fade-out (pointer returned to the
    // same node within the dismiss window).
    let mut m = TooltipManager::new();
    m.show(TooltipOwner::Pointer(1), "A", 0, 0, 120);
    m.hide(TooltipOwner::Pointer(1));
    m.keep_alive(TooltipOwner::Pointer(1));
    m.tick();
    if m.active.is_none() {
        io::print_str("[test] FAIL test_tooltip_hardening: keep_alive did not cancel\n");
        return false;
    }
    if m.active.as_ref().map(|t| t.alpha) != Some(255) {
        io::print_str("[test] FAIL test_tooltip_hardening: keep_alive not opaque\n");
        return false;
    }
    for _ in 0..8 {
        m.tick();
    }
    if m.active.is_none() {
        io::print_str("[test] FAIL test_tooltip_hardening: tooltip died after cancel\n");
        return false;
    }
    a11y_log_pass("test_tooltip_hardening", "keep_alive cancels fade");

    // Keep-alive from a foreign owner is ignored.
    let mut m = TooltipManager::new();
    m.show(TooltipOwner::Pointer(1), "A", 0, 0, 120);
    m.hide(TooltipOwner::Pointer(1));
    m.keep_alive(TooltipOwner::Focus(9));
    for _ in 0..16 {
        m.tick();
    }
    if m.active.is_some() {
        io::print_str("[test] FAIL test_tooltip_hardening: foreign keep_alive revived\n");
        return false;
    }
    a11y_log_pass("test_tooltip_hardening", "foreign keep_alive ignored");

    // Fade-in ramps alpha to full over a few ticks.
    let mut m = TooltipManager::new();
    m.show(TooltipOwner::Pointer(1), "A", 0, 0, 120);
    for _ in 0..8 {
        m.tick();
    }
    if m.active.as_ref().map(|t| t.alpha) != Some(255) {
        io::print_str("[test] FAIL test_tooltip_hardening: fade-in incomplete\n");
        return false;
    }
    a11y_log_pass("test_tooltip_hardening", "fade-in completes");

    // Desktop-level: hovering a taskbar button keeps the tooltip alive past
    // its original timeout (no re-show flicker), and moving off starts a
    // delayed fade-out that a quick return cancels.
    let mut d = Desktop::new(800, 600);
    let _wid = d.wm.create(AppWindow::new(100, 100, 400, 300, "LongTitle"));
    d.tick();
    let ty = d.taskbar_y() as i32;
    let btn = layout::taskbar_btn_rect(0, ty as u32);
    d.update_mouse(btn.x + btn.w as i32 / 2, btn.y + btn.h as i32 / 2, false);
    for _ in 0..40 {
        d.tick();
    }
    // The tooltip is up and the pointer is still on it past the show
    // timeout (120 ticks) — keep_alive keeps it alive; alpha fully faded in.
    for _ in 0..130 {
        d.tick();
    }
    let tip = match &d.tooltips.active {
        Some(t) => t,
        None => {
            io::print_str("[test] FAIL test_tooltip_hardening: tooltip expired while hovering\n");
            return false;
        }
    };
    if tip.alpha != 255 {
        io::print_str("[test] FAIL test_tooltip_hardening: hovered tooltip not opaque\n");
        return false;
    }
    a11y_log_pass("test_tooltip_hardening", "kept alive while hovering");

    // Move off: fade-out begins (still present a tick later), then clears.
    d.update_mouse(700, 300, false);
    d.tick();
    if d.tooltips.active.is_none() {
        io::print_str("[test] FAIL test_tooltip_hardening: leave vanished instantly\n");
        return false;
    }
    for _ in 0..16 {
        d.tick();
    }
    if d.tooltips.active.is_some() {
        io::print_str("[test] FAIL test_tooltip_hardening: leave fade-out never finished\n");
        return false;
    }
    a11y_log_pass("test_tooltip_hardening", "leave fades out");

    // Quick return within the dismiss window cancels the fade.
    let mut d = Desktop::new(800, 600);
    let _wid = d.wm.create(AppWindow::new(100, 100, 400, 300, "LongTitle"));
    d.tick();
    let ty = d.taskbar_y() as i32;
    let btn = layout::taskbar_btn_rect(0, ty as u32);
    d.update_mouse(btn.x + btn.w as i32 / 2, btn.y + btn.h as i32 / 2, false);
    for _ in 0..40 {
        d.tick();
    }
    d.update_mouse(700, 300, false);
    d.tick();
    d.update_mouse(btn.x + btn.w as i32 / 2, btn.y + btn.h as i32 / 2, false);
    d.tick();
    if d.tooltips.active.is_none() {
        io::print_str("[test] FAIL test_tooltip_hardening: return did not cancel fade\n");
        return false;
    }
    for _ in 0..8 {
        d.tick();
    }
    if d.tooltips.active.is_none() {
        io::print_str("[test] FAIL test_tooltip_hardening: tooltip died after return\n");
        return false;
    }
    a11y_log_pass("test_tooltip_hardening", "quick return cancels");

    // Snapshot carries the fade alpha (render consumes it).
    let mut d = Desktop::new(800, 600);
    let _wid = d.wm.create(AppWindow::new(100, 100, 400, 300, "LongTitle"));
    d.tick();
    let ty = d.taskbar_y() as i32;
    let btn = layout::taskbar_btn_rect(0, ty as u32);
    d.update_mouse(btn.x + btn.w as i32 / 2, btn.y + btn.h as i32 / 2, false);
    for _ in 0..40 {
        d.tick();
    }
    let snap = RenderSnapshot::from(&d);
    if snap.tooltip_alpha != 255 {
        io::print_str("[test] FAIL test_tooltip_hardening: snapshot alpha wrong\n");
        return false;
    }
    // The taskbar button now labels 'Switch to LongTitle', so match the
    // title substring rather than a bare prefix.
    if snap.tooltip.is_none_or(|t| !t.contains("LongTitle")) {
        io::print_str("[test] FAIL test_tooltip_hardening: snapshot tooltip text wrong\n");
        return false;
    }
    a11y_log_pass("test_tooltip_hardening", "snapshot alpha + text");

    // A modal overlay swallows the pointer, so tooltips are suppressed
    // entirely while one is up: with the settings panel open, hovering a
    // window Close button shows nothing — `hover_target()` is None under
    // the overlay, so the Close arm never fires and the owner fallback
    // would otherwise leak the plain title.
    let mut d = Desktop::new(800, 600);
    d.wm.create(AppWindow::new(100, 100, 400, 300, "SettingsWin"));
    d.settings.open = true;
    d.tick();
    let close = layout::close_btn_rect(100, 100, 400);
    d.update_mouse(
        close.x + close.w as i32 / 2,
        close.y + close.h as i32 / 2,
        false,
    );
    for _ in 0..40 {
        d.tick();
    }
    if d.tooltips.active.is_some() {
        io::print_str("[test] FAIL test_tooltip_hardening: tooltip shown under overlay\n");
        return false;
    }
    a11y_log_pass("test_tooltip_hardening", "overlay suppresses tooltips");

    // A tooltip shown BEFORE the overlay opened dismisses when the overlay
    // opens (hide-once on the transition, then the fade completes), and
    // never re-appears while the pointer keeps hovering under the overlay.
    let mut d = Desktop::new(800, 600);
    d.wm.create(AppWindow::new(100, 100, 400, 300, "SettingsWin"));
    d.tick();
    let close = layout::close_btn_rect(100, 100, 400);
    d.update_mouse(
        close.x + close.w as i32 / 2,
        close.y + close.h as i32 / 2,
        false,
    );
    for _ in 0..40 {
        d.tick();
    }
    if d.tooltips.active.is_none() {
        io::print_str("[test] FAIL test_tooltip_hardening: pre-overlay tooltip missing\n");
        return false;
    }
    d.settings.open = true;
    for _ in 0..20 {
        d.tick();
    }
    if d.tooltips.active.is_some() {
        io::print_str("[test] FAIL test_tooltip_hardening: tooltip survived overlay open\n");
        return false;
    }
    // Still hovering under the open overlay: it must not re-appear.
    for _ in 0..40 {
        d.tick();
    }
    if d.tooltips.active.is_some() {
        io::print_str("[test] FAIL test_tooltip_hardening: tooltip re-shown under overlay\n");
        return false;
    }
    a11y_log_pass("test_tooltip_hardening", "pre-overlay tooltip dismisses");

    io::print_str("[test] PASS test_tooltip_hardening\n");
    true
}

/// Assert the ring points at a live, focusable, visible node in the current
/// tree — the "without loss" invariant that must hold after every close and
/// settle, and after every arrow press.
fn focus_valid(d: &Desktop, test: &str, step: &str) -> bool {
    let Some(f) = d.focus.focused() else {
        io::print_str(&alloc::format!(
            "[test] FAIL {}: {} — focus lost\n",
            test,
            step
        ));
        return false;
    };
    if !d
        .a11y_tree
        .nodes
        .iter()
        .any(|n| n.id == f && n.focusable && n.state.visible)
    {
        io::print_str(&alloc::format!(
            "[test] FAIL {}: {} — focus stale\n",
            test,
            step
        ));
        return false;
    }
    a11y_log_pass(test, step);
    true
}

/// Walk the a11y ring via arrow probes until the focused node is the Close
/// button of `wid`. Each probe round tries Up, Right, Down, Left and breaks
/// on the first move — a deterministic nearest-neighbor step of `move_focus`,
/// so repeated probes converge on the reachable target. Returns false if the
/// target is unreachable or all four directions dead-end.
fn probe_to_close(d: &mut Desktop, wid: WindowId) -> bool {
    for _ in 0..48 {
        if d.focus.focused().is_some_and(|f| {
            d.a11y_tree.nodes.iter().any(|n| {
                n.id == f
                    && n.role == A11yRole::Button
                    && n.label == "Close"
                    && n.owner == Some(wid)
            })
        }) {
            return true;
        }
        let before = d.focus.focused();
        let mut moved = false;
        for scan in [
            keys::SCAN_UP,
            keys::SCAN_RIGHT,
            keys::SCAN_DOWN,
            keys::SCAN_LEFT,
        ] {
            d.handle_event(Event::Key(scan as u16));
            if d.focus.focused() != before {
                moved = true;
                break;
            }
        }
        if !moved {
            return false; // all four directions dead-end
        }
    }
    false
}

/// One full keyboard-loop phase: probe the ring to `wid`'s Close button,
/// assert it is there, Enter closes the window, assert the re-sync, settle
/// the close animation, and assert the window is gone with the ring still
/// alive. Returns false on any failure.
fn close_via_ring(d: &mut Desktop, wid: WindowId, name: &str) -> bool {
    let test = "test_a11y_full_keyboard_loop";
    if !probe_to_close(d, wid) {
        io::print_str(&alloc::format!(
            "[test] FAIL {}: could not reach Close ({})\n",
            test,
            name
        ));
        return false;
    }
    let fid = d.focus.focused().unwrap();
    if !d
        .a11y_tree
        .nodes
        .iter()
        .any(|n| n.id == fid && n.owner == Some(wid))
    {
        io::print_str(&alloc::format!(
            "[test] FAIL {}: ring not on {}'s Close\n",
            test,
            name
        ));
        return false;
    }
    a11y_log_pass(test, &alloc::format!("arrows reach {}'s Close", name));
    d.handle_event(Event::Key(keys::SCAN_ENTER as u16)); // closes the window
    if !focus_valid(d, test, &alloc::format!("re-sync after {} close", name)) {
        return false;
    }
    for _ in 0..60 {
        d.tick();
    }
    if d.wm.lookup(wid).is_some() {
        io::print_str(&alloc::format!(
            "[test] FAIL {}: {} not removed\n",
            test,
            name
        ));
        return false;
    }
    if !focus_valid(d, test, &alloc::format!("{} gone, ring survives", name)) {
        return false;
    }
    true
}

/// The full a11y keyboard loop, end to end: the ring starts on the first
/// arrow, arrows walk to a window's Close button, Enter closes the window,
/// focus re-syncs to a sibling surface, and arrows keep navigating without
/// loss — cycling through several windows closed one by one, down to the
/// Taskbar fallback.
///
/// Geometry is chosen so every step is deterministic (verified against the
/// exact `move_focus` rect math): wide-flat windows (w=400, h=110 — the
/// Right filter reaches the window chrome only when h < w/2 - 13) on an
/// 1800x700 desktop. Each Close is reachable via Right hops — Window ->
/// Minimize -> Close, since the Minimize chrome node sits nearest the
/// Window node's center — and the probe re-probes when it lands on a
/// Minimize or another window's Close, so the extra hop is absorbed. The
/// rightmost window's close is reached first (it has no right neighbor to
/// reach it later), so the cycle closes C, then B, then A.
///
/// The probe paths are pinned to the taskbar-button and close-button rects
/// in `layout` (`TASKBAR_BTN_X0`/`TASKBAR_BTN_PITCH`, `close_btn_rect`) and
/// the window rects below — a layout change that shifts the tree may
/// surface as a bare "could not reach Close (X)" failure.
pub(crate) fn test_a11y_full_keyboard_loop() -> bool {
    let mut d = Desktop::new(1800, 700);
    d.desktop_icons.icons.clear(); // icons would pollute the spatial nav
    let a = d.wm.create(AppWindow::new(30, 120, 400, 110, "WinA"));
    let b = d.wm.create(AppWindow::new(670, 120, 400, 110, "WinB"));
    let c = d.wm.create(AppWindow::new(1310, 120, 400, 110, "WinC"));
    d.tick();

    // The ring appears on the first arrow press (auto-starts on First).
    d.handle_event(Event::Key(keys::SCAN_DOWN as u16));
    if !d.focus_visible || d.focus.focused().is_none() {
        io::print_str("[test] FAIL test_a11y_full_keyboard_loop: ring did not start\n");
        return false;
    }
    a11y_log_pass("test_a11y_full_keyboard_loop", "ring starts on first arrow");

    // Close C (rightmost) first — its Close has no right neighbor to reach
    // it otherwise — then B, then A, each via the ring.
    if !close_via_ring(&mut d, c, "C") {
        return false;
    }
    // Arrows still navigate without loss right after a close.
    d.handle_event(Event::Key(keys::SCAN_RIGHT as u16));
    if !focus_valid(
        &d,
        "test_a11y_full_keyboard_loop",
        "arrow moves after C close",
    ) {
        return false;
    }
    if !close_via_ring(&mut d, b, "B") {
        return false;
    }
    if !close_via_ring(&mut d, a, "A") {
        return false;
    }

    // All three closed: the ring survives on the Taskbar/Start fallback and
    // still has bounds to draw, and arrows still navigate without loss.
    if !d.wm.is_empty() {
        io::print_str("[test] FAIL test_a11y_full_keyboard_loop: desktop not empty\n");
        return false;
    }
    if !focus_valid(
        &d,
        "test_a11y_full_keyboard_loop",
        "fallback after all closed",
    ) {
        return false;
    }
    if RenderSnapshot::from(&d).focused_bounds.is_none() {
        io::print_str("[test] FAIL test_a11y_full_keyboard_loop: ring has no bounds at end\n");
        return false;
    }
    d.handle_event(Event::Key(keys::SCAN_LEFT as u16));
    if !focus_valid(&d, "test_a11y_full_keyboard_loop", "arrow on empty desktop") {
        return false;
    }

    io::print_str("[test] PASS test_a11y_full_keyboard_loop\n");
    true
}

/// Arrow navigation from the byte stream — the userspace half of the §2.1
/// un-gate. The kernel's RawKey drop arm (gui/window.rs `RawKey(_k) => {}`)
/// is what blocks arrows today; once it forwards the E0 second bytes
/// (kernel-keyboard-gate.md §2.1), arrow bytes 72/75/77/80 arrive in the
/// key stream and `handle_a11y_key`'s SCAN_* arms drive the ring — this
/// test proves that path end-to-end (real `Event::Key` -> `handle_a11y_key`
/// -> `move_focus`) so the kernel change can land without a userspace
/// surprise. The test_keymap arrow legs pin the byte-level contract
/// (untouched decode, unbound, ungrabbed, ASCII collision); this test pins
/// the ring-level behavior.
///
/// Also pins the shifted-arrow inertness caveat: once the kernel packs
/// modifier bits (Design A), Shift+Arrow arrives as `code | (1<<10)` and
/// the a11y modifier guard (`key & 0xFF00 != 0`) swallows it before the
/// arrow arms — a shifted arrow must NOT move the ring, dismiss it, or type
/// anything. The same guard class covers ctrl (bit9) and alt (bit8)
/// variants. Only plain arrows navigate.
pub(crate) fn test_a11y_arrows_from_byte_stream() -> bool {
    let mut d = Desktop::new(1800, 700);
    d.desktop_icons.icons.clear(); // icons would pollute the spatial nav
    d.wm.create(AppWindow::new(30, 120, 400, 110, "WinA"));
    d.wm.create(AppWindow::new(670, 120, 400, 110, "WinB"));
    d.tick();

    // The ring activates on the first plain arrow (auto-starts on First —
    // the Taskbar, per the spatial tree order).
    d.handle_event(Event::Key(keys::SCAN_DOWN as u16));
    if !d.focus_visible || d.focus.focused().is_none() {
        io::print_str(
            "[test] FAIL test_a11y_arrows_from_byte_stream: ring did not start on first arrow\n",
        );
        return false;
    }
    let start = d.focus.focused().unwrap();

    // A plain arrow MOVES the ring: Up from the bottom taskbar lands on the
    // nearest window above (WinB). This is the byte-deliverable navigation
    // the §2.1 forward set unblocks.
    d.handle_event(Event::Key(keys::SCAN_UP as u16));
    if d.focus.focused() == Some(start) {
        io::print_str("[test] FAIL test_a11y_arrows_from_byte_stream: Up did not move the ring\n");
        return false;
    }
    if !focus_valid(&d, "test_a11y_arrows_from_byte_stream", "after Up") {
        return false;
    }

    // Every direction keeps the ring valid — no loss — and the arm consumes
    // the key (it never reaches the keymap/typing path).
    for code in [
        keys::SCAN_UP,
        keys::SCAN_DOWN,
        keys::SCAN_LEFT,
        keys::SCAN_RIGHT,
    ] {
        d.handle_event(Event::Key(code as u16));
        if !focus_valid(&d, "test_a11y_arrows_from_byte_stream", "direction sweep") {
            return false;
        }
    }

    // Shifted-arrow inertness: with the ring active, a modified arrow (the
    // packed stream's `code | (1<<10)` for shift, Design A bit layout) must
    // leave the ring exactly where it was — the `0xFF00` a11y modifier guard
    // returns false and the keymap has no arrow rows, so nothing moves,
    // nothing dismisses, and nothing types. Ctrl (bit9) and Alt (bit8) are
    // the same guard class.
    let before = d.focus.focused();
    let before_visible = d.focus_visible;
    for code in [
        keys::SCAN_UP,
        keys::SCAN_DOWN,
        keys::SCAN_LEFT,
        keys::SCAN_RIGHT,
    ] {
        for bit in [8u16, 9, 10] {
            let raw = code as u16 | (1 << bit);
            d.handle_event(Event::Key(raw));
            if d.focus.focused() != before || d.focus_visible != before_visible {
                io::print_str(&alloc::format!(
                    "[test] FAIL test_a11y_arrows_from_byte_stream: modified arrow 0x{:04x} disturbed the ring — only plain arrows navigate\n",
                    raw
                ));
                return false;
            }
        }
    }

    io::print_str("[test] PASS test_a11y_arrows_from_byte_stream\n");
    true
}

/// The tray panel is modeled in the a11y tree as one owner-stamped role
/// node spanning the whole panel (entries + clock) — the same
/// `tray_panel_rect` the taskbar draws and `hover_target` hit-tests — and
/// it is non-focusable (a status surface, not a keyboard control, so it
/// stays out of the ring). Hovering a tray entry reports through the
/// unified hover state.
pub(crate) fn test_a11y_tray_panel() -> bool {
    let mut d = Desktop::new(800, 600);
    d.tick();
    let ty = d.taskbar_y();
    let tray_len = d.tray.entries.len() as u32;

    // The tree carries exactly one tray-panel node, owner-stamped with the
    // sentinel and covering the drawn panel rect.
    let panel = d
        .a11y_tree
        .nodes
        .iter()
        .find(|n| n.role == A11yRole::TrayPanel);
    let Some(panel) = panel else {
        io::print_str("[test] FAIL test_a11y_tray_panel: no TrayPanel node\n");
        return false;
    };
    if panel.owner != Some(TRAY_PANEL_OWNER) {
        io::print_str("[test] FAIL test_a11y_tray_panel: tray panel owner wrong\n");
        return false;
    }
    if panel.bounds != layout::tray_panel_rect(ty, d.screen_w, tray_len) {
        io::print_str("[test] FAIL test_a11y_tray_panel: tray panel bounds wrong\n");
        return false;
    }
    if panel.focusable {
        io::print_str("[test] FAIL test_a11y_tray_panel: tray panel focusable\n");
        return false;
    }
    a11y_log_pass("test_a11y_tray_panel", "owner-stamped TrayPanel node");

    // Hover on entry 0 reports the unified tray hover, resolved through the
    // same panel-derived entry rect the draw uses.
    let tr = layout::tray_entry_rect(0, ty, d.screen_w, tray_len);
    d.update_mouse(tr.x + tr.w as i32 / 2, tr.y + tr.h as i32 / 2, false);
    if d.snapshot().hover != Some(HoverTarget::Tray(0)) {
        io::print_str("[test] FAIL test_a11y_tray_panel: hover on entry 0 wrong\n");
        return false;
    }
    a11y_log_pass("test_a11y_tray_panel", "hover on entry 0");

    io::print_str("[test] PASS test_a11y_tray_panel\n");
    true
}

/// The full keyboard chain that opens a window on today's kernel — the
/// exact sequence qemu_gui_login.exp drives on real input (Tab → Enter →
/// type → Enter). Arrows (E0-extended) are dropped on today's kernel, so
/// Tab's FocusNext fallback is the ONLY way the ring starts on an empty
/// desktop; and the start-menu Enter-launch arm below is the ONLY way a
/// typed search opens a window (`handle_key`'s menu-Enter arm is dead on
/// the real event path because `handle_a11y_key` consumes Enter first).
/// Pins both userspace halves so the QEMU leg has a synthetic mirror.
pub(crate) fn test_a11y_keyboard_window_open() -> bool {
    let mut d = Desktop::new(800, 600);
    d.tick();

    // Tab on an empty desktop starts the ring on the first focusable node
    // (the Taskbar) — not an orphaned ring (wm.focus_next() returns false
    // with no windows; the FocusNext fallback must land it on First).
    d.handle_event(Event::Key(keys::KEY_TAB as u16));
    if !d.focus_visible || d.focus.focused().is_none() {
        io::print_str(
            "[test] FAIL test_a11y_keyboard_window_open: Tab did not start ring on empty desktop\n",
        );
        return false;
    }
    a11y_log_pass(
        "test_a11y_keyboard_window_open",
        "Tab starts ring on Taskbar",
    );

    // Enter on the focused Taskbar node opens the start menu (Taskbar role
    // activation toggles it — the same toggle as the Start button).
    d.handle_event(Event::Key(keys::KEY_ENTER as u16));
    if !d.start_menu.open {
        io::print_str(
            "[test] FAIL test_a11y_keyboard_window_open: Enter did not open start menu\n",
        );
        return false;
    }
    a11y_log_pass("test_a11y_keyboard_window_open", "Enter opens menu");

    // Typing filters the catalog; "term" uniquely matches Terminal.
    for b in b"term" {
        d.handle_event(Event::Key(*b as u16));
    }
    let sel = match d.start_menu.selected_app() {
        Some(s) => s,
        None => {
            io::print_str(
                "[test] FAIL test_a11y_keyboard_window_open: no app selected after search\n",
            );
            return false;
        }
    };
    if d.app_reg.get(sel).map(|a| a.name) != Some("Terminal") {
        io::print_str(
            "[test] FAIL test_a11y_keyboard_window_open: search did not select Terminal\n",
        );
        return false;
    }
    a11y_log_pass(
        "test_a11y_keyboard_window_open",
        "search filters to Terminal",
    );

    // Enter with a typed search launches the selected app: a window
    // appears (the QEMU leg waits for the `[ade] launched Terminal` serial
    // marker). The empty-search dismiss/toggle behavior is pinned by
    // test_a11y_close_button; this arm only fires with a typed search.
    let before = d.wm.len();
    d.handle_event(Event::Key(keys::KEY_ENTER as u16));
    if d.wm.len() != before + 1 || d.start_menu.open {
        io::print_str(
            "[test] FAIL test_a11y_keyboard_window_open: Enter did not launch Terminal\n",
        );
        return false;
    }
    a11y_log_pass("test_a11y_keyboard_window_open", "Enter launches window");

    // Close the spawned window and settle so later tests start clean.
    if let Some(active) = d.wm.active() {
        d.wm.close(active);
    }
    for _ in 0..60 {
        d.tick();
    }

    io::print_str("[test] PASS test_a11y_keyboard_window_open\n");
    true
}
