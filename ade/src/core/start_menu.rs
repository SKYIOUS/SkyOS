//! Modern start menu — categories, search, pinned, keyboard navigation.

use crate::util::app_db::{AppCategory, CATEGORIES};
use crate::util::app_registry::{AppId, AppRegistry};
use crate::core::event::Event;
use crate::core::geometry::{Point, Rect};
use crate::render::compositor::Canvas;
use crate::render::snapshot::RenderSnapshot;
use alloc::vec::Vec;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum MenuSection {
    Search,
    Sidebar,
    AppList,
    Recent,
    Power,
}

pub(crate) struct StartMenuState {
    pub open: bool,
    pub search: Vec<u8>,
    pub cat_idx: usize,
    pub selected: usize,
    pub scroll: u32,
    pub filtered: Vec<AppId>,
    pub keyboard_active: bool,
    pub section: MenuSection,
    pub anim_progress: u8,
    pub anim_direction: i8,
    power_idx: usize,
}

const POWER_LABELS: &[(char, &str)] = &[
    ('P', "Shutdown"),
    ('R', "Restart"),
    ('L', "Logout"),
];

impl StartMenuState {
    pub fn new() -> Self {
        StartMenuState {
            open: false,
            search: Vec::new(),
            cat_idx: 0,
            selected: 0,
            scroll: 0,
            filtered: Vec::new(),
            keyboard_active: false,
            section: MenuSection::Search,
            anim_progress: 0,
            anim_direction: 0,
            power_idx: 0,
        }
    }

    pub fn open_with(&mut self, reg: &AppRegistry) {
        self.open = true;
        self.search.clear();
        self.cat_idx = 0;
        self.selected = 0;
        self.scroll = 0;
        self.anim_progress = 0;
        self.anim_direction = 1;
        self.keyboard_active = false;
        self.section = MenuSection::Search;
        self.power_idx = 0;
        self.rebuild_filter(reg);
    }

    pub fn rebuild_filter(&mut self, reg: &AppRegistry) {
        let cat = if self.cat_idx < CATEGORIES.len() {
            CATEGORIES[self.cat_idx].1
        } else {
            AppCategory::All
        };
        self.filtered = reg.filtered(cat, &self.search);
        if self.filtered.is_empty() {
            self.selected = 0;
        } else if self.selected >= self.filtered.len() {
            self.selected = self.filtered.len() - 1;
        }
    }

    pub fn selected_app(&self) -> Option<AppId> {
        if self.selected < self.filtered.len() {
            Some(self.filtered[self.selected])
        } else {
            None
        }
    }

    pub fn tick_anim(&mut self) {
        if self.anim_direction == 1 {
            if self.anim_progress < 255 {
                self.anim_progress = self.anim_progress.saturating_add(30);
            } else {
                self.anim_direction = 0;
            }
        } else if self.anim_direction == -1 {
            if self.anim_progress > 0 {
                self.anim_progress = self.anim_progress.saturating_sub(30);
            } else {
                self.anim_direction = 0;
                self.open = false;
            }
        }
    }

    pub fn start_close(&mut self) {
        self.anim_direction = -1;
    }

