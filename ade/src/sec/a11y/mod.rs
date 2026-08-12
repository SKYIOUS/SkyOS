pub(crate) mod focus;
pub(crate) mod node;
pub(crate) mod tree;

pub(crate) use focus::{FocusDirection, FocusManager};
pub(crate) use node::A11yRole;
pub(crate) use tree::A11yTree;

use crate::core::desktop::Desktop;
use crate::core::geometry::Rect;
use crate::core::window::{HoverTarget, WindowButton, WindowId, WindowState};
use crate::layout;
use crate::sec::a11y::node::A11yNode;
use crate::util::app_catalog::AppId;

/// Rebuild the accessibility tree from the desktop's current state — the
/// single place that maps windows, taskbar, start menu, icons, and
/// notifications onto a11y nodes. Lives here (not on Desktop) so the
/// coordinator stays a thin shell; `Desktop::tick` calls it every frame.
/// Deliberately builds a fresh tree (the old in-Desktop builder cleared and
/// reused one buffer — one small allocation per frame either way).
///
/// Takes `&mut Desktop` because the focus-sync step validates the
/// `FocusManager` against the fresh tree and may re-sync (or blur) a stale
/// focused id — the ring can't survive a window closing on any path
/// (mouse, Ctrl+W, reap) without that write-back.
pub(crate) fn build_tree(d: &mut Desktop) -> A11yTree {
    let mut tree = A11yTree::new();
    let ty = d.taskbar_y();

    // root: Desktop
    let desktop_id = tree.add_node(
        A11yRole::Desktop,
        "Desktop",
        Rect::new(0, 0, d.screen_w, ty),
        false,
    );

    // Taskbar
    let taskbar_id = tree.add_node(
        A11yRole::Taskbar,
        "Taskbar",
        Rect::new(0, ty as i32, d.screen_w, layout::TASKBAR_H),
        true,
    );
    tree.add_child(desktop_id, taskbar_id);

    // Start button — stamped with the sentinel owner so a11y activation can
    // tell it apart from taskbar window buttons (real window ids) and from
    // Close controls, and toggle the start menu — mirroring a mouse click on
    // the button. Tooltip resolution falls back to the "Start" label because
    // `wm.lookup` never resolves the sentinel.
    let sb_id = tree.add_node(A11yRole::Button, "Start", layout::start_btn_rect(ty), true);
    tree.set_owner(sb_id, crate::core::window::START_BUTTON_OWNER);
    tree.add_child(taskbar_id, sb_id);

    // Window buttons in taskbar — stamped with their window's owner so a11y
    // activation brings the window to front (restoring it first if
    // minimized), mirroring a taskbar mouse click. Their parent is the
    // Taskbar node, which keeps them distinct from window Close buttons
    // (children of Window nodes) even when a title happens to be "Close".
    // The loop mirrors `taskbar::draw`'s overflow cap (TASKBAR_MAX_BTNS):
    // with more windows than fit, the extra buttons are NOT drawn and must
    // not exist here either — otherwise keyboard focus could land on an
    // undrawn button, the focused light would silently miss, and the focus
    // ring would float over the overflow/tray region. Overflow windows stay
    // reachable via their Window nodes.
    let taskbar_overflow = d.wm.len() > layout::TASKBAR_MAX_BTNS;
    for i in 0..d.wm.len() {
        if taskbar_overflow && i >= layout::TASKBAR_MAX_BTNS {
            break;
        }
        let Some(wid) = d.wm.id_at(i) else {
            continue;
        };
        let aw = &d.wm.iter()[i];
        let btn_id = tree.add_node(
            A11yRole::Button,
            &aw.title,
            layout::taskbar_btn_rect(i, ty),
            true,
        );
        tree.set_owner(btn_id, wid);
        tree.add_child(taskbar_id, btn_id);
    }

    // System tray panel — the whole panel (entries + clock) as one node,
    // the same `tray_panel_rect` the taskbar draws, so tooltip and hover
    // geometry match the drawn panel exactly. Owner-stamped with the
    // TRAY_PANEL_OWNER sentinel (no window owns the tray); non-focusable:
    // it is a status surface, not a keyboard control, and keeping it out of
    // the ring preserves the spatial-navigation geometry the keyboard-loop
    // selftest pins.
    let tray_len = d.tray.entries.len() as u32;
    let tray_panel_id = tree.add_node(
        A11yRole::TrayPanel,
        "System Tray",
        layout::tray_panel_rect(ty, d.screen_w, tray_len),
        false,
    );
    tree.set_owner(tray_panel_id, crate::core::window::TRAY_PANEL_OWNER);
    tree.add_child(taskbar_id, tray_panel_id);

    // Start Menu — plus one focusable Button child per VISIBLE app row
    // (the same scroll-aware range the draw and `menu_hover_at` use), with
    // the shared `menu_item_rect` bounds, so a keyboard user can
    // ring-navigate the menu and Enter launches the focused row (mouse-click
    // semantics). The row's identity is resolved from its bounds by
    // `Desktop::menu_row_app` — the label is display text only, never the
    // row's id, so renamed apps and duplicate names stay correct.
    if d.start_menu.open {
        let start_menu_id = tree.add_node(
            A11yRole::StartMenu,
            "Start Menu",
            layout::menu_rect(ty),
            true,
        );
        tree.add_child(desktop_id, start_menu_id);

        let menu_r = layout::menu_rect(ty);
        let list_r = layout::menu_list_rect(menu_r);
        let avail = (list_r.h / layout::MENU_ITEM_H) as usize;
        let start = d.start_menu.scroll as usize;
        let end = (start + avail).min(d.start_menu.filtered.len());
        for i in start..end {
            let app_id = d.start_menu.filtered[i];
            let name = d
                .app_reg
                .get(app_id)
                .map(|app| layout::trunc(app.name, layout::MENU_APP_NAME_MAX))
                .unwrap_or("?");
            let row_id = tree.add_node(
                A11yRole::Button,
                name,
                layout::menu_item_rect(menu_r, i, start),
                true,
            );
            tree.add_child(start_menu_id, row_id);
        }
    }

    // Windows (index loop: the WindowId is needed to stamp the owner onto
    // the window node and its Close button so a11y activation can route
    // back to the real window). `id_at` cannot fail for i < len, so the
    // guard is purely defensive.
    for i in 0..d.wm.len() {
        let Some(wid) = d.wm.id_at(i) else {
            continue;
        };
        let aw = &d.wm.iter()[i];
        let win_id = tree.add_node(
            A11yRole::Window,
            &aw.title,
            Rect::new(aw.x, aw.y, aw.w, aw.h),
            true,
        );
        tree.set_owner(win_id, wid);
        tree.add_child(desktop_id, win_id);

        // close button
        let close_id = tree.add_node(
            A11yRole::Button,
            "Close",
            layout::close_btn_rect(aw.x, aw.y, aw.w),
            true,
        );
        tree.set_owner(close_id, wid);
        tree.add_child(win_id, close_id);

        // minimize button — same chrome pattern as Close: owner-stamped
        // Window child, so a11y activation can route back to the real
        // window, and the parent role keeps it distinct from a taskbar
        // button even when a window title is literally "Minimize". The
        // chrome Close/Minimize pair is discriminated by label inside
        // `activate_a11y_node` (parent-role alone cannot tell them apart).
        let min_id = tree.add_node(
            A11yRole::Button,
            "Minimize",
            layout::min_btn_rect(aw.x, aw.y, aw.w),
            true,
        );
        tree.set_owner(min_id, wid);
        tree.add_child(win_id, min_id);
    }

    // Desktop Icons
    for ic in &d.desktop_icons.icons {
        let icon_id = tree.add_node(
            A11yRole::Icon,
            &ic.name,
            Rect::new(ic.x, ic.y, 48, 56),
            true,
        );
        tree.add_child(desktop_id, icon_id);
    }

    // Notifications
    for n in d.services.notifications.visible_notifications() {
        let notif_id = tree.add_node(
            A11yRole::Notification,
            &n.title,
            Rect::new(0, 0, 0, 0),
            false,
        );
        tree.add_child(desktop_id, notif_id);
    }

    // Sync focus from FocusManager — but first validate it against the tree
    // just built. A focused id whose window closed on any non-a11y path
    // (mouse Close, Ctrl+W, reap) no longer exists here; `validate` re-syncs
    // to a sibling window's surface or blurs, so the ring can never
    // silently point at a vanished node. The PREVIOUS tree (`d.a11y_tree`
    // is still the old tree — the caller assigns this function's return
    // after evaluation) supplies the focused node's identity fingerprint,
    // so an id that SURVIVES the rebuild but now names a different node
    // (ids are positional) is also detected as stale and re-synced.
    let prev_fp = d
        .focus
        .focused()
        .and_then(|fid| d.a11y_tree.nodes.iter().find(|n| n.id == fid))
        .map(|n| crate::sec::a11y::focus::node_fingerprint(&d.a11y_tree, n));
    d.focus.validate(&tree, prev_fp);
    if let Some(fid) = d.focus.focused() {
        tree.set_focus(fid);
    }
    tree
}

