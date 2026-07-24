#![allow(dead_code)]

use crate::core::window::{VisualFlags, WindowState};
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
