use crate::syscall::*;
use crate::alloc::vec::Vec;
use crate::alloc::collections::BTreeMap;
use alloc::format;
use alloc::vec;

const SYS_GUI_CREATE_WINDOW: u64 = 100;
const SYS_GUI_FLUSH: u64 = 102;
const SYS_GUI_MAP_BUFFER: u64 = 103;
const SYS_GUI_GET_KEY: u64 = 105;

// ═════════════════════════════════════════════════════════════════════════════
// Glyph Cache
// ═════════════════════════════════════════════════════════════════════════════

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct CacheKey {
    glyph_id: u16,
    size: u16,
}

#[derive(Clone)]
pub struct CacheEntry {
    pub width: u16,
    pub height: u16,
    pub bearing_x: i16,
    pub bearing_y: i16,
    pub advance: u16,
    pub data: Vec<u8>,
}

pub struct GlyphCache {
    entries: BTreeMap<CacheKey, CacheEntry>,
    max_entries: usize,
}

impl GlyphCache {
    pub fn new(max_entries: usize) -> Self {
        GlyphCache { entries: BTreeMap::new(), max_entries }
    }

    pub fn get(&self, glyph_id: u16, size: u16) -> Option<&CacheEntry> {
        self.entries.get(&CacheKey { glyph_id, size })
    }

    pub fn insert(&mut self, glyph_id: u16, size: u16, entry: CacheEntry) {
        if self.entries.len() >= self.max_entries {
            if let Some(first_key) = self.entries.keys().next().copied() {
                self.entries.remove(&first_key);
            }
        }
        self.entries.insert(CacheKey { glyph_id, size }, entry);
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// TTF Outline Builder & Rasterizer
// ═════════════════════════════════════════════════════════════════════════════

struct OutlinePoint {
    x: f32,
    y: f32,
}

struct OutlineContour {
    points: Vec<OutlinePoint>,
}

struct OutlineBuilder {
    contours: Vec<OutlineContour>,
    current: Vec<OutlinePoint>,
    first: Option<OutlinePoint>,
}

impl OutlineBuilder {
    fn new() -> Self {
        OutlineBuilder { contours: Vec::new(), current: Vec::new(), first: None }
    }

    fn push(&mut self, p: OutlinePoint) {
        self.current.push(p);
    }

    fn finish_contour(&mut self) {
        if !self.current.is_empty() {
            let c = core::mem::replace(&mut self.current, Vec::new());
            self.contours.push(OutlineContour { points: c });
        }
        self.first = None;
    }
}

impl ttf_parser::OutlineBuilder for OutlineBuilder {
    fn move_to(&mut self, x: f32, y: f32) {
        self.finish_contour();
        let p = OutlinePoint { x, y };
        self.first = Some(OutlinePoint { x, y });
        self.push(p);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.push(OutlinePoint { x, y });
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        // Subdivide quadratic bezier into line segments
        let last = self.current.last().map(|p| (p.x, p.y)).unwrap_or((0.0, 0.0));
        let steps = 8u32;
        for i in 1..=steps {
            let t = i as f32 / steps as f32;
            let t0 = (1.0 - t) * (1.0 - t);
            let t1 = 2.0 * t * (1.0 - t);
            let t2 = t * t;
            let px = t0 * last.0 + t1 * x1 + t2 * x;
            let py = t0 * last.1 + t1 * y1 + t2 * y;
            self.push(OutlinePoint { x: px, y: py });
        }
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let last = self.current.last().map(|p| (p.x, p.y)).unwrap_or((0.0, 0.0));
        let steps = 12u32;
        for i in 1..=steps {
            let t = i as f32 / steps as f32;
            let t0 = (1.0 - t) * (1.0 - t) * (1.0 - t);
            let t1 = 3.0 * t * (1.0 - t) * (1.0 - t);
            let t2 = 3.0 * t * t * (1.0 - t);
            let t3 = t * t * t;
            let px = t0 * last.0 + t1 * x1 + t2 * x2 + t3 * x;
            let py = t0 * last.1 + t1 * y1 + t2 * y2 + t3 * y;
            self.push(OutlinePoint { x: px, y: py });
        }
    }

    fn close(&mut self) {
        if let Some(ref first) = self.first {
            // Only add closing point if current endpoint differs from first
            let should_close = self.current.last().map(|last| {
                (last.x - first.x).abs() > 0.001 || (last.y - first.y).abs() > 0.001
            }).unwrap_or(false);
            if should_close {
                self.push(OutlinePoint { x: first.x, y: first.y });
            }
        }
        self.finish_contour();
    }
}

fn outline_rasterize(contours: &[OutlineContour], scale: f32, width: u32, height: u32) -> Vec<u8> {
    let mut bitmap = vec![0u8; (width * height) as usize];
    if width == 0 || height == 0 { return bitmap; }

    // Even-odd scanline rasterization
    for y in 0..height {
        let yf = y as f32;
        // Collect x intersections for this scanline
        let mut hits: Vec<f32> = Vec::new();
        for contour in contours {
            for window in contour.points.windows(2) {
                let (x1, y1) = (window[0].x * scale, window[0].y * scale);
                let (x2, y2) = (window[1].x * scale, window[1].y * scale);
                // Check if this edge crosses the scanline
                let (ymin, ymax) = if y1 < y2 { (y1, y2) } else { (y2, y1) };
                if yf >= ymin && yf < ymax && (ymax - ymin).abs() > 0.001 {
                    let t = (yf - ymin) / (ymax - ymin);
                    let x_intersect = if y1 < y2 { x1 + t * (x2 - x1) } else { x2 + t * (x1 - x2) };
                    hits.push(x_intersect);
                }
            }
        }
        // Sort and fill pairs
        hits.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));
        let mut i = 0;
        while i + 1 < hits.len() {
            let x_start = hits[i].max(0.0).min(width as f32);
            let x_end = hits[i + 1].max(0.0).min(width as f32);
            if x_end > x_start {
                let sx = x_start as usize;
                let ex = x_end as usize;
                let row_start = y as usize * width as usize;
                for px in sx..ex.min(width as usize) {
                    bitmap[row_start + px] = 255;
                }
            }
            i += 2;
        }
    }
    bitmap
}

