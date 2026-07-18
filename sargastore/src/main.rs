#![no_std]
#![no_main]
extern crate alloc;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use libsarga::{sarga_main, gui::Window, theme::Theme, io, process};

#[derive(Clone, Copy, PartialEq)]
enum Cat { All, System, Utilities, Development, Games, Libraries }

impl Cat {
    fn name(self) -> &'static str {
        match self {
            Cat::All => "All Packages", Cat::System => "System", Cat::Utilities => "Utilities",
            Cat::Development => "Development", Cat::Games => "Games", Cat::Libraries => "Libraries",
        }
    }
    const ALL: [Cat; 6] = [
        Cat::All, Cat::System, Cat::Utilities, Cat::Development, Cat::Games, Cat::Libraries,
    ];
    fn from_str(s: &str) -> Option<Self> {
        match s {
            "All" => Some(Cat::All), "System" => Some(Cat::System),
            "Utilities" => Some(Cat::Utilities), "Development" => Some(Cat::Development),
            "Games" => Some(Cat::Games), "Libraries" => Some(Cat::Libraries),
            _ => None,
        }
    }
}

struct Pkg {
    name: &'static str,
    version: &'static str,
    desc: &'static str,
    cat: Cat,
    installed: bool,
}

fn defaults() -> Vec<Pkg> {
    vec![
        Pkg { name: "base-system",    version: "1.0.0", desc: "Core SARGA OS components",     cat: Cat::System,     installed: true },
        Pkg { name: "sarga-shell",    version: "1.1.0", desc: "Modern system shell",           cat: Cat::System,     installed: true },
        Pkg { name: "sarga-search",   version: "0.1.0", desc: "System-wide search hub",        cat: Cat::Utilities,  installed: false },
        Pkg { name: "sarga-editor",   version: "0.4.2", desc: "Text and code editor",          cat: Cat::Development, installed: false },
        Pkg { name: "sarga-terminal", version: "0.3.1", desc: "Terminal emulator",             cat: Cat::Utilities,  installed: false },
        Pkg { name: "sarga-files",    version: "0.5.0", desc: "Graphical file manager",        cat: Cat::Utilities,  installed: false },
        Pkg { name: "sarga-monitor",  version: "0.2.0", desc: "System resource monitor",       cat: Cat::System,     installed: false },
        Pkg { name: "sarga-settings", version: "0.6.0", desc: "System configuration panel",    cat: Cat::System,     installed: false },
        Pkg { name: "sarga-calculator", version: "1.0.0", desc: "Desktop calculator",           cat: Cat::Utilities,  installed: false },
        Pkg { name: "sarga-calendar", version: "0.1.0", desc: "Calendar and reminders",         cat: Cat::Utilities,  installed: false },
        Pkg { name: "sarga-paint",    version: "0.2.0", desc: "Simple drawing tool",           cat: Cat::Games,      installed: false },
        Pkg { name: "sarga-notes",    version: "0.3.0", desc: "Quick notes app",                cat: Cat::Utilities,  installed: false },
        Pkg { name: "vahi-compiler",  version: "0.1.0", desc: "Vahi language compiler",         cat: Cat::Development, installed: false },
        Pkg { name: "coreutils",      version: "1.0.0", desc: "Core userland utilities",        cat: Cat::System,     installed: true },
        Pkg { name: "nettools",       version: "1.0.0", desc: "Network utilities and tools",    cat: Cat::System,     installed: true },
        Pkg { name: "libsarga",       version: "0.1.0", desc: "Userspace system library",        cat: Cat::Libraries,  installed: true },
    ]
}

fn load_metadata(pkgs: &mut Vec<Pkg>) {
    if let Ok(s) = io::read_to_string("/etc/packages.list") {
        let mut name = String::new();
        let mut version = String::new();
        let mut desc = String::new();
        let mut cat: Option<Cat> = None;
        let mut installed: Option<bool> = None;
        let mut section: Option<String> = None;
        let mut flush = |name: &str, version: &str, desc: &str, cat: Option<Cat>, installed: Option<bool>, section: &Option<String>| {
            if let Some(sec) = section {
                if !sec.is_empty() {
                    for p in pkgs.iter_mut() {
                        if p.name == sec.as_str() {
                            if !version.is_empty() { p.version = leak_str(version); }
                            if !desc.is_empty() { p.desc = leak_str(desc); }
                            if let Some(c) = cat { p.cat = c; }
                            if let Some(i) = installed { p.installed = i; }
                            return;
                        }
                    }
                }
            }
        };
        for line in s.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') { continue; }
            if line.starts_with('[') && line.ends_with(']') {
                flush(&name, &version, &desc, cat, installed, &section);
                name.clear(); version.clear(); desc.clear(); cat = None; installed = None;
                section = Some(line[1..line.len()-1].to_string());
                continue;
            }
            if let Some(eq) = line.find('=') {
                let k = line[..eq].trim();
                let v = line[eq+1..].trim();
                match k {
                    "version" => version = v.to_string(),
                    "description" => desc = v.to_string(),
                    "category" => cat = Cat::from_str(v),
                    "installed" => installed = Some(v == "1" || v == "true"),
                    _ => {}
                }
            }
        }
        flush(&name, &version, &desc, cat, installed, &section);
    }
}

