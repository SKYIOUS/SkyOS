//! Application launcher — fork + execve + window creation.

use crate::util::app_registry::AppInfo;
use crate::core::desktop::Desktop;
use crate::core::window::{AppWindow, VisualFlags, WindowState};

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

pub(crate) fn spawn_app_from_registry(desktop: &mut Desktop, app: &AppInfo) {
    let path = app.executable;
    let title = app.name;
    if path == "/bin/skyfiles" {
        desktop.spawn_explorer();
        return;
    }
    if path.is_empty() {
        // About dialog or similar — handled in launch_app
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
    desktop.app_reg.record_launch(app.id);
    desktop
        .services
        .session
        .record_app_launch(app.id.0 as u64);
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
        id: 0,
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
        closing: false,
        anim_opacity: 0,
        always_on_top: false,
        explorer_id: None,
    };
    app_win.content.push(alloc::format!("> {}", path));
    app_win.content.push(alloc::string::String::new());

    if !path.is_empty() {
        let ipc_pair = libsarga::net::socketpair(
            libsarga::net::SocketDomain::Unix as u64,
            libsarga::net::SocketType::Stream as u64,
            0,
        )
        .ok();
        match libsarga::process::fork() {
            Ok(0) => {
                match ipc_pair {
                    Some((server_fd, client_fd)) => {
                        let _ = libsarga::io::close(server_fd);
                        let fd_arg = alloc::format!("{}", client_fd);
                        let argv = [path, "--ipc-fd", fd_arg.as_str()];
                        let _ = libsarga::process::execve(path, &argv, &[]);
                    }
                    None => {
                        let _ = libsarga::process::execve(path, &[path], &[]);
                    }
                }
                libsarga::process::exit(1);
            }
            Ok(pid) => {
                app_win.pid = Some(pid);
                let app_idx = desktop
                    .app_reg
                    .find_by_exec(path)
                    .map(|id| id.0)
                    .unwrap_or(0);
                desktop.lifecycle.register(pid, app_idx);
                desktop.permissions.register(pid, crate::sec::perms::default_grant());
                desktop.lifecycle.mark_running(pid);
                if let Some((server_fd, client_fd)) = ipc_pair {
                    let _ = libsarga::io::close(client_fd);
                    desktop.ipc_transport.register(pid, server_fd);
                }
                app_win
                    .content
                    .push(alloc::format!("[launched {} pid={}]", title, pid));
            }
            Err(e) => {
                if let Some((server_fd, client_fd)) = ipc_pair {
                    let _ = libsarga::io::close(server_fd);
                    let _ = libsarga::io::close(client_fd);
                }
                app_win.content.push(alloc::format!("[fork failed: {}]", e));
            }
        }
    }
    let id = desktop.wm.create(app_win);
    if let Some(w) = desktop.wm.lookup_mut(id) {
        w.flags.opacity = 0;
        w.animate_to(w.x, w.y, w.w, w.h);
    }
    desktop.services.notify("App Launched", title, 1, 120, desktop.clock_ticks);
    desktop.damage.mark_full();
}
