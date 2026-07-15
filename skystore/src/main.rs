#![no_std]
#![no_main]
extern crate alloc;

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use libsarga::{
    sarga_main, gui::Window, theme::Theme, process, io,
    widget::Widget, textbox::TextBox,
};

struct PkgInfo {
    name: &'static str,
    version: &'static str,
    description: &'static str,
    depends: &'static [&'static str],
}

static CATALOG: &[PkgInfo] = &[
    PkgInfo { name: "System Monitor", version: "1.0.0", description: "View CPU, memory and process information", depends: &[] },
    PkgInfo { name: "File Manager",   version: "1.1.0", description: "Browse and manage files and directories", depends: &[] },
    PkgInfo { name: "Terminal",       version: "2.0.0", description: "Command-line shell and terminal emulator", depends: &[] },
    PkgInfo { name: "Calculator",     version: "1.2.0", description: "Simple yet powerful calculator", depends: &[] },
    PkgInfo { name: "Clock",          version: "1.0.0", description: "World clock with alarms and timers", depends: &[] },
    PkgInfo { name: "Notes",          version: "1.1.0", description: "Quick note-taking application", depends: &[] },
    PkgInfo { name: "Paint",          version: "1.0.0", description: "Simple drawing and image editing tool", depends: &[] },
    PkgInfo { name: "SargaEdit",      version: "1.3.0", description: "Lightweight code editor with syntax highlighting", depends: &[] },
    PkgInfo { name: "SargaView",      version: "1.0.0", description: "Image and document viewer", depends: &[] },
    PkgInfo { name: "SargaBuild",     version: "0.9.0", description: "Build system and task runner", depends: &[] },
    PkgInfo { name: "SkyScript",      version: "0.5.0", description: "SkyOS scripting language runtime", depends: &[] },
    PkgInfo { name: "KorLang IDE",    version: "0.2.0", description: "IDE for KorLang development", depends: &["SkyScript"] },
    PkgInfo { name: "SargaPlayer",    version: "1.0.0", description: "Audio and video media player", depends: &[] },
    PkgInfo { name: "Image Viewer",   version: "1.0.0", description: "Quick image viewer", depends: &[] },
    PkgInfo { name: "Audio Recorder", version: "1.0.0", description: "Record audio from microphone", depends: &[] },
    PkgInfo { name: "Web Browser",    version: "0.8.0", description: "Browse the world wide web", depends: &[] },
    PkgInfo { name: "Mail Client",    version: "0.7.0", description: "Email client application", depends: &[] },
    PkgInfo { name: "Chat",           version: "1.0.0", description: "Instant messaging client", depends: &[] },
    PkgInfo { name: "SargaDocs",      version: "0.6.0", description: "Word processor and document editor", depends: &[] },
    PkgInfo { name: "SargaSheets",    version: "0.4.0", description: "Spreadsheet application", depends: &[] },
    PkgInfo { name: "Calendar",       version: "1.0.0", description: "Calendar and schedule manager", depends: &[] },
];

struct Package {
    name: String,
    version: String,
    description: String,
    depends: Vec<String>,
    installed: bool,
}

enum View {
    Browse,
    Detail(usize),
}

fn hash_color(s: &str) -> u32 {
    let mut h = 0u32;
    for b in s.bytes() {
        h = h.wrapping_mul(31).wrapping_add(b as u32);
    }
    0xFF000000 | ((h ^ 0x3D5AFE) & 0xFFFFFF)
}

fn ascii_lower(s: &str) -> String {
    let mut r = String::with_capacity(s.len());
    for b in s.bytes() {
        r.push(if b >= b'A' && b <= b'Z' { (b + 32) as char } else { b as char });
    }
    r
}

fn list_dir(path: &str) -> Vec<String> {
    let mut buf = [0u8; 256];
    let bytes = path.as_bytes();
    if bytes.len() > 254 { return Vec::new(); }
    buf[..bytes.len()].copy_from_slice(bytes);
    buf[bytes.len()] = 0;
    let fd = unsafe { libsarga::syscall::syscall2(2, buf.as_ptr() as u64, 0) };
    if fd < 0 { return Vec::new(); }
    let mut entries = Vec::new();
    let mut dent_buf = [0u8; 4096];
    loop {
        let n = unsafe { libsarga::syscall::syscall3(217, fd as u64, dent_buf.as_mut_ptr() as u64, dent_buf.len() as u64) };
        if n <= 0 { break; }
        let mut off = 0usize;
        while off + 19 <= n as usize {
            let reclen = u16::from_ne_bytes([dent_buf[off + 16], dent_buf[off + 17]]) as usize;
            if reclen < 19 { break; }
            let name_start = off + 19;
            let name_end = name_start + dent_buf[name_start..off + reclen].iter().position(|&b| b == 0).unwrap_or(reclen - 19);
            if let Ok(name) = core::str::from_utf8(&dent_buf[name_start..name_end]) {
                if !name.is_empty() && name != "." && name != ".." {
                    entries.push(name.to_string());
                }
            }
            off += reclen;
        }
    }
    unsafe { libsarga::syscall::syscall1(3, fd as u64); }
    entries
}

