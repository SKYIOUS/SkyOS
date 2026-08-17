use crate::core::window::{WindowId, START_BUTTON_OWNER, TRAY_PANEL_OWNER};
use crate::sec::a11y::node::{A11yNode, A11yRole};
use crate::sec::a11y::tree::A11yTree;
use alloc::vec::Vec;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FocusDirection {
    Up,
    Down,
    Left,
    Right,
    First,
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

    /// Re-sync focus after a window is closed: land on a *sibling* window's
    /// taskbar button (or its Window node) so the keyboard user lands on a
    /// window, not the empty taskbar — falling back to the first visible
    /// focusable node (Taskbar/Start) only when no other window remains.
    /// The closing window's nodes (its Window node, Close button, and
    /// taskbar button — all owner-stamped) remain in the tree until
    /// `process_closing` removes it, so they are excluded or the ring can
    /// land on a node that is about to vanish. Blurs if nothing focusable
    /// is left.
    pub fn resync_after_close(&mut self, tree: &A11yTree, closed_window: WindowId) {
        match preferred_target(tree, Some(closed_window)) {
            Some(id) => self.focus(id),
            None => self.blur(),
        }
    }

    /// Central focus-lifecycle safety net: validate the focused id against a
    /// freshly rebuilt tree and re-sync (or blur) if it no longer exists.
    ///
    /// Any close path that is NOT the a11y activation (mouse Close click,
    /// Ctrl+W, close_by_pid reap, session teardown) leaves `focused_id`
    /// pointing at a node the next `build_tree` no longer emits — the ring
    /// would silently die with no node to draw on. `build_tree` calls this
    /// in its focus-sync step every frame, so the ring is reparented to a
    /// sibling window's taskbar button / Window node when one remains
    /// (matching `resync_after_close`'s landing), or to the first visible
    /// focusable node, or blurred — no matter how the window went away.
    /// Unlike `resync_after_close`, no owner exclusion is needed: the
    /// closed window's nodes are simply gone from the rebuilt tree.
    /// `prev_fp` is the fingerprint of the node the focused id denoted in
    /// the PREVIOUS tree (captured by `build_tree` before rebuilding), or
    /// `None` when nothing was focused / no node matched. With a
    /// fingerprint, an id that survives the rebuild but now names a
    /// DIFFERENT node is detected and re-synced; without one (unit
    /// callers), the legacy id-existence check applies.
    pub fn validate(&mut self, new: &A11yTree, prev_fp: Option<NodeFp>) {
        let Some(cur) = self.focused_id else {
            return; // nothing focused — nothing to protect
        };
        let re_sync = |this: &mut Self| match preferred_target(new, None) {
            Some(id) => this.focus(id),
            None => this.blur(),
        };
        if let Some(prev) = prev_fp {
            // Identity check. Node ids are POSITIONAL — assigned in rebuild
            // order each frame — so when a window closes (or the window
            // order changes), a surviving id can silently name a different
            // node: the ring would park on an arbitrary-but-valid surface
            // instead of the intended sibling landing. The (owner, role,
            // parent-role) fingerprint is stable for a node across rebuilds
            // (a node never changes owner, role, or parent role), so a
            // mismatch means the id changed meaning -> stale. Fall through
            // to the focusable/visible check when the id still denotes the
            // same node.
            let new_fp = new
                .nodes
                .iter()
                .find(|n| n.id == cur)
                .map(|n| node_fingerprint(new, n));
            if new_fp != Some(prev) {
                re_sync(self);
                return;
            }
        }
        // Same node (or no previous fingerprint to compare): keep focus only
        // when the node is still focusable and visible — the criteria
        // `move_focus`/`resync_after_close` use, so a future builder that
        // emits a non-focusable node can't leave the ring parked on it.
        let alive = new
            .nodes
            .iter()
            .any(|n| n.id == cur && n.focusable && n.state.visible);
        if !alive {
            re_sync(self);
        }
    }

    pub fn move_focus(&mut self, dir: FocusDirection, tree: &A11yTree) -> bool {
        match dir {
            FocusDirection::First => {
                for n in &tree.nodes {
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
                let cx = cur_bounds.x + (cur_bounds.w as i32) / 2;
                let cy = cur_bounds.y + (cur_bounds.h as i32) / 2;
                let mut best = None;
                let mut best_dist = i32::MAX;
                for n in &tree.nodes {
                    if !n.focusable || !n.state.visible || n.id == cur {
                        continue;
                    }
                    let nx = n.bounds.x + (n.bounds.w as i32) / 2;
                    let ny = n.bounds.y + (n.bounds.h as i32) / 2;
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

/// Stable identity of a tree node across rebuilds: the owner stamp, the
/// role, and the parent's role. Node ids are positional (tree rebuild
/// order), so an id that survives a rebuild can silently name a different
/// node — `FocusManager::validate` compares fingerprints to detect that.
/// `parent_role` is derived from the tree (the node stores only the parent
/// id), which is why the fingerprint is computed against a tree, not from
/// the node alone.
pub(crate) fn node_fingerprint(tree: &A11yTree, n: &A11yNode) -> NodeFp {
    let parent_role = n
        .parent
        .and_then(|pid| tree.nodes.iter().find(|p| p.id == pid).map(|p| p.role));
    NodeFp {
        owner: n.owner,
        role: n.role,
        parent_role,
    }
}

/// The fingerprint payload — see [`node_fingerprint`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct NodeFp {
    pub owner: Option<WindowId>,
    pub role: A11yRole,
    pub parent_role: Option<A11yRole>,
}

/// Pick where a stale focused id should land after its window went away.
///
/// Prefers a *sibling* window: its taskbar button (a Button child of the
/// Taskbar node) or its Window node — so the keyboard user lands on a real
/// window instead of the empty taskbar. A sibling's Close control (a Button
/// child of the Window node) is deliberately NOT a landing target. The
/// Start button is also a Taskbar child, but its sentinel owner marks it as
/// a launcher control, not a window — it is excluded from Pass 1 and only
/// reachable via the Pass 2 fallback. The tray panel's sentinel is excluded
/// on the same rule (a status surface, never a window landing target), so
/// the exclusion contract covers every sentinel-stamped node, not just the
/// one that happens to be non-focusable today. Falls back to the first
/// visible focusable node (Taskbar/Start) only when no sibling window
/// remains.
/// `exclude` is the just-closed window's id when its nodes may still linger
/// in the tree (close animation); `None` from the central `validate`, where
/// the closed window's nodes are already gone. Returns the node id, or
/// `None` when nothing focusable remains.
fn preferred_target(tree: &A11yTree, exclude: Option<WindowId>) -> Option<u32> {
    // Pass 1: a sibling window's taskbar button or Window node.
    for n in &tree.nodes {
        if !n.focusable || !n.state.visible {
            continue;
        }
        let Some(owner) = n.owner else {
            continue; // no owner -> not a window surface
        };
        if exclude == Some(owner) || owner == START_BUTTON_OWNER || owner == TRAY_PANEL_OWNER {
            continue; // the closing window's surfaces, and the launcher/tray sentinels
        }
        match n.role {
            A11yRole::Window => return Some(n.id),
            A11yRole::Button => {
                // Taskbar buttons are Button children of the Taskbar node;
                // a window's Close control is a Button child of the Window
                // node and must not be a landing target.
                let parent_is_taskbar = n
                    .parent
                    .and_then(|pid| tree.nodes.iter().find(|p| p.id == pid).map(|p| p.role))
                    == Some(A11yRole::Taskbar);
                if parent_is_taskbar {
                    return Some(n.id);
                }
            }
            _ => {}
        }
    }
    // Pass 2: any remaining visible focusable node (Taskbar, Start button,
    // icons…) — only reached when no sibling window is left.
    for n in &tree.nodes {
        if !n.focusable || !n.state.visible {
            continue;
        }
        if exclude.is_some_and(|ex| n.owner == Some(ex)) {
            continue;
        }
        return Some(n.id);
    }
    None
}
