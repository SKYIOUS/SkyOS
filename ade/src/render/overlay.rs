//! Overlay drawing — context menu, window switcher, notifications.

use crate::render::compositor::Canvas;
use crate::render::snapshot::RenderSnapshot;

pub(crate) fn draw_clipboard(canvas: &mut Canvas, snap: &RenderSnapshot) {
    let cb = match snap.clipboard {
        Some(c) => c,
        None => return,
    };
    if !cb.panel_open {
        return;
    }
    let pw = 280u32;
    let ph = (cb.history.len() as u32 * 28 + 16).min(300);
    let px = (snap.screen_w - pw) / 2;
    let py = (snap.screen_h - ph) / 3;
    canvas.draw_rect_alpha(0, 0, snap.screen_w, snap.screen_h, 0x40000000);
    canvas.draw_rounded_rect(px, py, pw, ph, 8, 0xFF2D2D2D);
    canvas.draw_rounded_rect_outline(px, py, pw, ph, 8, 0xFF555555);
    canvas.draw_string(px + 10, py + 6, "Clipboard History", 0xFFFFFFFF, 0);
    for (i, entry) in cb.history.iter().enumerate() {
        let iy = py + 30 + i as u32 * 28;
        if iy + 24 > py + ph {
            break;
        }
        let hover =
            crate::geometry::Rect::new(px as i32 + 4, iy as i32, pw - 8, 24).hit_test(snap.mouse);
        let bg = if hover { 0xFF3A3A5C } else { 0xFF2D2D2D };
        canvas.draw_rounded_rect(px + 4, iy, pw - 8, 24, 4, bg);
        let txt = if entry.text.len() > 28 {
            &entry.text[..28]
        } else {
            &entry.text
        };
        canvas.draw_string(px + 10, iy + 5, txt, 0xFFD0D0D0, 0);
        if entry.pinned {
            canvas.draw_char(px + pw - 22, iy + 5, 'P', 0xFFFFAA00, 0);
        }
    }
}

pub(crate) fn draw_notifications(canvas: &mut Canvas, snap: &RenderSnapshot) {
    let mut ny = 10i32;
    for n in snap.notifications {
        let c = if n.priority >= 2 {
            0xFFFF4444u32
        } else {
            snap.theme.accent
        };
        canvas.draw_rounded_rect(snap.screen_w - 310, ny as u32, 300, 64, 8, 0xFF2D2D2D);
        canvas.draw_rounded_rect_outline(snap.screen_w - 310, ny as u32, 300, 64, 8, c);
        canvas.draw_string(snap.screen_w - 302, ny as u32 + 6, &n.title, 0xFFFFFFFF, 0);
        let body = if n.body.len() > 36 {
            &n.body[..36]
        } else {
            &n.body
        };
        canvas.draw_string(snap.screen_w - 302, ny as u32 + 26, body, 0xFFB0B0B0, 0);
        if n.timeout > 0 {
            let frac = n.ticks_left.max(1) as u32 * 290 / n.timeout;
            canvas.draw_rect(snap.screen_w - 305, ny as u32 + 56, frac, 3, c);
        }
        ny += 72;
        if ny > 300 {
            break;
        }
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
    canvas.draw_rect_alpha(0, 0, snap.screen_w, snap.screen_h, 0x40000000);
    canvas.draw_rounded_rect(bx, by, bw, bh, 8, 0xFF2D2D2D);
    canvas.draw_rounded_rect_outline(bx, by, bw, bh, 8, 0xFF555555);

    for (i, aw) in snap.windows.iter().enumerate() {
        let iy = by + 8 + i as u32 * 32;
        let selected = i == snap.switcher_idx;
        if selected {
            canvas.draw_rounded_rect(bx + 4, iy, bw - 8, 28, 4, 0xFF4A6FA5);
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
            if selected { 0xFFFFFFFF } else { 0xFFAAAAAA },
            0,
        );
    }
}

pub(crate) fn draw_context_menu(canvas: &mut Canvas, snap: &RenderSnapshot) {
    if let Some((mx, my, items)) = snap.context_menu {
        let mw = 150u32;
        let mh = items.len() as u32 * 28 + 10;
        canvas.draw_rounded_rect(mx as u32, my as u32, mw, mh, 6, snap.theme.bg_elevated);
        canvas.draw_rounded_rect_outline(mx as u32, my as u32, mw, mh, 6, snap.theme.border);
        for (i, (name, _)) in items.iter().enumerate() {
            let iy = my as u32 + 5 + i as u32 * 28;
            canvas.draw_string(mx as u32 + 10, iy + 6, name, snap.theme.text, 0);
        }
    }
}
