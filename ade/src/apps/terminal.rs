//! Terminal state — command history, cursor tracking, input handling.

use alloc::string::String;
use alloc::vec::Vec;

pub(crate) struct TerminalState {
    pub history: Vec<String>,
    pub history_pos: isize,
    pub cursor_pos: u16,
    pub prompt: String,
    pub scroll_offset: u32,
}

impl TerminalState {
    pub fn new() -> Self {
        TerminalState {
            history: Vec::new(),
            history_pos: -1,
            cursor_pos: 0,
            prompt: String::from("> "),
            scroll_offset: 0,
        }
    }

    pub fn update(&mut self, content: &mut Vec<String>, key: u8) {
        match key {
            0x0A | 0x0D => {
                let line = content.last().map(|l| l.clone()).unwrap_or_default();
                if !line.is_empty() {
                    self.history.push(line);
                    if self.history.len() > 50 {
                        self.history.remove(0);
                    }
                }
                self.history_pos = -1;
                self.cursor_pos = 0;
                content.push(String::new());
            }
            0x7F | 0x08 => {
                self.cursor_pos = self.cursor_pos.saturating_sub(1);
                if let Some(line) = content.last_mut() {
                    line.pop();
                }
            }
            ch if (ch >= 0x20 && ch <= 0x7E) => {
                if let Some(line) = content.last_mut() {
                    line.push(ch as char);
                    self.cursor_pos = line.len() as u16;
                }
            }
            _ => {}
        }
    }

    pub fn clear(&mut self, content: &mut Vec<String>) {
        content.clear();
        content.push(String::new());
        self.cursor_pos = 0;
        self.scroll_offset = 0;
    }

    pub fn history(&self) -> &[String] {
        &self.history
    }

    pub fn set_prompt(&mut self, prompt: &str) {
        self.prompt = String::from(prompt);
    }
}
