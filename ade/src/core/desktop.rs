//! Desktop coordinator — event dispatch, window management, layout logic.

use crate::util::app_registry::AppId;
use crate::core::constants::*;
use libsarga::io;
use crate::core::damage::DamageTracker;
use crate::core::desktop_icons::DesktopIcons;
use crate::core::event::Event;
use crate::core::geometry::{Point, Rect};
use crate::util::profiler::Profiler;
use crate::util::log::Logger;
use crate::util::crash_diagnostics::CrashDiagnostics;
use crate::core::start_menu::StartMenuState;
use crate::core::tray::SystemTray;
use crate::core::window::{VisualFlags, WindowId, WindowState};
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;
use libsarga::process;

pub(crate) enum Cursor {
    Default,
    Arrow,
    ResizeH,
    ResizeV,
    ResizeDiagonal,
    Move,
    Busy,
    Text,
    Hand,
}

const DESKTOP_MENU: &[(&str, &str)] = &[
    ("Refresh", "refresh"),
    ("---", ""),
    ("Settings", "settings"),
    ("Terminal", "terminal"),
    ("---", ""),
    ("Paste", "paste"),
    ("Create Folder", "new_folder"),
    ("Create File", "new_file"),
    ("---", ""),
    ("Properties", "properties"),
];

const ICON_MENU: &[(&str, &str)] = &[
    ("Open", "open"),
    ("Delete", "delete"),
    ("Rename", "rename"),
    ("---", ""),
    ("Properties", "properties"),
];

// Window system menu for right-click on titlebar
const SYSTEM_MENU: &[(&str, &str)] = &[
    ("Restore", "restore"),
    ("Move", "move"),
    ("Size", "size"),
    ("Minimize", "minimize"),
    ("Maximize", "maximize"),
    ("---", ""),
    ("Close", "close"),
];
use crate::core::window_manager::{SnapRegion, WindowManager};

pub(crate) enum TilingMode {
    Floating,
    Tile,
    Monocle,
}
use crate::render::snapshot::RenderSnapshot;
use crate::core::shortcut::{ShortcutAction, ShortcutManager};

pub struct Desktop {
    pub(crate) screen_w: u32,
    pub(crate) screen_h: u32,
    pub(crate) wm: WindowManager,
    pub(crate) start_menu: StartMenuState,
    pub(crate) context_menu: Option<(i32, i32, &'static [(&'static str, &'static str)])>,
    pub(crate) clock_ticks: u64,
    pub(crate) mouse_x: i32,
    pub(crate) mouse_y: i32,
    mouse_btn: bool,
    prev_mouse_btn: bool,
    pub(crate) drag_active: bool,
    pub(crate) cursor: Cursor,
    pub(crate) cursor_visible: bool,
    cursor_alpha: u8,
    cursor_blink_tick: u8,
    resize_win: Option<WindowId>,
    resize_edges: u8,
    resize_rect: (i32, i32, u32, u32),
    last_click_time: u64,
    last_click_pos: Point,
    pub(crate) double_click: bool,
    pub(crate) desktop_icons: DesktopIcons,
    pub(crate) theme_svc: crate::core::theme_service::ThemeService,
    pub(crate) damage: DamageTracker,
    pub(crate) clock_cache: crate::render::clock::ClockCache,
    shortcuts: ShortcutManager,
    tiling_mode: TilingMode,
    prev_tiling_geos: alloc::vec::Vec<(i32, i32, u32, u32)>,
    focus_history: VecDeque<u64>,
    switcher_active: bool,
    switcher_idx: usize,
    pub(crate) app_reg: crate::util::app_registry::AppRegistry,
    pub(crate) lifecycle: crate::sys::lifecycle::LifecycleManager,
    pub(crate) services: crate::service::service_manager::ServiceManager,
    pub(crate) tray: SystemTray,
    pub(crate) settings: crate::core::settings::SettingsState,
    pub(crate) config_store: crate::apps::config_store::ConfigStore,
    pub(crate) terminal_state: crate::apps::terminal::TerminalState,
    pub(crate) file_manager: crate::apps::files::FileManagerState,
    pub(crate) task_manager: crate::apps::task_manager::TaskManagerState,
    pub(crate) about_state: crate::apps::about::AboutState,
    pub(crate) settings_app: crate::apps::settings::SettingsAppState,
    #[allow(dead_code)]
    pub(crate) file_assoc: crate::util::file_assoc::FileAssociationEngine,
    #[allow(dead_code)]
    pub(crate) vfs: crate::sys::vfs::VfsContext,
    pub(crate) watcher: crate::sys::watcher::FileWatcher,
    pub(crate) explorers: alloc::vec::Vec<crate::util::explorer::ExplorerState>,
    #[allow(dead_code)]
    pub(crate) recovery: crate::util::recovery::RecoverySystem,
    pub(crate) a11y_tree: crate::sec::a11y::A11yTree,
    pub(crate) focus: crate::sec::a11y::FocusManager,
    pub(crate) tooltips: crate::apps::tooltip::TooltipManager,
    pub(crate) focus_visible: bool,
    tooltip_hover_ticks: u32,
    tooltip_last_hover: Option<u32>,
    system_menu_for: Option<WindowId>,
    pub(crate) ipc_server: crate::ipc::IpcServer,
    pub(crate) ipc_transport: crate::ipc::transport::IpcTransport,
    pub(crate) service_registry: crate::ipc::ServiceRegistry,
    pub(crate) crash_manager: crate::util::crash_manager::CrashManager,
    pub(crate) desktop_entries: alloc::vec::Vec<crate::util::desktop_entry::DesktopEntry>,
    pub(crate) permissions: crate::sec::perms::PermissionManager,
    pub(crate) profiler: Profiler,
    pub(crate) logger: Logger,
    pub(crate) crash_diag: CrashDiagnostics,
    pub(crate) debug_overlay: bool,
}

impl Desktop {
    pub fn new(w: u32, h: u32) -> Self {
        Self {
            screen_w: w,
            screen_h: h,
            wm: WindowManager::new(),
            start_menu: StartMenuState::new(),
            context_menu: None,
            clock_ticks: 0,
            mouse_x: (w / 2) as i32,
            mouse_y: (h / 2) as i32,
            mouse_btn: false,
            prev_mouse_btn: false,
            drag_active: false,
            cursor: Cursor::Default,
            cursor_visible: true,
            cursor_alpha: 255,
            cursor_blink_tick: 0,
            resize_win: None,
            resize_edges: 0,
            resize_rect: (0, 0, 0, 0),
            last_click_time: 0,
            last_click_pos: Point::new(0, 0),
            double_click: false,
            desktop_icons: DesktopIcons::new(),
            theme_svc: crate::core::theme_service::ThemeService::new(),
            damage: DamageTracker::new(),
            clock_cache: crate::render::clock::ClockCache::new(),
            shortcuts: ShortcutManager::new(),
            tiling_mode: TilingMode::Floating,
            prev_tiling_geos: alloc::vec::Vec::new(),
            focus_history: VecDeque::new(),
            switcher_active: false,
            switcher_idx: 0,
            app_reg: crate::util::app_registry::AppRegistry::new(),
            lifecycle: crate::sys::lifecycle::LifecycleManager::new(),
            services: crate::service::service_manager::ServiceManager::new(0),
            tray: SystemTray::new(),
            settings: crate::core::settings::SettingsState::new(),
            config_store: crate::apps::config_store::ConfigStore::new(),
            terminal_state: crate::apps::terminal::TerminalState::new(),
            file_manager: crate::apps::files::FileManagerState::new(),
            task_manager: crate::apps::task_manager::TaskManagerState::new(),
            about_state: crate::apps::about::AboutState::new(),
            settings_app: crate::apps::settings::SettingsAppState::new(),
            file_assoc: crate::util::file_assoc::FileAssociationEngine::new(),
            vfs: crate::sys::vfs::VfsContext::new(),
            watcher: crate::sys::watcher::FileWatcher::new(),
            explorers: alloc::vec::Vec::new(),
            recovery: crate::util::recovery::RecoverySystem::new(),
            a11y_tree: crate::sec::a11y::A11yTree::new(),
            focus: crate::sec::a11y::FocusManager::new(),
            tooltips: crate::apps::tooltip::TooltipManager::new(),
            focus_visible: false,
            tooltip_hover_ticks: 0,
            tooltip_last_hover: None,
            system_menu_for: None,
            ipc_server: crate::ipc::IpcServer::new(),
            ipc_transport: crate::ipc::transport::IpcTransport::new(),
            service_registry: {
                let mut reg = crate::ipc::ServiceRegistry::new();
                reg.register_defaults();
                reg
            },
            crash_manager: crate::util::crash_manager::CrashManager::new(),
            desktop_entries: alloc::vec::Vec::new(),
            permissions: crate::sec::perms::PermissionManager::new(),
            profiler: Profiler::new(),
            logger: Logger::new(),
            crash_diag: CrashDiagnostics::new(),
            debug_overlay: false,
        }
    }
    pub fn taskbar_y(&self) -> u32 {
        self.screen_h - TASKBAR_H
    }

