#![no_std]
#![no_main]
#![allow(unused_assignments)]
extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;
use libsarga::sarga_main;
use libsarga::gui::Window;
use libsarga::io::{self, getdents64, stat, clipboard_write};
use libsarga::theme::Theme;

struct Entry {
    name: String,
    is_dir: bool,
    size: u64,
}

struct FileManager {
    current_path: String,
    entries: Vec<Entry>,
    scroll: u32,
    selected: Option<usize>,
    hover: Option<usize>,
    status: String,
}

impl FileManager {
    fn new() -> Self {
        let mut fm = FileManager {
            current_path: String::from("/"),
            entries: Vec::new(),
            scroll: 0,
            selected: None,
            hover: None,
            status: String::new(),
        };
        fm.refresh();
        fm
    }

    fn refresh(&mut self) {
        self.entries.clear();
        self.scroll = 0;
        self.selected = None;

        // Add parent directory entry if not root
        if self.current_path != "/" {
            self.entries.push(Entry {
                name: String::from(".."),
                is_dir: true,
                size: 0,
            });
        }

        // Read directory entries via getdents64
        let fd = match io::open(&self.current_path, 0) {
            Ok(f) => f,
            Err(e) => {
                self.status = alloc::format!("open failed: {}", e);
                return;
            }
        };

        let mut buf = [0u8; 4096];
        loop {
            match getdents64(fd, &mut buf) {
                Ok(n) if n > 0 => {
                    let mut offset = 0;
                    while offset < n {
                        if offset + 19 > n { break; }
                        let ino = u64::from_ne_bytes(buf[offset..offset + 8].try_into().unwrap_or([0; 8]));
                        let _off = u64::from_ne_bytes(buf[offset + 8..offset + 16].try_into().unwrap_or([0; 8]));
                        let reclen = u16::from_ne_bytes(buf[offset + 16..offset + 18].try_into().unwrap_or([0; 2])) as usize;
                        let name_len = buf[offset + 18] as usize;
                        if reclen == 0 || offset + reclen > n { break; }
                        if ino != 0 && name_len > 0 && name_len < 256 {
                            let name_bytes = &buf[offset + 19..offset + 19 + name_len];
                            let name_end = name_bytes.iter().position(|&b| b == 0).unwrap_or(name_len);
                            if let Ok(name) = core::str::from_utf8(&name_bytes[..name_end]) {
                                if name != "." && name != ".." {
                                    let full_path = if self.current_path == "/" {
                                        alloc::format!("/{}", name)
                                    } else {
                                        alloc::format!("{}/{}", self.current_path, name)
                                    };
                                    let is_dir = stat(&full_path).map(|s| (s.mode & 0o170000) == 0o040000).unwrap_or(false);
                                    let size = stat(&full_path).map(|s| s.size).unwrap_or(0);
                                    self.entries.push(Entry {
                                        name: String::from(name),
                                        is_dir,
                                        size,
                                    });
                                }
                            }
                        }
                        offset += reclen;
                    }
                }
                _ => break,
            }
        }
        let _ = io::close(fd);

        // Sort: directories first, then alphabetically
        self.entries[1..].sort_by(|a, b| {
            if a.is_dir && !b.is_dir { core::cmp::Ordering::Less }
            else if !a.is_dir && b.is_dir { core::cmp::Ordering::Greater }
            else { a.name.cmp(&b.name) }
        });

        self.status = alloc::format!("{} items", self.entries.len().saturating_sub(1));
    }

    fn navigate(&mut self, name: &str) {
        if name == ".." {
            // Go to parent
            if let Some(pos) = self.current_path[..self.current_path.len() - 1].rfind('/') {
                self.current_path.truncate(pos + 1);
                if self.current_path.is_empty() { self.current_path.push('/'); }
            }
        } else {
            if self.current_path == "/" {
                self.current_path.push_str(name);
            } else {
                self.current_path.push_str(name);
            }
        }
        self.refresh();
    }
}

const ITEM_H: u32 = 22;
const HEADER_H: u32 = 28;
const PATH_BAR_H: u32 = 28;
const SIDEBAR_W: u32 = 140;

