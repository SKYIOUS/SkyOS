// Scaffold — used by future phase
#![allow(dead_code)]
//! Clipboard service — buffer, history, pinned entries.

use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

pub(crate) struct ClipboardEntry {
    pub text: String,
    pub pinned: bool,
}

#[allow(dead_code)]
pub(crate) struct ClipboardService {
    pub buf: Vec<u8>,
    pub history: VecDeque<ClipboardEntry>,
    pub panel_open: bool,
}

#[allow(dead_code)]
impl ClipboardService {
    pub fn new() -> Self {
        let mut history = VecDeque::new();
        history.push_back(ClipboardEntry {
            text: String::new(),
            pinned: false,
        });
        ClipboardService {
            buf: Vec::new(),
            history,
            panel_open: false,
        }
    }

    pub fn copy(&mut self, text: &str) {
        self.buf = text.as_bytes().to_vec();
        self.history.retain(|e| e.text != text);
        self.history.push_front(ClipboardEntry {
            text: String::from(text),
            pinned: false,
        });
        if self.history.len() > 20 {
            self.history.pop_back();
        }
    }

    pub fn paste(&self) -> Option<&str> {
        if self.buf.is_empty() {
            None
        } else {
            core::str::from_utf8(&self.buf).ok()
        }
    }

    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    pub fn toggle_pin(&mut self, idx: usize) {
        if idx < self.history.len() {
            self.history[idx].pinned = !self.history[idx].pinned;
        }
    }
}
