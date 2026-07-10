use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;
use alloc::ffi::CString;
use alloc::format;
use libsarga::println;

pub struct History {
    entries: Vec<String>,
    pos: usize,
    max: usize,
}

impl History {
    pub fn new(max: usize) -> Self {
        let mut h = History { entries: Vec::new(), pos: 0, max };
        h.load();
        h
    }

    pub fn add(&mut self, line: &str) {
        let trimmed = line.trim();
        if trimmed.is_empty() { return; }
        if self.entries.last().map_or(false, |e| e == trimmed) { return; }
        self.entries.push(trimmed.to_string());
        if self.entries.len() > self.max {
            self.entries.remove(0);
        }
        self.pos = self.entries.len();
    }

    pub fn prev(&mut self) -> Option<&str> {
        if self.pos == 0 { return None; }
        self.pos -= 1;
        self.entries.get(self.pos).map(|s| s.as_str())
    }

    pub fn next(&mut self) -> Option<&str> {
        if self.pos >= self.entries.len() { return None; }
        self.pos += 1;
        if self.pos >= self.entries.len() { return None; }
        self.entries.get(self.pos).map(|s| s.as_str())
    }

    pub fn search(&self, prefix: &str) -> Option<String> {
        for entry in self.entries.iter().rev() {
            if entry.starts_with(prefix) {
                return Some(entry.clone());
            }
        }
        self.entries.last().cloned()
    }

    pub fn print(&self, n: usize) {
        let start = if n >= self.entries.len() { 0 } else { self.entries.len() - n };
        for i in start..self.entries.len() {
            println!("  {}  {}", i + 1, self.entries[i]);
        }
    }

    fn load(&mut self) {
        let home = crate::get_env("HOME").unwrap_or_else(|| String::from("/"));
        let path = format!("{}/.sash_history", home);
        let c_str = CString::new(path.as_bytes()).ok();
        if c_str.is_none() { return; }
        let fd = unsafe { libsarga::syscall::syscall2(2, c_str.unwrap().as_ptr() as u64, 0u64) };
        if fd < 0 { return; }
        let mut buf = [0u8; 4096];
        let mut content = String::new();
        loop {
            let n = libsarga::io::read(fd, &mut buf).unwrap_or(0);
            if n == 0 { break; }
            content.push_str(core::str::from_utf8(&buf[..n]).unwrap_or(""));
        }
        let _ = unsafe { libsarga::syscall::syscall1(3, fd as u64) };
        for line in content.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                self.entries.push(trimmed.to_string());
            }
        }
    }

    #[allow(dead_code)]
    pub fn save(&self) {
        let home = crate::get_env("HOME").unwrap_or_else(|| String::from("/"));
        let path = format!("{}/.sash_history", home);
        let c_str = CString::new(path.as_bytes()).ok();
        if c_str.is_none() { return; }
        let fd = unsafe { libsarga::syscall::syscall2(2, c_str.unwrap().as_ptr() as u64, 0x241u64) };
        if fd < 0 { return; }
        for entry in &self.entries {
            let mut line = entry.clone();
            line.push('\n');
            let _ = libsarga::io::write(fd, line.as_bytes());
        }
        let _ = unsafe { libsarga::syscall::syscall1(3, fd as u64) };
    }
}

struct Completer;

impl Completer {
    fn complete(&self, prefix: &str) -> Vec<String> {
        let mut matches = Vec::new();
        // Search builtins
        if crate::builtins::matches_builtin(prefix) {
            matches.push(prefix.to_string());
        }
        // Search PATH
        let path = crate::get_env("PATH").unwrap_or_else(|| String::from("/bin"));
        for dir in path.split(':') {
            self.list_dir(dir, prefix, &mut matches);
        }
        // Search current dir for files (if prefix doesn't start with /)
        if !prefix.starts_with('/') {
            self.list_dir(".", prefix, &mut matches);
        }
        matches.sort();
        matches.dedup();
        matches
    }

    fn list_dir(&self, dir: &str, prefix: &str, matches: &mut Vec<String>) {
        let c_str = CString::new(dir.as_bytes()).ok();
        if c_str.is_none() { return; }
        let fd = unsafe { libsarga::syscall::syscall2(257, c_str.unwrap().as_ptr() as u64, 0x100000u64) };
        if fd < 0 { return; }
        let mut buf = [0u8; 4096];
        loop {
            let n = unsafe { libsarga::syscall::syscall3(217, fd as u64, buf.as_mut_ptr() as u64, 4096u64) };
            if n <= 0 { break; }
            let mut offset = 0;
            while offset < n as usize {
                let reclen_bytes = &buf[offset+16..offset+18];
                let reclen = u16::from_ne_bytes([reclen_bytes[0], reclen_bytes[1]]) as usize;
                let namelen = &buf[offset+18..offset+20];
                let namelen = u16::from_ne_bytes([namelen[0], namelen[1]]) as usize;
                let name = core::str::from_utf8(&buf[offset+20..offset+20+namelen]).unwrap_or("");
                if name.starts_with(prefix) && name != "." && name != ".." {
                    matches.push(name.to_string());
                }
                offset += reclen;
                if reclen == 0 { break; }
            }
        }
        let _ = unsafe { libsarga::syscall::syscall1(3, fd as u64) };
    }
}

