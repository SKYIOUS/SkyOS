#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{sarga_main, gui::Window, theme::Theme, io, fs};
use alloc::string::{String, ToString};
use alloc::vec::Vec;

fn load_index() -> Vec<(String, String, u64, bool)> {
    let mut entries = Vec::new();
    let data = match io::read_to_string("/tmp/search.idx") {
        Ok(s) => s,
        Err(_) => {
            let _ = walk_dir("/", &mut entries, 0);
            return entries;
        }
    };
    for line in data.lines() {
        let parts: Vec<&str> = line.splitn(4, '|').collect();
        if parts.len() >= 3 {
            let name = parts[0].to_string();
            let path = parts[1].to_string();
            let size = parts[2].parse().unwrap_or(0);
            let is_dir = parts.get(3).map(|s| *s == "1").unwrap_or(false);
            entries.push((name, path, size, is_dir));
        }
    }
    entries
}

fn walk_dir(path: &str, entries: &mut Vec<(String, String, u64, bool)>, depth: usize) -> Result<(), ()> {
    if depth > 6 { return Ok(()); }
    if path.len() > 256 { return Ok(()); }
    let fd = match io::open(path, 0) { Ok(f) => f, Err(_) => return Ok(()) };
    let mut buf = [0u8; 4096];
    let n = match io::getdents64(fd, &mut buf) { Ok(n) => n, Err(_) => { let _ = io::close(fd); return Ok(()); } };
    let _ = io::close(fd);

    let mut off = 0;
    while off < n {
        if off + 19 > n { break; }
        let reclen = u16::from_le_bytes(buf[off+16..off+18].try_into().unwrap_or([0; 2])) as usize;
        let d_type = buf[off+18];
        if reclen < 19 || off + reclen > n { break; }
        let name_bytes = &buf[off+19..off+reclen];
        let name_end = name_bytes.iter().position(|&b| b == 0).unwrap_or(name_bytes.len());
        let name = core::str::from_utf8(&name_bytes[..name_end]).unwrap_or("");
        if name.is_empty() || name == "." || name == ".." { off += reclen; continue; }

        let full = if path == "/" { alloc::format!("/{}", name) } else { alloc::format!("{}/{}", path, name) };
        let is_dir = d_type == 4;
        let size = fs::stat(&full).map(|s| s.size as u64).unwrap_or(0);
        let full_clone = full.clone();
        entries.push((name.to_string(), full_clone, size, is_dir));

        if is_dir && depth < 6 {
            let _ = walk_dir(&full, entries, depth + 1);
        }
        off += reclen;
    }
    Ok(())
}

fn user_main() -> i32 {
    let mut win = Window::create("Sarga Search", 500, 400).expect("Window::create failed");
    let theme = Theme::dark();
    let mut query = String::new();
    let mut results: Vec<(String, String, u64, bool)> = Vec::new();
    let mut all_entries: Vec<(String, String, u64, bool)> = load_index();
    let mut prev_query = String::new();
    let mut frame = 0u32;

    loop {
        win.fill(theme.bg_surface);

        // Search bar
        win.draw_rounded_rect(10, 10, 480, 40, 8, theme.bg_primary);
        win.draw_rounded_rect_outline(10, 10, 480, 40, 8, theme.accent);

        let text_x = 20u32;
        if query.is_empty() {
            win.draw_string(text_x, 22, "Search files, apps, and more... (Super+Space)", theme.text_secondary, 1);
        } else {
            win.draw_string(text_x, 22, &query, theme.text, 0);
            frame += 1;
            if frame % 30 < 15 {
                let cx = text_x + query.len() as u32 * 8;
                win.fill_rect(cx, 20, 2, 20, theme.accent);
            }
        }

        // Filter results
        if query != prev_query {
            prev_query = query.clone();
            results.clear();
            if !query.is_empty() {
                let q = query.to_lowercase();
                let mut count = 0;
                for (name, path, size, is_dir) in &all_entries {
                    if name.to_lowercase().contains(&q) || path.to_lowercase().contains(&q) {
                        if count < 20 {
                            results.push((name.clone(), path.clone(), *size, *is_dir));
                            count += 1;
                        }
                    }
                }
            }
        }

        // Results count
        win.draw_string(15, 65, &alloc::format!("{} results", results.len()), theme.text_secondary, 1);

        // Result items
        let mut y = 70u32;
        for (name, path, size, is_dir) in &results {
            y += 28;
            if y + 28 > 360 { break; }

            if *is_dir {
                win.draw_rect(20, y, 20, 20, 0xFFD4A017);
                win.draw_rect(20, y.wrapping_sub(3), 8, 7, 0xFFE8B830);
            } else {
                win.draw_rect(22, y + 2, 16, 18, 0xFF555577);
            }

            win.draw_string(48, y + 2, name, theme.text, 0);

            let path_display = if path.len() > 40 {
                alloc::format!("...{}", &path[path.len()-37..])
            } else { path.clone() };
            win.draw_string(48, y + 14, &path_display, theme.text_secondary, 1);

            if !*is_dir {
                let size_str = if *size > 1_000_000 {
                    alloc::format!("{} MB", size / 1_000_000)
                } else if *size > 1_000 {
                    alloc::format!("{} KB", size / 1_000)
                } else {
                    alloc::format!("{} B", size)
                };
                let sx = 490u32.wrapping_sub(size_str.len() as u32 * 8);
                win.draw_string(sx, y + 2, &size_str, theme.text_secondary, 1);
            }
        }

        let _ = win.flush();

        // Keyboard input
        while let Some(c) = win.get_key() {
            match c {
                0x7F | 0x08 => { query.pop(); }
                0x0D | 0x0A => {
                    if let Some((_, _, _, _)) = results.first() {
                    }
                }
                c if c >= 0x20 && c <= 0x7E => { query.push(c as char); }
                _ => {}
            }
        }

        if frame % 500 == 0 {
            all_entries = load_index();
        }

        unsafe { libsarga::syscall::syscall2(35, 0, 16_000_000u64); }
    }
}

sarga_main!(user_main);
