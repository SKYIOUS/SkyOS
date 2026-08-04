//! Config store — global settings key-value store (not yet persisted).

pub(crate) struct ConfigStore {
    pub theme_dark: bool,
    // keep: defaults for future window-blend settings
    #[allow(dead_code)]
    pub window_opacity: u8,
    #[allow(dead_code)]
    pub notification_timeout: u32,
}

impl ConfigStore {
    pub fn new() -> Self {
        ConfigStore {
            theme_dark: true,
            window_opacity: 255,
            notification_timeout: 120,
        }
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        match key {
            "theme" => Some(if self.theme_dark { "dark" } else { "light" }),
            _ => None,
        }
    }

    // keep: setter reserved until settings UI writes to the store
    #[allow(dead_code)]
    pub fn set(&mut self, key: &str, value: &str) {
        match (key, value) {
            ("theme", "dark") => self.theme_dark = true,
            ("theme", "light") => self.theme_dark = false,
            _ => {}
        }
    }

    // keep: persistence stub, store not yet backed by disk
    #[allow(dead_code)]
    pub fn save(&self) {}

    pub fn load() -> Self {
        Self::new()
    }
}
