//! Taskbar — bottom bar with start button, window buttons, clock.

use crate::constants::TASKBAR_H;
use crate::geometry::Rect;
use crate::render::snapshot::RenderSnapshot;
use crate::window::WindowState;
use libsarga::gui::Window;

pub(crate) fn draw(win: &mut Window, snap: &RenderSnapshot, clock_str: &str) {
    let ty = snap.taskbar_y();
    let th = snap.theme;
    win.draw_gradient_rect(
        0,
        ty,
        snap.screen_w,
        TASKBAR_H,
        th.bg_surface,
        th.bg_primary,
        true,
    );
    win.draw_line_h(0, ty, snap.screen_w, th.border);

    let start_hover = Rect::new(5, ty as i32 + 4, 58, TASKBAR_H - 8).hit_test(snap.mouse);
    let start_bg = if start_hover { th.hover } else { th.accent };
    win.draw_rounded_rect(5, ty + 4, 58, TASKBAR_H - 8, 6, start_bg);
    win.draw_string(13, ty + 10, "Start", 0xFFFFFFFF, 0);

    for (i, aw) in snap.windows.iter().enumerate() {
        let bx = 75 + i as u32 * 125;
        let is_top = i + 1 == snap.windows.len();
        let is_min = aw.state == WindowState::Minimized;
        let hover = Rect::new(bx as i32, ty as i32 + 4, 120, TASKBAR_H - 8).hit_test(snap.mouse);

        let bg = if is_min {
            th.bg_surface
        } else if is_top {
            th.bg_elevated
        } else if hover {
            th.hover
        } else {
            th.bg_surface
        };
        win.draw_rounded_rect(bx, ty + 4, 120, TASKBAR_H - 8, 6, bg);
        if is_top && !is_min {
            win.draw_line_h(bx + 10, ty + TASKBAR_H - 3, 100, th.accent);
        }
        let display = if aw.title.len() > 14 {
            &aw.title[..14]
        } else {
            &aw.title
        };
        let text_c = if is_top { th.text } else { th.text_secondary };
        win.draw_string(bx + 8, ty + 10, display, text_c, 0);
    }

    let tray_entries = snap.tray;
    let tray_w = tray_entries.len() as u32 * 28 + 20;
    let clock_w = 80u32;
    let panel_w = tray_w + clock_w + 10;
    let panel_x = snap.screen_w - panel_w - 8;
    win.draw_rounded_rect(panel_x, ty + 4, panel_w, TASKBAR_H - 8, 6, th.bg_elevated);
    for (i, entry) in tray_entries.iter().enumerate() {
        let ix = panel_x + 8 + i as u32 * 28;
        win.draw_rounded_rect(ix, ty + 6, 22, 22, 4, 0xFF2D2D40);
        win.draw_char(ix + 6, ty + 9, entry.icon, 0xFFB0B0B0, 0);
    }
    win.draw_string(panel_x + tray_w + 10, ty + 10, clock_str, th.text, 0);
}
