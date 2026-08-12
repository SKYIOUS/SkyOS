//! Notification overlay — draws notifications in top-right corner.

use crate::core::window::HoverTarget;
use crate::layout;
use crate::render::compositor::Canvas;
use crate::service::notification::Notification;

pub(crate) fn draw_notifications(
    canvas: &mut Canvas,
    notifications: &[Notification],
    hover: Option<HoverTarget>,
    theme: &libsarga::theme::Theme,
) {
    for (i, n) in notifications
        .iter()
        .take(layout::NOTIF_MAX_VISIBLE)
        .enumerate()
    {
        let r = layout::notification_rect(canvas.w, i);
        let hovered = hover == Some(HoverTarget::Notification(i));
        let border_color = if n.urgency >= 2 {
            theme.error
        } else if n.urgency == 0 {
            theme.text_disabled
        } else {
            theme.accent
        };
        let bg_color = if hovered {
            theme.hover
        } else if n.urgency == 0 {
            theme.bg_surface
        } else {
            theme.bg_elevated
        };
        canvas.draw_rounded_rect(r.x as u32, r.y as u32, r.w, r.h, 8, bg_color);
        canvas.draw_rounded_rect_outline(r.x as u32, r.y as u32, r.w, r.h, 8, border_color);
        // A hovered row fills with the theme-invariant indigo, so its text
        // is on_accent (white) — theme.text/theme.text_secondary flip dark
        // in the light theme and would vanish on it.
        let title_c = if hovered { theme.on_accent } else { theme.text };
        let body_c = if hovered {
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
