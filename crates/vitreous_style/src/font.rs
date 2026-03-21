/// Font weight ranging from Thin (100) to Black (900).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FontWeight {
    Thin,
    ExtraLight,
    Light,
    #[default]
    Regular,
    Medium,
    SemiBold,
    Bold,
    ExtraBold,
    Black,
}

impl FontWeight {
    /// Return the numeric weight value (100–900).
    pub fn numeric(self) -> u16 {
        match self {
            Self::Thin => 100,
            Self::ExtraLight => 200,
            Self::Light => 300,
            Self::Regular => 400,
            Self::Medium => 500,
            Self::SemiBold => 600,
            Self::Bold => 700,
            Self::ExtraBold => 800,
            Self::Black => 900,
        }
    }
}

/// Font family specification.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum FontFamily {
    /// The platform default sans-serif font.
    #[default]
    SansSerif,
    /// The platform default serif font.
    Serif,
    /// The platform default monospace font.
    Monospace,
    /// A specific font by name.
    Named(String),
}

/// Font style (normal or italic).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FontStyle {
    #[default]
    Normal,
    Italic,
}

/// Horizontal text alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TextAlign {
    #[default]
    Start,
    Center,
    End,
    Justify,
}

/// How overflowing text is handled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TextOverflow {
    /// Text is clipped at the boundary.
    #[default]
    Clip,
    /// Text is truncated with an ellipsis.
    Ellipsis,
}

#[cfg(test)]
mod tests {
    use super::*;

    // AC-13
    #[test]
    fn font_weight_numeric_values() {
        assert_eq!(FontWeight::Thin.numeric(), 100);
        assert_eq!(FontWeight::ExtraLight.numeric(), 200);
        assert_eq!(FontWeight::Light.numeric(), 300);
        assert_eq!(FontWeight::Regular.numeric(), 400);
        assert_eq!(FontWeight::Medium.numeric(), 500);
        assert_eq!(FontWeight::SemiBold.numeric(), 600);
        assert_eq!(FontWeight::Bold.numeric(), 700);
        assert_eq!(FontWeight::ExtraBold.numeric(), 800);
        assert_eq!(FontWeight::Black.numeric(), 900);
    }

    #[test]
    fn font_weight_default() {
        assert_eq!(FontWeight::default(), FontWeight::Regular);
    }

    #[test]
    fn font_family_default() {
        assert_eq!(FontFamily::default(), FontFamily::SansSerif);
    }

    #[test]
    fn font_family_named() {
        let family = FontFamily::Named("Helvetica".to_string());
        assert_eq!(family, FontFamily::Named("Helvetica".to_string()));
    }

    #[test]
    fn font_style_default() {
        assert_eq!(FontStyle::default(), FontStyle::Normal);
    }

    #[test]
    fn text_align_default() {
        assert_eq!(TextAlign::default(), TextAlign::Start);
    }

    #[test]
    fn text_overflow_default() {
        assert_eq!(TextOverflow::default(), TextOverflow::Clip);
    }
}
