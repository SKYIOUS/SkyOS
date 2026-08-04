use crate::apps::config_store::ConfigStore;
use libsarga::theme::Theme;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ThemeKind {
    Dark,
    Light,
    #[allow(dead_code)] // high-contrast theme, no toggle in settings UI yet
    HighContrast,
}

pub(crate) struct ThemeService {
    theme: Theme,
    #[allow(dead_code)] // kind kept for future settings persistence/API
    kind: ThemeKind,
    accent: u32,
}

pub(crate) fn high_contrast_theme() -> Theme {
    Theme {
        bg_primary: 0xFF000000,
        bg_surface: 0xFF000000,
        bg_elevated: 0xFF111111,
        accent: 0xFFFFFFFF,
        accent_light: 0xFFCCCCCC,
        accent_dark: 0xFFAAAAAA,
        text: 0xFFFFFFFF,
        text_secondary: 0xFFCCCCCC,
        text_disabled: 0xFF888888,
        border: 0xFFFFFFFF,
        hover: 0xFF333333,
        pressed: 0xFF222222,
        error: 0xFFFF4444,
        success: 0xFF44FF44,
        warning: 0xFFFFFF44,
        separator: 0xFFFFFFFF,
        shadow: 0x00000000,
        font_size: 14,
        border_radius: 8,
        padding: 10,
        spacing: 6,
    }
}

impl ThemeService {
    pub fn new() -> Self {
        let store = ConfigStore::load();
        let kind = if store.get("theme") == Some("light") {
            ThemeKind::Light
        } else {
            ThemeKind::Dark
        };
        let t = match kind {
            ThemeKind::Dark => Theme::dark(),
            ThemeKind::Light => Theme::light(),
            ThemeKind::HighContrast => high_contrast_theme(),
        };
        let accent = t.accent;
        ThemeService {
            theme: t,
            kind,
            accent,
        }
    }

    pub fn current(&self) -> &Theme {
        &self.theme
    }

    pub fn set(&mut self, theme: Theme) {
        let accent = theme.accent;
        self.theme = theme;
        self.accent = accent;
    }

    #[allow(dead_code)] // theme query API, callers use current()/set() today
    pub fn kind(&self) -> ThemeKind {
        self.kind
    }

    #[allow(dead_code)] // theme mutation API, callers use set() today
    pub fn set_kind(&mut self, kind: ThemeKind) {
        self.kind = kind;
        self.theme = match kind {
            ThemeKind::Dark => Theme::dark(),
            ThemeKind::Light => Theme::light(),
            ThemeKind::HighContrast => high_contrast_theme(),
        };
        self.accent = self.theme.accent;
    }

    #[allow(dead_code)] // theme mutation API, callers use set() today
    pub fn set_accent(&mut self, color: u32) {
        self.accent = color;
        self.theme.accent = color;
    }

    #[allow(dead_code)] // theme mutation API, callers use set() today
    pub fn set_dark(&mut self) {
        self.set_kind(ThemeKind::Dark);
    }

    #[allow(dead_code)] // theme mutation API, callers use set() today
    pub fn set_light(&mut self) {
        self.set_kind(ThemeKind::Light);
    }

    #[allow(dead_code)] // accent query API, callers use current() today
    pub fn accent(&self) -> u32 {
        self.accent
    }
}
