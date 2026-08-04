//! Window primitives — AppWindow, WindowId, Selection, text cursor input handling.

use libsarga::theme::Theme;

// Window API v1.0 — STABLE
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowId(pub(crate) u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowState {
    Normal,
    Minimized,
    Maximized,
    Fullscreen,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) struct Selection {
    pub start: (u32, u32),
    pub end: (u32, u32),
}

// Window API v1.0 — STABLE
#[derive(Clone, Copy, Debug)]
pub(crate) struct VisualFlags {
    pub shadow: bool,
    pub opacity: u8,
    pub rounded: bool,
    pub border: bool,
    pub active: bool,
    pub border_width: u8,
    pub blur: bool,
    pub transparent: bool,
}

impl VisualFlags {
    pub(crate) fn new() -> Self {
        VisualFlags {
            shadow: true,
            opacity: 255,
            rounded: true,
            border: true,
            active: true,
            border_width: 1,
            blur: false,
            transparent: false,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct AnimState {
    pub from_x: i32,
    pub from_y: i32,
    pub from_w: u32,
    pub from_h: u32,
    pub to_x: i32,
    pub to_y: i32,
    pub to_w: u32,
    pub to_h: u32,
    pub tick: u32,
    pub duration: u32,
}

pub struct AppWindow {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) w: u32,
    pub(crate) h: u32,
    pub(crate) prev_x: i32,
    pub(crate) prev_y: i32,
    pub(crate) prev_w: u32,
    pub(crate) prev_h: u32,
    pub(crate) title: alloc::string::String,
    pub(crate) content: alloc::vec::Vec<alloc::string::String>,
    pub(crate) scroll: u32,
    pub(crate) id: u64,
    pub(crate) pid: Option<u64>,
    pub(crate) focused: bool,
    pub(crate) dragging: bool,
    pub(crate) drag_ox: i32,
    pub(crate) drag_oy: i32,
    pub(crate) state: WindowState,
    pub(crate) prev_state: WindowState,
    pub(crate) flags: VisualFlags,
    #[allow(dead_code)]
    pub(crate) selection: Option<Selection>,
    pub(crate) anim: Option<AnimState>,
    pub(crate) closing: bool,
    pub(crate) always_on_top: bool,
    pub(crate) explorer_id: Option<u32>,
    pub(crate) anim_opacity: u8,
}

impl AppWindow {
    pub(crate) fn animate_to(&mut self, x: i32, y: i32, w: u32, h: u32) {
        self.anim = Some(AnimState {
            from_x: self.x,
            from_y: self.y,
            from_w: self.w,
            from_h: self.h,
            to_x: x,
            to_y: y,
            to_w: w,
            to_h: h,
            tick: 0,
            duration: 10,
        });
    }

    pub(crate) fn tick_animation(&mut self) -> bool {
        if let Some(ref mut a) = self.anim {
            a.tick += 1;
            let t = a.tick.min(a.duration);
            if t >= a.duration {
                self.x = a.to_x;
                self.y = a.to_y;
                self.w = a.to_w;
                self.h = a.to_h;
                self.anim = None;
            } else {
                let d = a.duration as f32;
                let u = 1.0 - t as f32 / d;
                let progress = 1.0 - u * u * u;
                self.x = a.from_x + ((a.to_x - a.from_x) as f32 * progress) as i32;
                self.y = a.from_y + ((a.to_y - a.from_y) as f32 * progress) as i32;
                self.w = a.from_w + ((a.to_w as f32 - a.from_w as f32) * progress) as u32;
                self.h = a.from_h + ((a.to_h as f32 - a.from_h as f32) * progress) as u32;
            }
            true
        } else {
            false
        }
    }

    pub(crate) fn animate_close(&mut self) {
        let cx = self.x + self.w as i32 / 2;
        let cy = self.y + self.h as i32 / 2;
        self.anim = Some(AnimState {
            from_x: self.x,
            from_y: self.y,
            from_w: self.w,
            from_h: self.h,
            to_x: cx - 1,
            to_y: cy - 1,
            to_w: 2,
            to_h: 2,
            tick: 0,
            duration: 8,
        });
    }
}

fn apply_alpha(color: u32, opacity: u8) -> u32 {
    (color & 0x00FFFFFF) | ((opacity as u32) << 24)
}

pub(crate) fn draw(
    canvas: &mut crate::render::compositor::Canvas,
    theme: &Theme,
    aw: &AppWindow,
    cursor_visible: bool,
    explorers: &[crate::util::explorer::ExplorerState],
) {
    // Don't draw minimized windows (but still draw during animation).
    if aw.state == WindowState::Minimized && aw.anim.is_none() {
        return;
    }

    // Safety check.
    if aw.x < -100 || aw.y < -100 {
        return;
    }

    let border_color = if aw.focused {
        theme.accent
    } else {
        theme.border
    };

    // Shadow
    canvas.draw_rect_alpha(aw.x as u32 + 6, aw.y as u32 + 6, aw.w, aw.h, 0x60000000);

    // Window body
    canvas.draw_rounded_rect(
        aw.x as u32,
        aw.y as u32,
        aw.w,
        aw.h,
        theme.border_radius,
        apply_alpha(theme.bg_surface, aw.flags.opacity),
    );

    // Focus glow
    if aw.focused {
        canvas.draw_rounded_rect_outline(
            aw.x as u32 - 1,
            aw.y as u32 - 1,
            aw.w + 2,
            aw.h + 2,
            theme.border_radius + 1,
            0x303D5AFE,
        );
    }

    canvas.draw_rounded_rect_outline(
        aw.x as u32,
        aw.y as u32,
        aw.w,
        aw.h,
        theme.border_radius,
        apply_alpha(border_color, aw.flags.opacity),
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

    canvas.draw_gradient_rect(
        aw.x as u32 + 1,
        aw.y as u32 + 1,
        aw.w - 2,
        28,
        apply_alpha(title_c1, aw.flags.opacity),
        apply_alpha(title_c2, aw.flags.opacity),
        false,
    );

    canvas.draw_string(
        aw.x as u32 + 12,
        aw.y as u32 + 7,
        &aw.title,
        apply_alpha(0xFFFFFFFF, aw.flags.opacity),
        0,
    );

    if aw.always_on_top {
        canvas.draw_string(
            aw.x as u32 + aw.w - 82,
            aw.y as u32 + 7,
            "[A]",
            apply_alpha(0xFFFFAA00, aw.flags.opacity),
            0,
        );
    }

    // Close button
    let close_x = aw.x as u32 + aw.w - 28;
    let close_y = aw.y as u32 + 6;

    canvas.draw_rounded_rect(
        close_x,
        close_y,
        22,
        18,
        4,
        apply_alpha(theme.error, aw.flags.opacity),
    );
    canvas.draw_string(
        close_x + 7,
        close_y + 2,
        "x",
        apply_alpha(0xFFFFFFFF, aw.flags.opacity),
        0,
    );

    // Minimize button
    let min_x = aw.x as u32 + aw.w - 54;
    canvas.draw_rounded_rect(
        min_x,
        close_y,
        22,
        18,
        4,
        apply_alpha(theme.bg_elevated, aw.flags.opacity),
    );
    canvas.draw_line_h(
        min_x + 6,
        close_y + 14,
        10,
        apply_alpha(0xFFFFFFFF, aw.flags.opacity),
    );

    // Separation line
    canvas.draw_line_h(
        aw.x as u32 + 1,
        aw.y as u32 + 29,
        aw.w - 2,
        apply_alpha(theme.separator, aw.flags.opacity),
    );

    // Explorer content
    if let Some(exp_id) = aw.explorer_id {
        crate::util::explorer::draw_explorer_content(canvas, theme, aw, explorers, exp_id);
        return;
    }

    // Window content
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

        canvas.draw_string(
            aw.x as u32 + 8,
            ly,
            display,
            apply_alpha(theme.text_secondary, aw.flags.opacity),
            0,
        );
    }

    if cursor_visible && aw.focused && !aw.content.is_empty() {
        let last = &aw.content[aw.content.len() - 1];
        let cx = aw.x as u32 + 8 + last.len() as u32 * 8;
        let cy = aw.y as u32
            + 30
            + (aw.content.len().saturating_sub(1) as u32 - aw.scroll).saturating_sub(1) * 14;
        if cy < aw.y as u32 + aw.h {
            canvas.draw_char(cx, cy, '_', apply_alpha(theme.accent, aw.flags.opacity), 0);
        }
    }
}