    pub fn advance_clock(&mut self) {
        self.clock_ticks += 1;
    }

    pub fn reap_children(&mut self) {
        loop {
            match process::waitpid(-1, 1) {
                Ok((pid, status)) if pid > 0 => {
                    use crate::sys::lifecycle::ExitClass;
                    match crate::sys::lifecycle::exit_class(status) {
                        ExitClass::Clean => self.lifecycle.mark_terminated(pid),
                        cls => {
                            self.lifecycle.mark_crashed(pid);
                            let reason = match cls {
                                ExitClass::Killed => alloc::string::String::from("killed"),
                                ExitClass::Signal(sig) => alloc::format!("signal {}", sig),
                                ExitClass::Error(code) => alloc::format!("exit {}", code),
                                ExitClass::Clean => unreachable!(),
                            };
                            self.services
                                .notify("Application Crashed", &reason, 2, 8000);
                        }
                    }
                    self.lifecycle.remove(pid);
                    self.permissions.unregister(pid);
                    self.ipc_transport.unregister(pid);
                    self.wm.close_by_pid(pid);
                    self.damage.mark_full();
                }
                _ => break,
            }
        }
    }

    pub fn tick(&mut self) {
        self.profiler.frame_timer.start(self.clock_ticks);
        self.advance_clock();
        // Breathing cursor: smooth alpha blink over 30 ticks
        self.cursor_blink_tick = (self.cursor_blink_tick + 1) % 30;
        if self.cursor_blink_tick < 15 {
            self.cursor_alpha = 255 - (self.cursor_blink_tick * 17);
        } else {
            self.cursor_alpha = (self.cursor_blink_tick - 15) * 17;
        }
        if self.cursor_alpha == 0 {
            self.cursor_visible = false;
            self.damage.mark_full();
        } else if !self.cursor_visible {
            self.cursor_visible = true;
            self.damage.mark_full();
        }
        self.reap_children();
        let reqs = self.ipc_transport.ingest();
        for req in reqs {
            self.ipc_server.submit_request(req);
        }
        self.process_ipc();
        let responses = self.ipc_server.drain_responses();
        self.ipc_transport.deliver(responses);
        let mut anim_active = false;
        for w in self.wm.iter_mut() {
            if w.flags.opacity < 255 {
                w.flags.opacity = w.flags.opacity.saturating_add(25);
            }
            if w.flags.opacity >= 255 {
                w.anim_opacity = 255;
            }
            if w.tick_animation() {
                anim_active = true;
            }
        }
        if anim_active {
            self.damage.mark_full();
        }
        // Process closing windows (remove after shrink animation)
        if !self.wm.process_closing().is_empty() {
            self.damage.mark_full();
        }
        self.services.tick(self.clock_ticks);
        self.watcher.poll();
        self.build_a11y_tree();
        self.tooltips.tick();
        self.tick_tooltip_hover();
        self.profiler.frame_timer.stop(self.clock_ticks);
        if self.clock_ticks % 1000 == 0 {
            self.logger.info(self.clock_ticks, "tick");
        }
    }

    fn tick_tooltip_hover(&mut self) {
        let hovered = self.a11y_tree.node_at(self.mouse_x, self.mouse_y);
        let hover_id = hovered.map(|n| n.id);
        if hover_id != self.tooltip_last_hover {
            self.tooltip_hover_ticks = 0;
            self.tooltip_last_hover = hover_id;
            self.tooltips.hide();
            return;
        }
        if self.tooltips.active.is_some() {
            return;
        }
        if let Some(id) = hover_id {
            self.tooltip_hover_ticks = self.tooltip_hover_ticks.saturating_add(1);
            if self.tooltip_hover_ticks >= 30 {
                if let Some(n) = self.a11y_tree.nodes.iter().find(|n| n.id == id) {
                    let label = if n.label.is_empty() {
                        match n.role {
                            crate::sec::a11y::A11yRole::Taskbar => "Taskbar",
                            crate::sec::a11y::A11yRole::StartMenu => "Start Menu",
                            crate::sec::a11y::A11yRole::Desktop => "Desktop",
                            _ => "",
                        }
                    } else {
                        &n.label
                    };
                    if !label.is_empty() {
                        let tx = self.mouse_x + 12;
                        let ty = self.mouse_y;
                        self.tooltips.show(label, tx, ty, 120);
                    }
                }
            }
        }
    }

    fn build_a11y_tree(&mut self) {
        self.a11y_tree.clear();
        let ty = self.taskbar_y();

        // root: Desktop
        let desktop_id = self.a11y_tree.add_node(
            crate::sec::a11y::A11yRole::Desktop,
            "Desktop",
            (0, 0, self.screen_w, ty),
            false,
        );

        // Taskbar
        let taskbar_id = self.a11y_tree.add_node(
            crate::sec::a11y::A11yRole::Taskbar,
            "Taskbar",
            (0, ty as i32, self.screen_w, crate::core::constants::TASKBAR_H),
            true,
        );
        self.a11y_tree.add_child(desktop_id, taskbar_id);

        // Start button
        let sb_id = self.a11y_tree.add_node(
            crate::sec::a11y::A11yRole::Button,
            "Start",
            (5, ty as i32 + 4, 58, crate::core::constants::TASKBAR_H - 8),
            true,
        );
        self.a11y_tree.add_child(taskbar_id, sb_id);

        // Window buttons in taskbar
        for i in 0..self.wm.len() {
            let aw = &self.wm.iter()[i];
            let bx = 75 + i as u32 * 125;
            let btn_id = self.a11y_tree.add_node(
                crate::sec::a11y::A11yRole::Button,
                &aw.title,
                (bx as i32, ty as i32 + 4, 120, crate::core::constants::TASKBAR_H - 8),
                true,
            );
            self.a11y_tree.add_child(taskbar_id, btn_id);
        }

        // Start Menu
        if self.start_menu.open {
            let menu_x = 4i32;
            let menu_y = ty as i32 - 5 - 460;
            let start_menu_id = self.a11y_tree.add_node(
                crate::sec::a11y::A11yRole::StartMenu,
                "Start Menu",
                (menu_x, menu_y, 480, 460),
                true,
            );
            self.a11y_tree.add_child(desktop_id, start_menu_id);
        }

        // Windows
        for (i, aw) in self.wm.iter().iter().enumerate() {
            let win_id = self.a11y_tree.add_node(
                crate::sec::a11y::A11yRole::Window,
                &aw.title,
                (aw.x, aw.y, aw.w, aw.h),
                true,
            );
            self.a11y_tree.add_child(desktop_id, win_id);

            // close button
            let close_id = self.a11y_tree.add_node(
                crate::sec::a11y::A11yRole::Button,
                "Close",
                (aw.x + aw.w as i32 - 28, aw.y + 6, 22, 18),
                true,
            );
            self.a11y_tree.add_child(win_id, close_id);
        }

        // Desktop Icons
        for ic in &self.desktop_icons.icons {
            let icon_id = self.a11y_tree.add_node(
                crate::sec::a11y::A11yRole::Icon,
                &ic.name,
                (ic.x, ic.y, 48, 56),
                true,
            );
            self.a11y_tree.add_child(desktop_id, icon_id);
        }

        // Notifications
        for n in self.services.notifications.visible_notifications() {
            let notif_id = self.a11y_tree.add_node(
                crate::sec::a11y::A11yRole::Notification,
                &n.title,
                (0, 0, 0, 0),
                false,
            );
            self.a11y_tree.add_child(desktop_id, notif_id);
        }

        // Sync focus from FocusManager
        if let Some(fid) = self.focus.focused() {
            self.a11y_tree.set_focus(fid);
        }
    }

