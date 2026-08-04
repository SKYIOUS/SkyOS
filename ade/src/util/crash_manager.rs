#![allow(dead_code)]

use crate::ipc::ApplicationId;
use alloc::vec::Vec;

pub(crate) struct CrashEntry {
    pub app_id: ApplicationId,
    pub crash_count: u32,
    pub last_crash_tick: u64,
    pub panic_reason: Option<&'static str>,
    pub restart_requested: bool,
}

pub(crate) struct CrashManager {
    pub entries: Vec<CrashEntry>,
}

impl CrashManager {
    pub fn new() -> Self {
        CrashManager {
            entries: Vec::new(),
        }
    }

    pub fn report_crash(&mut self, app_id: ApplicationId, tick: u64, reason: Option<&'static str>) {
        if let Some(e) = self.entries.iter_mut().find(|e| e.app_id == app_id) {
            e.crash_count += 1;
            e.last_crash_tick = tick;
            e.panic_reason = reason;
        } else {
            self.entries.push(CrashEntry {
                app_id,
                crash_count: 1,
                last_crash_tick: tick,
                panic_reason: reason,
                restart_requested: false,
            });
        }
    }

    pub fn request_restart(&mut self, app_id: ApplicationId) {
        if let Some(e) = self.entries.iter_mut().find(|e| e.app_id == app_id) {
            e.restart_requested = true;
        }
    }

    pub fn should_restart(&self, app_id: ApplicationId) -> bool {
        self.entries
            .iter()
            .find(|e| e.app_id == app_id)
            .is_some_and(|e| e.crash_count < 3 && !e.restart_requested)
    }

    pub fn tick(&mut self) {}
}
