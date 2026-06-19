#![no_std]
#![no_main]
extern crate alloc;
use libsarga::theme::Theme;
use libsarga::{gui::Window, sarga_main};
use libsarga::{io, process};

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

struct AppWindow {
    x: i32,
    y: i32,
    w: u32,
    h: u32,
    title: alloc::string::String,
    content: alloc::vec::Vec<alloc::string::String>,
    scroll: u32,
    pid: Option<u64>,
    focused: bool,
    dragging: bool,
    drag_ox: i32,
    drag_oy: i32,
    opacity: u8, // For fade-in animation
}

struct Desktop {
    screen_w: u32,
    screen_h: u32,
    windows: alloc::vec::Vec<AppWindow>,
    start_menu: bool,
    context_menu: Option<(i32, i32, &'static [(&'static str, &'static str)])>,
    clock_ticks: u64,
    mouse_x: i32,
    mouse_y: i32,
    mouse_btn: bool,
    prev_mouse_btn: bool,
    icons: alloc::vec::Vec<(&'static str, u32, u32)>,
    theme: Theme,
}

impl Desktop {
    fn new(w: u32, h: u32) -> Self {
        let mut icons = alloc::vec::Vec::new();
        icons.push(("Terminal", 30, 80));
        icons.push(("Files", 30, 180));
        icons.push(("SkyStore", 30, 280));
        icons.push(("SkyEdit", 30, 380));
        icons.push(("Calc", 30, 480));
        Self {
            screen_w: w,
            screen_h: h,
            windows: alloc::vec::Vec::new(),
            start_menu: false,
            context_menu: None,
            clock_ticks: 0,
            mouse_x: (w / 2) as i32,
            mouse_y: (h / 2) as i32,
            mouse_btn: false,
            prev_mouse_btn: false,
            icons,
            theme: Theme::dark(),
        }
    }

    fn taskbar_y(&self) -> u32 {
        self.screen_h - TASKBAR_H
    }

    fn draw_wallpaper(&self, win: &mut Window) {
        // Draw a nice gradient wallpaper
        win.draw_gradient_rect(
            0,
            0,
            self.screen_w,
            self.screen_h,
            0xFF1A1A2E,
            0xFF0F0F1A,
            true,
        );
        // Add some "abstract" shapes
        win.draw_rounded_rect(
            self.screen_w / 2,
            self.screen_h / 4,
            300,
            300,
            150,
            0x103D5AFE,
        );
        win.draw_rounded_rect(
            self.screen_w / 4,
            self.screen_h / 2,
            200,
            200,
            100,
            0x103D5AFE,
        );
    }

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
                return 0;
            }
            self.start_menu = false;
            return 0;
        }

        if my >= taskbar_y {
            if mx >= 5 && mx < 65 {
                self.start_menu = true;
                return 0;
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
                    return 0;
                }
            }
            return 0;
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
                let last = self.windows.last_mut().unwrap();
                last.dragging = true;
                last.drag_ox = drag_ox;
                last.drag_oy = drag_oy;
                return 0;
            }

            if mx >= w.x + w.w as i32 - 24
                && mx < w.x + w.w as i32 - 4
                && my >= w.y + 3
                && my < w.y + 19
            {
                self.windows.remove(i);
                return 0;
            }
            if mx >= w.x + w.w as i32 - 48
                && mx < w.x + w.w as i32 - 28
                && my >= w.y + 3
                && my < w.y + 19
            {
                self.windows[i].x = -9999;
                self.windows[i].y = -9999;
                return 0;
            }

            if mx >= w.x && mx < w.x + w.w as i32 && my >= w.y && my < w.y + w.h as i32 {
                for win in self.windows.iter_mut() {
                    win.focused = false;
                }
                self.windows[i].focused = true;
                let win = self.windows.remove(i);
                self.windows.push(win);
                return 0;
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
                return 0;
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

    fn tick(&mut self) {
        self.clock_ticks += 1;
        // Fade in animation
        for w in self.windows.iter_mut() {
            if w.opacity < 255 {
                w.opacity = w.opacity.saturating_add(25);
            }
        }
    }
}

