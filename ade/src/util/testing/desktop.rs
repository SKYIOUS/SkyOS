#![allow(dead_code)]

use crate::core::desktop::Desktop;
use crate::core::window::{AppWindow, VisualFlags, WindowState};
use alloc::string::String;
use libsarga::io;

pub(crate) fn test_window_creation(desktop: &mut Desktop) -> bool {
    let before = desktop.wm.len();
    let win = AppWindow {
        x: 100,
        y: 100,
        w: 400,
        h: 300,
        prev_x: 100,
        prev_y: 100,
        prev_w: 400,
        prev_h: 300,
        title: String::from("TestWin"),
        content: alloc::vec::Vec::new(),
        scroll: 0,
        id: 0,
        pid: None,
        focused: true,
        dragging: false,
        drag_ox: 0,
        drag_oy: 0,
        state: WindowState::Normal,
        prev_state: WindowState::Normal,
        flags: VisualFlags::new(),
        selection: None,
        anim: None,
        closing: false,
        anim_opacity: 0,
        always_on_top: false,
        explorer_id: None,
    };
    let id = desktop.wm.create(win);
    if desktop.wm.len() != before + 1 {
        io::print_str("[test] FAIL test_window_creation: wm.len() did not increase\n");
        return false;
    }
    if desktop.wm.active().is_none() {
        io::print_str("[test] FAIL test_window_creation: no active window\n");
        return false;
    }
    desktop.wm.close(id);
    if desktop.wm.len() != before {
        io::print_str("[test] FAIL test_window_creation: close did not restore count\n");
        return false;
    }
    io::print_str("[test] PASS test_window_creation\n");
    true
}

pub(crate) fn test_window_focus(desktop: &mut Desktop) -> bool {
    let before = desktop.wm.len();
    let win_a = AppWindow {
        x: 50,
        y: 50,
        w: 300,
        h: 200,
        prev_x: 50,
        prev_y: 50,
        prev_w: 300,
        prev_h: 200,
        title: String::from("FocusA"),
        content: alloc::vec::Vec::new(),
        scroll: 0,
        id: 0,
        pid: None,
        focused: true,
        dragging: false,
        drag_ox: 0,
        drag_oy: 0,
        state: WindowState::Normal,
        prev_state: WindowState::Normal,
        flags: VisualFlags::new(),
        selection: None,
        anim: None,
        closing: false,
        anim_opacity: 0,
        always_on_top: false,
        explorer_id: None,
    };
    let win_b = AppWindow {
        x: 200,
        y: 200,
        w: 300,
        h: 200,
        prev_x: 200,
        prev_y: 200,
        prev_w: 300,
        prev_h: 200,
        title: String::from("FocusB"),
        content: alloc::vec::Vec::new(),
        scroll: 0,
        id: 0,
        pid: None,
        focused: false,
        dragging: false,
        drag_ox: 0,
        drag_oy: 0,
        state: WindowState::Normal,
        prev_state: WindowState::Normal,
        flags: VisualFlags::new(),
        selection: None,
        anim: None,
        closing: false,
        anim_opacity: 0,
        always_on_top: false,
        explorer_id: None,
    };
    let id_a = desktop.wm.create(win_a);
    let id_b = desktop.wm.create(win_b);

    desktop.wm.bring_to_front(id_a);
    desktop.wm.bring_to_front(id_b);
    desktop.wm.close(id_b);
    desktop.wm.close(id_a);

    if desktop.wm.len() != before {
        io::print_str("[test] FAIL test_window_focus: windows not cleaned up\n");
        return false;
    }
    io::print_str("[test] PASS test_window_focus\n");
    true
}

pub(crate) fn test_start_menu(desktop: &mut Desktop) -> bool {
    if desktop.start_menu.open {
        io::print_str("[test] FAIL test_start_menu: menu already open\n");
        return false;
    }
    desktop.start_menu.open_with(&desktop.app_reg);
    if !desktop.start_menu.open {
        io::print_str("[test] FAIL test_start_menu: open_with did not set open=true\n");
        return false;
    }
    desktop.start_menu.open = false;
    if desktop.start_menu.open {
        io::print_str("[test] FAIL test_start_menu: could not close menu\n");
        return false;
    }
    io::print_str("[test] PASS test_start_menu\n");
    true
}
