//! Taskbar — bottom bar with start button, window buttons, clock.

use crate::core::window::{
    window_button_face, HoverTarget, WindowButtonFace, WindowId, WindowState,
};
use crate::layout::{self, TASKBAR_H};
use crate::render::compositor::Canvas;
use crate::render::snapshot::RenderSnapshot;
use libsarga::theme::Theme;

/// The taskbar window-button fill for one interaction state, resolved by
/// priority: pressed (hover held) > keyboard focus (the `accent_light`
/// blue, visually distinct from pointer hover at a glance) > hover
/// (indigo `th.hover`) > minimized > top window > resting. Pure so the
/// focus-vs-hover color contract is selftest-pinnable without pixel
/// tests — `test_a11y_taskbar_focus_feedback` pins the exact choice.
pub(crate) fn window_button_fill(
    focused: bool,
    hover: bool,
    pressed: bool,
    minimized: bool,
    top: bool,
    th: &Theme,
) -> u32 {
    if pressed {
        th.pressed
    } else if focused {
        th.accent_light
    } else if hover {
        th.hover
    } else if minimized {
        th.bg_surface
    } else if top {
        th.bg_elevated
    } else {
        th.bg_surface
    }
}

/// The Start-button fill — same focus-vs-hover distinction as the window
/// buttons: pressed > focused (accent_light blue) > hover (indigo) > the
/// resting accent.
pub(crate) fn start_button_fill(focused: bool, hover: bool, pressed: bool, th: &Theme) -> u32 {
    if pressed {
        th.pressed
    } else if focused {
        th.accent_light
    } else if hover {
        th.hover
    } else {
        th.accent
    }
}

