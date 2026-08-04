#![allow(dead_code)]

use crate::core::desktop::Desktop;
use crate::ipc::permission::PERM_SETTINGS;
use crate::ipc::ApplicationId;

/// Desktop API v1.0
pub(crate) fn current(desktop: &Desktop, _app: ApplicationId) -> &libsarga::theme::Theme {
    desktop.theme_svc.current()
}

/// Desktop API v1.0
pub(crate) fn set_dark(desktop: &mut Desktop, app: ApplicationId) {
    if !desktop.permission_check(app, PERM_SETTINGS) {
        return;
    }
    desktop.theme_svc.set(libsarga::theme::Theme::dark());
    desktop.damage.mark_full();
}

/// Desktop API v1.0
pub(crate) fn set_light(desktop: &mut Desktop, app: ApplicationId) {
    if !desktop.permission_check(app, PERM_SETTINGS) {
        return;
    }
    desktop.theme_svc.set(libsarga::theme::Theme::light());
    desktop.damage.mark_full();
}
