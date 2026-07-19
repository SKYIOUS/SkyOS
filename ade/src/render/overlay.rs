pub(crate) fn draw_context_menu(win: &mut libsarga::gui::Window, desktop: &crate::desktop::Desktop) {
    if let Some((mx, my, items)) = desktop.context_menu {
        let mw = 150u32;
        let mh = items.len() as u32 * 28 + 10;
        win.draw_rounded_rect(mx as u32, my as u32, mw, mh, 6, desktop.theme.bg_elevated);
        win.draw_rounded_rect_outline(mx as u32, my as u32, mw, mh, 6, desktop.theme.border);
        for (i, (name, _)) in items.iter().enumerate() {
            let iy = my as u32 + 5 + i as u32 * 28;
            win.draw_string(mx as u32 + 10, iy + 6, name, desktop.theme.text, 0);
        }
    }
}
