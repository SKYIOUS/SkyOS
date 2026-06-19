#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{sarga_main, gui::Window, theme::Theme};
use alloc::format;

fn user_main() -> i32 {
    let mut win = Window::create("System Info", 500, 400).unwrap();
    let theme = Theme::dark();

    loop {
        win.clear(theme.bg_primary);
        win.draw_gradient_rect(0, 0, 500, 100, theme.accent, theme.accent_dark, false);
        win.draw_string_centered(40, "SARGA OS", 0xFFFFFFFF, 0);

        let infos = [
            ("Version", "0.5.0 (Aurora)"),
            ("Kernel", "SARGA v1.0.2"),
            ("Arch", "x86_64"),
            ("CPU", "Intel(R) Core(TM) i9-12900K @ 3.20GHz"),
            ("Memory", "1024 MB RAM"),
            ("Graphics", "SARGA Generic Framebuffer"),
            ("Networking", "Intel E1000 (Gigabit)"),
            ("Uptime", "0 days, 2 hours, 14 mins"),
        ];

        for (i, (key, val)) in infos.iter().enumerate() {
            let y = 130 + i as u32 * 30;
            win.draw_string(30, y, key, theme.text_secondary, 0);
            win.draw_string(200, y, val, theme.text, 0);
        }

        win.draw_string_centered(370, "Copyright (c) 2026 SKYIOUS", theme.text_disabled, 0);

        let _ = win.flush();
        unsafe { libsarga::syscall::syscall1(35, 100_000_000u64); }
    }
}

sarga_main!(user_main);
