#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{sarga_main, gui::Window};
use libsarga::{io, args, fs};
use libsarga::io::{clipboard_read, clipboard_write};

struct Editor {
    lines: alloc::vec::Vec<alloc::string::String>,
    cursor_x: usize,
    cursor_y: usize,
    scroll_y: u32,
    modified: bool,
    filename: alloc::string::String,
    status: alloc::string::String,
    mode: EditMode,
}

#[derive(PartialEq)]
enum EditMode {
    Normal,
    Command,
}

impl Editor {
    fn new() -> Self {
        Self {
            lines: alloc::vec![alloc::string::String::new()],
            cursor_x: 0,
            cursor_y: 0,
            scroll_y: 0,
            modified: false,
            filename: alloc::string::String::new(),
            status: alloc::string::String::from("[New File]"),
            mode: EditMode::Normal,
        }
    }

    fn open(path: &str) -> Self {
        let mut ed = Self::new();
        ed.filename = alloc::string::String::from(path);
        match fs::read_to_string(path) {
            Ok(content) => {
                ed.lines.clear();
                for line in content.lines() {
                    ed.lines.push(alloc::string::String::from(line));
                }
                if ed.lines.is_empty() {
                    ed.lines.push(alloc::string::String::new());
                }
                ed.status = alloc::format!("[{}]", path);
                ed.modified = false;
            }
            Err(_) => {
                ed.status = alloc::format!("[New: {}]", path);
            }
        }
        ed
    }

    fn save(&mut self) {
        if self.filename.is_empty() {
            self.status = alloc::string::String::from("No filename!");
            return 0;
        }
        let mut content = alloc::string::String::new();
        for (i, line) in self.lines.iter().enumerate() {
            content.push_str(line);
            if i < self.lines.len() - 1 {
                content.push('\n');
            }
        }
        match fs::write_file(&self.filename, &content) {
            Ok(_) => {
                self.modified = false;
                self.status = alloc::format!("[Saved: {}]", self.filename);
            }
            Err(e) => {
                self.status = alloc::format!("[Error: {}]", e);
            }
        }
    }

    fn insert_char(&mut self, c: char) {
        if let Some(line) = self.lines.get_mut(self.cursor_y) {
            line.insert(self.cursor_x, c);
            self.cursor_x += 1;
            self.modified = true;
        }
    }

    fn delete_char(&mut self) {
        if self.cursor_x > 0 {
            if let Some(line) = self.lines.get_mut(self.cursor_y) {
                line.remove(self.cursor_x - 1);
                self.cursor_x -= 1;
                self.modified = true;
            }
        } else if self.cursor_y > 0 {
            let current_line = self.lines.remove(self.cursor_y);
            self.cursor_y -= 1;
            self.cursor_x = self.lines[self.cursor_y].len();
            self.lines[self.cursor_y].push_str(&current_line);
            self.modified = true;
        }
    }

    fn insert_newline(&mut self) {
        let rest = if let Some(line) = self.lines.get(self.cursor_y) {
            alloc::string::String::from(&line[self.cursor_x..])
        } else {
            alloc::string::String::new()
        };
        if let Some(line) = self.lines.get_mut(self.cursor_y) {
            line.truncate(self.cursor_x);
        }
        self.cursor_y += 1;
        self.cursor_x = 0;
        self.lines.insert(self.cursor_y, rest);
        self.modified = true;
    }

