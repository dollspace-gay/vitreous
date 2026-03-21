use core::time::Duration;

use crate::animation::Shadow;
use crate::color::Color;
use crate::dimension::Corners;

/// Complete semantic theme with colors, typography scale, spacing, and timing.
#[derive(Debug, Clone, PartialEq)]
pub struct Theme {
    pub is_dark: bool,

    // ── Colors ────────────────────────────────────────────────────────
    pub primary: Color,
    pub primary_variant: Color,
    pub secondary: Color,
    pub secondary_variant: Color,
    pub background: Color,
    pub surface: Color,
    pub error: Color,
    pub warning: Color,
    pub success: Color,
    pub info: Color,

    // ── Text colors ───────────────────────────────────────────────────
    pub text_primary: Color,
    pub text_secondary: Color,
    pub text_disabled: Color,
    pub text_on_primary: Color,
    pub text_on_secondary: Color,
    pub text_on_error: Color,

    // ── Border / divider ──────────────────────────────────────────────
    pub border: Color,
    pub divider: Color,

    // ── Typography scale (px) ─────────────────────────────────────────
    pub font_size_xs: f32,
    pub font_size_sm: f32,
    pub font_size_md: f32,
    pub font_size_lg: f32,
    pub font_size_xl: f32,
    pub font_size_2xl: f32,
    pub font_size_3xl: f32,

    // ── Spacing scale (px) ────────────────────────────────────────────
    pub spacing_xs: f32,
    pub spacing_sm: f32,
    pub spacing_md: f32,
    pub spacing_lg: f32,
    pub spacing_xl: f32,
    pub spacing_2xl: f32,
    pub spacing_3xl: f32,

    // ── Border radii ──────────────────────────────────────────────────
    pub radius_sm: Corners,
    pub radius_md: Corners,
    pub radius_lg: Corners,
    pub radius_full: Corners,

    // ── Shadows ───────────────────────────────────────────────────────
    pub shadow_sm: Shadow,
    pub shadow_md: Shadow,
    pub shadow_lg: Shadow,

    // ── Animation durations ───────────────────────────────────────────
    pub duration_fast: Duration,
    pub duration_normal: Duration,
    pub duration_slow: Duration,
}

impl Theme {
    /// A light theme with WCAG AA–compliant contrast ratios.
    pub fn light() -> Self {
        let background = Color::WHITE;
        let text_primary = Color::from_f32(0.12, 0.12, 0.12, 1.0); // #1f1f1f
        let text_secondary = Color::from_f32(0.38, 0.38, 0.38, 1.0); // #616161
        let shadow_color = Color::from_f32(0.0, 0.0, 0.0, 0.12);

        Self {
            is_dark: false,

            primary: Color::from_f32(0.098, 0.463, 0.824, 1.0), // #1976D2
            primary_variant: Color::from_f32(0.063, 0.278, 0.643, 1.0), // #1048A4
            secondary: Color::from_f32(0.012, 0.588, 0.533, 1.0), // #00897B
            secondary_variant: Color::from_f32(0.0, 0.412, 0.373, 1.0), // #00695F
            background,
            surface: Color::from_f32(0.976, 0.976, 0.976, 1.0), // #F9F9F9
            error: Color::from_f32(0.827, 0.184, 0.184, 1.0),   // #D32F2F
            warning: Color::from_f32(0.957, 0.620, 0.043, 1.0), // #F49E0B
            success: Color::from_f32(0.176, 0.655, 0.341, 1.0), // #2DA757
            info: Color::from_f32(0.012, 0.608, 0.898, 1.0),    // #039BE5

            text_primary,
            text_secondary,
            text_disabled: Color::from_f32(0.62, 0.62, 0.62, 1.0), // #9E9E9E
            text_on_primary: Color::WHITE,
            text_on_secondary: Color::WHITE,
            text_on_error: Color::WHITE,

            border: Color::from_f32(0.88, 0.88, 0.88, 1.0), // #E0E0E0
            divider: Color::from_f32(0.88, 0.88, 0.88, 0.6),

            font_size_xs: 10.0,
            font_size_sm: 12.0,
            font_size_md: 14.0,
            font_size_lg: 16.0,
            font_size_xl: 20.0,
            font_size_2xl: 24.0,
            font_size_3xl: 32.0,

            spacing_xs: 4.0,
            spacing_sm: 8.0,
            spacing_md: 16.0,
            spacing_lg: 24.0,
            spacing_xl: 32.0,
            spacing_2xl: 48.0,
            spacing_3xl: 64.0,

            radius_sm: Corners::all(4.0),
            radius_md: Corners::all(8.0),
            radius_lg: Corners::all(12.0),
            radius_full: Corners::all(9999.0),

            shadow_sm: Shadow::new(0.0, 1.0, 2.0, 0.0, shadow_color),
            shadow_md: Shadow::new(0.0, 2.0, 8.0, 0.0, shadow_color),
            shadow_lg: Shadow::new(0.0, 4.0, 16.0, 0.0, shadow_color),

            duration_fast: Duration::from_millis(100),
            duration_normal: Duration::from_millis(200),
            duration_slow: Duration::from_millis(400),
        }
    }

