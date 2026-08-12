//! Modern start menu — categories, search, pinned, keyboard navigation.

use crate::core::geometry::{Point, Rect};
use crate::core::window::HoverTarget;
use crate::layout;
use crate::render::compositor::Canvas;
use crate::render::snapshot::RenderSnapshot;
use crate::util::app_catalog::{AppCatalog, AppCategory, AppId, CATEGORIES};
use alloc::vec::Vec;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum MenuSection {
    Search,
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

const POWER_LABELS: &[(char, &str)] = &[('P', "Shutdown"), ('R', "Restart"), ('L', "Logout")];

/// Which row of the menu the pointer is over, if any. The single source of
/// truth for start-menu row geometry — shared by `Desktop::hover_target`
/// (hover feedback), `Desktop::handle_click` (maps the returned
/// `HoverTarget` to its click action), and the draw below (compares against
/// `snap.hover`) — so hover and clicks always match the rows the menu
/// lights up. Applies the same sidebar-bottom cap, recent-strip cap, and
/// right-reserve break the draw uses.
pub(crate) fn menu_hover_at(
    state: &StartMenuState,
    reg: &AppCatalog,
    mouse: Point,
    ty: u32,
) -> Option<HoverTarget> {
    let menu_r = layout::menu_rect(ty);
    if !menu_r.hit_test(mouse) {
        return None;
    }

    // Sidebar categories (same sidebar-bottom cap as the draw).
    let sidebar_r = layout::menu_sidebar_rect(menu_r);
    for (i, _) in CATEGORIES.iter().enumerate() {
        let cat_r = layout::menu_category_rect(menu_r, i);
        if cat_r.y + cat_r.h as i32 > sidebar_r.y + sidebar_r.h as i32 {
            break;
        }
        if cat_r.hit_test(mouse) {
            return Some(HoverTarget::StartCategory(i));
        }
    }

    // App list (visible rows only, scroll-aware — same range as the draw).
    let list_r = layout::menu_list_rect(menu_r);
    let avail = (list_r.h / layout::MENU_ITEM_H) as usize;
    let start = state.scroll as usize;
    let end = (start + avail).min(state.filtered.len());
    for i in start..end {
        if layout::menu_item_rect(menu_r, i, start).hit_test(mouse) {
            return Some(HoverTarget::StartApp(i));
        }
    }

    // Recent strip (capped and right-reserve-broken exactly like the draw).
    let mut rx = layout::menu_recent_x0(menu_r);
    let recent_n = reg.recent.len().min(layout::MENU_RECENT_MAX);
    for ri in 0..recent_n {
        let idx = reg.recent[ri];
        if idx >= reg.apps.len() {
            continue;
        }
        if rx + layout::MENU_RECENT_PITCH as i32
            > menu_r.x + layout::MENU_W as i32 - layout::MENU_RECENT_RIGHT_RESERVE as i32
        {
            break;
        }
        if layout::menu_recent_rect(menu_r, rx).hit_test(mouse) {
            return Some(HoverTarget::StartRecent(ri));
        }
        rx += layout::MENU_RECENT_PITCH as i32;
    }

    // Power buttons.
    for (pi, _) in POWER_LABELS.iter().enumerate() {
        if layout::menu_power_rect(menu_r, pi).hit_test(mouse) {
            return Some(HoverTarget::StartPower(pi));
        }
    }

    None
}

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

    pub fn open_with(&mut self, reg: &AppCatalog) {
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

    /// Toggle the menu: close it if open, otherwise open fresh. The a11y
    /// Start-button activation uses this so the same action that opens the
    /// menu closes it — a keyboard user's expectation, and the same button
    /// the mouse clicks.
    pub fn toggle(&mut self, reg: &AppCatalog) {
        if self.open {
            self.open = false;
            self.anim_direction = -1;
        } else {
            self.open_with(reg);
        }
    }

    pub fn rebuild_filter(&mut self, reg: &AppCatalog) {
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
}

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
        layout::MENU_H - (layout::MENU_H * anim) / 255
    } else {
        0
    };
    let menu_x = layout::MENU_X;
    let menu_y = ty.saturating_sub(layout::MENU_H + layout::MENU_BOTTOM_GAP) + anim_offset;
    let menu_r = Rect::new(menu_x as i32, menu_y as i32, layout::MENU_W, layout::MENU_H);
    let search_r = layout::menu_search_rect(menu_r);
    let sidebar_r = layout::menu_sidebar_rect(menu_r);
    let list_r = layout::menu_list_rect(menu_r);
    let bottom_y = layout::menu_bottom_y(menu_r);

    canvas.draw_shadow(
        menu_x,
        menu_y,
        layout::MENU_W,
        layout::MENU_H,
        8,
        theme.shadow,
    );
    canvas.draw_rounded_rect(
        menu_x,
        menu_y,
        layout::MENU_W,
        layout::MENU_H,
        8,
        theme.bg_primary,
    );
    canvas.draw_rounded_rect_outline(
        menu_x,
        menu_y,
        layout::MENU_W,
        layout::MENU_H,
        8,
        theme.border,
    );

    canvas.draw_rounded_rect(
        search_r.x as u32,
        search_r.y as u32,
        search_r.w,
        search_r.h,
        6,
        theme.bg_surface,
    );
    if state.section == MenuSection::Search && state.keyboard_active {
        canvas.draw_rounded_rect_outline(
            search_r.x as u32,
            search_r.y as u32,
            search_r.w,
            search_r.h,
            6,
            theme.accent,
        );
    }
    let display = if state.search.is_empty() {
        ">  Search applications..."
    } else {
        core::str::from_utf8(&state.search).unwrap_or("")
    };
    canvas.draw_string(
        search_r.x as u32 + 6,
        search_r.y as u32 + 8,
        display,
        if state.search.is_empty() {
            theme.text_disabled
        } else {
            theme.text
        },
        0,
    );

    canvas.draw_rounded_rect(
        sidebar_r.x as u32,
        sidebar_r.y as u32,
        sidebar_r.w,
        sidebar_r.h,
        6,
        theme.bg_surface,
    );

    for (i, &(cat_name, _)) in CATEGORIES.iter().enumerate() {
        let cat_r = layout::menu_category_rect(menu_r, i);
        if cat_r.y + cat_r.h as i32 > sidebar_r.y + sidebar_r.h as i32 {
            break;
        }
        let selected = i == state.cat_idx;
        let hover = snap.hover == Some(HoverTarget::StartCategory(i));
        // Held-down rows darken like the taskbar buttons (hover+pressed
        // only) — visible here because a category click does not close the
        // menu, so the hold frame actually renders.
        let bg = if hover && snap.mouse_down {
            theme.pressed
        } else if selected {
            theme.accent
        } else if hover {
            theme.hover
        } else {
            theme.bg_surface
        };
        // Indigo fills (accent when selected, hover) carry white text —
        // theme.text flips to black in the light theme and would vanish on
        // them. Only the pressed fill (light gray in light mode) keeps
        // theme.text.
        let txt = if hover && snap.mouse_down {
            theme.text
        } else if selected || hover {
            theme.on_accent
        } else {
            theme.text_secondary
        };
        canvas.draw_rounded_rect(cat_r.x as u32, cat_r.y as u32, cat_r.w, cat_r.h, 4, bg);
        canvas.draw_string(cat_r.x as u32 + 6, cat_r.y as u32 + 5, cat_name, txt, 0);
    }

    let avail = (list_r.h / layout::MENU_ITEM_H) as usize;

    if !state.filtered.is_empty() {
        let start = state.scroll as usize;
        let end = (start + avail).min(state.filtered.len());
        for i in start..end {
            let app_id = state.filtered[i];
            let app = &reg.apps[app_id.0];
            let item_r = layout::menu_item_rect(menu_r, i, start);
            let sel = i == state.selected && state.section == MenuSection::AppList;
            let hover = snap.hover == Some(HoverTarget::StartApp(i));
            // The a11y ring on this row lights it — the keyboard user sees
            // where the ring is on the menu, and the focused row's
            // Enter-launch target is the row that looks lit. `snap.focused`
            // carries the same `StartApp(i)` target the hover does, so the
            // union is one equality per row.
            let focused = snap.focused == Some(HoverTarget::StartApp(i));
            // Held app rows darken while the button is down. The click
            // dispatches on press and `launch_app` closes the menu the same
            // frame, so this particular pressed frame is masked in
            // production — category/power rows (whose clicks keep the menu
            // open) show it live. The arm is the consistent contract
            // either way.
            // Keyboard focus is the accent_light blue, distinct from the
            // indigo hover — the same rule as the taskbar buttons, so
            // "blue = ring" holds on every navigation surface.
            let bg = if hover && snap.mouse_down {
                theme.pressed
            } else if sel {
                theme.accent
            } else if focused {
                theme.accent_light
            } else if hover {
                theme.hover
            } else {
                theme.bg_primary
            };
            canvas.draw_rounded_rect(item_r.x as u32, item_r.y as u32, item_r.w, item_r.h, 4, bg);
            let label = layout::trunc(app.name, layout::MENU_APP_NAME_MAX);
            // Same on_accent rule as the other rows: the accent (selected)
            // and hover fills carry white text.
            let txt = if hover && snap.mouse_down {
                theme.text
            } else if sel || hover || focused {
                theme.on_accent
            } else {
                theme.text_secondary
            };
            canvas.draw_char(item_r.x as u32 + 8, item_r.y as u32 + 7, app.icon, txt, 0);
            canvas.draw_string(item_r.x as u32 + 18, item_r.y as u32 + 7, label, txt, 0);
            if reg.pinned[app_id.0] {
                // The pin tag follows the row text: white on the selected
                // indigo fill (theme.warning yellow is 1.3:1 on it in both
                // themes), secondary gray otherwise (warning yellow is
                // 1.3:1 on the white light menu body).
                canvas.draw_string(
                    item_r.x as u32 + item_r.w - 50,
                    item_r.y as u32 + 7,
                    "[Pin]",
                    if sel || focused {
                        theme.on_accent
                    } else {
                        theme.text_secondary
                    },
                    0,
                );
            }
        }
    }

    canvas.draw_rect(
        menu_x + 2,
        bottom_y as u32,
        layout::MENU_W - 4,
        layout::MENU_BOTTOM_STRIP_HT,
        theme.bg_surface,
    );

    canvas.draw_string(
        menu_x + 12,
        bottom_y as u32 + 10,
        "Recent:",
        theme.text_disabled,
        0,
    );
    let mut rx = layout::menu_recent_x0(menu_r);
    let recent_n = reg.recent.len().min(layout::MENU_RECENT_MAX);
    for ri in 0..recent_n {
        let idx = reg.recent[ri];
        if idx >= reg.apps.len() {
            continue;
        }
        let r = layout::menu_recent_rect(menu_r, rx);
        if rx + layout::MENU_RECENT_PITCH as i32
            > menu_r.x + layout::MENU_W as i32 - layout::MENU_RECENT_RIGHT_RESERVE as i32
        {
            break;
        }
        let app = &reg.apps[idx];
        let label = layout::trunc(app.name, layout::MENU_RECENT_NAME_MAX);
        let hover = snap.hover == Some(HoverTarget::StartRecent(ri));
        let sel = state.section == MenuSection::Recent && ri == 0;
        let bg = if hover && snap.mouse_down {
            theme.pressed
        } else if sel {
            theme.accent
        } else if hover {
            theme.border
        } else {
            theme.bg_elevated
        };
        canvas.draw_rounded_rect(r.x as u32, r.y as u32, r.w, r.h, 4, bg);
        // Selected tile is indigo accent -> on_accent text (see category
        // arm); hover uses the light-gray border fill, which keeps the
        // secondary gray.
        canvas.draw_string(
            r.x as u32 + 6,
            r.y as u32 + 3,
            label,
            if hover && snap.mouse_down {
                theme.text
            } else if sel {
                theme.on_accent
            } else {
                theme.text_secondary
            },
            0,
        );
        rx += layout::MENU_RECENT_PITCH as i32;
    }

    for (pi, &(icon, label)) in POWER_LABELS.iter().enumerate() {
        let pr = layout::menu_power_rect(menu_r, pi);
        let hover = snap.hover == Some(HoverTarget::StartPower(pi));
        let sel = pi == state.power_idx && state.section == MenuSection::Power;
        // Power rows have no click action, so the menu stays open and the
        // pressed frame renders for the whole hold — the clearest pressed
        // case in the menu.
        let bg = if hover && snap.mouse_down {
            theme.pressed
        } else if sel {
            theme.accent
        } else if hover {
            theme.hover
        } else {
            theme.bg_elevated
        };
        canvas.draw_rounded_rect(pr.x as u32, pr.y as u32, pr.w, pr.h, 4, bg);
        // Indigo accent/hover fills carry white text (see category arm).
        let txt = if hover && snap.mouse_down {
            theme.text
        } else if sel || hover {
            theme.on_accent
        } else {
            theme.text_secondary
        };
        canvas.draw_char(pr.x as u32 + 6, pr.y as u32 + 5, icon, txt, 0);
        canvas.draw_string(pr.x as u32 + 16, pr.y as u32 + 5, label, txt, 0);
    }
}
