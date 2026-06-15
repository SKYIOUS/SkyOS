#![no_std]
#![no_main]
extern crate alloc;
use alloc::vec::Vec;
use alloc::string::String;
use libsarga::sarga_main;
use libsarga::io::{self, openpty, write, read, close};
use libsarga::process::{fork, execve, exit};
use libsarga::gui::Window;
use libsarga::theme::Theme;

const COLS: usize = 80;
const ROWS: usize = 25;
const CELL_W: u32 = 8;
const CELL_H: u32 = 16;
const TAB_H: u32 = 28;

const ANSI_COLORS: [u32; 16] = [
    0xFF000000, 0xFFCC0000, 0xFF00CC00, 0xFFCCCC00,
    0xFF0000CC, 0xFFCC00CC, 0xFF00CCCC, 0xFFCCCCCC,
    0xFF555555, 0xFFFF5555, 0xFF55FF55, 0xFFFFFF55,
    0xFF5555FF, 0xFFFF55FF, 0xFF55FFFF, 0xFFFFFFFF,
];

struct Terminal {
    chars: Vec<u8>,
    fg: Vec<u32>,
    bg: Vec<u32>,
    cursor: usize,
    saved_cursor: usize,
    scroll_top: usize,
    scroll_bottom: usize,
    current_fg: u32,
    current_bg: u32,
    bold: bool,
    reverse: bool,
    default_fg: u32,
    default_bg: u32,
}

impl Terminal {
    fn new(theme: &Theme) -> Self {
        Self {
            chars: alloc::vec![b' '; COLS * ROWS],
            fg: alloc::vec![theme.text_secondary; COLS * ROWS],
            bg: alloc::vec![theme.bg_primary; COLS * ROWS],
            cursor: 0,
            saved_cursor: 0,
            scroll_top: 0,
            scroll_bottom: ROWS,
            current_fg: theme.text_secondary,
            current_bg: theme.bg_primary,
            bold: false,
            reverse: false,
            default_fg: theme.text_secondary,
            default_bg: theme.bg_primary,
        }
    }

    fn effective_fg(&self) -> u32 {
        if self.reverse { self.current_bg } else { self.current_fg }
    }

    fn effective_bg(&self) -> u32 {
        if self.reverse { self.current_fg } else { self.current_bg }
    }

    fn row(&self) -> usize { self.cursor / COLS }
    fn col(&self) -> usize { self.cursor % COLS }

    fn set_pos(&mut self, row: usize, col: usize) {
        let r = row.min(ROWS - 1);
        let c = col.min(COLS - 1);
        self.cursor = r * COLS + c;
    }

    fn scroll_up(&mut self) {
        for y in self.scroll_top..self.scroll_bottom.saturating_sub(1) {
            let src = (y + 1) * COLS;
            let dst = y * COLS;
            self.chars.copy_within(src..src + COLS, dst);
            self.fg.copy_within(src..src + COLS, dst);
            self.bg.copy_within(src..src + COLS, dst);
        }
        let clear_row = self.scroll_bottom - 1;
        for i in 0..COLS {
            self.chars[clear_row * COLS + i] = b' ';
            self.fg[clear_row * COLS + i] = self.default_fg;
            self.bg[clear_row * COLS + i] = self.default_bg;
        }
    }

    fn scroll_down(&mut self) {
        for y in (self.scroll_top..self.scroll_bottom.saturating_sub(1)).rev() {
            let src = y * COLS;
            let dst = (y + 1) * COLS;
            self.chars.copy_within(src..src + COLS, dst);
            self.fg.copy_within(src..src + COLS, dst);
            self.bg.copy_within(src..src + COLS, dst);
        }
        for i in 0..COLS {
            self.chars[self.scroll_top * COLS + i] = b' ';
            self.fg[self.scroll_top * COLS + i] = self.default_fg;
            self.bg[self.scroll_top * COLS + i] = self.default_bg;
        }
    }

    fn erase_line(&mut self, mode: u8) {
        let start = match mode {
            0 => self.cursor,
            1 => self.row() * COLS,
            _ => self.row() * COLS,
        };
        let end = match mode {
            0 => (self.row() + 1) * COLS,
            _ => (self.row() + 1) * COLS,
        };
        let clear_end = if mode == 2 { ROWS * COLS } else { end };
        for i in start..clear_end {
            self.chars[i] = b' ';
            self.fg[i] = self.default_fg;
            self.bg[i] = self.default_bg;
        }
    }

