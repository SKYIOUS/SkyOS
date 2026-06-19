#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{sarga_main, gui::Window, theme::Theme};

fn user_main() -> i32 {
    let mut win = Window::create("Clock", 300, 300).unwrap();
    let theme = Theme::dark();

    loop {
        win.clear(theme.bg_surface);

        let ticks = unsafe { libsarga::syscall::syscall0(1) };
        let secs = ticks / 10;
        let hrs = (secs / 3600) % 24;
        let mins = (secs / 60) % 60;
        let s = secs % 60;

        let time_str = alloc::format!("{:02}:{:02}:{:02}", hrs, mins, s);

        // Analog clock face mock
        win.draw_rounded_rect(50, 50, 200, 200, 100, theme.bg_elevated);
        win.draw_rounded_rect_outline(50, 50, 200, 200, 100, theme.accent);

        win.draw_string_centered(140, &time_str, 0xFFFFFFFF, 0);
        win.draw_string_centered(180, "London, UK", theme.text_secondary, 0);

        let _ = win.flush();
        unsafe { libsarga::syscall::syscall1(35, 100_000_000u64); }
    }
}

sarga_main!(user_main);
