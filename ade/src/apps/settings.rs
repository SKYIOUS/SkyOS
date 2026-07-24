//! Settings application — full settings panel with sidebar and pages.

use alloc::string::String;
use crate::render::compositor::Canvas;
use crate::render::snapshot::RenderSnapshot;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SettingsPage {
    Appearance,
    Desktop,
    Keyboard,
    Mouse,
    Display,
    About,
    System,
    Power,
    Notification,
    Theme,
}

const PAGES: &[(&str, SettingsPage)] = &[
    ("Appearance", SettingsPage::Appearance),
    ("Desktop", SettingsPage::Desktop),
    ("Keyboard", SettingsPage::Keyboard),
    ("Mouse", SettingsPage::Mouse),
    ("Display", SettingsPage::Display),
    ("About", SettingsPage::About),
    ("System", SettingsPage::System),
    ("Power", SettingsPage::Power),
    ("Notification", SettingsPage::Notification),
    ("Theme", SettingsPage::Theme),
];

pub(crate) struct SettingsAppState {
    pub open: bool,
    pub current_page: SettingsPage,
    pub app: bool,
    pub hover_idx: i32,
    pub scroll: u32,
}

impl SettingsAppState {
    pub fn new() -> Self {
        SettingsAppState {
            open: false,
            current_page: SettingsPage::Appearance,
            app: true,
            hover_idx: -1,
            scroll: 0,
        }
    }

    pub fn draw(&self, canvas: &mut Canvas, snap: &RenderSnapshot) {
        if !self.open {
            return;
        }
        let pw = 560u32;
        let ph = 400u32;
        let px = (snap.screen_w - pw) / 2;
        let py = (snap.screen_h - ph) / 3;
        crate::core::dialog::draw_backdrop(canvas, snap.screen_w, snap.screen_h);
        crate::core::dialog::draw_panel(canvas, px, py, pw, ph, "Settings");

        // Sidebar
        let sidebar_x = px + 4;
        let sidebar_y = py + 32;
        let sidebar_w = 140u32;
        let sidebar_h = ph - 40;
        canvas.draw_rounded_rect(sidebar_x, sidebar_y, sidebar_w, sidebar_h, 6, 0xFF252535);

        for (i, &(name, page)) in PAGES.iter().enumerate() {
            let iy = sidebar_y + 4 + i as u32 * 28;
            if iy + 24 > sidebar_y + sidebar_h {
                break;
            }
            let is_cur = page == self.current_page;
            let bg = if is_cur { snap.theme.accent } else { 0xFF252535 };
            let fg = if is_cur { 0xFFFFFFFF } else { 0xFFB0B0B0 };
            canvas.draw_rounded_rect(sidebar_x + 4, iy, sidebar_w - 8, 24, 4, bg);
            canvas.draw_string(sidebar_x + 10, iy + 5, name, fg, 0);
        }

        // Content area
        let cx = sidebar_x + sidebar_w + 6;
        let cy = sidebar_y;
        let cw = pw - (cx - px) - 8;
        canvas.draw_rounded_rect(cx, cy, cw, sidebar_h, 6, 0xFF1E1E2E);

        match self.current_page {
            SettingsPage::Appearance => self.draw_page_appearance(canvas, cx, cy, cw),
            SettingsPage::About => self.draw_page_about(canvas, cx, cy, cw),
            _ => {
                let page_name = PAGES
                    .iter()
                    .find(|(_, p)| *p == self.current_page)
                    .map(|(n, _)| *n)
                    .unwrap_or("");
                canvas.draw_string(cx + 10, cy + 10, page_name, 0xFFFFFFFF, 0);
                canvas.draw_string(cx + 10, cy + 30, "Coming soon", 0xFF888888, 0);
            }
        }
    }

    fn draw_page_appearance(&self, canvas: &mut Canvas, cx: u32, cy: u32, cw: u32) {
        canvas.draw_string(cx + 10, cy + 10, "Appearance", 0xFFFFFFFF, 0);

        // Dark Theme toggle
        let toggle_y = cy + 36;
        let hover = self.hover_idx == 0;
        let bg = if hover { 0xFF3A3A5C } else { 0xFF2D2D40 };
        canvas.draw_rounded_rect(cx + 8, toggle_y, cw - 16, 28, 4, bg);
        canvas.draw_string(cx + 16, toggle_y + 6, "Dark Theme", 0xFFD0D0D0, 0);
        let toggle_fg = if self.app { 0xFF4CAF50 } else { 0xFF555555 };
        canvas.draw_char(cx + cw - 36, toggle_y + 6, if self.app { 'Y' } else { 'N' }, toggle_fg, 0);

        // Window Opacity (placeholder)
        let opacity_y = toggle_y + 32;
        canvas.draw_rounded_rect(cx + 8, opacity_y, cw - 16, 28, 4, 0xFF2D2D40);
        canvas.draw_string(cx + 16, opacity_y + 6, "Window Opacity", 0xFFD0D0D0, 0);
        canvas.draw_string(cx + cw - 90, opacity_y + 6, "[slider]", 0xFF555555, 0);
    }

    fn draw_page_about(&self, canvas: &mut Canvas, cx: u32, cy: u32, cw: u32) {
        canvas.draw_string(cx + 10, cy + 10, "About", 0xFFFFFFFF, 0);

        // Logo area
        let logo_x = cx + cw / 2 - 40;
        let logo_y = cy + 40;
        canvas.draw_rounded_rect(logo_x, logo_y, 80, 40, 8, 0xFF3D5AFE);
        canvas.draw_string(logo_x + 16, logo_y + 14, "SARGA", 0xFFFFFFFF, 0);

        let info_y = logo_y + 56;
        let lines = &[
            alloc::format!("SARGA OS v{}", libsarga::version::SKYOS_VERSION),
            String::from("Kernel: SARGA"),
            String::from("Arch: x86_64"),
            String::from("Desktop: ADE"),
            String::new(),
            String::from("A modern OS written in Rust."),
        ];
        for (i, line) in lines.iter().enumerate() {
            canvas.draw_string(cx + 10, info_y + i as u32 * 16, line, 0xFFD0D0D0, 0);
        }
    }

    pub fn toggle_theme(&mut self, desktop: &mut crate::core::desktop::Desktop) {
        self.app = !self.app;
        if self.app {
            desktop.theme_svc.set(libsarga::theme::Theme::dark());
        } else {
            desktop.theme_svc.set(libsarga::theme::Theme::light());
        }
    }

    pub fn hit_test(&self, mx: i32, my: i32, snap: &RenderSnapshot) -> Option<usize> {
        if !self.open {
            return None;
        }
        let pw = 560u32;
        let ph = 400u32;
        let px = (snap.screen_w - pw) / 2;
        let py = (snap.screen_h - ph) / 3;

        // Sidebar click → page index
        let sidebar_x = px + 4;
        let sidebar_y = py + 32;
        for (i, _) in PAGES.iter().enumerate() {
            let iy = sidebar_y + 4 + i as u32 * 28;
            if mx >= sidebar_x as i32 + 4
                && mx <= (sidebar_x + 136) as i32
                && my >= iy as i32
                && my <= (iy + 24) as i32
            {
                return Some(i); // 0..9 = switch to page
            }
        }

        // Content area - Appearance page theme toggle
        if self.current_page == SettingsPage::Appearance {
            let toggle_y = py + 32 + 36;
            if mx >= (px + 8) as i32
                && mx <= (px + 544) as i32
                && my >= toggle_y as i32
                && my <= (toggle_y + 28) as i32
            {
                return Some(10); // theme toggle
            }
        }

        None
    }
}