    fn save_geometries(&mut self) {
        self.prev_tiling_geos.clear();
        for w in self.wm.iter() {
            self.prev_tiling_geos.push((w.x, w.y, w.w, w.h));
        }
    }

    fn restore_geometries(&mut self) {
        for (i, &(x, y, w, h)) in self.prev_tiling_geos.iter().enumerate() {
            if let Some(wid) = self.wm.id_at(i) {
                if let Some(aw) = self.wm.lookup_mut(wid) {
                    aw.x = x;
                    aw.y = y;
                    aw.w = w;
                    aw.h = h;
                }
            }
        }
        self.prev_tiling_geos.clear();
    }

    fn apply_tile(&mut self) {
        self.save_geometries();
        let sw = self.screen_w;
        let th = self.taskbar_y();
        let n = self.wm.len();
        if n == 0 {
            return;
        }
        if n == 1 {
            if let Some(wid) = self.wm.id_at(0) {
                if let Some(w) = self.wm.lookup_mut(wid) {
                    w.x = 0;
                    w.y = 0;
                    w.w = sw;
                    w.h = th;
                }
            }
            return;
        }
        let master_w = sw * 6 / 10;
        let stack_w = sw - master_w;
        let stack_h = th / (n as u32 - 1);
        for i in 0..n {
            if let Some(wid) = self.wm.id_at(i) {
                if let Some(w) = self.wm.lookup_mut(wid) {
                    if i == 0 {
                        w.x = 0;
                        w.y = 0;
                        w.w = master_w;
                        w.h = th;
                    } else {
                        w.x = master_w as i32;
                        w.y = ((i as u32 - 1) * stack_h) as i32;
                        w.w = stack_w;
                        w.h = stack_h;
                    }
                }
            }
        }
        self.tiling_mode = TilingMode::Tile;
        self.damage.mark_full();
    }

    fn apply_monocle(&mut self) {
        self.save_geometries();
        let sw = self.screen_w;
        let th = self.taskbar_y();
        for i in 0..self.wm.len() {
            if let Some(wid) = self.wm.id_at(i) {
                if let Some(w) = self.wm.lookup_mut(wid) {
                    w.x = 0;
                    w.y = 0;
                    w.w = sw;
                    w.h = th;
                }
            }
        }
        self.tiling_mode = TilingMode::Monocle;
        self.damage.mark_full();
    }

    fn set_floating(&mut self) {
        self.restore_geometries();
        self.tiling_mode = TilingMode::Floating;
        self.damage.mark_full();
    }

    fn record_current_focus(&mut self) {
        if let Some(id) = self.wm.active() {
            self.focus_history.retain(|&i| i != id.0);
            self.focus_history.push_back(id.0);
            if self.focus_history.len() > 20 {
                self.focus_history.pop_front();
            }
        }
    }

    fn cycle_window(&mut self) {
        if self.wm.len() < 2 {
            return;
        }
        if !self.switcher_active {
            let current = self
                .wm
                .active()
                .and_then(|id| self.wm.position_of(id))
                .unwrap_or(0);
            self.switcher_idx = current;
            self.switcher_active = true;
        } else {
            self.switcher_idx = (self.switcher_idx + 1) % self.wm.len();
        }
        self.damage.mark_full();
    }

    fn cycle_tiling(&mut self) {
        match self.tiling_mode {
            TilingMode::Floating => self.apply_tile(),
            TilingMode::Tile => self.apply_monocle(),
            TilingMode::Monocle => self.set_floating(),
        }
    }

    pub fn handle_event(&mut self, ev: Event) {
        match ev {
            Event::Key(key) => {
                if !self.handle_a11y_key(key) {
                    self.handle_key(key);
                }
            }
            Event::MouseClick(x, y) => {
                self.focus_visible = false;
                self.handle_click(x, y);
            }
            Event::MouseMiddle(x, y) => {
                self.focus_visible = false;
                self.handle_middle_click(x, y);
            }
            Event::MouseRight(x, y) => {
                self.focus_visible = false;
                self.handle_right_click(x, y);
            }
            Event::MouseDrag(x, y) => self.handle_drag(x, y),
            Event::Scroll(delta) => self.handle_scroll(delta),
            Event::MouseRelease => self.release_drag(),
            Event::NotificationAdded(_id) => {}
            Event::NotificationRemoved(_id) => {}
            Event::ClipboardChanged => {}
            Event::SessionChanged => {}
            // Session lifecycle: PowerRequest → clean session end
            Event::PowerRequest(_req) => {
                io::print_str("[ade] session ending via PowerRequest\n");
                process::exit(0);
            }
            Event::AppStarted(_id) => {}
            Event::AppClosed(_id) => {}
            Event::AppFocused(_id) => {}
            Event::AppCrashed(_id) => {
                self.crash_diag.record_event("app_crashed");
                self.services.notify("App Crashed", "An application has crashed", 2, 120);
            }
            Event::SettingsChanged => {}
            Event::FocusChanged(_id) => {}
            Event::ElementActivated(_id) => {}
            Event::ThemeChanged => {}
            Event::TooltipOpened => {}
            Event::TooltipClosed => {}
            Event::AppInstalled(_id) => {}
            Event::AppRemoved(_id) => {}
            Event::PermissionGranted(_id) => {}
            Event::PermissionDenied(_id) => {}
            Event::IPCConnected(_id) => {}
            Event::IPCDisconnected(_id) => {}
            Event::ServiceRegistered(_name) => {}
            Event::ServiceUnavailable(_name) => {}
            Event::FocusNext => {
                self.wm.focus_next();
                self.focus_visible = true;
                self.damage.mark_full();
            }
            Event::FocusPrev => {
                self.wm.focus_prev();
                self.focus_visible = true;
                self.damage.mark_full();
            }
        }
    }

