#![allow(dead_code)]

use crate::core::desktop::Desktop;
use crate::ipc::permission::PERM_SETTINGS;
use crate::ipc::ApplicationId;

/// Desktop API v1.0
pub(crate) fn open(desktop: &mut Desktop, app: ApplicationId) {
    if !desktop.permission_check(app, PERM_SETTINGS) {
        return;
    }
    desktop.settings_app.open = true;
}

/// Desktop API v1.0
pub(crate) fn close(desktop: &mut Desktop, app: ApplicationId) {
    if !desktop.permission_check(app, PERM_SETTINGS) {
        return;
    }
    desktop.settings_app.open = false;
}

/// Desktop API v1.0
pub(crate) fn is_open(desktop: &Desktop, _app: ApplicationId) -> bool {
    desktop.settings_app.open
}