    fn erase_display(&mut self, mode: u8) {
        match mode {
            0 => {
                let start = self.cursor;
                for i in start..ROWS * COLS {
                    self.chars[i] = b' ';
                    self.fg[i] = self.default_fg;
                    self.bg[i] = self.default_bg;
                }
            }
            1 => {
                for i in 0..=self.cursor {
                    self.chars[i] = b' ';
                    self.fg[i] = self.default_fg;
                    self.bg[i] = self.default_bg;
                }
            }
            _ => {
                for i in 0..ROWS * COLS {
                    self.chars[i] = b' ';
                    self.fg[i] = self.default_fg;
                    self.bg[i] = self.default_bg;
                }
                self.cursor = 0;
            }
        }
    }

    fn erase_chars(&mut self, n: usize) {
        let end = (self.cursor + n).min(self.row() * COLS + COLS);
        for i in self.cursor..end {
            self.chars[i] = b' ';
            self.fg[i] = self.effective_fg();
            self.bg[i] = self.effective_bg();
        }
    }

    fn insert_lines(&mut self, n: usize) {
        for _ in 0..n { self.scroll_down(); }
    }

    fn delete_lines(&mut self, n: usize) {
        for _ in 0..n { self.scroll_up(); }
    }

    fn put_char(&mut self, c: u8) {
        if self.cursor >= ROWS * COLS { return; }
        self.chars[self.cursor] = c;
        self.fg[self.cursor] = self.effective_fg();
        self.bg[self.cursor] = self.effective_bg();
        self.cursor += 1;
        if self.col() >= COLS {
            self.cursor -= self.col();
            if self.row() >= self.scroll_bottom - 1 {
                self.scroll_up();
            } else {
                self.cursor += COLS;
            }
        }
    }

    fn newline(&mut self) {
        let row = self.row();
        if row >= self.scroll_bottom - 1 {
            self.scroll_up();
        } else {
            self.cursor = (row + 1) * COLS + self.col();
        }
    }

    fn carriage_return(&mut self) { self.cursor = self.row() * COLS; }

    fn backspace(&mut self) {
        if self.col() > 0 { self.cursor -= 1; }
    }

    fn tab(&mut self) {
        let new_col = (self.col() / 8 + 1) * 8;
        if new_col < COLS { self.cursor = self.row() * COLS + new_col; }
    }
}

struct Vt100Parser {
    buf: Vec<u8>,
    params: [u16; 16],
    param_count: usize,
    state: ParserState,
}

#[derive(Clone, Copy, PartialEq)]
enum ParserState { Ground, Escape, CsiParam, CsiIntermediate }

impl Vt100Parser {
    fn new() -> Self {
        Self { buf: Vec::new(), params: [0; 16], param_count: 0, state: ParserState::Ground }
    }

    fn reset_params(&mut self) { self.params = [0; 16]; self.param_count = 0; }

    fn current_param(&mut self) -> &mut u16 {
        if self.param_count == 0 { self.param_count = 1; self.params[0] = 0; }
        &mut self.params[self.param_count - 1]
    }

