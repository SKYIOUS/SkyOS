use crate::core::desktop::Desktop;
use crate::core::window::AppWindow;
use libsarga::io;

pub(crate) fn test_window_creation(desktop: &mut Desktop) -> bool {
    let before = desktop.wm.len();
    let win = AppWindow::new(100, 100, 400, 300, "TestWin");
    let id = desktop.wm.create(win);
    if desktop.wm.len() != before + 1 {
        io::print_str("[test] FAIL test_window_creation: wm.len() did not increase\n");
        return false;
    }
    if desktop.wm.active().is_none() {
        io::print_str("[test] FAIL test_window_creation: no active window\n");
        return false;
    }
    desktop.wm.close(id);
    // close() is animated — the window leaves only when process_closing runs
    // during tick, so drain it before counting (same idiom as
    // launcher::check_spawn_registers).
    for _ in 0..60 {
        desktop.tick();
    }
    if desktop.wm.len() != before {
        io::print_str("[test] FAIL test_window_creation: close did not restore count\n");
        return false;
    }
    io::print_str("[test] PASS test_window_creation\n");
    true
}

pub(crate) fn test_window_focus(desktop: &mut Desktop) -> bool {
    let before = desktop.wm.len();
    let win_a = AppWindow::new(50, 50, 300, 200, "FocusA");
    let mut win_b = AppWindow::new(200, 200, 300, 200, "FocusB");
    win_b.focused = false;
    let id_a = desktop.wm.create(win_a);
    let id_b = desktop.wm.create(win_b);

    desktop.wm.bring_to_front(id_a);
    desktop.wm.bring_to_front(id_b);
    desktop.wm.close(id_b);
    desktop.wm.close(id_a);
    // Drain the animated close before counting (see test_window_creation).
    for _ in 0..60 {
        desktop.tick();
    }
    if desktop.wm.len() != before {
        io::print_str("[test] FAIL test_window_focus: windows not cleaned up\n");
        return false;
    }
    io::print_str("[test] PASS test_window_focus\n");
    true
}

pub(crate) fn test_start_menu(desktop: &mut Desktop) -> bool {
    if desktop.start_menu.open {
        io::print_str("[test] FAIL test_start_menu: menu already open\n");
        return false;
    }
    desktop.start_menu.open_with(&desktop.app_reg);
    if !desktop.start_menu.open {
        io::print_str("[test] FAIL test_start_menu: open_with did not set open=true\n");
        return false;
    }
    desktop.start_menu.open = false;
    if desktop.start_menu.open {
        io::print_str("[test] FAIL test_start_menu: could not close menu\n");
        return false;
    }
    io::print_str("[test] PASS test_start_menu\n");
    true
}

/// Pins the start-menu click→action routing: `Desktop::handle_click`
/// delegates row geometry to `start_menu::menu_hover_at` (the single source
/// of truth shared with hover and the draw), so this test asserts the
/// HoverTarget→action mapping each row type drives. Every click below is a
/// real `handle_click` call through the delegate, so a routing or geometry
/// regression fails here without a QEMU boot.
pub(crate) fn test_start_menu_clicks(desktop: &mut Desktop) -> bool {
    let ty = desktop.taskbar_y();
    let menu_r = crate::layout::menu_rect(ty);

    desktop.start_menu.open_with(&desktop.app_reg);
    if !desktop.start_menu.open {
        io::print_str("[test] FAIL test_start_menu_clicks: menu did not open\n");
        return false;
    }

    // Category row click → switches category (selection/scroll reset).
    let cat = crate::layout::menu_category_rect(menu_r, 1);
    desktop.handle_click(cat.x + cat.w as i32 / 2, cat.y + cat.h as i32 / 2);
    if desktop.start_menu.cat_idx != 1 {
        io::print_str("[test] FAIL test_start_menu_clicks: category click did not switch\n");
        return false;
    } // App-row click → launches. "About SARGA" (exec "") only opens the
      // about dialog — no fork — so the launch is observable in-process.
      // Narrow the list so the row is row 0 (visible regardless of scroll).
    let about_idx = match desktop
        .app_reg
        .apps
        .iter()
        .position(|a| a.name == "About SARGA")
    {
        Some(i) => i,
        None => {
            io::print_str("[test] FAIL test_start_menu_clicks: About SARGA missing from catalog\n");
            return false;
        }
    };
    desktop.start_menu.search.extend_from_slice(b"about");
    desktop.start_menu.rebuild_filter(&desktop.app_reg);
    if desktop.start_menu.filtered != [crate::util::app_catalog::AppId(about_idx)] {
        io::print_str("[test] FAIL test_start_menu_clicks: about filter wrong\n");
        return false;
    }
    let row0 = crate::layout::menu_item_rect(menu_r, 0, 0);
    desktop.handle_click(row0.x + row0.w as i32 / 2, row0.y + row0.h as i32 / 2);
    if !desktop.about_state.open {
        io::print_str("[test] FAIL test_start_menu_clicks: app-row click did not launch\n");
        return false;
    }

    // Recent tile click → launches too. Seed the recent queue (the single
    // launch-history owner) and tap the first tile.
    desktop.about_state.open = false;
    desktop.app_reg.recent.clear();
    desktop
        .app_reg
        .record_launch(crate::util::app_catalog::AppId(about_idx));
    desktop.start_menu.open_with(&desktop.app_reg);
    let rx0 = crate::layout::menu_recent_x0(menu_r);
    let tile = crate::layout::menu_recent_rect(menu_r, rx0);
    desktop.handle_click(tile.x + tile.w as i32 / 2, tile.y + tile.h as i32 / 2);
    if !desktop.about_state.open {
        io::print_str("[test] FAIL test_start_menu_clicks: recent click did not launch\n");
        return false;
    }

    // Power buttons have no click action (keyboard nav + hover only): a
    // click is a deliberate no-op, not a miss. The menu must be open for
    // the click to reach the start-menu block at all — `launch_app` above
    // closed it.
    desktop.about_state.open = false;
    desktop.start_menu.open_with(&desktop.app_reg);
    let pow = crate::layout::menu_power_rect(menu_r, 0);
    desktop.handle_click(pow.x + pow.w as i32 / 2, pow.y + pow.h as i32 / 2);
    if desktop.about_state.open || !desktop.start_menu.open {
        io::print_str("[test] FAIL test_start_menu_clicks: power click changed state\n");
        return false;
    }

    // Empty menu area (no row under the pointer) → no-op, menu stays open.
    desktop.start_menu.search.clear();
    desktop.start_menu.search.extend_from_slice(b"zzzznomatch");
    desktop.start_menu.rebuild_filter(&desktop.app_reg);
    let list = crate::layout::menu_list_rect(menu_r);
    desktop.handle_click(list.x + list.w as i32 / 2, list.y + list.h as i32 / 2);
    if !desktop.start_menu.open {
        io::print_str("[test] FAIL test_start_menu_clicks: empty-area click closed menu\n");
        return false;
    }

    // Outside the menu → closes it (the outer gate).
    desktop.handle_click(menu_r.x - 10, menu_r.y - 10);
    if desktop.start_menu.open {
        io::print_str("[test] FAIL test_start_menu_clicks: outside click did not close menu\n");
        return false;
    }

    io::print_str("[test] PASS test_start_menu_clicks\n");
    true
}
