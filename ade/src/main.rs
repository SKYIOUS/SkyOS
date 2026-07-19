#![no_std]
#![no_main]
extern crate alloc;
use libsarga::theme::Theme;
use libsarga::{gui::Window, sarga_main};
use libsarga::{io, process};
use desktop::Desktop;
use crate::wallpaper::draw;
use crate::window::{AppWindow, WindowState};

mod constants;
mod desktop;
mod window;
mod taskbar;
mod start_menu;
mod wallpaper;
mod icons;
mod window_manager;
mod render;





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
            if key == b'q' && desktop.wm.is_empty() {
                return 0;
            }
            desktop.dirty = true;
            if let Some(last) = desktop.wm.last_mut() {
                if last.focused && last.x > -100 {
                    let ch = key as char;
                    if ch.is_ascii_graphic() || ch == ' ' {
                        if last.content.last().map_or(true, |l| l.len() > 80) {
                            last.content.push(alloc::string::String::new());
                        }
                        if let Some(line) = last.content.last_mut() {
                            line.push(ch);
                        }
                    } else if key == 0x0A || key == 0x0D {
                        if let Some(line) = last.content.last_mut() {
                            let cmd = line.clone();
                            last.content.push(alloc::format!("$ {}", cmd));
                        }
                    } else if key == 0x7F || key == 0x08 {
                        if let Some(line) = last.content.last_mut() {
                            line.pop();
                        }
                    }
                }
            }
        }

        let ms = desktop_win.get_mouse();
        let (pressed, released) = desktop.update_mouse(ms.x as i32, ms.y as i32, ms.buttons & 1 != 0);
        if pressed {
            desktop.handle_click(ms.x as i32, ms.y as i32);
        } else if ms.buttons & 1 != 0 {
            desktop.handle_drag(ms.x as i32, ms.y as i32);
        }
        if released {
            desktop.release_drag();
        }

        if desktop.dirty {
            render::render(&mut desktop_win, &desktop);
            let _ = desktop_win.flush();
            desktop.dirty = false;
        }
        unsafe {
            libsarga::syscall::syscall2(35, 0, 16_000_000u64);
        }
    }
}

sarga_main!(user_main);
