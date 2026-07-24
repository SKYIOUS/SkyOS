#![allow(dead_code)]

use crate::ipc::ApplicationId;
use alloc::vec::Vec;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AppState {
    Running,
    Suspended,
    Background,
    Closing,
    Crashed,
    Restarting,
}

pub(crate) struct AppLifecycleEntry {
    pub app_id: ApplicationId,
    pub state: AppState,
    pub restart_count: u32,
    pub started_at: u64,
}

pub(crate) struct AppLifecycleManager {
    pub apps: Vec<AppLifecycleEntry>,
}

impl AppLifecycleManager {
    pub fn new() -> Self {
        AppLifecycleManager {
            apps: Vec::new(),
        }
    }

    pub fn register(&mut self, app_id: ApplicationId) {
        self.apps.push(AppLifecycleEntry {
            app_id,
            state: AppState::Running,
            restart_count: 0,
            started_at: 0,
        });
    }

    pub fn unregister(&mut self, app_id: ApplicationId) {
        self.apps.retain(|e| e.app_id != app_id);
    }

    pub fn set_state(&mut self, app_id: ApplicationId, state: AppState) {
        if let Some(entry) = self.apps.iter_mut().find(|e| e.app_id == app_id) {
            entry.state = state;
        }
    }

    pub fn state(&self, app_id: ApplicationId) -> Option<AppState> {
        self.apps.iter().find(|e| e.app_id == app_id).map(|e| e.state)
    }

    pub fn tick(&mut self) {}
}
