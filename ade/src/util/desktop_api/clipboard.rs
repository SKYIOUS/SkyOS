#![allow(dead_code)]

use crate::core::desktop::Desktop;
use crate::ipc::permission::PERM_CLIPBOARD;
use crate::ipc::ApplicationId;

/// Desktop API v1.0
pub(crate) fn copy(desktop: &mut Desktop, app: ApplicationId, text: &[u8]) {
    if !desktop.permission_check(app, PERM_CLIPBOARD) {
        return;
    }
    let s = core::str::from_utf8(text).unwrap_or("");
    desktop.services.clipboard.copy(s, desktop.clock_ticks);
}

/// Desktop API v1.0
pub(crate) fn paste<'a>(desktop: &'a Desktop, app: ApplicationId) -> Option<&'a str> {
    if !desktop.permission_check(app, PERM_CLIPBOARD) {
        return None;
    }
    let s = desktop.services.clipboard.paste();
    if s.is_empty() { None } else { Some(s) }
}
