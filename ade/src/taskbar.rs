use libsarga::gui::Window;
use crate::desktop::Desktop;
use crate::constants::TASKBAR_H;

pub(crate) fn draw(win: &mut Window, desktop: &mut Desktop) {
    let ty = desktop.taskbar_y();
    let th = &desktop.theme;
    win.draw_gradient_rect(0, ty, desktop.screen_w, TASKBAR_H, th.bg_surface, th.bg_primary, true);
    win.draw_line_h(0, ty, desktop.screen_w, th.border);

    let start_hover = desktop.mouse_x >= 5
        && desktop.mouse_x < 63
        && desktop.mouse_y >= ty as i32 + 4
        && desktop.mouse_y < ty as i32 + TASKBAR_H as i32 - 4;
    let start_bg = if start_hover { th.hover } else { th.accent };
    win.draw_rounded_rect(5, ty + 4, 58, TASKBAR_H - 8, 6, start_bg);
    win.draw_string(13, ty + 10, "Start", 0xFFFFFFFF, 0);

    for (i, aw) in desktop.wm.windows().iter().enumerate() {
        let bx = 75 + i as u32 * 125;
        let is_top = i == desktop.wm.len() - 1;
        let is_min = aw.x == -9999;
        let hover = desktop.mouse_x >= bx as i32
            && desktop.mouse_x < bx as i32 + 120
            && desktop.mouse_y >= ty as i32 + 4
            && desktop.mouse_y < ty as i32 + TASKBAR_H as i32 - 4;

        let bg = if is_min { th.bg_surface } else if is_top { th.bg_elevated } else if hover { th.hover } else { th.bg_surface };
        win.draw_rounded_rect(bx, ty + 4, 120, TASKBAR_H - 8, 6, bg);
        if is_top && !is_min {
            win.draw_line_h(bx + 10, ty + TASKBAR_H - 3, 100, th.accent);
        }
        let display = if aw.title.len() > 14 { &aw.title[..14] } else { &aw.title };
        let text_c = if is_top { th.text } else { th.text_secondary };
        win.draw_string(bx + 8, ty + 10, display, text_c, 0);
    }

    let tray_x = desktop.screen_w - 180;
    let clock_str = crate::render::clock::format_time(desktop.clock_ticks, &mut desktop.clock_cache);

    win.draw_rounded_rect(tray_x, ty + 4, 175, TASKBAR_H - 8, 6, th.bg_elevated);
    win.draw_string(tray_x + 10, ty + 10, "NET", th.success, 0);
    win.draw_string(tray_x + 50, ty + 10, "VOL", th.accent, 0);
    win.draw_string(tray_x + 100, ty + 10, clock_str, th.text, 0);
}