    fn handle_a11y_key(&mut self, key: u8) -> bool {
        match key {
            72 | 80 => {
                // Up / Down arrows
                self.focus_visible = true;
                let dir = if key == 72 {
                    crate::sec::a11y::FocusDirection::Up
                } else {
                    crate::sec::a11y::FocusDirection::Down
                };
                self.focus.move_focus(dir, &self.a11y_tree);
                self.damage.mark_full();
                true
            }
            75 | 77 => {
                // Left / Right arrows
                self.focus_visible = true;
                let dir = if key == 75 {
                    crate::sec::a11y::FocusDirection::Left
                } else {
                    crate::sec::a11y::FocusDirection::Right
                };
                self.focus.move_focus(dir, &self.a11y_tree);
                self.damage.mark_full();
                true
            }
            13 | 28 => {
                // Enter (ASCII or scan code)
                self.focus_visible = true;
                if let Some(fid) = self.focus.focused() {
                    self.activate_a11y_node(fid);
                }
                self.damage.mark_full();
                true
            }
            27 | 1 => {
                // Escape (ASCII or scan code)
                self.focus_visible = false;
                self.focus.blur();
                if self.start_menu.open {
                    self.start_menu.open = false;
                }
                self.context_menu = None;
                self.damage.mark_full();
                true
            }
            _ => false,
        }
    }

    fn handle_key_focus(&mut self, key: u8) {
        match key {
            0x09 => {
                self.wm.focus_next();
                self.focus_visible = true;
                self.damage.mark_full();
            }
            0x1B => {
                self.focus_visible = false;
                self.damage.mark_full();
            }
            _ => {}
        }
    }

    fn handle_keyboard_nav(&mut self, key: u8) {
        match key {
            0x09 | 0x1B => self.handle_key_focus(key),
            _ => {}
        }
    }

    fn activate_a11y_node(&mut self, id: u32) {
        let node = match self.a11y_tree.nodes.iter().find(|n| n.id == id) {
            Some(n) => n.clone(),
            None => return,
        };
        match node.role {
            crate::sec::a11y::A11yRole::Window => {
                // bring window to front
                let win_idx = node.label.parse::<usize>().unwrap_or(usize::MAX);
                if let Some(wid) = self.wm.id_at(win_idx) {
                    self.wm.bring_to_front(wid);
                }
            }
            crate::sec::a11y::A11yRole::Icon => {
                // find and open the icon
                let name = node.label.clone();
                for ic in &self.desktop_icons.icons {
                    if ic.name == name {
                        self.exec_context_action("open");
                        break;
                    }
                }
            }
            crate::sec::a11y::A11yRole::Taskbar => {
                // click start button
                if self.mouse_y as u32 >= self.taskbar_y() {
                    self.start_menu.open_with(&self.app_reg);
                }
            }
            _ => {}
        }
    }

    fn exec_context_action(&mut self, action: &str) {
        match action {
            "terminal" => self.spawn_app("/bin/sash", "Terminal"),
            "arrange" => {
                let positions: &[(i32, i32)] =
                    &[(30, 80), (30, 180), (30, 280), (30, 380), (30, 480)];
                for (i, ic) in self.desktop_icons.icons.iter_mut().enumerate() {
                    if i < positions.len() {
                        ic.x = positions[i].0;
                        ic.y = positions[i].1;
                    }
                }
            }
            "paste" => {}
            "wallpaper" => {}
            "refresh" => {
                self.damage.mark_full();
            }
            "new_folder" => {}
            "new_file" => {}
            "properties" => {}
            "settings" => {
                self.settings.toggle();
                self.context_menu = None;
            }
            "open" => {
                let mut to_launch: Vec<(String, String)> = Vec::new();
                for ic in &self.desktop_icons.icons {
                    if ic.selected {
                        let path = match ic.name.as_str() {
                            "Terminal" => "/bin/sash",
                            "Files" | "File Browser" => "/bin/skyfiles",
                            "SkyStore" => "/bin/skystore",
                            "SkyEdit" => "/bin/skyedit",
                            "Calc" | "Calculator" => "/bin/calculator",
                            _ => "",
                        };
                        if !path.is_empty() {
                            to_launch.push((ic.name.clone(), String::from(path)));
                        }
                    }
                }
                for (name, path) in to_launch {
                    self.spawn_app(&path, &name);
                }
            }
            "delete" => {
                self.desktop_icons.icons.retain(|ic| !ic.selected);
            }
            // Window system menu actions
            "restore" => {
                if let Some(wi) = self.system_menu_for {
                    self.wm.restore(wi);
                }
                self.system_menu_for = None;
            }
            "move" => {
                if let Some(wi) = self.system_menu_for {
                    if self.wm.lookup(wi).is_some() {
                        self.wm.begin_drag(wi, self.mouse_x, self.mouse_y);
                    }
                }
                self.system_menu_for = None;
            }
            "size" => {
                if let Some(wi) = self.system_menu_for {
                    if let Some(w) = self.wm.lookup(wi) {
                        self.resize_win = Some(wi);
                        self.resize_edges = 4;
                        self.resize_rect = (w.x, w.y, w.w, w.h);
                    }
                }
                self.system_menu_for = None;
            }
            "minimize" => {
                if let Some(wi) = self.system_menu_for {
                    self.wm.minimize(wi, self.screen_w, self.taskbar_y());
                }
                self.system_menu_for = None;
            }
            "maximize" => {
                if let Some(wi) = self.system_menu_for {
                    self.wm.toggle_maximize(wi, self.screen_w, self.taskbar_y());
                }
                self.system_menu_for = None;
            }
            "close" => {
                if let Some(wi) = self.system_menu_for {
                    self.wm.close(wi);
                }
                self.system_menu_for = None;
            }
            _ => {}
        }
    }

    pub fn close_focused_window(&mut self) {
        if let Some(active) = self.wm.active() {
            self.wm.close(active);
            self.damage.mark_full();
        }
    }

    fn launch_app(&mut self, app_id: AppId) {
        self.start_menu.open = false;
        let app = match self.app_reg.get(app_id) {
            Some(a) => *a,
            None => {
                self.damage.mark_full();
                return;
            }
        };
        match app.startup_mode {
            crate::util::app_registry::StartupMode::Singleton => {
                if app.name == "Settings" {
                    self.settings_app.open = true;
                }
                self.damage.mark_full();
                return;
            }
            crate::util::app_registry::StartupMode::Background => {
                self.damage.mark_full();
                return;
            }
            _ => {}
        }
        if app.executable.is_empty() || app.name == "About SARGA" || app.name == "About SARGA OS" {
            self.about_state.open = true;
            self.damage.mark_full();
            return;
        }
        if app.executable == "/bin/skyfiles" && app.name.starts_with("File") {
            self.spawn_explorer();
            return;
        }
        crate::core::launcher::spawn_app_from_registry(self, &app);
        self.damage.mark_full();
    }