    fn handle_key(&mut self, key: u8) -> Option<i32> {
        match self.mode {
            EditMode::Normal => match key {
                b'j' => { self.cursor_y = (self.cursor_y + 1).min(self.lines.len() - 1); }
                b'k' => { self.cursor_y = self.cursor_y.saturating_sub(1); }
                b'h' => { self.cursor_x = self.cursor_x.saturating_sub(1); }
                b'l' => {
                    let max_x = self.lines.get(self.cursor_y).map_or(0, |l| l.len());
                    self.cursor_x = (self.cursor_x + 1).min(max_x);
                }
                b'0' => { self.cursor_x = 0; }
                b'$' => { self.cursor_x = self.lines.get(self.cursor_y).map_or(0, |l| l.len()); }
                b'i' => { self.mode = EditMode::Normal; /* just enter insert for simplicity */ }
                b'a' => {
                    let max_x = self.lines.get(self.cursor_y).map_or(0, |l| l.len());
                    self.cursor_x = (self.cursor_x + 1).min(max_x);
                }
                b'o' => {
                    let new_line = alloc::string::String::new();
                    self.lines.insert(self.cursor_y + 1, new_line);
                    self.cursor_y += 1;
                    self.cursor_x = 0;
                    self.modified = true;
                }
                b'O' => {
                    let new_line = alloc::string::String::new();
                    self.lines.insert(self.cursor_y, new_line);
                    self.cursor_x = 0;
                    self.modified = true;
                }
                b'x' => {
                    if let Some(line) = self.lines.get_mut(self.cursor_y) {
                        if self.cursor_x < line.len() {
                            line.remove(self.cursor_x);
                            self.modified = true;
                        }
                    }
                }
                b'd' => { self.status = alloc::string::String::from("Press dd to delete line"); }
                b':' => { self.mode = EditMode::Command; self.status = alloc::string::String::from(":"); }
                b'q' => {
                    if !self.modified { return Some(0); }
                    else { self.status = alloc::string::String::from("Use :q! to quit"); }
                }
                b'w' => { self.save(); }
                _ => {}
            },
            EditMode::Command => match key {
                0x0A | 0x0D => { return self.execute_command(); }
                0x7F | 0x08 => { self.status.pop(); }
                c if c.is_ascii_graphic() || c == b' ' => {
                    self.status.push(c as char);
                }
                _ => {}
            },
        }
        None
    }

    fn execute_command(&mut self) -> Option<i32> {
        let cmd = self.status.trim_start_matches(':').trim();
        let mut result = None;
        match cmd {
            "q" | "quit" => {
                if !self.modified { result = Some(0); }
                else { self.status = alloc::string::String::from("Modified! Use :q!"); }
            }
            "q!" => { result = Some(0); }
            "w" => { self.save(); }
            "wq" => { self.save(); result = Some(0); }
            s if s.starts_with("w ") => {
                self.filename = alloc::string::String::from(&s[2..]);
                self.save();
            }
            s if s.starts_with("e ") => {
                let path = &s[2..];
                *self = Editor::open(path);
            }
            _ => { self.status = alloc::format!("Unknown: {}", cmd); }
        }
        self.mode = EditMode::Normal;
        result
    }

    fn render(&self, win: &mut Window) {
        win.clear(0xFF1E1E1E);

        let line_y_start = 24u32;
        let char_h = 12u32;
        let max_lines = ((win.height - line_y_start - 20) / char_h) as usize;
        let line_x = 40u32;

        for i in 0..max_lines {
            let line_idx = i + self.scroll_y as usize;
            let y = line_y_start + i as u32 * char_h;
            if line_idx >= self.lines.len() { break; }

            let line_num_str = alloc::format!("{:>4} ", line_idx + 1);
            win.draw_string(2, y, &line_num_str, 0xFF555555, C_BG);

            let line = &self.lines[line_idx];
            let display = if line.len() > 80 { &line[..80] } else { line };
            draw_syntax_highlighted(win, line_x, y, display);

            if line_idx == self.cursor_y && self.cursor_x <= line.len() {
                let cx = line_x + self.cursor_x as u32 * 8;
                win.fill_rect(cx, y, 8, char_h, 0xFF0078D4);
                if let Some(c) = line.chars().nth(self.cursor_x) {
                    win.draw_char(cx, y, c, 0xFF1E1E1E, 0xFF0078D4);
                }
            }
        }

        win.draw_line_h(0, line_y_start - 2, win.width, 0xFF0078D4);

        let status_y = win.height - 18;
        win.fill_rect(0, status_y, win.width, 18, 0xFF2D2D2D);
        let mode_str = match self.mode {
            EditMode::Normal => "NORMAL",
            EditMode::Command => "COMMAND",
        };
        win.draw_string(4, status_y + 3, mode_str, 0xFF0078D4, 0xFF2D2D2D);
        win.draw_string(80, status_y + 3, &self.status, 0xFFCCCCCC, 0xFF2D2D2D);
        let pos = alloc::format!("Ln {}, Col {}", self.cursor_y + 1, self.cursor_x + 1);
        let pos_x = win.width - pos.len() as u32 * 8 - 8;
        win.draw_string(pos_x, status_y + 3, &pos, 0xFF888888, 0xFF2D2D2D);

        let title = if self.filename.is_empty() {
            alloc::string::String::from("SARGA Edit - [New]")
        } else if self.modified {
            alloc::format!("SARGA Edit - [{}] *", self.filename)
        } else {
            alloc::format!("SARGA Edit - [{}]", self.filename)
        };
        win.draw_string(8, 4, &title, 0xFFFFFFFF, 0xFF2D2D2D);

        win.draw_string(win.width - 160, 4, "Ctrl+C/V/X clipboard", 0xFF888888, 0xFF2D2D2D);
    }
}

