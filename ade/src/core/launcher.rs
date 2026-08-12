//! Application launcher — fork + execve + window creation.
//!
//! Every launch funnels through [`spawn`]; the [`SpawnKind`] discriminates
//! the child setup (pty slave → sash, IPC socketpair → `--ipc-fd`, plain
//! exec for the explorer's detached `/bin/skyfiles`). One registration
//! sequence runs for every forked pid: lifecycle + permissions + running
//! mark, plus the IPC transport when a socketpair exists.

use crate::core::desktop::Desktop;
use crate::core::geometry::Rect;
use crate::core::window::AppWindow;
use crate::util::app_catalog::AppInfo;

#[derive(Clone, Copy)]
enum SpawnKind {
    /// External binary over the socket IPC (gets `--ipc-fd` argv).
    External,
    /// Kernel pty; sash runs on the slave side.
    Terminal,
    /// In-process explorer window; `/bin/skyfiles` forks detached.
    Explorer(u32),
}

/// Cascade geometry for a new floating window: offsets grow with the window
/// count so windows don't stack exactly on top of each other.
fn cascade_geom(desktop: &Desktop) -> Rect {
    Rect::new(
        80 + desktop.wm.len() as i32 * 30,
        40 + desktop.wm.len() as i32 * 20,
        520,
        360,
    )
}

pub(crate) fn spawn_app(desktop: &mut Desktop, path: &str, title: &str) {
    if path == "/bin/skyfiles" {
        spawn_explorer(desktop);
        return;
    }
    if path == "/bin/sash" {
        spawn_terminal(desktop);
        return;
    }
    spawn(
        desktop,
        path,
        title,
        SpawnKind::External,
        cascade_geom(desktop),
    );
}

pub(crate) fn spawn_app_from_registry(desktop: &mut Desktop, app: &AppInfo) {
    let path = app.executable;
    let title = app.name;
    if path == "/bin/skyfiles" {
        spawn_explorer(desktop);
        return;
    }
    if path == "/bin/sash" {
        spawn_terminal(desktop);
        return;
    }
    if path.is_empty() {
        // About dialog or similar — handled in launch_app
        return;
    }
    spawn(
        desktop,
        path,
        title,
        SpawnKind::External,
        cascade_geom(desktop),
    );
    desktop.app_reg.record_launch(app.id);
}

pub(crate) fn spawn_terminal(desktop: &mut Desktop) {
    spawn(
        desktop,
        "/bin/sash",
        "Terminal",
        SpawnKind::Terminal,
        cascade_geom(desktop),
    );
}

/// External binary at an explicit position (used by the launcher selftests).
pub(crate) fn spawn_app_at(
    desktop: &mut Desktop,
    path: &str,
    title: &str,
    px: i32,
    py: i32,
    pw: u32,
    ph: u32,
) {
    spawn(
        desktop,
        path,
        title,
        SpawnKind::External,
        Rect::new(px, py, pw, ph),
    );
}

pub(crate) fn spawn_explorer(desktop: &mut Desktop) {
    let id = desktop.explorers.len() as u32;
    let mut explorer = crate::util::explorer::ExplorerState::new(id, "/home");
    explorer.refresh();
    desktop.explorers.push(explorer);
    spawn(
        desktop,
        "/bin/skyfiles",
        "File Explorer",
        SpawnKind::Explorer(id),
        Rect::new(60, 40, 640, 440),
    );
}