    /// A dark theme with WCAG AA–compliant contrast ratios.
    pub fn dark() -> Self {
        let background = Color::from_f32(0.075, 0.075, 0.075, 1.0); // #131313
        let shadow_color = Color::from_f32(0.0, 0.0, 0.0, 0.30);

        Self {
            is_dark: true,

            primary: Color::from_f32(0.357, 0.608, 0.871, 1.0), // #5B9BDE
            primary_variant: Color::from_f32(0.506, 0.718, 0.933, 1.0), // #81B7EE
            secondary: Color::from_f32(0.298, 0.761, 0.714, 1.0), // #4CC2B6
            secondary_variant: Color::from_f32(0.502, 0.843, 0.808, 1.0), // #80D7CE
            background,
            surface: Color::from_f32(0.118, 0.118, 0.118, 1.0), // #1E1E1E
            error: Color::from_f32(0.937, 0.482, 0.482, 1.0),   // #EF7B7B
            warning: Color::from_f32(1.0, 0.718, 0.302, 1.0),   // #FFB74D
            success: Color::from_f32(0.506, 0.831, 0.576, 1.0), // #81D493
            info: Color::from_f32(0.302, 0.733, 0.933, 1.0),    // #4DBBEE

            text_primary: Color::from_f32(0.93, 0.93, 0.93, 1.0), // #EDEDED
            text_secondary: Color::from_f32(0.70, 0.70, 0.70, 1.0), // #B3B3B3
            text_disabled: Color::from_f32(0.45, 0.45, 0.45, 1.0), // #737373
            text_on_primary: Color::from_f32(0.06, 0.06, 0.06, 1.0),
            text_on_secondary: Color::from_f32(0.06, 0.06, 0.06, 1.0),
            text_on_error: Color::from_f32(0.06, 0.06, 0.06, 1.0),

            border: Color::from_f32(0.25, 0.25, 0.25, 1.0), // #404040
            divider: Color::from_f32(0.25, 0.25, 0.25, 0.6),

            font_size_xs: 10.0,
            font_size_sm: 12.0,
            font_size_md: 14.0,
            font_size_lg: 16.0,
            font_size_xl: 20.0,
            font_size_2xl: 24.0,
            font_size_3xl: 32.0,

            spacing_xs: 4.0,
            spacing_sm: 8.0,
            spacing_md: 16.0,
            spacing_lg: 24.0,
            spacing_xl: 32.0,
            spacing_2xl: 48.0,
            spacing_3xl: 64.0,

            radius_sm: Corners::all(4.0),
            radius_md: Corners::all(8.0),
            radius_lg: Corners::all(12.0),
            radius_full: Corners::all(9999.0),

            shadow_sm: Shadow::new(0.0, 1.0, 3.0, 0.0, shadow_color),
            shadow_md: Shadow::new(0.0, 3.0, 10.0, 0.0, shadow_color),
            shadow_lg: Shadow::new(0.0, 6.0, 20.0, 0.0, shadow_color),

            duration_fast: Duration::from_millis(100),
            duration_normal: Duration::from_millis(200),
            duration_slow: Duration::from_millis(400),
        }
    }

