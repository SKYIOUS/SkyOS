//! Overlay drawing — context menu, window switcher, notifications.

use crate::core::window::HoverTarget;
use crate::layout;
use crate::render::compositor::Canvas;
use crate::render::snapshot::RenderSnapshot;

pub(crate) fn draw_clipboard(canvas: &mut Canvas, snap: &RenderSnapshot) {
    let cb = match snap.clipboard {
        Some(c) => c,
        None => return,
    };
    if cb.is_empty() {
        return;
    }
    let hist = cb.history();
    let panel = layout::clipboard_panel_rect(snap.screen_w, snap.screen_h, hist.len());
    crate::core::dialog::draw_backdrop(canvas, snap.screen_w, snap.screen_h, snap.theme);
    crate::core::dialog::draw_panel(
        canvas,
        panel.x as u32,
        panel.y as u32,
        panel.w,
        panel.h,
        "Clipboard History",
        snap.theme,
    );
    for (i, entry) in hist.iter().enumerate() {
        let row = layout::clipboard_row_rect(panel, i);
        if row.y + layout::CLIPBOARD_ROW_INNER_H as i32 > panel.y + panel.h as i32 {
            break;
        }
        let hover = snap.hover == Some(HoverTarget::ClipboardRow(i));
        let bg = if hover {
            snap.theme.hover
        } else {
            snap.theme.bg_surface
        };
        canvas.draw_rounded_rect(row.x as u32, row.y as u32, row.w, row.h, 4, bg);
        let txt = if entry.text.len() > 28 {
            &entry.text[..28]
        } else {
            &entry.text
        };
        canvas.draw_string(
            row.x as u32 + 10,
            row.y as u32 + 5,
            txt,
            // Hovered row is the theme-invariant indigo -> white text (see
            // the notification arm); the base surface keeps the gray.
            if hover {
                snap.theme.on_accent
            } else {
                snap.theme.text_secondary
            },
            0,
        );
    }
}

pub(crate) fn draw_switcher(canvas: &mut Canvas, snap: &RenderSnapshot) {
    let n = snap.windows.len();
    if n == 0 {
        return;
    }
    let bw = 300u32;
    let bh = (n as u32 * 32 + 16).min(snap.screen_h / 2);
    let bx = (snap.screen_w - bw) / 2;
    let by = (snap.screen_h - bh) / 3;

    // semi-transparent backdrop
    canvas.draw_rect_alpha(0, 0, snap.screen_w, snap.screen_h, snap.theme.shadow);
    canvas.draw_rounded_rect(bx, by, bw, bh, 8, snap.theme.bg_surface);
    canvas.draw_rounded_rect_outline(bx, by, bw, bh, 8, snap.theme.border);

    for (i, aw) in snap.windows.iter().enumerate() {
        let iy = by + 8 + i as u32 * 32;
        let selected = i == snap.switcher_idx;
        if selected {
            canvas.draw_rounded_rect(bx + 4, iy, bw - 8, 28, 4, snap.theme.accent);
        }
        let label = if aw.title.len() > 28 {
            &aw.title[..28]
        } else {
            &aw.title
        };
        canvas.draw_string(
            bx + 12,
            iy + 6,
            label,
            // The selected row fills with the indigo accent -> white text
            // (see the notification arm); base rows keep the gray.
            if selected {
                snap.theme.on_accent
            } else {
                snap.theme.text_secondary
            },
            0,
        );
    }
}

pub(crate) fn draw_context_menu(canvas: &mut Canvas, snap: &RenderSnapshot) {
    if let Some(cm) = snap.context_menu {
        let mw = 150u32;
        let mh = cm.items.len() as u32 * 28 + 10;
        canvas.draw_rounded_rect(cm.x as u32, cm.y as u32, mw, mh, 6, snap.theme.bg_elevated);
        canvas.draw_rounded_rect_outline(cm.x as u32, cm.y as u32, mw, mh, 6, snap.theme.border);
        for (i, item) in cm.items.iter().enumerate() {
            let iy = cm.y as u32 + 5 + i as u32 * 28;
            canvas.draw_string(cm.x as u32 + 10, iy + 6, item.label, snap.theme.text, 0);
        }
    }
}
