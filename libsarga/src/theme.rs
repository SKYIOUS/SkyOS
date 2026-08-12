/// Chrome colors with no `Theme` field equivalent, consumed directly by draw
/// code. Palette values live inline in the `Theme` constructors (`dark()`/
/// `light()`) — the CI `dead-code` job rejects any constant in this module
/// with zero references outside theme.rs, so a color can never sit here
/// unused while a hardcoded duplicate value lives elsewhere (the
/// WIN_CLOSE_HOVER history: it sat unreferenced for months, then a
/// hardcoded copy appeared in window.rs before the constant was wired in).
pub mod colors {
    // Window close button — red in both themes, no theme-field equivalent.
    pub const WIN_CLOSE_HOVER: u32 = 0xFFE81123; // Close button hover
    pub const WIN_CLOSE_PRESSED: u32 = 0xFFB71C1C; // Close button held down
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
    /// Text drawn on an accent/hover (indigo) fill. The accent and hover
    /// colors are theme-invariant (same indigo in both themes), so any text
    /// on them must stay white in both themes too — `theme.text` flips to
    /// black in the light theme and would vanish on the indigo surfaces
    /// (start button, selected menu rows, hovered buttons, notification
    /// rows). 5.13:1 on accent in both themes (WCAG AA).
    pub on_accent: u32,
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
    pub fn light() -> Self {
        Theme {
            bg_primary: 0xFFF5F5F5,
            bg_surface: 0xFFFFFFFF,
            bg_elevated: 0xFFEEEEEE,
            accent: 0xFF3D5AFE,
            accent_light: 0xFF1A8FE8,
            accent_dark: 0xFF005A9E,
            text: 0xFF000000,
            text_secondary: 0xFF555555,
            // 0xFF757575 (gray 600) instead of 0xFFAAAAAA: the old value is
            // 2.3:1 on white — a hint/placeholder (start-menu search,
            // "Recent:" label, "Coming soon") must be readable on the
            // white light surfaces. 4.6:1 on white; still clearly
            // "disabled" vs text_secondary.
            text_disabled: 0xFF757575,
            // Darker success green (green-700 family instead of 500): the
            // old 0x4CAF50 is only 2.4:1 on the light elevated surface the
            // toggles sit on. 0xFF2B6A30 is 5.6:1 there.
            success: 0xFF2B6A30,
            on_accent: 0xFFFFFFFF, // Text on accent/hover fills (5.13:1 in both themes)
            border: 0xFFCCCCCC,
            hover: 0xFF3D5AFE,
            pressed: 0xFFE0E0E0,
            error: 0xFFD32F2F,
            warning: 0xFFFFC107,
            separator: 0xFFCCCCCC,
            shadow: 0x40000000,
            font_size: 14,
            border_radius: 12,
            padding: 10,
            spacing: 6,
        }
    }

    pub fn dark() -> Self {
        Theme {
            bg_primary: 0xFF0F0F1A,     // Darker navy background
            bg_surface: 0xFF1A1A2E,     // Card/surface
            bg_elevated: 0xFF252540,    // Elevated surface
            accent: 0xFF3D5AFE,         // Indigo accent
            accent_light: 0xFF1A8FE8,   // Lighter accent
            accent_dark: 0xFF005A9E,    // Darker accent
            text: 0xFFFFFFFF,           // White text
            text_secondary: 0xFFB0B0B0, // Gray text
            text_disabled: 0xFF606060,  // Disabled text
            on_accent: 0xFFFFFFFF,      // Text on accent/hover fills (5.13:1 in both themes)
            border: 0xFF30304D,         // Border color
            hover: 0xFF3D5AFE,          // Hover state (indigo, same as accent)
            pressed: 0xFF1A1A30,        // Pressed state
            error: 0xFFD32F2F,          // Error red
            success: 0xFF4CAF50,        // Success green
            warning: 0xFFFFC107,        // Warning yellow
            separator: 0xFF3A3A5C,      // Separator line
            shadow: 0x80000000,         // Semi-transparent black
            font_size: 14,
            border_radius: 12, // More rounded corners
            padding: 10,
            spacing: 6,
        }
    }
}
