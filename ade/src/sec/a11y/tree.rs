use crate::sec::a11y::focus::FocusDirection;
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

    pub fn clear(&mut self) {
        self.nodes.clear();
        self.focused_id = None;
        self.next_id = 0;
    }

    pub fn add_node(
        &mut self,
        role: A11yRole,
        label: &str,
        bounds: (i32, i32, u32, u32),
        focusable: bool,
    ) -> u32 {
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
                enabled: true,
                selected: false,
                checked: None,
            },
            focusable,
            parent: None,
            children: Vec::new(),
        });
        id
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

    // keep: stub; FocusManager::move_focus is the live navigation path
    #[allow(dead_code)]
    pub fn move_focus(&mut self, _dir: FocusDirection) -> bool {
        false
    }

    // keep: getter reserved for a11y clients
    #[allow(dead_code)]
    pub fn focused_node(&self) -> Option<&A11yNode> {
        self.focused_id
            .and_then(|id| self.nodes.iter().find(|n| n.id == id))
    }

    pub fn node_at(&self, x: i32, y: i32) -> Option<&A11yNode> {
        self.nodes.iter().rev().find(|&n| {
            x >= n.bounds.0
                && x < n.bounds.0 + n.bounds.2 as i32
                && y >= n.bounds.1
                && y < n.bounds.1 + n.bounds.3 as i32
        })
    }

    // keep: lookup reserved for a11y clients
    #[allow(dead_code)]
    pub fn find_by_role(&self, role: A11yRole) -> Option<&A11yNode> {
        self.nodes.iter().find(|n| n.role == role)
    }
}
