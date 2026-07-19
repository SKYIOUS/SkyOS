//! Application registry — installed apps, metadata, capabilities, launch tracking.

use alloc::vec::Vec;
use crate::app_db::{AppDb, AppEntry, APPS};

#[allow(dead_code)]
pub(crate) struct RegisteredApp {
    pub entry: &'static AppEntry,
    pub version: &'static str,
    pub capabilities: u32,
    pub icon: char,
    pub launch_count: u32,
}

pub(crate) struct AppRegistry {
    pub apps: Vec<RegisteredApp>,
    pub db: AppDb,
}

impl AppRegistry {
    pub fn new() -> Self {
        let apps = APPS.iter().map(|entry| RegisteredApp {
            entry,
            version: "0.1.0",
            capabilities: cap_of(entry.cat),
            icon: entry.name.as_bytes()[0] as char,
            launch_count: 0,
        }).collect();
        AppRegistry { apps, db: AppDb::new() }
    }

    pub fn record_launch(&mut self, idx: usize) {
        if idx < self.apps.len() {
            self.apps[idx].launch_count += 1;
        }
        self.db.record_launch(idx);
    }

    #[allow(dead_code)]
    pub fn filtered(&self, cat: crate::app_db::AppCategory, search: &[u8]) -> Vec<usize> {
        self.db.filtered(cat, search)
    }
}

fn cap_of(cat: crate::app_db::AppCategory) -> u32 {
    match cat {
        crate::app_db::AppCategory::System => 0x0001,
        crate::app_db::AppCategory::Development => 0x0002,
        crate::app_db::AppCategory::Office => 0x0004,
        crate::app_db::AppCategory::Graphics => 0x0008,
        crate::app_db::AppCategory::Network => 0x0010,
        crate::app_db::AppCategory::Multimedia => 0x0020,
        crate::app_db::AppCategory::Games => 0x0040,
        crate::app_db::AppCategory::Utilities => 0x0080,
        _ => 0x0000,
    }
}
