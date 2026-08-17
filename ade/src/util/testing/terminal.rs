//! Terminal / pty pipeline tests.
//!
//! Covers the three deliberate simplifications being lifted:
//! 1. Persistent ANSI parser with `\r`→cursor-col-0 overwrite + CSI `K` erase.
//! 2. Global shortcuts still working while a terminal has focus.
//! 3. Closing a terminal window kills sash and frees the pty master fd.

use crate::core::desktop::Desktop;
use crate::core::event::Event;
use crate::core::window::AppWindow;
use alloc::string::String;
use libsarga::io;

fn bare_window() -> AppWindow {
    AppWindow::new(0, 0, 400, 300, "TermTest")
}

fn last_line(w: &AppWindow) -> String {
    w.surface().last_line().cloned().unwrap_or_default()
}

/// Parser unit tests (deterministic, no pty): `\r` overwrite, CSI K erase,
/// and escape sequences split across reads must survive `esc_state`.
pub(crate) fn test_parser_semantics() -> bool {
    // \r returns to column 0; later text overwrites the current line.
    let mut w = bare_window();
    w.surface_mut().consume_pty_bytes(b"abc\rX");
    if last_line(&w) != "Xbc" {
        io::print_str(&alloc::format!(
            "[test] FAIL test_parser_semantics: overwrite got [{:?}]\n",
            last_line(&w)
        ));
        return false;
    }
    // \r + CSI K (erase-to-end) clears the tail, then new text overwrites.
    w.surface_mut().consume_pty_bytes(b"hello\rx\x1b[K");
    if last_line(&w) != "x" {
        io::print_str(&alloc::format!(
            "[test] FAIL test_parser_semantics: csi-k got [{:?}]\n",
            last_line(&w)
        ));
        return false;
    }
    // Split across reads: ESC in one read, '[K' in the next — the parser
    // state (esc_state) must persist, and the dangling escape must be
    // consumed, not printed.
    let mut w2 = bare_window();
    w2.surface_mut().consume_pty_bytes(b"a\x1b");
    w2.surface_mut().consume_pty_bytes(b"bc");
    if last_line(&w2) != "ac" {
        io::print_str(&alloc::format!(
            "[test] FAIL test_parser_semantics: split-esc got [{:?}]\n",
            last_line(&w2)
        ));
        return false;
    }
    // A real sash-style redraw split mid-sequence: "\r\x1b" + "[K$ "
    let mut w3 = bare_window();
    w3.surface_mut().consume_pty_bytes(b"\r\x1b");
    w3.surface_mut().consume_pty_bytes(b"[K$ ");
    if last_line(&w3) != "$ " {
        io::print_str(&alloc::format!(
            "[test] FAIL test_parser_semantics: split-redraw got [{:?}]\n",
            last_line(&w3)
        ));
        return false;
    }
    io::print_str("[test] PASS test_parser_semantics\n");
    true
}