    fn handle_key(&mut self, key: u8) {
        if key == 88 || key == 0x57 {
            self.debug_overlay = !self.debug_overlay;
            self.damage.mark_full();
            return;
        }
        if key == 0x1B {
            self.context_menu = None;
            self.start_menu.open = false;
            self.settings.open = false;
            self.settings_app.open = false;
            if let Some(id) = self.wm.active() {
                if let Some(w) = self.wm.lookup(id) {
                    if w.state == WindowState::Fullscreen {
                        self.wm.toggle_fullscreen(id, self.screen_w, self.screen_h);
                    }
                }
            }
            self.damage.mark_full();
            return;
        }
        if self.start_menu.open {
            match key {
                0x1B => {
                    // Esc
                    self.start_menu.open = false;
                    self.damage.mark_full();
                }
                0x0D | 0x0A => {
                    // Enter
                    if let Some(app_id) = self.start_menu.selected_app() {
                        self.launch_app(app_id);
                        self.damage.mark_full();
                    }
                }
                0x09 => {
                    // Tab → next category
                    self.start_menu.cat_idx =
                        (self.start_menu.cat_idx + 1) % crate::util::app_db::CATEGORIES.len();
                    self.start_menu.selected = 0;
                    self.start_menu.scroll = 0;
                    self.start_menu.rebuild_filter(&self.app_reg);
                    self.damage.mark_full();
                }
                0x7F | 0x08 => {
                    // Backspace
                    self.start_menu.search.pop();
                    self.start_menu.rebuild_filter(&self.app_reg);
                    self.damage.mark_full();
                }
                ch if (ch >= 0x20 && ch <= 0x7E) => {
                    // printable ASCII → search
                    self.start_menu.search.push(ch);
                    self.start_menu.rebuild_filter(&self.app_reg);
                    self.damage.mark_full();
                }
                _ => {}
            }
            return;
        }
        if self.switcher_active {
            match key {
                0x09 => {
                    // Tab → next window
                    self.switcher_idx = (self.switcher_idx + 1) % self.wm.len();
                    self.damage.mark_full();
                }
                0x0D | 0x0A | 0x1B => {
                    // Enter / Escape → confirm selection
                    if let Some(wid) = self.wm.id_at(self.switcher_idx) {
                        self.wm.bring_to_front(wid);
                    }
                    self.switcher_active = false;
                    self.damage.mark_full();
                }
                _ => {}
            }
            return;
        }
        if let Some(action) = self.shortcuts.handle(key) {
            match action {
                ShortcutAction::Quit => {
                    if self.wm.is_empty() {
                        process::exit(0);
                    }
                }
                ShortcutAction::CloseFocused => {
                    if let Some(id) = self.wm.active() {
                        self.wm.close(id);
                        self.damage.mark_full();
                    }
                }
                ShortcutAction::CycleTiling => self.cycle_tiling(),
                ShortcutAction::CycleWindow => self.cycle_window(),
                ShortcutAction::ClipboardPanel => {
                    self.damage.mark_full();
                }
                ShortcutAction::ToggleAot => {
                    if let Some(id) = self.wm.active() {
                        if let Some(w) = self.wm.lookup_mut(id) {
                            w.always_on_top = !w.always_on_top;
                        }
                        self.damage.mark_full();
                    }
                }
                ShortcutAction::DemoNotification => {
                    self.services.notify("Demo", "This is a test notification", 1, 120);
                    self.damage.mark_full();
                }
                ShortcutAction::DismissNotification => {
                    let visible = self.services.notifications.visible_notifications();
                    if let Some(last) = visible.last() {
                        self.services.notifications.dismiss(last.id);
                        self.damage.mark_full();
                    }
                }
                ShortcutAction::ClearNotifications => {
                    self.services.notifications.dismiss_all();
                    self.damage.mark_full();
                }
                ShortcutAction::OpenSettings => {
                    self.settings_app.open = !self.settings_app.open;
                    if self.settings_app.open {
                        self.settings_app.current_page = crate::apps::settings::SettingsPage::Appearance;
                    }
                    self.damage.mark_full();
                }
                ShortcutAction::OpenTaskManager => {
                    self.task_manager.open = !self.task_manager.open;
                    self.damage.mark_full();
                }
            }
            return;
        }
        if key == 0x0C {
            // Ctrl+L = clear terminal
            if let Some(last) = self.wm.focused_mut() {
                if last.focused {
                    last.content.clear();
                    self.damage.mark_full();
                }
            }
            return;
        }
        if key == 0x7F || key == 0x08 {
            // Delete/Backspace → delete selected icons
            let before = self.desktop_icons.icons.len();
            self.desktop_icons.icons.retain(|ic| !ic.selected);
            if self.desktop_icons.icons.len() < before {
                self.damage.mark_full();
                return;
            }
        }
        if b'q' == key && self.wm.is_empty() {
            process::exit(0);
        }
        if key == 0x09 {
            self.handle_key_focus(key);
            return;
        }
        self.damage.mark_full();
        if let Some(last) = self.wm.focused_mut() {
            if last.focused && last.x > -100 {
                let ch = key as char;
                if ch.is_ascii_graphic() || ch == ' ' {
                    if last.content.last().map_or(true, |l| l.len() > 80) {
                        last.content.push(alloc::string::String::new());
                    }
                    if let Some(line) = last.content.last_mut() {
                        line.push(ch);
                    }
                } else if key == 0x0A || key == 0x0D {
                    if let Some(line) = last.content.last_mut() {
                        let cmd = line.clone();
                        last.content.push(alloc::format!("$ {}", cmd));
                    }
                    if last.content.len() > 500 {
                        last.content.drain(0..last.content.len() - 500);
                    }
                } else if key == 0x7F || key == 0x08 {
                    if let Some(line) = last.content.last_mut() {
                        line.pop();
                    }
                }
            }
        }
    }

    pub(crate) fn spawn_app(&mut self, path: &str, title: &str) {
        crate::core::launcher::spawn_app(self, path, title);
    }

    #[allow(dead_code)]
    pub(crate) fn spawn_explorer(&mut self) {
        let id = self.explorers.len() as u32;
        let mut explorer = crate::util::explorer::ExplorerState::new(id, "/home");
        explorer.refresh();
        self.explorers.push(explorer);
        let path = "/bin/skyfiles";
        let mut app_win = crate::core::window::AppWindow {
            x: 60,
            y: 40,
            w: 640,
            h: 440,
            prev_x: 60,
            prev_y: 40,
            prev_w: 640,
            prev_h: 440,
            title: alloc::string::String::from("File Explorer"),
            content: alloc::vec::Vec::new(),
            scroll: 0,
            id: 0,
            pid: None,
            focused: true,
            dragging: false,
            drag_ox: 0,
            drag_oy: 0,
            state: crate::core::window::WindowState::Normal,
            prev_state: crate::core::window::WindowState::Normal,
            flags: VisualFlags::new(),
            selection: None,
            anim: None,
            closing: false,
            anim_opacity: 0,
            always_on_top: false,
            explorer_id: Some(id),
        };
        app_win.content.push(alloc::string::String::new());
        if !path.is_empty() {
            match libsarga::process::fork() {
                Ok(0) => {
                    let _ = libsarga::process::execve(path, &[path], &[]);
                    libsarga::process::exit(1);
                }
                Ok(pid) => {
                    app_win.pid = Some(pid);
                    let app_idx = crate::util::app_db::APPS
                        .iter()
                        .position(|a| a.exec == path)
                        .unwrap_or(0);
                    self.lifecycle.register(pid, app_idx);
                }
                Err(_) => {}
            }
        }
        let wid = self.wm.create(app_win);
        if let Some(w) = self.wm.lookup_mut(wid) {
            w.flags.opacity = 0;
            w.animate_to(w.x, w.y, w.w, w.h);
        }
        self.services.notify("App Launched", "File Explorer", 1, 120);
        self.damage.mark_full();
    }

    #[allow(dead_code)]
    pub(crate) fn spawn_app_ex(&mut self, path: &str, title: &str, x: i32, y: i32, w: u32, h: u32) {
        crate::core::launcher::spawn_app_at(self, path, title, x, y, w, h);
    }

    pub fn update_mouse(&mut self, mx: i32, my: i32, btn: bool) -> (bool, bool, bool) {
        self.mouse_x = mx;
        self.mouse_y = my;
        let just_pressed = btn && !self.mouse_btn;
        let just_released = !btn && self.mouse_btn;
        self.prev_mouse_btn = self.mouse_btn;
        self.mouse_btn = btn;

        if just_pressed {
            let dt = self.clock_ticks.saturating_sub(self.last_click_time);
            let dist = (mx - self.last_click_pos.x).abs() + (my - self.last_click_pos.y).abs();
            self.double_click = dt < 25 && dist < 6;
            self.last_click_time = self.clock_ticks;
            self.last_click_pos = Point::new(mx, my);
            self.drag_active = false;
        }
        if self.mouse_btn && !self.drag_active {
            let dx = mx - self.last_click_pos.x;
            let dy = my - self.last_click_pos.y;
            self.drag_active = dx.abs() + dy.abs() > 4;
        } else if just_released {
            self.drag_active = false;
        }

        self.update_cursor();
        (just_pressed, just_released, self.drag_active)
    }