    /// Returns [`Theme::light()`] as a default; intended as a placeholder for
    /// system theme detection which requires platform integration.
    pub fn system() -> Self {
        Self::light()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // AC-6: both themes fully populated, is_dark correct
    #[test]
    fn light_theme_populated() {
        let t = Theme::light();
        assert!(!t.is_dark);
        assert_ne!(t.primary, Color::TRANSPARENT);
        assert_ne!(t.secondary, Color::TRANSPARENT);
        assert_ne!(t.background, Color::TRANSPARENT);
        assert_ne!(t.surface, Color::TRANSPARENT);
        assert_ne!(t.text_primary, Color::TRANSPARENT);
        assert_ne!(t.text_secondary, Color::TRANSPARENT);
        assert!(t.font_size_md > 0.0);
        assert!(t.spacing_md > 0.0);
    }

    #[test]
    fn dark_theme_populated() {
        let t = Theme::dark();
        assert!(t.is_dark);
        assert_ne!(t.primary, Color::TRANSPARENT);
        assert_ne!(t.secondary, Color::TRANSPARENT);
        assert_ne!(t.background, Color::TRANSPARENT);
        assert_ne!(t.surface, Color::TRANSPARENT);
        assert_ne!(t.text_primary, Color::TRANSPARENT);
        assert_ne!(t.text_secondary, Color::TRANSPARENT);
        assert!(t.font_size_md > 0.0);
        assert!(t.spacing_md > 0.0);
    }

    // AC-7: WCAG AA contrast (4.5:1 for normal text)
    #[test]
    fn light_theme_wcag_aa_text_primary() {
        let t = Theme::light();
        let ratio = Color::contrast_ratio(t.text_primary, t.background);
        assert!(
            ratio >= 4.5,
            "text_primary/background contrast {ratio:.2} < 4.5"
        );
    }

    #[test]
    fn light_theme_wcag_aa_text_secondary() {
        let t = Theme::light();
        let ratio = Color::contrast_ratio(t.text_secondary, t.background);
        assert!(
            ratio >= 4.5,
            "text_secondary/background contrast {ratio:.2} < 4.5"
        );
    }

    #[test]
    fn dark_theme_wcag_aa_text_primary() {
        let t = Theme::dark();
        let ratio = Color::contrast_ratio(t.text_primary, t.background);
        assert!(
            ratio >= 4.5,
            "dark text_primary/background contrast {ratio:.2} < 4.5"
        );
    }

    #[test]
    fn dark_theme_wcag_aa_text_secondary() {
        let t = Theme::dark();
        let ratio = Color::contrast_ratio(t.text_secondary, t.background);
        assert!(
            ratio >= 4.5,
            "dark text_secondary/background contrast {ratio:.2} < 4.5"
        );
    }

    #[test]
    fn system_theme_returns_light() {
        let system = Theme::system();
        let light = Theme::light();
        assert_eq!(system, light);
    }

    #[test]
    fn typography_scale_ordered() {
        let t = Theme::light();
        assert!(t.font_size_xs < t.font_size_sm);
        assert!(t.font_size_sm < t.font_size_md);
        assert!(t.font_size_md < t.font_size_lg);
        assert!(t.font_size_lg < t.font_size_xl);
        assert!(t.font_size_xl < t.font_size_2xl);
        assert!(t.font_size_2xl < t.font_size_3xl);
    }

    #[test]
    fn spacing_scale_ordered() {
        let t = Theme::light();
        assert!(t.spacing_xs < t.spacing_sm);
        assert!(t.spacing_sm < t.spacing_md);
        assert!(t.spacing_md < t.spacing_lg);
        assert!(t.spacing_lg < t.spacing_xl);
        assert!(t.spacing_xl < t.spacing_2xl);
        assert!(t.spacing_2xl < t.spacing_3xl);
    }
}
