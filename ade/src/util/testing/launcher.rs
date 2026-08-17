use crate::core::desktop::Desktop;
use crate::core::launcher::spawn_app;
use crate::service::session::AppState;
use libsarga::io;

pub(crate) fn test_spawn(desktop: &mut Desktop) -> bool {
    let before = desktop.wm.len();
    spawn_app(desktop, "", "TestSpawn");
    if desktop.wm.len() != before + 1 {
        io::print_str("[test] FAIL test_spawn: window count did not increase\n");
        return false;
    }
    if let Some(active) = desktop.wm.active() {
        desktop.wm.close(active);
    }
    // Drain the animated close before counting (see check_spawn_registers).
    for _ in 0..60 {
        desktop.tick();
    }
    if desktop.wm.len() != before {
        io::print_str("[test] FAIL test_spawn: close did not restore count\n");
        return false;
    }
    io::print_str("[test] PASS test_spawn\n");
    true
}

/// Unified launcher: every spawn path must leave exactly one new window
/// whose pid is registered with lifecycle (state Running) and has a
/// permission grant. `spawn` runs the path under test; the window is then
/// closed and ticked out so the next assertion starts from a clean count.
fn check_spawn_registers<F>(desktop: &mut Desktop, label: &str, spawn: F) -> bool
where
    F: FnOnce(&mut Desktop),
{
    // Settle first: close() is animated, so earlier tests that close without
    // ticking leave stale `closing` windows; process_closing on tick would
    // flush them mid-test and skew the count. A pre-snapshot settle loop
    // makes the baseline stable for the whole test.
    for _ in 0..60 {
        desktop.tick();
    }
    let before = desktop.wm.len();
    spawn(desktop);
    if desktop.wm.len() != before + 1 {
        io::print_str(&alloc::format!(
            "[test] FAIL test_spawn_registers ({label}): expected {} window(s), got {}\n",
            before + 1,
            desktop.wm.len()
        ));
        return false;
    }
    let id = match desktop.wm.active() {
        Some(id) => id,
        None => {
            io::print_str(&alloc::format!(
                "[test] FAIL test_spawn_registers ({label}): no active window\n"
            ));
            return false;
        }
    };
    let pid = match desktop.wm.lookup(id).and_then(|w| w.pid) {
        Some(pid) => pid,
        None => {
            io::print_str(&alloc::format!(
                "[test] FAIL test_spawn_registers ({label}): window has no pid\n"
            ));
            return false;
        }
    };
    let running = desktop
        .session
        .lifecycle
        .procs
        .iter()
        .any(|p| p.pid == pid && p.state == AppState::Running);
    if !running {
        io::print_str(&alloc::format!(
            "[test] FAIL test_spawn_registers ({label}): pid {pid} not lifecycle Running\n"
        ));
        return false;
    }
    if desktop.permissions.granted(pid).is_none() {
        io::print_str(&alloc::format!(
            "[test] FAIL test_spawn_registers ({label}): pid {pid} has no permission grant\n"
        ));
        return false;
    }
    // Close, then tick until the window is reaped/removed (animated close
    // finishes via process_closing; dead children are reaped into
    // close_by_pid). The baseline is stable now, so only this window can
    // disappear.
    desktop.wm.close(id);
    for _ in 0..60 {
        desktop.tick();
    }
    if desktop.wm.len() != before {
        io::print_str(&alloc::format!(
            "[test] FAIL test_spawn_registers ({label}): window not removed after close\n"
        ));
        return false;
    }
    true
}

pub(crate) fn test_spawn_registers(desktop: &mut Desktop) -> bool {
    if !check_spawn_registers(desktop, "terminal", |d| d.spawn_terminal()) {
        return false;
    }
    if !check_spawn_registers(desktop, "explorer", |d| d.spawn_explorer()) {
        return false;
    }
    // spawn_app with a guaranteed-absent binary still forks and registers
    // the pid (the child exec fails instantly; we assert the parent side).
    if !check_spawn_registers(desktop, "app", |d| {
        spawn_app(d, "/bin/__ade_nonexistent__", "TestSpawnReg");
    }) {
        return false;
    }
    io::print_str("[test] PASS test_spawn_registers\n");
    true
}

pub(crate) fn test_spawn_at(desktop: &mut Desktop) -> bool {
    let before = desktop.wm.len();
    crate::core::launcher::spawn_app_at(desktop, "", "TestAt", 50, 60, 300, 200);
    if desktop.wm.len() != before + 1 {
        io::print_str("[test] FAIL test_spawn_at: window count did not increase\n");
        return false;
    }
    if let Some(active) = desktop.wm.active() {
        if let Some(w) = desktop.wm.lookup(active) {
            if w.x != 50 || w.y != 60 || w.w != 300 || w.h != 200 {
                io::print_str("[test] FAIL test_spawn_at: position mismatch\n");
                return false;
            }
        } else {
            io::print_str("[test] FAIL test_spawn_at: lookup failed\n");
            return false;
        }
        desktop.wm.close(active);
    } else {
        io::print_str("[test] FAIL test_spawn_at: no active window\n");
        return false;
    }
    // Drain the animated close before counting (see check_spawn_registers).
    for _ in 0..60 {
        desktop.tick();
    }
    if desktop.wm.len() != before {
        io::print_str("[test] FAIL test_spawn_at: close did not restore count\n");
        return false;
    }
    io::print_str("[test] PASS test_spawn_at\n");
    true
}
