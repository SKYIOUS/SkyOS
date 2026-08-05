#![allow(dead_code)]

use crate::core::desktop::Desktop;
use crate::core::window::{AppWindow, VisualFlags, WindowState};
use crate::ipc::message::{IpcTarget, MessageBus};
use alloc::string::String;
use alloc::vec::Vec;
use libsarga::io;

pub(crate) fn run_stress_tests(desktop: &mut Desktop) -> bool {
    test_100_windows(desktop);
    test_1000_notifications(desktop);
    test_1000_ipc_messages();
    test_rapid_focus(desktop);
    true
}

pub(crate) fn test_100_windows(desktop: &mut Desktop) {
    let start = desktop.clock_ticks;
    let mut ids = Vec::new();
    for i in 0..100 {
        let win = AppWindow {
            x: 10 + (i % 10) * 30,
            y: 10 + (i / 10) * 20,
            w: 200,
            h: 150,
            prev_x: 0,
            prev_y: 0,
            prev_w: 200,
            prev_h: 150,
            title: String::from("StressWin"),
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
            pty_fd: None,
        };
        ids.push(desktop.wm.create(win));
    }
    let create_ticks = desktop.clock_ticks - start;
    io::print_str(&alloc::format!(
        "[stress] 100 windows created in {} ticks\n",
        create_ticks
    ));

    for id in ids {
        desktop.wm.close(id);
    }
    io::print_str("[stress] 100 windows closed\n");
}

pub(crate) fn test_1000_notifications(desktop: &mut Desktop) {
    for i in 0..1000 {
        desktop
            .services
            .notifications
            .notify(&alloc::format!("N{}", i), "stress body", 0, 10);
    }
    let visible = desktop.services.notifications.visible_notifications().len();
    io::print_str(&alloc::format!(
        "[stress] 1000 notifications queued, {} visible\n",
        visible
    ));
    desktop.services.notifications.dismiss_all();
}

pub(crate) fn test_1000_ipc_messages() {
    let mut bus = MessageBus::new();
    for _i in 0..1000 {
        bus.request(IpcTarget::Desktop, "ping", Vec::new());
    }
    let drained = bus.drain();
    io::print_str(&alloc::format!(
        "[stress] 1000 IPC messages sent, {} drained\n",
        drained.len()
    ));
}

pub(crate) fn test_rapid_focus(desktop: &mut Desktop) {
    let mut ids = Vec::new();
    for _ in 0..50 {
        let win = AppWindow {
            x: 100,
            y: 100,
            w: 300,
            h: 200,
            prev_x: 100,
            prev_y: 100,
            prev_w: 300,
            prev_h: 200,
            title: String::from("FocusWin"),
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
            pty_fd: None,
        };
        ids.push(desktop.wm.create(win));
    }
    for &id in &ids {
        desktop.wm.bring_to_front(id);
    }
    io::print_str(&alloc::format!(
        "[stress] rapid focus switched across {} windows\n",
        ids.len()
    ));
    for id in ids {
        desktop.wm.close(id);
    }
}
