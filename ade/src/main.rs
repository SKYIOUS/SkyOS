#![no_std]
#![no_main]
extern crate alloc;
use libsarga::theme::Theme;
use libsarga::{gui::Window, sarga_main};
use libsarga::{io, process};
use desktop::Desktop;
use crate::wallpaper::draw;
use crate::window::AppWindow;

mod desktop;
mod window;
mod taskbar;
mod start_menu;
mod wallpaper;
mod icons;

const TASKBAR_H: u32 = 36;
const MENU_ITEMS: &[(&str, &str)] = &[
    ("Terminal", "/bin/sash"),
    ("File Manager", "/bin/skyfiles"),
    ("SkyStore", "/bin/skystore"),
    ("System Monitor", "/bin/sysmon"),
    ("Calendar", "/bin/calendar"),
    ("Notes", "/bin/notes"),
    ("Paint", "/bin/paint"),
    ("Clock", "/bin/clock"),
    ("Tasks", "/bin/tasks"),
    ("Search", "/bin/search"),
    ("System Info", "/bin/sysinfo"),
    ("Settings", "/bin/skysettings"),
    ("SkyEdit", "/bin/skyedit"),
    ("---", ""),
    ("About SARGA OS", ""),
    ("Shutdown", ""),
];





impl Desktop {

    fn spawn_app(&mut self, path: &str, title: &str) {
        let w = 520u32;
        let h = 360u32;
        let x = 80 + self.windows.len() as i32 * 30;
        let y = 40 + self.windows.len() as i32 * 20;
        let mut app_win = AppWindow {
            x,
            y,
            w,
            h,
            title: alloc::string::String::from(title),
            content: alloc::vec::Vec::new(),
            scroll: 0,
            pid: None,
            focused: true,
            dragging: false,
            drag_ox: 0,
            drag_oy: 0,
            opacity: 0,
        };
        app_win.content.push(alloc::format!("> {}", path));
        app_win.content.push(alloc::string::String::new());

        if !path.is_empty() {
            match process::fork() {
                Ok(0) => {
                    let _ = process::execve(path, &[path], &[]);
                    process::exit(1);
                }
                Ok(pid) => {
                    app_win.pid = Some(pid);
                    app_win
                        .content
                        .push(alloc::format!("[launched {} pid={}]", title, pid));
                }
                Err(e) => {
                    app_win.content.push(alloc::format!("[fork failed: {}]", e));
                }
            }
        }
        self.windows.push(app_win);
    }

