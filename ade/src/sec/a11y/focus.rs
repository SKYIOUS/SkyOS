use crate::sec::a11y::tree::A11yTree;
use alloc::vec::Vec;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
// keep: Next/Prev/Last reserved for tab-cycle and end-key focus
#[allow(dead_code)]
pub(crate) enum FocusDirection {
    Next,
    Prev,
    Up,
    Down,
    Left,
    Right,
    First,
    Last,
}

pub(crate) struct FocusManager {
    pub focused_id: Option<u32>,
    pub focus_history: Vec<u32>,
}

impl FocusManager {
    pub fn new() -> Self {
        FocusManager {
            focused_id: None,
            focus_history: Vec::new(),
        }
    }

    pub fn focus(&mut self, id: u32) {
        self.focused_id = Some(id);
        self.focus_history.retain(|&i| i != id);
        self.focus_history.push(id);
        if self.focus_history.len() > 20 {
            self.focus_history.remove(0);
        }
    }

    pub fn blur(&mut self) {
        self.focused_id = None;
    }

    pub fn focused(&self) -> Option<u32> {
        self.focused_id
    }

    pub fn move_focus(&mut self, dir: FocusDirection, tree: &A11yTree) -> bool {
        match dir {
            FocusDirection::Next => {
                let start = self.focused_id.map_or(0, |id| {
                    tree.nodes
                        .iter()
                        .position(|n| n.id == id)
                        .map_or(0, |i| i + 1)
                });
                for i in start..tree.nodes.len() {
                    if tree.nodes[i].focusable && tree.nodes[i].state.visible {
                        self.focus(tree.nodes[i].id);
                        return true;
                    }
                }
                // wrap to first
                for i in 0..start.min(tree.nodes.len()) {
                    if tree.nodes[i].focusable && tree.nodes[i].state.visible {
                        self.focus(tree.nodes[i].id);
                        return true;
                    }
                }
                false
            }
            FocusDirection::Prev => {
                let start = self.focused_id.map_or(0, |id| {
                    tree.nodes.iter().position(|n| n.id == id).map_or(0, |i| i)
                });
                for i in (0..start).rev() {
                    if tree.nodes[i].focusable && tree.nodes[i].state.visible {
                        self.focus(tree.nodes[i].id);
                        return true;
                    }
                }
                // wrap to last
                for i in (start..tree.nodes.len()).rev() {
                    if tree.nodes[i].focusable && tree.nodes[i].state.visible {
                        self.focus(tree.nodes[i].id);
                        return true;
                    }
                }
                false
            }
            FocusDirection::First => {
                for n in &tree.nodes {
                    if n.focusable && n.state.visible {
                        self.focus(n.id);
                        return true;
                    }
                }
                false
            }
            FocusDirection::Last => {
                for n in tree.nodes.iter().rev() {
                    if n.focusable && n.state.visible {
                        self.focus(n.id);
                        return true;
                    }
                }
                false
            }
            FocusDirection::Left
            | FocusDirection::Right
            | FocusDirection::Up
            | FocusDirection::Down => {
                let cur = match self.focused_id {
                    Some(id) => id,
                    None => return self.move_focus(FocusDirection::First, tree),
                };
                let cur_bounds = match tree.nodes.iter().find(|n| n.id == cur) {
                    Some(n) => n.bounds,
                    None => return false,
                };
                let cx = cur_bounds.0 + (cur_bounds.2 as i32) / 2;
                let cy = cur_bounds.1 + (cur_bounds.3 as i32) / 2;
                let mut best = None;
                let mut best_dist = i32::MAX;
                for n in &tree.nodes {
                    if !n.focusable || !n.state.visible || n.id == cur {
                        continue;
                    }
                    let nx = n.bounds.0 + (n.bounds.2 as i32) / 2;
                    let ny = n.bounds.1 + (n.bounds.3 as i32) / 2;
                    let dx = nx - cx;
                    let dy = ny - cy;
                    let passes = match dir {
                        FocusDirection::Left => dy.abs() * 2 < (nx - cx).abs() && dx < 0,
                        FocusDirection::Right => dy.abs() * 2 < (nx - cx).abs() && dx > 0,
                        FocusDirection::Up => dx.abs() * 2 < (ny - cy).abs() && dy < 0,
                        FocusDirection::Down => dx.abs() * 2 < (ny - cy).abs() && dy > 0,
                        _ => false,
                    };
                    if passes {
                        let dist = dx.abs() + dy.abs();
                        if dist < best_dist {
                            best_dist = dist;
                            best = Some(n.id);
                        }
                    }
                }
                match best {
                    Some(id) => {
                        self.focus(id);
                        true
                    }
                    None => false,
                }
            }
        }
    }
}
