#![allow(dead_code)]

use alloc::string::String;
use crate::core::desktop::Desktop;
use crate::ipc::message::{IpcTarget, MessageBus};
use crate::core::window::{AppWindow, VisualFlags, WindowState};
use alloc::vec::Vec;
use libsarga::io;

pub(crate) fn run_regression_suite(desktop: &mut Desktop) -> bool {
    let mut ok = true;
    ok &= test_window_create_close(desktop);
    ok &= test_window_focus(desktop);
    ok &= test_drag(desktop);
    ok &= test_start_menu(desktop);
    ok &= test_notifications(desktop);
    ok &= test_clipboard(desktop);
    ok &= test_ipc();
    ok &= test_theme(desktop);
    ok
}

fn test_window_create_close(desktop: &mut Desktop) -> bool {
    let before = desktop.wm.len();
    let win = AppWindow {
        x: 10, y: 10, w: 200, h: 150,
        prev_x: 10, prev_y: 10, prev_w: 200, prev_h: 150,
        title: String::from("RegWin"), content: alloc::vec::Vec::new(),
        scroll: 0, pid: None, focused: true, dragging: false,
        drag_ox: 0, drag_oy: 0, state: WindowState::Normal,
        prev_state: WindowState::Normal, flags: VisualFlags::new(),
        selection: None, anim: None, closing: false, anim_opacity: 0,
        always_on_top: false, explorer_id: None,
    };
    let id = desktop.wm.create(win);
    if desktop.wm.len() != before + 1 {
        io::print_str("[regression] FAIL window_create_close\n");
        return false;
    }
    desktop.wm.close(id);
    if desktop.wm.len() != before {
        io::print_str("[regression] FAIL window_create_close: close\n");
        return false;
    }
    io::print_str("[regression] PASS window_create_close\n");
    true
}

fn test_window_focus(desktop: &mut Desktop) -> bool {
    let win_a = AppWindow {
        x: 20, y: 20, w: 200, h: 150,
        prev_x: 20, prev_y: 20, prev_w: 200, prev_h: 150,
        title: String::from("RegA"), content: alloc::vec::Vec::new(),
        scroll: 0, pid: None, focused: true, dragging: false,
        drag_ox: 0, drag_oy: 0, state: WindowState::Normal,
        prev_state: WindowState::Normal, flags: VisualFlags::new(),
        selection: None, anim: None, closing: false, anim_opacity: 0,
        always_on_top: false, explorer_id: None,
    };
    let win_b = AppWindow {
        x: 100, y: 100, w: 200, h: 150,
        prev_x: 100, prev_y: 100, prev_w: 200, prev_h: 150,
        title: String::from("RegB"), content: alloc::vec::Vec::new(),
        scroll: 0, pid: None, focused: false, dragging: false,
        drag_ox: 0, drag_oy: 0, state: WindowState::Normal,
        prev_state: WindowState::Normal, flags: VisualFlags::new(),
        selection: None, anim: None, closing: false, anim_opacity: 0,
        always_on_top: false, explorer_id: None,
    };
    let id_a = desktop.wm.create(win_a);
    let _id_b = desktop.wm.create(win_b);
    desktop.wm.bring_to_front(id_a);
    desktop.wm.close(id_a);
    desktop.wm.close(desktop.wm.active().unwrap());
    io::print_str("[regression] PASS window_focus\n");
    return true;
}

fn test_drag(desktop: &mut Desktop) -> bool {
    let win = AppWindow {
        x: 30, y: 30, w: 200, h: 150,
        prev_x: 30, prev_y: 30, prev_w: 200, prev_h: 150,
        title: String::from("DragWin"), content: alloc::vec::Vec::new(),
        scroll: 0, pid: None, focused: true, dragging: false,
        drag_ox: 0, drag_oy: 0, state: WindowState::Normal,
        prev_state: WindowState::Normal, flags: VisualFlags::new(),
        selection: None, anim: None, closing: false, anim_opacity: 0,
        always_on_top: false, explorer_id: None,
    };
    let id = desktop.wm.create(win);
    desktop.wm.begin_drag(id, 40, 40);
    desktop.wm.close(id);
    io::print_str("[regression] PASS drag\n");
    true
}

fn test_start_menu(desktop: &mut Desktop) -> bool {
    desktop.start_menu.open_with(&desktop.app_reg);
    if !desktop.start_menu.open {
        io::print_str("[regression] FAIL start_menu\n");
        return false;
    }
    desktop.start_menu.open = false;
    io::print_str("[regression] PASS start_menu\n");
    true
}

fn test_notifications(desktop: &mut Desktop) -> bool {
    let id = desktop.services.notifications.notify("Reg Title", "Reg Body", 1, 60);
    if id == 0 {
        io::print_str("[regression] FAIL notifications\n");
        return false;
    }
    desktop.services.notifications.dismiss(id);
    io::print_str("[regression] PASS notifications\n");
    true
}

fn test_clipboard(desktop: &mut Desktop) -> bool {
    desktop.services.clipboard.copy("regression data", 0);
    let data = desktop.services.clipboard.paste();
    if data != "regression data" {
        io::print_str("[regression] FAIL clipboard\n");
        return false;
    }
    desktop.services.clipboard.clear();
    io::print_str("[regression] PASS clipboard\n");
    true
}

fn test_ipc() -> bool {
    let mut bus = MessageBus::new();
    let seq = bus.request(IpcTarget::Desktop, "ping", Vec::new());
    if seq == 0 {
        io::print_str("[regression] FAIL ipc\n");
        return false;
    }
    bus.respond(seq, true, Vec::new());
    let drained = bus.drain();
    if drained.len() != 2 {
        io::print_str("[regression] FAIL ipc: drain count\n");
        return false;
    }
    io::print_str("[regression] PASS ipc\n");
    true
}

fn test_theme(desktop: &mut Desktop) -> bool {
    let theme = desktop.theme_svc.current();
    if theme.accent == 0 {
        io::print_str("[regression] FAIL theme: accent is 0\n");
        return false;
    }
    io::print_str("[regression] PASS theme\n");
    true
}
