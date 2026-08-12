//! Task manager — process list overlay with focus/close actions.

use crate::render::compositor::Canvas;
use crate::render::snapshot::RenderSnapshot;

pub(crate) struct TaskManagerState {
    pub open: bool,
    pub selected: usize,
}

impl TaskManagerState {
    pub fn new() -> Self {
        TaskManagerState {
            open: false,
            selected: 0,
        }
    }

    pub fn draw(&self, canvas: &mut Canvas, snap: &RenderSnapshot) {
        if !self.open {
            return;
        }
        // Panel + row geometry comes from `layout` — the same rects
        // `Desktop::hover_target` hit-tests, so hover always lights the
        // drawn row.
        let panel = crate::layout::task_manager_panel_rect(snap.screen_w, snap.screen_h);
        crate::core::dialog::draw_backdrop(canvas, snap.screen_w, snap.screen_h, snap.theme);
        crate::core::dialog::draw_panel(
            canvas,
            panel.x as u32,
            panel.y as u32,
            panel.w,
            panel.h,
            "Task Manager",
            snap.theme,
        );

        // Column headers
        let header_y = panel.y as u32 + 32;
        canvas.draw_rect(
            panel.x as u32 + 4,
            header_y,
            panel.w - 8,
            20,
            snap.theme.bg_elevated,
        );
        // Column headers are functional labels — text_secondary keeps them
        // readable in light mode where text_disabled would vanish on the
        // elevated surface.
        canvas.draw_string(
            panel.x as u32 + 10,
            header_y + 4,
            "PID",
            snap.theme.text_secondary,
            0,
        );
        canvas.draw_string(
            panel.x as u32 + 60,
            header_y + 4,
            "Name",
            snap.theme.text_secondary,
            0,
        );
        canvas.draw_string(
            panel.x as u32 + 220,
            header_y + 4,
            "State",
            snap.theme.text_secondary,
            0,
        );
        canvas.draw_string(
            panel.x as u32 + 310,
            header_y + 4,
            "Memory",
            snap.theme.text_secondary,
            0,
        );
        canvas.draw_string(
            panel.x as u32 + 400,
            header_y + 4,
            "CPU",
            snap.theme.text_secondary,
            0,
        );

        // Process list — hover/pressed come from the unified hover state
        // (`Desktop::hover_target`); the indigo hover/selected fills carry
        // white text, the pressed fill keeps the theme text.
        let count = crate::layout::task_manager_max_visible(panel).min(snap.windows.len());
        for (i, w) in snap.windows.iter().enumerate().take(count) {
            let r = crate::layout::task_manager_row_rect(panel, i);
            let sel = i == self.selected;
            let hover = snap.hover == Some(crate::core::window::HoverTarget::TaskManagerRow(i));
            // Zebra order mirrors the original palette (even rows lighter):
            // bg_elevated is lighter than bg_surface in both themes.
            // Pressed beats selected (the start-menu row convention): a held
            // selected row shows the pressed fill with theme.text, never
            // black text on the indigo accent (4.09:1 in the light theme).
            let bg = if hover && snap.mouse_down {
                snap.theme.pressed
            } else if sel {
                snap.theme.accent
            } else if hover {
                snap.theme.hover
            } else if i % 2 == 0 {
                snap.theme.bg_elevated
            } else {
                snap.theme.bg_surface
            };
            canvas.draw_rect(r.x as u32, r.y as u32, r.w, r.h, bg);
            let pid_str = match w.pid {
                Some(p) => alloc::format!("{}", p),
                None => alloc::string::String::new(),
            };
            let state_str = match w.state {
                crate::core::window::WindowState::Normal => "Running",
                crate::core::window::WindowState::Minimized => "Minimized",
                crate::core::window::WindowState::Maximized => "Maximized",
                crate::core::window::WindowState::Fullscreen => "Fullscreen",
            };
            // The selected/hovered rows fill with indigo -> white text
            // (theme.text flips black in the light theme); the pressed fill
            // keeps the theme text.
            let fg = if hover && snap.mouse_down {
                snap.theme.text
            } else if sel || hover {
                snap.theme.on_accent
            } else {
                snap.theme.text_secondary
            };
            canvas.draw_string(r.x as u32 + 10, r.y as u32 + 4, &pid_str, fg, 0);
            let title = if w.title.len() > 22 {
                &w.title[..22]
            } else {
                &w.title
            };
            canvas.draw_string(r.x as u32 + 60, r.y as u32 + 4, title, fg, 0);
            canvas.draw_string(r.x as u32 + 220, r.y as u32 + 4, state_str, fg, 0);
            canvas.draw_string(r.x as u32 + 310, r.y as u32 + 4, "\u{2014}", fg, 0);
            canvas.draw_string(r.x as u32 + 400, r.y as u32 + 4, "\u{2014}", fg, 0);
        }
    }

    /// Decode a click into an `AppAction`: a row hit → `FocusWindow(idx)`,
    /// everything else → `None` (coordinator closes the overlay).
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
        let ph = 360u32;
        let px = (snap.screen_w - pw) / 2;
        let py = (snap.screen_h - ph) / 3;
        let header_y = py + 32;
        let list_y = header_y + 22;
        let item_h = 20u32;
        let rel = (my as u32).saturating_sub(list_y);
        let idx = (rel / item_h) as usize;
        if idx < snap.windows.len()
            && mx >= px as i32 + 4
            && mx <= (px + pw - 4) as i32
            && (my as u32) >= list_y
            && (my as u32) < list_y + snap.windows.len() as u32 * item_h
        {
            return Some(crate::apps::AppAction::FocusWindow(idx));
        }
        None
    }
}
