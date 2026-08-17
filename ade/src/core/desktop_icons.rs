//! Desktop icons — selection, multi-select, move, delete, rectangle selection.

use crate::core::geometry::{Point, Rect, RubberBand};
use crate::render::compositor::Canvas;
use alloc::string::String;
use alloc::vec::Vec;

pub(crate) struct DesktopIcon {
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub selected: bool,
}

pub(crate) struct DesktopIcons {
    pub icons: Vec<DesktopIcon>,
    pub drag_icon: bool,
    pub rubber: Option<RubberBand>,
}

impl DesktopIcons {
    pub fn new() -> Self {
        let mut icons = Vec::new();
        let entries: &[(&str, i32, i32)] = &[
            ("Terminal", 30, 80),
            ("Files", 30, 180),
            ("SkyStore", 30, 280),
            ("SkyEdit", 30, 380),
            ("Calc", 30, 480),
        ];
        for &(name, x, y) in entries {
            icons.push(DesktopIcon {
                name: String::from(name),
                x,
                y,
                selected: false,
            });
        }
        DesktopIcons {
            icons,
            drag_icon: false,
            rubber: None,
        }
    }

    pub fn icon_at(&self, mx: i32, my: i32) -> Option<usize> {
        for (i, ic) in self.icons.iter().enumerate().rev() {
            if Rect::new(ic.x, ic.y, 48, 56).hit_test(Point::new(mx, my)) {
                return Some(i);
            }
        }
        None
    }

    /// Toggle the selection of icon `idx`; a newly selected icon becomes
    /// drag-eligible.
    pub fn toggle_icon(&mut self, idx: usize) {
        let sel = !self.icons[idx].selected;
        self.icons[idx].selected = sel;
        if sel {
            self.drag_icon = true; // will move on drag
        }
    }

    /// Click on empty desktop space: clear all selections and start a rubber
    /// band at `(mx, my)`.
    pub fn click_empty(&mut self, mx: i32, my: i32) {
        for ic in &mut self.icons {
            ic.selected = false;
        }
        self.begin_select(mx, my);
    }

    pub fn begin_select(&mut self, mx: i32, my: i32) {
        self.rubber = Some(RubberBand::new(mx, my));
    }

    pub fn update_rubber(&mut self, mx: i32, my: i32) {
        if let Some(r) = self.rubber.as_mut() {
            r.drag_to(mx, my);
        }
    }

    pub fn end_select(&mut self) -> Vec<usize> {
        let mut selected = Vec::new();
        if let Some(rubber) = self.rubber {
            self.rubber = None;
            let rr = rubber.rect();
            if rr.w < 4 && rr.h < 4 {
                return selected;
            } // click, not drag
            for (i, ic) in self.icons.iter().enumerate() {
                if rr.intersects(&Rect::new(ic.x, ic.y, 48, 56)) {
                    selected.push(i);
                }
            }
        }
        selected
    }

    pub fn move_selected(&mut self, dx: i32, dy: i32) {
        for ic in &mut self.icons {
            if ic.selected {
                ic.x += dx;
                ic.y += dy;
            }
        }
    }
}

pub(crate) fn draw(
    canvas: &mut Canvas,
    icons: &[DesktopIcon],
    theme: &libsarga::theme::Theme,
    rubber: Option<RubberBand>,
) {
    for ic in icons {
        if ic.selected {
            canvas.draw_rounded_rect_outline(
                ic.x as u32 - 2,
                ic.y as u32 - 2,
                52,
                60,
                6,
                theme.accent,
            );
        }
        canvas.draw_rounded_rect(ic.x as u32 + 8, ic.y as u32 + 2, 32, 32, 8, theme.accent);
        canvas.draw_string(
            ic.x as u32 + 4,
            ic.y as u32 + 40,
            if ic.name.len() > 8 {
                &ic.name[..8]
            } else {
                &ic.name
            },
            theme.text,
            0,
        );
    }
    if let Some(r) = rubber {
        let rr = r.rect();
        let accent = theme.accent;
        let fill = (accent & 0x00FF_FFFF) | 0x22_000000;
        canvas.draw_rect_alpha(rr.x as u32, rr.y as u32, rr.w, rr.h, fill);
        canvas.draw_rect_outline(rr.x as u32, rr.y as u32, rr.w, rr.h, accent);
    }
}
