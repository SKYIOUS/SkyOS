use libsarga::process;
use libsarga::theme::Theme;
use crate::constants::{MENU_ITEMS, TASKBAR_H};
use crate::window::WindowState;

const TITLE_H: i32 = 22;
const BTN_TOP: i32 = 3;
const BTN_BOT: i32 = 19;
const CLOSE_R: i32 = 4;
const CLOSE_L: i32 = 24;
const MIN_R: i32 = 28;
const MIN_L: i32 = 48;
use crate::window_manager::WindowManager;


pub struct Desktop {
    pub(crate) screen_w: u32,
    pub(crate) screen_h: u32,
    pub(crate) wm: WindowManager,
    pub(crate) start_menu: bool,
    pub(crate) context_menu: Option<(i32, i32, &'static [(&'static str, &'static str)])>,
    pub(crate) clock_ticks: u64,
    pub(crate) mouse_x: i32,
    pub(crate) mouse_y: i32,
    mouse_btn: bool,
    prev_mouse_btn: bool,
    pub(crate) icons: alloc::vec::Vec<(&'static str, u32, u32)>,
    pub(crate) theme: Theme,
    pub(crate) dirty: bool,
    pub(crate) clock_cache: crate::render::clock::ClockCache,
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
            wm: WindowManager::new(),
            start_menu: false,
            context_menu: None,
            clock_ticks: 0,
            mouse_x: (w / 2) as i32,
            mouse_y: (h / 2) as i32,
            mouse_btn: false,
            prev_mouse_btn: false,
            icons,
            theme: Theme::dark(),
            dirty: true,
            clock_cache: crate::render::clock::ClockCache::new(),
        }
    }

    pub fn taskbar_y(&self) -> u32 {
        self.screen_h - TASKBAR_H
    }

    pub fn advance_clock(&mut self) {
        self.clock_ticks += 1;
    }

    pub fn tick(&mut self) {
        self.advance_clock();
        for w in self.wm.windows_mut().iter_mut() {
            if w.opacity < 255 {
                w.opacity = w.opacity.saturating_add(25);
            }
        }
    }

    pub(crate) fn spawn_app(&mut self, path: &str, title: &str) {
        crate::launcher::spawn_app(self, path, title);
    }

    pub fn update_mouse(&mut self, mx: i32, my: i32, btn: bool) -> (bool, bool) {
        self.mouse_x = mx;
        self.mouse_y = my;
        let just_pressed = btn && !self.mouse_btn;
        let just_released = !btn && self.mouse_btn;
        self.prev_mouse_btn = self.mouse_btn;
        self.mouse_btn = btn;
        (just_pressed, just_released)
    }

    pub(crate) fn handle_click(&mut self, mx: i32, my: i32) {
        self.dirty = true;
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
                            if let Some(w) = self.wm.last_mut() {
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
            for (i, _) in self.wm.windows().iter().enumerate() {
                let bx = btn_x + i as i32 * 120;
                if mx >= bx && mx < bx + 115 {
                    if self.wm.windows()[i].state == WindowState::Minimized {
                        self.wm.restore(i);
                    }
                    self.wm.bring_to_front(i);
                    return;
                }
            }
            return;
        }

        for i in (0..self.wm.len()).rev() {
            let wx = self.wm.windows()[i].x;
            let wy = self.wm.windows()[i].y;
            let ww = self.wm.windows()[i].w;
            let wh = self.wm.windows()[i].h;
            if mx >= wx && mx < wx + ww as i32 && my >= wy && my < wy + TITLE_H {
                self.wm.bring_to_front(i);
                self.wm.begin_drag(i, mx, my);
                return;
            }

            if mx >= wx + ww as i32 - CLOSE_L
                && mx < wx + ww as i32 - CLOSE_R
                && my >= wy + BTN_TOP
                && my < wy + BTN_BOT
            {
                self.wm.close(i);
                return;
            }
            if mx >= wx + ww as i32 - MIN_L
                && mx < wx + ww as i32 - MIN_R
                && my >= wy + BTN_TOP
                && my < wy + BTN_BOT
            {
                self.wm.minimize(i);
                return;
            }

            if mx >= wx && mx < wx + ww as i32 && my >= wy && my < wy + wh as i32 {
                self.wm.bring_to_front(i);
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

    pub(crate) fn handle_drag(&mut self, mx: i32, my: i32) {
        self.wm.update_drag(mx, my);
        self.dirty = true;
    }

    pub(crate) fn release_drag(&mut self) {
        self.wm.end_drag();
    }
}