use crate::animation::{Shadow, Transition};
use crate::color::Color;
use crate::dimension::{Corners, Dimension, Edges};
use crate::font::{FontFamily, FontStyle, FontWeight, TextAlign, TextOverflow};

/// Standard cursor icon types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CursorIcon {
    #[default]
    Default,
    Pointer,
    Text,
    Crosshair,
    Move,
    NotAllowed,
    Grab,
    Grabbing,
    ColResize,
    RowResize,
    NResize,
    EResize,
    SResize,
    WResize,
    NeResize,
    NwResize,
    SeResize,
    SwResize,
    EwResize,
    NsResize,
    NeswResize,
    NwseResize,
    Wait,
    Progress,
    Help,
    ZoomIn,
    ZoomOut,
    None,
}

/// Overflow behavior for content that exceeds its container.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Overflow {
    #[default]
    Visible,
    Hidden,
    Scroll,
}

/// Aggregated visual style properties for a widget node.
///
/// All `Option` fields default to `None`, meaning the property is not set
/// and the widget should inherit or use the system default.
#[derive(Debug, Clone, PartialEq)]
pub struct Style {
    // ── Layout dimensions ─────────────────────────────────────────────
    pub width: Dimension,
    pub height: Dimension,
    pub min_width: Dimension,
    pub min_height: Dimension,
    pub max_width: Dimension,
    pub max_height: Dimension,

    // ── Spacing ───────────────────────────────────────────────────────
    pub padding: Edges,
    pub margin: Edges,

    // ── Visual ────────────────────────────────────────────────────────
    pub background: Option<Color>,
    pub foreground: Option<Color>,
    pub border_color: Option<Color>,
    pub border_width: Edges,
    pub border_radius: Corners,
    pub opacity: f32,
    pub shadow: Option<Shadow>,

    // ── Text ──────────────────────────────────────────────────────────
    pub font_size: Option<f32>,
    pub font_weight: Option<FontWeight>,
    pub font_family: Option<FontFamily>,
    pub font_style: Option<FontStyle>,
    pub text_align: Option<TextAlign>,
    pub text_overflow: Option<TextOverflow>,
    pub line_height: Option<f32>,
    pub letter_spacing: Option<f32>,

    // ── Interaction ───────────────────────────────────────────────────
    pub cursor: Option<CursorIcon>,

    // ── Overflow ──────────────────────────────────────────────────────
    pub overflow: Overflow,
    pub clip_content: bool,

    // ── Transitions ───────────────────────────────────────────────────
    pub transitions: Vec<Transition>,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            width: Dimension::Auto,
            height: Dimension::Auto,
            min_width: Dimension::Auto,
            min_height: Dimension::Auto,
            max_width: Dimension::Auto,
            max_height: Dimension::Auto,

            padding: Edges::default(),
            margin: Edges::default(),

            background: None,
            foreground: None,
            border_color: None,
            border_width: Edges::default(),
            border_radius: Corners::default(),
            opacity: 1.0,
            shadow: None,

            font_size: None,
            font_weight: None,
            font_family: None,
            font_style: None,
            text_align: None,
            text_overflow: None,
            line_height: None,
            letter_spacing: None,

            cursor: None,

            overflow: Overflow::Visible,
            clip_content: false,

            transitions: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // AC-10
    #[test]
    fn style_default() {
        let s = Style::default();
        assert_eq!(s.opacity, 1.0);
        assert!(!s.clip_content);
        assert!(s.background.is_none());
        assert!(s.foreground.is_none());
        assert!(s.border_color.is_none());
        assert!(s.shadow.is_none());
        assert!(s.font_size.is_none());
        assert!(s.font_weight.is_none());
        assert!(s.font_family.is_none());
        assert!(s.font_style.is_none());
        assert!(s.text_align.is_none());
        assert!(s.text_overflow.is_none());
        assert!(s.line_height.is_none());
        assert!(s.letter_spacing.is_none());
        assert!(s.cursor.is_none());
        assert_eq!(s.overflow, Overflow::Visible);
        assert!(s.transitions.is_empty());
    }

    #[test]
    fn cursor_icon_default() {
        assert_eq!(CursorIcon::default(), CursorIcon::Default);
    }

    #[test]
    fn overflow_default() {
        assert_eq!(Overflow::default(), Overflow::Visible);
    }

    #[test]
    fn style_dimensions_default_auto() {
        let s = Style::default();
        assert_eq!(s.width, Dimension::Auto);
        assert_eq!(s.height, Dimension::Auto);
        assert_eq!(s.min_width, Dimension::Auto);
        assert_eq!(s.min_height, Dimension::Auto);
        assert_eq!(s.max_width, Dimension::Auto);
        assert_eq!(s.max_height, Dimension::Auto);
    }
}
