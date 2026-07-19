use alloc::ffi::CString;
use alloc::format;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;
use libsarga::print;
use libsarga::println;

pub struct History {
    entries: Vec<String>,
    pos: usize,
    max: usize,
}

impl History {
    pub fn new(max: usize) -> Self {
        let mut h = History {
            entries: Vec::new(),
            pos: 0,
            max,
        };
        h.load();
        h
    }

    pub fn add(&mut self, line: &str) {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return;
        }
        if self.entries.last().map_or(false, |e| e == trimmed) {
            return;
        }
        self.entries.push(trimmed.to_string());
        if self.entries.len() > self.max {
            self.entries.remove(0);
        }
        self.pos = self.entries.len();
    }

    pub fn prev(&mut self) -> Option<&str> {
        if self.pos == 0 {
            return None;
        }
        self.pos -= 1;
        self.entries.get(self.pos).map(|s| s.as_str())
    }

    pub fn next(&mut self) -> Option<&str> {
        if self.pos >= self.entries.len() {
            return None;
        }
        self.pos += 1;
        if self.pos >= self.entries.len() {
            return None;
        }
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

    pub fn search_containing(&self, needle: &str) -> Option<(usize, String)> {
        for (i, entry) in self.entries.iter().rev().enumerate() {
            if entry.contains(needle) {
                return Some((self.entries.len() - 1 - i, entry.clone()));
            }
        }
        None
    }

    pub fn print(&self, n: usize) {
        let start = if n >= self.entries.len() {
            0
        } else {
            self.entries.len() - n
        };
        for i in start..self.entries.len() {
            println!("  {}  {}", i + 1, self.entries[i]);
        }
    }

    pub fn entries_rev(&self) -> impl Iterator<Item = &String> {
        self.entries.iter().rev()
    }

    fn load(&mut self) {
        let home = crate::get_env("HOME").unwrap_or_else(|| String::from("/"));
        let path = format!("{}/.sash_history", home);
        let c_str = match CString::new(path.as_bytes()) {
            Ok(c) => c,
            Err(_) => return,
        };
        let fd = unsafe { libsarga::syscall::syscall2(2, c_str.as_ptr() as u64, 0u64) };
        if fd < 0 {
            return;
        }
        let mut buf = [0u8; 4096];
        let mut content = String::new();
        loop {
            let n = libsarga::io::read(fd, &mut buf).unwrap_or(0);
            if n == 0 {
                break;
            }
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
        let c_str = match CString::new(path.as_bytes()) {
            Ok(c) => c,
            Err(_) => return,
        };
        let fd = unsafe { libsarga::syscall::syscall2(2, c_str.as_ptr() as u64, 0x241u64) };
        if fd < 0 {
            return;
        }
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
        if crate::builtins::matches_builtin(prefix) {
            matches.push(prefix.to_string());
        }
        let path = crate::get_env("PATH").unwrap_or_else(|| String::from("/bin"));
        for dir in path.split(':') {
            self.list_dir(dir, prefix, &mut matches);
        }
        if !prefix.starts_with('/') {
            self.list_dir(".", prefix, &mut matches);
        }
        matches.sort();
        matches.dedup();
        matches
    }

    fn list_dir(&self, dir: &str, prefix: &str, matches: &mut Vec<String>) {
        let c_str = match CString::new(dir.as_bytes()) {
            Ok(c) => c,
            Err(_) => return,
        };
        let fd = unsafe { libsarga::syscall::syscall2(257, c_str.as_ptr() as u64, 0x100000u64) };
        if fd < 0 {
            return;
        }
        let mut buf = [0u8; 4096];
        loop {
            let n = unsafe {
                libsarga::syscall::syscall3(217, fd as u64, buf.as_mut_ptr() as u64, 4096u64)
            };
            if n <= 0 {
                break;
            }
            let mut offset = 0;
            while offset < n as usize {
                let reclen_bytes = &buf[offset + 16..offset + 18];
                let reclen = u16::from_ne_bytes([reclen_bytes[0], reclen_bytes[1]]) as usize;
                let namelen = &buf[offset + 18..offset + 20];
                let namelen = u16::from_ne_bytes([namelen[0], namelen[1]]) as usize;
                let name =
                    core::str::from_utf8(&buf[offset + 20..offset + 20 + namelen]).unwrap_or("");
                if name.starts_with(prefix) && name != "." && name != ".." {
                    matches.push(name.to_string());
                }
                offset += reclen;
                if reclen == 0 {
                    break;
                }
            }
        }
        let _ = unsafe { libsarga::syscall::syscall1(3, fd as u64) };
    }

    fn suggest_first_word(&self, prefix: &str) -> Option<String> {
        if crate::builtins::matches_builtin(prefix) {
            return Some(prefix.to_string());
        }
        let path = crate::get_env("PATH").unwrap_or_else(|| String::from("/bin"));
        for dir in path.split(':') {
            let mut v = Vec::new();
            self.list_dir(dir, prefix, &mut v);
            if let Some(m) = v.into_iter().next() {
                return Some(m);
            }
        }
        if !prefix.starts_with('/') {
            let mut v = Vec::new();
            self.list_dir(".", prefix, &mut v);
            if let Some(m) = v.into_iter().next() {
                return Some(m);
            }
        }
        None
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Insert,
    Command,
    Visual,
}

fn is_word_sep(b: u8) -> bool {
    matches!(b, b' ' | b'\t')
}

fn is_word_char(b: u8) -> bool {
    !is_word_sep(b)
}

fn motion_word_right(input: &str, cursor: usize) -> usize {
    let bytes = input.as_bytes();
    if cursor >= bytes.len() {
        return cursor;
    }
    let mut j = cursor;
    if is_word_char(bytes[j]) {
        while j < bytes.len() && is_word_char(bytes[j]) {
            j += 1;
        }
    }
    while j < bytes.len() && is_word_sep(bytes[j]) {
        j += 1;
    }
    j
}

fn motion_word_left(input: &str, cursor: usize) -> usize {
    let bytes = input.as_bytes();
    if cursor == 0 {
        return 0;
    }
    let mut j = cursor;
    while j > 0 && is_word_sep(bytes[j - 1]) {
        j -= 1;
    }
    while j > 0 && is_word_char(bytes[j - 1]) {
        j -= 1;
    }
    j
}

fn motion_end_word(input: &str, cursor: usize) -> usize {
    let bytes = input.as_bytes();
    if cursor >= bytes.len() {
        return bytes.len();
    }
    let mut j = cursor;
    while j < bytes.len() && is_word_sep(bytes[j]) {
        j += 1;
    }
    while j + 1 < bytes.len() && is_word_char(bytes[j + 1]) {
        j += 1;
    }
    if j < bytes.len() && is_word_char(bytes[j]) {
        j
    } else {
        cursor
    }
}

fn motion_first_nonws(input: &str) -> usize {
    let bytes = input.as_bytes();
    let mut j = 0;
    while j < bytes.len() && is_word_sep(bytes[j]) {
        j += 1;
    }
    j
}

fn motion_word_end_big(input: &str, cursor: usize) -> usize {
    let bytes = input.as_bytes();
    if cursor >= bytes.len() {
        return bytes.len();
    }
    let mut j = cursor;
    while j < bytes.len() && !(bytes[j] == b' ' || bytes[j] == b'\t') {
        j += 1;
    }
    if j > cursor {
        j - 1
    } else {
        cursor
    }
}

fn motion_find_char(input: &str, cursor: usize, c: u8, dir: bool, before: bool) -> Option<usize> {
    let bytes = input.as_bytes();
    if dir {
        let start = cursor + 1;
        for i in start..bytes.len() {
            if bytes[i] == c {
                return Some(if before && i > 0 { i - 1 } else { i });
            }
        }
    } else {
        let mut i = cursor.wrapping_sub(1);
        loop {
            if bytes[i] == c {
                return Some(if before { i + 1 } else { i });
            }
            if i == 0 {
                break;
            }
            i -= 1;
        }
    }
    None
}

fn motion_match_bracket(input: &str, cursor: usize) -> Option<usize> {
    let bytes = input.as_bytes();
    if cursor >= bytes.len() {
        return None;
    }
    let open = bytes[cursor];
    let (op, cl) = match open {
        b'(' => (b'(', b')'),
        b')' => (b')', b'('),
        b'{' => (b'{', b'}'),
        b'}' => (b'}', b'{'),
        b'[' => (b'[', b']'),
        b']' => (b']', b'['),
        _ => return None,
    };
    if op == b'(' || op == b'{' || op == b'[' {
        let mut depth = 1i32;
        for i in (cursor + 1)..bytes.len() {
            if bytes[i] == cl {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            } else if bytes[i] == op {
                depth += 1;
            }
        }
    } else {
        let mut depth = 1i32;
        let mut i = cursor.wrapping_sub(1);
        loop {
            if bytes[i] == cl {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            } else if bytes[i] == op {
                depth += 1;
            }
            if i == 0 {
                break;
            }
            i -= 1;
        }
    }
    None
}

fn recompute_suggestion(input: &str, history: &History, completer: &Completer) -> Option<String> {
    if input.is_empty() {
        return None;
    }
    for entry in history.entries_rev() {
        if entry.len() > input.len() && entry.starts_with(input) {
            return Some(entry[input.len()..].to_string());
        }
    }
    if let Some(last) = input.bytes().last() {
        if !is_word_sep(last) {
            let bytes = input.as_bytes();
            let mut start = bytes.len();
            while start > 0 && !is_word_sep(bytes[start - 1]) {
                start -= 1;
            }
            let word = &input[start..];
            if is_command_position(input, start) {
                if let Some(full) = completer.suggest_first_word(word) {
                    if full.len() > word.len() && full.starts_with(word) {
                        return Some(full[word.len()..].to_string());
                    }
                }
            }
        }
    }
    None
}

fn is_command_position(input: &str, start: usize) -> bool {
    if start == 0 {
        return true;
    }
    let prefix = &input[..start];
    let trimmed = prefix.trim_end();
    if trimmed.is_empty() {
        return true;
    }
    let ends_with_op = trimmed.ends_with('|')
        || trimmed.ends_with(';')
        || trimmed.ends_with("&&")
        || trimmed.ends_with("||");
    ends_with_op
}

// ---- Syntax Highlighting -----------------------------------------------------

struct Highlighter {
    out: String,
    cmd_position: bool,
    in_single: bool,
    in_double: bool,
    word: String,
}

fn is_number(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    s.chars().all(|c| c >= '0' && c <= '9')
}

impl Highlighter {
    fn new() -> Self {
        Highlighter {
            out: String::new(),
            cmd_position: true,
            in_single: false,
            in_double: false,
            word: String::new(),
        }
    }

    fn flush_word(&mut self) {
        if self.word.is_empty() {
            return;
        }
        let color = if is_number(&self.word) {
            "33"
        } else if self.word.starts_with('-') && self.word.len() > 1 {
            "36"
        } else if self.word.starts_with('$') {
            "34"
        } else if self.cmd_position {
            self.cmd_position = false;
            "1;32"
        } else {
            "0"
        };
        self.out.push_str("\x1b[");
        self.out.push_str(color);
        self.out.push('m');
        self.out.push_str(&self.word);
        self.out.push_str("\x1b[0m");
        self.word.clear();
    }
}

fn highlight(input: &str) -> String {
    let mut h = Highlighter::new();
    let bytes: Vec<char> = input.chars().collect();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];

        if !h.in_single && !h.in_double && c == '#' && h.word.is_empty() {
            h.flush_word();
            h.out.push_str("\x1b[90m");
            while i < bytes.len() {
                h.out.push(bytes[i]);
                i += 1;
            }
            h.out.push_str("\x1b[0m");
            continue;
        }

        if !h.in_single && !h.in_double && (c == '\'' || c == '"') {
            h.flush_word();
            let quote = c;
            if quote == '\'' {
                h.in_single = true;
            } else {
                h.in_double = true;
            }
            h.out.push_str("\x1b[35m");
            h.out.push(c);
            i += 1;
            let mut closed = false;
            while i < bytes.len() {
                let d = bytes[i];
                if (h.in_single && d == '\'') || (h.in_double && d == '"') {
                    h.out.push(d);
                    i += 1;
                    closed = true;
                    break;
                }
                if h.in_double && d == '\\' && i + 1 < bytes.len() {
                    h.out.push(d);
                    h.out.push(bytes[i + 1]);
                    i += 2;
                    continue;
                }
                if h.in_double && d == '$' {
                    h.out.push_str("\x1b[0m\x1b[34m");
                    h.out.push('$');
                    i += 1;
                    if i < bytes.len() && bytes[i] == '{' {
                        h.out.push('{');
                        i += 1;
                        while i < bytes.len() && bytes[i] != '}' {
                            h.out.push(bytes[i]);
                            i += 1;
                        }
                        if i < bytes.len() {
                            h.out.push('}');
                            i += 1;
                        }
                    } else {
                        while i < bytes.len() && (bytes[i].is_alphanumeric() || bytes[i] == '_') {
                            h.out.push(bytes[i]);
                            i += 1;
                        }
                    }
                    h.out.push_str("\x1b[0m\x1b[35m");
                    continue;
                }
                h.out.push(d);
                i += 1;
            }
            h.out.push_str("\x1b[0m");
            h.in_single = false;
            h.in_double = false;
            let _ = closed;
            h.cmd_position = false;
            continue;
        }

        if h.in_single {
            h.out.push(c);
            i += 1;
            continue;
        }

        if c == '|' || c == '&' || c == ';' || c == '<' || c == '>' {
            h.flush_word();
            h.out.push_str("\x1b[31m");
            if (c == '|' || c == '&' || c == '>') && i + 1 < bytes.len() && bytes[i + 1] == c {
                h.out.push(c);
                h.out.push(bytes[i + 1]);
                i += 2;
            } else {
                h.out.push(c);
                i += 1;
            }
            h.out.push_str("\x1b[0m");
            h.cmd_position = true;
            continue;
        }

        if c == ' ' || c == '\t' {
            h.flush_word();
            h.out.push(c);
            i += 1;
            continue;
        }

        if c == '$' {
            h.flush_word();
            h.out.push_str("\x1b[34m");
            h.out.push(c);
            i += 1;
            if i < bytes.len() && bytes[i] == '{' {
                h.out.push('{');
                i += 1;
                while i < bytes.len() && bytes[i] != '}' {
                    h.out.push(bytes[i]);
                    i += 1;
                }
                if i < bytes.len() {
                    h.out.push('}');
                    i += 1;
                }
            } else {
                while i < bytes.len() && (bytes[i].is_alphanumeric() || bytes[i] == '_') {
                    h.out.push(bytes[i]);
                    i += 1;
                }
            }
            h.out.push_str("\x1b[0m");
            h.cmd_position = false;
            continue;
        }

        h.word.push(c);
        i += 1;
    }
    h.flush_word();
    h.out
}

