#![allow(dead_code)]

use alloc::string::String;
use crate::core::desktop::Desktop;
use crate::core::window::{AppWindow, VisualFlags, WindowState};
use libsarga::io;

pub(crate) fn test_full_flow(desktop: &mut Desktop) -> bool {
    let before = desktop.wm.len();

    let win = AppWindow {
        x: 50, y: 50, w: 400, h: 300,
        prev_x: 50, prev_y: 50, prev_w: 400, prev_h: 300,
        title: String::from("FullFlow"),
        content: alloc::vec::Vec::new(), scroll: 0, pid: None,
        focused: true, dragging: false, drag_ox: 0, drag_oy: 0,
        state: WindowState::Normal, prev_state: WindowState::Normal,
        flags: VisualFlags::new(), selection: None, anim: None,
        closing: false, anim_opacity: 0,
        always_on_top: false, explorer_id: None,
    };
    let id = desktop.wm.create(win);
    if desktop.wm.len() != before + 1 {
        io::print_str("[test] FAIL test_full_flow: create failed\n");
        return false;
    }

    let win2 = AppWindow {
        x: 200, y: 200, w: 300, h: 200,
        prev_x: 200, prev_y: 200, prev_w: 300, prev_h: 200,
        title: String::from("FullFlow2"),
        content: alloc::vec::Vec::new(), scroll: 0, pid: None,
        focused: true, dragging: false, drag_ox: 0, drag_oy: 0,
        state: WindowState::Normal, prev_state: WindowState::Normal,
        flags: VisualFlags::new(), selection: None, anim: None,
        closing: false, anim_opacity: 0,
        always_on_top: false, explorer_id: None,
    };
    let id2 = desktop.wm.create(win2);
    if desktop.wm.len() != before + 2 {
        io::print_str("[test] FAIL test_full_flow: create second failed\n");
        return false;
    }

    desktop.wm.bring_to_front(id);

    desktop.wm.close(id);
    desktop.wm.close(id2);

    if desktop.wm.len() != before {
        io::print_str("[test] FAIL test_full_flow: close did not restore count\n");
        return false;
    }

    io::print_str("[test] PASS test_full_flow\n");
    true
}
