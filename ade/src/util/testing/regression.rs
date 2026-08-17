//! Regression suite — spot checks across subsystems that a later change
//! could silently break.
//!
//! Deliberately slim: the original suite's other checks (window
//! create/close, focus, start menu, notifications, clipboard) are already
//! covered by dedicated tests in `run_all` — duplicating them here would
//! re-run the same assertions. What was unique is kept: the window drag
//! path (`begin_drag`/`update_drag`/`end_drag`, which no other test
//! exercises) and the theme service's default state. The old IPC check
//! tested the legacy `MessageBus`, deleted in Phase 1.
//!
//! Close idiom: `WindowManager::close` is animated; windows leave the list
//! only after `Desktop::tick` drains them via `process_closing`. Count
//! assertions tick first, like `launcher::check_spawn_registers`.

use crate::core::desktop::Desktop;
use crate::core::window::AppWindow;
use libsarga::io;

/// Drain pending close animations so the window count is a stable baseline.
fn settle(desktop: &mut Desktop) {
    for _ in 0..60 {
        desktop.tick();
    }
}

pub(crate) fn run_regression_suite(desktop: &mut Desktop) -> bool {
    let mut ok = true;
    ok &= test_drag(desktop);
    ok &= test_theme(desktop);
    ok
}

/// Window drag: `begin_drag` pins the grab offset; `update_drag` moves the
/// window so it tracks the cursor; `end_drag` releases the grab.
pub(crate) fn test_drag(desktop: &mut Desktop) -> bool {
    settle(desktop);
    let before = desktop.wm.len();
    let win = AppWindow::new(30, 30, 200, 150, "DragWin");
    let id = desktop.wm.create(win);

    desktop.wm.begin_drag(id, 40, 40); // grab 10px right/down of the origin
    desktop.wm.update_drag(60, 60);
    let w = desktop.wm.lookup(id).unwrap();
    if w.x != 50 || w.y != 50 {
        io::print_str(&alloc::format!(
            "[test] FAIL test_drag: expected (50, 50), got ({}, {})\n",
            w.x,
            w.y
        ));
        return false;
    }
    if !w.dragging {
        io::print_str("[test] FAIL test_drag: window not marked dragging\n");
        return false;
    }
    desktop.wm.end_drag();
    if desktop.wm.lookup(id).unwrap().dragging {
        io::print_str("[test] FAIL test_drag: end_drag did not release\n");
        return false;
    }

    desktop.wm.close(id);
    settle(desktop);
    if desktop.wm.len() != before {
        io::print_str("[test] FAIL test_drag: window leaked after close\n");
        return false;
    }
    io::print_str("[test] PASS test_drag\n");
    true
}

/// The theme service must come up with a usable theme (ConfigStore default
/// is unset → `Theme::dark()`), so a non-zero accent is the load-bearing
/// invariant behind every window titlebar draw.
pub(crate) fn test_theme(desktop: &mut Desktop) -> bool {
    let theme = desktop.theme_svc.current();
    if theme.accent == 0 {
        io::print_str("[test] FAIL test_theme: accent is 0\n");
        return false;
    }
    io::print_str("[test] PASS test_theme\n");
    true
}
