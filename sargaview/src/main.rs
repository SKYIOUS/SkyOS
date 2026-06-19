#![no_std]
#![no_main]
extern crate alloc;
use alloc::vec::Vec;
use libsarga::sarga_main;
use libsarga::gui::Window;
use libsarga::io::{self, clipboard_write};
use libsarga::theme::Theme;
use libsarga::args;
use libsarga::png;

struct Image {
    width: u32,
    height: u32,
    pixels: Vec<u32>, // BGRA format
}

fn read_file(path: &str) -> Vec<u8> {
    let fd = match io::open(path, 0) {
        Ok(f) => f,
        Err(_) => return Vec::new(),
    };
    let mut data = Vec::new();
    let mut buf = [0u8; 8192];
    loop {
        match io::read(fd, &mut buf) {
            Ok(0) => break,
            Ok(n) => data.extend_from_slice(&buf[..n]),
            Err(_) => break,
        }
    }
    let _ = io::close(fd);
    data
}

fn u16_le(data: &[u8], off: usize) -> u16 {
    if off + 2 > data.len() { return 0; }
    data[off] as u16 | (data[off + 1] as u16) << 8
}

fn u32_le(data: &[u8], off: usize) -> u32 {
    if off + 4 > data.len() { return 0; }
    data[off] as u32 | (data[off + 1] as u32) << 8 | (data[off + 2] as u32) << 16 | (data[off + 3] as u32) << 24
}

fn i32_le(data: &[u8], off: usize) -> i32 {
    u32_le(data, off) as i32
}

fn parse_bmp(data: &[u8]) -> Option<Image> {
    if data.len() < 54 { return None; }
    if data[0] != b'B' || data[1] != b'M' { return None; }

    let pixel_offset = u32_le(data, 10) as usize;
    let header_size = u32_le(data, 14);
    let width = if header_size >= 12 { i32_le(data, 18) } else { return None };
    let height_raw = if header_size >= 12 { i32_le(data, 22) } else { return None };
    let bpp = if header_size >= 12 { u16_le(data, 28) } else { 24 };
    let compression = if header_size >= 40 { u32_le(data, 30) } else { 0 };

    if compression != 0 { return None; } // Only uncompressed BMP
    if bpp != 24 && bpp != 32 { return None; }

    let h = height_raw.abs() as u32;
    let w = width.abs() as u32;
    let row_stride = ((w * bpp as u32 + 31) / 32) * 4;
    let bottom_up = height_raw > 0;

    let bytes_pp = bpp as usize / 8;
    let mut pixels = Vec::with_capacity((w * h) as usize);

    for row in 0..h {
        let src_row = if bottom_up { h - 1 - row } else { row };
        let row_start = pixel_offset + src_row as usize * row_stride as usize;

        for col in 0..w {
            let px = row_start + col as usize * bytes_pp;
            if px + bytes_pp > data.len() {
                pixels.push(0xFF000000);
                continue;
            }
            let b = data[px] as u32;
            let g = data[px + 1] as u32;
            let r = data[px + 2] as u32;
            let a = if bytes_pp == 4 { data[px + 3] as u32 } else { 0xFF };
            // Store as 0xAARRGGBB (kernel format)
            pixels.push((a << 24) | (r << 16) | (g << 8) | b);
        }
    }

    Some(Image { width: w, height: h, pixels })
}