    pub(crate) fn handle_click(&mut self, mx: i32, my: i32) {
        self.damage.mark_full();
        if self.settings.open {
            if let Some(idx) = self.settings.hit_test(mx, my, &self.snapshot()) {
                match idx {
                    0 => self.settings.sound_on = !self.settings.sound_on,
                    1 => {
                        self.settings.theme_dark = !self.settings.theme_dark;
                        if self.settings.theme_dark {
                            self.theme_svc.set(libsarga::theme::Theme::dark());
                        } else {
                            self.theme_svc.set(libsarga::theme::Theme::light());
                        }
                    }
                    _ => {
                        self.settings.open = false;
                        self.context_menu = None;
                    }
                }
                self.damage.mark_full();
                return;
            }
            self.settings.open = false;
            self.damage.mark_full();
            return;
        }
        self.record_current_focus();
        let taskbar_y = self.taskbar_y() as i32;

        if self.start_menu.open {
            // modern start menu click handling
            let menu_x = 4i32;
            let menu_y = taskbar_y - 5 - 460;
            let menu_w = 480i32;
            let menu_h = 460i32;
            let menu_rect = Rect::new(menu_x, menu_y, menu_w as u32, menu_h as u32);

            if !menu_rect.hit_test(Point::new(mx, my)) {
                self.start_menu.open = false;
                return;
            }

            // search bar click
            let search_y = menu_y + 8;
            if mx >= menu_x + 8 && mx < menu_x + menu_w - 8 && my >= search_y && my < search_y + 36
            {
                return; // focus search (keyboard will handle input)
            }

            // sidebar categories
            let sidebar_x = menu_x + 4;
            let sidebar_y = search_y + 36 + 6;
            for (i, _) in crate::util::app_db::CATEGORIES.iter().enumerate() {
                let iy = sidebar_y + 4 + i as i32 * 28;
                if mx >= sidebar_x + 4 && mx < sidebar_x + 126 && my >= iy && my < iy + 24 {
                    self.start_menu.cat_idx = i;
                    self.start_menu.selected = 0;
                    self.start_menu.scroll = 0;
                    self.start_menu.rebuild_filter(&self.app_reg);
                    return;
                }
            }

            // app list
            let list_x = sidebar_x + 130 + 4;
            let list_y = search_y + 36 + 6;
            let list_w = menu_w - 4 - (list_x - menu_x);
            let list_h = menu_h - (list_y - menu_y) - 44;
            let avail = (list_h as u32 / 32) as usize;
            let start = self.start_menu.scroll as usize;
            let end = (start + avail).min(self.start_menu.filtered.len());
            for i in start..end {
                let iy = list_y + 2 + (i - start) as i32 * 32;
                if mx >= list_x && mx < list_x + list_w && my >= iy && my < iy + 30 {
                    let app_id = self.start_menu.filtered[i];
                    self.launch_app(app_id);
                    return;
                }
            }

            // recent strip
            let bottom_y = menu_y + menu_h - 36;
            let mut rx = menu_x + 72;
            for &idx in self.app_reg.db.recent.iter() {
                if rx > menu_x + menu_w - 20 {
                    break;
                }
                if mx >= rx && mx < rx + 80 && my >= bottom_y + 2 && my < bottom_y + 32 {
                    self.launch_app(AppId(idx));
                    return;
                }
                rx += 84;
            }
            return;
        }

        if self.settings_app.open {
            let snap = self.snapshot();
            let hit_idx = self.settings_app.hit_test(mx, my, &snap);
            drop(snap);
            if let Some(idx) = hit_idx {
                if idx < 10 {
                    let pages = [
                        crate::apps::settings::SettingsPage::Appearance,
                        crate::apps::settings::SettingsPage::Desktop,
                        crate::apps::settings::SettingsPage::Keyboard,
                        crate::apps::settings::SettingsPage::Mouse,
                        crate::apps::settings::SettingsPage::Display,
                        crate::apps::settings::SettingsPage::About,
                        crate::apps::settings::SettingsPage::System,
                        crate::apps::settings::SettingsPage::Power,
                        crate::apps::settings::SettingsPage::Notification,
                        crate::apps::settings::SettingsPage::Theme,
                    ];
                    if idx < pages.len() {
                        self.settings_app.current_page = pages[idx];
                    }
                } else if idx == 10 {
                    self.settings_app.app = !self.settings_app.app;
                    if self.settings_app.app {
                        self.theme_svc.set(libsarga::theme::Theme::dark());
                    } else {
                        self.theme_svc.set(libsarga::theme::Theme::light());
                    }
                }
                self.damage.mark_full();
                return;
            }
            self.settings_app.open = false;
            self.damage.mark_full();
            return;
        }
        if self.task_manager.open {
            let snap = self.snapshot();
            let hit = self.task_manager.hit_test(mx, my, &snap);
            drop(snap);
            if let Some((idx, _action)) = hit {
                self.task_manager.selected = idx;
                if let Some(wid) = self.wm.id_at(idx) {
                    self.wm.bring_to_front(wid);
                }
                self.damage.mark_full();
                return;
            }
            self.task_manager.open = false;
            self.damage.mark_full();
            return;
        }
        if self.about_state.open {
            self.about_state.open = false;
            self.damage.mark_full();
            return;
        }
        if my >= taskbar_y {
            if mx >= 5 && mx < 65 {
                self.start_menu.open_with(&self.app_reg);
                return;
            }
            let btn_x = 75i32;
            for i in 0..self.wm.len() {
                let bx = btn_x + i as i32 * 120;
                if mx >= bx && mx < bx + 115 {
                    let is_min = {
                        let s = self.wm.iter();
                        s[i].state == WindowState::Minimized
                    };
                    if let Some(wid) = self.wm.id_at(i) {
                        if is_min {
                            self.wm.restore(wid);
                        }
                        self.wm.bring_to_front(wid);
                    }
                    return;
                }
            }
            return;
        }

        let pt = Point::new(mx, my);

        // context menu click
        if let Some((cmx, cmy, items)) = self.context_menu {
            let mw = 150u32;
            let mh = items.len() as u32 * 28 + 10;
            if Rect::new(cmx, cmy, mw, mh).hit_test(Point::new(mx, my)) {
                let idx = ((my - cmy - 5) / 28) as usize;
                if idx < items.len() {
                    let action = items[idx].1;
                    self.exec_context_action(action);
                }
            }
            self.context_menu = None;
            self.damage.mark_full();
            return;
        }

        // icon click
        if let Some(idx) = self.desktop_icons.icon_at(mx, my) {
            self.desktop_icons.icons[idx].selected = !self.desktop_icons.icons[idx].selected;
            if self.desktop_icons.icons[idx].selected {
                self.desktop_icons.drag_icon = true; // will move on drag
            }
            return;
        }

        for i in (0..self.wm.len()).rev() {
            let (x, y, w, h) = {
                let s = self.wm.iter();
                (s[i].x, s[i].y, s[i].w, s[i].h)
            };
            let wid = match self.wm.id_at(i) {
                Some(wid) => wid,
                None => continue,
            };
            let wr = Rect::new(x, y, w, h);
            if Rect::new(x, y, w, TITLE_H as u32).hit_test(pt) {
                if self.double_click {
                    self.wm
                        .toggle_maximize(wid, self.screen_w, self.taskbar_y());
                    return;
                }
                self.wm.bring_to_front(wid);
                self.wm.begin_drag(wid, mx, my);
                return;
            }

            if Rect::new(
                x + w as i32 - CLOSE_L,
                y + BTN_TOP,
                (CLOSE_L - CLOSE_R) as u32,
                (BTN_BOT - BTN_TOP) as u32,
            )
            .hit_test(pt)
            {
                self.wm.close(wid);
                return;
            }
            if Rect::new(
                x + w as i32 - MAX_L,
                y + BTN_TOP,
                (MAX_L - MAX_R) as u32,
                (BTN_BOT - BTN_TOP) as u32,
            )
            .hit_test(pt)
            {
                self.wm
                    .toggle_maximize(wid, self.screen_w, self.taskbar_y());
                return;
            }
            if Rect::new(
                x + w as i32 - MIN_L,
                y + BTN_TOP,
                (MIN_L - MIN_R) as u32,
                (BTN_BOT - BTN_TOP) as u32,
            )
            .hit_test(pt)
            {
                self.wm.minimize(wid, self.screen_w, self.taskbar_y());
                return;
            }

            let edges = Self::hit_window_edge(x, y, w, h, mx, my);
            if edges != 0 {
                self.resize_win = Some(wid);
                self.resize_edges = edges;
                self.resize_rect = (x, y, w, h);
                self.wm.bring_to_front(wid);
                return;
            }

            // Explorer content click
            if wr.hit_test(pt) {
                let is_explorer = { self.wm.iter()[i].explorer_id.is_some() };
                if let Some(exp_id) = self.wm.iter()[i].explorer_id {
                    if let Some(exp_state) = self.explorers.iter_mut().find(|e| e.id == exp_id) {
                        let aw_ref = &self.wm.iter()[i];
                        crate::util::explorer::handle_explorer_click(
                            exp_state,
                            mx,
                            my,
                            aw_ref,
                            self.double_click,
                        );
                    }
                    self.wm.bring_to_front(wid);
                    return;
                }
                self.wm.bring_to_front(wid);
                return;
            }
        }

        // desktop click → deselect icons, start rubber band
        for ic in &mut self.desktop_icons.icons {
            ic.selected = false;
        }
        self.desktop_icons.begin_select(mx, my);
    }

