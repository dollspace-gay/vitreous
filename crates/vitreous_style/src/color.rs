/// RGBA color stored as f32 components in the range 0.0..=1.0.
#[derive(Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl core::fmt::Debug for Color {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if (self.a - 1.0).abs() < f32::EPSILON {
            write!(
                f,
                "Color::rgb({}, {}, {})",
                (self.r * 255.0).round() as u8,
                (self.g * 255.0).round() as u8,
                (self.b * 255.0).round() as u8,
            )
        } else {
            write!(
                f,
                "Color::rgba({}, {}, {}, {:.2})",
                (self.r * 255.0).round() as u8,
                (self.g * 255.0).round() as u8,
                (self.b * 255.0).round() as u8,
                self.a,
            )
        }
    }
}

// ── Constructors ──────────────────────────────────────────────────────────────

impl Color {
    /// Create a color from 8-bit RGB values (0–255). Alpha defaults to 1.0.
    pub fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: 1.0,
        }
    }

    /// Create a color from 8-bit RGBA values (0–255 for RGB, 0.0–1.0 for alpha).
    pub fn rgba(r: u8, g: u8, b: u8, a: f32) -> Self {
        Self {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a,
        }
    }

    /// Create a color from a hex string.
    ///
    /// Supported formats: `#RGB`, `#RRGGBB`, `#RRGGBBAA` (with or without `#` prefix).
    ///
    /// # Panics
    ///
    /// Panics in debug builds on invalid input. Returns [`Color::BLACK`] in release builds.
    pub fn hex(s: &str) -> Self {
        match Self::try_hex(s) {
            Some(c) => c,
            None => {
                debug_assert!(false, "Invalid hex color: {s}");
                Self::BLACK
            }
        }
    }

    fn try_hex(s: &str) -> Option<Self> {
        let s = s.strip_prefix('#').unwrap_or(s);
        match s.len() {
            3 => {
                let r = u8::from_str_radix(&s[0..1], 16).ok()?;
                let g = u8::from_str_radix(&s[1..2], 16).ok()?;
                let b = u8::from_str_radix(&s[2..3], 16).ok()?;
                Some(Self::rgb(r * 17, g * 17, b * 17))
            }
            6 => {
                let r = u8::from_str_radix(&s[0..2], 16).ok()?;
                let g = u8::from_str_radix(&s[2..4], 16).ok()?;
                let b = u8::from_str_radix(&s[4..6], 16).ok()?;
                Some(Self::rgb(r, g, b))
            }
            8 => {
                let r = u8::from_str_radix(&s[0..2], 16).ok()?;
                let g = u8::from_str_radix(&s[2..4], 16).ok()?;
                let b = u8::from_str_radix(&s[4..6], 16).ok()?;
                let a = u8::from_str_radix(&s[6..8], 16).ok()?;
                Some(Self::rgba(r, g, b, a as f32 / 255.0))
            }
            _ => None,
        }
    }

    /// Create a color from HSL values.
    ///
    /// - `h`: hue in degrees (0.0–360.0)
    /// - `s`: saturation (0.0–1.0)
    /// - `l`: lightness (0.0–1.0)
    pub fn hsl(h: f32, s: f32, l: f32) -> Self {
        Self::hsla(h, s, l, 1.0)
    }

    /// Create a color from HSLA values.
    ///
    /// - `h`: hue in degrees (0.0–360.0)
    /// - `s`: saturation (0.0–1.0)
    /// - `l`: lightness (0.0–1.0)
    /// - `a`: alpha (0.0–1.0)
    pub fn hsla(h: f32, s: f32, l: f32, a: f32) -> Self {
        let (r, g, b) = hsl_to_rgb(h, s, l);
        Self { r, g, b, a }
    }

    /// Create a color from f32 components directly (all 0.0–1.0).
    pub fn from_f32(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }
}

// ── Manipulation ──────────────────────────────────────────────────────────────

impl Color {
    /// Return a new color with the given alpha value.
    pub fn with_alpha(self, a: f32) -> Self {
        Self { a, ..self }
    }

    /// Lighten the color by the given amount (0.0–1.0).
    ///
    /// Converts to HSL, increases lightness, converts back.
    pub fn lighten(self, amount: f32) -> Self {
        let (h, s, l) = rgb_to_hsl(self.r, self.g, self.b);
        let l = (l + amount).min(1.0);
        let (r, g, b) = hsl_to_rgb(h, s, l);
        Self { r, g, b, a: self.a }
    }

    /// Darken the color by the given amount (0.0–1.0).
    ///
    /// Converts to HSL, decreases lightness, converts back.
    pub fn darken(self, amount: f32) -> Self {
        let (h, s, l) = rgb_to_hsl(self.r, self.g, self.b);
        let l = (l - amount).max(0.0);
        let (r, g, b) = hsl_to_rgb(h, s, l);
        Self { r, g, b, a: self.a }
    }

