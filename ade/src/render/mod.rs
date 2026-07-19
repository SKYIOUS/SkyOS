//! Render pipeline — frames a snapshot onto the window surface.

pub(crate) mod clock;
pub(crate) mod overlay;
pub(crate) mod snapshot;

pub(crate) fn render(
    win: &mut libsarga::gui::Window,
    snap: &snapshot::RenderSnapshot,
    clock_str: &str,
) {
    crate::wallpaper::draw(win, snap);

    if !snap.fullscreen {
        crate::desktop_icons::draw(win, snap.icons, snap.theme, snap.rubber);
    }

    for aw in snap.windows {
        if !aw.always_on_top {
            crate::window::draw(win, snap.theme, aw, snap.cursor_visible, snap.explorers);
        }
    }
    for aw in snap.windows {
        if aw.always_on_top {
            crate::window::draw(win, snap.theme, aw, snap.cursor_visible, snap.explorers);
        }
    }

    if !snap.fullscreen {
        crate::taskbar::draw(win, snap, clock_str);
    }

    if snap.start_menu {
        crate::start_menu::draw(win, snap); // reads start_menu_state + app_db from snapshot
    }

    overlay::draw_context_menu(win, snap);
    overlay::draw_clipboard(win, snap);
    overlay::draw_notifications(win, snap);
    if let Some(s) = snap.settings {
        s.draw(win, snap);
    }

    if snap.switcher_active {
        overlay::draw_switcher(win, snap);
    }
}