const C_KW: u32 = 0xFF569CD6;
const C_STR: u32 = 0xFFCE9178;
const C_CMT: u32 = 0xFF6A9955;
const C_NUM: u32 = 0xFFB5CEA8;
const C_TY: u32 = 0xFF4EC9B0;
const C_DEF: u32 = 0xFFCCCCCC;
const C_BG: u32 = 0xFF1E1E1E;

fn is_kw(w: &str) -> bool {
    matches!(w, "fn"|"let"|"if"|"else"|"while"|"for"|"return"|"struct"|"enum"|"impl"
        |"pub"|"use"|"mod"|"match"|"break"|"continue"|"true"|"false"|"mut"|"const"
        |"static"|"extern"|"unsafe"|"trait"|"type"|"self"|"super"|"crate"|"where"
        |"as"|"ref"|"move"|"dyn"|"async"|"await"|"in"|"loop"
        |"int"|"char"|"void"|"ifdef"|"endif"|"include"|"define"|"typedef"|"union"
        |"sizeof"|"volatile"|"register"|"signed"|"unsigned"|"long"|"short")
}

fn is_ty(w: &str) -> bool {
    matches!(w, "u8"|"u16"|"u32"|"u64"|"i8"|"i16"|"i32"|"i64"|"bool"|"char"
        |"usize"|"isize"|"String"|"Vec"|"Option"|"Result"|"Box"|"Arc"|"Mutex"
        |"Rc"|"RefCell"|"HashMap"|"str"|"f32"|"f64")
}

fn draw_syntax_highlighted(win: &mut Window, x: u32, y: u32, line: &str) {
    // Quick check for line comment
    if let Some(cs) = line.find("//") {
        let code = &line[..cs];
        let cmt = &line[cs..];
        draw_tokens(win, x, y, code);
        win.draw_string(x + cs as u32 * 8, y, cmt, C_CMT, C_BG);
        return 0;
    }
    draw_tokens(win, x, y, line);
}

fn draw_tokens(win: &mut Window, x: u32, y: u32, s: &str) {
    let mut i = 0u32;
    let bytes = s.as_bytes();
    while (i as usize) < bytes.len() {
        let c = bytes[i as usize] as char;
        if c.is_whitespace() {
            win.draw_char(x + i * 8, y, c, C_DEF, C_BG);
            i += 1;
        } else if c == '"' {
            win.draw_char(x + i * 8, y, c, C_STR, C_BG);
            i += 1;
            while (i as usize) < bytes.len() && bytes[i as usize] as char != '"' {
                if bytes[i as usize] as char == '\\' {
                    win.draw_char(x + i * 8, y, bytes[i as usize] as char, C_STR, C_BG);
                    i += 1;
                    if (i as usize) < bytes.len() {
                        win.draw_char(x + i * 8, y, bytes[i as usize] as char, C_STR, C_BG);
                        i += 1;
                    }
                } else {
                    win.draw_char(x + i * 8, y, bytes[i as usize] as char, C_STR, C_BG);
                    i += 1;
                }
            }
            if (i as usize) < bytes.len() {
                win.draw_char(x + i * 8, y, bytes[i as usize] as char, C_STR, C_BG);
                i += 1;
            }
        } else if c.is_ascii_digit() {
            while (i as usize) < bytes.len() && ((bytes[i as usize] as char).is_alphanumeric() || bytes[i as usize] == b'.') {
                win.draw_char(x + i * 8, y, bytes[i as usize] as char, C_NUM, C_BG);
                i += 1;
            }
        } else if c.is_ascii_alphabetic() || c == '_' {
            let start = i as usize;
            i += 1;
            while (i as usize) < bytes.len() && ((bytes[i as usize] as char).is_alphanumeric() || bytes[i as usize] == b'_') {
                i += 1;
            }
            let word = &s[start..i as usize];
            let fg = if is_kw(word) { C_KW } else if is_ty(word) { C_TY } else { C_DEF };
            win.draw_string(x + start as u32 * 8, y, word, fg, C_BG);
        } else {
            win.draw_char(x + i * 8, y, c, C_DEF, C_BG);
            i += 1;
        }
    }
}

