use crate::core::desktop::Desktop;
use crate::ipc::permission::PERM_NOTIFICATIONS;
use crate::ipc::ApplicationId;

/// Desktop API v1.0
pub(crate) fn notify(
    desktop: &mut Desktop,
    app: ApplicationId,
    title: &str,
    body: &str,
    urgency: u8,
    timeout: u32,
) {
    if !desktop.permission_check(app, PERM_NOTIFICATIONS) {
        return;
    }
    desktop
        .services
        .notify(title, body, urgency, timeout, desktop.clock_ticks);
}

/// Desktop API v1.0
pub(crate) fn dismiss_all(desktop: &mut Desktop, _app: ApplicationId) {
    desktop.services.notifications.dismiss_all();
}
