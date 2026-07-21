//! Application launcher — fork + execve + window creation.

use crate::app_db::APPS;
use crate::desktop::Desktop;
use crate::window::{AppWindow, VisualFlags, WindowState};

pub(crate) fn spawn_app(desktop: &mut Desktop, path: &str, title: &str) {
    if path == "/bin/skyfiles" {
        desktop.spawn_explorer();
        return;
    }
    spawn_app_at(
        desktop,
        path,
        title,
        80 + desktop.wm.len() as i32 * 30,
        40 + desktop.wm.len() as i32 * 20,
        520,
        360,
    );
}

pub(crate) fn spawn_app_at(
    desktop: &mut Desktop,
    path: &str,
    title: &str,
    px: i32,
    py: i32,
    pw: u32,
    ph: u32,
) {
    let w = pw;
    let h = ph;
    let x = px;
    let y = py;
    let mut app_win = AppWindow {
        x,
        y,
        w,
        h,
        prev_x: x,
        prev_y: y,
        prev_w: w,
        prev_h: h,
        title: alloc::string::String::from(title),
        content: alloc::vec::Vec::new(),
        scroll: 0,
        pid: None,
        focused: true,
        dragging: false,
        drag_ox: 0,
        drag_oy: 0,
        state: WindowState::Normal,
        prev_state: WindowState::Normal,
        flags: VisualFlags::new(),
        selection: None,
        anim: None,
        always_on_top: false,
        explorer_id: None,
    };
    app_win.content.push(alloc::format!("> {}", path));
    app_win.content.push(alloc::string::String::new());

    if !path.is_empty() {
        match libsarga::process::fork() {
            Ok(0) => {
                let _ = libsarga::process::execve(path, &[path], &[]);
                libsarga::process::exit(1);
            }
            Ok(pid) => {
                app_win.pid = Some(pid);
                let app_idx = APPS.iter().position(|a| a.exec == path).unwrap_or(0);
                desktop.lifecycle.register(pid, app_idx);
                app_win
                    .content
                    .push(alloc::format!("[launched {} pid={}]", title, pid));
            }
            Err(e) => {
                app_win.content.push(alloc::format!("[fork failed: {}]", e));
            }
        }
    }
    let id = desktop.wm.create(app_win);
    if let Some(w) = desktop.wm.lookup_mut(id) {
        w.flags.opacity = 0;
        w.animate_to(w.x, w.y, w.w, w.h);
    }
    desktop.notif.push("App Launched", title, 1, 120);
    desktop.damage.mark_full();
}
