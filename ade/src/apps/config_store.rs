//! Config store — global settings key-value store (not yet persisted).

pub(crate) struct ConfigStore {
    pub theme_dark: bool,
}

impl ConfigStore {
    pub fn new() -> Self {
        ConfigStore { theme_dark: true }
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        match key {
            "theme" => Some(if self.theme_dark { "dark" } else { "light" }),
            _ => None,
        }
    }

    pub fn load() -> Self {
        Self::new()
    }
}