    fn hit_window_edge(x: i32, y: i32, w: u32, h: u32, mx: i32, my: i32) -> u8 {
        let mut edges = 0u8;
        if mx >= x && mx < x + RESIZE_MARGIN && my >= y && my < y + h as i32 {
            edges |= 1;
        }
        if mx >= x + w as i32 - RESIZE_MARGIN && mx < x + w as i32 && my >= y && my < y + h as i32 {
            edges |= 2;
        }
        if my >= y + h as i32 - RESIZE_MARGIN && my < y + h as i32 {
            edges |= 4;
        }
        edges
    }

    fn handle_right_click(&mut self, mx: i32, my: i32) {
        self.context_menu = None;
        let pt = Point::new(mx, my);
        let ty = self.taskbar_y() as i32;

        // taskbar right-click
        if my >= ty {
            return;
        }

        // icon right-click
        if let Some(_idx) = self.desktop_icons.icon_at(mx, my) {
            self.context_menu = Some((mx, my, ICON_MENU));
            self.damage.mark_full();
            return;
        }

        // window titlebar right-click → system menu
        for i in (0..self.wm.len()).rev() {
            let (x, y, w, _h) = {
                let s = self.wm.iter();
                (s[i].x, s[i].y, s[i].w, s[i].h)
            };
            if Rect::new(x, y, w, 22).hit_test(pt) {
                if let Some(wid) = self.wm.id_at(i) {
                    self.system_menu_for = Some(wid);
                    self.context_menu = Some((mx, my, SYSTEM_MENU));
                }
                self.damage.mark_full();
                return;
            }
        }

        // desktop right-click
        self.context_menu = Some((mx, my, DESKTOP_MENU));
        self.damage.mark_full();
    }

    fn handle_middle_click(&mut self, mx: i32, my: i32) {
        let taskbar_y = self.taskbar_y() as i32;
        if my >= taskbar_y {
            let btn_x = 75i32;
            for i in 0..self.wm.len() {
                let bx = btn_x + i as i32 * 120;
                if mx >= bx && mx < bx + 115 {
                    if let Some(wid) = self.wm.id_at(i) {
                        self.wm.close(wid);
                    }
                    self.damage.mark_full();
                    return;
                }
            }
        }
        // middle-click on titlebar → close window
        let pt = Point::new(mx, my);
        for i in (0..self.wm.len()).rev() {
            let (x, y, w, _h) = {
                let s = self.wm.iter();
                (s[i].x, s[i].y, s[i].w, s[i].h)
            };
            if Rect::new(x, y, w, 22).hit_test(pt) {
                if let Some(wid) = self.wm.id_at(i) {
                    self.wm.close(wid);
                }
                self.damage.mark_full();
                return;
            }
        }
    }

    fn handle_scroll(&mut self, delta: i8) {
        self.damage.mark_full();
        if let Some(w) = self.wm.focused_mut() {
            let max = w.content.len().saturating_sub(1);
            let step = delta as i32;
            w.scroll = (w.scroll as i32 - step).clamp(0, max as i32) as u32;
        }
    }

    fn update_cursor(&mut self) {
        if self.resize_win.is_some() || self.drag_active {
            return;
        }
        let pt = Point::new(self.mouse_x, self.mouse_y);
        for i in (0..self.wm.len()).rev() {
            let (x, y, w, h) = {
                let s = self.wm.iter();
                (s[i].x, s[i].y, s[i].w, s[i].h)
            };
            let edges = Self::hit_window_edge(x, y, w, h, pt.x, pt.y);
            if edges != 0 {
                self.cursor = match edges {
                    1 | 2 => Cursor::ResizeH,
                    4 => Cursor::ResizeV,
                    3 => Cursor::ResizeDiagonal,
                    _ => Cursor::ResizeDiagonal,
                };
                return;
            }
            if Rect::new(x, y, w, TITLE_H as u32).hit_test(pt) {
                self.cursor = Cursor::Move;
                return;
            }
        }
        // icon hover → Hand cursor
        if self.desktop_icons.icon_at(pt.x, pt.y).is_some() {
            self.cursor = Cursor::Hand;
            return;
        }
        self.cursor = Cursor::Default;
    }

    pub(crate) fn handle_drag(&mut self, mx: i32, my: i32) {
        self.damage.mark_full();
        if self.desktop_icons.drag_icon {
            let dx = mx - self.last_click_pos.x;
            let dy = my - self.last_click_pos.y;
            self.desktop_icons.move_selected(dx, dy);
            return;
        }
        if self.desktop_icons.rubber.is_some() {
            self.desktop_icons.update_rubber(mx, my);
            return;
        }
        if let Some(id) = self.resize_win {
            let (ox, oy, ow, oh) = self.resize_rect;
            let dx = mx - self.last_click_pos.x;
            let dy = my - self.last_click_pos.y;
            if let Some(w) = self.wm.lookup_mut(id) {
                let mut nx = ox;
                let mut nw = ow;
                let mut nh = oh;
                if self.resize_edges & 1 != 0 {
                    nx = ox + dx;
                    nw = (ow as i32 - dx) as u32;
                }
                if self.resize_edges & 2 != 0 {
                    nw = (ow as i32 + dx) as u32;
                }
                if self.resize_edges & 4 != 0 {
                    nh = (oh as i32 + dy) as u32;
                }
                if nw < MIN_WIN_W {
                    nw = MIN_WIN_W;
                    nx = ox + ow as i32 - MIN_WIN_W as i32;
                }
                if nh < MIN_WIN_H {
                    nh = MIN_WIN_H;
                }
                w.x = nx;
                w.y = oy;
                w.w = nw;
                w.h = nh;
            }
        } else {
            self.wm.update_drag(mx, my);
            self.wm.show_snap_preview(mx, my, self.screen_w, self.screen_h, self.taskbar_y());
        }
    }

