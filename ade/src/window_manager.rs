use alloc::vec::Vec;

use crate::window::{AppWindow, WindowState};

pub struct WindowManager {
    windows: Vec<AppWindow>,
    focused: Option<usize>,
    dragging: Option<usize>,
}

impl WindowManager {
    pub fn new() -> Self {
        Self {
            windows: Vec::new(),
            focused: None,
            dragging: None,
        }
    }

    pub fn windows(&self) -> &[AppWindow] {
        &self.windows
    }

    pub fn windows_mut(&mut self) -> &mut Vec<AppWindow> {
        &mut self.windows
    }

    pub fn push(&mut self, window: AppWindow) {
        self.windows.push(window);
        self.focused = Some(self.windows.len() - 1);
    }

    pub fn len(&self) -> usize {
        self.windows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.windows.is_empty()
    }

    pub fn focused(&self) -> Option<usize> {
        self.focused
    }

    pub fn set_focus(&mut self, index: usize) {
        if index >= self.windows.len() {
            return;
        }

        for w in &mut self.windows {
            w.focused = false;
        }

        self.windows[index].focused = true;
        self.focused = Some(index);
    }

    pub fn bring_to_front(&mut self, index: usize) {
        if index >= self.windows.len() {
            return;
        }

        let mut w = self.windows.remove(index);
        w.focused = true;

        for other in &mut self.windows {
            other.focused = false;
        }

        self.windows.push(w);
        self.focused = Some(self.windows.len() - 1);
    }

    pub fn minimize(&mut self, index: usize) {
        if let Some(w) = self.windows.get_mut(index) {
            w.state = WindowState::Minimized;
        }
    }

    pub fn restore(&mut self, index: usize) {
        if let Some(w) = self.windows.get_mut(index) {
            w.state = WindowState::Normal;
        }
    }

    pub fn close(&mut self, index: usize) {
        if index < self.windows.len() {
            self.windows.remove(index);
        }

        if self.windows.is_empty() {
            self.focused = None;
        }
    }

    pub fn begin_drag(&mut self, index: usize, mx: i32, my: i32) {
        if let Some(w) = self.windows.get_mut(index) {
            w.dragging = true;
            w.drag_ox = mx - w.x;
            w.drag_oy = my - w.y;
            self.dragging = Some(index);
        }
    }

    pub fn update_drag(&mut self, mx: i32, my: i32) {
        if let Some(i) = self.dragging {
            if let Some(w) = self.windows.get_mut(i) {
                w.x = mx - w.drag_ox;
                w.y = my - w.drag_oy;
            }
        }
    }

    pub fn end_drag(&mut self) {
        if let Some(i) = self.dragging {
            if let Some(w) = self.windows.get_mut(i) {
                w.dragging = false;
            }
        }

        self.dragging = None;
    }
}

pub enum ResizeEdge {
    Left,
    Right,
    Top,
    Bottom,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

pub enum WindowAction {
    Close,
    Minimize,
    Maximize,
    Restore,
}

pub struct Workspace<WindowId> {
    windows: Vec<WindowId>,
}