pub fn read_line(history: &mut History, prompt: &str) -> String {
    libsarga::print!("{}", prompt);
    let mut buf = [0u8; 4096];
    let mut input = String::new();
    let mut cursor: usize = 0;
    let completer = Completer;

    loop {
        let n = unsafe { libsarga::syscall::syscall3(0, 0u64, buf.as_mut_ptr() as u64, 4096u64) };
        if n <= 0 { break; }
        let mut i = 0;
        while i < n as usize {
            let c = buf[i];
            match c {
                b'\n' => {
                    libsarga::print!("\n");
                    history.add(&input);
                    if input.starts_with('!') {
                        if input == "!!" {
                            if let Some(last) = history.search("") {
                                input = last;
                                libsarga::print!("{}\n", input);
                            }
                        } else if let Ok(n) = input[1..].parse::<usize>() {
                            if n > 0 && n <= history.entries.len() {
                                input = history.entries[n - 1].clone();
                                libsarga::print!("{}\n", input);
                            }
                        }
                    }
                    return input;
                }
                0x7f | 0x08 => { // backspace or DEL
                    if cursor > 0 {
                        cursor -= 1;
                        input.remove(cursor);
                        redraw_line(prompt, &input, cursor);
                    }
                }
                0x09 => { // TAB
                    let matches = completer.complete(&input);
                    if matches.len() == 1 {
                        input = matches[0].clone();
                        cursor = input.len();
                        redraw_line(prompt, &input, cursor);
                    } else if !matches.is_empty() {
                        libsarga::print!("\n");
                        let mut line = String::new();
                        for m in &matches { line.push_str(m); line.push(' '); }
                        if line.len() > 60 { line.truncate(60); line.push_str("..."); }
                        libsarga::print!("{}\n", line);
                        redraw_line(prompt, &input, cursor);
                    }
                }
                0x1b => { // ESC sequences
                    if i + 2 < n as usize && buf[i+1] == b'[' {
                        match buf[i+2] {
                            b'A' => { // Up
                                if let Some(prev) = history.prev() {
                                    input = prev.to_string();
                                    cursor = input.len();
                                    redraw_line(prompt, &input, cursor);
                                }
                            }
                            b'B' => { // Down
                                if let Some(next) = history.next() {
                                    input = next.to_string();
                                    cursor = input.len();
                                    redraw_line(prompt, &input, cursor);
                                } else {
                                    input.clear();
                                    cursor = 0;
                                    redraw_line(prompt, &input, cursor);
                                }
                            }
                            b'C' => { // Right arrow
                                if cursor < input.len() {
                                    cursor += 1;
                                    libsarga::print!("\x1b[C");
                                }
                            }
                            b'D' => { // Left arrow
                                if cursor > 0 {
                                    cursor -= 1;
                                    libsarga::print!("\x1b[D");
                                }
                            }
                            b'H' | b'1' => { // Home
                                if cursor > 0 {
                                    libsarga::print!("\x1b[{}D", cursor);
                                    cursor = 0;
                                }
                            }
                            b'F' | b'4' => { // End
                                let dist = input.len() - cursor;
                                if dist > 0 {
                                    libsarga::print!("\x1b[{}C", dist);
                                    cursor = input.len();
                                }
                            }
                            b'3' => { // Delete
                                if cursor < input.len() {
                                    input.remove(cursor);
                                    redraw_line(prompt, &input, cursor);
                                }
                            }
                            _ => {}
                        }
                        i += 2;
                    }
                }
                0x03 => { // Ctrl+C
                    libsarga::print!("\n");
                    return String::new();
                }
                0x04 => { // Ctrl+D
                    if input.is_empty() {
                        return String::new();
                    }
                }
                _ => {
                    if c >= 0x20 && c <= 0x7e {
                        input.insert(cursor, c as char);
                        cursor += 1;
                        redraw_line(prompt, &input, cursor);
                    }
                }
            }
            i += 1;
        }
    }
    input
}

fn redraw_line(prompt: &str, input: &str, cursor: usize) {
    libsarga::print!("\r{} {}", prompt, input);
    // Clear to end of line
    libsarga::print!("\x1b[K");
    // Move cursor back to correct position
    let after_prompt = prompt.len() + 1 + cursor;
    let current_pos = prompt.len() + 1 + input.len();
    if current_pos > after_prompt {
        libsarga::print!("\x1b[{}D", current_pos - after_prompt);
    }
}