pub(crate) fn draw(canvas: &mut Canvas, snap: &RenderSnapshot, clock_str: &str) {
    let ty = snap.taskbar_y();
    let th = snap.theme;
    canvas.draw_gradient_rect(
        0,
        ty,
        snap.screen_w,
        TASKBAR_H,
        th.bg_surface,
        th.bg_primary,
        true,
    );
    canvas.draw_line_h(0, ty, snap.screen_w, th.border);

    let start_btn = layout::start_btn_rect(ty);
    let start_focused = snap.focused == Some(HoverTarget::StartButton);
    // Pressed: the surface under a held-down primary button darkens — the
    // same hover+pressed combination the window control buttons use. The
    // focused Start button gets the accent_light blue like the window
    // buttons, distinct from the indigo hover.
    let start_hover = snap.hover == Some(HoverTarget::StartButton);
    let start_pressed = start_hover && snap.mouse_down;
    let start_bg = start_button_fill(start_focused, start_hover, start_pressed, th);
    canvas.draw_rounded_rect(
        start_btn.x as u32,
        start_btn.y as u32,
        start_btn.w,
        start_btn.h,
        6,
        start_bg,
    );
    // The start button sits on the indigo accent/hover fill in both themes,
    // so its label is on_accent (white) — th.text flips to black in the
    // light theme and would vanish. Only the pressed fill (light gray in
    // light mode) keeps th.text.
    canvas.draw_string(
        start_btn.x as u32 + 8,
        start_btn.y as u32 + 6,
        "Start",
        if start_pressed { th.text } else { th.on_accent },
        0,
    );

    let overflow = snap.windows.len() > layout::TASKBAR_MAX_BTNS;

    for (i, aw) in snap.windows.iter().enumerate() {
        if overflow && i >= layout::TASKBAR_MAX_BTNS {
            break;
        }
        let btn = layout::taskbar_btn_rect(i, ty);
        let is_top = i + 1 == snap.windows.len();
        let is_min = aw.state == WindowState::Minimized;
        let hover = snap.hover == Some(HoverTarget::TaskbarButton(WindowId(aw.id)));
        // Keyboard focus lights the button under the ring — but distinctly
        // from pointer hover: the focused fill is the accent_light blue,
        // while hover keeps the indigo `th.hover`. Users can tell ring-focus
        // from mouse-hover at a glance, and a focused minimized button still
        // lights (before, `is_min` kept it dark). When both apply, focus
        // wins — the ring is the active mode.
        let focused = snap.focused == Some(HoverTarget::TaskbarButton(WindowId(aw.id)));
        // Holding the button over this entry darkens it (the pressed state
        // only applies to the surface under the pointer, like the window
        // control buttons).
        let pressed = hover && snap.mouse_down;

        let bg = window_button_fill(focused, hover, pressed, is_min, is_top, th);
        canvas.draw_rounded_rect(btn.x as u32, btn.y as u32, btn.w, btn.h, 6, bg);
        if is_top && !is_min {
            canvas.draw_line_h(btn.x as u32 + 10, ty + TASKBAR_H - 3, 100, th.accent);
        }
        let display = layout::trunc(&aw.title, layout::TASKBAR_TITLE_MAX);
        // Both lit fills carry white text: the hover indigo gives on_accent
        // 5.13:1, and the focused accent_light blue gives ~3.42:1 (the same
        // accent_light + white pairing libsarga's Button widget already
        // uses; above the 3:1 UI-component floor, below the 4.5:1 AA the
        // indigo gets — a deliberate tradeoff for the distinct focus hue,
        // flagged for the light-theme audit). Pressed (light gray in light
        // mode) keeps th.text.
        let text_c = if pressed {
            th.text
        } else if hover || focused {
            th.on_accent
        } else if is_top {
            th.text
        } else {
            th.text_secondary
        };
        canvas.draw_string(btn.x as u32 + 8, btn.y as u32 + 6, display, text_c, 0);
    }

    if overflow {
        let ox = layout::taskbar_overflow_x();
        canvas.draw_rounded_rect(
            ox,
            ty + 4,
            layout::TASKBAR_OVERFLOW_W,
            TASKBAR_H - 8,
            6,
            th.bg_surface,
        );
        canvas.draw_string(ox + 6, ty + 10, "...", th.text_secondary, 0);
    }

    let tray_entries = snap.tray;
    let tray_len = tray_entries.len() as u32;
    let panel = layout::tray_panel_rect(ty, snap.screen_w, tray_len);
    canvas.draw_rounded_rect(
        panel.x as u32,
        panel.y as u32,
        panel.w,
        panel.h,
        6,
        th.bg_elevated,
    );
    for (i, entry) in tray_entries.iter().enumerate() {
        let r = layout::tray_entry_rect(i, ty, snap.screen_w, tray_len);
        // The same union as the window chrome controls: pressed (hover
        // while held) > the hover/focused light > the resting surface.
        // The focused (keyboard) state lights like hover — pressed stays
        // pointer-only, exactly like the chrome controls.
        let tray_bg = match window_button_face(
            snap.hover == Some(HoverTarget::Tray(i)),
            snap.focused == Some(HoverTarget::Tray(i)),
            snap.mouse_down,
        ) {
            WindowButtonFace::Pressed => th.pressed,
            WindowButtonFace::Focused => th.accent_light,
            WindowButtonFace::Hover => th.hover,
            WindowButtonFace::Base => th.bg_surface,
        };
        // The lit fills (hover indigo and focused accent_light blue) carry
        // the white on_accent icon, exactly like the taskbar buttons — the
        // secondary gray would vanish on them. Pressed (light gray) keeps
        // the secondary text.
        let tray_lit =
            snap.hover == Some(HoverTarget::Tray(i)) || snap.focused == Some(HoverTarget::Tray(i));
        canvas.draw_rounded_rect(r.x as u32, r.y as u32, r.w, r.h, 4, tray_bg);
        // Icon y derives from the panel rect (ty + 4 + 5), not the raw
        // taskbar top, so per-entry rendering stays anchored to the panel
        // geometry the tree and hit-testing share.
        canvas.draw_char(
            r.x as u32 + 6,
            panel.y as u32 + 5,
            entry.icon,
            if tray_lit {
                th.on_accent
            } else {
                th.text_secondary
            },
            0,
        );
    }
    canvas.draw_string(
        panel.x as u32 + layout::tray_entries_w(tray_len) + layout::TRAY_CLOCK_GAP,
        ty + 10,
        clock_str,
        th.text,
        0,
    );
}
