//! Window manager — ordered window list, focus, drag, minimize, close.

use crate::core::constants::SNAP_MARGIN;
use crate::core::window::{AppWindow, WindowId, WindowState};
use alloc::vec::Vec;

pub(crate) enum SnapRegion {
    Left,
    Right,
    Top,
    Bottom,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

pub(crate) struct SnapPreview {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
    pub active: bool,
}

// WindowManager API v1.0 — STABLE
pub struct WindowManager {
    windows: Vec<AppWindow>,
    focused: Option<usize>,
    dragging: Option<usize>,
    pub(crate) snap_preview: Option<SnapPreview>,
}

impl WindowManager {
    pub fn new() -> Self {
        Self {
            windows: Vec::new(),
            focused: None,
            dragging: None,
            snap_preview: None,
        }
    }

    /// WindowManager API v1.0
    pub fn create(&mut self, window: AppWindow) -> WindowId {
        self.windows.push(window);
        let id = WindowId(self.windows.len() - 1);
        self.focused = Some(id.0);
        id
    }

    /// WindowManager API v1.0
    pub fn close(&mut self, id: WindowId) {
        if let Some(w) = self.windows.get_mut(id.0) {
            if !w.closing {
                w.closing = true;
                w.animate_close();
            }
        }
    }

    pub fn close_by_pid(&mut self, pid: u64) {
        if let Some(pos) = self.windows.iter().position(|w| w.pid == Some(pid)) {
            self.windows.remove(pos);
            if self.windows.is_empty() {
                self.focused = None;
            }
        }
    }

    /// Remove windows whose close animation has completed.
    pub fn process_closing(&mut self) -> Vec<WindowId> {
        let mut closed = Vec::new();
        let mut i = self.windows.len();
        while i > 0 {
            i -= 1;
            if self.windows[i].closing && self.windows[i].anim.is_none() {
                closed.push(WindowId(i));
                self.windows.remove(i);
            }
        }
        if !closed.is_empty() && self.windows.is_empty() {
            self.focused = None;
        }
        closed
    }

    #[allow(dead_code)]
    pub fn focus(&mut self, id: WindowId) {
        if id.0 >= self.windows.len() {
            return;
        }
        for w in &mut self.windows {
            w.focused = false;
        }
        self.windows[id.0].focused = true;
        self.focused = Some(id.0);
    }

    /// WindowManager API v1.0
    pub fn bring_to_front(&mut self, id: WindowId) {
        if id.0 >= self.windows.len() {
            return;
        }
        let mut w = self.windows.remove(id.0);
        w.focused = true;
        for other in &mut self.windows {
            other.focused = false;
        }
        self.windows.push(w);
        self.focused = Some(self.windows.len() - 1);
    }

    /// WindowManager API v1.0
    pub fn minimize(&mut self, id: WindowId, screen_w: u32, taskbar_h: u32) {
        if let Some(w) = self.windows.get_mut(id.0) {
            w.prev_state = w.state;
            w.state = WindowState::Minimized;
            let tab_x = 75 + id.0 as u32 * 125;
            w.animate_to(tab_x as i32, (taskbar_h - 28) as i32, 120, 28);
        }
    }

    #[allow(dead_code)]
    pub fn maximize(&mut self, id: WindowId, screen_w: u32, taskbar_h: u32) {
        if let Some(w) = self.windows.get_mut(id.0) {
            w.prev_x = w.x;
            w.prev_y = w.y;
            w.prev_w = w.w;
            w.prev_h = w.h;
            w.animate_to(0, 0, screen_w, taskbar_h);
            w.state = WindowState::Maximized;
        }
    }