    /// Linearly interpolate between two colors in RGB space.
    ///
    /// `t = 0.0` returns `a`, `t = 1.0` returns `b`.
    pub fn mix(a: Color, b: Color, t: f32) -> Self {
        Self {
            r: a.r + (b.r - a.r) * t,
            g: a.g + (b.g - a.g) * t,
            b: a.b + (b.b - a.b) * t,
            a: a.a + (b.a - a.a) * t,
        }
    }

    /// Return the relative luminance (WCAG definition).
    pub fn relative_luminance(self) -> f32 {
        fn linearize(c: f32) -> f32 {
            if c <= 0.03928 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        }
        0.2126 * linearize(self.r) + 0.7152 * linearize(self.g) + 0.0722 * linearize(self.b)
    }

    /// Return the WCAG contrast ratio between two colors (1.0–21.0).
    pub fn contrast_ratio(a: Color, b: Color) -> f32 {
        let la = a.relative_luminance();
        let lb = b.relative_luminance();
        let (lighter, darker) = if la > lb { (la, lb) } else { (lb, la) };
        (lighter + 0.05) / (darker + 0.05)
    }
}

// ── Named Constants ───────────────────────────────────────────────────────────

impl Color {
    pub const WHITE: Self = Self {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    };
    pub const BLACK: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    pub const TRANSPARENT: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.0,
    };
    pub const RED: Self = Self {
        r: 1.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    pub const GREEN: Self = Self {
        r: 0.0,
        g: 128.0 / 255.0,
        b: 0.0,
        a: 1.0,
    };
    pub const BLUE: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    };
    pub const YELLOW: Self = Self {
        r: 1.0,
        g: 1.0,
        b: 0.0,
        a: 1.0,
    };
    pub const CYAN: Self = Self {
        r: 0.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    };
    pub const MAGENTA: Self = Self {
        r: 1.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    };
    pub const ORANGE: Self = Self {
        r: 1.0,
        g: 165.0 / 255.0,
        b: 0.0,
        a: 1.0,
    };
    pub const PURPLE: Self = Self {
        r: 128.0 / 255.0,
        g: 0.0,
        b: 128.0 / 255.0,
        a: 1.0,
    };
    pub const GRAY: Self = Self {
        r: 128.0 / 255.0,
        g: 128.0 / 255.0,
        b: 128.0 / 255.0,
        a: 1.0,
    };
    pub const LIGHT_GRAY: Self = Self {
        r: 211.0 / 255.0,
        g: 211.0 / 255.0,
        b: 211.0 / 255.0,
        a: 1.0,
    };
    pub const DARK_GRAY: Self = Self {
        r: 169.0 / 255.0,
        g: 169.0 / 255.0,
        b: 169.0 / 255.0,
        a: 1.0,
    };
    pub const PINK: Self = Self {
        r: 1.0,
        g: 192.0 / 255.0,
        b: 203.0 / 255.0,
        a: 1.0,
    };
    pub const BROWN: Self = Self {
        r: 165.0 / 255.0,
        g: 42.0 / 255.0,
        b: 42.0 / 255.0,
        a: 1.0,
    };
    pub const NAVY: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 128.0 / 255.0,
        a: 1.0,
    };
    pub const TEAL: Self = Self {
        r: 0.0,
        g: 128.0 / 255.0,
        b: 128.0 / 255.0,
        a: 1.0,
    };
    pub const CORAL: Self = Self {
        r: 1.0,
        g: 127.0 / 255.0,
        b: 80.0 / 255.0,
        a: 1.0,
    };
    pub const GOLD: Self = Self {
        r: 1.0,
        g: 215.0 / 255.0,
        b: 0.0,
        a: 1.0,
    };
}

// ── Into<Color> for common types ──────────────────────────────────────────────

impl From<(u8, u8, u8)> for Color {
    fn from((r, g, b): (u8, u8, u8)) -> Self {
        Self::rgb(r, g, b)
    }
}

impl From<(u8, u8, u8, f32)> for Color {
    fn from((r, g, b, a): (u8, u8, u8, f32)) -> Self {
        Self::rgba(r, g, b, a)
    }
}

// ── HSL conversion helpers ────────────────────────────────────────────────────

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (f32, f32, f32) {
    if s == 0.0 {
        return (l, l, l);
    }

    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;
    let h = h / 360.0;

    let r = hue_to_rgb(p, q, h + 1.0 / 3.0);
    let g = hue_to_rgb(p, q, h);
    let b = hue_to_rgb(p, q, h - 1.0 / 3.0);

    (r, g, b)
}

fn hue_to_rgb(p: f32, q: f32, t: f32) -> f32 {
    let t = if t < 0.0 {
        t + 1.0
    } else if t > 1.0 {
        t - 1.0
    } else {
        t
    };

    if t < 1.0 / 6.0 {
        p + (q - p) * 6.0 * t
    } else if t < 1.0 / 2.0 {
        q
    } else if t < 2.0 / 3.0 {
        p + (q - p) * (2.0 / 3.0 - t) * 6.0
    } else {
        p
    }
}

