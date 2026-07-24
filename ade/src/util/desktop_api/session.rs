#![allow(dead_code)]

use crate::core::desktop::Desktop;
use crate::ipc::permission::PERM_POWER;
use crate::ipc::ApplicationId;

/// Desktop API v1.0
pub(crate) fn uptime(desktop: &Desktop, _app: ApplicationId) -> u64 {
    desktop.services.session.uptime(desktop.clock_ticks)
}

/// Desktop API v1.0
pub(crate) fn shutdown(desktop: &mut Desktop, app: ApplicationId) {
    if !desktop.permission_check(app, PERM_POWER) {
        return;
    }
    desktop.services.session.request_shutdown();
}

/// Desktop API v1.0
pub(crate) fn restart(desktop: &mut Desktop, app: ApplicationId) {
    if !desktop.permission_check(app, PERM_POWER) {
        return;
    }
    desktop.services.session.request_restart();
}

/// Desktop API v1.0
pub(crate) fn logout(desktop: &mut Desktop, app: ApplicationId) {
    if !desktop.permission_check(app, PERM_POWER) {
        return;
    }
    desktop.services.session.request_logout();
}
