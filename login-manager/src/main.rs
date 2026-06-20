#![no_std]
#![no_main]
extern crate alloc;
use alloc::string::String;
use libsarga::theme::Theme;
use libsarga::{gui::Window, sarga_main};
use libsarga::{io, process};

const SHADOW_PATH: &str = "/etc/shadow";

fn hex_decode(s: &[u8]) -> Option<alloc::vec::Vec<u8>> {
    if s.len() % 2 != 0 {
        return None;
    }
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
        if line.is_empty() {
            continue;
        }
        let mut parts = line.splitn(2, |&b| b == b':');
        let name = parts.next().unwrap_or(b"");
        if name != username.as_bytes() {
            continue;
        }
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
                iterations = core::str::from_utf8(&rest3[pos + 1..])
                    .unwrap_or("10000")
                    .parse()
                    .unwrap_or(10000);
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
        Err(_) => {
            io::print_str("[login] failed to create window\n");
            return 0;
        }
    };

    let mut username_buf = alloc::vec::Vec::new();
    let mut password_buf = alloc::vec::Vec::new();
    let mut active_field = 0usize;
    let mut error_msg = String::new();

    let mut show_password = false;
    let mut power_menu = false;

    loop {
        let mouse = win.get_mouse();
        let mx = mouse.x;
        let my = mouse.y;
        let m_pressed = mouse.buttons != 0;

        while let Some(key) = win.get_key() {
            match key {
                0x09 => {
                    active_field = (active_field + 1) % 2;
                } // Tab
                0x0A | 0x0D => {
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
                    if active_field == 0 {
                        username_buf.pop();
                    } else {
                        password_buf.pop();
                    }
                    error_msg.clear();
                }
                c if c >= 0x20 && c < 0x7F => {
                    if active_field == 0 {
                        if username_buf.len() < 32 {
                            username_buf.push(c);
                        }
                    } else {
                        if password_buf.len() < 64 {
                            password_buf.push(c);
                        }
                    }
                    error_msg.clear();
                }
                _ => {}
            }
        }

        win.clear(theme.bg_primary);
        win.draw_gradient_rect(0, 0, 800, 600, theme.bg_primary, 0xFF000000, true);

        // Login panel
        let panel_w = 400u32;
        let panel_h = 320u32;
        let px = (800 - panel_w) / 2;
        let py = (600 - panel_h) / 2;

        win.draw_rounded_rect(
            px,
            py,
            panel_w,
            panel_h,
            theme.border_radius,
            theme.bg_surface,
        );
        win.draw_rounded_rect_outline(px, py, panel_w, panel_h, theme.border_radius, theme.border);

        // Logo / Title
        win.draw_gradient_rect(
            px + 10,
            py + 10,
            panel_w - 20,
            40,
            theme.accent,
            theme.accent_dark,
            false,
        );
        win.draw_string_centered(py + 22, "SARGA OS", 0xFFFFFFFF, 0);

        // Username
        let field_x = px + 40;
        let field_w = panel_w - 80;
        win.draw_string(field_x, py + 70, "Username", theme.text_secondary, 0);
        let uy = py + 95;
        let u_bg = if active_field == 0 {
            theme.bg_elevated
        } else {
            theme.bg_primary
        };
        win.draw_rounded_rect(field_x, uy, field_w, 35, 6, u_bg);
        if active_field == 0 {
            win.draw_rounded_rect_outline(field_x, uy, field_w, 35, 6, theme.accent);
        }
        let u_text = core::str::from_utf8(&username_buf).unwrap_or("");
        win.draw_string(field_x + 10, uy + 10, u_text, theme.text, 0);

        // Password
        win.draw_string(field_x, py + 145, "Password", theme.text_secondary, 0);
        let pwy = py + 170;
        let p_bg = if active_field == 1 {
            theme.bg_elevated
        } else {
            theme.bg_primary
        };
        win.draw_rounded_rect(field_x, pwy, field_w, 35, 6, p_bg);
        if active_field == 1 {
            win.draw_rounded_rect_outline(field_x, pwy, field_w, 35, 6, theme.accent);
        }

        let pw_text: String = if show_password {
            core::str::from_utf8(&password_buf).unwrap_or("").into()
        } else {
            core::iter::repeat('*').take(password_buf.len()).collect()
        };
        win.draw_string(field_x + 10, pwy + 10, &pw_text, theme.text, 0);

        // Show Password toggle
        let eye_x = field_x + field_w - 30;
        let eye_y = pwy + 10;
        win.draw_string(
            eye_x,
            eye_y,
            if show_password { "O" } else { "X" },
            theme.text_disabled,
            0,
        );
        if m_pressed
            && mx >= eye_x as u64
            && mx < (eye_x + 20) as u64
            && my >= eye_y as u64
            && my < (eye_y + 20) as u64
        {
            show_password = !show_password;
            unsafe {
                libsarga::syscall::syscall1(35, 100_000_000u64);
            }
        }

        // Error message
        if !error_msg.is_empty() {
            win.draw_string_centered(py + 215, &error_msg, theme.error, 0);
        }

        // Login button
        let btn_w = 120;
        let btn_x = px + (panel_w - btn_w) / 2;
        let btn_y = py + 250;
        win.draw_gradient_rect(
            btn_x,
            btn_y,
            btn_w,
            35,
            theme.accent,
            theme.accent_dark,
            true,
        );
        win.draw_string_centered(btn_y + 10, "Login", 0xFFFFFFFF, 0);

        // Power button
        let pwr_x = 760u32;
        let pwr_y = 560u32;
        win.draw_rounded_rect(pwr_x, pwr_y, 30, 30, 15, theme.bg_elevated);
        win.draw_string(pwr_x + 10, pwr_y + 7, "P", 0xFFFFFFFF, 0);
        if m_pressed && mx >= pwr_x as u64 && my >= pwr_y as u64 {
            power_menu = !power_menu;
            unsafe {
                libsarga::syscall::syscall1(35, 100_000_000u64);
            }
        }

        if power_menu {
            let menu_w = 120;
            let menu_h = 80;
            let mx_pos = pwr_x - menu_w + 30;
            let my_pos = pwr_y - menu_h - 5;
            win.draw_rounded_rect(mx_pos, my_pos, menu_w, menu_h, 8, theme.bg_surface);
            win.draw_rounded_rect_outline(mx_pos, my_pos, menu_w, menu_h, 8, theme.border);
            win.draw_string(mx_pos + 10, my_pos + 15, "Reboot", theme.text, 0);
            win.draw_string(mx_pos + 10, my_pos + 45, "Shutdown", theme.text, 0);

            if m_pressed && mx >= mx_pos as u64 && mx < (mx_pos + menu_w) as u64 {
                if my >= (my_pos + 10) as u64 && my < (my_pos + 40) as u64 {
                    // Reboot syscall or exit
                    process::exit(0);
                } else if my >= (my_pos + 40) as u64 && my < (my_pos + 70) as u64 {
                    // Shutdown syscall
                    process::exit(0);
                }
            }
        }

        let _ = win.flush();
        unsafe {
            libsarga::syscall::syscall1(35, 16_000_000u64);
        }
    }
}

sarga_main!(user_main);