// ═════════════════════════════════════════════════════════════════════════════
// TTF Font (full renderer via ttf-parser outline)
// ═════════════════════════════════════════════════════════════════════════════

pub struct TtfFont {
    #[allow(dead_code)]
    data: Vec<u8>,
    font: ttf_parser::Face<'static>,
    cache: GlyphCache,
}

unsafe impl Send for TtfFont {}

impl TtfFont {
    pub fn from_bytes(data: Vec<u8>) -> Option<Self> {
        let font = unsafe {
            core::mem::transmute::<ttf_parser::Face<'_>, ttf_parser::Face<'static>>(
                ttf_parser::Face::parse(&data, 0).ok()?
            )
        };
        Some(TtfFont { data, font, cache: GlyphCache::new(2048) })
    }

    pub fn glyph_id(&self, ch: char) -> Option<u16> {
        self.font.glyph_index(ch).map(|g| g.0)
    }

    pub fn advance(&self, glyph_id: u16, size: u32) -> u32 {
        let units_per_em = self.font.units_per_em() as f32;
        let scale = size as f32 / units_per_em;
        let advance = self.font.glyph_hor_advance(ttf_parser::GlyphId(glyph_id)).unwrap_or(0);
        (advance as f32 * scale) as u32
    }

    pub fn bounding_box(&self, glyph_id: u16, size: u32) -> (i32, i32, u32, u32) {
        let units_per_em = self.font.units_per_em() as f32;
        let scale = size as f32 / units_per_em;
        if let Some(bbox) = self.font.glyph_bounding_box(ttf_parser::GlyphId(glyph_id)) {
            let x_min = (bbox.x_min as f32 * scale) as i32;
            let y_min = (bbox.y_min as f32 * scale) as i32;
            let x_max = (bbox.x_max as f32 * scale) as i32;
            let y_max = (bbox.y_max as f32 * scale) as i32;
            (x_min, y_min, (x_max - x_min).max(0) as u32, (y_max - y_min).max(0) as u32)
        } else {
            (0, 0, 0, 0)
        }
    }

