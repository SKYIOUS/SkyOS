#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{sarga_main, gui::Window, theme::Theme, process, io};
use alloc::vec::Vec;
use alloc::string::String;

struct App {
    name: &'static str,
    desc: &'static str,
    installed: bool,
}

fn user_main() -> i32 {
    let mut win = Window::create("SkyStore", 600, 450).unwrap();
    let theme = Theme::dark();
    let apps = [
        App { name: "System Monitor", desc: "View CPU and Memory usage", installed: false },
        App { name: "Calendar", desc: "Manage your schedule", installed: false },
        App { name: "Paint", desc: "Simple drawing tool", installed: true },
        App { name: "Notes", desc: "Quickly take notes", installed: false },
        App { name: "Clock", desc: "World time and alarms", installed: true },
        App { name: "Weather", desc: "Check current weather", installed: false },
        App { name: "Maps", desc: "Explore the world", installed: false },
        App { name: "Player", desc: "Music and Video player", installed: false },
    ];

    loop {
        let mouse = win.get_mouse();
        while let Some(_) = win.get_key() {}

        win.draw_gradient_rect(0, 0, 600, 450, theme.bg_primary, theme.bg_surface, true);
        win.draw_string(20, 20, "SkyStore - Discover Apps", 0xFFFFFFFF, 0);

        for (i, app) in apps.iter().enumerate() {
            let ay = 60 + i as u32 * 45;
            let hover = mouse.y >= ay as i32 && mouse.y < (ay + 40) as i32;
            let bg = if hover { theme.bg_elevated } else { theme.bg_surface };

            win.draw_rounded_rect(10, ay, 580, 40, 8, bg);
            win.draw_string(20, ay + 12, app.name, 0xFFFFFFFF, 0);
            win.draw_string(180, ay + 12, app.desc, theme.text_secondary, 0);

            if app.installed {
                win.draw_string(500, ay + 12, "Installed", theme.success, 0);
            } else {
                win.draw_rounded_rect(490, ay + 8, 80, 24, 6, theme.accent);
                win.draw_string(510, ay + 12, "Get", 0xFFFFFFFF, 0);
            }
        }

        let _ = win.flush();
        unsafe { libsarga::syscall::syscall1(35, 16_000_000u64); }
    }
}

sarga_main!(user_main);
