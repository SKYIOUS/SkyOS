//! Modern start menu — categories, search, pinned, keyboard navigation.

use crate::app_db::{AppCategory, AppDb, APPS, CATEGORIES};
use crate::geometry::{Point, Rect};
use crate::render::compositor::Canvas;
use crate::render::snapshot::RenderSnapshot;
use alloc::vec::Vec;

pub(crate) struct StartMenuState {
    pub open: bool,
    pub search: Vec<u8>,
    pub cat_idx: usize,
    pub selected: usize,
    pub scroll: u32,
    pub filtered: Vec<usize>,
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
        }
    }

    pub fn open_with(&mut self, db: &AppDb) {
        self.open = true;
        self.search.clear();
        self.cat_idx = 0;
        self.selected = 0;
        self.scroll = 0;
        self.rebuild_filter(db);
    }

    pub fn rebuild_filter(&mut self, db: &AppDb) {
        let cat = if self.cat_idx < CATEGORIES.len() {
            CATEGORIES[self.cat_idx].1
        } else {
            AppCategory::All
        };
        self.filtered = db.filtered(cat, &self.search);
        if self.filtered.is_empty() {
            self.selected = 0;
        } else if self.selected >= self.filtered.len() {
            self.selected = self.filtered.len() - 1;
        }
    }

    pub fn selected_app(&self) -> Option<usize> {
        if self.selected < self.filtered.len() {
            Some(self.filtered[self.selected])
        } else {
            None
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
    let db = match snap.app_db {
        Some(d) => d,
        None => return,
    };
    if !state.open {
        return;
    }

    let ty = snap.taskbar_y();
    let theme = snap.theme;
    let menu_x = 4u32;
    let menu_y = ty.saturating_sub(MENU_H + 4);

    // backdrop
    canvas.draw_rounded_rect(menu_x, menu_y, MENU_W, MENU_H, 8, 0xFF1E1E2E);
    canvas.draw_rounded_rect_outline(menu_x, menu_y, MENU_W, MENU_H, 8, 0xFF3A3A5C);

    // search bar
    let search_y = menu_y + 8;
    canvas.draw_rounded_rect(menu_x + 8, search_y, MENU_W - 16, SEARCH_H, 6, 0xFF2D2D40);
    let display = if state.search.is_empty() {
        "  Search applications..."
    } else {
        core::str::from_utf8(&state.search).unwrap_or("")
    };
    canvas.draw_string(
        menu_x + 14,
        search_y + 8,
        display,
        if state.search.is_empty() {
            0xFF606060
        } else {
            0xFFFFFFFF
        },
        0,
    );

    // sidebar: categories
    let sidebar_x = menu_x + 4;
    let sidebar_y = search_y + SEARCH_H + 6;
    let sidebar_h = MENU_H - (sidebar_y - menu_y) - 8;
    canvas.draw_rounded_rect(sidebar_x, sidebar_y, SIDEBAR_W, sidebar_h, 6, 0xFF252535);

    for (i, &(cat_name, _)) in CATEGORIES.iter().enumerate() {
        let iy = sidebar_y + 4 + i as u32 * 28;
        if iy + 24 > sidebar_y + sidebar_h {
            break;
        }
        let selected = i == state.cat_idx;
        let bg = if selected { theme.accent } else { 0xFF252535 };
        let txt = if selected { 0xFFFFFFFF } else { 0xFFB0B0B0 };
        canvas.draw_rounded_rect(sidebar_x + 4, iy, SIDEBAR_W - 8, 24, 4, bg);
        canvas.draw_string(sidebar_x + 10, iy + 5, cat_name, txt, 0);
    }

    // app list
    let list_x = sidebar_x + SIDEBAR_W + 4;
    let list_y = search_y + SEARCH_H + 6;
    let list_w = MENU_W - 4 - (list_x - menu_x);
    let list_h = MENU_H - (list_y - menu_y) - 44;
    let avail = (list_h / ITEM_H) as usize;

    if !state.filtered.is_empty() {
        let start = state.scroll as usize;
        let end = (start + avail).min(state.filtered.len());
        for i in start..end {
            let app_idx = state.filtered[i];
            let app = &APPS[app_idx];
            let iy = list_y + 2 + (i - start) as u32 * ITEM_H;
            let sel = i == state.selected;
            let hover =
                Rect::new(list_x as i32, iy as i32, list_w, ITEM_H - 2).hit_test(snap.mouse);
            let bg = if sel {
                0xFF3D5AFE
            } else if hover {
                0xFF333348
            } else {
                0xFF1E1E2E
            };
            canvas.draw_rounded_rect(list_x, iy, list_w, ITEM_H - 2, 4, bg);
            let label = if app.name.len() > 28 {
                &app.name[..28]
            } else {
                app.name
            };
            canvas.draw_string(
                list_x + 8,
                iy + 7,
                label,
                if sel { 0xFFFFFFFF } else { 0xFFD0D0D0 },
                0,
            );
            if db.pinned[app_idx] {
                let pin_label = if app.name.len() > 26 {
                    &app.name[..26]
                } else {
                    app.name
                };
                canvas.draw_string(
                    list_x + 8,
                    iy + 7,
                    pin_label,
                    if sel { 0xFFFFFFFF } else { 0xFFD0D0D0 },
                    0,
                );
                canvas.draw_string(list_x + list_w - 50, iy + 7, "[Pin]", 0xFFFFAA00, 0);
            }
        }
    }

    // bottom: recent strip
    let bottom_y = menu_y + MENU_H - 36;
    canvas.draw_rect(menu_x + 2, bottom_y, MENU_W - 4, 34, 0xFF252540);
    canvas.draw_string(menu_x + 12, bottom_y + 10, "Recent:", 0xFF888888, 0);
    let mut rx = menu_x + 72;
    for &idx in db.recent.iter() {
        if rx > menu_x + MENU_W - 20 {
            break;
        }
        let app = &APPS[idx];
        let label = if app.name.len() > 12 {
            &app.name[..12]
        } else {
            app.name
        };
        let hover = Rect::new(rx as i32, bottom_y as i32 + 2, 80, 30)
            .hit_test(Point::new(snap.mouse.x, snap.mouse.y));
        canvas.draw_rounded_rect(
            rx,
            bottom_y + 4,
            80,
            26,
            4,
            if hover { 0xFF3A3A5C } else { 0xFF2D2D40 },
        );
        canvas.draw_string(rx + 6, bottom_y + 7, label, 0xFFD0D0D0, 0);
        rx += 84;
    }
}