fn parse_ppm(data: &[u8]) -> Option<Image> {
    // P6 format: "P6\nWIDTH HEIGHT MAXVAL\n<pixel data>"
    if data.len() < 4 { return None; }
    if data[0] != b'P' || data[1] != b'6' { return None; }

    let mut offset = 3; // Skip "P6\n"
    // Skip comments
    while offset < data.len() && data[offset] == b'#' {
        while offset < data.len() && data[offset] != b'\n' { offset += 1; }
        offset += 1;
    }

    // Parse width
    let mut width: u32 = 0;
    while offset < data.len() && data[offset].is_ascii_digit() {
        width = width * 10 + (data[offset] - b'0') as u32;
        offset += 1;
    }
    offset += 1; // Skip whitespace

    // Parse height
    let mut height: u32 = 0;
    while offset < data.len() && data[offset].is_ascii_digit() {
        height = height * 10 + (data[offset] - b'0') as u32;
        offset += 1;
    }
    offset += 1; // Skip whitespace

    // Parse maxval (skip)
    while offset < data.len() && data[offset].is_ascii_digit() { offset += 1; }
    offset += 1; // Skip newline

    let pixel_data_len = width as usize * height as usize * 3;
    if offset + pixel_data_len > data.len() { return None; }

    let mut pixels = Vec::with_capacity((width * height) as usize);
    for _ in 0..(width * height) {
        let r = data[offset] as u32;
        let g = data[offset + 1] as u32;
        let b = data[offset + 2] as u32;
        pixels.push(0xFF000000 | (r << 16) | (g << 8) | b);
        offset += 3;
    }

    Some(Image { width, height, pixels })
}

fn parse_image(data: &[u8]) -> Option<Image> {
    // Try PNG
    if data.len() > 8 && data[0] == 0x89 && data[1] == b'P' && data[2] == b'N' && data[3] == b'G' {
        return png::decode_png(data).map(|p| Image { width: p.width, height: p.height, pixels: p.pixels });
    }
    // Try BMP
    if data.len() > 2 && data[0] == b'B' && data[1] == b'M' {
        return parse_bmp(data);
    }
    // Try PPM
    if data.len() > 3 && data[0] == b'P' && data[1] == b'6' {
        return parse_ppm(data);
    }
    // Try P3 (ASCII PPM)
    if data.len() > 3 && data[0] == b'P' && data[1] == b'3' {
        return parse_ppm_p3(data);
    }
    None
}

fn parse_ppm_p3(data: &[u8]) -> Option<Image> {
    // P3: "P3\nWIDTH HEIGHT MAXVAL\n<values separated by whitespace>"
    if data.len() < 4 { return None; }
    if data[0] != b'P' || data[1] != b'3' { return None; }

    let mut offset = 3;
    while offset < data.len() && data[offset] == b'#' {
        while offset < data.len() && data[offset] != b'\n' { offset += 1; }
        offset += 1;
    }

    let mut width: u32 = 0;
    while offset < data.len() && data[offset].is_ascii_digit() {
        width = width * 10 + (data[offset] - b'0') as u32;
        offset += 1;
    }
    offset += 1;

    let mut height: u32 = 0;
    while offset < data.len() && data[offset].is_ascii_digit() {
        height = height * 10 + (data[offset] - b'0') as u32;
        offset += 1;
    }
    offset += 1;

    while offset < data.len() && data[offset].is_ascii_digit() { offset += 1; }
    offset += 1;

    let mut pixels = Vec::with_capacity((width * height) as usize);
    let mut values_read = 0u32;
    let total = width * height * 3;
    let mut r = 0u32;
    let mut g = 0u32;

    while offset < data.len() && values_read < total {
        // Skip whitespace
        while offset < data.len() && (data[offset] == b' ' || data[offset] == b'\n' || data[offset] == b'\t') {
            offset += 1;
        }
        if offset >= data.len() { break; }

        let mut val: u32 = 0;
        while offset < data.len() && data[offset].is_ascii_digit() {
            val = val * 10 + (data[offset] - b'0') as u32;
            offset += 1;
        }

        match values_read % 3 {
            0 => r = val,
            1 => g = val,
            2 => {
                let b = val;
                let r8 = r.min(255);
                let g8 = g.min(255);
                let b8 = (b.min(255)) as u32;
                pixels.push(0xFF000000 | (r8 << 16) | (g8 << 8) | b8);
            }
            _ => unreachable!(),
        }
        values_read += 1;
    }

    Some(Image { width, height, pixels })
}