fn user_main() -> i32 {
    let theme = Theme::dark();
    let mut fm = FileManager::new();

    let win_w = 520u32;
    let win_h = 380u32;

    let mut win = match Window::create("SARGA Files", win_w, win_h) {
        Ok(w) => w,
        Err(e) => { io::print_str(&alloc::format!("skyfiles: window failed: {}\n", e)); return 0; }
    };

    let mut prev_pressed = false;

    loop {
        let mouse = win.get_mouse();
        let pressed = (mouse.buttons & 1) != 0;
        let mx = mouse.x as u32;
        let my = mouse.y as u32;
        let scroll_delta = mouse.scroll;

        // Scroll
        if scroll_delta > 0 {
            fm.scroll = fm.scroll.saturating_add(ITEM_H * scroll_delta as u32);
        } else if scroll_delta < 0 {
            fm.scroll = fm.scroll.saturating_sub(ITEM_H * (-scroll_delta) as u32);
        }

        // Click handling
        if pressed && !prev_pressed {
            let content_x = SIDEBAR_W;
            let content_y = PATH_BAR_H + HEADER_H;

            // Path bar back button
            if mx >= 4 && mx < 36 && my >= 4 && my < PATH_BAR_H - 4 {
                if fm.current_path != "/" {
                    if let Some(pos) = fm.current_path[..fm.current_path.len() - 1].rfind('/') {
                        fm.current_path.truncate(pos + 1);
                        if fm.current_path.is_empty() { fm.current_path.push('/'); }
                    }
                    fm.refresh();
                }
            }

            // File list clicks
            if mx >= content_x && my >= content_y {
                let idx = ((my - content_y + fm.scroll) / ITEM_H) as usize;
                if idx < fm.entries.len() {
                    fm.selected = Some(idx);
                    // Double-click to navigate (use click count approximation)
                    if fm.hover == Some(idx) {
                        fm.navigate(&fm.entries[idx].name.clone());
                    }
                    fm.hover = Some(idx);
                }
            }
        }
        if !pressed { prev_pressed = false; }

        // Hover tracking
        if mx >= SIDEBAR_W && my >= PATH_BAR_H + HEADER_H {
            let idx = ((my - PATH_BAR_H - HEADER_H + fm.scroll) / ITEM_H) as usize;
            if idx < fm.entries.len() {
                fm.hover = Some(idx);
            }
        } else {
            fm.hover = None;
        }

        // Render
        win.clear(theme.bg_primary);

        // Path bar background
        win.draw_rect(0, 0, win_w, PATH_BAR_H, theme.bg_surface);

        // Back button
        let back_bg = if mx >= 4 && mx < 36 && my >= 4 && my < PATH_BAR_H - 4 { theme.hover } else { theme.bg_elevated };
        win.draw_rounded_rect(4, 4, 32, PATH_BAR_H - 8, 4, back_bg);
        win.draw_string(10, 8, "<-", theme.text, 0);

        // Path display
        win.draw_string(42, 8, &fm.current_path, theme.text, 0);

        // Separator
        win.draw_line_h(0, PATH_BAR_H, win_w, theme.border);

        // Column headers
        win.draw_rect(0, PATH_BAR_H, win_w, HEADER_H, theme.bg_surface);
        win.draw_string(SIDEBAR_W + 8, PATH_BAR_H + 6, "Name", theme.text_secondary, 0);
        win.draw_string(win_w - 80, PATH_BAR_H + 6, "Size", theme.text_secondary, 0);
        win.draw_line_h(0, PATH_BAR_H + HEADER_H, win_w, theme.border);

        // Sidebar (bookmarks)
        win.draw_rect(0, PATH_BAR_H + HEADER_H, SIDEBAR_W, win_h - PATH_BAR_H - HEADER_H, theme.bg_surface);
        win.draw_line_v(SIDEBAR_W, PATH_BAR_H + HEADER_H, win_h - PATH_BAR_H - HEADER_H, theme.border);

        let bookmarks = ["/", "/bin", "/dev", "/etc", "/home", "/tmp", "/usr"];
        for (i, path) in bookmarks.iter().enumerate() {
            let by = PATH_BAR_H + HEADER_H + 4 + i as u32 * 24;
            let is_active = fm.current_path == *path;
            let bg = if is_active { theme.accent } else if mx < SIDEBAR_W && my >= by && my < by + 22 { theme.hover } else { 0 };
            if bg != 0 { win.draw_rect(2, by, SIDEBAR_W - 4, 22, bg); }
            let tc = if is_active { 0xFFFFFFFF } else { theme.text_secondary };
            win.draw_string(8, by + 4, path, tc, 0);
        }

        // File list
        let content_x = SIDEBAR_W;
        let content_y = PATH_BAR_H + HEADER_H;
        let content_w = win_w - SIDEBAR_W;
        let max_visible = ((win_h - content_y) / ITEM_H) as usize;

        for (i, entry) in fm.entries.iter().enumerate() {
            let iy = content_y + i as u32 * ITEM_H - fm.scroll;
            if iy < content_y || iy + ITEM_H > win_h { continue; }
            if i > max_visible + fm.scroll as usize / ITEM_H as usize + 1 { break; }

            // Selection/hover highlight
            if fm.selected == Some(i) {
                win.draw_rect(content_x, iy, content_w, ITEM_H, theme.accent);
            } else if fm.hover == Some(i) {
                win.draw_rect(content_x, iy, content_w, ITEM_H, theme.hover);
            }

            // Icon
            let icon = if entry.is_dir { ">>" } else { "  " };
            let icon_color = if entry.is_dir { theme.accent } else { theme.text_secondary };
            win.draw_string(content_x + 8, iy + 4, icon, icon_color, 0);

            // Name
            let display_name = if entry.name.len() > 40 { &entry.name[..40] } else { &entry.name };
            win.draw_string(content_x + 28, iy + 4, display_name, theme.text, 0);

            // Size
            if !entry.is_dir {
                let size_str = if entry.size < 1024 {
                    alloc::format!("{} B", entry.size)
                } else if entry.size < 1024 * 1024 {
                    alloc::format!("{} KB", entry.size / 1024)
                } else {
                    alloc::format!("{} MB", entry.size / (1024 * 1024))
                };
                win.draw_string(win_w - 80, iy + 4, &size_str, theme.text_disabled, 0);
            } else {
                win.draw_string(win_w - 30, iy + 4, "DIR", theme.accent, 0);
            }
        }

        // Status bar
        win.draw_rect(0, win_h - 20, win_w, 20, theme.bg_surface);
        win.draw_line_h(0, win_h - 20, win_w, theme.border);
        win.draw_string(8, win_h - 16, &fm.status, theme.text_disabled, 0);

        // Scrollbar (if needed)
        let total_h = fm.entries.len() as u32 * ITEM_H;
        let view_h = win_h - content_y;
        if total_h > view_h {
            let sb_x = win_w - 8;
            let sb_h = view_h;
            let thumb_h = (view_h as f32 / total_h as f32 * sb_h as f32) as u32;
            let thumb_y = (fm.scroll as f32 / total_h as f32 * sb_h as f32) as u32;
            win.draw_rect(sb_x, content_y, 6, sb_h, theme.bg_elevated);
            win.draw_rect(sb_x, content_y + thumb_y, 6, thumb_h.max(10), theme.text_disabled);
        }

        // Keyboard shortcuts
        while let Some(key) = win.get_key() {
            match key {
                b'q' | b'Q' => return 0,
                b'r' | b'R' => fm.refresh(),
                b'/' => { fm.current_path = String::from("/"); fm.refresh(); }
                0x08 => { // Backspace = parent dir
                    if fm.current_path != "/" {
                        if let Some(pos) = fm.current_path[..fm.current_path.len() - 1].rfind('/') {
                            fm.current_path.truncate(pos + 1);
                            if fm.current_path.is_empty() { fm.current_path.push('/'); }
                        }
                        fm.refresh();
                    }
                }
                0x0A | 0x0D => { // Enter = navigate into selected
                    if let Some(idx) = fm.selected {
                        if fm.entries[idx].is_dir {
                            fm.navigate(&fm.entries[idx].name.clone());
                        }
                    }
                }
                // Ctrl+C: Copy selected file path to clipboard
                0x03 => {
                    if let Some(idx) = fm.selected {
                        let full_path = if fm.current_path == "/" {
                            alloc::format!("/{}", fm.entries[idx].name)
                        } else {
                            alloc::format!("{}/{}", fm.current_path, fm.entries[idx].name)
                        };
                        clipboard_write(full_path.as_bytes());
                        fm.status = alloc::format!("Copied: {}", full_path);
                    }
                }
                _ => {}
            }
        }

        let _ = win.flush();
        prev_pressed = pressed;
        unsafe { libsarga::syscall::syscall1(35, 16_666_000); }
    }
}

sarga_main!(user_main);