    fn process_csi(&self, term: &mut Terminal) {
        let p = self.params;
        let n = self.param_count;
        let final_byte = self.buf.last().copied().unwrap_or(b'm');

        match final_byte {
            b'm' => {
                if n == 0 || p[0] == 0 {
                    term.current_fg = term.default_fg;
                    term.current_bg = term.default_bg;
                    term.bold = false;
                    term.reverse = false;
                } else {
                    let mut i = 0;
                    while i < n {
                        match p[i] {
                            0 => { term.current_fg = term.default_fg; term.current_bg = term.default_bg; term.bold = false; term.reverse = false; }
                            1 => term.bold = true,
                            2 => {}
                            7 => term.reverse = true,
                            22 => term.bold = false,
                            27 => term.reverse = false,
                            30..=37 => {
                                let mut c = ANSI_COLORS[(p[i] - 30) as usize];
                                if term.bold { c = (c & 0xFF000000) | (c & 0x00FFFFFF).wrapping_mul(2); }
                                term.current_fg = c;
                            }
                            39 => term.current_fg = term.default_fg,
                            40..=47 => { term.current_bg = ANSI_COLORS[(p[i] - 40) as usize]; }
                            49 => term.current_bg = term.default_bg,
                            90..=97 => {
                                let mut c = ANSI_COLORS[(p[i] - 90 + 8) as usize];
                                if term.bold { c = (c & 0xFF000000) | (c & 0x00FFFFFF).wrapping_mul(2); }
                                term.current_fg = c;
                            }
                            100..=107 => { term.current_bg = ANSI_COLORS[(p[i] - 100 + 8) as usize]; }
                            _ => {}
                        }
                        i += 1;
                    }
                }
            }
            b'H' | b'f' => {
                let row = if n > 0 { (p[0] as usize).saturating_sub(1) } else { 0 };
                let col = if n > 1 { (p[1] as usize).saturating_sub(1) } else { 0 };
                term.set_pos(row, col);
            }
            b'A' => { let n = if n > 0 { p[0] as usize } else { 1 }; term.set_pos(term.row().saturating_sub(n), term.col()); }
            b'B' => { let n = if n > 0 { p[0] as usize } else { 1 }; term.set_pos((term.row() + n).min(ROWS - 1), term.col()); }
            b'C' => { let n = if n > 0 { p[0] as usize } else { 1 }; term.set_pos(term.row(), (term.col() + n).min(COLS - 1)); }
            b'D' => { let n = if n > 0 { p[0] as usize } else { 1 }; term.set_pos(term.row(), term.col().saturating_sub(n)); }
            b'E' => { let n = if n > 0 { p[0] as usize } else { 1 }; term.set_pos(term.row() + n, 0); }
            b'F' => { let n = if n > 0 { p[0] as usize } else { 1 }; term.set_pos(term.row().saturating_sub(n), 0); }
            b'G' => { let col = if n > 0 { (p[0] as usize).saturating_sub(1) } else { 0 }; term.set_pos(term.row(), col); }
            b'J' => { let mode = if n > 0 { p[0] as u8 } else { 0 }; term.erase_display(mode); }
            b'K' => { let mode = if n > 0 { p[0] as u8 } else { 0 }; term.erase_line(mode); }
            b'X' => { let n = if n > 0 { p[0] as usize } else { 1 }; term.erase_chars(n); }
            b'P' => { let n = if n > 0 { p[0] as usize } else { 1 }; term.delete_lines(n); }
            b'L' => { let n = if n > 0 { p[0] as usize } else { 1 }; term.insert_lines(n); }
            b'S' => { let n = if n > 0 { p[0] as usize } else { 1 }; for _ in 0..n { term.scroll_up(); } }
            b'T' => { let n = if n > 0 { p[0] as usize } else { 1 }; for _ in 0..n { term.scroll_down(); } }
            b'@' => {
                let n = if n > 0 { p[0] as usize } else { 1 };
                let row = term.row(); let col = term.col();
                let end = (row + 1) * COLS;
                let start = row * COLS + col;
                let shift = n.min(COLS - col);
                for i in (start..end - shift).rev() {
                    term.chars[i + shift] = term.chars[i];
                    term.fg[i + shift] = term.fg[i];
                    term.bg[i + shift] = term.bg[i];
                }
                for i in start..start + shift {
                    term.chars[i] = b' ';
                    term.fg[i] = term.effective_fg();
                    term.bg[i] = term.effective_bg();
                }
            }
            b'd' => { let row = if n > 0 { (p[0] as usize).saturating_sub(1) } else { 0 }; term.set_pos(row.min(ROWS - 1), term.col()); }
            b'r' => {
                let top = if n > 0 { (p[0] as usize).saturating_sub(1) } else { 0 };
                let bottom = if n > 1 { p[1] as usize } else { ROWS };
                term.scroll_top = top.min(ROWS);
                term.scroll_bottom = bottom.min(ROWS);
                term.set_pos(top, 0);
            }
            b's' => { term.saved_cursor = term.cursor; }
            b'u' => { term.cursor = term.saved_cursor; }
            _ => {}
        }
    }

