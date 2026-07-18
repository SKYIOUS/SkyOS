use libsarga::gui::Window;
use libsarga::process;
use libsarga::theme::Theme;
use crate::{AppWindow, MENU_ITEMS, TASKBAR_H};

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
        // Fade in animation
        for w in self.windows.iter_mut() {
            if w.opacity < 255 {
                w.opacity = w.opacity.saturating_add(25);
            }
        }
    }
}