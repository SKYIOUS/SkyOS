//! Render snapshot — read-only capture of desktop state for the frame renderer.

use crate::util::app_db::AppDb;
use crate::util::app_registry::AppRegistry;
use crate::apps::about::AboutState;
use crate::apps::settings::SettingsAppState;
use crate::apps::task_manager::TaskManagerState;
use crate::core::desktop_icons::DesktopIcon;
use crate::util::explorer::ExplorerState;
use crate::util::profiler::MetricsSnapshot;
use crate::service::clipboard::ClipboardManager;
use crate::service::notification::Notification;
use crate::core::settings::SettingsState;
use crate::core::start_menu::StartMenuState;
use crate::core::window::AppWindow;

pub struct RenderSnapshot<'a> {
    pub screen_w: u32,
    pub screen_h: u32,
    pub theme: &'a libsarga::theme::Theme,
    pub windows: &'a [AppWindow],
    pub icons: &'a [DesktopIcon],
    pub mouse: crate::core::geometry::Point,
    pub start_menu: bool,
    pub start_menu_state: Option<&'a StartMenuState>,
    pub app_db: Option<&'a AppDb>,
    pub context_menu: Option<(i32, i32, &'static [(&'static str, &'static str)])>,
    pub cursor_visible: bool,
    pub cursor_alpha: u8,
    pub fullscreen: bool,
    pub switcher_active: bool,
    pub switcher_idx: usize,
    pub rubber: Option<(i32, i32, i32, i32)>,
    pub notifications: &'a [Notification],
    pub tray: &'a [crate::core::tray::TrayEntry],
    pub clipboard: Option<&'a ClipboardManager>,
    pub settings: Option<&'a SettingsState>,
    pub app_reg: Option<&'a AppRegistry>,
    pub(crate) explorers: &'a [ExplorerState],
    pub settings_app: Option<&'a SettingsAppState>,
    pub task_manager: Option<&'a TaskManagerState>,
    pub about: Option<&'a AboutState>,
    pub focused_id: Option<u32>,
    pub focus_visible: bool,
    pub focused_bounds: Option<(i32, i32, u32, u32)>,
    pub tooltip: Option<&'a str>,
    pub tooltip_x: i32,
    pub tooltip_y: i32,
    pub debug_overlay: bool,
    pub debug_metrics: MetricsSnapshot,
    pub window_count: usize,
    pub notification_count: usize,
    pub snap_preview: Option<(i32, i32, u32, u32)>,
}

impl<'a> RenderSnapshot<'a> {
    pub fn taskbar_y(&self) -> u32 {
        self.screen_h - crate::core::constants::TASKBAR_H
    }
}