    /// WindowManager API v1.0
    pub fn toggle_maximize(&mut self, id: WindowId, screen_w: u32, taskbar_h: u32) {
        if let Some(w) = self.windows.get_mut(id.0) {
            match w.state {
                WindowState::Maximized => {
                    w.animate_to(w.prev_x, w.prev_y, w.prev_w, w.prev_h);
                    w.state = WindowState::Normal;
                }
                _ => {
                    w.prev_x = w.x;
                    w.prev_y = w.y;
                    w.prev_w = w.w;
                    w.prev_h = w.h;
                    w.animate_to(0, 0, screen_w, taskbar_h);
                    w.state = WindowState::Maximized;
                }
            }
        }
    }

    /// WindowManager API v1.0
    pub fn toggle_fullscreen(&mut self, id: WindowId, screen_w: u32, screen_h: u32) {
        if let Some(w) = self.windows.get_mut(id.0) {
            match w.state {
                WindowState::Fullscreen => {
                    w.animate_to(w.prev_x, w.prev_y, w.prev_w, w.prev_h);
                    w.state = WindowState::Normal;
                }
                _ => {
                    w.prev_x = w.x;
                    w.prev_y = w.y;
                    w.prev_w = w.w;
                    w.prev_h = w.h;
                    w.animate_to(0, 0, screen_w, screen_h);
                    w.state = WindowState::Fullscreen;
                }
            }
        }
    }

    pub fn snap_to_region(
        &mut self,
        id: WindowId,
        region: SnapRegion,
        sw: u32,
        _sh: u32,
        tb_h: u32,
    ) {
        if let Some(w) = self.windows.get_mut(id.0) {
            let half_w = sw / 2;
            let half_h = tb_h / 2;
            let (tx, ty, tw, th) = match region {
                SnapRegion::Left => (0, 0, half_w, tb_h),
                SnapRegion::Right => (half_w as i32, 0, half_w, tb_h),
                SnapRegion::Top => (0, 0, sw, half_h),
                SnapRegion::Bottom => (0, half_h as i32, sw, half_h),
                SnapRegion::TopLeft => (0, 0, half_w, half_h),
                SnapRegion::TopRight => (half_w as i32, 0, half_w, half_h),
                SnapRegion::BottomLeft => (0, half_h as i32, half_w, half_h),
                SnapRegion::BottomRight => (half_w as i32, half_h as i32, half_w, half_h),
            };
            self.snap_preview = Some(SnapPreview { x: tx, y: ty, w: tw, h: th, active: true });
            w.prev_x = w.x;
            w.prev_y = w.y;
            w.prev_w = w.w;
            w.prev_h = w.h;
            w.animate_to(tx, ty, tw, th);
            if w.state == WindowState::Maximized || w.state == WindowState::Fullscreen {
                w.state = WindowState::Normal;
            }
        }
    }

    pub fn show_snap_preview(&mut self, mx: i32, my: i32, sw: u32, _sh: u32, tb_h: u32) {
        let swi = sw as i32;
        let tb_hi = tb_h as i32;
        let half_w = sw / 2;
        let half_h = tb_h / 2;
        if mx < SNAP_MARGIN && my < SNAP_MARGIN {
            self.snap_preview = Some(SnapPreview { x: 0, y: 0, w: half_w, h: half_h, active: true });
        } else if mx > swi - SNAP_MARGIN && my < SNAP_MARGIN {
            self.snap_preview = Some(SnapPreview { x: half_w as i32, y: 0, w: half_w, h: half_h, active: true });
        } else if mx < SNAP_MARGIN && my > tb_hi - SNAP_MARGIN {
            self.snap_preview = Some(SnapPreview { x: 0, y: half_h as i32, w: half_w, h: half_h, active: true });
        } else if mx > swi - SNAP_MARGIN && my > tb_hi - SNAP_MARGIN {
            self.snap_preview = Some(SnapPreview { x: half_w as i32, y: half_h as i32, w: half_w, h: half_h, active: true });
        } else if mx < SNAP_MARGIN {
            self.snap_preview = Some(SnapPreview { x: 0, y: 0, w: half_w, h: tb_h, active: true });
        } else if mx > swi - SNAP_MARGIN {
            self.snap_preview = Some(SnapPreview { x: half_w as i32, y: 0, w: half_w, h: tb_h, active: true });
        } else if my < SNAP_MARGIN {
            self.snap_preview = Some(SnapPreview { x: 0, y: 0, w: sw, h: half_h, active: true });
        } else if my > tb_hi - SNAP_MARGIN {
            self.snap_preview = Some(SnapPreview { x: 0, y: half_h as i32, w: sw, h: half_h, active: true });
        } else {
            self.snap_preview = None;
        }
    }

