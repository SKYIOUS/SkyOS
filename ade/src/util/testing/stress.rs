//! Stress suite — window-churn and notification-flood pressure tests.
//!
//! These run inside `run_all` (QEMU `--selftest`) and assert structural
//! invariants the functional tests don't: create/close hundreds of windows
//! without leaking them, focus-flip through a stack, and flood the
//! notification queue past its cap.
//!
//! Close idiom: `WindowManager::close` starts an animated shrink and the
//! window only leaves the list when `process_closing` (called from
//! `Desktop::tick`) sees the animation finish. Count assertions therefore
//! tick the desktop to drain before comparing lengths — the same settle
//! loop `launcher::check_spawn_registers` uses.

use crate::core::desktop::Desktop;
use crate::core::window::AppWindow;
use alloc::vec::Vec;
use libsarga::io;

/// Drain pending close animations and expired notifications so the window
/// count is a stable baseline.
fn settle(desktop: &mut Desktop) {
    for _ in 0..60 {
        desktop.tick();
    }
}

pub(crate) fn run_stress_tests(desktop: &mut Desktop) -> bool {
    let mut ok = true;
    ok &= test_100_windows(desktop);
    ok &= test_rapid_focus(desktop);
    ok &= test_1000_notifications(desktop);
    ok
}

pub(crate) fn test_100_windows(desktop: &mut Desktop) -> bool {
    settle(desktop);
    let before = desktop.wm.len();
    let mut ids = Vec::new();
    for i in 0..100 {
        let x = 10 + (i % 10) * 30;
        let y = 10 + (i / 10) * 20;
        let win = AppWindow::new(x, y, 200, 150, "StressWin");
        ids.push(desktop.wm.create(win));
    }
    if desktop.wm.len() != before + 100 {
        io::print_str(&alloc::format!(
            "[test] FAIL test_100_windows: expected {} windows, got {}\n",
            before + 100,
            desktop.wm.len()
        ));
        return false;
    }
    for id in ids {
        desktop.wm.close(id);
    }
    settle(desktop);
    if desktop.wm.len() != before {
        io::print_str(&alloc::format!(
            "[test] FAIL test_100_windows: close left {} windows, expected {}\n",
            desktop.wm.len(),
            before
        ));
        return false;
    }
    io::print_str("[test] PASS test_100_windows (100 created + closed, no leak)\n");
    true
}

pub(crate) fn test_rapid_focus(desktop: &mut Desktop) -> bool {
    settle(desktop);
    let before = desktop.wm.len();
    let mut ids = Vec::new();
    for _ in 0..50 {
        let win = AppWindow::new(100, 100, 300, 200, "FocusWin");
        ids.push(desktop.wm.create(win));
    }
    for &id in &ids {
        desktop.wm.bring_to_front(id);
    }
    if desktop.wm.active() != ids.last().copied() {
        io::print_str("[test] FAIL test_rapid_focus: last focused window is not active\n");
        return false;
    }
    for id in ids {
        desktop.wm.close(id);
    }
    settle(desktop);
    if desktop.wm.len() != before {
        io::print_str(&alloc::format!(
            "[test] FAIL test_rapid_focus: close left {} windows, expected {}\n",
            desktop.wm.len(),
            before
        ));
        return false;
    }
    io::print_str("[test] PASS test_rapid_focus (50 focus flips, no leak)\n");
    true
}

pub(crate) fn test_1000_notifications(desktop: &mut Desktop) -> bool {
    settle(desktop);
    for i in 0..1000 {
        desktop
            .services
            .notifications
            .notify(&alloc::format!("N{}", i), "stress body", 0, 0);
    }
    // The queue caps at 64, dropping the oldest past the cap.
    let visible = desktop.services.notifications.visible_notifications().len();
    if visible != 64 {
        io::print_str(&alloc::format!(
            "[test] FAIL test_1000_notifications: expected 64 visible (cap), got {}\n",
            visible
        ));
        return false;
    }
    desktop.services.notifications.dismiss_all();
    if !desktop
        .services
        .notifications
        .visible_notifications()
        .is_empty()
    {
        io::print_str("[test] FAIL test_1000_notifications: dismiss_all left visible entries\n");
        return false;
    }
    io::print_str("[test] PASS test_1000_notifications (1000 queued, capped at 64, cleared)\n");
    true
}
