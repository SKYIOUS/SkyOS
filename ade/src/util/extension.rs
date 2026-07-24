// Scaffold — used by future phase
#![allow(dead_code)]
//! Extension Framework — desktop widgets, panels, dock, wallpaper, notification plugins, themes.
//!
//! Provides safe extension points for desktop customization and enhancement.

use alloc::string::String;
use alloc::vec::Vec;

/// Extension type
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExtensionType {
    /// Desktop widget (floating/pinned)
    Widget,
    /// Panel (taskbar-like)
    Panel,
    /// Dock extension
    DockExtension,
    /// Wallpaper engine
    Wallpaper,
    /// Notification handler
    NotificationHandler,
    /// Theme provider
    Theme,
    /// Status bar indicator
    StatusIndicator,
}

/// Widget position
#[derive(Clone, Copy, Debug)]
pub enum WidgetPosition {
    Floating(i32, i32),     // x, y
    CornerTopLeft,
    CornerTopRight,
    CornerBottomLeft,
    CornerBottomRight,
    EdgeTop,
    EdgeBottom,
    EdgeLeft,
    EdgeRight,
}

/// Widget configuration
#[derive(Clone)]
pub struct Widget {
    pub id: String,
    pub extension_id: String,
    pub position: WidgetPosition,
    pub width: u16,
    pub height: u16,
    pub always_on_top: bool,
    pub enabled: bool,
}

/// Panel configuration
#[derive(Clone)]
pub struct Panel {
    pub id: String,
    pub extension_id: String,
    pub position: WidgetPosition,
    pub height: u16,
    pub enabled: bool,
}

/// Theme metadata
#[derive(Clone)]
pub struct Theme {
    pub id: String,
    pub name: String,
    pub author: String,
    pub primary_color: u32,
    pub secondary_color: u32,
    pub accent_color: u32,
}

/// Wallpaper configuration
#[derive(Clone)]
pub struct Wallpaper {
    pub id: String,
    pub name: String,
    pub path: String,
    pub mode: WallpaperMode,
}

/// Wallpaper scaling mode
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WallpaperMode {
    Stretch,
    Fit,
    Fill,
    Tile,
    Center,
}

/// Status indicator
#[derive(Clone)]
pub struct StatusIndicator {
    pub id: String,
    pub extension_id: String,
    pub label: String,
    pub icon: String,
    pub enabled: bool,
}

/// Notification handler metadata
#[derive(Clone)]
pub struct NotificationHandler {
    pub id: String,
    pub extension_id: String,
    pub name: String,
    pub enabled: bool,
}

/// Extension metadata
#[derive(Clone)]
pub struct Extension {
    pub id: String,
    pub name: String,
    pub extension_type: ExtensionType,
    pub version: String,
    pub author: String,
    pub enabled: bool,
}

/// Extension event
#[derive(Clone, Copy, Debug)]
pub enum ExtensionEvent {
    /// Extension enabled
    Enabled(u32),
    /// Extension disabled
    Disabled(u32),
    /// Widget created
    WidgetCreated(u32),
    /// Widget removed
    WidgetRemoved(u32),
    /// Panel created
    PanelCreated(u32),
    /// Panel removed
    PanelRemoved(u32),
    /// Theme changed
    ThemeChanged(u32),
    /// Wallpaper changed
    WallpaperChanged(u32),
}

/// Extension Manager
pub struct ExtensionManager {
    /// Registered extensions
    extensions: Vec<Extension>,
    /// Active widgets
    widgets: Vec<Widget>,
    /// Active panels
    panels: Vec<Panel>,
    /// Available themes
    themes: Vec<Theme>,
    /// Current theme
    current_theme: Option<String>,
    /// Current wallpaper
    current_wallpaper: Option<String>,
    /// Status indicators
    indicators: Vec<StatusIndicator>,
    /// Notification handlers
    notification_handlers: Vec<NotificationHandler>,
    /// Pending events
    events: Vec<ExtensionEvent>,
}

impl ExtensionManager {
    /// Create a new extension manager
    pub fn new() -> Self {
        ExtensionManager {
            extensions: Vec::new(),
            widgets: Vec::new(),
            panels: Vec::new(),
            themes: Vec::new(),
            current_theme: None,
            current_wallpaper: None,
            indicators: Vec::new(),
            notification_handlers: Vec::new(),
            events: Vec::new(),
        }
    }

    /// Register an extension
    pub fn register_extension(&mut self, extension: Extension) -> u32 {
        self.extensions.push(extension);
        self.extensions.len() as u32 - 1
    }

    /// Get extension by ID
    pub fn get_extension(&self, ext_id: &str) -> Option<&Extension> {
        self.extensions.iter().find(|e| e.id == ext_id)
    }

    /// Enable/disable extension
    pub fn set_extension_enabled(&mut self, ext_id: &str, enabled: bool) -> bool {
        if let Some(ext) = self.extensions.iter_mut().find(|e| e.id == ext_id) {
            ext.enabled = enabled;
            true
        } else {
            false
        }
    }

    /// Create widget from extension
    pub fn create_widget(&mut self, ext_id: &str, widget_id: &str, position: WidgetPosition, width: u16, height: u16) -> bool {
        if self.get_extension(ext_id).is_none() {
            return false;
        }

        self.widgets.push(Widget {
            id: String::from(widget_id),
            extension_id: String::from(ext_id),
            position,
            width,
            height,
            always_on_top: false,
            enabled: true,
        });

        self.events.push(ExtensionEvent::WidgetCreated(0));
        true
    }