fn user_main() -> i32 {
    let mut ed = if args::argc() > 1 {
        let path = args::get(1).unwrap_or("");
        Editor::open(path)
    } else {
        Editor::new()
    };

    let mut win = match Window::create("SARGA Edit", 800, 600) {
        Ok(w) => w,
        Err(e) => {
            io::print_str(&alloc::format!("[sargaedit] failed: {}\n", e));
            return 0;
        }
    };

    loop {
        while let Some(key) = win.get_key() {
            match key {
                0x1B => { return 0; }
                0x7F | 0x08 => { ed.delete_char(); }
                0x0A | 0x0D => {
                    if ed.mode == EditMode::Command {
                        ed.handle_key(key);
                    } else {
                        ed.insert_newline();
                    }
                }
                // Ctrl+C: Copy current line to clipboard
                0x03 => {
                    if let Some(line) = ed.lines.get(ed.cursor_y) {
                        clipboard_write(line.as_bytes());
                        ed.status = alloc::format!("Copied line {}", ed.cursor_y + 1);
                    }
                }
                // Ctrl+V: Paste from clipboard
                0x16 => {
                    let mut buf = [0u8; 4096];
                    let n = clipboard_read(&mut buf);
                    if n > 0 {
                        if let Ok(text) = core::str::from_utf8(&buf[..n]) {
                            for ch in text.chars() {
                                if ch == '\n' {
                                    ed.insert_newline();
                                } else if ch.is_ascii() {
                                    ed.insert_char(ch);
                                }
                            }
                            ed.status = alloc::format!("Pasted {} bytes", n);
                        }
                    }
                }
                // Ctrl+X: Cut current line to clipboard
                0x18 => {
                    if let Some(line) = ed.lines.get(ed.cursor_y) {
                        clipboard_write(line.as_bytes());
                        if ed.lines.len() > 1 {
                            ed.lines.remove(ed.cursor_y);
                            if ed.cursor_y >= ed.lines.len() {
                                ed.cursor_y = ed.lines.len() - 1;
                            }
                            ed.cursor_x = 0;
                            ed.modified = true;
                            ed.status = alloc::format!("Cut line {}", ed.cursor_y + 1);
                        }
                    }
                }
                b'j' if ed.mode == EditMode::Normal => {
                    ed.cursor_y = (ed.cursor_y + 1).min(ed.lines.len() - 1);
                }
                b'k' if ed.mode == EditMode::Normal => {
                    ed.cursor_y = ed.cursor_y.saturating_sub(1);
                }
                b'h' if ed.mode == EditMode::Normal => {
                    ed.cursor_x = ed.cursor_x.saturating_sub(1);
                }
                b'l' if ed.mode == EditMode::Normal => {
                    let max_x = ed.lines.get(ed.cursor_y).map_or(0, |l| l.len());
                    ed.cursor_x = (ed.cursor_x + 1).min(max_x);
                }
                b'i' if ed.mode == EditMode::Normal => { ed.mode = EditMode::Normal; }
                b':' if ed.mode == EditMode::Normal => {
                    ed.mode = EditMode::Command;
                    ed.status = alloc::string::String::from(":");
                }
                b'w' if ed.mode == EditMode::Normal => { ed.save(); }
                b'q' if ed.mode == EditMode::Normal => {
                    if !ed.modified { return 0; }
                    else { ed.status = alloc::string::String::from("Modified! Use :q!"); }
                }
                c => {
                    if ed.mode == EditMode::Command || (c >= 0x20 && c < 0x7F) {
                        if c >= 0x20 && c < 0x7F {
                            ed.insert_char(c as char);
                        } else {
                            if let Some(code) = ed.handle_key(c) { return code; }
                        }
                    }
                }
            }

            let visible_lines = ((win.height - 44) / 12) as usize;
            if ed.cursor_y >= (ed.scroll_y as usize + visible_lines) {
                ed.scroll_y = (ed.cursor_y as u32 + 1).saturating_sub(visible_lines as u32);
            }
            if ed.cursor_y < ed.scroll_y as usize {
                ed.scroll_y = ed.cursor_y as u32;
            }
        }

        ed.render(&mut win);
        let _ = win.flush();
        unsafe { libsarga::syscall::syscall1(35, 16_000_000u64); }
    }
    0
}

sarga_main!(user_main);
