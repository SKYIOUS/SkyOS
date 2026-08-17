//! Text surface — the line buffer + view state + pty ANSI parser for a window.
//!
//! Every window draws its text from a `TextSurface`, which owns the line
//! storage (`lines`), the view offset (`scroll`), and the parser/cursor state
//! used by terminal windows (`esc_state`, `cursor`). `AppWindow` keeps only
//! geometry and pty plumbing (`pty_fd`), so text storage and window layout
//! are no longer one structure.

use alloc::string::String;
use alloc::vec::Vec;

/// Hard line width: input and pty output wrap at 80 visible chars.
const LINE_WRAP: usize = 80;

pub(crate) struct TextSurface {
    lines: Vec<String>,
    scroll: u32,
    /// Persistent ANSI parser state (0 = plain, then ESC/CSI/OSC/OSC-esc
    /// states) so sequences split across reads survive.
    esc_state: u8,
    /// Cursor column; `\r` returns it to 0 and later text overwrites the
    /// current line (sash redraws every keystroke).
    cursor: u16,
}

impl TextSurface {
    pub(crate) fn new() -> Self {
        TextSurface {
            lines: Vec::new(),
            scroll: 0,
            esc_state: 0,
            cursor: 0,
        }
    }

    /// All lines, oldest first. Draw code reads through this; callers must
    /// not assume any particular layout beyond "one String per line".
    pub(crate) fn lines(&self) -> &[String] {
        &self.lines
    }

    pub(crate) fn last_line(&self) -> Option<&String> {
        self.lines.last()
    }

    /// Append a whole line (launcher seeds `> path` / `[launched …]`,
    /// the legacy `$ cmd` echo, and empty first lines).
    pub(crate) fn push_line(&mut self, line: String) {
        self.lines.push(line);
    }

    /// Drop oldest lines until at most `max` remain (terminal scrollback cap).
    pub(crate) fn truncate(&mut self, max: usize) {
        if self.lines.len() > max {
            self.lines.drain(0..self.lines.len() - max);
        }
    }

    /// Wipe the surface: lines, view offset, cursor, and parser state, so a
    /// Ctrl+L mid-escape-sequence cannot corrupt the next pty bytes.
    pub(crate) fn clear(&mut self) {
        self.lines.clear();
        self.scroll = 0;
        self.esc_state = 0;
        self.cursor = 0;
    }

    pub(crate) fn scroll(&self) -> u32 {
        self.scroll
    }

    /// Scroll the view: `delta` is subtracted from the offset (positive
    /// `delta` moves toward the newest lines), clamped to the buffer.
    pub(crate) fn scroll_by(&mut self, delta: i8) {
        let max = self.lines.len().saturating_sub(1) as i32;
        self.scroll = (self.scroll as i32 - delta as i32).clamp(0, max) as u32;
    }

    /// Append one visible char at the end of the current line, wrapping to a
    /// new line past `LINE_WRAP` (legacy non-pty input path).
    pub(crate) fn push_char(&mut self, c: char) {
        if self.lines.last().is_none_or(|l| l.len() > LINE_WRAP) {
            self.lines.push(String::new());
        }
        if let Some(line) = self.lines.last_mut() {
            line.push(c);
        }
    }

    /// Backspace: pop the last char of the current line (legacy input path).
    pub(crate) fn pop_char(&mut self) {
        if let Some(line) = self.lines.last_mut() {
            line.pop();
        }
    }

    /// Feed pty output bytes into the terminal's text surface.
    ///
    /// Persistent mini-ANSI parser (`esc_state` survives across reads):
    /// ESC/CSI/OSC sequences are consumed, `\r` moves the cursor column to 0
    /// and later text overwrites the line (sash redraws on every keystroke),
    /// CSI K (erase-to-end) truncates the tail, `\n` starts a new line.
    /// Returns true when visible content changed.
    pub(crate) fn consume_pty_bytes(&mut self, bytes: &[u8]) -> bool {
        let mut changed = false;
        for &b in bytes {
            match self.esc_state {
                0 => match b {
                    0x1B => self.esc_state = 1,
                    b'\r' => self.cursor = 0,
                    b'\n' => {
                        self.lines.push(String::new());
                        self.cursor = 0;
                        changed = true;
                    }
                    b'\t' => {
                        for _ in 0..4 {
                            self.pty_put_char(' ');
                        }
                        changed = true;
                    }
                    0x20..=0x7E => {
                        self.pty_put_char(b as char);
                        changed = true;
                    }
                    _ => {}
                },
                1 => match b {
                    b'[' => self.esc_state = 2, // CSI
                    b']' => self.esc_state = 3, // OSC
                    0x20..=0x2F => {}           // intermediate: keep scanning
                    _ => self.esc_state = 0,    // single-char escape or garbage
                },
                2 => {
                    // CSI: params/intermediates until the final byte.
                    if (0x40..=0x7E).contains(&b) {
                        if b == b'K' && self.pty_erase_to_end() {
                            changed = true;
                        }
                        self.esc_state = 0;
                    } else if !(0x20..=0x3F).contains(&b) {
                        self.esc_state = 0;
                    }
                }
                3 => {
                    // OSC: terminated by BEL or ST (ESC \).
                    if b == 0x07 {
                        self.esc_state = 0;
                    } else if b == 0x1B {
                        self.esc_state = 4;
                    }
                }
                4 => self.esc_state = if b == b'\\' { 0 } else { 3 },
                _ => self.esc_state = 0,
            }
        }
        changed
    }

    /// Write one visible char at the cursor column, overwriting if the
    /// cursor has moved back (after `\r`). Wraps lines past `LINE_WRAP`.
    fn pty_put_char(&mut self, c: char) {
        if self.lines.is_empty() {
            self.lines.push(String::new());
        }
        let cur = self.cursor as usize;
        if let Some(line) = self.lines.last_mut() {
            if cur < line.len() {
                // Overwrite the char at the cursor, keep the tail (sash
                // redraws by printing "\r<line>" over the previous line).
                // Lines are ASCII-only (parser accepts 0x20..=0x7E), so
                // byte index == char index.
                line.replace_range(cur..cur + 1, &String::from(c));
            } else {
                line.push(c);
            }
            self.cursor = (cur + 1) as u16;
            if line.len() > LINE_WRAP {
                self.lines.push(String::new());
                self.cursor = 0;
            }
        }
    }

    /// CSI `K` (erase from cursor to end of line). Returns true if it cut.
    fn pty_erase_to_end(&mut self) -> bool {
        let cur = self.cursor as usize;
        let Some(line) = self.lines.last_mut() else {
            return false;
        };
        if line.len() > cur {
            line.truncate(cur);
            true
        } else {
            false
        }
    }
}