fn parse_manifest(data: &str) -> Option<(String, String, String, Vec<String>)> {
    let mut name = String::new();
    let mut version = String::new();
    let mut desc = String::new();
    let mut deps = Vec::new();
    for line in data.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') { continue; }
        if let Some(eq) = line.find('=') {
            let key = line[..eq].trim();
            let val = line[eq + 1..].trim();
            match key {
                "name" => name = val.to_string(),
                "version" => version = val.to_string(),
                "description" => desc = val.to_string(),
                "depends" => {
                    for d in val.split(',') {
                        let d = d.trim();
                        if !d.is_empty() { deps.push(d.to_string()); }
                    }
                }
                _ => {}
            }
        }
    }
    if name.is_empty() { None } else { Some((name, version, desc, deps)) }
}

fn discover_packages() -> Vec<Package> {
    let installed: Vec<String> = list_dir("/etc/spkg/packages/");

    let files = list_dir("/packages/");
    if !files.is_empty() {
        let mut pkgs = Vec::new();
        for fname in &files {
            if !fname.ends_with(".skp") { continue; }
            let path = alloc::format!("/packages/{}", fname);
            if let Ok(data) = io::read_to_string(&path) {
                if let Some((name, version, desc, deps)) = parse_manifest(&data) {
                    let inst = installed.iter().any(|n| n == &name);
                    pkgs.push(Package { name, version, description: desc, depends: deps, installed: inst });
                }
            }
        }
        if !pkgs.is_empty() { return pkgs; }
    }

    CATALOG.iter().map(|p| {
        let inst = installed.iter().any(|n| n.as_str() == p.name);
        Package {
            name: p.name.to_string(),
            version: p.version.to_string(),
            description: p.description.to_string(),
            depends: p.depends.iter().map(|d| d.to_string()).collect(),
            installed: inst,
        }
    }).collect()
}

fn filter_packages(pkgs: &[Package], query: &str) -> Vec<usize> {
    if query.is_empty() { return (0..pkgs.len()).collect(); }
    let q = ascii_lower(query);
    pkgs.iter().enumerate()
        .filter(|(_, p)| ascii_lower(&p.name).contains(&q) || ascii_lower(&p.description).contains(&q))
        .map(|(i, _)| i)
        .collect()
}

fn run_install(path: &str) -> bool {
    if let Ok(pid) = process::fork() {
        if pid == 0 {
            let _ = process::execve("/bin/spkg", &["spkg", "install", path], &[]);
            process::exit(1);
        }
        if let Ok((_, status)) = process::waitpid(pid as i64, 0) {
            return status == 0;
        }
    }
    false
}

fn run_remove(name: &str) -> bool {
    if let Ok(pid) = process::fork() {
        if pid == 0 {
            let _ = process::execve("/bin/spkg", &["spkg", "remove", name], &[]);
            process::exit(1);
        }
        if let Ok((_, status)) = process::waitpid(pid as i64, 0) {
            return status == 0;
        }
    }
    false
}

fn show_progress(win: &mut Window, theme: &Theme, text: &str) {
    win.draw_rect(0, 0, win.width, win.height, 0x80000000);
    win.draw_rounded_rect(150, 200, 400, 100, 10, theme.bg_elevated);
    win.draw_string(180, 230, text, theme.text, 0);
    win.draw_rounded_rect(170, 260, 360, 10, 5, theme.bg_elevated);
    win.draw_rounded_rect(170, 260, 180, 10, 5, theme.accent);
    let _ = win.flush();
}

fn show_confirm(win: &mut Window, theme: &Theme, msg: &str) -> bool {
    // Wait for button release first so the initial click doesn't fire again
    while (win.get_mouse().buttons & 1) != 0 {
        while let Some(_) = win.get_key() {}
        unsafe { libsarga::syscall::syscall1(35, 8_000_000u64); }
    }
    let mut prev = 0u8;
    loop {
        while let Some(k) = win.get_key() { if k == 0x1B { return false; } }
        let mouse = win.get_mouse();
        let clicked = (mouse.buttons & 1) == 1 && (prev & 1) == 0;
        prev = mouse.buttons;

        win.draw_rect(0, 0, win.width, win.height, 0x80000000);
        win.draw_rounded_rect(160, 190, 380, 120, 10, theme.bg_elevated);
        win.draw_rect(160, 190, 380, 30, theme.accent);
        win.draw_string(180, 198, "Remove Package", 0xFFFFFFFF, 0);
        win.draw_string(190, 235, msg, theme.text, 0);

        let rx = 240u32;
        win.draw_rounded_rect(rx, 270, 80, 28, 6, theme.error);
        win.draw_string(rx + 8, 277, "Remove", 0xFFFFFFFF, 0);
        let cx = 340u32;
        win.draw_rounded_rect(cx, 270, 80, 28, 6, theme.bg_surface);
        win.draw_string(cx + 12, 277, "Cancel", 0xFFFFFFFF, 0);

        let _ = win.flush();

        if clicked {
            let mx = mouse.x as i32;
            let my = mouse.y as i32;
            if mx >= rx as i32 && mx < (rx + 80) as i32 && my >= 270 && my < 298 { return true; }
            if mx >= cx as i32 && mx < (cx + 80) as i32 && my >= 270 && my < 298 { return false; }
        }
        unsafe { libsarga::syscall::syscall1(35, 8_000_000u64); }
    }
}

