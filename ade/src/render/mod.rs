pub(crate) mod overlay;

pub(crate) fn render(win: &mut libsarga::gui::Window, desktop: &crate::desktop::Desktop) {
    crate::wallpaper::draw(win, desktop);

    for icon in &desktop.icons {
        crate::icons::draw(win, &desktop.theme, icon.0, icon.1, icon.2);
    }

    for aw in desktop.wm.windows() {
        crate::window::draw(win, &desktop.theme, aw);
    }

    crate::taskbar::draw(win, &desktop.theme, desktop);

    if desktop.start_menu {
        crate::start_menu::draw(win, &desktop.theme, desktop);
    }

    overlay::draw_context_menu(win, desktop);
}
