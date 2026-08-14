//! Window primitives — AppWindow, WindowId, text cursor input handling.

use crate::core::text_surface::TextSurface;
use crate::layout;
use libsarga::theme::Theme;

// Window API v1.0 — STABLE
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowId(pub(crate) u64);

/// Sentinel `WindowId` stamping the a11y Start-button node. The a11y owner
/// field is `Option<WindowId>`; the Start button owns no window, so it gets
/// this reserved id (the window manager never hands it out) to distinguish it
/// structurally from taskbar window buttons — whose activation brings a
/// window to front — and from window Close controls. Activation code matches
/// this constant to toggle the start menu; tooltip label resolution falls
/// back to the node label when `wm.lookup` misses (it always will for this
/// id).
pub(crate) const START_BUTTON_OWNER: WindowId = WindowId(u64::MAX);

/// Sentinel `WindowId` stamping the a11y tray-panel node. The tray panel
/// (entries + clock) owns no window, so its tree node gets this reserved id
/// (never handed out by the window manager, distinct from the Start-button
/// sentinel) so tooltip resolution and future activation can identify it
/// structurally.
pub(crate) const TRAY_PANEL_OWNER: WindowId = WindowId(u64::MAX - 1);

/// Sentinel `WindowId` stamping the a11y notification-row nodes. A
/// notification owns no window, so its overlay rows get this reserved id
/// (never handed out by the window manager, distinct from the Start-button
/// and tray-panel sentinels) so tooltip resolution, focus resolution, and
/// future activation can identify them structurally.
pub(crate) const NOTIFICATION_OWNER: WindowId = WindowId(u64::MAX - 2);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowState {
    Normal,
    Minimized,
    Maximized,
    Fullscreen,
}

/// Window control buttons — hover feedback target for the titlebar chrome.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WindowButton {
    Close,
    Minimize,
}

/// Map a tree chrome-control label back to its `WindowButton`. The a11y
/// tree stamps Close/Minimize nodes with exactly these labels; both the
/// activation arm and the render snapshot's focused-control resolution need
/// the reverse map. One helper keeps the three sites (tree stamping,
/// activation, focus resolution) in lockstep, so a label rename can't
/// silently break one path.
pub(crate) fn window_button_from_label(label: &str) -> Option<WindowButton> {
    match label {
        "Close" => Some(WindowButton::Close),
        "Minimize" => Some(WindowButton::Minimize),
        _ => None,
    }
}

/// A pointer hover target — which interactive surface the mouse is over.
/// Computed once per frame by `Desktop` (`hover_target()`, the single hit
/// test) and threaded through the render snapshot, so every surface (window
/// controls, taskbar, start menu, tray, clipboard) reads one hover state
/// instead of hit-testing the mouse position in each draw. Payloads carry
/// exactly what the draw needs to light up the right element.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HoverTarget {
    /// A window's control button (payload: window + which button).
    Window { win: WindowId, btn: WindowButton },
    /// A taskbar window button (payload: the window it switches to).
    TaskbarButton(WindowId),
    /// The Start button.
    StartButton,
    /// A start-menu category row (payload: index into CATEGORIES).
    StartCategory(usize),
    /// A start-menu app row (payload: index into the filtered list).
    StartApp(usize),
    /// A start-menu recent tile (payload: index into the recent strip).
    StartRecent(usize),
    /// A start-menu power button (payload: index into POWER_LABELS).
    StartPower(usize),
    /// A system-tray entry (payload: index into the tray entries).
    Tray(usize),
    /// A clipboard history row (payload: index into the history).
    ClipboardRow(usize),
    /// A notification overlay row (payload: index into the visible
    /// notifications).
    Notification(usize),
    /// A legacy settings-panel row (payload: 0 = Sound, 1 = Dark Theme,
    /// 2 = Close). Computed by `Desktop::hover_target` — the panels no
    /// longer hit-test the mouse themselves.
    SettingsRow(usize),
    /// A settings-app row (payload: 0 = the Appearance theme toggle; the
    /// only row the app currently draws a hover state for).
    SettingsAppRow(usize),
    /// A task-manager row (payload: index into the visible window list).
    TaskManagerRow(usize),
}

