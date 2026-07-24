//! Render pipeline — composite layers via the compositor onto the window surface.

pub(crate) mod clock;
pub(crate) mod compositor;
pub(crate) mod layer;
pub(crate) mod notification_overlay;
pub(crate) mod overlay;
pub(crate) mod snapshot;

use alloc::format;
use layer::Layer;
use compositor::Compositor;

pub(crate) fn render(
    win: &mut libsarga::gui::Window,
    snap: &snapshot::RenderSnapshot,
    clock_str: &str,
    comp: &mut Compositor,
) {
    comp.clear_all();

    // Wallpaper
    {
        let mut cv = comp.layer_canvas(Layer::Wallpaper);
        crate::core::wallpaper::draw(&mut cv, snap);
    }

    // Fade-in overlay for windows being animated in
    {
        let mut cv = comp.layer_canvas(Layer::Overlay);
        for aw in snap.windows {
            if aw.flags.opacity < 255 {
                let alpha = ((255 - aw.flags.opacity) as u32) << 24;
                cv.draw_rect_alpha(aw.x as u32, aw.y as u32, aw.w, aw.h, alpha);
            }
        }
    }

    if !snap.fullscreen {
        // Desktop icons
        let mut cv = comp.layer_canvas(Layer::Desktop);
        crate::core::desktop_icons::draw(&mut cv, snap.icons, snap.theme, snap.rubber);
    }

    // Windows (normal then always-on-top)
    {
        let mut cv = comp.layer_canvas(Layer::Windows);
        for aw in snap.windows {
            if !aw.always_on_top {
                if aw.flags.shadow {
                    cv.draw_shadow(aw.x as u32, aw.y as u32, aw.w, aw.h, 8, 0x60000000);
                }
                crate::core::window::draw(&mut cv, snap.theme, aw, snap.cursor_visible, snap.explorers);
            }
        }
        for aw in snap.windows {
            if aw.always_on_top {
                if aw.flags.shadow {
                    cv.draw_shadow(aw.x as u32, aw.y as u32, aw.w, aw.h, 8, 0x60000000);
                }
                crate::core::window::draw(&mut cv, snap.theme, aw, snap.cursor_visible, snap.explorers);
            }
        }
    }

    if !snap.fullscreen {
        let mut cv = comp.layer_canvas(Layer::Popups);
        crate::core::taskbar::draw(&mut cv, snap, clock_str);

        if snap.start_menu {
            crate::core::start_menu::draw(&mut cv, snap);
        }
    }

    // Overlay
    {
        let mut cv = comp.layer_canvas(Layer::Overlay);
        overlay::draw_context_menu(&mut cv, snap);
        overlay::draw_clipboard(&mut cv, snap);
        notification_overlay::draw_notifications(&mut cv, snap.notifications, snap.mouse, snap.theme);
        if let Some(s) = snap.settings {
            s.draw(&mut cv, snap);
        }
        if let Some(sa) = snap.settings_app {
            sa.draw(&mut cv, snap);
        }
        if let Some(tm) = snap.task_manager {
            tm.draw(&mut cv, snap.windows, snap.theme);
        }
        if let Some(ab) = snap.about {
            ab.draw(&mut cv, snap);
        }
        if snap.switcher_active {
            overlay::draw_switcher(&mut cv, snap);
        }
        // Focus indicator
        if snap.focus_visible {
            if let Some((fx, fy, fw, fh)) = snap.focused_bounds {
                let c = snap.theme.accent;
                let len = 12u32;
                // top-left
                cv.draw_line_h(fx as u32, fy as u32, len, c);
                cv.draw_line_h(fx as u32, fy as u32 + 1, len, c);
                cv.draw_line_v(fx as u32, fy as u32, len, c);
                cv.draw_line_v(fx as u32 + 1, fy as u32, len, c);
                // top-right
                cv.draw_line_h(fx as u32 + fw - len, fy as u32, len, c);
                cv.draw_line_h(fx as u32 + fw - len, fy as u32 + 1, len, c);
                cv.draw_line_v(fx as u32 + fw - 1, fy as u32, len, c);
                cv.draw_line_v(fx as u32 + fw - 2, fy as u32, len, c);
                // bottom-left
                cv.draw_line_h(fx as u32, fy as u32 + fh - 1, len, c);
                cv.draw_line_h(fx as u32, fy as u32 + fh - 2, len, c);
                cv.draw_line_v(fx as u32, fy as u32 + fh - len, len, c);
                cv.draw_line_v(fx as u32 + 1, fy as u32 + fh - len, len, c);
                // bottom-right
                cv.draw_line_h(fx as u32 + fw - len, fy as u32 + fh - 1, len, c);
                cv.draw_line_h(fx as u32 + fw - len, fy as u32 + fh - 2, len, c);
                cv.draw_line_v(fx as u32 + fw - 1, fy as u32 + fh - len, len, c);
                cv.draw_line_v(fx as u32 + fw - 2, fy as u32 + fh - len, len, c);
            }
        }
        // Tooltip
        if let Some(tt) = snap.tooltip {
            if !tt.is_empty() {
                let tw = (tt.len() as u32 * 8 + 16).min(snap.screen_w.saturating_sub(8));
                let mut tx = snap.tooltip_x;
                let mut ty = snap.tooltip_y.saturating_sub(26);
                if tx + tw as i32 > snap.screen_w as i32 {
                    tx = (snap.screen_w as i32).saturating_sub(tw as i32 + 4);
                }
                if tx < 2 {
                    tx = 2;
                }
                if ty < 2 {
                    ty = snap.tooltip_y + 20;
                }
                cv.draw_rounded_rect(tx as u32, ty as u32, tw, 22, 4, 0xFF2D2D2D);
                cv.draw_rounded_rect_outline(tx as u32, ty as u32, tw, 22, 4, 0xFF555555);
                cv.draw_string(tx as u32 + 8, ty as u32 + 5, tt, 0xFFFFFFFF, 0);
            }
        }
        // Snap preview (translucent rect showing where window will land)
        if let Some((sx, sy, sw, sh)) = snap.snap_preview {
            cv.draw_rect_alpha(sx as u32, sy as u32, sw, sh, crate::core::constants::SNAP_PREVIEW_COLOR);
        }
        // Debug overlay (F12)
        if snap.debug_overlay {
            cv.draw_rect_alpha(snap.screen_w - 300, 0, 300, 200, 0xA0000000);
            let mut ly = 10u32;
            let x = snap.screen_w - 290;
            let fps = 62 / snap.debug_metrics.frame_time_avg.max(1);
            cv.draw_string(x, ly, &format!("FPS: {}", fps), 0xFFFFFF00, 0);
            ly += 16;
            cv.draw_string(x, ly, &format!("Frame: {} ticks", snap.debug_metrics.frame_time_avg), 0xFFFFFF00, 0);
            ly += 16;
            cv.draw_string(x, ly, &format!("Mem: {}B", snap.debug_metrics.heap_usage), 0xFFFFFF00, 0);
            ly += 16;
            cv.draw_string(x, ly, &format!("Windows: {}", snap.window_count), 0xFFFFFF00, 0);
            ly += 16;
            cv.draw_string(x, ly, &format!("Notifs: {}", snap.notification_count), 0xFFFFFF00, 0);
            ly += 16;
            cv.draw_string(x, ly, &format!("Mouse: {},{}", snap.mouse.x, snap.mouse.y), 0xFFFFFF00, 0);
            ly += 16;
            cv.draw_string(x, ly, &format!("IPC msgs: {}", snap.debug_metrics.event_dispatch_count), 0xFFFFFF00, 0);
            ly += 16;
            cv.draw_string(x, ly, &format!("Dirty: {}", snap.debug_metrics.dirty_regions), 0xFFFFFF00, 0);
        }
    }

    // Cursor layer
    {
        let mut cv = comp.layer_canvas(Layer::Cursor);
        let alpha = snap.cursor_alpha;
        if alpha > 0 {
            let mx = snap.mouse.x as u32;
            let my = snap.mouse.y as u32;
            // Simple arrow cursor (12x16), hotspot at top-left (tip of arrow)
            const CURSOR_BITMAP: [u16; 16] = [
                0b100000000000,
                0b110000000000,
                0b111000000000,
                0b111100000000,
                0b111110000000,
                0b111111000000,
                0b111111100000,
                0b111111110000,
                0b111111111000,
                0b111111111100,
                0b111111111110,
                0b111111111111,
                0b111111111110,
                0b111111111100,
                0b111111111000,
                0b111111110000,
            ];
            let color = (alpha as u32) << 24 | 0xFFFFFF;
            for (row, &bits) in CURSOR_BITMAP.iter().enumerate() {
                for col in 0..12 {
                    if (bits >> (11 - col)) & 1 != 0 {
                        cv.fill_pixel(mx.wrapping_add(col), my.wrapping_add(row as u32), color);
                    }
                }
            }
        }
    }

    comp.compose(win, None);
}
