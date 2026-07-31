//! Application database — desktop entries, categories, pinned/recent tracking.

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
pub(crate) struct AppEntry {
    pub name: &'static str,
    pub cat: AppCategory,
    pub exec: &'static str,
    pub desc: &'static str,
    pub icon: char,
}

pub(crate) static APPS: &[AppEntry] = &[
    AppEntry { name: "Terminal",        cat: AppCategory::System,       exec: "/bin/sash",          desc: "System terminal emulator",     icon: '\x1B' },
    AppEntry { name: "File Manager",    cat: AppCategory::System,       exec: "/bin/skyfiles",      desc: "Browse files and folders",     icon: 'F' },
    AppEntry { name: "SkyStore",        cat: AppCategory::Utilities,    exec: "/bin/skystore",      desc: "Application marketplace",      icon: 'S' },
    AppEntry { name: "System Info",     cat: AppCategory::System,       exec: "/bin/uname",         desc: "View system information",      icon: 'i' },
    AppEntry { name: "Calculator",      cat: AppCategory::Utilities,    exec: "/bin/calculator",   desc: "Simple calculator",            icon: '+' },
    AppEntry { name: "SkyEdit",         cat: AppCategory::Development,  exec: "/bin/skyedit",       desc: "Text editor",                  icon: 'E' },
    AppEntry { name: "Settings",        cat: AppCategory::System,       exec: "/bin/skysettings",   desc: "System settings",              icon: '\u{2699}' },
    AppEntry { name: "System Monitor",  cat: AppCategory::System,       exec: "/bin/sysmon",        desc: "Monitor system resources",     icon: 'M' },
    AppEntry { name: "Calendar",        cat: AppCategory::Office,       exec: "/bin/calendar",      desc: "Calendar application",         icon: 'D' },
    AppEntry { name: "Notes",           cat: AppCategory::Office,       exec: "/bin/notes",         desc: "Take notes",                   icon: 'N' },
    AppEntry { name: "Paint",           cat: AppCategory::Graphics,     exec: "/bin/paint",         desc: "Simple paint program",         icon: 'P' },
    AppEntry { name: "Clock",           cat: AppCategory::Office,       exec: "/bin/clock",         desc: "Clock and alarms",             icon: 'T' },
    AppEntry { name: "Tasks",           cat: AppCategory::Office,       exec: "/bin/tasks",         desc: "Task management",              icon: 'K' },
    AppEntry { name: "Search",          cat: AppCategory::Utilities,    exec: "/bin/search",        desc: "Search files and apps",        icon: '?' },
    AppEntry { name: "Archive Manager", cat: AppCategory::Utilities,    exec: "/bin/archive",       desc: "Archive management",           icon: 'Z' },
    AppEntry { name: "Image Viewer",    cat: AppCategory::Graphics,     exec: "/bin/sargaview",     desc: "View images",                  icon: 'I' },
    AppEntry { name: "About SARGA",     cat: AppCategory::System,       exec: "",                   desc: "About the operating system",   icon: 'A' },
    // Network
    AppEntry { name: "Web Browser",     cat: AppCategory::Network,      exec: "/bin/skywebbrowser", desc: "Browse the web",               icon: '@' },
    AppEntry { name: "Mail",            cat: AppCategory::Network,      exec: "/bin/skymail",       desc: "Email client",                 icon: 'M' },
    // Multimedia
    AppEntry { name: "Media Player",    cat: AppCategory::Multimedia,   exec: "/bin/skymedia",      desc: "Play audio and video",         icon: '\u{266B}' },
    AppEntry { name: "Audio Recorder",  cat: AppCategory::Multimedia,   exec: "/bin/skyrecorder",   desc: "Record audio",                 icon: 'R' },
    // Games
    AppEntry { name: "Chess",           cat: AppCategory::Games,        exec: "/bin/skychess",      desc: "Play chess",                   icon: '\u{265E}' },
    AppEntry { name: "Sudoku",          cat: AppCategory::Games,        exec: "/bin/skysudoku",     desc: "Number puzzle game",           icon: '9' },
    // More utilities
    AppEntry { name: "Text Extractor",  cat: AppCategory::Utilities,    exec: "/bin/skyextractor",  desc: "Extract text from files",      icon: 'X' },
    AppEntry { name: "Disk Usage",      cat: AppCategory::Utilities,    exec: "/bin/skydisk",       desc: "Analyze disk space usage",     icon: 'D' },
    AppEntry { name: "Package Manager", cat: AppCategory::Utilities,    exec: "/bin/skypkg",        desc: "Install and remove packages",  icon: '#' },
    AppEntry { name: "Screenshot",      cat: AppCategory::Utilities,    exec: "/bin/skyscreenshot", desc: "Capture the screen",           icon: 'C' },
    AppEntry { name: "IPC Echo",        cat: AppCategory::Utilities,    exec: "/bin/ipc_echo",      desc: "Test the ADE IPC channel",     icon: '=' },
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

pub(crate) struct AppDb {
    pub pinned: Vec<bool>,
    pub recent: VecDeque<usize>,
}

impl AppDb {
    pub fn new(count: usize) -> Self {
        AppDb {
            pinned: vec![false; count],
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
}