// ---- Render ------------------------------------------------------------------

fn visible_len(s: &str) -> usize {
    let bytes = s.as_bytes();
    let mut count = 0usize;
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == 0x1b {
            i += 1;
            if i < bytes.len() && bytes[i] == b'[' {
                i += 1;
            }
            while i < bytes.len() && !(bytes[i] >= 0x40 && bytes[i] <= 0x7e) {
                i += 1;
            }
            if i < bytes.len() {
                i += 1;
            }
        } else {
            count += 1;
            i += 1;
        }
    }
    count
}

fn mode_prefix(mode: Mode) -> &'static str {
    match mode {
        Mode::Insert => "",
        Mode::Command => "\x1b[36m[C]\x1b[0m ",
        Mode::Visual => "\x1b[7m[V]\x1b[0m ",
    }
}

fn mode_gutter(mode: Mode) -> &'static str {
    match mode {
        Mode::Command => "\x1b[90m1 \x1b[0m",
        Mode::Visual => "\x1b[90m1 \x1b[0m",
        Mode::Insert => "",
    }
}

#[allow(unused_assignments)]
fn inject_selection(highlighted: &str, sel_start: usize, sel_end: usize) -> String {
    if sel_start >= sel_end {
        return highlighted.to_string();
    }
    let h_bytes = highlighted.as_bytes();
    let mut start_pos = 0;
    let mut end_pos = 0;
    let mut visible = 0usize;
    let mut i = 0usize;
    while i < h_bytes.len() && visible < sel_start {
        if h_bytes[i] == 0x1b {
            i += 1;
            if i < h_bytes.len() && h_bytes[i] == b'[' {
                i += 1;
            }
            while i < h_bytes.len() && !(h_bytes[i] >= 0x40 && h_bytes[i] <= 0x7e) {
                i += 1;
            }
            if i < h_bytes.len() {
                i += 1;
            }
        } else {
            visible += 1;
            i += 1;
        }
    }
    start_pos = i;
    while i < h_bytes.len() && visible < sel_end {
        if h_bytes[i] == 0x1b {
            i += 1;
            if i < h_bytes.len() && h_bytes[i] == b'[' {
                i += 1;
            }
            while i < h_bytes.len() && !(h_bytes[i] >= 0x40 && h_bytes[i] <= 0x7e) {
                i += 1;
            }
            if i < h_bytes.len() {
                i += 1;
            }
        } else {
            visible += 1;
            i += 1;
        }
    }
    end_pos = i;
    let mut out = String::new();
    out.push_str(&highlighted[..start_pos]);
    out.push_str("\x1b[7m");
    out.push_str(&highlighted[start_pos..end_pos]);
    out.push_str("\x1b[27m");
    out.push_str(&highlighted[end_pos..]);
    out
}

