//! Session manager — boot/login tracking, shutdown/restart/logout requests, recent apps.

use alloc::collections::VecDeque;

pub(crate) struct SessionManager {
    boot_tick: u64,
    login_tick: u64,
    pub(crate) shutdown_requested: bool,
    pub(crate) restart_requested: bool,
    pub(crate) logout_requested: bool,
    pub(crate) desktop_state_saved: bool,
    pub(crate) recent_apps: VecDeque<u64>,
}

impl SessionManager {
    pub fn new(boot_tick: u64) -> Self {
        SessionManager {
            boot_tick,
            login_tick: boot_tick,
            shutdown_requested: false,
            restart_requested: false,
            logout_requested: false,
            desktop_state_saved: false,
            recent_apps: VecDeque::new(),
        }
    }

    pub fn uptime(&self, current_tick: u64) -> u64 {
        current_tick.saturating_sub(self.boot_tick)
    }

    pub fn session_duration(&self, current_tick: u64) -> u64 {
        current_tick.saturating_sub(self.login_tick)
    }

    pub fn request_shutdown(&mut self) {
        self.shutdown_requested = true;
    }

    pub fn request_restart(&mut self) {
        self.restart_requested = true;
    }

    pub fn request_logout(&mut self) {
        self.logout_requested = true;
    }

    pub fn mark_state_saved(&mut self) {
        self.desktop_state_saved = true;
    }

    pub fn record_app_launch(&mut self, app_id: u64) {
        self.recent_apps.retain(|&id| id != app_id);
        self.recent_apps.push_front(app_id);
        if self.recent_apps.len() > 10 {
            self.recent_apps.pop_back();
        }
    }

    pub fn reset(&mut self, current_tick: u64) {
        self.login_tick = current_tick;
        self.shutdown_requested = false;
        self.restart_requested = false;
        self.logout_requested = false;
        self.desktop_state_saved = false;
    }
}
