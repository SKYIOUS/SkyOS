#![allow(dead_code)]

use crate::render::compositor::Compositor;
use crate::render::layer::Layer;
use libsarga::io;

pub(crate) fn test_compositor_clear() -> bool {
    let mut comp = match Compositor::new(320, 200) {
        Some(c) => c,
        None => {
            io::print_str("[test] FAIL test_compositor_clear: buffer allocation failed\n");
            return false;
        }
    };
    {
        let mut canvas = comp.layer_canvas(Layer::Wallpaper);
        canvas.fill_pixel(10, 10, 0xFFFF0000);
    }
    // Read back and check
    let px = {
        let canvas = comp.layer_canvas(Layer::Wallpaper);
        canvas.data[10 * 320 + 10]
    };
    if px != 0xFFFF0000 {
        io::print_str("[test] FAIL test_compositor_clear: pixel write failed\n");
        return false;
    }
    comp.clear_all();
    let px2 = {
        let canvas = comp.layer_canvas(Layer::Wallpaper);
        canvas.data[10 * 320 + 10]
    };
    if px2 != 0 {
        io::print_str("[test] FAIL test_compositor_clear: clear_all did not zero pixel\n");
        return false;
    }
    io::print_str("[test] PASS test_compositor_clear\n");
    true
}

pub(crate) fn test_compositor_layers() -> bool {
    let mut comp = match Compositor::new(100, 100) {
        Some(c) => c,
        None => {
            io::print_str("[test] FAIL test_compositor_layers: buffer allocation failed\n");
            return false;
        }
    };

    // Write a different color into each layer (one at a time to avoid borrow conflicts)
    comp.layer_canvas(Layer::Wallpaper).fill_pixel(5, 5, 0xFF111111);
    comp.layer_canvas(Layer::Desktop).fill_pixel(5, 5, 0xFF222222);
    comp.layer_canvas(Layer::Windows).fill_pixel(5, 5, 0xFF333333);
    comp.layer_canvas(Layer::Popups).fill_pixel(5, 5, 0xFF444444);
    comp.layer_canvas(Layer::Overlay).fill_pixel(5, 5, 0xFF555555);
    comp.layer_canvas(Layer::Cursor).fill_pixel(5, 5, 0xFF666666);

    // Verify each layer (one at a time)
    let expected = [
        (Layer::Wallpaper, 0xFF111111u32),
        (Layer::Desktop, 0xFF222222),
        (Layer::Windows, 0xFF333333),
        (Layer::Popups, 0xFF444444),
        (Layer::Overlay, 0xFF555555),
        (Layer::Cursor, 0xFF666666),
    ];
    for &(layer, expected_color) in &expected {
        let canvas = comp.layer_canvas(layer);
        let actual = canvas.data[5 * 100 + 5];
        if actual != expected_color {
            io::print_str("[test] FAIL test_compositor_layers\n");
            return false;
        }
    }

    // Clear a single layer
    comp.clear_layer(Layer::Windows);
    let cleared = {
        let canvas = comp.layer_canvas(Layer::Windows);
        canvas.data[5 * 100 + 5]
    };
    if cleared != 0 {
        io::print_str("[test] FAIL test_compositor_layers: clear_layer did not work\n");
        return false;
    }

    io::print_str("[test] PASS test_compositor_layers\n");
    true
}
