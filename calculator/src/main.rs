#![no_std]
#![no_main]
extern crate alloc;
use libsarga::sarga_main;
use libsarga::gui::Window;
use libsarga::io;
use libsarga::theme::Theme;

struct Calculator {
    display: [u8; 32],
    display_len: usize,
    operand: f64,
    operator: u8,
    new_number: bool,
    #[allow(dead_code)]
    memory: f64,
    error: bool,
}

impl Calculator {
    fn new() -> Self {
        Calculator {
            display: [0; 32],
            display_len: 1,
            operand: 0.0,
            operator: b'=',
            new_number: true,
            memory: 0.0,
            error: false,
        }
    }

    fn set_display(&mut self, val: f64) {
        if val.is_infinite() || val.is_nan() {
            self.display[0..5].copy_from_slice(b"Error");
            self.display_len = 5;
            self.error = true;
            return;
        }
        self.error = false;
        let int_part = val.abs() as u64;
        let frac_part = ((val.abs() - int_part as f64) * 1_000_000.0) as u64;
        let negative = val < 0.0 && val != 0.0;
        let mut buf = [0u8; 24];
        let mut len = 0;
        if negative { buf[len] = b'-'; len += 1; }
        // Integer part
        if int_part == 0 {
            buf[len] = b'0'; len += 1;
        } else {
            let mut digits = [0u8; 20];
            let mut n = int_part;
            let mut d = 0;
            while n > 0 { digits[d] = (n % 10) as u8; n /= 10; d += 1; }
            while d > 0 { d -= 1; buf[len] = b'0' + digits[d]; len += 1; }
        }
        // Fractional part
        if frac_part > 0 {
            buf[len] = b'.'; len += 1;
            let mut frac = [0u8; 6];
            let mut n = frac_part;
            let mut d = 6;
            while d > 0 && n > 0 { d -= 1; frac[d] = (n % 10) as u8; n /= 10; }
            // Remove trailing zeros
            while d < 6 && frac[5] == 0 { /* keep as is */ break; }
            let mut end = 6;
            while end > d + 1 && frac[end - 1] == 0 { end -= 1; }
            for i in d..end { buf[len] = b'0' + frac[i]; len += 1; }
        }
        self.display_len = len;
        for i in 0..len { self.display[i] = buf[i]; }
    }

    fn push_digit(&mut self, d: u8) {
        if self.error { return; }
        if self.new_number {
            self.display_len = 0;
            self.new_number = false;
        }
        if self.display_len < 20 {
            self.display[self.display_len] = b'0' + d;
            self.display_len += 1;
        }
    }

    fn push_dot(&mut self) {
        if self.error { return; }
        if self.new_number {
            self.display[0] = b'0';
            self.display_len = 1;
            self.new_number = false;
        }
        // Check if dot already exists
        for i in 0..self.display_len {
            if self.display[i] == b'.' { return; }
        }
        self.display[self.display_len] = b'.';
        self.display_len += 1;
    }

    fn current_value(&self) -> f64 {
        let s = core::str::from_utf8(&self.display[..self.display_len]).unwrap_or("0");
        let mut result: f64 = 0.0;
        let mut decimal_pos = s.len();
        let mut negative = false;
        for (i, &b) in s.as_bytes().iter().enumerate() {
            if b == b'-' { negative = true; }
            else if b == b'.' { decimal_pos = i + 1; }
        }
        // Parse integer part
        let int_end = if decimal_pos <= s.len() { decimal_pos - 1 } else { s.len() };
        let mut power = 1.0;
        for i in (0..int_end).rev() {
            let b = s.as_bytes()[i];
            if b >= b'0' && b <= b'9' {
                result += (b - b'0') as f64 * power;
                power *= 10.0;
            }
        }
        // Parse fractional part
        if decimal_pos < s.len() {
            let mut frac_val: f64 = 0.0;
            let mut frac_power = 0.1;
            for i in decimal_pos..s.len() {
                let b = s.as_bytes()[i];
                if b >= b'0' && b <= b'9' {
                    frac_val += (b - b'0') as f64 * frac_power;
                    frac_power /= 10.0;
                }
            }
            result += frac_val;
        }
        if negative { -result } else { result }
    }