fn rgb_to_hsl(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;

    if (max - min).abs() < f32::EPSILON {
        return (0.0, 0.0, l);
    }

    let d = max - min;
    let s = if l > 0.5 {
        d / (2.0 - max - min)
    } else {
        d / (max + min)
    };

    let h = if (max - r).abs() < f32::EPSILON {
        let mut h = (g - b) / d;
        if g < b {
            h += 6.0;
        }
        h
    } else if (max - g).abs() < f32::EPSILON {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };

    (h * 60.0, s, l)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f32, b: f32) -> bool {
        (a - b).abs() < 0.01
    }

    fn colors_approx_eq(a: Color, b: Color) -> bool {
        approx_eq(a.r, b.r) && approx_eq(a.g, b.g) && approx_eq(a.b, b.b) && approx_eq(a.a, b.a)
    }

    // AC-1
    #[test]
    fn rgb_red() {
        let c = Color::rgb(255, 0, 0);
        assert_eq!(c.r, 1.0);
        assert_eq!(c.g, 0.0);
        assert_eq!(c.b, 0.0);
        assert_eq!(c.a, 1.0);
    }

    // AC-2
    #[test]
    fn hex_6digit() {
        let c = Color::hex("#ff0000");
        assert!(colors_approx_eq(c, Color::rgb(255, 0, 0)));
    }

    #[test]
    fn hex_3digit() {
        let c = Color::hex("#f00");
        assert!(colors_approx_eq(c, Color::rgb(255, 0, 0)));
    }

    #[test]
    fn hex_8digit_alpha() {
        let c = Color::hex("#ff000080");
        assert!(approx_eq(c.r, 1.0));
        assert!(approx_eq(c.g, 0.0));
        assert!(approx_eq(c.b, 0.0));
        assert!(approx_eq(c.a, 128.0 / 255.0));
    }

    #[test]
    fn hex_without_hash() {
        let c = Color::hex("ff0000");
        assert!(colors_approx_eq(c, Color::rgb(255, 0, 0)));
    }

    // AC-3
    #[test]
    fn hsl_red() {
        let c = Color::hsl(0.0, 1.0, 0.5);
        assert!(colors_approx_eq(c, Color::rgb(255, 0, 0)));
    }

    // AC-4
    #[test]
    fn lighten() {
        let c = Color::rgb(128, 0, 0);
        let lighter = c.lighten(0.2);
        assert!(lighter.relative_luminance() > c.relative_luminance());
    }

    #[test]
    fn darken() {
        let c = Color::rgb(128, 0, 0);
        let darker = c.darken(0.2);
        assert!(darker.relative_luminance() < c.relative_luminance());
    }

    // AC-5
    #[test]
    fn mix_white_black() {
        let mixed = Color::mix(Color::WHITE, Color::BLACK, 0.5);
        assert!(approx_eq(mixed.r, 0.5));
        assert!(approx_eq(mixed.g, 0.5));
        assert!(approx_eq(mixed.b, 0.5));
    }

    #[test]
    fn with_alpha() {
        let c = Color::rgb(255, 0, 0).with_alpha(0.5);
        assert_eq!(c.r, 1.0);
        assert_eq!(c.a, 0.5);
    }

    #[test]
    fn named_constants() {
        assert_eq!(Color::WHITE.r, 1.0);
        assert_eq!(Color::WHITE.g, 1.0);
        assert_eq!(Color::WHITE.b, 1.0);
        assert_eq!(Color::BLACK.r, 0.0);
        assert_eq!(Color::BLACK.g, 0.0);
        assert_eq!(Color::BLACK.b, 0.0);
        assert_eq!(Color::TRANSPARENT.a, 0.0);
    }

    #[test]
    fn from_tuple() {
        let c: Color = (255u8, 0u8, 0u8).into();
        assert!(colors_approx_eq(c, Color::RED));
    }

    #[test]
    fn contrast_ratio_black_white() {
        let ratio = Color::contrast_ratio(Color::WHITE, Color::BLACK);
        assert!(ratio > 20.0);
    }

    #[test]
    fn hsl_roundtrip() {
        let original = Color::rgb(100, 150, 200);
        let (h, s, l) = rgb_to_hsl(original.r, original.g, original.b);
        let roundtripped = Color::hsl(h, s, l);
        assert!(colors_approx_eq(original, roundtripped));
    }

    #[test]
    fn debug_format_opaque() {
        let c = Color::rgb(255, 0, 0);
        let s = format!("{c:?}");
        assert_eq!(s, "Color::rgb(255, 0, 0)");
    }

    #[test]
    fn debug_format_alpha() {
        let c = Color::rgba(255, 0, 0, 0.5);
        let s = format!("{c:?}");
        assert_eq!(s, "Color::rgba(255, 0, 0, 0.50)");
    }
}