    fn process_byte(&mut self, byte: u8, term: &mut Terminal) {
        match self.state {
            ParserState::Ground => {
                match byte {
                    0x1b => { self.state = ParserState::Escape; self.buf.clear(); self.buf.push(byte); self.reset_params(); }
                    b'\n' | b'\x0b' | b'\x0c' => term.newline(),
                    b'\r' => term.carriage_return(),
                    b'\t' => term.tab(),
                    0x08 => term.backspace(),
                    0x07 => {}
                    c if c >= 32 => term.put_char(c),
                    _ => {}
                }
            }
            ParserState::Escape => {
                self.buf.push(byte);
                match byte {
                    b'[' => { self.state = ParserState::CsiParam; self.reset_params(); }
                    b']' => { self.state = ParserState::Ground; }
                    b'M' => { if term.row() > 0 { term.set_pos(term.row() - 1, term.col()); } self.state = ParserState::Ground; }
                    b'D' => { if term.col() > 0 { term.set_pos(term.row(), term.col() - 1); } self.state = ParserState::Ground; }
                    b'E' => { term.newline(); self.state = ParserState::Ground; }
                    b'7' => { term.saved_cursor = term.cursor; self.state = ParserState::Ground; }
                    b'8' => { term.cursor = term.saved_cursor; self.state = ParserState::Ground; }
                    b'c' => { term.erase_display(2); term.set_pos(0, 0); self.state = ParserState::Ground; }
                    _ => { self.state = ParserState::Ground; }
                }
            }
            ParserState::CsiParam => {
                self.buf.push(byte);
                match byte {
                    b'0'..=b'9' => { let param = self.current_param(); *param = (*param) * 10 + (byte - b'0') as u16; }
                    b';' => { self.param_count += 1; if self.param_count > 15 { self.param_count = 15; } self.params[self.param_count - 1] = 0; }
                    b'?' | b'>' | b'!' => { self.state = ParserState::CsiIntermediate; }
                    _ => { self.process_csi(term); self.state = ParserState::Ground; }
                }
            }
            ParserState::CsiIntermediate => {
                self.buf.push(byte);
                match byte {
                    b'0'..=b'9' => { let param = self.current_param(); *param = (*param) * 10 + (byte - b'0') as u16; }
                    b';' => { self.param_count += 1; if self.param_count > 15 { self.param_count = 15; } self.params[self.param_count - 1] = 0; }
                    _ => { self.process_csi(term); self.state = ParserState::Ground; }
                }
            }
        }
    }
}

struct TabPage {
    master_fd: i64,
    term: Terminal,
    parser: Vt100Parser,
    title: String,
}

fn spawn_shell(slave_fd: i64) -> ! {
    io::dup2(slave_fd, 0);
    io::dup2(slave_fd, 1);
    io::dup2(slave_fd, 2);
    if slave_fd > 2 { let _ = close(slave_fd); }
    let args = ["/bin/sash"];
    let env = ["TERM=xterm-256color"];
    execve("/bin/sash", &args, &env);
    exit(1);
}

fn create_tab(theme: &Theme) -> Option<TabPage> {
    let pty = openpty().ok()?;
    let master_fd: i64 = pty.0;
    let slave_fd: i64 = pty.1;
    match fork() {
        Ok(0) => spawn_shell(slave_fd),
        Ok(_) => {
            if slave_fd > 2 { let _ = close(slave_fd); }
            Some(TabPage {
                master_fd,
                term: Terminal::new(theme),
                parser: Vt100Parser::new(),
                title: String::from("sash"),
            })
        }
        Err(_) => None,
    }
}

