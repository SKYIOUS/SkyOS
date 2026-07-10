#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{sarga_main, gui::Window, theme::Theme};

fn user_main() -> i32 {
    let mut win = Window::create("Paint", 800, 600).unwrap();
    let theme = Theme::dark();
    let mut colors = [0xFFFFFFFF, 0xFFFF0000, 0xFF00FF00, 0xFF0000FF, 0xFFFFFF00, 0xFFFF00FF, 0xFF00FFFF, 0xFF000000];
    let mut selected_color = 0;

    win.clear(0xFFFFFFFF); // Canvas

    loop {
        let mouse = win.get_mouse();
        if mouse.buttons & 1 != 0 {
            if mouse.y > 60 {
                win.draw_rounded_rect(mouse.x as u32 - 4, mouse.y as u32 - 4, 8, 8, 4, colors[selected_color]);
            } else {
                // Toolbar click
                for i in 0..8 {
                    let cx = 10 + i as u32 * 50;
                    if mouse.x >= cx as u64 && mouse.x < (cx + 40) as u64 && mouse.y >= 10 && mouse.y < 50 {
                        selected_color = i;
                    }
                }
            }
        }

        // Draw Toolbar
        win.draw_rect(0, 0, 800, 60, theme.bg_surface);
        win.draw_line_h(0, 60, 800, theme.border);

        for i in 0..8 {
            let cx = 10 + i as u32 * 50;
            win.draw_rounded_rect(cx, 10, 40, 40, 6, colors[i]);
            if selected_color == i {
                win.draw_rounded_rect_outline(cx, 10, 40, 40, 6, theme.accent);
            }
        }

        let _ = win.flush();
        unsafe { libsarga::syscall::syscall1(35, 10_000_000u64); }
    }
}

sarga_main!(user_main);