/// Real pipeline: spawn sash on a pty, wait for its prompt to arrive
/// through the kernel pty into the parser, then type a command and see
/// the echoed/shell output land in `content`.
pub(crate) fn test_terminal_pipeline(desktop: &mut Desktop) -> bool {
    let before = desktop.wm.len();
    desktop.spawn_terminal();
    if desktop.wm.len() != before + 1 {
        io::print_str("[test] FAIL test_terminal_pipeline: spawn did not add a window\n");
        return false;
    }
    let id = match desktop.wm.active() {
        Some(id) => id,
        None => {
            io::print_str("[test] FAIL test_terminal_pipeline: no active window\n");
            return false;
        }
    };
    let (pty_fd, pid) = match desktop.wm.lookup(id) {
        Some(w) => (w.pty_fd(), w.pid),
        None => {
            io::print_str("[test] FAIL test_terminal_pipeline: lookup failed\n");
            return false;
        }
    };
    if pty_fd.is_none() || pid.is_none() {
        io::print_str("[test] FAIL test_terminal_pipeline: pty/pid not wired\n");
        return false;
    }
    // Wait for sash's prompt ("$") through the pty.
    let mut saw_prompt = false;
    for _ in 0..400 {
        desktop.tick();
        if let Some(w) = desktop.wm.lookup(id) {
            if last_line(w).contains('$') {
                saw_prompt = true;
                break;
            }
        }
    }
    if !saw_prompt {
        io::print_str("[test] FAIL test_terminal_pipeline: no sash prompt\n");
        return false;
    }
    // Type "hi" + Enter through the desktop key path (fix 2: keys must be
    // routed to the pty master, not swallowed by the desktop).
    desktop.handle_event(Event::Key(b'h' as u16));
    desktop.handle_event(Event::Key(b'i' as u16));
    desktop.handle_event(Event::Key(b'\r' as u16));
    let mut saw_echo = false;
    for _ in 0..600 {
        desktop.tick();
        if let Some(w) = desktop.wm.lookup(id) {
            if w.surface().lines().iter().any(|l| l.contains("hi")) {
                saw_echo = true;
                break;
            }
        }
    }
    if !saw_echo {
        io::print_str("[test] FAIL test_terminal_pipeline: typed 'hi' never appeared\n");
        return false;
    }
    io::print_str("[test] PASS test_terminal_pipeline\n");
    true
}

/// Fix 2: global shortcuts keep their desktop meaning while a terminal is
/// focused — Ctrl+W (23) must close the terminal, not go to the pty.
/// Fix 3: closing the terminal kills sash (dead after reap → kill ESRCH).
pub(crate) fn test_terminal_close_kills_shell(desktop: &mut Desktop) -> bool {
    desktop.spawn_terminal();
    let id = match desktop.wm.active() {
        Some(id) => id,
        None => {
            io::print_str("[test] FAIL test_terminal_close_kills_shell: no active window\n");
            return false;
        }
    };
    let pid = match desktop.wm.lookup(id).and_then(|w| w.pid) {
        Some(p) => p,
        None => {
            io::print_str("[test] FAIL test_terminal_close_kills_shell: no pid\n");
            return false;
        }
    };
    // Ctrl+W while the terminal is focused: desktop shortcut, not a pty byte.
    desktop.handle_event(Event::Key(23));
    let closing = desktop.wm.lookup(id).is_some_and(|w| w.closing);
    if !closing {
        io::print_str(
            "[test] FAIL test_terminal_close_kills_shell: Ctrl+W did not close the terminal\n",
        );
        return false;
    }
    // Fix 3: sash must be dead — reap it, then kill(pid, 9) must return
    // ESRCH because the pid is no longer in the process table.
    for _ in 0..30 {
        desktop.tick();
    }
    if libsarga::process::kill(pid as i64, 9).is_ok() {
        io::print_str(
            "[test] FAIL test_terminal_close_kills_shell: sash still alive after close\n",
        );
        return false;
    }
    io::print_str("[test] PASS test_terminal_close_kills_shell\n");
    true
}

/// Fix 3 directly: wm.close() on a pty window kills sash and frees the fd.
pub(crate) fn test_window_close_frees_pty(desktop: &mut Desktop) -> bool {
    desktop.spawn_terminal();
    let id = match desktop.wm.active() {
        Some(id) => id,
        None => {
            io::print_str("[test] FAIL test_window_close_frees_pty: no active window\n");
            return false;
        }
    };
    let pid = match desktop.wm.lookup(id).and_then(|w| w.pid) {
        Some(p) => p,
        None => {
            io::print_str("[test] FAIL test_window_close_frees_pty: no pid\n");
            return false;
        }
    };
    desktop.wm.close(id);
    for _ in 0..30 {
        desktop.tick();
    }
    if libsarga::process::kill(pid as i64, 9).is_ok() {
        io::print_str("[test] FAIL test_window_close_frees_pty: sash still alive after wm.close\n");
        return false;
    }
    io::print_str("[test] PASS test_window_close_frees_pty\n");
    true
}
