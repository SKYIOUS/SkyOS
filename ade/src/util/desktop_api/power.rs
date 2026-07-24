#![allow(dead_code)]

use crate::core::desktop::Desktop;
use crate::ipc::permission::PERM_POWER;
use crate::ipc::ApplicationId;

/// Desktop API v1.0
pub(crate) fn battery_available(desktop: &Desktop, _app: ApplicationId) -> bool {
    desktop.services.power.battery_available
}

/// Desktop API v1.0
pub(crate) fn battery_percentage(desktop: &Desktop, _app: ApplicationId) -> u8 {
    desktop.services.power.battery_percentage
}

/// Desktop API v1.0
pub(crate) fn ac_connected(desktop: &Desktop, _app: ApplicationId) -> bool {
    desktop.services.power.ac_connected
}

/// Desktop API v1.0
pub(crate) fn request_suspend(desktop: &mut Desktop, app: ApplicationId) {
    if !desktop.permission_check(app, PERM_POWER) {
        return;
    }
    desktop.services.power.request_suspend();
}
