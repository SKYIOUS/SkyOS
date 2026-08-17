pub(crate) mod about;
pub(crate) mod config_store;
pub(crate) mod settings;
pub(crate) mod task_manager;
pub(crate) mod tooltip;

use crate::apps::settings::SettingsPage;

/// A decoded overlay click. Each overlay app's `hit_test_action` maps a
/// pointer position to one of these, so the Desktop coordinator never
/// re-derives page indices, theme state, or close semantics from magic
/// numbers — the app that owns the geometry decides what a click means.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AppAction {
    /// Legacy settings panel: the Sound row was clicked.
    ToggleSound,
    /// A Dark Theme toggle was clicked; `dark` is the *new* desired state
    /// (the app computes it from its own flag before returning the action).
    SetTheme(bool),
    /// Settings app: a sidebar page was clicked.
    SelectPage(SettingsPage),
    /// Task manager: a process-list row was clicked.
    FocusWindow(usize),
    /// The overlay's close affordance (or click-anywhere for About).
    Close,
}