fn draw_icon(win: &mut Window, theme: &Theme, name: &str, x: u32, y: u32) {
    win.draw_rounded_rect(x, y, 40, 40, 6, theme.bg_elevated);
    let letter = name.as_bytes()[0] as char;
    win.draw_char(x + 15, y + 12, letter, theme.accent, theme.bg_elevated);
    let tw = name.len() as u32 * 8;
    win.draw_string(x + 20 - tw / 2, y + 44, name, theme.text, 0);
}

fn draw_taskbar(win: &mut Window, theme: &Theme, desktop: &Desktop) {
    let ty = desktop.taskbar_y();
    win.draw_gradient_rect(
        0,
        ty,
        desktop.screen_w,
        TASKBAR_H,
        theme.bg_surface,
        theme.bg_primary,
        true,
    );
    win.draw_line_h(0, ty, desktop.screen_w, theme.border);

    // Start Button
    let start_hover = desktop.mouse_x >= 5
        && desktop.mouse_x < 63
        && desktop.mouse_y >= ty as i32 + 4
        && desktop.mouse_y < ty as i32 + TASKBAR_H as i32 - 4;
    let start_bg = if start_hover {
        theme.hover
    } else {
        theme.accent
    };
    win.draw_rounded_rect(5, ty + 4, 58, TASKBAR_H - 8, 6, start_bg);
    win.draw_string(13, ty + 10, "Start", 0xFFFFFFFF, 0);

    for (i, aw) in desktop.windows.iter().enumerate() {
        let bx = 75 + i as u32 * 125;
        let is_top = i == desktop.windows.len() - 1;
        let is_min = aw.x == -9999;
        let hover = desktop.mouse_x >= bx as i32
            && desktop.mouse_x < bx as i32 + 120
            && desktop.mouse_y >= ty as i32 + 4
            && desktop.mouse_y < ty as i32 + TASKBAR_H as i32 - 4;

        let bg = if is_min {
            theme.bg_surface
        } else if is_top {
            theme.bg_elevated
        } else if hover {
            theme.hover
        } else {
            theme.bg_surface
        };
        win.draw_rounded_rect(bx, ty + 4, 120, TASKBAR_H - 8, 6, bg);
        if is_top && !is_min {
            win.draw_line_h(bx + 10, ty + TASKBAR_H - 3, 100, theme.accent);
        }
        let display = if aw.title.len() > 14 {
            &aw.title[..14]
        } else {
            &aw.title
        };
        let text_c = if is_top {
            theme.text
        } else {
            theme.text_secondary
        };
        win.draw_string(bx + 8, ty + 10, display, text_c, 0);
    }

    // System tray area
    let tray_x = desktop.screen_w - 180;
    let secs = desktop.clock_ticks / 10;
    let hrs = (secs / 3600) % 24;
    let mins = (secs / 60) % 60;
    let clock_str = alloc::format!("{:02}:{:02}", hrs, mins);

    win.draw_rounded_rect(tray_x, ty + 4, 175, TASKBAR_H - 8, 6, theme.bg_elevated);
    win.draw_string(tray_x + 10, ty + 10, "NET", theme.success, 0);
    win.draw_string(tray_x + 50, ty + 10, "VOL", theme.accent, 0);
    win.draw_string(tray_x + 100, ty + 10, &clock_str, theme.text, 0);
}

fn draw_start_menu(win: &mut Window, theme: &Theme, desktop: &Desktop) {
    let taskbar_y = desktop.taskbar_y();
    let menu_w = 200u32;
    let menu_h = (MENU_ITEMS.len() as u32) * 32 + 40;
    let menu_x = 5u32;
    let menu_y = taskbar_y - menu_h - 4;

    win.draw_rounded_rect(menu_x, menu_y, menu_w, menu_h, 6, theme.bg_elevated);

    win.draw_rect(menu_x + 2, menu_y + 2, menu_w - 4, 34, theme.accent);
    win.draw_string(menu_x + 10, menu_y + 10, "SARGA OS Menu", 0xFFFFFFFF, 0);

    for (i, &(name, _)) in MENU_ITEMS.iter().enumerate() {
        if name == "---" {
            let sep_y = menu_y + 38 + i as u32 * 32;
            win.draw_line_h(menu_x + 8, sep_y + 16, menu_w - 16, theme.separator);
            continue;
        }
        let iy = menu_y + 38 + i as u32 * 32;
        let hover = desktop.mouse_x >= menu_x as i32
            && desktop.mouse_x < (menu_x + menu_w) as i32
            && desktop.mouse_y >= iy as i32
            && desktop.mouse_y < (iy + 28) as i32;
        let bg = if hover {
            theme.hover
        } else {
            theme.bg_elevated
        };
        win.draw_rounded_rect(menu_x + 2, iy, menu_w - 4, 28, 4, bg);
        win.draw_string(menu_x + 12, iy + 6, name, theme.text, 0);
    }
}