fn leak_str(s: &str) -> &'static str {
    let mut v = alloc::vec::Vec::with_capacity(s.len());
    v.extend_from_slice(s.as_bytes());
    let leaked: &'static [u8] = v.leak();
    core::str::from_utf8(leaked).unwrap_or("")
}

fn visible<'a>(pkgs: &'a [Pkg], cat: Cat, query: &str) -> Vec<usize> {
    let q = query.trim().to_lowercase();
    pkgs.iter().enumerate().filter(|(_, p)| {
        (cat == Cat::All || p.cat == cat) && (q.is_empty() || p.name.to_lowercase().contains(&q) || p.desc.to_lowercase().contains(&q))
    }).map(|(i, _)| i).collect()
}

const W: u32 = 700;
const H: u32 = 500;
const SIDEBAR_W: u32 = 160;
const TOP_H: u32 = 50;
const BOT_H: u32 = 30;
const LIST_TOP: u32 = TOP_H + 8;
const LIST_BOT: u32 = H - BOT_H - 4;
const CARD_H: u32 = 56;
const CARD_PAD: u32 = 6;
const CARD_X: u32 = SIDEBAR_W + 8;
const CARD_W: u32 = W - SIDEBAR_W - 16;

fn in_rect(mx: u64, my: u64, x: u32, y: u32, w: u32, h: u32) -> bool {
    mx >= x as u64 && mx < (x + w) as u64 && my >= y as u64 && my < (y + h) as u64
}

fn try_install(pkg: &Pkg) -> bool {
    let path = alloc::format!("/var/spkg/{}.skp", pkg.name);
    if io::stat(path.as_str()).is_ok() {
        let cmd = alloc::format!("/bin/spkg install {} 1", path);
        if let Ok(_) = process::spawn(cmd.as_str()) { return true; }
    }
    false
}