    pub fn clear_snap_preview(&mut self) {
        self.snap_preview = None;
    }

    /// WindowManager API v1.0
    pub fn restore(&mut self, id: WindowId) {
        if let Some(w) = self.windows.get_mut(id.0) {
            let target = w.prev_state;
            if target == WindowState::Maximized {
                w.state = WindowState::Maximized;
            } else if target == WindowState::Fullscreen {
                w.state = WindowState::Fullscreen;
            } else {
                w.state = WindowState::Normal;
            }
        }
    }

    /// WindowManager API v1.0
    pub fn begin_drag(&mut self, id: WindowId, mx: i32, my: i32) {
        if let Some(w) = self.windows.get_mut(id.0) {
            w.anim = None;
            w.dragging = true;
            w.drag_ox = mx - w.x;
            w.drag_oy = my - w.y;
            self.dragging = Some(id.0);
        }
    }

    /// WindowManager API v1.0
    pub fn update_drag(&mut self, mx: i32, my: i32) {
        if let Some(i) = self.dragging {
            if let Some(w) = self.windows.get_mut(i) {
                w.x = mx - w.drag_ox;
                w.y = my - w.drag_oy;
            }
        }
    }

    /// WindowManager API v1.0
    pub fn end_drag(&mut self) {
        if let Some(i) = self.dragging {
            if let Some(w) = self.windows.get_mut(i) {
                w.dragging = false;
            }
        }
        self.dragging = None;
    }

    /// WindowManager API v1.0
    pub fn iter(&self) -> &[AppWindow] {
        &self.windows
    }

    pub fn iter_mut(&mut self) -> &mut [AppWindow] {
        &mut self.windows
    }

    /// WindowManager API v1.0
    pub fn lookup(&self, id: WindowId) -> Option<&AppWindow> {
        self.windows.get(id.0)
    }

    /// WindowManager API v1.0
    pub fn lookup_mut(&mut self, id: WindowId) -> Option<&mut AppWindow> {
        self.windows.get_mut(id.0)
    }

    /// WindowManager API v1.0
    pub fn active(&self) -> Option<WindowId> {
        self.focused.map(WindowId)
    }

    pub fn len(&self) -> usize {
        self.windows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.windows.is_empty()
    }

    pub fn focused_mut(&mut self) -> Option<&mut AppWindow> {
        self.focused.and_then(|i| self.windows.get_mut(i))
    }

    pub fn focus_next(&mut self) -> bool {
        if self.windows.is_empty() { return false; }
        for w in &mut self.windows { w.focused = false; }
        match self.focused {
            Some(idx) => {
                let n = (idx + 1) % self.windows.len();
                self.windows[n].focused = true;
                self.focused = Some(n);
            }
            None => {
                self.windows[0].focused = true;
                self.focused = Some(0);
            }
        }
        true
    }

    pub fn focus_prev(&mut self) -> bool {
        if self.windows.is_empty() { return false; }
        for w in &mut self.windows { w.focused = false; }
        match self.focused {
            Some(idx) => {
                let p = if idx == 0 { self.windows.len() - 1 } else { idx - 1 };
                self.windows[p].focused = true;
                self.focused = Some(p);
            }
            None => {
                let p = self.windows.len() - 1;
                self.windows[p].focused = true;
                self.focused = Some(p);
            }
        }
        true
    }
}