    fn calculate(&mut self) {
        if self.error { return; }
        let b = self.current_value();
        let result = match self.operator {
            b'+' => self.operand + b,
            b'-' => self.operand - b,
            b'*' => self.operand * b,
            b'/' => if b != 0.0 { self.operand / b } else { f64::INFINITY },
            _ => b,
        };
        self.set_display(result);
        self.operator = b'=';
        self.new_number = true;
    }

    fn op(&mut self, op: u8) {
        if self.error { return; }
        if self.operator != b'=' && !self.new_number {
            self.calculate();
        } else {
            self.operand = self.current_value();
        }
        self.operator = op;
        self.new_number = true;
    }

    fn equals(&mut self) { self.calculate(); }

    fn clear(&mut self) {
        self.display_len = 1;
        self.display[0] = b'0';
        self.operand = 0.0;
        self.operator = b'=';
        self.new_number = true;
        self.error = false;
    }

    fn backspace(&mut self) {
        if self.error || self.new_number { return; }
        if self.display_len > 1 {
            self.display_len -= 1;
        } else {
            self.display[0] = b'0';
            self.display_len = 1;
            self.new_number = true;
        }
    }

    fn negate(&mut self) {
        if self.error { return; }
        if self.display_len > 0 && self.display[0] == b'-' {
            for i in 1..self.display_len {
                self.display[i - 1] = self.display[i];
            }
            self.display_len -= 1;
        } else if self.display_len > 0 && self.display_len < 20 {
            for i in (1..=self.display_len).rev() {
                self.display[i] = self.display[i - 1];
            }
            self.display[0] = b'-';
            self.display_len += 1;
        }
    }

    fn percent(&mut self) {
        let v = self.current_value() / 100.0;
        self.set_display(v);
        self.new_number = true;
    }
}

// Button grid: 4 columns x 5 rows
const BTN_LABELS: [[&str; 4]; 5] = [
    ["C", "±", "%", "÷"],
    ["7", "8", "9", "×"],
    ["4", "5", "6", "−"],
    ["1", "2", "3", "+"],
    ["0", ".", " ", "="],
];

const BTN_W: u32 = 60;
const BTN_H: u32 = 44;
const BTN_PAD: u32 = 4;
const GRID_X: u32 = 8;
const GRID_Y: u32 = 70;

fn btn_rect(col: usize, row: usize) -> (u32, u32, u32, u32) {
    let x = GRID_X + col as u32 * (BTN_W + BTN_PAD);
    let y = GRID_Y + row as u32 * (BTN_H + BTN_PAD);
    let w = if col == 0 && row == 4 { BTN_W * 2 + BTN_PAD } else { BTN_W };
    (x, y, w, BTN_H)
}

fn hit_test(mx: u32, my: u32) -> Option<(usize, usize)> {
    for row in 0..5 {
        for col in 0..4 {
            if BTN_LABELS[row][col].is_empty() { continue; }
            let (x, y, w, h) = btn_rect(col, row);
            if mx >= x && mx < x + w && my >= y && my < y + h {
                return Some((col, row));
            }
        }
    }
    None
}