    pub fn move_selection(&mut self, dx: i32, dy: i32, reg: &AppRegistry) {
        self.keyboard_active = true;
        let cat_count = CATEGORIES.len();
        match self.section {
            MenuSection::Search => {
                if dy > 0 || dx > 0 {
                    self.section = MenuSection::Sidebar;
                    self.cat_idx = 0;
                    self.rebuild_filter(reg);
                }
            }
            MenuSection::Sidebar => {
                if dy > 0 && self.cat_idx + 1 < cat_count {
                    self.cat_idx += 1;
                    self.rebuild_filter(reg);
                } else if dy < 0 && self.cat_idx > 0 {
                    self.cat_idx -= 1;
                    self.rebuild_filter(reg);
                } else if dx > 0 {
                    self.section = MenuSection::AppList;
                    self.selected = 0;
                    self.scroll = 0;
                } else if dx < 0 {
                    self.section = MenuSection::Search;
                }
            }
            MenuSection::AppList => {
                if !self.filtered.is_empty() {
                    let max = self.filtered.len();
                    if dy > 0 && self.selected + 1 < max {
                        self.selected += 1;
                        let page = ((MENU_H - 80) / ITEM_H) as usize;
                        if self.selected >= self.scroll as usize + page {
                            self.scroll += 1;
                        }
                    } else if dy < 0 && self.selected > 0 {
                        self.selected -= 1;
                        if self.selected < self.scroll as usize {
                            self.scroll = self.scroll.saturating_sub(1);
                        }
                    } else if dy < 0 && self.selected == 0 {
                        self.section = MenuSection::Sidebar;
                    } else if dy > 0 && self.selected + 1 >= max {
                        self.section = MenuSection::Recent;
                    } else if dx < 0 {
                        self.section = MenuSection::Sidebar;
                    } else if dx > 0 {
                        self.section = MenuSection::Recent;
                    }
                } else if dx < 0 {
                    self.section = MenuSection::Sidebar;
                }
            }
            MenuSection::Recent => {
                if dy > 0 {
                    self.section = MenuSection::Power;
                } else if dy < 0 {
                    self.section = MenuSection::AppList;
                } else if dx > 0 {
                    self.section = MenuSection::Power;
                }
            }
            MenuSection::Power => {
                if dy < 0 {
                    self.section = MenuSection::Recent;
                } else if dx > 0 && self.power_idx + 1 < POWER_LABELS.len() {
                    self.power_idx += 1;
                } else if dx < 0 && self.power_idx > 0 {
                    self.power_idx -= 1;
                } else if dx < 0 && self.power_idx == 0 {
                    self.section = MenuSection::Recent;
                } else if dx > 0 && self.power_idx + 1 >= POWER_LABELS.len() {
                    self.section = MenuSection::Recent;
                }
            }
        }
    }

    pub fn activate_selected(&mut self) -> Option<Event> {
        match self.section {
            MenuSection::AppList => {
                self.selected_app().map(|id| Event::ElementActivated(id.0 as u32))
            }
            MenuSection::Power => {
                let code = match self.power_idx {
                    0 => 0,
                    1 => 1,
                    2 => 2,
                    _ => return None,
                };
                Some(Event::PowerRequest(code))
            }
            _ => None,
        }
    }
}

const MENU_W: u32 = 480;
const MENU_H: u32 = 460;
const SEARCH_H: u32 = 36;
const ITEM_H: u32 = 32;
const SIDEBAR_W: u32 = 130;

