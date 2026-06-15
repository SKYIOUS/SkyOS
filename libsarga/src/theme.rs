/// Color constants for the dark theme
pub mod colors {
    pub const BG_PRIMARY: u32 = 0xFF1B1B2F;     // Deep navy background
    pub const BG_SURFACE: u32 = 0xFF252540;     // Card/surface
    pub const BG_ELEVATED: u32 = 0xFF2D2D4A;    // Elevated surface
    pub const ACCENT: u32 = 0xFF0078D4;         // Windows blue accent
    pub const ACCENT_LIGHT: u32 = 0xFF1A8FE8;   // Lighter accent
    pub const ACCENT_DARK: u32 = 0xFF005A9E;    // Darker accent
    pub const TEXT_PRIMARY: u32 = 0xFFFFFFFF;    // White text
    pub const TEXT_SECONDARY: u32 = 0xFFB0B0B0;  // Gray text
    pub const TEXT_DISABLED: u32 = 0xFF606060;   // Disabled text
    pub const BORDER: u32 = 0xFF3A3A5C;         // Border color
    pub const HOVER: u32 = 0xFF3A3A5C;          // Hover state
    pub const PRESSED: u32 = 0xFF1A1A30;        // Pressed state
    pub const ERROR: u32 = 0xFFD32F2F;          // Error red
    pub const SUCCESS: u32 = 0xFF4CAF50;        // Success green
    pub const WARNING: u32 = 0xFFFFC107;        // Warning yellow
    pub const SEPARATOR: u32 = 0xFF3A3A5C;      // Separator line
    pub const SHADOW: u32 = 0x80000000;         // Semi-transparent black

    // Taskbar
    pub const TASKBAR: u32 = 0xFF1A1A2E;        // Taskbar background
    pub const TASKBAR_HOVER: u32 = 0xFF2D2D4A;  // Taskbar hover

    // Start menu
    pub const MENU_BG: u32 = 0xFF252540;        // Menu background
    pub const MENU_HOVER: u32 = 0xFF3A3A5C;     // Menu hover
    pub const MENU_TEXT: u32 = 0xFFFFFFFF;       // Menu text

    // Window
    pub const WIN_TITLE: u32 = 0xFF1B1B2F;      // Title bar
    pub const WIN_BG: u32 = 0xFF1E1E32;         // Window background
    pub const WIN_BORDER: u32 = 0xFF3A3A5C;     // Window border
    pub const WIN_CLOSE_HOVER: u32 = 0xFFE81123; // Close button hover
}

pub struct Theme {
    pub bg_primary: u32,
    pub bg_surface: u32,
    pub bg_elevated: u32,
    pub accent: u32,
    pub accent_light: u32,
    pub accent_dark: u32,
    pub text: u32,
    pub text_secondary: u32,
    pub text_disabled: u32,
    pub border: u32,
    pub hover: u32,
    pub pressed: u32,
    pub error: u32,
    pub success: u32,
    pub warning: u32,
    pub separator: u32,
    pub shadow: u32,
    pub font_size: u32,
    pub border_radius: u32,
    pub padding: u32,
    pub spacing: u32,
}

impl Theme {
    pub fn dark() -> Self {
        Theme {
            bg_primary: colors::BG_PRIMARY,
            bg_surface: colors::BG_SURFACE,
            bg_elevated: colors::BG_ELEVATED,
            accent: colors::ACCENT,
            accent_light: colors::ACCENT_LIGHT,
            accent_dark: colors::ACCENT_DARK,
            text: colors::TEXT_PRIMARY,
            text_secondary: colors::TEXT_SECONDARY,
            text_disabled: colors::TEXT_DISABLED,
            border: colors::BORDER,
            hover: colors::HOVER,
            pressed: colors::PRESSED,
            error: colors::ERROR,
            success: colors::SUCCESS,
            warning: colors::WARNING,
            separator: colors::SEPARATOR,
            shadow: colors::SHADOW,
            font_size: 14,
            border_radius: 6,
            padding: 8,
            spacing: 4,
        }
    }
}
