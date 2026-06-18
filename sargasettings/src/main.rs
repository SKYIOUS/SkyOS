#![no_std]
#![no_main]
#![allow(unused_assignments)]
extern crate alloc;
use libsarga::sarga_main;
use libsarga::gui::Window;
use libsarga::io::{self, notify, clipboard_write};
use libsarga::theme::Theme;

const TAB_NAMES: &[&str] = &["Display", "Theme", "About"];
const SIDEBAR_W: u32 = 120;
const TAB_H: u32 = 28;

struct Settings {
    active_tab: usize,
    // Display
    resolution_idx: usize,
    // Theme
    accent_idx: usize,
    // Hover states
    hover_tab: Option<usize>,
    hover_item: Option<usize>,
}

impl Settings {
    fn new() -> Self {
        Settings {
            active_tab: 0,
            resolution_idx: 0,
            accent_idx: 0,
            hover_tab: None,
            hover_item: None,
        }
    }
}

const RESOLUTIONS: &[(&str, u32, u32)] = &[
    ("800x600", 800, 600),
    ("1024x768", 1024, 768),
    ("1280x720", 1280, 720),
    ("1280x800", 1280, 800),
    ("1366x768", 1366, 768),
    ("1440x900", 1440, 900),
    ("1600x900", 1600, 900),
    ("1920x1080", 1920, 1080),
];

const ACCENT_COLORS: &[(&str, u32)] = &[
    ("Blue", 0xFF0078D4),
    ("Teal", 0xFF0099BC),
    ("Green", 0xFF107C10),
    ("Red", 0xFFD32F2F),
    ("Purple", 0xFF8764B8),
    ("Orange", 0xFFCA5010),
    ("Pink", 0xFFE3008C),
];

fn draw_checkbox(win: &mut Window, theme: &Theme, x: u32, y: u32, label: &str, checked: bool) {
    // Checkbox box
    let bg = if checked { theme.accent } else { theme.bg_elevated };
    win.draw_rounded_rect(x, y, 16, 16, 3, bg);
    if checked {
        win.draw_string(x + 2, y + 1, "x", 0xFFFFFFFF, 0);
    }
    // Label
    win.draw_string(x + 22, y + 1, label, theme.text, 0);
}

fn draw_radio(win: &mut Window, theme: &Theme, x: u32, y: u32, label: &str, selected: bool) {
    // Radio circle (draw as rounded rect approximation)
    let bg = if selected { theme.accent } else { theme.bg_elevated };
    win.draw_rounded_rect(x, y, 16, 16, 8, bg);
    if selected {
        win.draw_rounded_rect(x + 4, y + 4, 8, 8, 4, 0xFFFFFFFF);
    }
    // Label
    win.draw_string(x + 22, y + 1, label, theme.text, 0);
}

