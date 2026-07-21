//! Desktop coordinator — event dispatch, window management, layout logic.

use crate::app_db::APPS;
use crate::constants::TASKBAR_H;
use crate::damage::DamageTracker;
use crate::desktop_icons::DesktopIcons;
use crate::event::Event;
use crate::geometry::{Point, Rect};
use crate::notification::NotificationCenter;
use crate::start_menu::StartMenuState;
use crate::tray::SystemTray;
use crate::window::{VisualFlags, WindowId, WindowState};
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;
use libsarga::process;

pub(crate) enum Cursor {
    Default,
    Move,
    ResizeH,
    ResizeV,
}

const RESIZE_MARGIN: i32 = 4;
const MIN_WIN_W: u32 = 100;
const MIN_WIN_H: u32 = 80;

const TITLE_H: i32 = 22;
const BTN_TOP: i32 = 3;
const BTN_BOT: i32 = 19;
const CLOSE_R: i32 = 4;
const CLOSE_L: i32 = 24;
const MAX_R: i32 = 58;
const MAX_L: i32 = 80;
const MIN_R: i32 = 28;
const MIN_L: i32 = 48;
const SNAP_MARGIN: i32 = 15;

const DESKTOP_MENU: &[(&str, &str)] = &[
    ("New Terminal", "terminal"),
    ("Arrange Icons", "arrange"),
    ("Paste", "paste"),
    ("---", ""),
    ("Change Wallpaper", "wallpaper"),
    ("Settings", "settings"),
];

const ICON_MENU: &[(&str, &str)] = &[
    ("Open", "open"),
    ("Delete", "delete"),
    ("Rename", "rename"),
    ("---", ""),
    ("Properties", "properties"),
];
use crate::window_manager::{SnapRegion, WindowManager};