/// The single launch path: build the window, fork, exec the child per
/// `kind`, register the pid, then present the window (fade-in, notify).
fn spawn(desktop: &mut Desktop, path: &str, title: &str, kind: SpawnKind, geo: Rect) {
    let (master, slave) = match kind {
        SpawnKind::Terminal => match libsarga::io::openpty() {
            Ok(v) => v,
            Err(_) => return,
        },
        _ => (-1, -1),
    };
    let ipc_pair = match kind {
        SpawnKind::External if !path.is_empty() => libsarga::net::socketpair(
            libsarga::net::SocketDomain::Unix as u64,
            libsarga::net::SocketType::Stream as u64,
            0,
        )
        .ok(),
        _ => None,
    };

    let mut app_win = AppWindow::new(geo.x, geo.y, geo.w, geo.h, title);
    match kind {
        SpawnKind::External => {
            app_win
                .surface_mut()
                .push_line(alloc::format!("> {}", path));
            app_win
                .surface_mut()
                .push_line(alloc::string::String::new());
        }
        SpawnKind::Terminal | SpawnKind::Explorer(_) => {
            app_win
                .surface_mut()
                .push_line(alloc::string::String::new());
        }
    }
    if let SpawnKind::Explorer(id) = kind {
        app_win.explorer_id = Some(id);
    }

    // Empty path (test scaffolding) or an explorer fork failure still leaves
    // a window behind; only a terminal without a pty aborts the whole spawn.
    if kind_should_fork(kind, path) {
        match libsarga::process::fork() {
            Ok(0) => {
                // Child: set up its world, then exec. Exec failure exits.
                match kind {
                    SpawnKind::Terminal => {
                        // pty slave becomes stdin/stdout/stderr, then sash.
                        let _ = libsarga::io::dup2(slave, 0);
                        let _ = libsarga::io::dup2(slave, 1);
                        let _ = libsarga::io::dup2(slave, 2);
                        let _ = libsarga::io::close(master);
                        let _ = libsarga::io::close(slave);
                        let _ = libsarga::process::execve(path, &[path], &[]);
                    }
                    SpawnKind::External => match ipc_pair {
                        Some((server_fd, client_fd)) => {
                            let _ = libsarga::io::close(server_fd);
                            let fd_arg = alloc::format!("{}", client_fd);
                            let argv = [path, "--ipc-fd", fd_arg.as_str()];
                            let _ = libsarga::process::execve(path, &argv, &[]);
                        }
                        None => {
                            let _ = libsarga::process::execve(path, &[path], &[]);
                        }
                    },
                    SpawnKind::Explorer(_) => {
                        let _ = libsarga::process::execve(path, &[path], &[]);
                    }
                }
                libsarga::process::exit(1);
            }
            Ok(pid) => {
                app_win.pid = Some(pid);
                // One registration sequence for every spawned process.
                desktop.session.lifecycle.register(pid);
                desktop
                    .permissions
                    .register(pid, crate::sec::perms::default_grant());
                desktop.session.lifecycle.mark_running(pid);
                match kind {
                    SpawnKind::Terminal => {
                        let _ = libsarga::io::close(slave);
                        // The seeded surface (first empty line) rides along.
                        app_win.attach_terminal(master);
                    }
                    SpawnKind::External => {
                        if let Some((server_fd, client_fd)) = ipc_pair {
                            let _ = libsarga::io::close(client_fd);
                            desktop.ipc_transport.register(pid, server_fd);
                        }
                        app_win.surface_mut().push_line(alloc::format!(
                            "[launched {} pid={}]",
                            title,
                            pid
                        ));
                    }
                    SpawnKind::Explorer(_) => {}
                }
            }
            Err(e) => {
                match kind {
                    SpawnKind::Terminal => {
                        // No window without a pty; free the fds and bail.
                        let _ = libsarga::io::close(master);
                        let _ = libsarga::io::close(slave);
                        return;
                    }
                    SpawnKind::External => {
                        if let Some((server_fd, client_fd)) = ipc_pair {
                            let _ = libsarga::io::close(server_fd);
                            let _ = libsarga::io::close(client_fd);
                        }
                        app_win
                            .surface_mut()
                            .push_line(alloc::format!("[fork failed: {}]", e));
                    }
                    SpawnKind::Explorer(_) => {}
                }
            }
        }
    }

    let id = desktop.wm.create(app_win);
    if let Some(w) = desktop.wm.lookup_mut(id) {
        w.flags.opacity = 0;
        w.animate_to(w.x, w.y, w.w, w.h);
    }
    desktop
        .services
        .notify("App Launched", title, 1, 120, desktop.clock_ticks);
    desktop.damage.mark_full();
}

/// Whether a child process should be forked at all: every kind except an
/// external app with an empty executable path (test scaffolding) forks.
fn kind_should_fork(kind: SpawnKind, path: &str) -> bool {
    match kind {
        SpawnKind::External => !path.is_empty(),
        SpawnKind::Terminal | SpawnKind::Explorer(_) => true,
    }
}
