#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{sarga_main, gui::Window, theme::Theme, io, process, fs};
use alloc::string::String;
use alloc::vec::Vec;

fn user_main() -> i32 {
    let mut win = Window::create("SARGA OS Installer", 640, 480).unwrap();
    let theme = Theme::dark();
    let mut step = 0;
    let mut username = String::new();
    let mut password = String::new();
    let mut target_disk = String::from("/dev/sda");

    loop {
        let mouse = win.get_mouse();
        let m_pressed = mouse.buttons != 0;

        while let Some(key) = win.get_key() {
            if step == 2 {
                if key == 0x08 || key == 0x7F { username.pop(); }
                else if key >= 0x20 && key < 0x7F { username.push(key as char); }
            } else if step == 3 {
                if key == 0x08 || key == 0x7F { password.pop(); }
                else if key >= 0x20 && key < 0x7F { password.push(key as char); }
            }
        }

        win.clear(theme.bg_primary);
        win.draw_gradient_rect(0, 0, 640, 60, theme.accent, theme.accent_dark, false);
        win.draw_string_centered(20, "SARGA OS INSTALLATION", 0xFFFFFFFF, 0);

        match step {
            0 => {
                win.draw_string_centered(150, "Welcome to the SARGA OS Installer.", theme.text, 0);
                win.draw_string_centered(180, "This will install SARGA OS on your computer.", theme.text_secondary, 0);
                if draw_button(&mut win, &theme, 260, 300, 120, 40, "Begin", mouse) && m_pressed {
                    step = 1;
                    unsafe { libsarga::syscall::syscall1(35, 200_000_000u64); }
                }
            }
            1 => {
                win.draw_string(50, 100, "Select Target Disk:", theme.text, 0);
                win.draw_rounded_rect(50, 130, 540, 40, 6, theme.bg_surface);
                win.draw_string(70, 142, &target_disk, theme.text, 0);
                win.draw_string(50, 190, "WARNING: All data on this disk will be erased.", theme.error, 0);
                if draw_button(&mut win, &theme, 260, 300, 120, 40, "Next", mouse) && m_pressed {
                    step = 2;
                    unsafe { libsarga::syscall::syscall1(35, 200_000_000u64); }
                }
            }
            2 => {
                win.draw_string(50, 100, "Create Username:", theme.text, 0);
                win.draw_rounded_rect(50, 130, 540, 40, 6, theme.bg_surface);
                win.draw_string(70, 142, &username, theme.text, 0);
                if draw_button(&mut win, &theme, 260, 300, 120, 40, "Next", mouse) && m_pressed {
                    step = 3;
                    unsafe { libsarga::syscall::syscall1(35, 200_000_000u64); }
                }
            }
            3 => {
                win.draw_string(50, 100, "Create Password:", theme.text, 0);
                win.draw_rounded_rect(50, 130, 540, 40, 6, theme.bg_surface);
                let stars: String = core::iter::repeat('*').take(password.len()).collect();
                win.draw_string(70, 142, &stars, theme.text, 0);
                if draw_button(&mut win, &theme, 260, 300, 120, 40, "Install", mouse) && m_pressed {
                    step = 4;
                }
            }
            4 => {
                win.draw_string_centered(150, "Installing SARGA OS...", theme.text, 0);
                win.draw_rounded_rect(100, 200, 440, 20, 10, theme.bg_elevated);
                win.draw_rounded_rect(100, 200, 300, 20, 10, theme.accent); // Progress
                win.draw_string_centered(240, "Copying files to /bin...", theme.text_secondary, 0);

                if draw_button(&mut win, &theme, 260, 300, 120, 40, "Finish", mouse) && m_pressed {
                    process::exit(0);
                }
            }
            _ => {}
        }

        let _ = win.flush();
        unsafe { libsarga::syscall::syscall1(35, 16_000_000u64); }
    }
}

fn draw_button(win: &mut Window, theme: &Theme, x: u32, y: u32, w: u32, h: u32, text: &str, mouse: io::MouseState) -> bool {
    let hover = mouse.x >= x as i32 && mouse.x < (x + w) as i32 && mouse.y >= y as i32 && mouse.y < (y + h) as i32;
    let bg = if hover { theme.hover } else { theme.accent };
    win.draw_rounded_rect(x, y, w, h, 8, bg);
    win.draw_string_centered(y + (h - 14) / 2, text, 0xFFFFFFFF, 0);
    hover
}

sarga_main!(user_main);
