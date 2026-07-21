//! Render pipeline — composite layers via the compositor onto the window surface.

pub(crate) mod clock;
pub(crate) mod compositor;
pub(crate) mod layer;
pub(crate) mod overlay;
pub(crate) mod snapshot;

use layer::Layer;
use compositor::Compositor;

pub(crate) fn render(
    win: &mut libsarga::gui::Window,
    snap: &snapshot::RenderSnapshot,
    clock_str: &str,
    comp: &mut Compositor,
) {
    comp.clear_all();

    // Wallpaper
    {
        let mut cv = comp.layer_canvas(Layer::Wallpaper);
        crate::wallpaper::draw(&mut cv, snap);
    }

    if !snap.fullscreen {
        // Desktop icons
        let mut cv = comp.layer_canvas(Layer::Desktop);
        crate::desktop_icons::draw(&mut cv, snap.icons, snap.theme, snap.rubber);
    }

    // Windows (normal then always-on-top)
    {
        let mut cv = comp.layer_canvas(Layer::Windows);
        for aw in snap.windows {
            if !aw.always_on_top {
                crate::window::draw(&mut cv, snap.theme, aw, snap.cursor_visible, snap.explorers);
            }
        }
        for aw in snap.windows {
            if aw.always_on_top {
                crate::window::draw(&mut cv, snap.theme, aw, snap.cursor_visible, snap.explorers);
            }
        }
    }

    if !snap.fullscreen {
        let mut cv = comp.layer_canvas(Layer::Popups);
        crate::taskbar::draw(&mut cv, snap, clock_str);

        if snap.start_menu {
            crate::start_menu::draw(&mut cv, snap);
        }
    }

    // Overlay
    {
        let mut cv = comp.layer_canvas(Layer::Overlay);
        overlay::draw_context_menu(&mut cv, snap);
        overlay::draw_clipboard(&mut cv, snap);
        overlay::draw_notifications(&mut cv, snap);
        if let Some(s) = snap.settings {
            s.draw(&mut cv, snap);
        }
        if snap.switcher_active {
            overlay::draw_switcher(&mut cv, snap);
        }
    }

    comp.compose(win);
}
