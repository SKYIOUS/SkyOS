#![allow(dead_code)]

use crate::core::desktop::Desktop;
use crate::core::window::{AppWindow, VisualFlags, WindowState};
use crate::util::benchmark::BenchmarkResult;
use alloc::string::String;
use alloc::vec::Vec;
use libsarga::io;

pub(crate) fn bench_create_destroy(desktop: &mut Desktop) -> BenchmarkResult {
    let n = 50;
    let start = desktop.clock_ticks;

    let mut ids = Vec::new();
    for i in 0..n {
        let win = AppWindow {
            x: i * 10,
            y: i * 10,
            w: 200,
            h: 150,
            prev_x: 0,
            prev_y: 0,
            prev_w: 200,
            prev_h: 150,
            title: String::from("Bench"),
            content: Vec::new(),
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
        ids.push(desktop.wm.create(win));
    }
    for id in ids {
        desktop.wm.close(id);
    }

    let elapsed = desktop.clock_ticks - start;
    io::print_str(&alloc::format!(
        "[bench] window_create_destroy: {} iterations in {} ticks\n",
        n,
        elapsed
    ));
    BenchmarkResult {
        name: "window_create_destroy",
        duration_ticks: elapsed,
        allocation_count: 0,
        memory_delta: 0,
    }
}