    /// Remove widget
    pub fn remove_widget(&mut self, widget_id: &str) -> bool {
        if let Some(pos) = self.widgets.iter().position(|w| w.id == widget_id) {
            self.widgets.remove(pos);
            self.events.push(ExtensionEvent::WidgetRemoved(0));
            true
        } else {
            false
        }
    }

    /// Get all widgets
    pub fn widgets(&self) -> &[Widget] {
        &self.widgets
    }

    /// Create panel
    pub fn create_panel(&mut self, ext_id: &str, panel_id: &str, position: WidgetPosition, height: u16) -> bool {
        if self.get_extension(ext_id).is_none() {
            return false;
        }

        self.panels.push(Panel {
            id: String::from(panel_id),
            extension_id: String::from(ext_id),
            position,
            height,
            enabled: true,
        });

        self.events.push(ExtensionEvent::PanelCreated(0));
        true
    }

    /// Remove panel
    pub fn remove_panel(&mut self, panel_id: &str) -> bool {
        if let Some(pos) = self.panels.iter().position(|p| p.id == panel_id) {
            self.panels.remove(pos);
            self.events.push(ExtensionEvent::PanelRemoved(0));
            true
        } else {
            false
        }
    }

    /// Get all panels
    pub fn panels(&self) -> &[Panel] {
        &self.panels
    }

    /// Register theme
    pub fn register_theme(&mut self, theme: Theme) {
        self.themes.push(theme);
    }

    /// Get theme by ID
    pub fn get_theme(&self, theme_id: &str) -> Option<&Theme> {
        self.themes.iter().find(|t| t.id == theme_id)
    }

    /// Set current theme
    pub fn set_theme(&mut self, theme_id: &str) -> bool {
        if self.themes.iter().any(|t| t.id == theme_id) {
            self.current_theme = Some(String::from(theme_id));
            self.events.push(ExtensionEvent::ThemeChanged(0));
            true
        } else {
            false
        }
    }

    /// Get current theme
    pub fn current_theme(&self) -> Option<&Theme> {
        self.current_theme.as_ref().and_then(|id| self.get_theme(id))
    }

    /// Set wallpaper
    pub fn set_wallpaper(&mut self, wallpaper_id: &str) -> bool {
        // In real implementation, this would verify the wallpaper exists
        self.current_wallpaper = Some(String::from(wallpaper_id));
        self.events.push(ExtensionEvent::WallpaperChanged(0));
        true
    }

    /// Get current wallpaper
    pub fn current_wallpaper(&self) -> Option<&str> {
        self.current_wallpaper.as_ref().map(|s| s.as_str())
    }

    /// Register status indicator
    pub fn register_indicator(&mut self, indicator: StatusIndicator) {
        self.indicators.push(indicator);
    }

    /// Remove status indicator
    pub fn remove_indicator(&mut self, indicator_id: &str) -> bool {
        if let Some(pos) = self.indicators.iter().position(|i| i.id == indicator_id) {
            self.indicators.remove(pos);
            true
        } else {
            false
        }
    }

    /// Get all indicators
    pub fn indicators(&self) -> &[StatusIndicator] {
        &self.indicators
    }

    /// Register notification handler
    pub fn register_notification_handler(&mut self, handler: NotificationHandler) {
        self.notification_handlers.push(handler);
    }

    /// Remove notification handler
    pub fn remove_notification_handler(&mut self, handler_id: &str) -> bool {
        if let Some(pos) = self.notification_handlers.iter().position(|h| h.id == handler_id) {
            self.notification_handlers.remove(pos);
            true
        } else {
            false
        }
    }

    /// Get all notification handlers
    pub fn notification_handlers(&self) -> &[NotificationHandler] {
        &self.notification_handlers
    }

    /// Drain pending extension events
    pub fn drain_events(&mut self) -> Vec<ExtensionEvent> {
        let events = self.events.clone();
        self.events.clear();
        events
    }

    /// Get extensions by type
    pub fn extensions_by_type(&self, ext_type: ExtensionType) -> Vec<&Extension> {
        self.extensions
            .iter()
            .filter(|e| e.extension_type == ext_type && e.enabled)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extension_manager_creation() {
        let em = ExtensionManager::new();
        assert_eq!(em.widgets().len(), 0);
    }

    #[test]
    fn test_register_extension() {
        let mut em = ExtensionManager::new();
        let ext = Extension {
            id: String::from("test-widget"),
            name: String::from("Test Widget"),
            extension_type: ExtensionType::Widget,
            version: String::from("1.0.0"),
            author: String::from("Test"),
            enabled: true,
        };
        em.register_extension(ext);
        assert!(em.get_extension("test-widget").is_some());
    }

    #[test]
    fn test_create_widget() {
        let mut em = ExtensionManager::new();
        let ext = Extension {
            id: String::from("ext1"),
            name: String::from("Test"),
            extension_type: ExtensionType::Widget,
            version: String::from("1.0.0"),
            author: String::from("Test"),
            enabled: true,
        };
        em.register_extension(ext);
        assert!(em.create_widget("ext1", "widget1", WidgetPosition::CornerTopRight, 200, 200));
        assert_eq!(em.widgets().len(), 1);
    }

    #[test]
    fn test_themes() {
        let mut em = ExtensionManager::new();
        let theme = Theme {
            id: String::from("dark"),
            name: String::from("Dark"),
            author: String::from("Test"),
            primary_color: 0xFF1E1E1E,
            secondary_color: 0xFF2D2D2D,
            accent_color: 0xFF0078D4,
        };
        em.register_theme(theme);
        assert!(em.set_theme("dark"));
        assert!(em.current_theme().is_some());
    }
}
