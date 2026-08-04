//! Clipboard manager — buffer, history.

use alloc::string::String;
use alloc::vec::Vec;

pub(crate) struct ClipboardEntry {
    pub text: String,
    #[allow(dead_code)] // history timestamps, no history UI yet
    pub timestamp: u64,
}

pub(crate) struct ClipboardManager {
    pub(crate) text: String,
    pub(crate) length: usize,
    pub(crate) timestamp: u64,
    history: Vec<ClipboardEntry>,
}

impl ClipboardManager {
    pub fn new() -> Self {
        ClipboardManager {
            text: String::new(),
            length: 0,
            timestamp: 0,
            history: Vec::new(),
        }
    }

    pub fn copy(&mut self, text: &str, timestamp: u64) {
        self.text = String::from(text);
        self.length = text.len();
        self.timestamp = timestamp;
        self.history.retain(|e| e.text != text);
        self.history.push(ClipboardEntry {
            text: String::from(text),
            timestamp,
        });
        if self.history.len() > 16 {
            self.history.remove(0);
        }
    }

    pub fn paste(&self) -> &str {
        &self.text
    }

    pub fn clear(&mut self) {
        self.text.clear();
        self.length = 0;
        self.timestamp = 0;
    }

    pub fn history(&self) -> &[ClipboardEntry] {
        &self.history
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }
}