fn redraw_with_mode(
    prompt: &str,
    input: &str,
    cursor: usize,
    suggestion: Option<&str>,
    mode: Mode,
    selection: Option<(usize, usize)>,
) {
    let mp = mode_prefix(mode);
    let gutter = mode_gutter(mode);
    let mp_visible = visible_len(mp);
    let gutter_visible = visible_len(gutter);
    let prompt_visible = visible_len(prompt);
    let highlighted = highlight(input);
    let display = if let Some((s, e)) = selection {
        inject_selection(&highlighted, s, e)
    } else {
        highlighted
    };
    let input_visible = visible_len(&display);

    let ghost: String = match suggestion {
        Some(s) if !s.is_empty() && mode == Mode::Insert => format!("\x1b[90m{}\x1b[0m", s),
        _ => String::new(),
    };

    print!("\r{}{}{}{}{}", mp, gutter, prompt, display, ghost);
    print!("\x1b[K");

    let total_visible =
        mp_visible + gutter_visible + prompt_visible + input_visible + visible_len(&ghost);
    let cursor_col = mp_visible
        + gutter_visible
        + prompt_visible
        + visible_len(&display[..byte_index(&display, cursor)]);
    let back = total_visible.saturating_sub(cursor_col);
    if back > 0 {
        print!("\x1b[{}D", back);
    }
}

fn byte_index(highlighted: &str, n: usize) -> usize {
    let bytes = highlighted.as_bytes();
    let mut visible = 0usize;
    let mut i = 0usize;
    while i < bytes.len() && visible < n {
        if bytes[i] == 0x1b {
            i += 1;
            if i < bytes.len() && bytes[i] == b'[' {
                i += 1;
            }
            while i < bytes.len() && !(bytes[i] >= 0x40 && bytes[i] <= 0x7e) {
                i += 1;
            }
            if i < bytes.len() {
                i += 1;
            }
        } else {
            visible += 1;
            i += 1;
        }
    }
    i
}

// ---- Main readline function --------------------------------------------------

