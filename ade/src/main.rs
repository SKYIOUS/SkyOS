//! ADE desktop entrypoint — init, event loop, rendering.

#![no_std]
#![no_main]
extern crate alloc;
use core::desktop::Desktop;
use libsarga::io;
use libsarga::{gui::Window, sarga_main};

mod app_manifest;
mod apps;
mod core;
mod ipc;
mod render;
mod sec;
mod service;
mod sys;
mod util;
use render::compositor::Compositor;

fn user_main() -> i32 {
    io::print_str("[ade] starting desktop environment\n");

    let mut desktop_win = match Window::create("SARGA OS Desktop", 800, 600) {
        Ok(w) => w,
        Err(e) => {
            io::print_str(&alloc::format!("[ade] failed to create window: {}\n", e));
            return 0;
        }
    };

    let mut desktop = Desktop::new(desktop_win.width, desktop_win.height);
    let mut compositor = match Compositor::new(desktop_win.width, desktop_win.height) {
        Some(c) => c,
        None => {
            io::print_str("[ade] failed to allocate compositor buffers\n");
            return 0;
        }
    };
    if (0..libsarga::args::argc()).any(|i| libsarga::args::get(i as usize) == Some("--selftest")) {
        let ok = util::testing::run_all(&mut desktop);
        io::print_str(if ok {
            "[ade] selftest PASS\n"
        } else {
            "[ade] selftest FAIL\n"
        });
    }
    // Session lifecycle: desktop environment session established
    let _ = io::write_all(1, b"[ade] session established\n");
    // ponytail: terminal auto-launch removed — opens on icon click instead
    io::print_str("[ade] desktop running\n");

    let mut running = true;
    let mut last_frame_ticks = 0u64;
    while running {
        desktop.tick();

        while let Some(key) = desktop_win.get_key() {
            // Session lifecycle: Ctrl+Alt+Backspace → clean session end
            if key == 0x7F || key == 0x08 {
                io::print_str("[ade] session ending via keyboard\n");
                running = false;
                break;
            }
            desktop.handle_event(core::event::Event::Key(key));
        }
        if !running {
            break;
        }

        let ms = desktop_win.get_mouse();
        let (pressed, released, dragging) =
            desktop.update_mouse(ms.x as i32, ms.y as i32, ms.buttons & 1 != 0);
        if ms.scroll != 0 {
            desktop.handle_event(core::event::Event::Scroll(ms.scroll));
        }
        if pressed {
            desktop.handle_event(core::event::Event::MouseClick(ms.x as i32, ms.y as i32));
        } else if ms.buttons & 4 != 0 {
            desktop.handle_event(core::event::Event::MouseMiddle(ms.x as i32, ms.y as i32));
        } else if ms.buttons & 2 != 0 {
            desktop.handle_event(core::event::Event::MouseRight(ms.x as i32, ms.y as i32));
        } else if dragging {
            desktop.handle_event(core::event::Event::MouseDrag(ms.x as i32, ms.y as i32));
        }
        if released {
            desktop.handle_event(core::event::Event::MouseRelease);
        }

        if desktop.damage.is_dirty() {
            let clock_str = desktop.prepare_clock();
            let snap = desktop.snapshot();
            render::render(&mut desktop_win, &snap, &clock_str, &mut compositor);
            if let Err(e) = desktop_win.flush() {
                io::print_str(&alloc::format!("[ade] flush error: {}\n", e));
            }
            desktop.damage.clear();
        }
        // ponytail: adaptive frame pacing — shorten sleep if frame took longer
        let frame_ticks = desktop.clock_ticks - last_frame_ticks;
        last_frame_ticks = desktop.clock_ticks;
        let target_ns = 16_666_667u64;
        let elapsed_ns = frame_ticks * target_ns;
        let sleep_ns = if elapsed_ns < target_ns {
            target_ns - elapsed_ns
        } else {
            1_000_000
        };
        unsafe {
            libsarga::syscall::syscall2(35, 0, sleep_ns);
        }
    }
    // Session lifecycle: session ended — clean return
    io::print_str("[ade] session ended\n");
    0
}

sarga_main!(user_main);