fn user_main() -> i32 {
    let theme = Theme::dark();
    let mut settings = Settings::new();

    let win_w = 500u32;
    let win_h = 360u32;

    let mut win = match Window::create("System Settings", win_w, win_h) {
        Ok(w) => w,
        Err(e) => { io::print_str(&alloc::format!("sargasettings: window failed: {}\n", e)); return 0; }
    };

    let mut prev_pressed = false;

    loop {
        let mouse = win.get_mouse();
        let pressed = (mouse.buttons & 1) != 0;
        let mx = mouse.x as u32;
        let my = mouse.y as u32;

        // Click handling
        if pressed && !prev_pressed {
            // Tab clicks (sidebar)
            if mx < SIDEBAR_W {
                for (i, _) in TAB_NAMES.iter().enumerate() {
                    let ty = 8 + i as u32 * (TAB_H + 4);
                    if my >= ty && my < ty + TAB_H {
                        settings.active_tab = i;
                        settings.hover_item = None;
                    }
                }
            }

            // Content area clicks
            if mx >= SIDEBAR_W {
                match settings.active_tab {
                    0 => {
                        // Display settings - resolution selection
                        let content_y = 40;
                        for (i, &(_name, _, _)) in RESOLUTIONS.iter().enumerate() {
                            let iy = content_y + i as u32 * 24;
                            if mx >= SIDEBAR_W + 16 && mx < win_w - 16 && my >= iy && my < iy + 20 {
                                settings.resolution_idx = i;
                                let (_, w, h) = RESOLUTIONS[i];
                                let _ = libsarga::gpu::set_mode(w, h, 32);
                                notify(&alloc::format!("Resolution: {}x{}", w, h), 2000);
                            }
                        }
                    }
                    1 => {
                        // Theme settings - accent color selection
                        let content_y = 40;
                        for (i, &(name, color)) in ACCENT_COLORS.iter().enumerate() {
                            let iy = content_y + i as u32 * 28;
                            if mx >= SIDEBAR_W + 16 && mx < win_w - 16 && my >= iy && my < iy + 24 {
                                settings.accent_idx = i;
                                let _ = libsarga::gpu::set_accent_color(color);
                                notify(&alloc::format!("Accent: {}", name), 2000);
                            }
                        }
                    }
                    2 => {
                        // About - copy info button
                        if mx >= SIDEBAR_W + 16 && mx < SIDEBAR_W + 160 && my >= 240 && my < 270 {
                            clipboard_write(b"SARGA OS v0.4.0\nKernel: SARGA\nArch: x86_64\n");
                            notify("Copied system info to clipboard", 2000);
                        }
                    }
                    _ => {}
                }
            }
        }
        if !pressed { prev_pressed = false; }

        // Hover tracking
        settings.hover_tab = None;
        settings.hover_item = None;
        if mx < SIDEBAR_W {
            for (i, _) in TAB_NAMES.iter().enumerate() {
                let ty = 8 + i as u32 * (TAB_H + 4);
                if my >= ty && my < ty + TAB_H {
                    settings.hover_tab = Some(i);
                }
            }
        }

        // Render
        win.clear(theme.bg_primary);

        // Sidebar
        win.draw_rect(0, 0, SIDEBAR_W, win_h, theme.bg_surface);

        // Header
        win.draw_rect(0, 0, SIDEBAR_W, 32, theme.accent);
        win.draw_string(12, 8, "Settings", 0xFFFFFFFF, 0);

        // Tab buttons
        for (i, name) in TAB_NAMES.iter().enumerate() {
            let ty = 40 + i as u32 * (TAB_H + 4);
            let is_active = settings.active_tab == i;
            let is_hover = settings.hover_tab == Some(i);
            let bg = if is_active { theme.accent } else if is_hover { theme.hover } else { 0 };
            if bg != 0 { win.draw_rounded_rect(4, ty, SIDEBAR_W - 8, TAB_H, 4, bg); }
            let tc = if is_active { 0xFFFFFFFF } else { theme.text_secondary };
            win.draw_string(12, ty + 7, name, tc, 0);
        }

        // Separator
        win.draw_line_v(SIDEBAR_W, 0, win_h, theme.border);

        // Content area
        match settings.active_tab {
            0 => {
                // Display Settings
                win.draw_string(SIDEBAR_W + 16, 12, "Display Settings", theme.text, 0);
                win.draw_line_h(SIDEBAR_W + 16, 32, win_w - SIDEBAR_W - 32, theme.separator);

                // Resolution selection
                win.draw_string(SIDEBAR_W + 16, 40, "Resolution", theme.text_secondary, 0);
                for (i, &(name, _w, _h)) in RESOLUTIONS.iter().enumerate() {
                    let iy = 64 + i as u32 * 24;
                    let selected = settings.resolution_idx == i;
                    draw_radio(&mut win, &theme, SIDEBAR_W + 16, iy, name, selected);
                }
            }
            1 => {
                // Theme Settings
                win.draw_string(SIDEBAR_W + 16, 12, "Theme Settings", theme.text, 0);
                win.draw_line_h(SIDEBAR_W + 16, 32, win_w - SIDEBAR_W - 32, theme.separator);

                // Accent color
                win.draw_string(SIDEBAR_W + 16, 40, "Accent Color", theme.text_secondary, 0);
                for (i, &(name, color)) in ACCENT_COLORS.iter().enumerate() {
                    let iy = 64 + i as u32 * 28;
                    let selected = settings.accent_idx == i;

                    // Color swatch
                    win.draw_rounded_rect(SIDEBAR_W + 16, iy, 20, 20, 4, color);
                    if selected {
                        win.draw_rect(SIDEBAR_W + 14, iy - 2, 24, 24, theme.text);
                        win.draw_rounded_rect(SIDEBAR_W + 16, iy, 20, 20, 4, color);
                    }

                    // Label
                    win.draw_string(SIDEBAR_W + 44, iy + 3, name, theme.text, 0);
                }

                // Dark mode toggle
                let toggle_y = 64 + ACCENT_COLORS.len() as u32 * 28 + 16;
                win.draw_string(SIDEBAR_W + 16, toggle_y, "Appearance", theme.text_secondary, 0);
                draw_checkbox(&mut win, &theme, SIDEBAR_W + 16, toggle_y + 24, "Dark Mode (always on)", true);
                draw_checkbox(&mut win, &theme, SIDEBAR_W + 16, toggle_y + 48, "Show animations", true);
            }
            2 => {
                // About
                win.draw_string(SIDEBAR_W + 16, 12, "About SARGA OS", theme.text, 0);
                win.draw_line_h(SIDEBAR_W + 16, 32, win_w - SIDEBAR_W - 32, theme.separator);

                // Logo area
                win.draw_rounded_rect(SIDEBAR_W + 16, 48, 80, 80, 8, theme.accent);
                win.draw_string(SIDEBAR_W + 32, 72, "Sky", 0xFFFFFFFF, 0);
                win.draw_string(SIDEBAR_W + 28, 88, "OS", 0xFFFFFFFF, 0);

                // Info
                let info_x = SIDEBAR_W + 112;
                win.draw_string(info_x, 52, "SARGA OS v0.4.0", theme.text, 0);
                win.draw_string(info_x, 72, "Kernel: SARGA", theme.text_secondary, 0);
                win.draw_string(info_x, 88, "Arch: x86_64", theme.text_secondary, 0);
                win.draw_string(info_x, 104, "Shell: SargaSH", theme.text_secondary, 0);
                win.draw_string(info_x, 120, "Desktop: ADE", theme.text_secondary, 0);

                win.draw_line_h(SIDEBAR_W + 16, 148, win_w - SIDEBAR_W - 32, theme.separator);

                // Features
                win.draw_string(SIDEBAR_W + 16, 160, "Features", theme.text_secondary, 0);
                let features = [
                    "In-kernel GUI compositor",
                    "TTF font rendering",
                    "Widget toolkit (14 widgets)",
                    "VT100 terminal emulator",
                    "60+ core utilities",
                    "Package manager (.skp)",
                    "TCP/UDP networking",
                    "DAC + Capabilities security",
                    "eBPF virtual machine",
                    "io_uring async I/O",
                ];
                for (i, feat) in features.iter().enumerate() {
                    win.draw_string(SIDEBAR_W + 24, 180 + i as u32 * 16, feat, theme.text, 0);
                }

                // Copy button
                let btn_y = 240;
                let btn_hover = mx >= SIDEBAR_W + 16 && mx < SIDEBAR_W + 160 && my >= btn_y && my < btn_y + 28;
                let btn_bg = if btn_hover { theme.hover } else { theme.accent };
                win.draw_rounded_rect(SIDEBAR_W + 16, btn_y, 144, 28, 4, btn_bg);
                win.draw_string(SIDEBAR_W + 28, btn_y + 6, "Copy Info", 0xFFFFFFFF, 0);
            }
            _ => {}
        }

        // Keyboard
        while let Some(key) = win.get_key() {
            match key {
                b'q' | b'Q' => return,
                b'1' => settings.active_tab = 0,
                b'2' => settings.active_tab = 1,
                b'3' => settings.active_tab = 2,
                _ => {}
            }
        }

        let _ = win.flush();
        prev_pressed = pressed;
        unsafe { libsarga::syscall::syscall1(35, 16_666_000); }
    }
    0
}

sarga_main!(user_main);
