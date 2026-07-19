use libsarga::gui::Window;
use libsarga::process;
use libsarga::theme::Theme;
use crate::constants::{MENU_ITEMS, TASKBAR_H};
use crate::window::{AppWindow, WindowState};


pub struct Desktop {
    pub(crate) screen_w: u32,
    pub(crate) screen_h: u32,
    pub(crate) windows: alloc::vec::Vec<AppWindow>,
    pub(crate) start_menu: bool,
    pub(crate) context_menu: Option<(i32, i32, &'static [(&'static str, &'static str)])>,
    pub(crate) clock_ticks: u64,
    pub(crate) mouse_x: i32,
    pub(crate) mouse_y: i32,
    mouse_btn: bool,
    prev_mouse_btn: bool,
    pub(crate) icons: alloc::vec::Vec<(&'static str, u32, u32)>,
    pub(crate) theme: Theme,
}


impl Desktop {
    pub fn new(w: u32, h: u32) -> Self {
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

    pub fn taskbar_y(&self) -> u32 {
        self.screen_h - TASKBAR_H
    }

    pub fn tick(&mut self) {
        self.clock_ticks += 1;
        for w in self.windows.iter_mut() {
            if w.opacity < 255 {
                w.opacity = w.opacity.saturating_add(25);
            }
        }
    }

    pub(crate) fn spawn_app(&mut self, path: &str, title: &str) {
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
            state: WindowState::Normal,
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
                    let was_minimized =
                    self.windows[i].state == WindowState::Minimized;
                    if was_minimized {
                        self.windows[i].state = WindowState::Normal;
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
                self.windows[i].state = WindowState::Minimized;
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