fn user_main() -> i32 {
    let theme = Theme::dark();
    let mut win = match Window::create("Sarga Store", W, H) {
        Ok(w) => w,
        Err(e) => { io::print_str(&alloc::format!("sargastore: window failed: {}\n", e)); return 1; }
    };

    let mut pkgs = defaults();
    load_metadata(&mut pkgs);

    let mut sel_cat: Cat = Cat::All;
    let mut query = String::new();
    let mut search_focus = false;
    let mut prev_pressed = false;
    let mut selected: usize = 0;
    let mut scroll = 0u32;
    let mut installing: Option<usize> = None;
    let mut install_ticks: u32 = 0;
    let mut status: String = String::new();
    let search_box_x = SIDEBAR_W + 20;
    let search_box_w = W - SIDEBAR_W - 40;

    loop {
        let mouse = win.get_mouse();
        let pressed = (mouse.buttons & 1) != 0;
        let mx = mouse.x;
        let my = mouse.y;
        let clicked = pressed && !prev_pressed;

        while let Some(key) = win.get_key() {
            match key {
                0x1B => { if search_focus { query.clear(); } else { sel_cat = Cat::All; query.clear(); } }
                0x0A | 0x0D => {
                    let vis = visible(&pkgs, sel_cat, &query);
                    if let Some(&idx) = vis.get(selected) {
                        if !pkgs[idx].installed {
                            installing = Some(idx);
                            install_ticks = 0;
                        }
                    }
                }
                0x48 | 0x26 => { if selected > 0 { selected -= 1; } }
                0x50 | 0x28 => {
                    let vis = visible(&pkgs, sel_cat, &query);
                    if selected + 1 < vis.len() { selected += 1; }
                }
                0x4B | 0x25 => {
                    let pos = Cat::ALL.iter().position(|c| *c == sel_cat).unwrap_or(0);
                    if pos > 0 { sel_cat = Cat::ALL[pos - 1]; selected = 0; scroll = 0; }
                }
                0x4D | 0x27 | 0x09 => {
                    let pos = Cat::ALL.iter().position(|c| *c == sel_cat).unwrap_or(0);
                    if pos + 1 < Cat::ALL.len() { sel_cat = Cat::ALL[pos + 1]; selected = 0; scroll = 0; }
                }
                k if search_focus && (k.is_ascii_graphic() || k == b' ' || k == 0x08) => {
                    if k == 0x08 { query.pop(); }
                    else { query.push(k as char); }
                    selected = 0; scroll = 0;
                }
                b'/' => { search_focus = true; query.clear(); }
                _ => {}
            }
        }

        if clicked {
            if in_rect(mx, my, search_box_x, 14, search_box_w, 26) {
                search_focus = true;
            } else if my >= TOP_H as u64 {
                search_focus = false;
            }
            for (i, c) in Cat::ALL.iter().enumerate() {
                let y = TOP_H + 10 + i as u32 * 32;
                if in_rect(mx, my, 8, y, SIDEBAR_W - 16, 28) {
                    sel_cat = *c; selected = 0; scroll = 0;
                }
            }
            let vis = visible(&pkgs, sel_cat, &query);
            for (rank, &idx) in vis.iter().enumerate() {
                let y = LIST_TOP + rank as u32 * (CARD_H + CARD_PAD) - scroll;
                if y >= LIST_TOP && y < LIST_BOT {
                    let bx = CARD_X + CARD_W - 90;
                    let by = y + (CARD_H - 28) / 2;
                    if in_rect(mx, my, bx, by, 80, 28) {
                        if pkgs[idx].installed {
                            pkgs[idx].installed = false;
                            status = alloc::format!("Removed {}", pkgs[idx].name);
                        } else {
                            installing = Some(idx);
                            install_ticks = 0;
                        }
                    }
                }
            }
        }

        if let Some(idx) = installing {
            install_ticks += 1;
            let pct = (install_ticks as u32 * 10).min(100);
            status = alloc::format!("Installing {}... {}%", pkgs[idx].name, pct);
            if install_ticks >= 10 {
                let ok = try_install(&pkgs[idx]);
                pkgs[idx].installed = true;
                if ok {
                    status = alloc::format!("Installed {} (spkg)", pkgs[idx].name);
                } else {
                    status = alloc::format!("Installed {} (simulated)", pkgs[idx].name);
                }
                io::notify(status.as_str(), 3000);
                installing = None;
            }
        } else if status.is_empty() {
            let n = visible(&pkgs, sel_cat, &query).len();
            status = alloc::format!("{} packages", n);
        } else {
            let n = visible(&pkgs, sel_cat, &query).len();
            let mut s = status.clone();
            s.push_str(&alloc::format!("  |  {} packages", n));
            status.clear();
            status.push_str(&s);
        }

        if mouse.scroll != 0 {
            let new_scroll = (scroll as i64 - mouse.scroll as i64 * 40).max(0) as u32;
            let vis = visible(&pkgs, sel_cat, &query);
            let total_h = vis.len() as u32 * (CARD_H + CARD_PAD);
            let max_scroll = total_h.saturating_sub(LIST_BOT - LIST_TOP);
            scroll = new_scroll.min(max_scroll);
        }

        win.fill(theme.bg_primary);
        win.draw_rect(0, 0, W, TOP_H, theme.bg_surface);
        win.draw_rect(0, TOP_H, SIDEBAR_W, H - TOP_H, theme.bg_surface);
        win.draw_line_h(0, TOP_H, W, theme.border);
        win.draw_line_v(SIDEBAR_W, TOP_H, H - TOP_H, theme.border);
        win.draw_line_h(SIDEBAR_W, H - BOT_H, W - SIDEBAR_W, theme.border);

        win.draw_string(16, 18, "Sarga Store", theme.text, 0);
        let sb_bg = if search_focus { theme.bg_elevated } else { theme.bg_primary };
        win.draw_rounded_rect(search_box_x, 14, search_box_w, 26, 6, sb_bg);
        win.draw_rounded_rect_outline(search_box_x, 14, search_box_w, 26, 6, theme.border);
        let mut display_query = query.clone();
        if search_focus && install_ticks % 16 < 8 {
            display_query.push('|');
        }
        let qs = if display_query.is_empty() { "Search packages... (press /)" } else { display_query.as_str() };
        let qc = if display_query.is_empty() { theme.text_disabled } else { theme.text };
        win.draw_string(search_box_x + 10, 22, qs, qc, 0);

        for (i, c) in Cat::ALL.iter().enumerate() {
            let y = TOP_H + 10 + i as u32 * 32;
            let active = *c == sel_cat;
            let hover = in_rect(mx, my, 8, y, SIDEBAR_W - 16, 28);
            let bg = if active { theme.accent } else if hover { theme.bg_elevated } else { theme.bg_surface };
            win.draw_rounded_rect(8, y, SIDEBAR_W - 16, 28, 6, bg);
            win.draw_string(18, y + 8, c.name(), theme.text, 0);
        }

        let vis = visible(&pkgs, sel_cat, &query);
        let usable_h = LIST_BOT - LIST_TOP;
        let max_scroll = (vis.len() as u32 * (CARD_H + CARD_PAD)).saturating_sub(usable_h);
        scroll = scroll.min(max_scroll);
        let mut drawn = 0u32;
        for (rank, &idx) in vis.iter().enumerate() {
            let y_full = LIST_TOP + rank as u32 * (CARD_H + CARD_PAD);
            if y_full + CARD_H < LIST_TOP + scroll { continue; }
            if y_full > LIST_TOP + usable_h + scroll { break; }
            let y = y_full.saturating_sub(scroll);
            if y + CARD_H > LIST_BOT { continue; }
            let card_bg = if rank % 2 == 0 { theme.bg_surface } else { theme.bg_elevated };
            let is_sel = rank == selected;
            let bg = if is_sel { theme.hover } else { card_bg };
            win.draw_rounded_rect(CARD_X, y, CARD_W, CARD_H, 8, bg);
            win.draw_rounded_rect_outline(CARD_X, y, CARD_W, CARD_H, 8, theme.border);
            let p = &pkgs[idx];
            win.draw_string(CARD_X + 12, y + 8, p.name, theme.text, 0);
            let ver = alloc::format!("v{}", p.version);
            win.draw_string(CARD_X + 12 + p.name.len() as u32 * 8 + 12, y + 10, ver.as_str(), theme.text_secondary, 0);
            win.draw_string(CARD_X + 12, y + 28, p.desc, theme.text_secondary, 0);
            let cat_color = match p.cat {
                Cat::System => theme.accent_light, Cat::Utilities => theme.success,
                Cat::Development => theme.warning, Cat::Games => theme.accent,
                Cat::Libraries => theme.text_secondary, Cat::All => theme.text_secondary,
            };
            win.draw_string(CARD_X + 12, y + 42, p.cat.name(), cat_color, 0);

            let bt_x = CARD_X + CARD_W - 90;
            let bt_y = y + (CARD_H - 28) / 2;
            if let Some(ii) = installing {
                if ii == idx {
                    let pw = (install_ticks * 8) as u32;
                    win.draw_rounded_rect(bt_x, bt_y, 80, 28, 6, theme.bg_elevated);
                    win.draw_rounded_rect(bt_x, bt_y, pw, 28, 6, theme.accent);
                    win.draw_string(bt_x + 4, bt_y + 8, "...", theme.text, 0);
                    continue;
                }
            }
            if p.installed {
                win.draw_rounded_rect(bt_x, bt_y, 80, 28, 6, theme.success);
                let lbl = "Remove";
                let lw = lbl.len() as u32 * 8;
                win.draw_string(bt_x + (80 - lw) / 2, bt_y + 8, lbl, theme.text, 0);
            } else {
                let hover = in_rect(mx, my, bt_x, bt_y, 80, 28);
                let bg = if hover { theme.hover } else { theme.accent };
                win.draw_rounded_rect(bt_x, bt_y, 80, 28, 6, bg);
                let lbl = "Install";
                let lw = lbl.len() as u32 * 8;
                win.draw_string(bt_x + (80 - lw) / 2, bt_y + 8, lbl, theme.text, 0);
            }
            drawn += 1;
        }

        if drawn == 0 {
            win.draw_string_centered(LIST_TOP + 40, "No packages match.", theme.text_secondary, 0);
        }

        let total_h = vis.len() as u32 * (CARD_H + CARD_PAD);
        if total_h > usable_h {
            let track_h = usable_h;
            let thumb_h = (usable_h * usable_h / total_h).max(20);
            let track_x = W - 8;
            win.draw_rect(track_x, LIST_TOP, 4, track_h, theme.bg_elevated);
            let thumb_y = LIST_TOP + (scroll * (track_h - thumb_h) / max_scroll.max(1));
            win.draw_rect(track_x, thumb_y, 4, thumb_h, theme.accent);
        }

        win.draw_rect(0, H - BOT_H, W, BOT_H, theme.bg_surface);
        win.draw_string(12, H - BOT_H + 9, status.as_str(), theme.text_secondary, 0);
        win.draw_string(W - 130, H - BOT_H + 9, "/:search  Esc:clear", theme.text_disabled, 0);

        let _ = win.flush();
        prev_pressed = pressed;
        unsafe { libsarga::syscall::syscall2(35, 0, 16_000_000u64); }
    }
}

sarga_main!(user_main);