pub(crate) enum TilingMode {
    Floating,
    Tile,
    Monocle,
}
use crate::render::snapshot::RenderSnapshot;
use crate::shortcut::{ShortcutAction, ShortcutManager};

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
    resize_win: Option<WindowId>,
    resize_edges: u8,
    resize_rect: (i32, i32, u32, u32),
    last_click_time: u64,
    last_click_pos: Point,
    pub(crate) double_click: bool,
    pub(crate) desktop_icons: DesktopIcons,
    pub(crate) theme_svc: crate::theme_service::ThemeService,
    pub(crate) clipboard_svc: crate::clipboard_service::ClipboardService,
    pub(crate) damage: DamageTracker,
    pub(crate) clock_cache: crate::render::clock::ClockCache,
    shortcuts: ShortcutManager,
    tiling_mode: TilingMode,
    prev_tiling_geos: alloc::vec::Vec<(i32, i32, u32, u32)>,
    focus_history: VecDeque<usize>,
    switcher_active: bool,
    switcher_idx: usize,
    pub(crate) app_reg: crate::app_registry::AppRegistry,
    pub(crate) lifecycle: crate::lifecycle::LifecycleManager,
    pub(crate) notif: NotificationCenter,
    pub(crate) tray: SystemTray,
    pub(crate) settings: crate::settings::SettingsState,
    pub(crate) services: crate::service_manager::ServiceManager,
    #[allow(dead_code)]
    pub(crate) session: crate::login_session::LoginSession,
    #[allow(dead_code)]
    pub(crate) file_assoc: crate::file_assoc::FileAssociationEngine,
    #[allow(dead_code)]
    pub(crate) vfs: crate::vfs::VfsContext,
    pub(crate) watcher: crate::watcher::FileWatcher,
    pub(crate) explorers: alloc::vec::Vec<crate::explorer::ExplorerState>,
    #[allow(dead_code)]
    pub(crate) recovery: crate::recovery::RecoverySystem,
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
            resize_win: None,
            resize_edges: 0,
            resize_rect: (0, 0, 0, 0),
            last_click_time: 0,
            last_click_pos: Point::new(0, 0),
            double_click: false,
            desktop_icons: DesktopIcons::new(),
            theme_svc: crate::theme_service::ThemeService::new(),
            clipboard_svc: crate::clipboard_service::ClipboardService::new(),
            damage: DamageTracker::new(),
            clock_cache: crate::render::clock::ClockCache::new(),
            shortcuts: ShortcutManager::new(),
            tiling_mode: TilingMode::Floating,
            prev_tiling_geos: alloc::vec::Vec::new(),
            focus_history: VecDeque::new(),
            switcher_active: false,
            switcher_idx: 0,
            app_reg: crate::app_registry::AppRegistry::new(),
            lifecycle: crate::lifecycle::LifecycleManager::new(),
            notif: NotificationCenter::new(),
            tray: SystemTray::new(),
            settings: crate::settings::SettingsState::new(),
            services: crate::service_manager::ServiceManager::new(),
            session: crate::login_session::LoginSession::new(),
            file_assoc: crate::file_assoc::FileAssociationEngine::new(),
            vfs: crate::vfs::VfsContext::new(),
            watcher: crate::watcher::FileWatcher::new(),
            explorers: alloc::vec::Vec::new(),
            recovery: crate::recovery::RecoverySystem::new(),
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
                Ok((pid, _)) if pid > 0 => {
                    self.lifecycle.mark_terminated(pid);
                    self.wm.close_by_pid(pid);
                    self.damage.mark_full();
                }
                _ => break,
            }
        }
    }

    pub fn tick(&mut self) {
        self.advance_clock();
        if self.clock_ticks % 30 == 0 {
            self.cursor_visible = !self.cursor_visible;
            self.damage.mark_full();
        }
        self.reap_children();
        let mut anim_active = false;
        for w in self.wm.iter_mut() {
            if w.flags.opacity < 255 {
                w.flags.opacity = w.flags.opacity.saturating_add(25);
            }
            if w.tick_animation() {
                anim_active = true;
            }
        }
        if anim_active {
            self.damage.mark_full();
        }
        self.notif.tick();
        self.services.tick();
        self.watcher.poll();
    }

    fn save_geometries(&mut self) {
        self.prev_tiling_geos.clear();
        for w in self.wm.iter() {
            self.prev_tiling_geos.push((w.x, w.y, w.w, w.h));
        }
    }

    fn restore_geometries(&mut self) {
        for (i, &(x, y, w, h)) in self.prev_tiling_geos.iter().enumerate() {
            if let Some(aw) = self.wm.lookup_mut(WindowId(i)) {
                aw.x = x;
                aw.y = y;
                aw.w = w;
                aw.h = h;
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
            if let Some(w) = self.wm.lookup_mut(WindowId(0)) {
                w.x = 0;
                w.y = 0;
                w.w = sw;
                w.h = th;
            }
            return;
        }
        let master_w = sw * 6 / 10;
        let stack_w = sw - master_w;
        let stack_h = th / (n as u32 - 1);
        for i in 0..n {
            if let Some(w) = self.wm.lookup_mut(WindowId(i)) {
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
        self.tiling_mode = TilingMode::Tile;
        self.damage.mark_full();
    }

    fn apply_monocle(&mut self) {
        self.save_geometries();
        let sw = self.screen_w;
        let th = self.taskbar_y();
        for i in 0..self.wm.len() {
            if let Some(w) = self.wm.lookup_mut(WindowId(i)) {
                w.x = 0;
                w.y = 0;
                w.w = sw;
                w.h = th;
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
            let current = self.wm.active().map(|id| id.0).unwrap_or(0);
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
            Event::Key(key) => self.handle_key(key),
            Event::MouseClick(x, y) => self.handle_click(x, y),
            Event::MouseMiddle(x, y) => self.handle_middle_click(x, y),
            Event::MouseRight(x, y) => self.handle_right_click(x, y),
            Event::MouseDrag(x, y) => self.handle_drag(x, y),
            Event::Scroll(delta) => self.handle_scroll(delta),
            Event::MouseRelease => self.release_drag(),
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
            _ => {}
        }
    }

    fn launch_app(&mut self, app_idx: usize) {
        self.start_menu.open = false;
        self.app_reg.record_launch(app_idx);
        let app = &APPS[app_idx];
        if app.exec == "/bin/skyfiles" && app.name.starts_with("File") {
            self.spawn_explorer();
            return;
        }
        if app.exec.is_empty() {
            if app.name == "About SARGA" || app.name == "About SARGA OS" {
                self.spawn_app("", "About SARGA OS");
                if let Some(w) = self.wm.focused_mut() {
                    w.content.clear();
                    w.content.push(alloc::format!(
                        "  SARGA OS v{}",
                        libsarga::version::SKYOS_VERSION
                    ));
                    w.content.push(alloc::string::String::new());
                    w.content
                        .push(alloc::string::String::from("  Kernel: SARGA"));
                    w.content
                        .push(alloc::string::String::from("  Arch: x86_64"));
                    w.content
                        .push(alloc::string::String::from("  Shell: SargaSH"));
                    w.content
                        .push(alloc::string::String::from("  Desktop: ADE"));
                    w.content
                        .push(alloc::string::String::from("  Widgets: libsarga"));
                    w.content.push(alloc::string::String::new());
                    w.content.push(alloc::string::String::from(
                        "  A modern OS written in Rust.",
                    ));
                }
            }
        } else {
            self.spawn_app(app.exec, app.name);
        }
    }

    fn handle_key(&mut self, key: u8) {
        if self.start_menu.open {
            match key {
                0x1B => {
                    // Esc
                    self.start_menu.open = false;
                    self.damage.mark_full();
                }
                0x0D | 0x0A => {
                    // Enter
                    if let Some(app_idx) = self.start_menu.selected_app() {
                        self.launch_app(app_idx);
                        self.damage.mark_full();
                    }
                }
                0x09 => {
                    // Tab → next category
                    self.start_menu.cat_idx =
                        (self.start_menu.cat_idx + 1) % crate::app_db::CATEGORIES.len();
                    self.start_menu.selected = 0;
                    self.start_menu.scroll = 0;
                    self.start_menu.rebuild_filter(&self.app_reg.db);
                    self.damage.mark_full();
                }
                0x7F | 0x08 => {
                    // Backspace
                    self.start_menu.search.pop();
                    self.start_menu.rebuild_filter(&self.app_reg.db);
                    self.damage.mark_full();
                }
                ch if (ch >= 0x20 && ch <= 0x7E) => {
                    // printable ASCII → search
                    self.start_menu.search.push(ch);
                    self.start_menu.rebuild_filter(&self.app_reg.db);
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
                    if self.switcher_idx < self.wm.len() {
                        self.wm.bring_to_front(WindowId(self.switcher_idx));
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
                    self.clipboard_svc.panel_open = !self.clipboard_svc.panel_open;
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
        if key == 0x1B {
            // Escape exits fullscreen
            if let Some(id) = self.wm.active() {
                if let Some(w) = self.wm.lookup(id) {
                    if w.state == WindowState::Fullscreen {
                        self.wm.toggle_fullscreen(id, self.screen_w, self.screen_h);
                        self.damage.mark_full();
                        return;
                    }
                }
            }
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
        crate::launcher::spawn_app(self, path, title);
    }

    #[allow(dead_code)]
    pub(crate) fn spawn_explorer(&mut self) {
        let id = self.explorers.len() as u32;
        let mut explorer = crate::explorer::ExplorerState::new(id, "/home");
        explorer.refresh();
        self.explorers.push(explorer);
        let path = "/bin/skyfiles";
        let mut app_win = crate::window::AppWindow {
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
            pid: None,
            focused: true,
            dragging: false,
            drag_ox: 0,
            drag_oy: 0,
            state: crate::window::WindowState::Normal,
            prev_state: crate::window::WindowState::Normal,
            flags: VisualFlags::new(),
            selection: None,
            anim: None,
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
                    let app_idx = crate::app_db::APPS
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
        self.notif.push("App Launched", "File Explorer", 1, 120);
        self.damage.mark_full();
    }

    #[allow(dead_code)]
    pub(crate) fn spawn_app_ex(&mut self, path: &str, title: &str, x: i32, y: i32, w: u32, h: u32) {
        crate::launcher::spawn_app_at(self, path, title, x, y, w, h);
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
            for (i, _) in crate::app_db::CATEGORIES.iter().enumerate() {
                let iy = sidebar_y + 4 + i as i32 * 28;
                if mx >= sidebar_x + 4 && mx < sidebar_x + 126 && my >= iy && my < iy + 24 {
                    self.start_menu.cat_idx = i;
                    self.start_menu.selected = 0;
                    self.start_menu.scroll = 0;
                    self.start_menu.rebuild_filter(&self.app_reg.db);
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
                    let app_idx = self.start_menu.filtered[i];
                    self.launch_app(app_idx);
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
                    self.launch_app(idx);
                    return;
                }
                rx += 84;
            }
            return;
        }

        if my >= taskbar_y {
            if mx >= 5 && mx < 65 {
                self.start_menu.open_with(&self.app_reg.db);
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
                    if is_min {
                        self.wm.restore(WindowId(i));
                    }
                    self.wm.bring_to_front(WindowId(i));
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
            let wr = Rect::new(x, y, w, h);
            if Rect::new(x, y, w, TITLE_H as u32).hit_test(pt) {
                if self.double_click {
                    self.wm
                        .toggle_maximize(WindowId(i), self.screen_w, self.taskbar_y());
                    return;
                }
                self.wm.bring_to_front(WindowId(i));
                self.wm.begin_drag(WindowId(i), mx, my);
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
                self.wm.close(WindowId(i));
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
                    .toggle_maximize(WindowId(i), self.screen_w, self.taskbar_y());
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
                self.wm.minimize(WindowId(i));
                return;
            }

            let edges = Self::hit_window_edge(x, y, w, h, mx, my);
            if edges != 0 {
                self.resize_win = Some(WindowId(i));
                self.resize_edges = edges;
                self.resize_rect = (x, y, w, h);
                self.wm.bring_to_front(WindowId(i));
                return;
            }

            // Explorer content click
            if wr.hit_test(pt) {
                let is_explorer = { self.wm.iter()[i].explorer_id.is_some() };
                if is_explorer {
                    let exp_id = self.wm.iter()[i].explorer_id.unwrap();
                    if let Some(exp_state) = self.explorers.iter_mut().find(|e| e.id == exp_id) {
                        let aw_ref = &self.wm.iter()[i];
                        crate::explorer::handle_explorer_click(
                            exp_state,
                            mx,
                            my,
                            aw_ref,
                            self.double_click,
                        );
                    }
                    self.wm.bring_to_front(WindowId(i));
                    return;
                }
                self.wm.bring_to_front(WindowId(i));
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

        // window titlebar right-click
        for i in (0..self.wm.len()).rev() {
            let (x, y, w, _h) = {
                let s = self.wm.iter();
                (s[i].x, s[i].y, s[i].w, s[i].h)
            };
            if Rect::new(x, y, w, 22).hit_test(pt) {
                self.wm
                    .toggle_maximize(WindowId(i), self.screen_w, self.taskbar_y());
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
                    self.wm.close(WindowId(i));
                    self.damage.mark_full();
                    return;
                }
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
                    _ => Cursor::ResizeV,
                };
                return;
            }
            if Rect::new(x, y, w, TITLE_H as u32).hit_test(pt) {
                self.cursor = Cursor::Move;
                return;
            }
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
        }
    }

    pub(crate) fn release_drag(&mut self) {
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

    pub(crate) fn prepare_clock(&mut self) -> alloc::string::String {
        alloc::string::String::from(crate::render::clock::format_time(
            self.clock_ticks,
            &mut self.clock_cache,
        ))
    }

    pub fn snapshot(&self) -> RenderSnapshot<'_> {
        let fs = self
            .wm
            .iter()
            .iter()
            .any(|w| w.state == WindowState::Fullscreen);
        RenderSnapshot {
            screen_w: self.screen_w,
            screen_h: self.screen_h,
            theme: self.theme_svc.current(),
            windows: self.wm.iter(),
            icons: &self.desktop_icons.icons,
            mouse: crate::geometry::Point::new(self.mouse_x, self.mouse_y),
            start_menu: self.start_menu.open,
            start_menu_state: Some(&self.start_menu),
            app_db: Some(&self.app_reg.db),
            app_reg: Some(&self.app_reg),
            context_menu: self.context_menu,
            cursor_visible: self.cursor_visible,
            fullscreen: fs,
            switcher_active: self.switcher_active,
            switcher_idx: self.switcher_idx,
            rubber: self.desktop_icons.rubber,
            notifications: &self.notif.active,
            tray: self.tray.entries,
            clipboard: Some(&self.clipboard_svc),
            settings: Some(&self.settings),
            explorers: &self.explorers,
        }
    }
}
