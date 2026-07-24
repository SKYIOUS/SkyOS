#![allow(dead_code)]

use crate::core::desktop::Desktop;
use crate::ipc::ApplicationId;

/// Desktop API v1.0
pub(crate) fn launch(desktop: &mut Desktop, _app: ApplicationId, path: &str, title: &str) {
    crate::core::launcher::spawn_app(desktop, path, title);
}

/// Desktop API v1.0
pub(crate) fn launch_at(
    desktop: &mut Desktop,
    _app: ApplicationId,
    path: &str,
    title: &str,
    x: i32,
    y: i32,
    w: u32,
    h: u32,
) {
    crate::core::launcher::spawn_app_at(desktop, path, title, x, y, w, h);
}
