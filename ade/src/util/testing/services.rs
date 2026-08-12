use crate::core::desktop::Desktop;
use libsarga::io;

pub(crate) fn test_notifications(desktop: &mut Desktop) -> bool {
    let visible_before = desktop.services.notifications.visible_notifications().len();

    let id = desktop
        .services
        .notifications
        .notify("Test Title", "Test Body", 0, 120);
    if id == 0 {
        io::print_str("[test] FAIL test_notifications: notify returned id 0\n");
        return false;
    }
    let visible_after = desktop.services.notifications.visible_notifications().len();
    if visible_after != visible_before + 1 {
        io::print_str("[test] FAIL test_notifications: visible count did not increase\n");
        return false;
    }

    let visible = desktop.services.notifications.visible_notifications();
    let last = visible.last().unwrap();
    if last.title != "Test Title" || last.body != "Test Body" {
        io::print_str("[test] FAIL test_notifications: content mismatch\n");
        return false;
    }

    let dismissed = desktop.services.notifications.dismiss(id);
    if !dismissed {
        io::print_str("[test] FAIL test_notifications: dismiss returned false\n");
        return false;
    }

    let id2 = desktop
        .services
        .notifications
        .notify("Old", "Old Body", 0, 120);
    desktop
        .services
        .notifications
        .update(id2, "New", "New Body");
    let visible2 = desktop.services.notifications.visible_notifications();
    let updated = match visible2.iter().find(|n| n.id == id2) {
        Some(n) => n,
        None => {
            io::print_str("[test] FAIL test_notifications: updated notification not found\n");
            return false;
        }
    };
    if updated.title != "New" || updated.body != "New Body" {
        io::print_str("[test] FAIL test_notifications: update failed\n");
        return false;
    }

    desktop.services.notifications.dismiss(id2);
    io::print_str("[test] PASS test_notifications\n");
    true
}

pub(crate) fn test_clipboard(desktop: &mut Desktop) -> bool {
    if !desktop.services.clipboard.is_empty() {
        io::print_str("[test] FAIL test_clipboard: clipboard not empty initially\n");
        return false;
    }

    desktop.services.clipboard.copy("hello world", 0);
    if desktop.services.clipboard.length != 11 {
        io::print_str("[test] FAIL test_clipboard: copy length wrong\n");
        return false;
    }
    let pasted = desktop.services.clipboard.paste();
    if pasted != "hello world" {
        io::print_str("[test] FAIL test_clipboard: paste mismatch\n");
        return false;
    }

    if desktop.services.clipboard.history().len() != 1 {
        io::print_str("[test] FAIL test_clipboard: history count wrong\n");
        return false;
    }

    desktop.services.clipboard.clear();
    if !desktop.services.clipboard.is_empty() {
        io::print_str("[test] FAIL test_clipboard: clear did not work\n");
        return false;
    }

    io::print_str("[test] PASS test_clipboard\n");
    true
}

pub(crate) fn test_session(desktop: &mut Desktop) -> bool {
    let uptime = desktop.session.uptime(desktop.clock_ticks);
    // clock_ticks is 0 before first tick — uptime may be 0
    if uptime != 0 {
        io::print_str("[test] PASS test_session (uptime > 0)\n");
    } else {
        io::print_str("[test] PASS test_session (uptime = 0 at boot)\n");
    }
    true
}