/// Pure tooltip label resolution — the single place all hover text is
/// formatted (Close/Minimize controls, taskbar buttons, start-menu rows,
/// and the owner/label fallback). `hover` is the unified hover target from
/// `Desktop::hover_target`; the lookups are injected so the function has no
/// `Desktop` dependency and is host-testable (`tests/test_tooltip_contract.py`
/// ports this exact shape). Returns the text to show, or an empty string for
/// no tooltip.
pub(crate) fn format_tooltip<'a>(
    node: &A11yNode,
    hover: Option<HoverTarget>,
    title_of: impl Fn(WindowId) -> Option<&'a str>,
    desc_of: impl Fn(usize) -> Option<&'a str>,
    recent_desc_of: impl Fn(usize) -> Option<&'a str>,
    minimized_of: impl Fn(WindowId) -> bool,
) -> alloc::string::String {
    match hover {
        // Window control buttons: "Close <title>" / "Minimize <title>" — the
        // owner stamp resolves the title from the injected lookup.
        Some(HoverTarget::Window { win, btn }) => {
            let action = match btn {
                WindowButton::Close => "Close",
                WindowButton::Minimize => "Minimize",
            };
            let title = title_of(win).unwrap_or("");
            alloc::format!("{} {}", action, title)
        }
        // Taskbar window buttons: "Switch to <title>" ("Restore <title>" for
        // a minimized window, whose taskbar click restores instead of
        // switching) — same owner resolution, same action semantics as the
        // taskbar mouse click.
        Some(HoverTarget::TaskbarButton(wid)) => {
            let title = title_of(wid).unwrap_or("");
            if minimized_of(wid) {
                alloc::format!("Restore {}", title)
            } else {
                alloc::format!("Switch to {}", title)
            }
        }
        // Start-menu app rows (the tree only models the menu container, so
        // the hover target carries the row index): show the app description.
        Some(HoverTarget::StartApp(i)) => desc_of(i).unwrap_or_default().into(),
        // Recent strip is the same catalog, so descriptions apply there too.
        Some(HoverTarget::StartRecent(ri)) => recent_desc_of(ri).unwrap_or_default().into(),
        // The Start button opens the menu; name the action like the others.
        Some(HoverTarget::StartButton) => "Open Start menu".into(),
        _ => {
            // Everything else: owner-stamped controls (window Close buttons
            // and taskbar buttons when the hover target is stale) show the
            // owning window's title; plain nodes show their label, with the
            // role-name fallback for empty labels (the tree stamps labels,
            // so this only fires for defensive completeness).
            if let Some(wid) = node.owner {
                title_of(wid)
                    .map(alloc::string::String::from)
                    .unwrap_or_else(|| node.label.clone())
            } else if node.label.is_empty() {
                match node.role {
                    A11yRole::Taskbar => "Taskbar".into(),
                    A11yRole::StartMenu => "Start Menu".into(),
                    A11yRole::Desktop => "Desktop".into(),
                    A11yRole::TrayPanel => "System Tray".into(),
                    _ => alloc::string::String::new(),
                }
            } else {
                node.label.clone()
            }
        }
    }
}

/// Desktop adapter for [`format_tooltip`]: injects the live lookups (WM
/// title lookup, start-menu app descriptions, recent-strip descriptions).
pub(crate) fn tooltip_label(
    d: &Desktop,
    node: &A11yNode,
    hover: Option<HoverTarget>,
) -> alloc::string::String {
    format_tooltip(
        node,
        hover,
        |wid| d.wm.lookup(wid).map(|w| w.title.as_str()),
        |i| {
            d.start_menu
                .filtered
                .get(i)
                .copied()
                .and_then(|id| d.app_reg.get(id))
                .map(|app| app.description)
        },
        |ri| {
            d.app_reg
                .recent
                .get(ri)
                .copied()
                .and_then(|id| d.app_reg.get(AppId(id)))
                .map(|app| app.description)
        },
        |wid| {
            d.wm.lookup(wid)
                .is_some_and(|w| w.state == WindowState::Minimized)
        },
    )
}
