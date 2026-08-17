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
    // the panel is a container, and keeping IT out of the ring leaves the
    // spatial-navigation geometry the keyboard-loop selftest pins
    // otherwise intact — each tray ENTRY below is its own focusable Button.
    let tray_len = d.tray.entries.len() as u32;
    let tray_panel_id = tree.add_node(
        A11yRole::TrayPanel,
        "System Tray",
        layout::tray_panel_rect(ty, d.screen_w, tray_len),
        false,
    );
    tree.set_owner(tray_panel_id, crate::core::window::TRAY_PANEL_OWNER);
    tree.add_child(taskbar_id, tray_panel_id);

    // Tray entries — one focusable Button per drawn entry, owner-stamped
    // with the same TRAY_PANEL_OWNER sentinel and bounds equal to the
    // `tray_entry_rect` the draw and hover share. Mirrors the taskbar
    // window buttons: the ring can land on an entry and the focused light
    // resolves it exactly like the hover light (`HoverTarget::Tray(i)`).
    for i in 0..tray_len as usize {
        let label = alloc::format!("{}", d.tray.entries[i].icon);
        let entry_id = tree.add_node(
            A11yRole::Button,
            &label,
            layout::tray_entry_rect(i, ty, d.screen_w, tray_len),
            true,
        );
        tree.set_owner(entry_id, crate::core::window::TRAY_PANEL_OWNER);
        tree.add_child(tray_panel_id, entry_id);
    }

    // Start Menu — plus one focusable Button child per VISIBLE app row
    // (the same scroll-aware range the draw and `menu_hover_at` use), with
    // the shared `menu_item_rect` bounds, so a keyboard user can
    // ring-navigate the menu and Enter launches the focused row (mouse-click
    // semantics). The row's identity is resolved from its bounds by
    // `Desktop::menu_row_app` — the label is display text only, never the
    // row's id, so renamed apps and duplicate names stay correct.
    //
    // The intended row (`Desktop::menu_focus_app`, set by the scroll-aware
    // arrow path) is clamped INTO the visible window BEFORE the rows are
    // built: the ring can advance past the tree's edge (arrow keys) and the
    // filter can re-index the focused app (typed search), and this one
    // clamp — every frame — keeps the focused row reachable and visible.
    // The matching row node is remembered (`row_focus_id`) and re-focused
    // in the tail, because node ids are positional: a scroll/filter change
    // can renumber the node at a given id without changing its
    // (owner, role, parent-role) fingerprint, which `validate` cannot see.
    let mut row_focus_id: Option<u32> = None;
    if d.start_menu.open {
        if let Some(app) = d.menu_focus_app {
            if let Some(i) = d.start_menu.filtered.iter().position(|&a| a == app) {
                // Clamp via the shared window rule. `end` is used instead of
                // `start + avail`: when a row can sit BEYOND the window,
                // `end == start + avail` by construction (`end` is only
                // clipped by the list length below `start + avail`), so the
                // clamp is identical.
                let menu_r = layout::menu_rect(ty);
                let (start, end, _) = d.start_menu.visible_range(menu_r);
                if end > start {
                    if i < start {
                        d.start_menu.scroll = i as u32;
                    } else if i >= end {
                        d.start_menu.scroll = (i + 1 - (end - start)) as u32;
                    }
                }
            } else {
                // The focused app was filtered out — the ring falls back to
                // `validate`'s re-sync instead of floating on a dead row.
                d.menu_focus_app = None;
            }
        }
        let start_menu_id = tree.add_node(
            A11yRole::StartMenu,
            "Start Menu",
            layout::menu_rect(ty),
            true,
        );
        tree.add_child(desktop_id, start_menu_id);

        let menu_r = layout::menu_rect(ty);
        let (start, _, rows) = d.start_menu.visible_range(menu_r);
        for (k, &app_id) in rows.iter().enumerate() {
            let i = start + k;
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
            if d.menu_focus_app == Some(app_id) {
                row_focus_id = Some(row_id);
            }
        }

        // Sidebar categories — one focusable Button per drawn category (the
        // same sidebar-bottom cap the draw and `menu_hover_at` use), with
        // the shared `menu_category_rect` bounds, so the ring can reach and
        // light them like app rows. Labels come straight from CATEGORIES
        // (display text; identity is resolved by bounds in
        // `Desktop::menu_category_index`).
        let sidebar_r = layout::menu_sidebar_rect(menu_r);
        for (i, &(cat_name, _)) in crate::util::app_catalog::CATEGORIES.iter().enumerate() {
            let cat_r = layout::menu_category_rect(menu_r, i);
            if cat_r.y + cat_r.h as i32 > sidebar_r.y + sidebar_r.h as i32 {
                break;
            }
            let cat_id = tree.add_node(A11yRole::Button, cat_name, cat_r, true);
            tree.add_child(start_menu_id, cat_id);
        }

        // Recent strip — one focusable Button per drawn tile (capped and
        // right-reserve-broken exactly like the draw and `menu_hover_at`),
        // with the shared `menu_recent_rect` bounds, so the ring can reach
        // and light the recent apps too.
        let mut rx = layout::menu_recent_x0(menu_r);
        let recent_n = d.app_reg.recent.len().min(layout::MENU_RECENT_MAX);
        for ri in 0..recent_n {
            let idx = d.app_reg.recent[ri];
            if idx >= d.app_reg.apps.len() {
                continue;
            }
            if rx + layout::MENU_RECENT_PITCH as i32
                > menu_r.x + layout::MENU_W as i32 - layout::MENU_RECENT_RIGHT_RESERVE as i32
            {
                break;
            }
            let label = d
                .app_reg
                .apps
                .get(idx)
                .map(|app| layout::trunc(app.name, layout::MENU_RECENT_NAME_MAX))
                .unwrap_or("?");
            let r_id = tree.add_node(
                A11yRole::Button,
                label,
                layout::menu_recent_rect(menu_r, rx),
                true,
            );
            tree.add_child(start_menu_id, r_id);
            rx += layout::MENU_RECENT_PITCH as i32;
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
        // Mirror `window::draw`'s skip conditions: a minimized window that
        // is not animating, and a window pushed fully off-screen (x or y
        // below -100), paint nothing — only their shadow. Their Window/
        // Close/Minimize nodes must leave the ring for the same reason the
        // taskbar overflow caps its buttons: the ring must never land on a
        // surface the draw does not paint, or the focused light silently
        // misses and the ring floats over empty space. The taskbar button
        // remains the restore path for a minimized window, and restoring
        // makes the chrome visible again next frame.
        let drawn = !(aw.state == WindowState::Minimized && aw.anim.is_none())
            && aw.x >= -100
            && aw.y >= -100;
        let win_id = tree.add_node(
            A11yRole::Window,
            &aw.title,
            Rect::new(aw.x, aw.y, aw.w, aw.h),
            true,
        );
        tree.set_owner(win_id, wid);
        tree.add_child(desktop_id, win_id);
        if !drawn {
            tree.set_visible(win_id, false);
        }

        // close button
        let close_id = tree.add_node(
            A11yRole::Button,
            "Close",
            layout::close_btn_rect(aw.x, aw.y, aw.w),
            true,
        );
        tree.set_owner(close_id, wid);
        tree.add_child(win_id, close_id);
        if !drawn {
            tree.set_visible(close_id, false);
        }

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
        if !drawn {
            tree.set_visible(min_id, false);
        }
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

    // Notification rows — one focusable Button per VISIBLE row, so the
    // ring can land on a row and the focused light resolves it exactly
    // like the hover light (`HoverTarget::Notification(i)`). Owner-stamped
    // with the NOTIFICATION_OWNER sentinel (no window owns a notification)
    // and bounds equal to the `notification_rect` the overlay draw and
    // hover share. Parented to the Desktop node like the overlay itself;
    // rows are the only Button children of Desktop, which is what keeps
    // the Desktop-parent arm in `focused_target` unambiguous.
    let notifs = d.services.notifications.visible_notifications();
    for (i, n) in notifs.iter().take(layout::NOTIF_MAX_VISIBLE).enumerate() {
        let notif_id = tree.add_node(
            A11yRole::Button,
            &n.title,
            layout::notification_rect(d.screen_w, i),
            true,
        );
        tree.set_owner(notif_id, crate::core::window::NOTIFICATION_OWNER);
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
    // Start-menu row intent: the ring means a specific APP ROW, but node ids
    // are positional and the row window rebuilds every frame, so a scroll or
    // filter change can silently rename the node at a given id — `validate`
    // cannot detect that (the fingerprint is identical for any StartMenu
    // row). Re-land the ring on the intended row's node now that the tree is
    // built; clear the intent when the menu is closed so a stale row can't
    // resurrect on the next open.
    if d.start_menu.open {
        if let Some(nid) = row_focus_id {
            d.focus.focus(nid);
        }
    } else {
        d.menu_focus_app = None;
    }
    // Window-activation intent: `activate_a11y_node` brought a window to
    // front, reordering `wm` and renumbering every window-surface node id
    // (positional). `validate` above would have re-synced the ring to a
    // sibling taskbar button (the old id's fingerprint no longer matches).
    // Re-land the ring on the ACTIVATED window's own node in the fresh
    // tree — the durable-identity twin of the menu-row re-land above. The
    // intent is one-shot: consumed (or cleared) here, every rebuild.
    if let Some(wid) = d.pending_window_focus.take() {
        if let Some(nid) = tree
            .nodes
            .iter()
            .find(|n| n.owner == Some(wid) && n.role == A11yRole::Window)
            .map(|n| n.id)
        {
            d.focus.focus(nid);
        }
    }
    if let Some(fid) = d.focus.focused() {
        tree.set_focus(fid);
    }
    tree
}

/// The Button node with the given id if its parent node has the given
/// role — the shared parent-role + focus-id lookup every focus-resolution
/// computation uses: taskbar buttons resolve by owner, start-menu rows by
/// bounds, and window chrome controls by label, but all three first need
/// "is this focused Button under that parent role?", which is this one
/// function. `fid` is the focused id at every call site (the snapshot's
/// `focused_target`), but the helper is pure over (tree, id) so callers
/// and tests can hand it any node id.
pub(crate) fn focused_button_under_role(
    tree: &A11yTree,
    fid: u32,
    role: A11yRole,
) -> Option<&A11yNode> {
    let node = tree.nodes.iter().find(|n| n.id == fid)?;
    if node.role != A11yRole::Button {
        return None;
    }
    let parent_role = node
        .parent
        .and_then(|pid| tree.nodes.iter().find(|p| p.id == pid))
        .map(|p| p.role)?;
    (parent_role == role).then_some(node)
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
