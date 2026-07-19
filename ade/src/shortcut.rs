//! Keyboard shortcut registry — map Ctrl+key to actions.

pub(crate) enum ShortcutAction {
    CloseFocused,
    Quit,
    CycleTiling,
    CycleWindow,
    ToggleAot,
    ClipboardPanel,
}

pub(crate) struct ShortcutManager {
    bindings: [(u8, ShortcutAction); 6],
}

impl ShortcutManager {
    pub fn new() -> Self {
        ShortcutManager { bindings: [
            (23, ShortcutAction::CloseFocused), // Ctrl+W
            (17, ShortcutAction::Quit),          // Ctrl+Q
            (20, ShortcutAction::CycleTiling),   // Ctrl+T
            (5,  ShortcutAction::CycleWindow),   // Ctrl+E
            (1,  ShortcutAction::ToggleAot),     // Ctrl+A
            (2,  ShortcutAction::ClipboardPanel),// Ctrl+B
        ]}
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
