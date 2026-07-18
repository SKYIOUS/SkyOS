use libsarga::gui::Window;
use libsarga::theme::Theme;
use crate::desktop::Desktop;
use crate::MENU_ITEMS;

pub(crate) fn draw(win: &mut Window, theme: &Theme, desktop: &Desktop) {
    let taskbar_y = desktop.taskbar_y();
    let menu_w = 200u32;
    let menu_h = (MENU_ITEMS.len() as u32) * 32 + 40;
    let menu_x = 5u32;
    let menu_y = taskbar_y - menu_h - 4;

    win.draw_rounded_rect(menu_x, menu_y, menu_w, menu_h, 6, theme.bg_elevated);

    win.draw_rect(menu_x + 2, menu_y + 2, menu_w - 4, 34, theme.accent);
    win.draw_string(menu_x + 10, menu_y + 10, "SARGA OS Menu", 0xFFFFFFFF, 0);

    for (i, &(name, _)) in MENU_ITEMS.iter().enumerate() {
        if name == "---" {
            let sep_y = menu_y + 38 + i as u32 * 32;
            win.draw_line_h(menu_x + 8, sep_y + 16, menu_w - 16, theme.separator);
            continue;
        }
        let iy = menu_y + 38 + i as u32 * 32;
        let hover = desktop.mouse_x >= menu_x as i32
            && desktop.mouse_x < (menu_x + menu_w) as i32
            && desktop.mouse_y >= iy as i32
            && desktop.mouse_y < (iy + 28) as i32;
        let bg = if hover {
            theme.hover
        } else {
            theme.bg_elevated
        };
        win.draw_rounded_rect(menu_x + 2, iy, menu_w - 4, 28, 4, bg);
        win.draw_string(menu_x + 12, iy + 6, name, theme.text, 0);
    }
}
