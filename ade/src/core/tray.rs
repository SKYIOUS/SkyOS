//! System tray — background service indicators in the taskbar.

pub(crate) struct TrayEntry {
    pub icon: char,
}

pub(crate) struct SystemTray {
    pub entries: &'static [TrayEntry],
}

impl SystemTray {
    pub fn new() -> Self {
        SystemTray {
            entries: &[
                TrayEntry { icon: 'N' },
                TrayEntry { icon: 'S' },
                TrayEntry { icon: 'B' },
            ],
        }
    }
}