pub(crate) fn draw(canvas: &mut Canvas, snap: &RenderSnapshot) {
    let state = match snap.start_menu_state {
        Some(s) => s,
        None => return,
    };
    let reg = match snap.app_reg {
        Some(r) => r,
        None => return,
    };
    if !state.open {
        return;
    }

    let theme = snap.theme;
    let ty = snap.taskbar_y();

    let anim = state.anim_progress as u32;
    let anim_offset = if anim < 255 {
        MENU_H - (MENU_H * anim) / 255
    } else {
        0
    };
    let menu_x = 4u32;
    let menu_y = ty.saturating_sub(MENU_H + 4) + anim_offset;

    canvas.draw_shadow(menu_x, menu_y, MENU_W, MENU_H, 8, theme.shadow);
    canvas.draw_rounded_rect(menu_x, menu_y, MENU_W, MENU_H, 8, theme.bg_primary);
    canvas.draw_rounded_rect_outline(menu_x, menu_y, MENU_W, MENU_H, 8, theme.border);

    let search_y = menu_y + 8;
    canvas.draw_rounded_rect(menu_x + 8, search_y, MENU_W - 16, SEARCH_H, 6, theme.bg_surface);
    if state.section == MenuSection::Search && state.keyboard_active {
        canvas.draw_rounded_rect_outline(menu_x + 8, search_y, MENU_W - 16, SEARCH_H, 6, theme.accent);
    }
    let display = if state.search.is_empty() {
        ">  Search applications..."
    } else {
        core::str::from_utf8(&state.search).unwrap_or("")
    };
    canvas.draw_string(
        menu_x + 14,
        search_y + 8,
        display,
        if state.search.is_empty() {
            theme.text_disabled
        } else {
            theme.text
        },
        0,
    );

    let sidebar_x = menu_x + 4;
    let sidebar_y = search_y + SEARCH_H + 6;
    let sidebar_h = MENU_H - (sidebar_y - menu_y) - 8;
    canvas.draw_rounded_rect(sidebar_x, sidebar_y, SIDEBAR_W, sidebar_h, 6, theme.bg_surface);

    for (i, &(cat_name, _)) in CATEGORIES.iter().enumerate() {
        let iy = sidebar_y + 4 + i as u32 * 28;
        if iy + 24 > sidebar_y + sidebar_h {
            break;
        }
        let selected = i == state.cat_idx;
        let hover = Rect::new((sidebar_x + 4) as i32, iy as i32, SIDEBAR_W - 8, 24).hit_test(snap.mouse);
        let bg = if selected {
            theme.accent
        } else if hover {
            theme.hover
        } else {
            theme.bg_surface
        };
        let txt = if selected { theme.text } else { theme.text_secondary };
        canvas.draw_rounded_rect(sidebar_x + 4, iy, SIDEBAR_W - 8, 24, 4, bg);
        canvas.draw_string(sidebar_x + 10, iy + 5, cat_name, txt, 0);
    }

    let list_x = sidebar_x + SIDEBAR_W + 4;
    let list_y = search_y + SEARCH_H + 6;
    let list_w = MENU_W - 4 - (list_x - menu_x);
    let list_h = MENU_H - (list_y - menu_y) - 44;
    let avail = (list_h / ITEM_H) as usize;

    if !state.filtered.is_empty() {
        let start = state.scroll as usize;
        let end = (start + avail).min(state.filtered.len());
        for i in start..end {
            let app_id = state.filtered[i];
            let app = &reg.apps[app_id.0];
            let iy = list_y + 2 + (i - start) as u32 * ITEM_H;
            let sel = i == state.selected && state.section == MenuSection::AppList;
            let hover = Rect::new(list_x as i32, iy as i32, list_w, ITEM_H - 2).hit_test(snap.mouse);
            let bg = if sel {
                theme.accent
            } else if hover {
                theme.hover
            } else {
                theme.bg_primary
            };
            canvas.draw_rounded_rect(list_x, iy, list_w, ITEM_H - 2, 4, bg);
            let label = if app.name.len() > 26 {
                &app.name[..26]
            } else {
                app.name
            };
            canvas.draw_char(list_x + 8, iy + 7, app.icon, if sel { theme.text } else { theme.text_secondary }, 0);
            canvas.draw_string(
                list_x + 18,
                iy + 7,
                label,
                if sel { theme.text } else { theme.text_secondary },
                0,
            );
            if reg.db.pinned[app_id.0] {
                canvas.draw_string(list_x + list_w - 50, iy + 7, "[Pin]", theme.warning, 0);
            }
        }
    }

    let bottom_y = menu_y + MENU_H - 36;
    canvas.draw_rect(menu_x + 2, bottom_y, MENU_W - 4, 34, theme.bg_surface);

    canvas.draw_string(menu_x + 12, bottom_y + 10, "Recent:", theme.text_disabled, 0);
    let mut rx = menu_x + 72;
    let recent_n = if reg.db.recent.len() > 2 { 2 } else { reg.db.recent.len() };
    for ri in 0..recent_n {
        let idx = reg.db.recent[ri];
        if idx >= reg.apps.len() {
            continue;
        }
        if rx + 84 > menu_x + MENU_W - 210 {
            break;
        }
        let app = &reg.apps[idx];
        let label = if app.name.len() > 10 {
            &app.name[..10]
        } else {
            app.name
        };
        let hover = Rect::new(rx as i32, bottom_y as i32 + 2, 80, 30)
            .hit_test(Point::new(snap.mouse.x, snap.mouse.y));
        let sel = state.section == MenuSection::Recent && ri == 0;
        canvas.draw_rounded_rect(rx, bottom_y + 4, 80, 26, 4,
            if sel { theme.accent } else if hover { theme.border } else { theme.bg_elevated });
        canvas.draw_string(rx + 6, bottom_y + 7, label,
            if sel { theme.text } else { theme.text_secondary }, 0);
        rx += 84;
    }

    let power_x = menu_x + MENU_W - 202;
    let power_y = bottom_y + 4;
    for (pi, &(icon, label)) in POWER_LABELS.iter().enumerate() {
        let px = power_x + pi as u32 * 64;
        let hover = Rect::new(px as i32, power_y as i32, 58, 26).hit_test(snap.mouse);
        let sel = pi == state.power_idx && state.section == MenuSection::Power;
        let bg = if sel { theme.accent } else if hover { theme.hover } else { theme.bg_elevated };
        canvas.draw_rounded_rect(px, power_y, 58, 26, 4, bg);
        canvas.draw_char(px + 6, power_y + 5, icon, if sel { theme.text } else { theme.text_secondary }, 0);
        canvas.draw_string(px + 16, power_y + 5, label, if sel { theme.text } else { theme.text_secondary }, 0);
    }
}
