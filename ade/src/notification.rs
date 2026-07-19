//! Notification center — model, queue, history, popups.

use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

#[derive(Clone, Debug)]
pub(crate) struct Notification {
    pub title: String,
    pub body: String,
    pub priority: u8, // 0=low, 1=normal, 2=high
    pub timeout: u32, // ticks before auto-dismiss (0=persistent)
    pub ticks_left: u32,
    #[allow(dead_code)]
    pub actions: Vec<(&'static str, &'static str)>,
}

pub(crate) struct NotificationCenter {
    pub active: Vec<Notification>,
    pub history: VecDeque<Notification>,
    #[allow(dead_code)]
    pub expanded: bool,
}

impl NotificationCenter {
    pub fn new() -> Self {
        NotificationCenter {
            active: Vec::new(),
            history: VecDeque::new(),
            expanded: false,
        }
    }

    pub fn push(&mut self, title: &str, body: &str, priority: u8, timeout: u32) {
        let n = Notification {
            title: String::from(title),
            body: String::from(body),
            priority,
            timeout,
            ticks_left: timeout,
            actions: Vec::new(),
        };
        self.active.push(n);
    }

    pub fn tick(&mut self) {
        let mut i = 0;
        while i < self.active.len() {
            if self.active[i].timeout > 0 {
                self.active[i].ticks_left = self.active[i].ticks_left.saturating_sub(1);
                if self.active[i].ticks_left == 0 {
                    let n = self.active.remove(i);
                    self.add_history(n);
                    continue;
                }
            }
            i += 1;
        }
    }

    #[allow(dead_code)]
    pub fn dismiss(&mut self, idx: usize) {
        if idx < self.active.len() {
            let n = self.active.remove(idx);
            self.add_history(n);
        }
    }

    fn add_history(&mut self, n: Notification) {
        if self.history.len() >= 50 {
            self.history.pop_front();
        }
        self.history.push_back(n);
    }

    #[allow(dead_code)]
    pub fn clear_history(&mut self) {
        self.history.clear();
    }
}
