#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{sarga_main, gui::Window, theme::Theme};

fn user_main() -> i32 {
    let mut win = Window::create("System Monitor", 500, 400).unwrap();
    let theme = Theme::dark();
    let mut history = [0u32; 50];
    let mut head = 0usize;

    loop {
        history[head] = (unsafe { libsarga::syscall::syscall0(1) } % 100) as u32; // Mock CPU
        head = (head + 1) % 50;

        win.clear(theme.bg_primary);
        win.draw_string(20, 20, "CPU Usage History", 0xFFFFFFFF, 0);

        // Draw graph
        for i in 0..49 {
            let idx = (head + i) % 50;
            let next = (head + i + 1) % 50;
            let y1 = 300 - history[idx] * 2;
            let y2 = 300 - history[next] * 2;
            let x1 = 20 + i as u32 * 9;
            let x2 = 20 + (i + 1) as u32 * 9;
            // Draw line
            for x in x1..x2 {
                let t = (x - x1) as f32 / (x2 - x1) as f32;
                let y = (y1 as f32 + t * (y2 as f32 - y1 as f32)) as u32;
                win.draw_pixel(x, y, theme.success);
            }
        }

        win.draw_string(20, 320, "Memory: 128MB / 512MB (25%)", theme.text_secondary, 0);
        win.draw_rounded_rect(20, 340, 460, 10, 5, theme.bg_elevated);
        win.draw_rounded_rect(20, 340, 115, 10, 5, theme.accent);

        let _ = win.flush();
        unsafe { libsarga::syscall::syscall1(35, 100_000_000u64); }
    }
}

sarga_main!(user_main);
