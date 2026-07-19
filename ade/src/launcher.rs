use crate::desktop::Desktop;
use crate::window::{AppWindow, WindowState};

pub(crate) fn spawn_app(desktop: &mut Desktop, path: &str, title: &str) {
    let w = 520u32;
    let h = 360u32;
    let x = 80 + desktop.wm.len() as i32 * 30;
    let y = 40 + desktop.wm.len() as i32 * 20;
    let mut app_win = AppWindow {
        x, y, w, h,
        title: alloc::string::String::from(title),
        content: alloc::vec::Vec::new(),
        scroll: 0, pid: None, focused: true,
        dragging: false, drag_ox: 0, drag_oy: 0,
        state: WindowState::Normal, opacity: 0,
    };
    app_win.content.push(alloc::format!("> {}", path));
    app_win.content.push(alloc::string::String::new());

    if !path.is_empty() {
        match libsarga::process::fork() {
            Ok(0) => { let _ = libsarga::process::execve(path, &[path], &[]); libsarga::process::exit(1); }
            Ok(pid) => {
                app_win.pid = Some(pid);
                app_win.content.push(alloc::format!("[launched {} pid={}]", title, pid));
            }
            Err(e) => { app_win.content.push(alloc::format!("[fork failed: {}]", e)); }
        }
    }
    desktop.wm.push(app_win);
    desktop.dirty = true;
}
