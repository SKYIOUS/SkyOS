#![no_std]
#![no_main]
extern crate alloc;
use alloc::string::String;
use libsarga::theme::Theme;
use libsarga::{gui::Window, sarga_main};
use libsarga::{io, process, hash};

const SHADOW_PATH: &str = "/etc/shadow";

fn verify_password(username: &str, password: &str) -> bool {
    let data = match libsarga::fs::read_to_string(SHADOW_PATH) {
        Ok(d) => d.into_bytes(),
        Err(_) => return false,
    };
    libsarga::hash::verify_password(&data, username, password)
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
    let mut m_was_pressed = false;

    loop {
        let mouse = win.get_mouse();
        let mx = mouse.x;
        let my = mouse.y;
        let m_pressed = mouse.buttons != 0;
        let m_down = m_pressed && !m_was_pressed;
        m_was_pressed = m_pressed;

        while let Some(key) = win.get_key() {
            match key {
                0x09 => {
                    active_field = (active_field + 1) % 2;
                } // Tab
                0x0A | 0x0D => {
                    let user = core::str::from_utf8(&username_buf).unwrap_or("");
                    let pass = core::str::from_utf8(&password_buf).unwrap_or("");
                    if verify_password(user, pass) {
                        match process::execve("/bin/ade", &["/bin/ade"], &[]) {
                            Ok(_) => return 0,
                            Err(_) => {
                                let _ = io::write_all(1, b"[login] execve failed, continuing\n");
                                password_buf.clear();
                            }
                        }
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
        if m_down
            && mx >= eye_x as u64
            && mx < (eye_x + 20) as u64
            && my >= eye_y as u64
            && my < (eye_y + 20) as u64
        {
            show_password = !show_password;
            let _ = io::nanosleep(100_000_000);
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
        if m_down && mx >= pwr_x as u64 && my >= pwr_y as u64 {
            power_menu = !power_menu;
            let _ = io::nanosleep(100_000_000);
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

            if m_down && mx >= mx_pos as u64 && mx < (mx_pos + menu_w) as u64 {
                if my >= (my_pos + 10) as u64 && my < (my_pos + 40) as u64 {
                    io::reboot();
                } else if my >= (my_pos + 40) as u64 && my < (my_pos + 70) as u64 {
                    io::poweroff();
                }
            }
        }

        let _ = win.flush();
        let _ = io::nanosleep(16_000_000);
    }
}

sarga_main!(user_main);
