#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{sarga_main, gui::Window, theme::Theme};

fn user_main() -> i32 {
    let mut win = Window::create("Archive Manager", 500, 400).unwrap();
    let theme = Theme::dark();
    let files = [
        "backup.tar",
        "photos.zip",
        "docs.7z",
        "source_code.tar.gz",
    ];

    loop {
        win.clear(theme.bg_primary);
        win.draw_rect(0, 0, 500, 40, theme.bg_elevated);
        win.draw_string(10, 12, "Open | Extract | Compress", theme.text, 0);

        for (i, file) in files.iter().enumerate() {
            let y = 60 + i as u32 * 35;
            win.draw_string(20, y, "[A]", theme.warning, 0);
            win.draw_string(60, y, file, theme.text, 0);
            win.draw_string(400, y, "1.2 MB", theme.text_secondary, 0);
        }

        let _ = win.flush();
        unsafe { libsarga::syscall::syscall1(35, 100_000_000u64); }
    }
}

sarga_main!(user_main);
