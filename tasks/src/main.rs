#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{gui::Window, sarga_main, theme::Theme, version::SKYOS_VERSION};

fn user_main() -> i32 {
    let mut win = Window::create("Tasks", 400, 500).expect("Window::create failed");
    let theme = Theme::dark();
    let tasks = [
        ("Improve UI/UX", true),
        ("Add 10 apps", true),
        ("Performance tuning", false),
        ("Bug fixes", false),
        ("Write documentation", true),
        (&alloc::format!("Release v{}", SKYOS_VERSION), false),
    ];

    loop {
        win.clear(theme.bg_primary);
        win.draw_gradient_rect(0, 0, 400, 60, theme.accent, theme.accent_dark, false);
        win.draw_string(20, 22, "My Tasks", 0xFFFFFFFF, 0);

        for (i, (name, done)) in tasks.iter().enumerate() {
            let y = 80 + i as u32 * 50;
            win.draw_rounded_rect(10, y, 380, 40, 8, theme.bg_surface);

            if *done {
                win.draw_rounded_rect(20, y + 10, 20, 20, 4, theme.success);
                win.draw_string(25, y + 12, "v", 0xFFFFFFFF, 0);
                win.draw_string(55, y + 12, name, theme.text_disabled, 0);
                win.draw_line_h(55, y + 20, 100, theme.text_disabled);
            } else {
                win.draw_rounded_rect_outline(20, y + 10, 20, 20, 4, theme.border);
                win.draw_string(55, y + 12, name, theme.text, 0);
            }
        }

        let _ = win.flush();
        unsafe {
            libsarga::syscall::syscall2(35, 0, 100_000_000u64);
        }
    }
}

sarga_main!(user_main);
