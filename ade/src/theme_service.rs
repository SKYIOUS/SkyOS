//! Theme management — access and switch the desktop theme.

use libsarga::theme::Theme;

pub(crate) struct ThemeService {
    theme: Theme,
}

impl ThemeService {
    pub fn new() -> Self {
        ThemeService { theme: Theme::dark() }
    }

    pub fn current(&self) -> &Theme {
        &self.theme
    }

    pub fn set(&mut self, theme: Theme) {
        self.theme = theme;
    }
}
