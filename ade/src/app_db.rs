//! Application database — desktop entries, categories, pinned/recent tracking.

use alloc::vec::Vec;
use alloc::vec;
use alloc::collections::VecDeque;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AppCategory {
    All,
    Favorites,
    System,
    Development,
    Office,
    Graphics,
    Network,
    Multimedia,
    Games,
    Utilities,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct AppEntry {
    pub name: &'static str,
    pub cat: AppCategory,
    pub exec: &'static str,
}

pub(crate) static APPS: &[AppEntry] = &[
    AppEntry { name: "Terminal",      cat: AppCategory::System,       exec: "/bin/sash" },
    AppEntry { name: "File Manager",  cat: AppCategory::System,       exec: "/bin/skyfiles" },
    AppEntry { name: "SkyStore",      cat: AppCategory::Utilities,    exec: "/bin/skystore" },
    AppEntry { name: "System Info",   cat: AppCategory::System,       exec: "/bin/uname" },
    AppEntry { name: "Calculator",    cat: AppCategory::Utilities,    exec: "/bin/calculator" },
    AppEntry { name: "SkyEdit",       cat: AppCategory::Development,  exec: "/bin/skyedit" },
    AppEntry { name: "Settings",      cat: AppCategory::System,       exec: "/bin/skysettings" },
    AppEntry { name: "System Monitor",cat: AppCategory::System,       exec: "/bin/sysmon" },
    AppEntry { name: "Calendar",      cat: AppCategory::Office,       exec: "/bin/calendar" },
    AppEntry { name: "Notes",         cat: AppCategory::Office,       exec: "/bin/notes" },
    AppEntry { name: "Paint",         cat: AppCategory::Graphics,     exec: "/bin/paint" },
    AppEntry { name: "Clock",         cat: AppCategory::Utilities,    exec: "/bin/clock" },
    AppEntry { name: "Tasks",         cat: AppCategory::Office,       exec: "/bin/tasks" },
    AppEntry { name: "Search",        cat: AppCategory::Utilities,    exec: "/bin/search" },
    AppEntry { name: "File Browser",  cat: AppCategory::System,       exec: "/bin/skyfiles" },
    AppEntry { name: "About SARGA",   cat: AppCategory::System,       exec: "" },
];

pub(crate) static CATEGORIES: &[(&str, AppCategory)] = &[
    ("All",         AppCategory::All),
    ("Favorites",   AppCategory::Favorites),
    ("System",      AppCategory::System),
    ("Development", AppCategory::Development),
    ("Office",      AppCategory::Office),
    ("Graphics",    AppCategory::Graphics),
    ("Network",     AppCategory::Network),
    ("Multimedia",  AppCategory::Multimedia),
    ("Games",       AppCategory::Games),
    ("Utilities",   AppCategory::Utilities),
];

pub(crate) struct AppDb {
    pub pinned: Vec<bool>,
    pub recent: VecDeque<usize>,
}

impl AppDb {
    pub fn new() -> Self {
        AppDb {
            pinned: vec![false; APPS.len()],
            recent: VecDeque::new(),
        }
    }

    pub fn record_launch(&mut self, idx: usize) {
        self.recent.retain(|&i| i != idx);
        self.recent.push_front(idx);
        if self.recent.len() > 10 {
            self.recent.pop_back();
        }
    }

    #[allow(dead_code)]
    pub fn toggle_pin(&mut self, idx: usize) {
        if idx < self.pinned.len() {
            self.pinned[idx] = !self.pinned[idx];
        }
    }

    pub fn filtered(&self, cat: AppCategory, search: &[u8]) -> Vec<usize> {
        let query = core::str::from_utf8(search).unwrap_or("");
        let query_lower: Vec<u8> = query.bytes().map(|b| b.to_ascii_lowercase()).collect();
        let mut result = Vec::new();
        for i in 0..APPS.len() {
            if cat != AppCategory::All && APPS[i].cat != cat && cat != AppCategory::Favorites {
                continue;
            }
            if cat == AppCategory::Favorites && !self.pinned[i] {
                continue;
            }
            if !query_lower.is_empty() {
                let name_lower: Vec<u8> = APPS[i].name.bytes().map(|b| b.to_ascii_lowercase()).collect();
                if !name_lower.windows(query_lower.len()).any(|w| w == &query_lower[..]) {
                    continue;
                }
            }
            result.push(i);
        }
        // pinned items first
        if cat == AppCategory::All || query_lower.is_empty() {
            result.sort_by(|a, b| {
                let pa = self.pinned[*a] as u8;
                let pb = self.pinned[*b] as u8;
                pb.cmp(&pa).then_with(|| APPS[*a].name.cmp(APPS[*b].name))
            });
        }
        result
    }
}
