//! Notification overlay — draws notifications in top-right corner.

use crate::core::geometry::Point;
use crate::render::compositor::Canvas;
use crate::service::notification::Notification;

pub(crate) fn draw_notifications(
    canvas: &mut Canvas,
    notifications: &[Notification],
    mouse: Point,
    theme: &libsarga::theme::Theme,
) {
    let mut ny = 10i32;
    let max_visible = 4;
    for n in notifications.iter().take(max_visible) {
        let border_color = if n.urgency >= 2 {
            theme.error
        } else if n.urgency == 0 {
            theme.text_disabled
        } else {
            theme.accent
        };
        let bg_color = if n.urgency == 0 {
            0xFF1A1A2E
        } else {
            0xFF2D2D2D
        };
        canvas.draw_rounded_rect(canvas.w - 310, ny as u32, 300, 64, 8, bg_color);
        canvas.draw_rounded_rect_outline(canvas.w - 310, ny as u32, 300, 64, 8, border_color);
        canvas.draw_string(canvas.w - 302, ny as u32 + 6, &n.title, 0xFFFFFFFF, 0);
        let body = if n.body.len() > 36 {
            &n.body[..36]
        } else {
            &n.body
        };
        canvas.draw_string(canvas.w - 302, ny as u32 + 26, body, 0xFFB0B0B0, 0);
        ny += 72;
    }
}