fn user_main() -> i32 {
    let theme = Theme::dark();

    let path = if args::argc() > 1 {
        args::get(1).unwrap_or("")
    } else {
        io::print_str("Usage: skyview <image.bmp|image.ppm>\n");
        io::print_str("Supported formats: PNG, BMP (24/32-bit), PPM (P3/P6)\n");
        return 0;
    };

    let data = read_file(path);
    if data.is_empty() {
        io::print_str(&alloc::format!("skyview: cannot read '{}'\n", path));
        return 0;
    }

    let image = match parse_image(&data) {
        Some(img) => img,
        None => {
            io::print_str(&alloc::format!("skyview: unsupported format '{}'\n", path));
            return 0;
        }
    };

    io::print_str(&alloc::format!("skyview: {}x{} image loaded\n", image.width, image.height));

    let win_w = (image.width + 16).min(1024);
    let win_h = (image.height + 40).min(768);

    let mut win = match Window::create("SARGA View", win_w, win_h) {
        Ok(w) => w,
        Err(e) => {
            io::print_str(&alloc::format!("skyview: window failed: {}\n", e));
            return 0;
        }
    };

    let mut zoom: f32 = 1.0;
    let mut scroll_x: i32 = 0;
    let mut scroll_y: i32 = 0;

    loop {
        let mouse = win.get_mouse();
        let scroll = mouse.scroll;

        // Zoom with scroll wheel
        if scroll > 0 { zoom = (zoom * 1.2).min(8.0); }
        if scroll < 0 { zoom = (zoom / 1.2).max(0.1); }

        // Keyboard
        while let Some(key) = win.get_key() {
            match key {
                b'q' | b'Q' => return 0,
                b'+' | b'=' => { zoom = (zoom * 1.2).min(8.0); }
                b'-' => { zoom = (zoom / 1.2).max(0.1); }
                b'0' => { zoom = 1.0; scroll_x = 0; scroll_y = 0; }
                b'c' | b'C' => {
                    // Copy image info
                    let info = alloc::format!("{}x{}", image.width, image.height);
                    clipboard_write(info.as_bytes());
                }
                _ => {}
            }
        }

        // Render
        win.clear(theme.bg_primary);

        let disp_w = (image.width as f32 * zoom) as u32;
        let disp_h = (image.height as f32 * zoom) as u32;
        let off_x = ((win_w - disp_w) / 2).max(0) as i32 + scroll_x;
        let off_y = ((win_h - 40 - disp_h) / 2).max(0) as i32 + scroll_y;

        // Draw image (nearest-neighbor scaling)
        for dy in 0..disp_h {
            for dx in 0..disp_w {
                let src_x = (dx as f32 / zoom) as u32;
                let src_y = (dy as f32 / zoom) as u32;
                if src_x < image.width && src_y < image.height {
                    let px = (src_y * image.width + src_x) as usize;
                    if px < image.pixels.len() {
                        let color = image.pixels[px];
                        let screen_x = off_x as u32 + dx;
                        let screen_y = off_y as u32 + dy;
                        if screen_x < win_w && screen_y < win_h {
                            let buf = win.buffer_mut();
                            let idx = screen_y as usize * win_w as usize + screen_x as usize;
                            if idx < buf.len() {
                                buf[idx] = color;
                            }
                        }
                    }
                }
            }
        }

        // Title bar info
        let info = alloc::format!("{}x{} | Zoom: {:.0}% | {}",
            image.width, image.height, zoom * 100.0, path);
        win.draw_rect(0, 0, win_w, 20, theme.bg_surface);
        win.draw_string(8, 4, &info, theme.text_secondary, theme.bg_surface);

        // Border
        if disp_w > 0 && disp_h > 0 {
            let bx = off_x.max(0) as u32;
            let by = off_y.max(0) as u32;
            win.draw_rect(bx.saturating_sub(1), by.saturating_sub(1), disp_w + 2, 1, theme.border);
            win.draw_rect(bx.saturating_sub(1), by + disp_h, disp_w + 2, 1, theme.border);
            win.draw_rect(bx.saturating_sub(1), by, 1, disp_h, theme.border);
            win.draw_rect(bx + disp_w, by, 1, disp_h, theme.border);
        }

        // Help text
        win.draw_rect(0, win_h - 20, win_w, 20, theme.bg_surface);
        win.draw_string(8, win_h - 16, "Q:Quit +/-:Zoom 0:Reset", theme.text_disabled, theme.bg_surface);

        let _ = win.flush();
        unsafe { libsarga::syscall::syscall1(35, 16_666_000); }
    }
    0
}

sarga_main!(user_main);
