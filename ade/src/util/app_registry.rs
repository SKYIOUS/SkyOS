//! Application registry — installed apps, metadata, capabilities, launch tracking.

use crate::util::app_db::{AppCategory, AppDb, APPS};
use alloc::vec::Vec;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AppId(pub usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StartupMode {
    Normal,
    #[allow(dead_code)] // reserved startup mode, matched in desktop::launch_app
    Background,
    Singleton,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct AppInfo {
    pub id: AppId,
    pub name: &'static str,
    pub icon: char,
    pub category: AppCategory,
    pub executable: &'static str,
    #[allow(dead_code)] // app metadata surface
    pub description: &'static str,
    #[allow(dead_code)] // app metadata surface
    pub version: &'static str,
    pub startup_mode: StartupMode,
}

pub(crate) struct AppRegistry {
    pub apps: Vec<AppInfo>,
    pub db: AppDb,
}

impl AppRegistry {
    pub fn new() -> Self {
        let apps = APPS
            .iter()
            .enumerate()
            .map(|(i, entry)| {
                let icon = if entry.icon != '\0' {
                    entry.icon
                } else {
                    entry.name.as_bytes()[0] as char
                };
                AppInfo {
                    id: AppId(i),
                    name: entry.name,
                    icon,
                    category: entry.cat,
                    executable: entry.exec,
                    description: entry.desc,
                    version: "0.1.0",
                    startup_mode: if entry.name == "Settings" {
                        StartupMode::Singleton
                    } else {
                        StartupMode::Normal
                    },
                }
            })
            .collect();
        AppRegistry {
            apps,
            db: AppDb::new(APPS.len()),
        }
    }

    pub fn record_launch(&mut self, id: AppId) {
        if id.0 < self.apps.len() {
            self.apps[id.0].id = id;
        }
        self.db.record_launch(id.0);
    }

    pub fn find_by_exec(&self, exec: &str) -> Option<AppId> {
        self.apps
            .iter()
            .find(|a| a.executable == exec)
            .map(|a| a.id)
    }

    #[allow(dead_code)] // registry query API surface
    pub fn find_by_name(&self, name: &str) -> Option<AppId> {
        self.apps.iter().find(|a| a.name == name).map(|a| a.id)
    }

    #[allow(dead_code)] // registry query API surface
    pub fn find_by_permission(&self, _perm: u32) -> alloc::vec::Vec<AppId> {
        alloc::vec::Vec::new()
    }

    #[allow(dead_code)] // registry query API surface
    pub fn find_by_category(&self, _category: &str) -> alloc::vec::Vec<AppId> {
        alloc::vec::Vec::new()
    }

    #[allow(dead_code)] // registry query API surface
    pub fn apps_by_category(&self, cat: AppCategory) -> Vec<AppId> {
        self.apps
            .iter()
            .filter(|a| a.category == cat)
            .map(|a| a.id)
            .collect()
    }

    pub fn get(&self, id: AppId) -> Option<&AppInfo> {
        self.apps.get(id.0)
    }

    #[allow(dead_code)] // registry query API surface
    pub fn all_apps(&self) -> &[AppInfo] {
        &self.apps
    }

    pub fn filtered(&self, cat: AppCategory, search: &[u8]) -> Vec<AppId> {
        let query = core::str::from_utf8(search).unwrap_or("");
        let query_lower: Vec<u8> = query.bytes().map(|b| b.to_ascii_lowercase()).collect();
        let mut result = Vec::new();
        for app in &self.apps {
            let i = app.id.0;
            if cat != AppCategory::All && app.category != cat && cat != AppCategory::Favorites {
                continue;
            }
            if cat == AppCategory::Favorites && !self.db.pinned[i] {
                continue;
            }
            if !query_lower.is_empty() {
                let name_lower: Vec<u8> =
                    app.name.bytes().map(|b| b.to_ascii_lowercase()).collect();
                if !name_lower
                    .windows(query_lower.len())
                    .any(|w| w == &query_lower[..])
                {
                    continue;
                }
            }
            result.push(app.id);
        }
        if cat == AppCategory::All || query_lower.is_empty() {
            result.sort_by(|a, b| {
                let pa = self.db.pinned[a.0] as u8;
                let pb = self.db.pinned[b.0] as u8;
                pb.cmp(&pa)
                    .then_with(|| self.apps[a.0].name.cmp(self.apps[b.0].name))
            });
        }
        result
    }
}
