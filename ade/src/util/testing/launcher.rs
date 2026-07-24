#![allow(dead_code)]

use crate::core::desktop::Desktop;
use crate::core::launcher::spawn_app;
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
    if desktop.wm.len() != before {
        io::print_str("[test] FAIL test_spawn: close did not restore count\n");
        return false;
    }
    io::print_str("[test] PASS test_spawn\n");
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
    if desktop.wm.len() != before {
        io::print_str("[test] FAIL test_spawn_at: close did not restore count\n");
        return false;
    }
    io::print_str("[test] PASS test_spawn_at\n");
    true
}