    pub(crate) fn release_drag(&mut self) {
        self.wm.clear_snap_preview();
        if self.desktop_icons.rubber.is_some() {
            let sel = self.desktop_icons.end_select();
            for i in sel {
                if i < self.desktop_icons.icons.len() {
                    self.desktop_icons.icons[i].selected = true;
                }
            }
            self.damage.mark_full();
            return;
        }
        self.desktop_icons.drag_icon = false;
        if let Some(_id) = self.resize_win {
            self.resize_win = None;
            self.resize_edges = 0;
            self.wm.end_drag();
            return;
        }
        let id = self.wm.active();
        self.wm.end_drag();
        if let Some(id) = id {
            let mx = self.mouse_x;
            let my = self.mouse_y;
            let sw = self.screen_w as i32;
            let ty = self.taskbar_y() as i32;
            let edge_left = mx < SNAP_MARGIN;
            let edge_right = mx > sw - SNAP_MARGIN;
            let edge_top = my < SNAP_MARGIN;
            let edge_bot = my > ty - SNAP_MARGIN;
            match (edge_left, edge_right, edge_top, edge_bot) {
                (true, _, true, _) => self.wm.snap_to_region(
                    id,
                    SnapRegion::TopLeft,
                    self.screen_w,
                    self.screen_h,
                    ty as u32,
                ),
                (true, _, _, true) => self.wm.snap_to_region(
                    id,
                    SnapRegion::BottomLeft,
                    self.screen_w,
                    self.screen_h,
                    ty as u32,
                ),
                (_, true, true, _) => self.wm.snap_to_region(
                    id,
                    SnapRegion::TopRight,
                    self.screen_w,
                    self.screen_h,
                    ty as u32,
                ),
                (_, true, _, true) => self.wm.snap_to_region(
                    id,
                    SnapRegion::BottomRight,
                    self.screen_w,
                    self.screen_h,
                    ty as u32,
                ),
                (true, _, _, _) => self.wm.snap_to_region(
                    id,
                    SnapRegion::Left,
                    self.screen_w,
                    self.screen_h,
                    ty as u32,
                ),
                (_, true, _, _) => self.wm.snap_to_region(
                    id,
                    SnapRegion::Right,
                    self.screen_w,
                    self.screen_h,
                    ty as u32,
                ),
                (_, _, true, _) => self.wm.snap_to_region(
                    id,
                    SnapRegion::Top,
                    self.screen_w,
                    self.screen_h,
                    ty as u32,
                ),
                (_, _, _, true) => self.wm.snap_to_region(
                    id,
                    SnapRegion::Bottom,
                    self.screen_w,
                    self.screen_h,
                    ty as u32,
                ),
                _ => {}
            }
        }
        self.resize_win = None;
        self.resize_edges = 0;
    }

    pub(crate) fn cursor_alpha(&self) -> u8 {
        self.cursor_alpha
    }

    pub fn render_snap_preview(&self) -> Option<(i32, i32, u32, u32)> {
        self.wm.snap_preview.as_ref().filter(|sp| sp.active).map(|sp| (sp.x, sp.y, sp.w, sp.h))
    }

    pub(crate) fn prepare_clock(&mut self) -> alloc::string::String {
        alloc::string::String::from(crate::render::clock::format_time(
            self.clock_ticks,
            &mut self.clock_cache,
        ))
    }

    pub fn permission_check(&self, app: crate::ipc::ApplicationId, perm: crate::ipc::permission::AppPermission) -> bool {
        self.permissions.check(app.0, perm)
    }

    /// Drains pending IPC service requests, gates each on the service's required
    /// permissions for the caller, and dispatches allowed ones through the
    /// security portal. Runs once per frame from `tick()`.
    pub fn process_ipc(&mut self) {
        use crate::ipc::permission::AppPermission;
        // ponytail: soft-real-time ceiling — never stall a frame on a huge
        // queue; leftovers drain next frame. Load-bearing once a real IPC
        // transport lets external processes enqueue requests.
        const MAX_REQUESTS_PER_FRAME: usize = 64;
        let mut requests = self.ipc_server.drain_requests();
        if requests.len() > MAX_REQUESTS_PER_FRAME {
            self.ipc_server.pending_requests = requests.split_off(MAX_REQUESTS_PER_FRAME);
        }
        for req in requests {
            let app = req.sender;
            let granted = self.permissions.granted(app.0);
            let allowed = self
                .service_registry
                .find(req.service)
                .map(|info| {
                    granted.map_or(false, |g| {
                        g.contains(AppPermission::from_bits_truncate(info.required_permissions))
                    })
                })
                .unwrap_or(false);
            let resp = if allowed {
                crate::sec::portal::dispatch(self, app, &req)
            } else {
                crate::ipc::ServiceResponse {
                    request_id: req.request_id,
                    success: false,
                    data: alloc::vec::Vec::new(),
                    recipient: app,
                }
            };
            self.ipc_server.submit_response(resp);
        }
    }

    pub fn snapshot(&self) -> RenderSnapshot<'_> {
        let fs = self
            .wm
            .iter()
            .iter()
            .any(|w| w.state == WindowState::Fullscreen);

        let focused_bounds = self.focus.focused().and_then(|id| {
            self.a11y_tree.nodes.iter().find(|n| n.id == id).map(|n| n.bounds)
        });

        let (tooltip_text, tooltip_x, tooltip_y) = match self.tooltips.active {
            Some(ref t) if t.visible => (Some(t.text.as_str()), t.x, t.y),
            _ => (None, 0, 0),
        };

        RenderSnapshot {
            screen_w: self.screen_w,
            screen_h: self.screen_h,
            theme: self.theme_svc.current(),
            windows: self.wm.iter(),
            icons: &self.desktop_icons.icons,
            mouse: crate::core::geometry::Point::new(self.mouse_x, self.mouse_y),
            debug_overlay: self.debug_overlay,
            debug_metrics: self.profiler.snapshot(),
            window_count: self.wm.len(),
            notification_count: self.services.notifications.visible_notifications().len(),
            start_menu: self.start_menu.open,
            start_menu_state: Some(&self.start_menu),
            app_db: Some(&self.app_reg.db),
            app_reg: Some(&self.app_reg),
            context_menu: self.context_menu,
            cursor_visible: self.cursor_visible,
            cursor_alpha: self.cursor_alpha(),
            fullscreen: fs,
            switcher_active: self.switcher_active,
            switcher_idx: self.switcher_idx,
            rubber: self.desktop_icons.rubber,
            notifications: self.services.notifications.visible_notifications(),
            tray: self.tray.entries,
            clipboard: Some(&self.services.clipboard),
            settings: Some(&self.settings),
            explorers: &self.explorers,
            settings_app: Some(&self.settings_app),
            task_manager: Some(&self.task_manager),
            about: Some(&self.about_state),
            focused_id: self.focus.focused(),
            focus_visible: self.focus_visible,
            focused_bounds,
            tooltip: tooltip_text,
            tooltip_x,
            tooltip_y,
            snap_preview: self.render_snap_preview(),
        }
    }
}
