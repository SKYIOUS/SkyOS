#![no_std]
#![no_main]
extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;
use libsarga::{gui::Window, sarga_main, theme::Theme};

fn user_main() -> i32 {
    let mut win = Window::create("Notes", 500, 400).expect("Window::create failed");
    let theme = Theme::dark();
    let mut notes = Vec::new();
    notes.push(String::from("Welcome to SARGA OS!"));
    notes.push(String::from("This is the new Notes app."));
    notes.push(String::from("- Built in Rust"));
    notes.push(String::from("- Fast and secure"));

    loop {
        while win.get_key().is_some() {}

        win.clear(theme.bg_primary);
        win.draw_gradient_rect(0, 0, 150, 400, theme.bg_surface, theme.bg_primary, false);

        win.draw_string(10, 20, "All Notes", theme.text, 0);
        win.draw_rounded_rect(5, 50, 140, 30, 4, theme.accent);
        win.draw_string(15, 60, "My First Note", 0xFFFFFFFF, 0);

        win.draw_rect(160, 20, 320, 360, theme.bg_surface);
        for (i, line) in notes.iter().enumerate() {
            win.draw_string(170, 40 + i as u32 * 20, line, theme.text, 0);
        }

        let _ = win.flush();
        unsafe {
            libsarga::syscall::syscall2(35, 0, 33_000_000u64);
        }
    }
}

sarga_main!(user_main);
