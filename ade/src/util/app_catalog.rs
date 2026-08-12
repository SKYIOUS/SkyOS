//! Application catalog — the single app table, categories, and launch tracking.
//!
//! Merged from the former `app_db` + `app_registry`: the app table, category
//! list, pinned flags, and the one recent-apps list live here together.
//! `recent` is the **single** owner of launch history — the old
//! `SessionManager.recent_apps` parallel tracker was write-only and has been
//! removed.

use alloc::collections::VecDeque;
use alloc::vec;
use alloc::vec::Vec;

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
struct AppEntry {
    name: &'static str,
    /// One-line description shown in the start-menu tooltip.
    desc: &'static str,
    cat: AppCategory,
    exec: &'static str,
    icon: char,
}

static APPS: &[AppEntry] = &[
    AppEntry {
        name: "Terminal",
        desc: "Shell with pty support",
        cat: AppCategory::System,
        exec: "/bin/sash",
        icon: '\x1B',
    },
    AppEntry {
        name: "File Manager",
        desc: "Browse and manage files",
        cat: AppCategory::System,
        exec: "/bin/skyfiles",
        icon: 'F',
    },
    AppEntry {
        name: "SkyStore",
        desc: "Install and update apps",
        cat: AppCategory::Utilities,
        exec: "/bin/skystore",
        icon: 'S',
    },
    AppEntry {
        name: "System Info",
        desc: "Kernel and machine details",
        cat: AppCategory::System,
        exec: "/bin/uname",
        icon: 'i',
    },
    AppEntry {
        name: "Calculator",
        desc: "Basic arithmetic calculator",
        cat: AppCategory::Utilities,
        exec: "/bin/calculator",
        icon: '+',
    },
    AppEntry {
        name: "SkyEdit",
        desc: "Plain-text code editor",
        cat: AppCategory::Development,
        exec: "/bin/skyedit",
        icon: 'E',
    },
    AppEntry {
        name: "Settings",
        desc: "Desktop appearance and options",
        cat: AppCategory::System,
        exec: "/bin/skysettings",
        icon: '\u{2699}',
    },
    AppEntry {
        name: "System Monitor",
        desc: "Processes and memory usage",
        cat: AppCategory::System,
        exec: "/bin/sysmon",
        icon: 'M',
    },
    AppEntry {
        name: "Calendar",
        desc: "Month view calendar",
        cat: AppCategory::Office,
        exec: "/bin/calendar",
        icon: 'D',
    },
    AppEntry {
        name: "Notes",
        desc: "Quick sticky notes",
        cat: AppCategory::Office,
        exec: "/bin/notes",
        icon: 'N',
    },
    AppEntry {
        name: "Paint",
        desc: "Simple drawing canvas",
        cat: AppCategory::Graphics,
        exec: "/bin/paint",
        icon: 'P',
    },
    AppEntry {
        name: "Clock",
        desc: "Digital clock display",
        cat: AppCategory::Office,
        exec: "/bin/clock",
        icon: 'T',
    },
    AppEntry {
        name: "Tasks",
        desc: "Todo and task list",
        cat: AppCategory::Office,
        exec: "/bin/tasks",
        icon: 'K',
    },
    AppEntry {
        name: "Search",
        desc: "Find files and apps",
        cat: AppCategory::Utilities,
        exec: "/bin/search",
        icon: '?',
    },
    AppEntry {
        name: "Archive Manager",
        desc: "Create and extract archives",
        cat: AppCategory::Utilities,
        exec: "/bin/archive",
        icon: 'Z',
    },
    AppEntry {
        name: "Image Viewer",
        desc: "View images and screenshots",
        cat: AppCategory::Graphics,
        exec: "/bin/sargaview",
        icon: 'I',
    },
    AppEntry {
        name: "About SARGA",
        desc: "About this operating system",
        cat: AppCategory::System,
        exec: "",
        icon: 'A',
    },
    // Network
    AppEntry {
        name: "Web Browser",
        desc: "Browse the world wide web",
        cat: AppCategory::Network,
        exec: "/bin/skywebbrowser",
        icon: '@',
    },
    AppEntry {
        name: "Mail",
        desc: "Read and send email",
        cat: AppCategory::Network,
        exec: "/bin/skymail",
        icon: 'M',
    },
    // Multimedia
    AppEntry {
        name: "Media Player",
        desc: "Play audio and video",
        cat: AppCategory::Multimedia,
        exec: "/bin/skymedia",
        icon: '\u{266B}',
    },
    AppEntry {
        name: "Audio Recorder",
        desc: "Record audio input",
        cat: AppCategory::Multimedia,
        exec: "/bin/skyrecorder",
        icon: 'R',
    },
    // Games
    AppEntry {
        name: "Chess",
        desc: "Play chess against a friend",
        cat: AppCategory::Games,
        exec: "/bin/skychess",
        icon: '\u{265E}',
    },
    AppEntry {
        name: "Sudoku",
        desc: "Number puzzle game",
        cat: AppCategory::Games,
        exec: "/bin/skysudoku",
        icon: '9',
    },
    // More utilities
    AppEntry {
        name: "Text Extractor",
        desc: "Pull text from documents",
        cat: AppCategory::Utilities,
        exec: "/bin/skyextractor",
        icon: 'X',
    },
    AppEntry {
        name: "Disk Usage",
        desc: "Space used per directory",
        cat: AppCategory::Utilities,
        exec: "/bin/skydisk",
        icon: 'D',
    },
    AppEntry {
        name: "Package Manager",
        desc: "Install and remove packages",
        cat: AppCategory::Utilities,
        exec: "/bin/spkg",
        icon: '#',
    },
    AppEntry {
        name: "Screenshot",
        desc: "Capture the whole screen",
        cat: AppCategory::Utilities,
        exec: "/bin/skyscreenshot",
        icon: 'C',
    },
    AppEntry {
        name: "IPC Echo",
        desc: "Test the IPC transport",
        cat: AppCategory::Utilities,
        exec: "/bin/ipc_echo",
        icon: '=',
    },
];

