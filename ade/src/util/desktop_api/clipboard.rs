use crate::core::desktop::Desktop;
use crate::ipc::permission::PERM_CLIPBOARD;
use crate::ipc::ApplicationId;

/// Desktop API v1.0
///
/// The canonical clipboard store is the kernel's (`SYS_CLIPBOARD=125`), so a
/// yank in sash and a copy in an ade app share one buffer. The userspace
/// `ClipboardManager` is kept strictly as the history overlay for the
/// clipboard panel: `copy` records into it, but `paste` reads the kernel.
pub(crate) fn copy(desktop: &mut Desktop, app: ApplicationId, text: &[u8]) {
    if !desktop.permission_check(app, PERM_CLIPBOARD) {
        return;
    }
    // Kernel write is best-effort: the wrapper returns () and syscall 125 never
    // errnos today, so a failure would leave a history entry the kernel lacks.
    libsarga::io::clipboard_write(text);
    let s = core::str::from_utf8(text).unwrap_or("");
    desktop.services.clipboard.copy(s, desktop.clock_ticks);
}

/// Desktop API v1.0
pub(crate) fn paste(desktop: &Desktop, app: ApplicationId) -> Option<alloc::vec::Vec<u8>> {
    if !desktop.permission_check(app, PERM_CLIPBOARD) {
        return None;
    }
    let len = libsarga::io::clipboard_len();
    if len == 0 {
        return None;
    }
    // Kernel clamps reads to min(len, clipboard.len()), so a shrink between the
    // len and read calls is absorbed by the truncate; a clear yields n == 0.
    let mut buf = alloc::vec![0u8; len];
    let n = libsarga::io::clipboard_read(&mut buf);
    buf.truncate(n);
    if buf.is_empty() {
        None
    } else {
        Some(buf)
    }
}