fn user_main() -> i32 {
    let theme = Theme::dark();
    let mut calc = Calculator::new();

    let win_w = GRID_X * 2 + 4 * (BTN_W + BTN_PAD) + BTN_PAD;
    let win_h = GRID_Y + 5 * (BTN_H + BTN_PAD) + 16;

    let mut win = match Window::create("Calculator", win_w, win_h) {
        Ok(w) => w,
        Err(e) => { io::print_str(&alloc::format!("calculator: window failed: {}\n", e)); return 0; }
    };

    let mut prev_pressed = false;

    loop {
        let mouse = win.get_mouse();
        let pressed = (mouse.buttons & 1) != 0;
        let mx = mouse.x as u32;
        let my = mouse.y as u32;

        // Click handling (on press, not hold)
        if pressed && !prev_pressed {
            if let Some((col, row)) = hit_test(mx, my) {
                match BTN_LABELS[row][col] {
                    "C" => calc.clear(),
                    "±" => calc.negate(),
                    "%" => calc.percent(),
                    "÷" => calc.op(b'/'),
                    "×" => calc.op(b'*'),
                    "−" => calc.op(b'-'),
                    "+" => calc.op(b'+'),
                    "=" => calc.equals(),
                    "." => calc.push_dot(),
                    "0" => calc.push_digit(0),
                    "1" => calc.push_digit(1),
                    "2" => calc.push_digit(2),
                    "3" => calc.push_digit(3),
                    "4" => calc.push_digit(4),
                    "5" => calc.push_digit(5),
                    "6" => calc.push_digit(6),
                    "7" => calc.push_digit(7),
                    "8" => calc.push_digit(8),
                    "9" => calc.push_digit(9),
                    _ => {}
                }
            }
        }
        prev_pressed = pressed;

        // Render
        win.clear(theme.bg_primary);

        // Display area
        win.draw_rounded_rect(GRID_X, 8, win_w - GRID_X * 2, 54, 6, theme.bg_elevated);
        let display_str = core::str::from_utf8(&calc.display[..calc.display_len]).unwrap_or("0");
        let tw = display_str.len() as u32 * 12;
        let dx = (win_w).saturating_sub(tw + 16);
        win.draw_string(dx, 24, display_str, theme.text, 0);

        // Active operator indicator
        if calc.operator != b'=' && calc.new_number {
            let op_ch = match calc.operator {
                b'+' => "+",
                b'-' => "−",
                b'*' => "×",
                b'/' => "÷",
                _ => "",
            };
            win.draw_string(dx - 20, 24, op_ch, theme.accent, 0);
        }

        // Buttons
        for row in 0..5 {
            for col in 0..4 {
                let label = BTN_LABELS[row][col];
                if label.is_empty() { continue; }
                let (x, y, w, h) = btn_rect(col, row);

                // Button color
                let bg = match label {
                    "C" | "±" | "%" => theme.bg_surface,
                    "÷" | "×" | "−" | "+" | "=" => theme.accent,
                    _ => theme.bg_elevated,
                };

                // Hover highlight
                let hovered = mx >= x && mx < x + w && my >= y && my < y + h;
                let final_bg = if hovered { theme.hover } else { bg };

                win.draw_rounded_rect(x, y, w, h, 6, final_bg);

                // Label
                let text_color = if label == "C" || label == "±" || label == "%" { theme.text } else { 0xFFFFFFFF };
                let label_w = label.len() as u32 * 12;
                let lx = x + (w - label_w) / 2;
                let ly = y + (h - 14) / 2;
                win.draw_string(lx, ly, label, text_color, 0);
            }
        }

        // Keyboard input
        while let Some(key) = win.get_key() {
            match key {
                b'0'..=b'9' => calc.push_digit(key - b'0'),
                b'.' | b',' => calc.push_dot(),
                b'+' => calc.op(b'+'),
                b'-' => calc.op(b'-'),
                b'*' | b'x' | b'X' => calc.op(b'*'),
                b'/' => calc.op(b'/'),
                b'=' | 0x0A | 0x0D => calc.equals(),
                b'c' | b'C' => calc.clear(),
                0x08 | 0x7F => calc.backspace(),
                _ => {}
            }
        }

        let _ = win.flush();
        unsafe { libsarga::syscall::syscall1(35, 16_666_000); }
    }
}

sarga_main!(user_main);