fn user_main() -> i32 {
    let mut win = Window::create("Sarga Store", 700, 500).expect("Window::create failed");
    let theme = Theme::dark();

    let mut packages = discover_packages();
    let mut search = TextBox::new(10, 44, 680, 26).with_placeholder("Search packages...");
    let mut scroll = 0u32;
    let mut sel = 0usize;
    let mut view = View::Browse;
    let mut prev_btn = 0u8;

    loop {
        let mouse = win.get_mouse();
        let mx = mouse.x as i32;
        let my = mouse.y as i32;
        let click = (mouse.buttons & 1) == 1 && (prev_btn & 1) == 0;
        prev_btn = mouse.buttons;

        while let Some(key) = win.get_key() {
            match key {
                0x09 => search.set_focus(!search.is_focused()),
                0x2F => search.set_focus(true),
                0x1B => match view {
                    View::Browse => search.set_focus(false),
                    View::Detail(_) => { view = View::Browse; search.set_focus(false); }
                },
                0x0D => {
                    if let View::Browse = view {
                        if !search.is_focused() {
                            let fi = filter_packages(&packages, search.text());
                            if sel < fi.len() { view = View::Detail(fi[sel]); }
                        }
                    }
                }
                0x26 | 0x48 => {
                    if let View::Browse = view {
                        if !search.is_focused() {
                            let fi = filter_packages(&packages, search.text());
                            if !fi.is_empty() {
                                sel = sel.saturating_sub(1);
                                if sel < scroll as usize { scroll = sel as u32; }
                            }
                        }
                    }
                }
                0x28 | 0x50 => {
                    if let View::Browse = view {
                        if !search.is_focused() {
                            let fi = filter_packages(&packages, search.text());
                            if !fi.is_empty() {
                                sel = (sel + 1).min(fi.len() - 1);
                                let vis = 7;
                                if sel >= scroll as usize + vis { scroll = (sel - vis + 1) as u32; }
                            }
                        }
                    }
                }
                _ => { search.handle_key(key); }
            }
        }

        if click {
            search.handle_click(mx, my, true);
            match view {
                View::Browse => {
                    let fi = filter_packages(&packages, search.text());
                    for (i, &pi) in fi.iter().enumerate() {
                        let ey = 80 + i as u32 * 56 - scroll;
                        if mx >= 10 && mx < 690 && my >= ey as i32 && my < (ey + 50) as i32 {
                            if mx >= 610 && mx < 680 && ey + 10 <= my as u32 && (my as u32) < ey + 40 {
                                if packages[pi].installed {
                                    view = View::Detail(pi);
                                } else {
                                    let path = alloc::format!("/packages/{}.skp", packages[pi].name);
                                    show_progress(&mut win, &theme, &alloc::format!("Installing {}...", packages[pi].name));
                                    run_install(&path);
                                    packages = discover_packages();
                                }
                            } else {
                                view = View::Detail(pi);
                            }
                            break;
                        }
                    }
                }
                View::Detail(idx) if idx < packages.len() => {
                    if mx >= 10 && mx < 90 && my >= 80 && my < 108 { view = View::Browse; }
                    let bx = (700 - 120) / 2;
                    if mx >= bx && mx < bx + 120 && my >= 420 && my < 456 {
                        if packages[idx].installed {
                            let msg = alloc::format!("Remove {}?", packages[idx].name);
                            if show_confirm(&mut win, &theme, &msg) {
                                show_progress(&mut win, &theme, &alloc::format!("Removing {}...", packages[idx].name));
                                run_remove(&packages[idx].name);
                                packages = discover_packages();
                            }
                        } else {
                            let path = alloc::format!("/packages/{}.skp", packages[idx].name);
                            show_progress(&mut win, &theme, &alloc::format!("Installing {}...", packages[idx].name));
                            run_install(&path);
                            packages = discover_packages();
                        }
                        view = View::Browse;
                    }
                }
                _ => {}
            }
        }

        if mouse.scroll != 0 {
            if let View::Browse = view {
                let fi = filter_packages(&packages, search.text());
                let ms = (fi.len() as u32).saturating_sub(7) * 56;
                if mouse.scroll > 0 { scroll = scroll.saturating_sub(20); }
                else { scroll = (scroll + 20).min(ms); }
            }
        }

        win.draw_gradient_rect(0, 0, 700, 500, theme.bg_primary, theme.bg_surface, true);
        win.draw_string(20, 14, "Sarga Store", 0xFFFFFFFF, 0);
        search.render(&mut win, &theme);
        win.draw_line_h(10, 73, 680, theme.separator);

        match view {
            View::Browse => {
                let fi = filter_packages(&packages, search.text());
                if !fi.is_empty() && sel >= fi.len() { sel = fi.len() - 1; }

                for (i, &pi) in fi.iter().enumerate() {
                    let pkg = &packages[pi];
                    let ey = 80 + i as u32 * 56 - scroll;
                    if ey + 50 < 78 || ey > 475 { continue; }

                    let bg = if i == sel { theme.accent } else { theme.bg_surface };
                    win.draw_rounded_rect(10, ey, 680, 50, 8, bg);

                    let ic = hash_color(&pkg.name);
                    win.draw_rounded_rect(18, ey + 7, 36, 36, 8, ic);
                    if let Some(ch) = pkg.name.chars().next() {
                        let mut s = [0u8; 4];
                        let s = ch.encode_utf8(&mut s);
                        win.draw_string(31, ey + 18, s, 0xFFFFFFFF, 0);
                    }

                    win.draw_string(62, ey + 7, &pkg.name, theme.text, 0);
                    let desc = if pkg.description.len() > 42 {
                        alloc::format!("{}...", &pkg.description[..39])
                    } else {
                        pkg.description.clone()
                    };
                    win.draw_string(62, ey + 25, &desc, theme.text_secondary, 0);

                    let vw = pkg.version.len() as u32 * 8;
                    win.draw_rounded_rect(520, ey + 8, vw + 12, 18, 4, theme.bg_elevated);
                    win.draw_string(526, ey + 10, &pkg.version, theme.text_secondary, 0);

                    if pkg.installed {
                        win.draw_rounded_rect(610, ey + 10, 70, 30, 6, 0x60D32F2F);
                        win.draw_string(618, ey + 17, "Remove", 0xFFFFFFFF, 0);
                    } else {
                        win.draw_rounded_rect(610, ey + 10, 70, 30, 6, theme.accent);
                        win.draw_string(622, ey + 17, "Get", 0xFFFFFFFF, 0);
                    }
                }
                let s = alloc::format!("Showing {} packages", fi.len());
                win.draw_string(10, 485, &s, theme.text_secondary, 0);
            }
            View::Detail(idx) => {
                if idx >= packages.len() { view = View::Browse; }
                let pkg = &packages[idx];
                win.draw_rounded_rect(10, 80, 80, 28, 6, theme.bg_elevated);
                win.draw_string(22, 87, "<- Back", theme.text, 0);
                let ic = hash_color(&pkg.name);
                win.draw_rounded_rect(30, 120, 64, 64, 12, ic);
                if let Some(ch) = pkg.name.chars().next() {
                    let mut s = [0u8; 4];
                    let s = ch.encode_utf8(&mut s);
                    win.draw_string(54, 150, s, 0xFFFFFFFF, 0);
                }
                win.draw_string(110, 130, &pkg.name, 0xFFFFFFFF, 0);
                win.draw_string(110, 150, &pkg.version, theme.text_secondary, 0);
                win.draw_rounded_rect(30, 200, 640, 100, 8, theme.bg_elevated);
                win.draw_string_centered(248, "Screenshot", theme.text_disabled, 0);
                win.draw_string(30, 320, &pkg.description, theme.text, 0);
                if pkg.depends.is_empty() {
                    win.draw_string(30, 345, "No dependencies", theme.text_secondary, 0);
                } else {
                    let deps = pkg.depends.join(", ");
                    win.draw_string(30, 345, &alloc::format!("Deps: {}", deps), theme.text_secondary, 0);
                }
                let bx = (700 - 120) / 2;
                if pkg.installed {
                    win.draw_rounded_rect(bx, 420, 120, 36, 8, 0x60D32F2F);
                    win.draw_string(bx + 20, 428, "Remove", 0xFFFFFFFF, 0);
                } else {
                    win.draw_rounded_rect(bx, 420, 120, 36, 8, theme.accent);
                    win.draw_string(bx + 24, 428, "Install", 0xFFFFFFFF, 0);
                }
            }
        }

        let _ = win.flush();
        unsafe { libsarga::syscall::syscall1(35, 16_000_000u64); }
    }
}

sarga_main!(user_main);