    #[allow(dead_code)]
    fn handle_click(&mut self, mx: i32, my: i32) {
        let taskbar_y = self.taskbar_y() as i32;

        if self.start_menu {
            let menu_x = 5i32;
            let menu_y = taskbar_y - 5 - MENU_ITEMS.len() as i32 * 32 - 40;
            let menu_w = 200i32;
            let menu_h = MENU_ITEMS.len() as i32 * 32 + 40;
            if mx >= menu_x && mx < menu_x + menu_w && my >= menu_y && my < menu_y + menu_h {
                let header_h = 36;
                let idx = (my - menu_y - header_h) / 32;
                if idx >= 0 && (idx as usize) < MENU_ITEMS.len() {
                    let (name, path) = MENU_ITEMS[idx as usize];
                    self.start_menu = false;
                    match name {
                        "About SARGA OS" => {
                            self.spawn_app("", "About SARGA OS");
                            if let Some(w) = self.windows.last_mut() {
                                w.content.clear();
                                w.content
                                    .push(alloc::string::String::from("  SARGA OS v0.4.0"));
                                w.content.push(alloc::string::String::new());
                                w.content
                                    .push(alloc::string::String::from("  Kernel: SARGA"));
                                w.content
                                    .push(alloc::string::String::from("  Arch: x86_64"));
                                w.content
                                    .push(alloc::string::String::from("  Shell: SargaSH"));
                                w.content
                                    .push(alloc::string::String::from("  Desktop: ADE"));
                                w.content
                                    .push(alloc::string::String::from("  Widgets: libsarga"));
                                w.content.push(alloc::string::String::new());
                                w.content.push(alloc::string::String::from(
                                    "  A modern OS written in Rust.",
                                ));
                            }
                        }
                        "Shutdown" => {
                            process::exit(0);
                        }
                        "---" => {}
                        _ => {
                            self.spawn_app(path, name);
                        }
                    }
                }
                return;
            }
            self.start_menu = false;
            return;
        }

        if my >= taskbar_y {
            if mx >= 5 && mx < 65 {
                self.start_menu = true;
                return;
            }
            let btn_x = 75i32;
            for (i, _) in self.windows.iter().enumerate() {
                let bx = btn_x + i as i32 * 120;
                if mx >= bx && mx < bx + 115 {
                    let was_minimized = self.windows[i].x == -9999;
                    if was_minimized {
                        self.windows[i].x = 80 + i as i32 * 30;
                        self.windows[i].y = 40 + i as i32 * 20;
                    }
                    for w in self.windows.iter_mut() {
                        w.focused = false;
                    }
                    self.windows[i].focused = true;
                    let w = self.windows.remove(i);
                    self.windows.push(w);
                    return;
                }
            }
            return;
        }

        for i in (0..self.windows.len()).rev() {
            let w = &self.windows[i];
            if mx >= w.x && mx < w.x + w.w as i32 && my >= w.y && my < w.y + 22 {
                for win in self.windows.iter_mut() {
                    win.focused = false;
                }
                self.windows[i].focused = true;
                let win = self.windows.remove(i);
                let drag_ox = mx - win.x;
                let drag_oy = my - win.y;
                self.windows.push(win);
                if let Some(last) = self.windows.last_mut() {
                    last.dragging = true;
                    last.drag_ox = drag_ox;
                    last.drag_oy = drag_oy;
                }
                return;
            }

            if mx >= w.x + w.w as i32 - 24
                && mx < w.x + w.w as i32 - 4
                && my >= w.y + 3
                && my < w.y + 19
            {
                self.windows.remove(i);
                return;
            }
            if mx >= w.x + w.w as i32 - 48
                && mx < w.x + w.w as i32 - 28
                && my >= w.y + 3
                && my < w.y + 19
            {
                self.windows[i].x = -9999;
                self.windows[i].y = -9999;
                return;
            }

            if mx >= w.x && mx < w.x + w.w as i32 && my >= w.y && my < w.y + w.h as i32 {
                for win in self.windows.iter_mut() {
                    win.focused = false;
                }
                self.windows[i].focused = true;
                let win = self.windows.remove(i);
                self.windows.push(win);
                return;
            }
        }

        for icon in &self.icons {
            let (name, ix, iy) = icon;
            if mx >= *ix as i32 && mx < *ix as i32 + 40 && my >= *iy as i32 && my < *iy as i32 + 50
            {
                match *name {
                    "Terminal" => self.spawn_app("/bin/sash", "Terminal"),
                    "Files" => self.spawn_app("/bin/skyfiles", "Files"),
                    "System" => self.spawn_app("/bin/uname", "System Info"),
                    "SkyEdit" => self.spawn_app("/bin/skyedit", "SkyEdit"),
                    "Calc" => self.spawn_app("/bin/calculator", "Calculator"),
                    _ => {}
                }
                return;
            }
        }
    }

    #[allow(dead_code)]
    fn handle_drag(&mut self, mx: i32, my: i32) {
        if let Some(last) = self.windows.last_mut() {
            if last.dragging {
                last.x = mx - last.drag_ox;
                last.y = my - last.drag_oy;
            }
        }
    }

    #[allow(dead_code)]
    fn release_drag(&mut self) {
        if let Some(last) = self.windows.last_mut() {
            last.dragging = false;
        }
    }
}



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
