//! Desktop icons — selection, multi-select, move, delete, rectangle selection.

use crate::geometry::{Point, Rect};
use alloc::string::String;
use alloc::vec::Vec;
use libsarga::gui::Window;

pub(crate) struct DesktopIcon {
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub selected: bool,
}

pub(crate) struct DesktopIcons {
    pub icons: Vec<DesktopIcon>,
    pub drag_icon: bool,
    pub rubber: Option<(i32, i32, i32, i32)>,
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

    pub fn begin_select(&mut self, mx: i32, my: i32) {
        self.rubber = Some((mx, my, mx, my));
    }

    pub fn update_rubber(&mut self, mx: i32, my: i32) {
        if let Some((ox, oy, _, _)) = self.rubber {
            self.rubber = Some((ox, oy, mx, my));
        }
    }

    pub fn end_select(&mut self) -> Vec<usize> {
        let mut selected = Vec::new();
        if let Some((x1, y1, x2, y2)) = self.rubber {
            self.rubber = None;
            let rx = x1.min(x2);
            let ry = y1.min(y2);
            let rw = (x1 - x2).abs() as u32;
            let rh = (y1 - y2).abs() as u32;
            if rw < 4 && rh < 4 {
                return selected;
            } // click, not drag
            let rr = Rect::new(rx, ry, rw, rh);
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
    win: &mut Window,
    icons: &[DesktopIcon],
    theme: &libsarga::theme::Theme,
    rubber: Option<(i32, i32, i32, i32)>,
) {
    for ic in icons {
        if ic.selected {
            win.draw_rounded_rect_outline(
                ic.x as u32 - 2,
                ic.y as u32 - 2,
                52,
                60,
                6,
                theme.accent,
            );
        }
        win.draw_rounded_rect(ic.x as u32 + 8, ic.y as u32 + 2, 32, 32, 8, 0xFF3D5AFE);
        win.draw_string(
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
    if let Some((x1, y1, x2, y2)) = rubber {
        let rx = x1.min(x2) as u32;
        let ry = y1.min(y2) as u32;
        let rw = (x1 - x2).abs() as u32;
        let rh = (y1 - y2).abs() as u32;
        win.draw_rect_alpha(rx, ry, rw, rh, 0x223D5AFE);
        win.draw_rect_outline(rx, ry, rw, rh, 0xFF3D5AFE);
    }
}
