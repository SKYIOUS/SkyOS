use alloc::vec::Vec;
use miniz_oxide::inflate::decompress_to_vec_zlib;

const PNG_SIG: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];

#[derive(Clone, Copy)]
pub enum ColorType {
    Grayscale = 0,
    Rgb = 2,
    Indexed = 3,
    GrayscaleAlpha = 4,
    Rgba = 6,
}

pub struct PngImage {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u32>,
}

fn u32_be(data: &[u8], off: usize) -> u32 {
    if off + 4 > data.len() { return 0; }
    (data[off] as u32) << 24 | (data[off + 1] as u32) << 16
        | (data[off + 2] as u32) << 8 | data[off + 3] as u32
}

fn paeth_predictor(a: u8, b: u8, c: u8) -> u8 {
    let a = a as i16;
    let b = b as i16;
    let c = c as i16;
    let p = a + b - c;
    let pa = (p - a).abs();
    let pb = (p - b).abs();
    let pc = (p - c).abs();
    if pa <= pb && pa <= pc { a as u8 }
    else if pb <= pc { b as u8 }
    else { c as u8 }
}

pub fn decode_png(data: &[u8]) -> Option<PngImage> {
    if data.len() < 8 { return None; }
    if &data[..8] != PNG_SIG { return None; }

    let mut pos = 8;
    let mut width = 0u32;
    let mut height = 0u32;
    let mut color_type = ColorType::Rgba;
    let mut palette: Vec<[u8; 4]> = Vec::new();
    let mut idat_chunks: Vec<Vec<u8>> = Vec::new();
    let mut has_ihdr = false;

    while pos + 8 <= data.len() {
        let chunk_len = u32_be(data, pos) as usize;
        let chunk_type = &data[pos + 4..pos + 8];
        let chunk_data_start = pos + 8;
        let chunk_data_end = chunk_data_start + chunk_len;
        if chunk_data_end > data.len() { return None; }

        let type_str = core::str::from_utf8(chunk_type).unwrap_or("");
        match type_str {
            "IHDR" => {
                if chunk_len < 13 { return None; }
                width = u32_be(data, chunk_data_start);
                height = u32_be(data, chunk_data_start + 4);
                let bit_depth = data[chunk_data_start + 8];
                let ct = data[chunk_data_start + 9];
                if bit_depth != 8 { return None; }
                color_type = match ct {
                    0 => ColorType::Grayscale,
                    2 => ColorType::Rgb,
                    3 => ColorType::Indexed,
                    4 => ColorType::GrayscaleAlpha,
                    6 => ColorType::Rgba,
                    _ => return None,
                };
                has_ihdr = true;
            }
            "PLTE" => {
                if chunk_len % 3 != 0 { return None; }
                for i in 0..chunk_len / 3 {
                    let off = chunk_data_start + i * 3;
                    palette.push([data[off], data[off + 1], data[off + 2], 0xFF]);
                }
            }
            "tRNS" => {
                for i in 0..chunk_len.min(palette.len()) {
                    palette[i][3] = data[chunk_data_start + i];
                }
            }
            "IDAT" => {
                idat_chunks.push(data[chunk_data_start..chunk_data_end].to_vec());
            }
            "IEND" => { break; }
            _ => {}
        }

        pos = chunk_data_end + 4;
    }

    if !has_ihdr { return None; }
    if idat_chunks.is_empty() { return None; }

    let mut compressed = Vec::new();
    for chunk in &idat_chunks {
        compressed.extend_from_slice(chunk);
    }

    let raw = decompress_to_vec_zlib(&compressed).ok()?;

    let bytes_per_pixel = match color_type {
        ColorType::Grayscale => 1,
        ColorType::GrayscaleAlpha => 2,
        ColorType::Rgb => 3,
        ColorType::Indexed => 1,
        ColorType::Rgba => 4,
    };

    let row_len = 1 + width as usize * bytes_per_pixel;
    let expected = row_len * height as usize;
    if raw.len() < expected { return None; }

    let mut pixels = Vec::with_capacity((width * height) as usize);
    let mut prev_row: Vec<u8> = alloc::vec![0u8; width as usize * bytes_per_pixel];

    for y in 0..height as usize {
        let off = y * row_len;
        let filter = raw[off];
        let row = &raw[off + 1..off + row_len];

        let mut unfiltered = Vec::with_capacity(width as usize * bytes_per_pixel);

        for x in 0..(width as usize * bytes_per_pixel) {
            let a = if x >= bytes_per_pixel { unfiltered[x - bytes_per_pixel] } else { 0 };
            let b = prev_row[x];
            let c = if x >= bytes_per_pixel { prev_row[x - bytes_per_pixel] } else { 0 };

            let val = match filter {
                0 => row[x],
                1 => row[x].wrapping_add(a),
                2 => row[x].wrapping_add(b),
                3 => row[x].wrapping_add(((a as u16 + b as u16) / 2) as u8),
                4 => row[x].wrapping_add(paeth_predictor(a, b, c)),
                _ => return None,
            };
            unfiltered.push(val);
        }

        // Convert to RGBA
        match color_type {
            ColorType::Rgba => {
                for x in 0..width as usize {
                    let off = x * 4;
                    let r = unfiltered[off];
                    let g = unfiltered[off + 1];
                    let b = unfiltered[off + 2];
                    let a = unfiltered[off + 3];
                    pixels.push((a as u32) << 24 | (r as u32) << 16 | (g as u32) << 8 | b as u32);
                }
            }
            ColorType::Rgb => {
                for x in 0..width as usize {
                    let off = x * 3;
                    let r = unfiltered[off];
                    let g = unfiltered[off + 1];
                    let b = unfiltered[off + 2];
                    pixels.push(0xFF000000 | (r as u32) << 16 | (g as u32) << 8 | b as u32);
                }
            }
            ColorType::Indexed => {
                for x in 0..width as usize {
                    let idx = unfiltered[x] as usize;
                    if idx < palette.len() {
                        let rgba = palette[idx];
                        pixels.push((rgba[3] as u32) << 24 | (rgba[0] as u32) << 16 | (rgba[1] as u32) << 8 | rgba[2] as u32);
                    } else {
                        pixels.push(0xFFFF00FF);
                    }
                }
            }
            ColorType::Grayscale => {
                for x in 0..width as usize {
                    let g = unfiltered[x];
                    pixels.push(0xFF000000 | (g as u32) << 16 | (g as u32) << 8 | g as u32);
                }
            }
            ColorType::GrayscaleAlpha => {
                for x in 0..width as usize {
                    let off = x * 2;
                    let g = unfiltered[off];
                    let a = unfiltered[off + 1];
                    pixels.push((a as u32) << 24 | (g as u32) << 16 | (g as u32) << 8 | g as u32);
                }
            }
        }

        prev_row = unfiltered;
    }

    Some(PngImage { width, height, pixels })
}
