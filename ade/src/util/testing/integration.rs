use crate::core::desktop::Desktop;
use crate::core::window::AppWindow;
use libsarga::io;

pub(crate) fn test_full_flow(desktop: &mut Desktop) -> bool {
    let before = desktop.wm.len();

    let win = AppWindow::new(50, 50, 400, 300, "FullFlow");
    let id = desktop.wm.create(win);
    if desktop.wm.len() != before + 1 {
        io::print_str("[test] FAIL test_full_flow: create failed\n");
        return false;
    }

    let win2 = AppWindow::new(200, 200, 300, 200, "FullFlow2");
    let id2 = desktop.wm.create(win2);
    if desktop.wm.len() != before + 2 {
        io::print_str("[test] FAIL test_full_flow: create second failed\n");
        return false;
    }

    desktop.wm.bring_to_front(id);

    desktop.wm.close(id);
    desktop.wm.close(id2);
    // Drain the animated close before counting (close() only removes the
    // window once process_closing runs during tick).
    for _ in 0..60 {
        desktop.tick();
    }
    if desktop.wm.len() != before {
        io::print_str("[test] FAIL test_full_flow: close did not restore count\n");
        return false;
    }

    io::print_str("[test] PASS test_full_flow\n");
    true
}
