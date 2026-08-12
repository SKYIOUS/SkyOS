use crate::core::geometry::Rect;
use crate::core::window::WindowId;
use crate::sec::a11y::node::{A11yNode, A11yRole, A11yState};
use alloc::vec::Vec;

pub(crate) struct A11yTree {
    pub nodes: Vec<A11yNode>,
    pub focused_id: Option<u32>,
    next_id: u32,
}

impl A11yTree {
    pub fn new() -> Self {
        A11yTree {
            nodes: Vec::with_capacity(64),
            focused_id: None,
            next_id: 0,
        }
    }

    pub fn add_node(&mut self, role: A11yRole, label: &str, bounds: Rect, focusable: bool) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        self.nodes.push(A11yNode {
            id,
            role,
            label: label.into(),
            bounds,
            state: A11yState {
                focused: false,
                visible: true,
            },
            focusable,
            parent: None,
            children: Vec::new(),
            owner: None,
        });
        id
    }

    /// Stamp the owning window onto a node (window nodes and their control
    /// buttons), so activation can route back to the real window.
    pub fn set_owner(&mut self, id: u32, owner: WindowId) {
        if let Some(n) = self.nodes.iter_mut().find(|n| n.id == id) {
            n.owner = Some(owner);
        }
    }

    pub fn set_parent(&mut self, child: u32, parent: u32) {
        if let Some(n) = self.nodes.iter_mut().find(|n| n.id == child) {
            n.parent = Some(parent);
        }
    }

    pub fn add_child(&mut self, parent: u32, child: u32) {
        if let Some(n) = self.nodes.iter_mut().find(|n| n.id == parent) {
            n.children.push(child);
        }
        self.set_parent(child, parent);
    }

    pub fn set_focus(&mut self, id: u32) {
        self.focused_id = Some(id);
        for n in self.nodes.iter_mut() {
            n.state.focused = n.id == id;
        }
    }

    pub fn node_at(&self, x: i32, y: i32) -> Option<&A11yNode> {
        let p = crate::core::geometry::Point::new(x, y);
        self.nodes.iter().rev().find(|&n| n.bounds.hit_test(p))
    }
}
