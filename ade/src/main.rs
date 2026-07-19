//! ADE desktop entrypoint — init, event loop, rendering.

#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{gui::Window, sarga_main};
use libsarga::io;
use desktop::Desktop;

mod app_db;
mod app_registry;
mod constants;
mod damage;
mod desktop;
mod desktop_icons;
mod event;
mod file_assoc;
mod notification;
mod geometry;
mod perms;
mod window;
mod taskbar;
mod tray;
mod settings;
mod vfs;
mod watcher;
mod start_menu;
mod wallpaper;
mod icons;
mod ipc;
mod launcher;
mod login_session;
mod lifecycle;
mod window_manager;
mod shortcut;
mod theme_service;
mod clipboard_service;
mod config;
mod session_service;
mod recovery;
mod render;
mod service_manager;
mod session;


fn user_main() -> i32 {
    io::print_str("[ade] starting desktop environment\n");

    let mut desktop_win = match Window::create("SARGA OS Desktop", 1024, 768) {
        Ok(w) => w,
        Err(e) => {
            io::print_str(&alloc::format!("[ade] failed to create window: {}\n", e));
            return 0;
        }
    };

    let mut desktop = Desktop::new(desktop_win.width, desktop_win.height);
    desktop.spawn_app("/bin/sash", "Terminal");
    io::print_str("[ade] desktop running\n");

    loop {
        desktop.tick();

        while let Some(key) = desktop_win.get_key() {
            desktop.handle_event(event::Event::Key(key));
        }

        let ms = desktop_win.get_mouse();
        let (pressed, released, dragging) = desktop.update_mouse(ms.x as i32, ms.y as i32, ms.buttons & 1 != 0);
        if ms.scroll != 0 {
            desktop.handle_event(event::Event::Scroll(ms.scroll));
        }
        if pressed {
            desktop.handle_event(event::Event::MouseClick(ms.x as i32, ms.y as i32));
        } else if ms.buttons & 4 != 0 {
            desktop.handle_event(event::Event::MouseMiddle(ms.x as i32, ms.y as i32));
        } else if ms.buttons & 2 != 0 {
            desktop.handle_event(event::Event::MouseRight(ms.x as i32, ms.y as i32));
        } else if dragging {
            desktop.handle_event(event::Event::MouseDrag(ms.x as i32, ms.y as i32));
        }
        if released {
            desktop.handle_event(event::Event::MouseRelease);
        }

        if desktop.damage.is_dirty() {
            let clock_str = desktop.prepare_clock();
            let snap = desktop.snapshot();
            render::render(&mut desktop_win, &snap, &clock_str);
            if let Err(e) = desktop_win.flush() {
                io::print_str(&alloc::format!("[ade] flush error: {}\n", e));
            }
            desktop.damage.clear();
        }
        unsafe {
            libsarga::syscall::syscall2(35, 0, 16_000_000u64);
        }
    }
}

sarga_main!(user_main);