// Window API v1.0 — STABLE
#[derive(Clone, Copy, Debug)]
pub(crate) struct VisualFlags {
    pub shadow: bool,
    pub opacity: u8,
    pub rounded: bool,
    pub border: bool,
    pub active: bool,
    pub border_width: u8,
    pub blur: bool,
    pub transparent: bool,
}

impl VisualFlags {
    pub(crate) fn new() -> Self {
        VisualFlags {
            shadow: true,
            opacity: 255,
            rounded: true,
            border: true,
            active: true,
            border_width: 1,
            blur: false,
            transparent: false,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct AnimState {
    pub from_x: i32,
    pub from_y: i32,
    pub from_w: u32,
    pub from_h: u32,
    pub to_x: i32,
    pub to_y: i32,
    pub to_w: u32,
    pub to_h: u32,
    pub tick: u32,
    pub duration: u32,
}

/// Terminal window state: the pty master fd plus the text surface the shell's
/// output feeds into. A terminal window keeps its surface here rather than on
/// `AppWindow` itself; plain windows keep theirs on the window.
pub(crate) struct Terminal {
    pty_fd: i64,
    surface: TextSurface,
}

pub struct AppWindow {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) w: u32,
    pub(crate) h: u32,
    pub(crate) prev_x: i32,
    pub(crate) prev_y: i32,
    pub(crate) prev_w: u32,
    pub(crate) prev_h: u32,
    pub(crate) title: alloc::string::String,
    /// Text surface for plain (non-terminal) windows — legacy typed input
    /// and launcher seeds. Terminal windows host their surface inside
    /// `terminal`; reach both through the `surface()`/`surface_mut()`
    /// accessors, never the field directly.
    surface: TextSurface,
    /// Terminal host (pty master fd + the pty-fed surface), when this window
    /// runs a shell. Set by `attach_terminal`; cleared by `take_terminal`.
    terminal: Option<Terminal>,
    pub(crate) id: u64,
    pub(crate) pid: Option<u64>,
    pub(crate) focused: bool,
    pub(crate) dragging: bool,
    pub(crate) drag_ox: i32,
    pub(crate) drag_oy: i32,
    pub(crate) state: WindowState,
    pub(crate) prev_state: WindowState,
    pub(crate) flags: VisualFlags,
    pub(crate) anim: Option<AnimState>,
    pub(crate) closing: bool,
    pub(crate) always_on_top: bool,
    pub(crate) explorer_id: Option<u32>,
    pub(crate) anim_opacity: u8,
}

impl AppWindow {
    /// Construct a plain floating window. Every non-geometry field takes a
    /// sane default; callers mutate the handful of fields they need after
    /// the fact: explorer_id (spawn_explorer), pid (launcher after fork),
    /// terminal (via `attach_terminal`), surface (seeds its first line).
    /// New fields added to `AppWindow` land here, not at each call site.
    pub(crate) fn new(x: i32, y: i32, w: u32, h: u32, title: &str) -> Self {
        AppWindow {
            x,
            y,
            w,
            h,
            prev_x: x,
            prev_y: y,
            prev_w: w,
            prev_h: h,
            title: alloc::string::String::from(title),
            surface: TextSurface::new(),
            terminal: None,
            id: 0,
            pid: None,
            focused: true,
            dragging: false,
            drag_ox: 0,
            drag_oy: 0,
            state: WindowState::Normal,
            prev_state: WindowState::Normal,
            flags: VisualFlags::new(),
            anim: None,
            closing: false,
            always_on_top: false,
            explorer_id: None,
            anim_opacity: 0,
        }
    }

    /// The pty master fd, if this window hosts a terminal (sash).
    pub(crate) fn pty_fd(&self) -> Option<i64> {
        self.terminal.as_ref().map(|t| t.pty_fd)
    }

    /// The text surface this window draws. Terminal windows render from the
    /// pty-fed surface; plain windows from their own.
    pub(crate) fn surface(&self) -> &TextSurface {
        match &self.terminal {
            Some(t) => &t.surface,
            None => &self.surface,
        }
    }

    /// Mutable access to the surface this window draws (see [`surface`]).
    pub(crate) fn surface_mut(&mut self) -> &mut TextSurface {
        match &mut self.terminal {
            Some(t) => &mut t.surface,
            None => &mut self.surface,
        }
    }

    /// Turn this window into a terminal host: the current surface (with any
    /// lines seeded so far) moves into the terminal's own surface, which the
    /// pty then feeds. Replaces any existing terminal on the window.
    pub(crate) fn attach_terminal(&mut self, pty_fd: i64) {
        let surface = core::mem::replace(&mut self.surface, TextSurface::new());
        self.terminal = Some(Terminal { pty_fd, surface });
    }

    /// Detach the terminal, returning the pty master fd so the caller can
    /// close it. The surface goes with it — only use when the window itself
    /// is about to be removed (close animation already finished).
    pub(crate) fn take_terminal(&mut self) -> Option<i64> {
        self.terminal.take().map(|t| t.pty_fd)
    }

    /// Detach just the pty fd (to kill the shell and free the master), but
    /// keep the terminal's surface on the window so the close animation
    /// still draws its last text.
    pub(crate) fn detach_terminal_fd(&mut self) -> Option<i64> {
        if let Some(t) = self.terminal.take() {
            let Terminal { pty_fd, surface } = t;
            self.surface = surface;
            Some(pty_fd)
        } else {
            None
        }
    }

    pub(crate) fn animate_to(&mut self, x: i32, y: i32, w: u32, h: u32) {
        self.anim = Some(AnimState {
            from_x: self.x,
            from_y: self.y,
            from_w: self.w,
            from_h: self.h,
            to_x: x,
            to_y: y,
            to_w: w,
            to_h: h,
            tick: 0,
            duration: 10,
        });
    }

    pub(crate) fn tick_animation(&mut self) -> bool {
        if let Some(ref mut a) = self.anim {
            a.tick += 1;
            let t = a.tick.min(a.duration);
            if t >= a.duration {
                self.x = a.to_x;
                self.y = a.to_y;
                self.w = a.to_w;
                self.h = a.to_h;
                self.anim = None;
            } else {
                let d = a.duration as f32;
                let u = 1.0 - t as f32 / d;
                let progress = 1.0 - u * u * u;
                self.x = a.from_x + ((a.to_x - a.from_x) as f32 * progress) as i32;
                self.y = a.from_y + ((a.to_y - a.from_y) as f32 * progress) as i32;
                self.w = a.from_w + ((a.to_w as f32 - a.from_w as f32) * progress) as u32;
                self.h = a.from_h + ((a.to_h as f32 - a.from_h as f32) * progress) as u32;
            }
            true
        } else {
            false
        }
    }

    pub(crate) fn animate_close(&mut self) {
        let cx = self.x + self.w as i32 / 2;
        let cy = self.y + self.h as i32 / 2;
        self.anim = Some(AnimState {
            from_x: self.x,
            from_y: self.y,
            from_w: self.w,
            from_h: self.h,
            to_x: cx - 1,
            to_y: cy - 1,
            to_w: 2,
            to_h: 2,
            tick: 0,
            duration: 8,
        });
    }
}

fn apply_alpha(color: u32, opacity: u8) -> u32 {
    (color & 0x00FFFFFF) | ((opacity as u32) << 24)
}

/// Per-frame interaction state threaded into the window chrome draw: the
/// unified hover target, the a11y-focused control (the same `HoverTarget`
/// payloads the chrome compares for hover, from the snapshot's `focused`
/// field), and the raw primary-button state. Bundled so the draw signature
/// stays under clippy's argument ceiling, and a future interaction input
/// (e.g. a keyboard-pressed flag) lands as a field, not another parameter.
#[derive(Clone, Copy)]
pub(crate) struct WinInteraction {
    pub hover: Option<HoverTarget>,
    pub focused: Option<HoverTarget>,
    pub mouse_down: bool,
}

/// Which interactive face a window control button shows, from the unified
/// interaction inputs. One union drives both chrome controls (Close and
/// Minimize) so "pressed vs focused vs hover" can't drift between them:
/// pressed wins (hover while the primary button is held), then the
/// hover/focused light, then the base state. Pressed is deliberately
/// pointer-only — the focused (keyboard) state never presses, exactly like
/// the taskbar buttons. Pure over three booleans, so
/// `tests/test_window_button_contract.py` ports it host-side the way
/// `format_tooltip` is ported.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WindowButtonFace {
    Pressed,
    Focused,
    Hover,
    Base,
}

/// The interaction union every chrome control draw shares: pressed (hover
/// held) > keyboard focus (a `Focused` face, drawn as the accent_light
/// blue — visually distinct from pointer hover, matching the taskbar
/// buttons) > hover (indigo) > resting. Focus wins over hover when both
/// apply (the ring is the active mode), and pressed stays pointer-only.
pub(crate) fn window_button_face(hover: bool, focused: bool, mouse_down: bool) -> WindowButtonFace {
    if hover && mouse_down {
        WindowButtonFace::Pressed
    } else if focused {
        WindowButtonFace::Focused
    } else if hover {
        WindowButtonFace::Hover
    } else {
        WindowButtonFace::Base
    }
}

pub(crate) fn draw(
    canvas: &mut crate::render::compositor::Canvas,
    theme: &Theme,
    aw: &AppWindow,
    cursor_visible: bool,
    explorers: &[crate::util::explorer::ExplorerState],
    ix: WinInteraction,
) {
    // Don't draw minimized windows (but still draw during animation).
    if aw.state == WindowState::Minimized && aw.anim.is_none() {
        return;
    }

    // Safety check.
    if aw.x < -100 || aw.y < -100 {
        return;
    }

    let border_color = if aw.focused {
        theme.accent
    } else {
        theme.border
    };

    // Shadow
    canvas.draw_rect_alpha(aw.x as u32 + 6, aw.y as u32 + 6, aw.w, aw.h, 0x60000000);

    // Window body
    canvas.draw_rounded_rect(
        aw.x as u32,
        aw.y as u32,
        aw.w,
        aw.h,
        theme.border_radius,
        apply_alpha(theme.bg_surface, aw.flags.opacity),
    );

    // Focus glow
    if aw.focused {
        let glow = (theme.accent & 0x00FF_FFFF) | 0x30_000000;
        canvas.draw_rounded_rect_outline(
            aw.x as u32 - 1,
            aw.y as u32 - 1,
            aw.w + 2,
            aw.h + 2,
            theme.border_radius + 1,
            glow,
        );
    }

    canvas.draw_rounded_rect_outline(
        aw.x as u32,
        aw.y as u32,
        aw.w,
        aw.h,
        theme.border_radius,
        apply_alpha(border_color, aw.flags.opacity),
    );

    // Title bar
    let title_c1 = if aw.focused {
        theme.accent
    } else {
        theme.bg_elevated
    };

    let title_c2 = if aw.focused {
        theme.accent_dark
    } else {
        theme.bg_surface
    };

    canvas.draw_gradient_rect(
        aw.x as u32 + 1,
        aw.y as u32 + 1,
        aw.w - 2,
        layout::TITLE_H as u32,
        apply_alpha(title_c1, aw.flags.opacity),
        apply_alpha(title_c2, aw.flags.opacity),
        false,
    );

    // Title text is on_accent (white in both themes): the titlebar gradient
    // is accent-derived (theme-invariant), so theme.text (black in light
    // mode) would be unreadable on it. on_accent is exactly this case —
    // text on an accent fill — so the chrome uses the field, not a literal.
    canvas.draw_string(
        aw.x as u32 + layout::TITLE_PAD_X,
        aw.y as u32 + layout::TITLE_TEXT_Y,
        &aw.title,
        apply_alpha(theme.on_accent, aw.flags.opacity),
        0,
    );

    if aw.always_on_top {
        // Deliberate contrast exception: orange on the accent gradient is
        // ~1.6:1 (below AA) in both themes, but the marker must stay
        // distinct from the white title text next to it; making it on_accent
        // would make it invisible against the title. Kept orange and flagged
        // in the 2026-08-08 light-theme audit.
        canvas.draw_string(
            aw.x as u32 + aw.w - layout::TITLE_AOT_OFFSET,
            aw.y as u32 + layout::TITLE_TEXT_Y,
            "[A]",
            apply_alpha(0xFFFFAA00, aw.flags.opacity),
            0,
        );
    }

    // Close button — the same rect the hit-testing uses. Hover brightens the
    // fill to the dedicated WIN_CLOSE_HOVER red (was dead in libsarga);
    // pressing it while held deepens to WIN_CLOSE_PRESSED.
    let close = layout::close_btn_rect(aw.x, aw.y, aw.w);
    let close_x = close.x as u32;
    let close_y = close.y as u32;
    let hover_close = ix.hover
        == Some(HoverTarget::Window {
            win: WindowId(aw.id),
            btn: WindowButton::Close,
        });
    // The a11y ring on this control lights it exactly like hovering it
    // (the taskbar focused-button affordance, mirrored here): keyboard
    // users see where the ring is on the chrome, and the focused control's
    // Enter action is the control that looks lit. Pressed stays pointer-only
    // (hover + mouse_down), like the taskbar buttons.
    let focused_close = ix.focused
        == Some(HoverTarget::Window {
            win: WindowId(aw.id),
            btn: WindowButton::Close,
        });
    let close_fill = match window_button_face(hover_close, focused_close, ix.mouse_down) {
        WindowButtonFace::Pressed => libsarga::theme::colors::WIN_CLOSE_PRESSED,
        // Focus keeps the semantic close red (hover-brightened) — the fill
        // means "close". Keyboard focus is marked by the accent_light ring
        // drawn after the fill, so the ring stays visually distinct from
        // pointer hover without erasing the red semantics.
        WindowButtonFace::Focused | WindowButtonFace::Hover => {
            libsarga::theme::colors::WIN_CLOSE_HOVER
        }
        WindowButtonFace::Base => theme.error,
    };

    canvas.draw_rounded_rect(
        close_x,
        close_y,
        close.w,
        close.h,
        4,
        apply_alpha(close_fill, aw.flags.opacity),
    );
    // The accent_light focus ring: "blue = ring" on the chrome — the same
    // hue as the taskbar/menu focus fills — but as a ring around the
    // control, so the close red stays the close red.
    if focused_close {
        canvas.draw_rounded_rect_outline(
            close_x,
            close_y,
            close.w,
            close.h,
            4,
            apply_alpha(theme.accent_light, aw.flags.opacity),
        );
    }
    // Same rationale as the title: the close fill is theme.error / the
    // WIN_CLOSE reds (theme-invariant), so the glyph stays on_accent for
    // contrast (4.6+ in both themes).
    canvas.draw_string(
        close_x + 7,
        close_y + 2,
        "x",
        apply_alpha(theme.on_accent, aw.flags.opacity),
        0,
    );

    // Minimize button — the same rect the hit-testing uses. Hover lifts the
    // flat elevated surface with a white wash; pressing darkens it with a
    // black wash (theme.pressed is a distinct darker navy — the taskbar
    // surfaces use it as their pressed fill — but the wash here keeps the
    // rounded glyph surface flat instead of swapping its base color).
    let min = layout::min_btn_rect(aw.x, aw.y, aw.w);
    let min_x = min.x as u32;
    let hover_min = ix.hover
        == Some(HoverTarget::Window {
            win: WindowId(aw.id),
            btn: WindowButton::Minimize,
        });
    // Same focused union as Close: the ring marks focus with the white
    // wash (the minimize semantics) — pressed is pointer-only.
    let focused_min = ix.focused
        == Some(HoverTarget::Window {
            win: WindowId(aw.id),
            btn: WindowButton::Minimize,
        });
    canvas.draw_rounded_rect(
        min_x,
        close_y,
        min.w,
        min.h,
        4,
        apply_alpha(theme.bg_elevated, aw.flags.opacity),
    );
    match window_button_face(hover_min, focused_min, ix.mouse_down) {
        WindowButtonFace::Pressed => {
            canvas.draw_rect_alpha(min_x, close_y, min.w, min.h, 0x50000000)
        }
        // Focus keeps the white wash (the minimize semantics), marked by
        // the accent_light ring drawn after the wash.
        WindowButtonFace::Focused | WindowButtonFace::Hover => {
            canvas.draw_rect_alpha(min_x, close_y, min.w, min.h, 0x35FFFFFF)
        }
        WindowButtonFace::Base => {}
    }
    if focused_min {
        canvas.draw_rounded_rect_outline(
            min_x,
            close_y,
            min.w,
            min.h,
            4,
            apply_alpha(theme.accent_light, aw.flags.opacity),
        );
    }
    canvas.draw_line_h(
        min_x + 6,
        close_y + 14,
        10,
        // The wash is white at 21% under hover AND focus (the ring marks
        // focus now, not the fill), so the glyph stays theme.text on it.
        apply_alpha(theme.text, aw.flags.opacity),
    );

    // Separation line
    canvas.draw_line_h(
        aw.x as u32 + 1,
        aw.y as u32 + layout::TITLE_SEP_Y,
        aw.w - 2,
        apply_alpha(theme.separator, aw.flags.opacity),
    );

    // Explorer content
    if let Some(exp_id) = aw.explorer_id {
        crate::util::explorer::draw_explorer_content(canvas, theme, aw, explorers, exp_id);
        return;
    }

    // Window content
    let line_y = aw.y as u32 + layout::TITLE_H as u32;
    let max_lines =
        ((aw.h - (layout::TITLE_H as u32 + layout::CONTENT_BOTTOM_PAD)) / layout::LINE_H) as usize;
    let surface = aw.surface();
    let lines = surface.lines();
    let scroll = surface.scroll();

    let start = if lines.len() > max_lines {
        lines.len() - max_lines + scroll as usize
    } else {
        0
    };

    for (i, line) in lines.iter().skip(start).take(max_lines).enumerate() {
        let ly = line_y + i as u32 * layout::LINE_H;

        if ly + layout::LINE_H > aw.y as u32 + aw.h {
            break;
        }

        let display = layout::trunc(line, layout::LINE_TRUNCATE_MAX);

        canvas.draw_string(
            aw.x as u32 + layout::CONTENT_PAD_X,
            ly,
            display,
            apply_alpha(theme.text_secondary, aw.flags.opacity),
            0,
        );
    }

    if cursor_visible && aw.focused && !lines.is_empty() {
        let last = &lines[lines.len() - 1];
        let cx = aw.x as u32 + layout::CONTENT_PAD_X + last.len() as u32 * layout::CHAR_W;
        let cy = aw.y as u32
            + (layout::TITLE_H as u32 + 2)
            + (lines.len().saturating_sub(1) as u32 - scroll).saturating_sub(1) * layout::LINE_H;
        if cy < aw.y as u32 + aw.h {
            canvas.draw_char(cx, cy, '_', apply_alpha(theme.accent, aw.flags.opacity), 0);
        }
    }
}