pub(crate) static CATEGORIES: &[(&str, AppCategory)] = &[
    ("All", AppCategory::All),
    ("Favorites", AppCategory::Favorites),
    ("System", AppCategory::System),
    ("Development", AppCategory::Development),
    ("Office", AppCategory::Office),
    ("Graphics", AppCategory::Graphics),
    ("Network", AppCategory::Network),
    ("Multimedia", AppCategory::Multimedia),
    ("Games", AppCategory::Games),
    ("Utilities", AppCategory::Utilities),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AppId(pub usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StartupMode {
    Normal,
    Singleton,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct AppInfo {
    pub id: AppId,
    pub name: &'static str,
    /// One-line description shown in the start-menu tooltip.
    pub description: &'static str,
    pub icon: char,
    pub category: AppCategory,
    pub executable: &'static str,
    pub startup_mode: StartupMode,
}

/// The single app catalog: the installed-app list, pin flags, and the one
/// recent-apps queue. This is the only place launch history lives.
pub(crate) struct AppCatalog {
    pub apps: Vec<AppInfo>,
    pub pinned: Vec<bool>,
    pub recent: VecDeque<usize>,
}

impl AppCatalog {
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
                    description: entry.desc,
                    icon,
                    category: entry.cat,
                    executable: entry.exec,
                    startup_mode: if entry.name == "Settings" {
                        StartupMode::Singleton
                    } else {
                        StartupMode::Normal
                    },
                }
            })
            .collect();
        AppCatalog {
            apps,
            pinned: vec![false; APPS.len()],
            recent: VecDeque::new(),
        }
    }

    /// Record a launch: move the app to the front of the recent queue
    /// (most-recent-first, capped at 10).
    pub fn record_launch(&mut self, id: AppId) {
        self.recent.retain(|&i| i != id.0);
        self.recent.push_front(id.0);
        if self.recent.len() > 10 {
            self.recent.pop_back();
        }
    }

    pub fn get(&self, id: AppId) -> Option<&AppInfo> {
        self.apps.get(id.0)
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
            if cat == AppCategory::Favorites && !self.pinned[i] {
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
                let pa = self.pinned[a.0] as u8;
                let pb = self.pinned[b.0] as u8;
                pb.cmp(&pa)
                    .then_with(|| self.apps[a.0].name.cmp(self.apps[b.0].name))
            });
        }
        result
    }
}
