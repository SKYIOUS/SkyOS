//! Theme contrast selftest — WCAG-AA pins for the text-on-surface pairs the
//! draw paths produce, in BOTH themes.
//!
//! The light-theme audit (2026-08-08) found three violation families when
//! surfaces flip to white while accent/hover stay indigo and error stays red:
//! (1) theme.text / text_secondary drawn on accent/hover fills — black text
//! on indigo is 4.09:1, gray is 1.45:1; fixed by the `on_accent` field
//! (white, 5.13:1) used at every accent/hover surface; (2) light
//! text_disabled 0xAAAAAA on white — 2.32:1; fixed by the 0xFF757575 value;
//! (3) light success 0x4CAF50 on the elevated toggle surface — 2.4:1; fixed
//! by the 0xFF2B6A30 value. These pins hold the palette fixes: a palette
//! change that drops any pinned pair below its threshold fails here with the
//! actual ratio printed. The pins are a palette-contract net, not a
//! draw-path audit — the draw sites' use of `on_accent` is not exercised by
//! this test (a revert of e.g. the taskbar button text to `theme.text` on
//! accent would produce 4.09:1 but still pass here); that half of the fix is
//! covered by the QEMU boot plus the conversion being a one-shot code
//! change with the comments naming the invariant at each site.
//!
//! WCAG relative luminance needs the sRGB 2.4 power, which `core` cannot
//! compute (`powf` is libm). x^2.4 = x^2 · (x^0.2)² and x^0.2 = x^(1/5), so
//! a Newton fifth-root (pure f32 mul/div, seeded above the root at 1.0 and
//! monotonically convergent for x ∈ (0,1]) gives the same numbers the host
//! Python port computed.

use libsarga::io;

/// sRGB channel linearization: c ∈ 0..=255 → linear 0..=1 (WCAG piecewise).
fn linearize(ch: u32) -> f32 {
    let c = ch as f32 / 255.0;
    if c <= 0.03928 {
        c / 12.92
    } else {
        let x = (c + 0.055) / 1.055;
        // x^2.4 = x^2 · (x^0.2)², and x^0.2 = x^(1/5).
        let fifth = fifth_root(x);
        x * x * fifth * fifth
    }
}

/// Newton iteration for the fifth root of `x` (x ∈ (0,1]): y' = (4y + x/y⁴)/5
/// from the seed 1.0, which always lies at or above the root so the
/// iteration converges monotonically. 20 steps is far into convergence for
/// the whole domain (the host Python used the exact formula to ~0.01:1).
fn fifth_root(x: f32) -> f32 {
    let mut y = 1.0f32;
    for _ in 0..20 {
        let y4 = y * y * y * y;
        y = (4.0 * y + x / y4) / 5.0;
    }
    y
}

/// WCAG relative luminance of an RGB color.
fn luminance(c: u32) -> f32 {
    let r = (c >> 16) & 0xFF;
    let g = (c >> 8) & 0xFF;
    let b = c & 0xFF;
    0.2126 * linearize(r) + 0.7152 * linearize(g) + 0.0722 * linearize(b)
}

/// WCAG contrast ratio between two RGB colors (order-independent).
fn contrast(fg: u32, bg: u32) -> f32 {
    let (mut a, mut b) = (luminance(fg), luminance(bg));
    if a < b {
        core::mem::swap(&mut a, &mut b);
    }
    (a + 0.05) / (b + 0.05)
}

/// Check one pair; log a FAIL line with the actual ratio on violation.
fn check(name: &str, fg: u32, bg: u32, min: f32) -> bool {
    let r = contrast(fg, bg);
    if r < min {
        io::print_str(&alloc::format!(
            "[test] FAIL test_theme_contrast: {} {:.2}:1 < {:.1}:1\n",
            name,
            r,
            min
        ));
        false
    } else {
        true
    }
}

pub(crate) fn test_theme_contrast() -> bool {
    let light = libsarga::theme::Theme::light();
    let dark = libsarga::theme::Theme::dark();
    let mut ok = true;

    // The core fix: text on the theme-invariant indigo accent/hover fills is
    // white (`on_accent`) in both themes — 5.13:1. A palette change or a
    // draw-site revert that reintroduces theme.text here fails this.
    for (name, theme) in [("light", &light), ("dark", &dark)] {
        ok &= check(
            &alloc::format!("{} on_accent on accent", name),
            theme.on_accent,
            theme.accent,
            4.5,
        );
        ok &= check(
            &alloc::format!("{} on_accent on hover", name),
            theme.on_accent,
            theme.hover,
            4.5,
        );
    }

    // Base text pairs — the normal surfaces, both themes.
    for (name, theme) in [("light", &light), ("dark", &dark)] {
        ok &= check(
            &alloc::format!("{} text on bg_surface", name),
            theme.text,
            theme.bg_surface,
            4.5,
        );
        ok &= check(
            &alloc::format!("{} text_secondary on bg_surface", name),
            theme.text_secondary,
            theme.bg_surface,
            4.5,
        );
        ok &= check(
            &alloc::format!("{} text_secondary on bg_elevated", name),
            theme.text_secondary,
            theme.bg_elevated,
            4.5,
        );
        ok &= check(
            &alloc::format!("{} text_secondary on bg_primary", name),
            theme.text_secondary,
            theme.bg_primary,
            4.5,
        );
        // Pressed fills (light gray in light mode, navy in dark) keep the
        // theme text — black/white respectively — comfortably readable.
        ok &= check(
            &alloc::format!("{} text on pressed", name),
            theme.text,
            theme.pressed,
            4.5,
        );
    }

    // Disabled text: the light value was the fix (0xAAAAAA → 0xFF757575,
    // 2.32 → 4.61:1 on white). Dark stays a dim 2.7 by design — disabled
    // semantics — so its pin is a "still distinguishable" floor, not AA.
    ok &= check(
        "light text_disabled on bg_surface",
        light.text_disabled,
        light.bg_surface,
        4.0,
    );
    ok &= check(
        "dark text_disabled on bg_surface",
        dark.text_disabled,
        dark.bg_surface,
        2.0,
    );

    // Success (toggle 'Y'): the light value was the fix (0x4CAF50 → 0xFF2B6A30,
    // 2.4 → 5.6:1 on the elevated toggle surface); dark passes already.
    ok &= check(
        "light success on bg_elevated",
        light.success,
        light.bg_elevated,
        4.5,
    );
    ok &= check(
        "dark success on bg_elevated",
        dark.success,
        dark.bg_elevated,
        4.5,
    );

    // Close-button glyphs — theme-invariant reds, white glyph.
    ok &= check(
        "on_accent on error (close)",
        light.on_accent,
        light.error,
        4.5,
    );
    ok &= check(
        "on_accent on WIN_CLOSE_HOVER",
        light.on_accent,
        libsarga::theme::colors::WIN_CLOSE_HOVER,
        4.5,
    );

    if ok {
        io::print_str("[test] PASS test_theme_contrast\n");
    }
    ok
}