pub fn read_line(history: &mut History, prompt: &str) -> String {
    let hist_ptr = history as *mut History;
    let completer = Completer;
    let mut input = String::new();
    let mut cursor: usize = 0;
    let mut mode = Mode::Insert;
    let mut undo_stack: Vec<String> = Vec::new();
    let mut redo_stack: Vec<String> = Vec::new();
    let mut yank_buf = String::new();
    let mut visual_start: usize = 0;
    let mut last_search = String::new();
    let mut search_query = String::new();
    let mut in_search = false;

    let do_suggestion = |inp: &str, hist: &mut History, comp: &Completer| -> Option<String> {
        recompute_suggestion(inp, hist, comp)
    };

    let mut suggestion = do_suggestion(&input, unsafe { &mut *hist_ptr }, &completer);
    redraw_with_mode(prompt, &input, cursor, suggestion.as_deref(), mode, None);

    let mut buf = [0u8; 4096];
    loop {
        let n = unsafe { libsarga::syscall::syscall3(0, 0u64, buf.as_mut_ptr() as u64, 4096u64) };
        if n <= 0 {
            break;
        }
        let mut i = 0;
        while i < n as usize {
            let c = buf[i];

            if in_search {
                if c == b'\n' || c == b'\r' {
                    if !search_query.is_empty() {
                        last_search = search_query.clone();
                        let h = unsafe { &mut *hist_ptr };
                        if let Some((_idx, entry)) = h.search_containing(&search_query) {
                            input = entry;
                            cursor = input.len();
                        }
                    }
                    in_search = false;
                    search_query.clear();
                    suggestion = do_suggestion(&input, unsafe { &mut *hist_ptr }, &completer);
                    redraw_with_mode(prompt, &input, cursor, suggestion.as_deref(), mode, None);
                    i += 1;
                    continue;
                }
                if c == 0x1b {
                    in_search = false;
                    search_query.clear();
                    redraw_with_mode(prompt, &input, cursor, suggestion.as_deref(), mode, None);
                    i += 1;
                    continue;
                }
                if (c == 0x7f || c == 0x08) && !search_query.is_empty() {
                    search_query.pop();
                    print!("\r\x1b[K\x1b[36m/\x1b[0m{}", search_query);
                    i += 1;
                    continue;
                }
                if c >= 0x20 && c <= 0x7e && c != b'/' {
                    search_query.push(c as char);
                    print!("\r\x1b[K\x1b[36m/\x1b[0m{}", search_query);
                    i += 1;
                    continue;
                }
                i += 1;
                continue;
            }

            if c == 0x12 && mode == Mode::Command {
                if let Some(next_state) = redo_stack.pop() {
                    undo_stack.push(input.clone());
                    input = next_state;
                    cursor = input.len().min(cursor);
                    suggestion = do_suggestion(&input, unsafe { &mut *hist_ptr }, &completer);
                    redraw_with_mode(prompt, &input, cursor, suggestion.as_deref(), mode, None);
                }
                i += 1;
                continue;
            }

            if mode == Mode::Command {
                let consumed = handle_command_key(
                    c,
                    prompt,
                    &mut input,
                    &mut cursor,
                    &mut mode,
                    unsafe { &mut *hist_ptr },
                    &mut suggestion,
                    &completer,
                    &mut undo_stack,
                    &mut redo_stack,
                    &mut yank_buf,
                    &mut visual_start,
                    &mut last_search,
                );
                if consumed {
                    i += 1;
                    continue;
                }
            }

            match c {
                b'\n' | b'\r' => {
                    print!("\n");
                    let h = unsafe { &mut *hist_ptr };
                    let input_copy = input.clone();
                    h.add(&input_copy);
                    if input.starts_with('!') {
                        if input == "!!" {
                            if let Some(last) = h.search("") {
                                input = last;
                                print!("{}\n", input);
                            }
                        } else if let Ok(n) = input[1..].parse::<usize>() {
                            if n > 0 && n <= h.entries.len() {
                                input = h.entries[n - 1].clone();
                                print!("{}\n", input);
                            }
                        }
                    }
                    return input;
                }
                0x7f | 0x08 => {
                    if cursor > 0 && mode == Mode::Insert {
                        cursor -= 1;
                        input.remove(cursor);
                        suggestion = do_suggestion(&input, unsafe { &mut *hist_ptr }, &completer);
                        redraw_with_mode(prompt, &input, cursor, suggestion.as_deref(), mode, None);
                    }
                }
                0x09 => {
                    if mode == Mode::Insert && suggestion.is_some() {
                        if let Some(ref sug) = suggestion.clone() {
                            input.push_str(sug);
                            cursor = input.len();
                            suggestion = None;
                            redraw_with_mode(prompt, &input, cursor, None, mode, None);
                        }
                    } else {
                        let matches = completer.complete(&input);
                        if matches.len() == 1 {
                            input = matches[0].clone();
                            cursor = input.len();
                            suggestion =
                                do_suggestion(&input, unsafe { &mut *hist_ptr }, &completer);
                            redraw_with_mode(
                                prompt,
                                &input,
                                cursor,
                                suggestion.as_deref(),
                                mode,
                                None,
                            );
                        } else if !matches.is_empty() {
                            print!("\n");
                            let mut line = String::new();
                            for m in &matches {
                                line.push_str(m);
                                line.push(' ');
                            }
                            if line.len() > 60 {
                                line.truncate(60);
                                line.push_str("...");
                            }
                            print!("{}\n", line);
                            redraw_with_mode(
                                prompt,
                                &input,
                                cursor,
                                suggestion.as_deref(),
                                mode,
                                None,
                            );
                        }
                    }
                }
                0x1b => {
                    let esc_only = i + 1 >= n as usize || buf[i + 1] != b'[';
                    if esc_only {
                        if mode == Mode::Insert {
                            mode = Mode::Command;
                            if cursor > 0 {
                                cursor -= 1;
                                print!("\x1b[D");
                            }
                            redraw_with_mode(
                                prompt,
                                &input,
                                cursor,
                                suggestion.as_deref(),
                                mode,
                                None,
                            );
                        } else if mode == Mode::Visual {
                            mode = Mode::Command;
                            redraw_with_mode(
                                prompt,
                                &input,
                                cursor,
                                suggestion.as_deref(),
                                mode,
                                None,
                            );
                        }
                        i += 1;
                        continue;
                    }
                    if i + 2 < n as usize {
                        match buf[i + 2] {
                            b'A' => {
                                let h = unsafe { &mut *hist_ptr };
                                if let Some(prev) = h.prev() {
                                    undo_stack.push(input.clone());
                                    input = prev.to_string();
                                    cursor = input.len();
                                    suggestion = do_suggestion(&input, h, &completer);
                                    redraw_with_mode(
                                        prompt,
                                        &input,
                                        cursor,
                                        suggestion.as_deref(),
                                        mode,
                                        None,
                                    );
                                }
                            }
                            b'B' => {
                                let h = unsafe { &mut *hist_ptr };
                                if let Some(next) = h.next() {
                                    undo_stack.push(input.clone());
                                    input = next.to_string();
                                    cursor = input.len();
                                    suggestion = do_suggestion(&input, h, &completer);
                                    redraw_with_mode(
                                        prompt,
                                        &input,
                                        cursor,
                                        suggestion.as_deref(),
                                        mode,
                                        None,
                                    );
                                } else {
                                    input.clear();
                                    cursor = 0;
                                    suggestion = None;
                                    redraw_with_mode(prompt, &input, cursor, None, mode, None);
                                }
                            }
                            b'C' => {
                                if mode == Mode::Insert && cursor == input.len() {
                                    if let Some(ref sug) = suggestion.clone() {
                                        input.push_str(sug);
                                        cursor = input.len();
                                        suggestion = None;
                                        redraw_with_mode(prompt, &input, cursor, None, mode, None);
                                        i += 2;
                                        continue;
                                    }
                                }
                                if cursor < input.len() {
                                    cursor += 1;
                                    if mode == Mode::Visual {
                                        redraw_with_mode(
                                            prompt,
                                            &input,
                                            cursor,
                                            suggestion.as_deref(),
                                            mode,
                                            Some((
                                                visual_start.min(cursor),
                                                visual_start.max(cursor),
                                            )),
                                        );
                                    } else {
                                        print!("\x1b[C");
                                    }
                                }
                            }
                            b'D' => {
                                if cursor > 0 {
                                    cursor -= 1;
                                    if mode == Mode::Visual {
                                        redraw_with_mode(
                                            prompt,
                                            &input,
                                            cursor,
                                            suggestion.as_deref(),
                                            mode,
                                            Some((
                                                visual_start.min(cursor),
                                                visual_start.max(cursor),
                                            )),
                                        );
                                    } else {
                                        print!("\x1b[D");
                                    }
                                }
                            }
                            b'H' | b'1' => {
                                if i + 3 < n as usize && buf[i + 3] == b';' {
                                    let ctrl_right = i + 4 < n as usize
                                        && buf[i + 4] == b'5'
                                        && i + 5 < n as usize
                                        && buf[i + 5] == b'C';
                                    if ctrl_right && mode == Mode::Insert && cursor == input.len() {
                                        if let Some(ref sug) = suggestion.clone() {
                                            let bytes = sug.as_bytes();
                                            let mut word_end = 0;
                                            while word_end < bytes.len()
                                                && is_word_sep(bytes[word_end])
                                            {
                                                word_end += 1;
                                            }
                                            while word_end < bytes.len()
                                                && is_word_char(bytes[word_end])
                                            {
                                                word_end += 1;
                                            }
                                            let accept = &sug[..word_end];
                                            input.push_str(accept);
                                            cursor = input.len();
                                            if word_end >= sug.len() {
                                                suggestion = None;
                                            } else {
                                                suggestion = Some(sug[word_end..].to_string());
                                            }
                                            redraw_with_mode(
                                                prompt,
                                                &input,
                                                cursor,
                                                suggestion.as_deref(),
                                                mode,
                                                None,
                                            );
                                            i += 5;
                                            continue;
                                        }
                                    }
                                    let ctrl_left = i + 4 < n as usize
                                        && buf[i + 4] == b'5'
                                        && i + 5 < n as usize
                                        && buf[i + 5] == b'D';
                                    if ctrl_left {
                                        cursor = motion_word_left(&input, cursor);
                                        if mode == Mode::Visual {
                                            redraw_with_mode(
                                                prompt,
                                                &input,
                                                cursor,
                                                suggestion.as_deref(),
                                                mode,
                                                Some((
                                                    visual_start.min(cursor),
                                                    visual_start.max(cursor),
                                                )),
                                            );
                                        } else {
                                            print!("\x1b[{}D", 1);
                                        }
                                        i += 5;
                                        continue;
                                    }
                                }
                                if cursor > 0 {
                                    print!("\x1b[{}D", cursor);
                                    cursor = 0;
                                    if mode == Mode::Visual {
                                        redraw_with_mode(
                                            prompt,
                                            &input,
                                            cursor,
                                            suggestion.as_deref(),
                                            mode,
                                            Some((
                                                visual_start.min(cursor),
                                                visual_start.max(cursor),
                                            )),
                                        );
                                    }
                                }
                            }
                            b'F' | b'4' => {
                                if mode == Mode::Insert && cursor == input.len() {
                                    if let Some(ref sug) = suggestion.clone() {
                                        input.push_str(sug);
                                        cursor = input.len();
                                        suggestion = None;
                                        redraw_with_mode(prompt, &input, cursor, None, mode, None);
                                        i += 2;
                                        continue;
                                    }
                                }
                                let dist = input.len() - cursor;
                                if dist > 0 {
                                    cursor = input.len();
                                    if mode == Mode::Visual {
                                        redraw_with_mode(
                                            prompt,
                                            &input,
                                            cursor,
                                            suggestion.as_deref(),
                                            mode,
                                            Some((
                                                visual_start.min(cursor),
                                                visual_start.max(cursor),
                                            )),
                                        );
                                    } else {
                                        print!("\x1b[{}C", dist);
                                    }
                                }
                            }
                            b'3' => {
                                if cursor < input.len() && mode == Mode::Insert {
                                    input.remove(cursor);
                                    suggestion = do_suggestion(
                                        &input,
                                        unsafe { &mut *hist_ptr },
                                        &completer,
                                    );
                                    redraw_with_mode(
                                        prompt,
                                        &input,
                                        cursor,
                                        suggestion.as_deref(),
                                        mode,
                                        None,
                                    );
                                }
                            }
                            _ => {}
                        }
                        i += 2;
                    }
                }
                0x03 => {
                    print!("\n");
                    return String::new();
                }
                0x04 => {
                    if input.is_empty() {
                        return String::new();
                    }
                }
                _ => {
                    if c >= 0x20 && c <= 0x7e && mode == Mode::Insert {
                        input.insert(cursor, c as char);
                        cursor += 1;
                        suggestion = do_suggestion(&input, unsafe { &mut *hist_ptr }, &completer);
                        redraw_with_mode(prompt, &input, cursor, suggestion.as_deref(), mode, None);
                    }
                }
            }
            i += 1;
        }
    }
    input
}

