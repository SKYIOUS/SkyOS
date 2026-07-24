//! Service manager — owns all shell services, delegates tick/notify.

use crate::service::clipboard::ClipboardManager;
use crate::service::notification::NotificationManager;
use crate::service::power::PowerManager;
use crate::service::session::SessionManager;

pub(crate) struct ServiceManager {
    pub(crate) notifications: NotificationManager,
    pub(crate) clipboard: ClipboardManager,
    pub(crate) session: SessionManager,
    pub(crate) power: PowerManager,
}

impl ServiceManager {
    pub fn new(boot_tick: u64) -> Self {
        ServiceManager {
            notifications: NotificationManager::new(),
            clipboard: ClipboardManager::new(),
            session: SessionManager::new(boot_tick),
            power: PowerManager::new(),
        }
    }

    pub fn tick(&mut self, current_tick: u64) {
        self.notifications.tick(current_tick);
        self.power.tick(current_tick);
    }

    pub fn notify(&mut self, title: &str, body: &str, urgency: u8, timeout: u32) -> u64 {
        self.notifications.notify(title, body, urgency, timeout)
    }
}
