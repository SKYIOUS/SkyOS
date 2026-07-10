#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{sarga_main, gui::Window, theme::Theme};
use alloc::string::String;

fn user_main() -> i32 {
    let mut win = Window::create("Search", 400, 300).unwrap();
    let theme = Theme::dark();
    let mut query = String::from("sky");
    let results = ["skyfiles", "skyedit", "skysettings", "skystore"];

    loop {
        win.clear(theme.bg_surface);
        win.draw_rounded_rect(10, 10, 380, 40, 8, theme.bg_primary);
        win.draw_rounded_rect_outline(10, 10, 380, 40, 8, theme.accent);
        win.draw_string(20, 22, &query, theme.text, 0);
        win.fill_rect(20 + query.len() as u32 * 8, 20, 2, 20, theme.accent);

        win.draw_string(15, 70, "Results:", theme.text_secondary, 0);
        for (i, res) in results.iter().enumerate() {
            let y = 100 + i as u32 * 30;
            win.draw_string(20, y, ">", theme.accent, 0);
            win.draw_string(40, y, res, theme.text, 0);
        }

        let _ = win.flush();
        unsafe { libsarga::syscall::syscall1(35, 100_000_000u64); }
    }
}

sarga_main!(user_main);
