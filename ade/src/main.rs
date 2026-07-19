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
            if key == b'q' && desktop.windows.is_empty() {
                return 0;
            }
            if let Some(last) = desktop.windows.last_mut() {
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

        wallpaper::draw(&mut desktop_win, &desktop);

        for icon in &desktop.icons {
            icons::draw(&mut desktop_win, &desktop.theme, icon.0, icon.1, icon.2);
        }

        for aw in &desktop.windows {
            window::draw(&mut desktop_win, &desktop.theme, aw);
        }

        taskbar::draw(&mut desktop_win, &desktop.theme, &desktop);

        if desktop.start_menu {
            start_menu::draw(&mut desktop_win, &desktop.theme, &desktop);
        }

        if let Some((mx, my, items)) = desktop.context_menu {
            let mw = 150u32;
            let mh = items.len() as u32 * 28 + 10;
            desktop_win.draw_rounded_rect(
                mx as u32,
                my as u32,
                mw,
                mh,
                6,
                desktop.theme.bg_elevated,
            );
            desktop_win.draw_rounded_rect_outline(
                mx as u32,
                my as u32,
                mw,
                mh,
                6,
                desktop.theme.border,
            );
            for (i, (name, _)) in items.iter().enumerate() {
                let iy = my as u32 + 5 + i as u32 * 28;
                desktop_win.draw_string(mx as u32 + 10, iy + 6, name, desktop.theme.text, 0);
            }
        }

        let _ = desktop_win.flush();
        unsafe {
            libsarga::syscall::syscall2(35, 0, 16_000_000u64);
        }
    }
}

sarga_main!(user_main);
