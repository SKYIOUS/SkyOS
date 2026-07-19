//! Session manager — save/restore desktop state, crash-safe persistence.
#![allow(dead_code)]

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use crate::desktop::Desktop;
use crate::window::WindowState;

const SESSION_PATH: &str = "/tmp/skyos.session";
const SESSION_TMP: &str = "/tmp/skyos.session.tmp";

pub(crate) struct SessionManager;

impl SessionManager {
    pub fn save(desktop: &Desktop) {
        let mut lines = Vec::new();
        lines.push(format!(
            "theme:{}",
            if desktop.settings.theme_dark {
                "dark"
            } else {
                "light"
            }
        ));
        lines.push(format!("screen:{}x{}", desktop.screen_w, desktop.screen_h));
        for w in desktop.wm.iter() {
            let state = match w.state {
                WindowState::Normal => "normal",
                WindowState::Minimized => "minimized",
                WindowState::Maximized => "maximized",
                WindowState::Fullscreen => "fullscreen",
            };
            lines.push(format!(
                "win:{}:{}:{}:{}:{}:{}",
                w.title, w.x, w.y, w.w, w.h, state
            ));
        }
        let data = lines.join("\n");
        // crash-safe: write tmp then rename
        if let Ok(fd) = libsarga::io::open(SESSION_TMP, 0x241) {
            let _ = libsarga::io::write(fd, data.as_bytes());
            let _ = libsarga::io::close(fd);
            let _ = libsarga::posix::rename(SESSION_TMP, SESSION_PATH);
        }
    }

    pub fn restore(desktop: &mut Desktop, lines: &[&str]) {
        for line in lines {
            if let Some(rest) = line.strip_prefix("theme:") {
                desktop.settings.theme_dark = rest == "dark";
                if desktop.settings.theme_dark {
                    desktop.theme_svc.set(libsarga::theme::Theme::dark());
                } else {
                    desktop.theme_svc.set(libsarga::theme::Theme::light());
                }
            }
            if let Some(rest) = line.strip_prefix("win:") {
                let parts: Vec<&str> = rest.split(':').collect();
                if parts.len() >= 6 {
                    let x = parts[1].parse().unwrap_or(0);
                    let y = parts[2].parse().unwrap_or(0);
                    let w = parts[3].parse().unwrap_or(520u32);
                    let h = parts[4].parse().unwrap_or(360u32);
                    let _state = parts[5];
                    desktop.spawn_app_ex("", parts[0], x, y, w, h);
                }
            }
        }
    }

    pub fn load_lines() -> Vec<String> {
        let mut result = Vec::new();
        let fd = match libsarga::io::open(SESSION_PATH, 0) {
            Ok(f) => f,
            _ => return result,
        };
        let mut buf = [0u8; 4096];
        let n = libsarga::io::read(fd, &mut buf).unwrap_or(0);
        let _ = libsarga::io::close(fd);
        if n > 0 {
            let s = core::str::from_utf8(&buf[..n as usize]).unwrap_or("");
            for line in s.lines() {
                result.push(String::from(line));
            }
        }
        result
    }
}
