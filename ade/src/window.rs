use libsarga::gui::Window;
use libsarga::theme::Theme;


pub struct AppWindow {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) w: u32,
    pub(crate) h: u32,
    pub(crate) title: alloc::string::String,
    pub(crate) content: alloc::vec::Vec<alloc::string::String>,
    pub(crate) scroll: u32,
    pub(crate) pid: Option<u64>,
    pub(crate) focused: bool,
    pub(crate) dragging: bool,
    pub(crate) drag_ox: i32,
    pub(crate) drag_oy: i32,
    pub(crate) opacity: u8, // For fade-in animation
}
pub(crate) fn draw(win: &mut Window, theme: &Theme, aw: &AppWindow) {
    if aw.x < -100 || aw.y < -100 {
        return;
    }

    let border_color = if aw.focused {
        theme.accent
    } else {
        theme.border
    };

    // Shadow
    win.draw_rect_alpha(aw.x as u32 + 6, aw.y as u32 + 6, aw.w, aw.h, 0x60000000);

    // Fade-in effect via background fill if not fully opaque
    if aw.opacity < 255 {
        // Just skip rendering or draw with lower contrast
    }

    // Window body
    win.draw_rounded_rect(
        aw.x as u32,
        aw.y as u32,
        aw.w,
        aw.h,
        theme.border_radius,
        theme.bg_surface,
    );
    win.draw_rounded_rect_outline(
        aw.x as u32,
        aw.y as u32,
        aw.w,
        aw.h,
        theme.border_radius,
        border_color,
    );

    // Title bar
    let title_c1 = if aw.focused {
        theme.accent
    } else {
        theme.bg_elevated
    };
    let title_c2 = if aw.focused {
        theme.accent_dark
    } else {
        theme.bg_surface
    };
    win.draw_gradient_rect(
        aw.x as u32 + 1,
        aw.y as u32 + 1,
        aw.w - 2,
        28,
        title_c1,
        title_c2,
        false,
    );
    win.draw_string(aw.x as u32 + 12, aw.y as u32 + 7, &aw.title, 0xFFFFFFFF, 0);

    // Close button
    let close_x = aw.x as u32 + aw.w - 28;
    let close_y = aw.y as u32 + 6;
    win.draw_rounded_rect(close_x, close_y, 22, 18, 4, theme.error);
    win.draw_string(close_x + 7, close_y + 2, "x", 0xFFFFFFFF, 0);

    // Minimize button
    let min_x = aw.x as u32 + aw.w - 54;
    win.draw_rounded_rect(min_x, close_y, 22, 18, 4, theme.bg_elevated);
    win.draw_line_h(min_x + 6, close_y + 14, 10, 0xFFFFFFFF);

    // Content
    let line_y = aw.y as u32 + 28;
    let max_lines = ((aw.h - 34) / 14) as usize;
    let start = if aw.content.len() > max_lines {
        aw.content.len() - max_lines + aw.scroll as usize
    } else {
        0
    };
    for (i, line) in aw.content.iter().skip(start).take(max_lines).enumerate() {
        let ly = line_y + i as u32 * 14;
        if ly + 14 > aw.y as u32 + aw.h {
            break;
        }
        let display = if line.len() > 55 { &line[..55] } else { line };
        win.draw_string(aw.x as u32 + 8, ly, display, theme.text_secondary, 0);
    }
}
