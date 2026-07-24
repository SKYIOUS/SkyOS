//! Keyboard shortcut registry — map Ctrl+key to actions.

pub(crate) enum ShortcutAction {
    CloseFocused,
    Quit,
    CycleTiling,
    CycleWindow,
    ToggleAot,
    ClipboardPanel,
    DemoNotification,
    DismissNotification,
    ClearNotifications,
    OpenSettings,
    OpenTaskManager,
}

pub(crate) struct ShortcutManager {
    bindings: [(u8, ShortcutAction); 11],
}

impl ShortcutManager {
    pub fn new() -> Self {
        ShortcutManager {
            bindings: [
                (23, ShortcutAction::CloseFocused),   // Ctrl+W
                (17, ShortcutAction::Quit),           // Ctrl+Q
                (20, ShortcutAction::CycleTiling),    // Ctrl+T
                (5, ShortcutAction::CycleWindow),     // Ctrl+E
                (1, ShortcutAction::ToggleAot),       // Ctrl+A
                (2, ShortcutAction::ClipboardPanel),  // Ctrl+B
                (14, ShortcutAction::DemoNotification), // Ctrl+N
                (4, ShortcutAction::DismissNotification), // Ctrl+D
                (3, ShortcutAction::ClearNotifications), // Ctrl+C
                (19, ShortcutAction::OpenSettings),   // Ctrl+Shift+S
                (24, ShortcutAction::OpenTaskManager), // Ctrl+Shift+X
            ],
        }
    }

    pub fn handle(&self, key: u8) -> Option<&ShortcutAction> {
        for (k, a) in &self.bindings {
            if *k == key {
                return Some(a);
            }
        }
        None
    }
}
