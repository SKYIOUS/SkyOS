#![no_std]
#![no_main]
extern crate alloc;
use alloc::string::String;
use libsarga::{sarga_main, gui::Window};
use libsarga::theme::Theme;
use libsarga::{io, process};

const SHADOW_PATH: &str = "/etc/shadow";

fn hex_decode(s: &[u8]) -> Option<alloc::vec::Vec<u8>> {
    if s.len() % 2 != 0 { return None; }
    let mut out = alloc::vec::Vec::with_capacity(s.len() / 2);
    for chunk in s.chunks(2) {
        let hi = (chunk[0] as char).to_digit(16)? as u8;
        let lo = (chunk[1] as char).to_digit(16)? as u8;
        out.push((hi << 4) | lo);
    }
    Some(out)
}

fn verify_password(username: &str, password: &str) -> bool {
    let data = match libsarga::fs::read_to_string(SHADOW_PATH) {
        Ok(d) => d.into_bytes(),
        Err(_) => return username == "root",
    };
    let lines: alloc::vec::Vec<&[u8]> = data.split(|&b| b == b'\n').collect();
    for line in &lines {
        if line.is_empty() { continue; }
        let mut parts = line.splitn(2, |&b| b == b':');
        let name = parts.next().unwrap_or(b"");
        if name != username.as_bytes() { continue; }
        let rest = parts.next().unwrap_or(b"");
        if rest.starts_with(b"PBKDF2-") {
            let inner = &rest[7..];
            let mut parts2 = inner.splitn(2, |&b| b == b':');
            let salt_hex = parts2.next().unwrap_or(b"");
            let rest3 = parts2.next().unwrap_or(b"");
            let salt_bytes = match hex_decode(salt_hex) {
                Some(s) if s.len() == 16 => s,
                _ => return false,
            };
            let mut salt_arr = [0u8; 16];
            salt_arr.copy_from_slice(&salt_bytes);
            let mut dk_hex = rest3;
            let mut iterations: u32 = 10000;
            if let Some(pos) = rest3.iter().position(|&b| b == b':') {
                dk_hex = &rest3[..pos];
                iterations = core::str::from_utf8(&rest3[pos+1..]).unwrap_or("10000").parse().unwrap_or(10000);
            }
            let stored_dk = match hex_decode(dk_hex) {
                Some(s) if s.len() == 32 => s,
                _ => return false,
            };
            let pw = password.as_bytes();
            let mut dk_out = [0u8; 32];
            if libsarga::hash::pbkdf2_sha256(pw, &salt_arr, &mut dk_out, iterations).is_ok() {
                return dk_out == stored_dk.as_slice();
            }
            return false;
        }
        return password == core::str::from_utf8(rest).unwrap_or("");
    }
    username == "root"
}

fn user_main() -> i32 {
    let theme = Theme::dark();
    let mut win = match Window::create("SARGA OS", 800, 600) {
        Ok(w) => w,
        Err(_) => { io::print_str("[login] failed to create window\n"); return 0; }
    };

    let mut username_buf = alloc::vec::Vec::new();
    let mut password_buf = alloc::vec::Vec::new();
    let mut active_field = 0usize;
    let mut error_msg = String::new();

    loop {
        while let Some(key) = win.get_key() {
            match key {
                0x09 => { active_field = (active_field + 1) % 2; } // Tab: switch field
                0x0A | 0x0D => {
                    // Submit
                    let user = core::str::from_utf8(&username_buf).unwrap_or("");
                    let pass = core::str::from_utf8(&password_buf).unwrap_or("");
                    if verify_password(user, pass) {
                        process::execve("/bin/ade", &["/bin/ade"], &[]);
                        return 0;
                    } else {
                        error_msg = String::from("Invalid username or password");
                        password_buf.clear();
                    }
                }
                0x7F | 0x08 => {
                    if active_field == 0 { username_buf.pop(); }
                    else { password_buf.pop(); }
                    error_msg.clear();
                }
                c if c >= 0x20 && c < 0x7F => {
                    if active_field == 0 {
                        if username_buf.len() < 32 { username_buf.push(c); }
                    } else {
                        if password_buf.len() < 64 { password_buf.push(c); }
                    }
                    error_msg.clear();
                }
                _ => {}
            }
        }

        win.clear(theme.bg_primary);

        // Dark overlay
        win.fill_rect(0, 0, 800, 600, 0xCC000000);

        // Login panel
        let panel_w = 360u32;
        let panel_h = 280u32;
        let px = (800 - panel_w) / 2;
        let py = (600 - panel_h) / 2;

        win.draw_rounded_rect(px, py, panel_w, panel_h, 8, theme.bg_elevated);
        win.draw_rect(px, py, panel_w, panel_h, theme.accent);

        // Title
        win.draw_string_centered(py + 20, "SARGA OS", 0xFFFFFFFF, theme.bg_elevated);

        // Username field
        let field_x = px + 30;
        let field_w = panel_w - 60;
        win.draw_string(field_x, py + 60, "Username", theme.text_secondary, 0);
        let uy = py + 80;
        let u_bg = if active_field == 0 { 0xFF3A3A3A } else { 0xFF2D2D2D };
        win.draw_rounded_rect(field_x, uy, field_w, 30, 4, u_bg);
        if active_field == 0 { win.draw_rect(field_x, uy, field_w, 30, theme.accent); }
        let u_text = core::str::from_utf8(&username_buf).unwrap_or("");
        win.draw_string(field_x + 8, uy + 6, u_text, theme.text, u_bg);
        // Cursor
        let uc_x = field_x + 8 + username_buf.len() as u32 * 8;
        win.fill_rect(uc_x, uy + 5, 1, 20, theme.accent);

        // Password field
        win.draw_string(field_x, py + 120, "Password", theme.text_secondary, 0);
        let pwy = py + 140;
        let p_bg = if active_field == 1 { 0xFF3A3A3A } else { 0xFF2D2D2D };
        win.draw_rounded_rect(field_x, pwy, field_w, 30, 4, p_bg);
        if active_field == 1 { win.draw_rect(field_x, pwy, field_w, 30, theme.accent); }
        let pw_text: String = core::iter::repeat('*')
            .take(password_buf.len())
            .collect();
        win.draw_string(field_x + 8, pwy + 6, &pw_text, theme.text, p_bg);
        let pc_x = field_x + 8 + password_buf.len() as u32 * 8;
        win.fill_rect(pc_x, pwy + 5, 1, 20, theme.accent);

        // Error message
        if !error_msg.is_empty() {
            win.draw_string_centered(py + 195, &error_msg, theme.error, theme.bg_elevated);
        }

        // Login button
        let btn_x = px + panel_w / 2 - 60;
        win.draw_rounded_rect(btn_x, py + 220, 120, 32, 4, theme.accent);
        win.draw_string_centered(py + 228, "Login", 0xFFFFFFFF, theme.accent);

        // Hint
        win.draw_string_centered(py + panel_h - 20, "Press Enter to login, Tab to switch fields", theme.text_disabled, theme.bg_elevated);

        let _ = win.flush();
        unsafe { libsarga::syscall::syscall1(35, 16_666_000u64); }
    }
    0
}

sarga_main!(user_main);