fn handle_command_key(
    c: u8,
    prompt: &str,
    input: &mut String,
    cursor: &mut usize,
    mode: &mut Mode,
    history: &mut History,
    suggestion: &mut Option<String>,
    completer: &Completer,
    undo_stack: &mut Vec<String>,
    redo_stack: &mut Vec<String>,
    yank_buf: &mut String,
    visual_start: &mut usize,
    last_search: &mut String,
) -> bool {
    match c {
        b'i' => {
            if *mode == Mode::Visual {
                let s = (*visual_start).min(*cursor);
                let e = (*visual_start).max(*cursor);
                if s < e {
                    undo_stack.push(input.clone());
                    let yanked = input[s..e].to_string();
                    *yank_buf = yanked;
                    libsarga::io::clipboard_write(yank_buf.as_bytes());
                    input.drain(s..e);
                    *cursor = s;
                }
            }
            *mode = Mode::Insert;
            redraw_with_mode(prompt, input, *cursor, suggestion.as_deref(), *mode, None);
            true
        }
        b'I' => {
            *cursor = 0;
            *mode = Mode::Insert;
            redraw_with_mode(prompt, input, *cursor, suggestion.as_deref(), *mode, None);
            true
        }
        b'a' => {
            if *mode == Mode::Visual {
                let s = (*visual_start).min(*cursor);
                let e = (*visual_start).max(*cursor);
                if s < e {
                    undo_stack.push(input.clone());
                    let yanked = input[s..e].to_string();
                    *yank_buf = yanked;
                    libsarga::io::clipboard_write(yank_buf.as_bytes());
                    input.drain(s..e);
                    *cursor = s;
                }
            } else if *cursor < input.len() {
                *cursor += 1;
            }
            *mode = Mode::Insert;
            redraw_with_mode(prompt, input, *cursor, suggestion.as_deref(), *mode, None);
            true
        }
        b'A' => {
            *cursor = input.len();
            *mode = Mode::Insert;
            redraw_with_mode(prompt, input, *cursor, suggestion.as_deref(), *mode, None);
            true
        }
        b'h' => {
            if *cursor > 0 {
                *cursor -= 1;
            }
            if *mode == Mode::Visual {
                redraw_with_mode(
                    prompt,
                    input,
                    *cursor,
                    suggestion.as_deref(),
                    *mode,
                    Some(((*visual_start).min(*cursor), (*visual_start).max(*cursor))),
                );
            }
            true
        }
        b'l' => {
            if *cursor < input.len() {
                *cursor += 1;
            }
            if *mode == Mode::Visual {
                redraw_with_mode(
                    prompt,
                    input,
                    *cursor,
                    suggestion.as_deref(),
                    *mode,
                    Some(((*visual_start).min(*cursor), (*visual_start).max(*cursor))),
                );
            }
            true
        }
        b'k' => {
            if let Some(prev) = history.prev() {
                undo_stack.push(input.clone());
                *input = prev.to_string();
                *cursor = input.len();
                *suggestion = None;
                redraw_with_mode(prompt, input, *cursor, None, *mode, None);
            }
            true
        }
        b'j' => {
            if let Some(next) = history.next() {
                undo_stack.push(input.clone());
                *input = next.to_string();
                *cursor = input.len();
                *suggestion = None;
                redraw_with_mode(prompt, input, *cursor, None, *mode, None);
            } else {
                input.clear();
                *cursor = 0;
                *suggestion = None;
                redraw_with_mode(prompt, input, *cursor, None, *mode, None);
            }
            true
        }
        b'x' => {
            if *mode == Mode::Visual {
                let s = (*visual_start).min(*cursor);
                let e = (*visual_start).max(*cursor);
                if s < e {
                    undo_stack.push(input.clone());
                    *yank_buf = input[s..e].to_string();
                    libsarga::io::clipboard_write(yank_buf.as_bytes());
                    input.drain(s..e);
                    *cursor = s;
                    *suggestion = recompute_suggestion(input, history, completer);
                    *mode = Mode::Command;
                    redraw_with_mode(prompt, input, *cursor, suggestion.as_deref(), *mode, None);
                }
            } else if *cursor < input.len() {
                undo_stack.push(input.clone());
                let removed = input.remove(*cursor);
                let mut s = String::new();
                s.push(removed);
                *yank_buf = s;
                libsarga::io::clipboard_write(yank_buf.as_bytes());
                *suggestion = recompute_suggestion(input, history, completer);
                redraw_with_mode(prompt, input, *cursor, suggestion.as_deref(), *mode, None);
            }
            true
        }
        b'0' => {
            *cursor = 0;
            if *mode == Mode::Visual {
                redraw_with_mode(
                    prompt,
                    input,
                    *cursor,
                    suggestion.as_deref(),
                    *mode,
                    Some(((*visual_start).min(*cursor), (*visual_start).max(*cursor))),
                );
            }
            true
        }
        b'^' => {
            *cursor = motion_first_nonws(input);
            if *mode == Mode::Visual {
                redraw_with_mode(
                    prompt,
                    input,
                    *cursor,
                    suggestion.as_deref(),
                    *mode,
                    Some(((*visual_start).min(*cursor), (*visual_start).max(*cursor))),
                );
            }
            true
        }
        b'$' => {
            *cursor = input.len();
            if *mode == Mode::Visual {
                redraw_with_mode(
                    prompt,
                    input,
                    *cursor,
                    suggestion.as_deref(),
                    *mode,
                    Some(((*visual_start).min(*cursor), (*visual_start).max(*cursor))),
                );
            }
            true
        }
        b'w' => {
            *cursor = motion_word_right(input, *cursor);
            if *mode == Mode::Visual {
                redraw_with_mode(
                    prompt,
                    input,
                    *cursor,
                    suggestion.as_deref(),
                    *mode,
                    Some(((*visual_start).min(*cursor), (*visual_start).max(*cursor))),
                );
            }
            true
        }
        b'b' => {
            *cursor = motion_word_left(input, *cursor);
            if *mode == Mode::Visual {
                redraw_with_mode(
                    prompt,
                    input,
                    *cursor,
                    suggestion.as_deref(),
                    *mode,
                    Some(((*visual_start).min(*cursor), (*visual_start).max(*cursor))),
                );
            }
            true
        }
        b'e' => {
            *cursor = motion_end_word(input, *cursor);
            if *mode == Mode::Visual {
                redraw_with_mode(
                    prompt,
                    input,
                    *cursor,
                    suggestion.as_deref(),
                    *mode,
                    Some(((*visual_start).min(*cursor), (*visual_start).max(*cursor))),
                );
            }
            true
        }
        b'E' => {
            *cursor = motion_word_end_big(input, *cursor);
            if *mode == Mode::Visual {
                redraw_with_mode(
                    prompt,
                    input,
                    *cursor,
                    suggestion.as_deref(),
                    *mode,
                    Some(((*visual_start).min(*cursor), (*visual_start).max(*cursor))),
                );
            }
            true
        }
        b'v' => {
            if *mode == Mode::Command {
                *mode = Mode::Visual;
                *visual_start = *cursor;
                redraw_with_mode(
                    prompt,
                    input,
                    *cursor,
                    suggestion.as_deref(),
                    *mode,
                    Some(((*visual_start).min(*cursor), (*visual_start).max(*cursor))),
                );
            }
            true
        }
        b'd' => {
            if *mode == Mode::Visual {
                let s = (*visual_start).min(*cursor);
                let e = (*visual_start).max(*cursor);
                if s < e {
                    undo_stack.push(input.clone());
                    *yank_buf = input[s..e].to_string();
                    libsarga::io::clipboard_write(yank_buf.as_bytes());
                    input.drain(s..e);
                    *cursor = s;
                    *suggestion = recompute_suggestion(input, history, completer);
                }
                *mode = Mode::Command;
                redraw_with_mode(prompt, input, *cursor, suggestion.as_deref(), *mode, None);
                true
            } else {
                let mut sub_buf = [0u8; 16];
                let sn = unsafe {
                    libsarga::syscall::syscall3(0, 0u64, sub_buf.as_mut_ptr() as u64, 16u64)
                };
                if sn > 0 {
                    match sub_buf[0] {
                        b'd' => {
                            // dd
                            undo_stack.push(input.clone());
                            *yank_buf = input.clone();
                            libsarga::io::clipboard_write(yank_buf.as_bytes());
                            input.clear();
                            *cursor = 0;
                        }
                        b'w' => {
                            // dw
                            undo_stack.push(input.clone());
                            let end = motion_word_right(input, *cursor);
                            if end > *cursor {
                                *yank_buf = input[*cursor..end].to_string();
                                libsarga::io::clipboard_write(yank_buf.as_bytes());
                                input.drain(*cursor..end);
                            }
                        }
                        b'$' => {
                            // d$
                            undo_stack.push(input.clone());
                            *yank_buf = input[*cursor..].to_string();
                            libsarga::io::clipboard_write(yank_buf.as_bytes());
                            input.truncate(*cursor);
                        }
                        b'0' => {
                            // d0
                            undo_stack.push(input.clone());
                            *yank_buf = input[..*cursor].to_string();
                            libsarga::io::clipboard_write(yank_buf.as_bytes());
                            input.drain(..*cursor);
                            *cursor = 0;
                        }
                        _ => {}
                    }
                    *suggestion = recompute_suggestion(input, history, completer);
                    redraw_with_mode(prompt, input, *cursor, suggestion.as_deref(), *mode, None);
                }
                true
            }
        }
        b'y' => {
            if *mode == Mode::Visual {
                let s = (*visual_start).min(*cursor);
                let e = (*visual_start).max(*cursor);
                if s < e {
                    *yank_buf = input[s..e].to_string();
                    libsarga::io::clipboard_write(yank_buf.as_bytes());
                }
                *mode = Mode::Command;
                redraw_with_mode(prompt, input, *cursor, suggestion.as_deref(), *mode, None);
            } else {
                let mut sub_buf = [0u8; 16];
                let sn = unsafe {
                    libsarga::syscall::syscall3(0, 0u64, sub_buf.as_mut_ptr() as u64, 16u64)
                };
                if sn > 0 && sub_buf[0] == b'y' {
                    undo_stack.push(input.clone());
                    *yank_buf = input.clone();
                    libsarga::io::clipboard_write(yank_buf.as_bytes());
                    redraw_with_mode(prompt, input, *cursor, suggestion.as_deref(), *mode, None);
                }
            }
            true
        }
        b'c' => {
            if *mode == Mode::Visual {
                let s = (*visual_start).min(*cursor);
                let e = (*visual_start).max(*cursor);
                if s < e {
                    undo_stack.push(input.clone());
                    *yank_buf = input[s..e].to_string();
                    libsarga::io::clipboard_write(yank_buf.as_bytes());
                    input.drain(s..e);
                    *cursor = s;
                    *suggestion = recompute_suggestion(input, history, completer);
                }
                *mode = Mode::Insert;
                redraw_with_mode(prompt, input, *cursor, suggestion.as_deref(), *mode, None);
                true
            } else {
                let mut sub_buf = [0u8; 16];
                let sn = unsafe {
                    libsarga::syscall::syscall3(0, 0u64, sub_buf.as_mut_ptr() as u64, 16u64)
                };
                if sn > 0 && sub_buf[0] == b'c' {
                    undo_stack.push(input.clone());
                    *yank_buf = input.clone();
                    libsarga::io::clipboard_write(yank_buf.as_bytes());
                    input.clear();
                    *cursor = 0;
                    *mode = Mode::Insert;
                    *suggestion = recompute_suggestion(input, history, completer);
                    redraw_with_mode(prompt, input, *cursor, suggestion.as_deref(), *mode, None);
                }
                true
            }
        }
        b'D' => {
            undo_stack.push(input.clone());
            *yank_buf = input[*cursor..].to_string();
            libsarga::io::clipboard_write(yank_buf.as_bytes());
            input.truncate(*cursor);
            *suggestion = recompute_suggestion(input, history, completer);
            redraw_with_mode(prompt, input, *cursor, suggestion.as_deref(), *mode, None);
            true
        }
        b'C' => {
            undo_stack.push(input.clone());
            *yank_buf = input[*cursor..].to_string();
            libsarga::io::clipboard_write(yank_buf.as_bytes());
            input.truncate(*cursor);
            *mode = Mode::Insert;
            *suggestion = recompute_suggestion(input, history, completer);
            redraw_with_mode(prompt, input, *cursor, suggestion.as_deref(), *mode, None);
            true
        }
        b'Y' => {
            *yank_buf = input.clone();
            libsarga::io::clipboard_write(yank_buf.as_bytes());
            true
        }
        b'p' | b'P' => {
            let clip = if !yank_buf.is_empty() {
                yank_buf.clone()
            } else {
                let mut clip_buf = [0u8; 4096];
                let n = libsarga::io::clipboard_read(&mut clip_buf);
                if n > 0 {
                    core::str::from_utf8(&clip_buf[..n])
                        .unwrap_or("")
                        .to_string()
                } else {
                    String::new()
                }
            };
            if !clip.is_empty() {
                undo_stack.push(input.clone());
                input.insert_str(*cursor, &clip);
                *cursor += clip.len();
                *suggestion = recompute_suggestion(input, history, completer);
                redraw_with_mode(prompt, input, *cursor, suggestion.as_deref(), *mode, None);
            }
            true
        }
        b'u' => {
            if let Some(prev) = undo_stack.pop() {
                redo_stack.push(input.clone());
                *input = prev;
                *cursor = input.len().min(*cursor);
                *suggestion = recompute_suggestion(input, history, completer);
                redraw_with_mode(prompt, input, *cursor, suggestion.as_deref(), *mode, None);
            }
            true
        }
        b'.' => true,
        b'/' => {
            // Inline search mode
            print!("\r\x1b[K\x1b[36m/\x1b[0m");
            let mut sq = String::new();
            let mut sub_buf = [0u8; 4096];
            loop {
                let sn = unsafe {
                    libsarga::syscall::syscall3(0, 0u64, sub_buf.as_mut_ptr() as u64, 4096u64)
                };
                if sn <= 0 {
                    break;
                }
                let mut si = 0;
                while si < sn as usize {
                    let sc = sub_buf[si];
                    if sc == b'\n' || sc == b'\r' {
                        if !sq.is_empty() {
                            *last_search = sq.clone();
                            if let Some((_idx, entry)) = history.search_containing(&sq) {
                                undo_stack.push(input.clone());
                                *input = entry;
                                *cursor = input.len();
                                *suggestion = None;
                            }
                        }
                        redraw_with_mode(
                            prompt,
                            input,
                            *cursor,
                            suggestion.as_deref(),
                            *mode,
                            None,
                        );
                        return true;
                    }
                    if sc == 0x1b {
                        redraw_with_mode(
                            prompt,
                            input,
                            *cursor,
                            suggestion.as_deref(),
                            *mode,
                            None,
                        );
                        return true;
                    }
                    if (sc == 0x7f || sc == 0x08) && !sq.is_empty() {
                        sq.pop();
                        print!("\r\x1b[K\x1b[36m/\x1b[0m{}", sq);
                        si += 1;
                        continue;
                    }
                    if sc >= 0x20 && sc <= 0x7e && sc != b'/' {
                        sq.push(sc as char);
                        print!("\r\x1b[K\x1b[36m/\x1b[0m{}", sq);
                    }
                    si += 1;
                }
            }
            true
        }
        b'n' => {
            if !last_search.is_empty() {
                if let Some((_idx, entry)) = history.search_containing(last_search) {
                    undo_stack.push(input.clone());
                    *input = entry;
                    *cursor = input.len();
                    *suggestion = None;
                    redraw_with_mode(prompt, input, *cursor, suggestion.as_deref(), *mode, None);
                }
            }
            true
        }
        b'N' => true,
        b'g' => {
            *cursor = 0;
            if *mode == Mode::Visual {
                redraw_with_mode(
                    prompt,
                    input,
                    *cursor,
                    suggestion.as_deref(),
                    *mode,
                    Some(((*visual_start).min(*cursor), (*visual_start).max(*cursor))),
                );
            }
            true
        }
        b'G' => {
            *cursor = input.len();
            if *mode == Mode::Visual {
                redraw_with_mode(
                    prompt,
                    input,
                    *cursor,
                    suggestion.as_deref(),
                    *mode,
                    Some(((*visual_start).min(*cursor), (*visual_start).max(*cursor))),
                );
            }
            true
        }
        b'%' => {
            if let Some(pos) = motion_match_bracket(input, *cursor) {
                *cursor = pos;
                if *mode == Mode::Visual {
                    redraw_with_mode(
                        prompt,
                        input,
                        *cursor,
                        suggestion.as_deref(),
                        *mode,
                        Some(((*visual_start).min(*cursor), (*visual_start).max(*cursor))),
                    );
                }
            }
            true
        }
        b'f' => {
            let mut sub_buf = [0u8; 16];
            let sn =
                unsafe { libsarga::syscall::syscall3(0, 0u64, sub_buf.as_mut_ptr() as u64, 16u64) };
            if sn > 0 {
                let fc = sub_buf[0];
                if let Some(pos) = motion_find_char(input, *cursor, fc, true, false) {
                    *cursor = pos;
                    if *mode == Mode::Visual {
                        redraw_with_mode(
                            prompt,
                            input,
                            *cursor,
                            suggestion.as_deref(),
                            *mode,
                            Some(((*visual_start).min(*cursor), (*visual_start).max(*cursor))),
                        );
                    }
                }
            }
            true
        }
        b'F' => {
            let mut sub_buf = [0u8; 16];
            let sn =
                unsafe { libsarga::syscall::syscall3(0, 0u64, sub_buf.as_mut_ptr() as u64, 16u64) };
            if sn > 0 {
                let fc = sub_buf[0];
                if let Some(pos) = motion_find_char(input, *cursor, fc, false, false) {
                    *cursor = pos;
                    if *mode == Mode::Visual {
                        redraw_with_mode(
                            prompt,
                            input,
                            *cursor,
                            suggestion.as_deref(),
                            *mode,
                            Some(((*visual_start).min(*cursor), (*visual_start).max(*cursor))),
                        );
                    }
                }
            }
            true
        }
        b't' => {
            let mut sub_buf = [0u8; 16];
            let sn =
                unsafe { libsarga::syscall::syscall3(0, 0u64, sub_buf.as_mut_ptr() as u64, 16u64) };
            if sn > 0 {
                let fc = sub_buf[0];
                if let Some(pos) = motion_find_char(input, *cursor, fc, true, true) {
                    *cursor = pos;
                    if *mode == Mode::Visual {
                        redraw_with_mode(
                            prompt,
                            input,
                            *cursor,
                            suggestion.as_deref(),
                            *mode,
                            Some(((*visual_start).min(*cursor), (*visual_start).max(*cursor))),
                        );
                    }
                }
            }
            true
        }
        b'T' => {
            let mut sub_buf = [0u8; 16];
            let sn =
                unsafe { libsarga::syscall::syscall3(0, 0u64, sub_buf.as_mut_ptr() as u64, 16u64) };
            if sn > 0 {
                let fc = sub_buf[0];
                if let Some(pos) = motion_find_char(input, *cursor, fc, false, true) {
                    *cursor = pos;
                    if *mode == Mode::Visual {
                        redraw_with_mode(
                            prompt,
                            input,
                            *cursor,
                            suggestion.as_deref(),
                            *mode,
                            Some(((*visual_start).min(*cursor), (*visual_start).max(*cursor))),
                        );
                    }
                }
            }
            true
        }
        b'\n' | b'\r' => false,
        0x1b => false,
        _ => true,
    }
}
