//! Settings panel — sound, theme, display toggles.

use crate::render::compositor::Canvas;
use crate::render::snapshot::RenderSnapshot;

pub(crate) struct SettingsState {
    pub open: bool,
    pub sound_on: bool,
    pub theme_dark: bool,
}

impl SettingsState {
    pub fn new() -> Self {
        SettingsState {
            open: false,
            sound_on: true,
            theme_dark: true,
        }
    }

    pub fn toggle(&mut self) {
        self.open = !self.open;
    }

    pub fn draw(&self, canvas: &mut Canvas, snap: &RenderSnapshot) {
        if !self.open {
            return;
        }
        // Panel + row geometry comes from `layout` — the same rects
        // `Desktop::hover_target` hit-tests, so hover always lights the
        // drawn row.
        let panel = crate::layout::settings_panel_rect(snap.screen_w, snap.screen_h);
        canvas.draw_rect_alpha(0, 0, snap.screen_w, snap.screen_h, snap.theme.shadow);
        canvas.draw_rounded_rect(
            panel.x as u32,
            panel.y as u32,
            panel.w,
            panel.h,
            8,
            snap.theme.bg_surface,
        );
        canvas.draw_rounded_rect_outline(
            panel.x as u32,
            panel.y as u32,
            panel.w,
            panel.h,
            8,
            snap.theme.border,
        );
        canvas.draw_string(
            panel.x as u32 + 10,
            panel.y as u32 + 6,
            "Settings",
            snap.theme.text,
            0,
        );

        let rows = [("Sound", self.sound_on), ("Dark Theme", self.theme_dark)];
        for (i, (label, val)) in rows.iter().enumerate() {
            // Hover/pressed come from the unified hover state, not per-panel
            // mouse hit-testing (the old `hover_idx` was never even set).
            // Indigo hover fill -> white text; pressed (light gray in light
            // mode) keeps the theme text.
            let r = crate::layout::settings_row_rect(panel, i);
            let hover = snap.hover == Some(crate::core::window::HoverTarget::SettingsRow(i));
            let bg = if hover && snap.mouse_down {
                snap.theme.pressed
            } else if hover {
                snap.theme.hover
            } else {
                snap.theme.bg_elevated
            };
            canvas.draw_rounded_rect(r.x as u32, r.y as u32, r.w, r.h, 4, bg);
            // Hovered row is the indigo fill -> white text (see the
            // notification arm); pressed keeps the theme text; the base
            // surface keeps the gray.
            canvas.draw_string(
                r.x as u32 + 16,
                r.y as u32 + 6,
                label,
                if hover && snap.mouse_down {
                    snap.theme.text
                } else if hover {
                    snap.theme.on_accent
                } else {
                    snap.theme.text_secondary
                },
                0,
            );
            // Hovered: the whole row is white-on-indigo, so the Y/N color
            // cue collapses to white (see the settings-app arm); on leave the
            // success/disabled cue returns. The label switches color the same
            // way.
            let toggle_fg = if hover && snap.mouse_down {
                snap.theme.text
            } else if hover {
                snap.theme.on_accent
            } else if *val {
                snap.theme.success
            } else {
                snap.theme.text_disabled
            };
            canvas.draw_char(
                r.x as u32 + r.w - 40,
                r.y as u32 + 6,
                if *val { 'Y' } else { 'N' },
                toggle_fg,
                0,
            );
        }

        let cr = crate::layout::settings_close_rect(panel);
        let close_hover = snap.hover == Some(crate::core::window::HoverTarget::SettingsRow(2));
        let cb = if close_hover && snap.mouse_down {
            snap.theme.pressed
        } else if close_hover {
            snap.theme.hover
        } else {
            snap.theme.bg_elevated
        };
        canvas.draw_rounded_rect(cr.x as u32, cr.y as u32, cr.w, cr.h, 4, cb);
        canvas.draw_string(
            cr.x as u32 + 40,
            cr.y as u32 + 6,
            "Close",
            if close_hover && snap.mouse_down {
                snap.theme.text
            } else if close_hover {
                snap.theme.on_accent
            } else {
                snap.theme.text_secondary
            },
            0,
        );
    }

    /// Decode a click into an `AppAction`: Sound row → `ToggleSound`, Dark
    /// Theme row → `SetTheme(new_state)` (computed from the current flag),
    /// Close button → `Close`. `None` → the coordinator closes the panel.
    pub fn hit_test_action(
        &self,
        mx: i32,
        my: i32,
        snap: &RenderSnapshot,
    ) -> Option<crate::apps::AppAction> {
        if !self.open {
            return None;
        }
        let pw = 320u32;
        let ph = 200u32;
        let px = (snap.screen_w - pw) / 2;
        let py = (snap.screen_h - ph) / 3;
        for i in 0..2 {
            let iy = py + 36 + i as u32 * 32;
            if mx >= px as i32 + 8
                && mx <= (px + pw - 8) as i32
                && my >= iy as i32
                && my <= (iy + 28) as i32
            {
                return Some(if i == 0 {
                    crate::apps::AppAction::ToggleSound
                } else {
                    crate::apps::AppAction::SetTheme(!self.theme_dark)
                });
            }
        }
        let cy = py + ph - 36;
        if mx >= (px + 100) as i32
            && mx <= (px + 220) as i32
            && my >= cy as i32
            && my <= (cy + 28) as i32
        {
            return Some(crate::apps::AppAction::Close);
        }
        None
    }
}
