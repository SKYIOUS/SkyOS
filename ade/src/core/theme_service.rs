use crate::apps::config_store::ConfigStore;
use libsarga::theme::Theme;

pub(crate) struct ThemeService {
    theme: Theme,
}

impl ThemeService {
    pub fn new() -> Self {
        let store = ConfigStore::load();
        let theme = if store.get("theme") == Some("light") {
            Theme::light()
        } else {
            Theme::dark()
        };
        ThemeService { theme }
    }

    pub fn current(&self) -> &Theme {
        &self.theme
    }

    pub fn set(&mut self, theme: Theme) {
        self.theme = theme;
    }
}
