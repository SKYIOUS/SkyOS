#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{sarga_main, gui::Window, theme::Theme};
use alloc::format;

fn user_main() -> i32 {
    let mut win = Window::create("Calendar", 400, 350).unwrap();
    let theme = Theme::dark();
    let days = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

    loop {
        win.clear(theme.bg_surface);
        win.draw_gradient_rect(0, 0, 400, 50, theme.accent, theme.accent_dark, false);
        win.draw_string_centered(18, "October 2026", 0xFFFFFFFF, 0);

        for (i, day) in days.iter().enumerate() {
            win.draw_string(20 + i as u32 * 54, 70, day, theme.text_secondary, 0);
        }

        for i in 0..31 {
            let x = 20 + ((i + 4) % 7) as u32 * 54;
            let y = 100 + ((i + 4) / 7) as u32 * 40;
            let is_today = i == 23; // Mock today
            if is_today {
                win.draw_rounded_rect(x - 5, y - 5, 30, 30, 15, theme.accent);
            }
            win.draw_string(x, y, &format!("{}", i + 1), if is_today { 0xFFFFFFFF } else { theme.text }, 0);
        }

        let _ = win.flush();
        unsafe { libsarga::syscall::syscall1(35, 100_000_000u64); }
    }
}

sarga_main!(user_main);
