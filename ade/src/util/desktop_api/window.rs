#![allow(dead_code)]

use crate::core::desktop::Desktop;
use crate::ipc::permission::PERM_WINDOW_CONTROL;
use crate::ipc::ApplicationId;
use crate::core::window::WindowId;

/// Desktop API v1.0
pub(crate) fn create(
    desktop: &mut Desktop,
    app: ApplicationId,
    title: &str,
    x: i32,
    y: i32,
    w: u32,
    h: u32,
) -> Option<WindowId> {
    if !desktop.permission_check(app, PERM_WINDOW_CONTROL) {
        return None;
    }
    let mut app_win = crate::core::window::AppWindow {
        x,
        y,
        w,
        h,
        prev_x: x,
        prev_y: y,
        prev_w: w,
        prev_h: h,
        title: alloc::string::String::from(title),
        content: alloc::vec::Vec::new(),
        scroll: 0,
        pid: None,
        focused: true,
        dragging: false,
        drag_ox: 0,
        drag_oy: 0,
        state: crate::core::window::WindowState::Normal,
        prev_state: crate::core::window::WindowState::Normal,
        flags: crate::core::window::VisualFlags::new(),
        selection: None,
        anim: None,
        closing: false,
        anim_opacity: 0,
        always_on_top: false,
        explorer_id: None,
    };
    app_win.content.push(alloc::string::String::new());
    let wid = desktop.wm.create(app_win);
    desktop.damage.mark_full();
    Some(wid)
}

/// Desktop API v1.0
pub(crate) fn close(desktop: &mut Desktop, app: ApplicationId, wid: WindowId) {
    if !desktop.permission_check(app, PERM_WINDOW_CONTROL) {
        return;
    }
    desktop.wm.close(wid);
    desktop.damage.mark_full();
}

/// Desktop API v1.0
pub(crate) fn focus(desktop: &mut Desktop, app: ApplicationId, wid: WindowId) {
    if !desktop.permission_check(app, PERM_WINDOW_CONTROL) {
        return;
    }
    desktop.wm.bring_to_front(wid);
    desktop.damage.mark_full();
}
