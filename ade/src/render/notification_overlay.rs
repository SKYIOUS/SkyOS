//! Notification overlay — draws notifications in top-right corner.

use crate::core::window::HoverTarget;
use crate::layout;
use crate::render::compositor::Canvas;
use crate::service::notification::Notification;

pub(crate) fn draw_notifications(
    canvas: &mut Canvas,
    notifications: &[Notification],
    hover: Option<HoverTarget>,
    focused: Option<HoverTarget>,
    theme: &libsarga::theme::Theme,
) {
    for (i, n) in notifications
        .iter()
        .take(layout::NOTIF_MAX_VISIBLE)
        .enumerate()
    {
        let r = layout::notification_rect(canvas.w, i);
        let target = HoverTarget::Notification(i);
        // The same union as the window controls, split so the keyboard
        // state is distinct: the focused (ring) row fills with the
        // accent_light blue, the hovered row with the indigo `th.hover` —
        // "blue = ring" holds on the overlay too, and the ring shows where
        // Enter/activation would land.
        let hover_lit = hover == Some(target);
        let focused_lit = focused == Some(target);
        let lit = hover_lit || focused_lit;
        let border_color = if n.urgency >= 2 {
            theme.error
        } else if n.urgency == 0 {
            theme.text_disabled
        } else {
            theme.accent
        };
        let bg_color = if focused_lit {
            theme.accent_light
        } else if hover_lit {
            theme.hover
        } else if n.urgency == 0 {
            theme.bg_surface
        } else {
            theme.bg_elevated
        };
        canvas.draw_rounded_rect(r.x as u32, r.y as u32, r.w, r.h, 8, bg_color);
        canvas.draw_rounded_rect_outline(r.x as u32, r.y as u32, r.w, r.h, 8, border_color);
        // A lit row fills with a dark-indigo/blue fill, so its text is
        // on_accent (white) — theme.text/theme.text_secondary flip dark in
        // the light theme and would vanish on it.
        let title_c = if lit { theme.on_accent } else { theme.text };
        let body_c = if lit {
            theme.on_accent
        } else {
            theme.text_secondary
        };
        canvas.draw_string(r.x as u32 + 8, r.y as u32 + 6, &n.title, title_c, 0);
        let body = if n.body.len() > 36 {
            &n.body[..36]
        } else {
            &n.body
        };
        canvas.draw_string(r.x as u32 + 8, r.y as u32 + 26, body, body_c, 0);
    }
}