fn user_main() {
    let theme = Theme::dark();
    let term_h = (ROWS as u32) * CELL_H + 8;
    let tab_bar_h = TAB_H + 4;
    let win_w = (COLS as u32) * CELL_W + 16;
    let win_h = term_h + tab_bar_h;

    let mut win = match Window::create("sarga-term", win_w, win_h) {
        Ok(w) => w,
        Err(e) => { io::print_str(&alloc::format!("sarga-term: window failed: {}\n", e)); return; }
    };

    win.clear(theme.bg_primary);

    let mut pages: Vec<TabPage> = Vec::new();
    let mut active_tab: usize = 0;

    if let Some(page) = create_tab(&theme) {
        pages.push(page);
    } else {
        io::print_str("sarga-term: failed to create initial tab\n");
        return;
    }

    let mut read_buf = [0u8; 4096];

    loop {
        let page_count = pages.len();

        // Handle keyboard
        while let Some(k) = win.get_key() {
            match k {
                0x0E => {
                    // Ctrl+N — new tab
                    if let Some(page) = create_tab(&theme) {
                        pages.push(page);
                    }
                }
                0x17 => {
                    // Ctrl+W — close current tab
                    if pages.len() > 1 {
                        let _ = close(pages[active_tab].master_fd);
                        pages.remove(active_tab);
                        if active_tab >= pages.len() {
                            active_tab = pages.len() - 1;
                        }
                    }
                }
                _ => {
                    // Send to active tab's PTY
                    let bytes = [k];
                    let _ = write(pages[active_tab].master_fd, &bytes);
                }
            }
        }

        // Handle mouse
        loop {
            let mouse = win.get_mouse();
            if mouse.buttons == 0 && mouse.scroll == 0 { break; }
            if mouse.buttons & 1 != 0 {
                // Left click
                let mx = mouse.x as i32;
                let my = mouse.y as i32;
                // Check tab bar
                if my >= 0 && my < TAB_H as i32 {
                    const TAB_W: i32 = 110;
                    const PLUS_W: i32 = 24;
                    let end = (TAB_W + 2) * page_count as i32;
                    if mx >= end && mx < end + PLUS_W {
                        // "+" button
                        if let Some(page) = create_tab(&theme) {
                            pages.push(page);
                        }
                    } else {
                        let idx = (mx / (TAB_W + 2)) as usize;
                        if idx < pages.len() {
                            // Check close button (rightmost 16px of tab)
                            let tab_x = idx as i32 * (TAB_W + 2);
                            let close_x = tab_x + TAB_W - 16;
                            if mx >= close_x && mx < tab_x + TAB_W && pages.len() > 1 {
                                let _ = close(pages[idx].master_fd);
                                pages.remove(idx);
                                if active_tab >= pages.len() {
                                    active_tab = pages.len() - 1;
                                }
                            } else {
                                active_tab = idx;
                            }
                        }
                    }
                }
            }
            // Right-click on tab to close
            if mouse.buttons & 2 != 0 {
                let my = mouse.y as i32;
                if my >= 0 && my < TAB_H as i32 {
                    const TAB_W: i32 = 110;
                    let mx = mouse.x as i32;
                    let idx = (mx / (TAB_W + 2)) as usize;
                    if idx < pages.len() && pages.len() > 1 {
                        let _ = close(pages[idx].master_fd);
                        pages.remove(idx);
                        if active_tab >= pages.len() {
                            active_tab = pages.len() - 1;
                        }
                    }
                }
            }
            if mouse.buttons & 1 == 0 && mouse.buttons & 2 == 0 && mouse.scroll == 0 { break; }
        }

        // Read from all PTYs
        let n_pages = pages.len();
        for i in 0..n_pages {
            let fd = pages[i].master_fd;
            loop {
                match read(fd, &mut read_buf) {
                    Ok(n) if n > 0 => {
                        let page = &mut pages[i];
                        let term = &mut page.term;
                        let parser = &mut page.parser;
                        for &b in &read_buf[..n] {
                            parser.process_byte(b, term);
                        }
                    }
                    _ => { break; }
                }
            }
        }

        // Render
        win.clear(theme.bg_primary);

        // Tab bar
        const TAB_W: u32 = 110;
        win.draw_rect(0, 0, win_w, TAB_H + 2, theme.bg_surface);

        for i in 0..pages.len() {
            let tx = 2 + i as u32 * (TAB_W + 2);
            let active = i == active_tab;

            let bg = if active { theme.bg_elevated } else { theme.bg_surface };
            win.draw_rect(tx, 2, TAB_W, TAB_H, bg);

            if active {
                win.draw_line_h(tx, 2 + TAB_H - 2, TAB_W, theme.accent);
            }

            let display_title = if pages[i].title.len() > 10 {
                alloc::format!("{}..", &pages[i].title[..8])
            } else {
                pages[i].title.clone()
            };
            let text_color = if active { theme.text } else { theme.text_secondary };
            win.draw_string(tx + 6, 8, &display_title, text_color, 0);

            // Close button
            if pages.len() > 1 {
                let cx = tx + TAB_W - 16;
                win.draw_string(cx, 8, "x", theme.text_disabled, 0);
            }
        }

        // "+" button
        let plus_x = 2 + pages.len() as u32 * (TAB_W + 2);
        win.draw_rect(plus_x, 2, 24, TAB_H, theme.bg_surface);
        win.draw_string(plus_x + 6, 8, "+", theme.text_secondary, 0);

        // Terminal content area (below tab bar)
        let content_y = TAB_H + 4;
        for row in 0..ROWS {
            for col in 0..COLS {
                let idx = row * COLS + col;
                let c = pages[active_tab].term.chars[idx] as char;
                let fg = pages[active_tab].term.fg[idx];
                let bg = pages[active_tab].term.bg[idx];
                let px = 8 + col as u32 * CELL_W;
                let py = content_y + 4 + row as u32 * CELL_H;
                win.draw_rect(px, py, CELL_W, CELL_H, bg);
                win.draw_char(px, py, c, fg, bg);
            }
        }

        // Cursor
        let cx = 8 + pages[active_tab].term.col() as u32 * CELL_W;
        let cy = content_y + 4 + pages[active_tab].term.row() as u32 * CELL_H;
        win.draw_rect(cx, cy, CELL_W, CELL_H, theme.accent);

        let _ = win.flush();

        unsafe { libsarga::syscall::syscall1(35, 16_666_000); }
    }
}

sarga_main!(user_main);