    pub fn units_per_em(&self) -> u16 {
        self.font.units_per_em()
    }

    pub fn ascender(&self, size: u32) -> i32 {
        let upm = self.font.units_per_em() as f32;
        (self.font.ascender() as f32 * size as f32 / upm) as i32
    }

    pub fn descender(&self, size: u32) -> i32 {
        let upm = self.font.units_per_em() as f32;
        (self.font.descender() as f32 * size as f32 / upm) as i32
    }

    /// Get the glyph bitmap for rendering. Returns (width, height, bearing_x, bearing_y, advance, alpha_data)
    pub fn render_glyph(&mut self, ch: char, size: u32) -> Option<(u32, u32, i32, i32, u32, Vec<u8>)> {
        let gid = self.glyph_id(ch)?;
        let advance = self.advance(gid, size);
        let (bx, by, w, h) = self.bounding_box(gid, size);

        // Check cache
        if let Some(cached) = self.cache.get(gid, size as u16) {
            return Some((cached.width as u32, cached.height as u32, cached.bearing_x as i32, cached.bearing_y as i32, cached.advance as u32, cached.data.clone()));
        }

        let upm = self.font.units_per_em() as f32;
        let scale = size as f32 / upm;

        // Build outline
        let mut builder = OutlineBuilder::new();
        self.font.outline_glyph(ttf_parser::GlyphId(gid), &mut builder);
        builder.finish_contour();

        let gw = w.max(1) as u32;
        let gh = h.max(1) as u32;
        let data = outline_rasterize(&builder.contours, scale, gw, gh);

        // Cache the result
        self.cache.insert(gid, size as u16, CacheEntry {
            width: gw as u16,
            height: gh as u16,
            bearing_x: bx as i16,
            bearing_y: by as i16,
            advance: advance as u16,
            data: data.clone(),
        });

        Some((gw, gh, bx, by, advance, data))
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// Font Abstraction
// ═════════════════════════════════════════════════════════════════════════════

pub enum Font {
    Ttf(TtfFont),
    Bitmap,
}

impl Font {
    pub fn from_ttf(data: Vec<u8>) -> Option<Self> {
        TtfFont::from_bytes(data).map(Font::Ttf)
    }

    pub fn bitmap() -> Self { Font::Bitmap }

    pub fn advance(&self, ch: char, size: u32) -> u32 {
        match self {
            Font::Ttf(f) => {
                if let Some(gid) = f.glyph_id(ch) {
                    f.advance(gid, size)
                } else {
                    size * 6 / 10
                }
            }
            Font::Bitmap => 8,
        }
    }

    pub fn text_width(&self, text: &str, size: u32) -> u32 {
        text.chars().map(|c| self.advance(c, size)).sum()
    }

    pub fn line_height(&self, size: u32) -> u32 {
        match self {
            Font::Ttf(f) => {
                let asc = f.ascender(size).max(0) as u32;
                let desc = f.descender(size).abs() as u32;
                asc + desc + size / 6
            }
            Font::Bitmap => size,
        }
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// Color utilities
// ═════════════════════════════════════════════════════════════════════════════

#[inline]
pub fn alpha_blend(bg: u32, fg: u32, alpha: u8) -> u32 {
    if alpha == 0 { return bg; }
    if alpha == 255 { return fg; }
    let a = alpha as u32;
    let inv_a = 255 - a;
    let r = (((fg >> 16) & 0xFF) * a + ((bg >> 16) & 0xFF) * inv_a) / 255;
    let g = (((fg >> 8) & 0xFF) * a + ((bg >> 8) & 0xFF) * inv_a) / 255;
    let b = ((fg & 0xFF) * a + (bg & 0xFF) * inv_a) / 255;
    (r << 16) | (g << 8) | b
}

// ═════════════════════════════════════════════════════════════════════════════
// Window
// ═════════════════════════════════════════════════════════════════════════════

pub struct Window {
    id: u64,
    pub width: u32,
    pub height: u32,
    buffer: &'static mut [u32],
    font: Option<Font>,
    font_size: u32,
}

impl Window {
    pub fn create(title: &str, width: u32, height: u32) -> Result<Self, i64> {
        let title_c = format!("{}\0", title);
        let id = unsafe { syscall3(SYS_GUI_CREATE_WINDOW,
            title_c.as_ptr() as u64, width as u64, height as u64) };
        if id < 0 { return Err(-id); }
        let id = id as u64;

        let buf_ptr = unsafe { syscall1(SYS_GUI_MAP_BUFFER, id) } as *mut u32;
        if buf_ptr.is_null() { return Err(5); }
        let len = (width * height) as usize;
        let buffer = unsafe { core::slice::from_raw_parts_mut(buf_ptr, len) };
        Ok(Window { id, width, height, buffer, font: None, font_size: 14 })
    }

    pub fn set_font(&mut self, font: Font) { self.font = Some(font); }
    pub fn set_font_size(&mut self, size: u32) { self.font_size = size; }
    pub fn font_size(&self) -> u32 { self.font_size }
    pub fn buffer_mut(&mut self) -> &mut [u32] { self.buffer }

    pub fn get_key(&mut self) -> Option<u8> {
        let k = unsafe { syscall1(SYS_GUI_GET_KEY, self.id) };
        if k == 0 { None } else { Some(k as u8) }
    }

    pub fn flush(&self) -> Result<(), i64> {
        let ret = unsafe { syscall1(SYS_GUI_FLUSH, self.id) };
        if ret < 0 { Err(-ret) } else { Ok(()) }
    }

    pub fn fill(&mut self, color: u32) {
        for px in self.buffer.iter_mut() { *px = color; }
    }

    pub fn draw_rect(&mut self, x: u32, y: u32, w: u32, h: u32, color: u32) {
        let sw = self.width as usize;
        let sh = self.height as usize;
        let x0 = x.min(sw as u32) as usize;
        let y0 = y.min(sh as u32) as usize;
        let x1 = (x + w).min(sw as u32) as usize;
        let y1 = (y + h).min(sh as u32) as usize;
        for py in y0..y1 {
            let row = py * sw;
            for px in x0..x1 {
                self.buffer[row + px] = color;
            }
        }
    }

    pub fn draw_rect_alpha(&mut self, x: u32, y: u32, w: u32, h: u32, color: u32) {
        let a = ((color >> 24) & 0xFF) as u8;
        if a == 0 { return 0; }
        if a == 255 { self.draw_rect(x, y, w, h, color); return 0; }
        let sw = self.width as usize;
        let sh = self.height as usize;
        let x0 = x.min(sw as u32) as usize;
        let y0 = y.min(sh as u32) as usize;
        let x1 = (x + w).min(sw as u32) as usize;
        let y1 = (y + h).min(sh as u32) as usize;
        for py in y0..y1 {
            let row = py * sw;
            for px in x0..x1 {
                self.buffer[row + px] = alpha_blend(self.buffer[row + px], color, a);
            }
        }
    }

    pub fn draw_rounded_rect(&mut self, x: u32, y: u32, w: u32, h: u32, radius: u32, color: u32) {
        let sw = self.width as usize;
        let sh = self.height as usize;
        let r = radius as i32;
        for dy in 0..h {
            for dx in 0..w {
                let px = (x + dx) as i32;
                let py = (y + dy) as i32;
                if px < 0 || py < 0 || px >= sw as i32 || py >= sh as i32 { continue; }

                let mut skip = false;
                if (dx as i32) < r && (dy as i32) < r {
                    let cx = r - dx as i32;
                    let cy = r - dy as i32;
                    if cx * cx + cy * cy > r * r { skip = true; }
                } else if (dx as i32) >= (w as i32 - r) && (dy as i32) < r {
                    let cx = dx as i32 - (w as i32 - r - 1);
                    let cy = r - dy as i32;
                    if cx * cx + cy * cy > r * r { skip = true; }
                } else if (dx as i32) < r && (dy as i32) >= (h as i32 - r) {
                    let cx = r - dx as i32;
                    let cy = dy as i32 - (h as i32 - r - 1);
                    if cx * cx + cy * cy > r * r { skip = true; }
                } else if (dx as i32) >= (w as i32 - r) && (dy as i32) >= (h as i32 - r) {
                    let cx = dx as i32 - (w as i32 - r - 1);
                    let cy = dy as i32 - (h as i32 - r - 1);
                    if cx * cx + cy * cy > r * r { skip = true; }
                }

                if !skip {
                    self.buffer[py as usize * sw + px as usize] = color;
                }
            }
        }
    }

    pub fn draw_line_h(&mut self, x: u32, y: u32, w: u32, color: u32) {
        let sw = self.width as usize;
        let sh = self.height as usize;
        let x0 = x.min(sw as u32) as usize;
        let x1 = (x + w).min(sw as u32) as usize;
        if y as usize >= sh { return 0; }
        let row = y as usize * sw;
        for px in x0..x1 { self.buffer[row + px] = color; }
    }

    pub fn draw_line_v(&mut self, x: u32, y: u32, h: u32, color: u32) {
        let sw = self.width as usize;
        let sh = self.height as usize;
        if x as usize >= sw { return 0; }
        let y0 = y.min(sh as u32) as usize;
        let y1 = (y + h).min(sh as u32) as usize;
        for py in y0..y1 { self.buffer[py * sw + x as usize] = color; }
    }

    pub fn fill_rect(&mut self, x: u32, y: u32, w: u32, h: u32, color: u32) {
        self.draw_rect(x, y, w, h, color);
    }

    pub fn clear(&mut self, color: u32) {
        for px in self.buffer.iter_mut() { *px = color; }
    }

    pub fn pixel(&self, x: u32, y: u32) -> Option<u32> {
        if x < self.width && y < self.height {
            Some(self.buffer[(y * self.width + x) as usize])
        } else { None }
    }

    pub fn draw_pixel(&mut self, x: u32, y: u32, color: u32) {
        if x < self.width && y < self.height {
            self.buffer[(y * self.width + x) as usize] = color;
        }
    }

    pub fn draw_char(&mut self, x: u32, y: u32, c: char, fg: u32, bg: u32) {
        use font8x8::{UnicodeFonts, BASIC_FONTS};
        if let Some(glyph) = BASIC_FONTS.get(c) {
            for (row, &byte) in glyph.iter().enumerate() {
                for col in 0..8 {
                    let px = x + col;
                    let py = y + row as u32;
                    if px < self.width && py < self.height {
                        if (byte >> (7 - col)) & 1 != 0 {
                            self.buffer[(py * self.width + px) as usize] = fg;
                        } else if bg != 0 {
                            self.buffer[(py * self.width + px) as usize] = bg;
                        }
                    }
                }
            }
        }
    }

    pub fn draw_string(&mut self, x: u32, y: u32, s: &str, fg: u32, bg: u32) {
        let font_size = self.font_size;
        let is_ttf = self.font.is_some();
        if is_ttf {
            let mut cx = x;
            for c in s.chars() {
                self.draw_char_scaled(cx, y, c, fg, bg, font_size);
                let advance = match &self.font {
                    Some(Font::Ttf(f)) => {
                        if let Some(gid) = f.glyph_id(c) { f.advance(gid, font_size) } else { font_size * 6 / 10 }
                    }
                    _ => 8,
                };
                cx += advance;
            }
        } else {
            for (i, c) in s.chars().enumerate() {
                self.draw_char(x + i as u32 * 8, y, c, fg, bg);
            }
        }
    }

    /// Draw a character at the given font size
    fn draw_char_scaled(&mut self, x: u32, y: u32, c: char, fg: u32, bg: u32, size: u32) {
        if let Some(Font::Ttf(ref mut font)) = self.font {
            if let Some((gw, gh, bx, by, _advance, data)) = font.render_glyph(c, size) {
                let ox = x.wrapping_add(bx.max(0) as u32);
                let oy = y.wrapping_add((font.ascender(size) - by).max(0) as u32);
                let sw = self.width as usize;
                let sh = self.height as usize;
                for row in 0..gh as usize {
                    if oy as usize + row >= sh { break; }
                    for col in 0..gw as usize {
                        if ox as usize + col >= sw { break; }
                        let alpha = data[row * gw as usize + col];
                        if alpha > 0 {
                            let idx = (oy as usize + row) * sw + (ox as usize + col);
                            if alpha == 255 {
                                self.buffer[idx] = fg;
                            } else {
                                let a = alpha as u32;
                                let inv_a = 255 - a;
                                let old = self.buffer[idx];
                                let r = (((old >> 16) & 0xFF) * inv_a + ((fg >> 16) & 0xFF) * a) / 255;
                                let g = (((old >> 8) & 0xFF) * inv_a + ((fg >> 8) & 0xFF) * a) / 255;
                                let b = ((old & 0xFF) * inv_a + (fg & 0xFF) * a) / 255;
                                self.buffer[idx] = (0xFF << 24) | (r << 16) | (g << 8) | b;
                            }
                        } else if bg != 0 {
                            self.buffer[(oy as usize + row) * sw + (ox as usize + col)] = bg;
                        }
                    }
                }
                return 0;
            }
        }
        // Fallback to font8x8
        use font8x8::{UnicodeFonts, BASIC_FONTS};
        if let Some(glyph) = BASIC_FONTS.get(c) {
            let scale = (size / 8).max(1);
            for (row, &byte) in glyph.iter().enumerate() {
                for col in 0..8 {
                    let px = x + col * scale;
                    let py = y + row as u32 * scale;
                    if (byte >> (7 - col)) & 1 != 0 {
                        self.draw_rect(px, py, scale, scale, fg);
                    } else if bg != 0 {
                        self.draw_rect(px, py, scale, scale, bg);
                    }
                }
            }
        }
    }

    pub fn draw_string_centered(&mut self, y: u32, s: &str, fg: u32, bg: u32) {
        let w = match &self.font {
            Some(f) => f.text_width(s, self.font_size),
            None => s.len() as u32 * 8,
        };
        let x = self.width.saturating_sub(w) / 2;
        self.draw_string(x, y, s, fg, bg);
    }

    pub fn draw_string_shadow(&mut self, x: u32, y: u32, s: &str, fg: u32, bg: u32, shadow: u32) {
        self.draw_string(x + 1, y + 1, s, shadow, 0);
        self.draw_string(x, y, s, fg, bg);
    }

    pub fn measure_text(&self, s: &str) -> u32 {
        match &self.font {
            Some(f) => f.text_width(s, self.font_size),
            None => s.len() as u32 * 8,
        }
    }

    pub fn get_mouse(&self) -> crate::io::MouseState {
        crate::io::get_mouse(self.id)
    }

    pub fn set_title(&mut self, title: &str) {
        crate::io::set_title(self.id, title);
    }

    pub fn destroy(self) {
        crate::io::destroy_window(self.id);
    }

    pub fn resize(&mut self, width: u64, height: u64) {
        crate::io::resize_window(self.id, width, height);
        self.width = width as u32;
        self.height = height as u32;
    }

    pub fn move_to(&mut self, x: u64, y: u64) {
        crate::io::move_window(self.id, x, y);
    }
}
