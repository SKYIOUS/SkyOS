//! Notification manager — queue, history, urgency levels.

use alloc::string::String;
use alloc::vec::Vec;

pub(crate) struct Notification {
    pub id: u64,
    pub title: String,
    pub body: String,
    pub icon_id: u8,
    pub urgency: u8,
    pub created_tick: u64,
    pub timeout: u32,
    pub actions: Vec<(&'static str, &'static str)>,
    pub dismissed: bool,
}

pub(crate) struct NotificationManager {
    notifications: Vec<Notification>,
    next_id: u64,
    visible_count: usize,
}

impl NotificationManager {
    pub fn new() -> Self {
        NotificationManager {
            notifications: Vec::new(),
            next_id: 1,
            visible_count: 0,
        }
    }

    pub fn notify(&mut self, title: &str, body: &str, urgency: u8, timeout: u32) -> u64 {
        self.notify_with_icon(title, body, 0, urgency, timeout)
    }

    pub fn notify_with_icon(
        &mut self,
        title: &str,
        body: &str,
        icon_id: u8,
        urgency: u8,
        timeout: u32,
    ) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        if self.notifications.len() >= 64 {
            self.notifications.remove(0);
            if self.visible_count > 0 {
                self.visible_count -= 1;
            }
        }
        self.notifications.push(Notification {
            id,
            title: String::from(title),
            body: String::from(body),
            icon_id,
            urgency,
            created_tick: 0,
            timeout,
            actions: Vec::new(),
            dismissed: false,
        });
        self.visible_count += 1;
        id
    }

    pub fn dismiss(&mut self, id: u64) -> bool {
        if let Some(pos) = self.notifications.iter().position(|n| n.id == id) {
            if !self.notifications[pos].dismissed {
                self.notifications[pos].dismissed = true;
                self.visible_count = self.visible_count.saturating_sub(1);
                // Keep contiguous: swap dismissed to after visible range
                if pos < self.visible_count {
                    self.notifications.swap(pos, self.visible_count);
                }
            }
            true
        } else {
            false
        }
    }

    pub fn dismiss_all(&mut self) {
        for n in &mut self.notifications {
            n.dismissed = true;
        }
        self.visible_count = 0;
    }

    pub fn update(&mut self, id: u64, title: &str, body: &str) {
        if let Some(n) = self.notifications.iter_mut().find(|n| n.id == id) {
            n.title = String::from(title);
            n.body = String::from(body);
        }
    }

    pub fn tick(&mut self, current_tick: u64) {
        let mut i = 0;
        while i < self.notifications.len() {
            if self.notifications[i].timeout > 0 && !self.notifications[i].dismissed {
                if current_tick >= self.notifications[i].created_tick + self.notifications[i].timeout as u64 {
                    self.notifications[i].dismissed = true;
                    self.visible_count = self.visible_count.saturating_sub(1);
                    // Swap to keep visible contiguous
                    if i < self.visible_count {
                        self.notifications.swap(i, self.visible_count);
                    }
                }
            }
            i += 1;
        }
    }

    pub fn visible_notifications(&self) -> &[Notification] {
        &self.notifications[..self.visible_count]
    }
}
