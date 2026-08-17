//! Service manager — owns all shell services, delegates tick/notify.

use crate::service::clipboard::ClipboardManager;
use crate::service::notification::NotificationManager;
use crate::service::power::PowerManager;

pub(crate) struct ServiceManager {
    pub(crate) notifications: NotificationManager,
    pub(crate) clipboard: ClipboardManager,
    pub(crate) power: PowerManager,
}

impl ServiceManager {
    pub fn new() -> Self {
        ServiceManager {
            notifications: NotificationManager::new(),
            clipboard: ClipboardManager::new(),
            power: PowerManager::new(),
        }
    }

    pub fn tick(&mut self, current_tick: u64) {
        self.notifications.tick(current_tick);
        self.power.tick(current_tick);
    }

    pub fn notify(
        &mut self,
        title: &str,
        body: &str,
        urgency: u8,
        timeout: u32,
        current_tick: u64,
    ) -> u64 {
        self.notifications
            .notify_at_tick(title, body, urgency, timeout, current_tick)
    }
}
