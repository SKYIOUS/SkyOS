//! Render snapshot — read-only capture of desktop state for the frame renderer.

use crate::window::AppWindow;
use crate::app_db::AppDb;
use crate::app_registry::AppRegistry;
use crate::start_menu::StartMenuState;
use crate::desktop_icons::DesktopIcon;
use crate::notification::Notification;
use crate::clipboard_service::ClipboardService;
use crate::settings::SettingsState;

pub struct RenderSnapshot<'a> {
    pub screen_w: u32,
    pub screen_h: u32,
    pub theme: &'a libsarga::theme::Theme,
    pub windows: &'a [AppWindow],
    pub icons: &'a [DesktopIcon],
    pub mouse: crate::geometry::Point,
    pub start_menu: bool,
    pub start_menu_state: Option<&'a StartMenuState>,
    pub app_db: Option<&'a AppDb>,
    pub context_menu: Option<(i32, i32, &'static [(&'static str, &'static str)])>,
    pub cursor_visible: bool,
    pub fullscreen: bool,
    pub switcher_active: bool,
    pub switcher_idx: usize,
    pub rubber: Option<(i32, i32, i32, i32)>,
    pub notifications: &'a [Notification],
    pub tray: &'a [crate::tray::TrayEntry],
    pub clipboard: Option<&'a ClipboardService>,
    pub settings: Option<&'a SettingsState>,
    #[allow(dead_code)]
    pub app_reg: Option<&'a AppRegistry>,
}

impl<'a> RenderSnapshot<'a> {
    pub fn taskbar_y(&self) -> u32 {
        self.screen_h - crate::constants::TASKBAR_H
    }
}
