//! Settings application — full settings panel with sidebar and pages.

use crate::render::compositor::Canvas;
use crate::render::snapshot::RenderSnapshot;
use alloc::string::String;

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
}

impl SettingsAppState {
    pub fn new() -> Self {
        SettingsAppState {
            open: false,
            current_page: SettingsPage::Appearance,
            app: true,
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
        crate::core::dialog::draw_backdrop(canvas, snap.screen_w, snap.screen_h, snap.theme);
        crate::core::dialog::draw_panel(canvas, px, py, pw, ph, "Settings", snap.theme);

        // Sidebar
        let sidebar_x = px + 4;
        let sidebar_y = py + 32;
        let sidebar_w = 140u32;
        let sidebar_h = ph - 40;
        canvas.draw_rounded_rect(
            sidebar_x,
            sidebar_y,
            sidebar_w,
            sidebar_h,
            6,
            snap.theme.bg_elevated,
        );

        for (i, &(name, page)) in PAGES.iter().enumerate() {
            let iy = sidebar_y + 4 + i as u32 * 28;
            if iy + 24 > sidebar_y + sidebar_h {
                break;
            }
            let is_cur = page == self.current_page;
            let bg = if is_cur {
                snap.theme.accent
            } else {
                snap.theme.bg_elevated
            };
            // The selected row fills with the indigo accent -> white text
            // (theme.text flips black in the light theme).
            let fg = if is_cur {
                snap.theme.on_accent
            } else {
                snap.theme.text_secondary
            };
            canvas.draw_rounded_rect(sidebar_x + 4, iy, sidebar_w - 8, 24, 4, bg);
            canvas.draw_string(sidebar_x + 10, iy + 5, name, fg, 0);
        }

        // Content area
        let cx = sidebar_x + sidebar_w + 6;
        let cy = sidebar_y;
        let cw = pw - (cx - px) - 8;
        canvas.draw_rounded_rect(cx, cy, cw, sidebar_h, 6, snap.theme.bg_surface);

        match self.current_page {
            SettingsPage::Appearance => self.draw_page_appearance(canvas, snap, cx, cy, cw),
            SettingsPage::About => self.draw_page_about(canvas, snap, cx, cy, cw),
            _ => {
                let page_name = PAGES
                    .iter()
                    .find(|(_, p)| *p == self.current_page)
                    .map(|(n, _)| *n)
                    .unwrap_or("");
                canvas.draw_string(cx + 10, cy + 10, page_name, snap.theme.text, 0);
                canvas.draw_string(cx + 10, cy + 30, "Coming soon", snap.theme.text_disabled, 0);
            }
        }
    }

    fn draw_page_appearance(
        &self,
        canvas: &mut Canvas,
        snap: &RenderSnapshot,
        cx: u32,
        cy: u32,
        cw: u32,
    ) {
        canvas.draw_string(cx + 10, cy + 10, "Appearance", snap.theme.text, 0);

        // Dark Theme toggle — hover/pressed come from the unified hover
        // state (`Desktop::hover_target`), and the rect is the shared
        // `layout` one, so hover always lights the drawn row.
        let tr = crate::layout::settings_app_toggle_rect(crate::layout::settings_app_panel_rect(
            snap.screen_w,
            snap.screen_h,
        ));
        let hover = snap.hover == Some(crate::core::window::HoverTarget::SettingsAppRow(0));
        let bg = if hover && snap.mouse_down {
            snap.theme.pressed
        } else if hover {
            snap.theme.hover
        } else {
            snap.theme.bg_elevated
        };
        canvas.draw_rounded_rect(tr.x as u32, tr.y as u32, tr.w, tr.h, 4, bg);
        canvas.draw_string(
            tr.x as u32 + 16,
            tr.y as u32 + 6,
            "Dark Theme",
            if hover && snap.mouse_down {
                snap.theme.text
            } else if hover {
                snap.theme.on_accent
            } else {
                snap.theme.text_secondary
            },
            0,
        );
        // While hovered the whole row is white-on-indigo, so the Y/N color
        // cue (success green / disabled gray) intentionally collapses to
        // white — the correct contrast trade; the cue returns on leave.
        let toggle_fg = if hover && snap.mouse_down {
            snap.theme.text
        } else if hover {
            snap.theme.on_accent
        } else if self.app {
            snap.theme.success
        } else {
            snap.theme.text_disabled
        };
        canvas.draw_char(
            tr.x as u32 + tr.w - 36,
            tr.y as u32 + 6,
            if self.app { 'Y' } else { 'N' },
            toggle_fg,
            0,
        );

        // Window Opacity (placeholder)
        let opacity_y = tr.y as u32 + 32;
        canvas.draw_rounded_rect(cx + 8, opacity_y, cw - 16, 28, 4, snap.theme.bg_elevated);
        canvas.draw_string(
            cx + 16,
            opacity_y + 6,
            "Window Opacity",
            snap.theme.text_secondary,
            0,
        );
        canvas.draw_string(
            cx + cw - 90,
            opacity_y + 6,
            "[slider]",
            snap.theme.text_disabled,
            0,
        );
    }

    fn draw_page_about(
        &self,
        canvas: &mut Canvas,
        snap: &RenderSnapshot,
        cx: u32,
        cy: u32,
        cw: u32,
    ) {
        canvas.draw_string(cx + 10, cy + 10, "About", snap.theme.text, 0);

        // Logo area
        let logo_x = cx + cw / 2 - 40;
        let logo_y = cy + 40;
        canvas.draw_rounded_rect(logo_x, logo_y, 80, 40, 8, snap.theme.accent);
        // Logo sits on the indigo accent -> white text.
        canvas.draw_string(logo_x + 16, logo_y + 14, "SARGA", snap.theme.on_accent, 0);

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
            canvas.draw_string(
                cx + 10,
                info_y + i as u32 * 16,
                line,
                snap.theme.text_secondary,
                0,
            );
        }
    }

    /// Decode a click into an `AppAction`: sidebar → `SelectPage`, the
    /// Appearance theme row → `SetTheme(new_state)` (computed from the
    /// current flag so the coordinator never re-derives it). `None` means
    /// the click hit nothing — the coordinator closes the overlay.
    pub fn hit_test_action(
        &self,
        mx: i32,
        my: i32,
        snap: &RenderSnapshot,
    ) -> Option<crate::apps::AppAction> {
        if !self.open {
            return None;
        }
        let pw = 560u32;
        let ph = 400u32;
        let px = (snap.screen_w - pw) / 2;
        let py = (snap.screen_h - ph) / 3;

        // Sidebar click → page
        let sidebar_x = px + 4;
        let sidebar_y = py + 32;
        for (i, &(_, page)) in PAGES.iter().enumerate() {
            let iy = sidebar_y + 4 + i as u32 * 28;
            if mx >= sidebar_x as i32 + 4
                && mx <= (sidebar_x + 136) as i32
                && my >= iy as i32
                && my <= (iy + 24) as i32
            {
                return Some(crate::apps::AppAction::SelectPage(page));
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
                return Some(crate::apps::AppAction::SetTheme(!self.app));
            }
        }

        None
    }
}