fn draw_window(win: &mut Window, theme: &Theme, aw: &AppWindow) {
    if aw.x < -100 || aw.y < -100 {
        return;
    }

    let border_color = if aw.focused {
        theme.accent
    } else {
        theme.border
    };

    // Shadow
    win.draw_rect_alpha(aw.x as u32 + 6, aw.y as u32 + 6, aw.w, aw.h, 0x60000000);

    // Fade-in effect via background fill if not fully opaque
    if aw.opacity < 255 {
        // Just skip rendering or draw with lower contrast
    }

    // Window body
    win.draw_rounded_rect(
        aw.x as u32,
        aw.y as u32,
        aw.w,
        aw.h,
        theme.border_radius,
        theme.bg_surface,
    );
    win.draw_rounded_rect_outline(
        aw.x as u32,
        aw.y as u32,
        aw.w,
        aw.h,
        theme.border_radius,
        border_color,
    );

    // Title bar
    let title_c1 = if aw.focused {
        theme.accent
    } else {
        theme.bg_elevated
    };
    let title_c2 = if aw.focused {
        theme.accent_dark
    } else {
        theme.bg_surface
    };
    win.draw_gradient_rect(
        aw.x as u32 + 1,
        aw.y as u32 + 1,
        aw.w - 2,
        28,
        title_c1,
        title_c2,
        false,
    );
    win.draw_string(aw.x as u32 + 12, aw.y as u32 + 7, &aw.title, 0xFFFFFFFF, 0);

    // Close button
    let close_x = aw.x as u32 + aw.w - 28;
    let close_y = aw.y as u32 + 6;
    win.draw_rounded_rect(close_x, close_y, 22, 18, 4, theme.error);
    win.draw_string(close_x + 7, close_y + 2, "x", 0xFFFFFFFF, 0);

    // Minimize button
    let min_x = aw.x as u32 + aw.w - 54;
    win.draw_rounded_rect(min_x, close_y, 22, 18, 4, theme.bg_elevated);
    win.draw_line_h(min_x + 6, close_y + 14, 10, 0xFFFFFFFF);

    // Content
    let line_y = aw.y as u32 + 28;
    let max_lines = ((aw.h - 34) / 14) as usize;
    let start = if aw.content.len() > max_lines {
        aw.content.len() - max_lines + aw.scroll as usize
    } else {
        0
    };
    for (i, line) in aw.content.iter().skip(start).take(max_lines).enumerate() {
        let ly = line_y + i as u32 * 14;
        if ly + 14 > aw.y as u32 + aw.h {
            break;
        }
        let display = if line.len() > 55 { &line[..55] } else { line };
        win.draw_string(aw.x as u32 + 8, ly, display, theme.text_secondary, 0);
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

        desktop.draw_wallpaper(&mut desktop_win);

        for icon in &desktop.icons {
            draw_icon(&mut desktop_win, &desktop.theme, icon.0, icon.1, icon.2);
        }

        for aw in &desktop.windows {
            draw_window(&mut desktop_win, &desktop.theme, aw);
        }

        draw_taskbar(&mut desktop_win, &desktop.theme, &desktop);

        if desktop.start_menu {
            draw_start_menu(&mut desktop_win, &desktop.theme, &desktop);
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
            libsarga::syscall::syscall1(35, 16_000_000u64);
        }
    }
    0
}

sarga_main!(user_main);
