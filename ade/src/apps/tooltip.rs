//! Tooltip manager — one visible tooltip at a time, owned by whichever
//! surface showed it (the pointer hovering an a11y node, or keyboard focus
//! on one). Only the owner may dismiss it: a pointer that moves off a node
//! hides the pointer's tooltip, never one shown for a focused node (or a
//! different hovered node). Dismissal is a fade-out over a few ticks
//! (delayed), so a brief move off and back onto the same node cancels it
//! instead of flickering. While the pointer stays on the owning node the
//! tooltip is kept alive, so it never expires mid-hover.
//!
//! Owner ids come from the a11y tree, which is rebuilt every frame with a
//! fresh id counter — ids are stable only while the tree shape is unchanged
//! (a window created/closed, the start menu opened, a clipboard panel
//! appearing all shift subsequent ids). A stale owner id degrades to the
//! timeout→fade-out fallback: the tooltip fades out at its timeout instead
//! of being dismissed on leave. Bounded, never stuck — but the keep-alive
//! guarantee is shape-dependent.

use alloc::string::String;

/// Who owns the visible tooltip. Only the owner may dismiss it — a stale
/// pointer hide can't kill a focus-driven tooltip or another surface's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TooltipOwner {
    /// Shown because the pointer hovers the a11y node with this id.
    Pointer(u32),
    /// Shown because a11y focus is on the node with this id. Nothing in the
    /// pointer path constructs this yet — scaffolding for focus-driven
    /// tooltips; the owner-scoped hide/keep_alive already respect it.
    Focus(u32),
}

pub(crate) struct Tooltip {
    pub owner: TooltipOwner,
    pub text: String,
    pub x: i32,
    pub y: i32,
    pub timeout: u32,
    /// Fade progress 0..=255 — ramps up on show, down on dismiss.
    pub alpha: u8,
}

/// Alpha added per tick while fading in (≈6 ticks to fully opaque).
const FADE_IN_STEP: u8 = 43;
/// Ticks a dismissal takes (the delayed-dismiss window on mouse-leave).
const FADE_OUT_TICKS: u32 = 8;
/// Alpha removed per fade-out tick (255 over ~8 ticks).
const FADE_OUT_STEP: u8 = 32;
/// Timeout refreshed while the pointer stays on the owning node.
const KEEPALIVE_TIMEOUT: u32 = 120;

pub(crate) struct TooltipManager {
    pub active: Option<Tooltip>,
    /// Fade-out ticks remaining; `Some` while a dismissal is in progress.
    fade_out: Option<u32>,
}

impl TooltipManager {
    pub fn new() -> Self {
        TooltipManager {
            active: None,
            fade_out: None,
        }
    }

    /// Show a tooltip for `owner`, replacing any current one — only one is
    /// ever visible. Starts a fade-in from transparent. Returns true (a
    /// show is always a visible change for the frame renderer).
    pub fn show(&mut self, owner: TooltipOwner, text: &str, x: i32, y: i32, timeout: u32) -> bool {
        self.active = Some(Tooltip {
            owner,
            text: text.into(),
            x,
            y,
            timeout,
            alpha: 0,
        });
        self.fade_out = None;
        true
    }

    /// Begin dismissing the tooltip — but only if `owner` owns it. A stale
    /// owner is ignored, so one surface can't yank another's tooltip. The
    /// tooltip fades out over a few ticks rather than vanishing instantly,
    /// giving the pointer a chance to return to the same node and cancel.
    /// Returns true if a fade-out actually started.
    pub fn hide(&mut self, owner: TooltipOwner) -> bool {
        if self.active.as_ref().is_some_and(|t| t.owner == owner) {
            self.fade_out = Some(FADE_OUT_TICKS);
            true
        } else {
            false
        }
    }

    /// Refresh the owner's tooltip: extend its timeout and cancel any
    /// in-progress fade-out (the pointer returned to the same node within
    /// the dismiss window). A foreign owner is ignored. Returns true if the
    /// visual state changed (a fade-out was cancelled).
    pub fn keep_alive(&mut self, owner: TooltipOwner) -> bool {
        let Some(t) = self.active.as_mut() else {
            return false;
        };
        if t.owner != owner {
            return false;
        }
        t.timeout = KEEPALIVE_TIMEOUT.max(t.timeout);
        if self.fade_out.is_some() {
            self.fade_out = None;
            t.alpha = 255;
            true
        } else {
            false
        }
    }

    /// Advance the fade: ramp alpha up while visible, down during a
    /// dismissal, and clear the tooltip once the fade-out completes.
    /// Returns true if the visible state changed (a repaint is needed).
    pub fn tick(&mut self) -> bool {
        if self.active.is_none() {
            return false;
        }
        if let Some(remaining) = self.fade_out {
            let next = remaining.saturating_sub(1);
            if let Some(t) = self.active.as_mut() {
                t.alpha = t.alpha.saturating_sub(FADE_OUT_STEP);
            }
            if next == 0 {
                self.active = None;
                self.fade_out = None;
            } else {
                self.fade_out = Some(next);
            }
            return true;
        }
        let (expired, changed) = {
            let t = self.active.as_mut().unwrap();
            if t.timeout > 0 {
                t.timeout -= 1;
            }
            // Report a repaint only when the alpha actually moved: a
            // fully-faded-in tooltip sitting on screen is static, so the
            // damage-gated render loop skips it (no per-frame full repaint
            // just for hovering a node).
            let old = t.alpha;
            t.alpha = t.alpha.saturating_add(FADE_IN_STEP);
            (t.timeout == 0, old != t.alpha)
        };
        if expired {
            // Timed out without a keep-alive (e.g. a focus tooltip whose
            // owner went away): fade out instead of vanishing instantly.
            self.fade_out = Some(FADE_OUT_TICKS);
            true
        } else {
            changed
        }
    }
}
