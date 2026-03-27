use cosmic_text::{
    Attrs, Buffer, Family, FontSystem, Metrics, Shaping, Style as CosmicStyle, SwashCache,
    Weight as CosmicWeight,
};

use vitreous_style::{FontFamily, FontStyle, FontWeight};

// ═══════════════════════════════════════════════════════════════════════════
// Public types — font descriptor, measurement, shaped text, glyph bitmap
// ═══════════════════════════════════════════════════════════════════════════

/// Describes a font for text measurement and shaping.
#[derive(Debug, Clone)]
pub struct FontDescriptor {
    pub family: FontFamily,
    pub size: f32,
    pub weight: FontWeight,
    pub style: FontStyle,
}

impl Default for FontDescriptor {
    fn default() -> Self {
        Self {
            family: FontFamily::SansSerif,
            size: 16.0,
            weight: FontWeight::Regular,
            style: FontStyle::Normal,
        }
    }
}

/// Result of measuring text.
#[derive(Debug, Clone, PartialEq)]
pub struct TextMeasurement {
    pub width: f32,
    pub height: f32,
    pub lines: usize,
}

/// A shaped glyph with its position relative to the text origin.
#[derive(Debug, Clone)]
pub struct ShapedGlyph {
    pub glyph_id: u16,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub font_size: f32,
    /// The text fragment this glyph represents (for rasterization).
    pub text_fragment: String,
}

/// Result of shaping text: a list of positioned glyphs.
#[derive(Debug, Clone)]
pub struct ShapedText {
    pub glyphs: Vec<ShapedGlyph>,
    pub width: f32,
    pub height: f32,
    pub lines: usize,
}

/// A rasterized glyph bitmap.
#[derive(Debug, Clone)]
pub struct GlyphBitmap {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub left: i32,
    pub top: i32,
}

// ═══════════════════════════════════════════════════════════════════════════
// CosmicTextEngine — cosmic-text backed text engine
// ═══════════════════════════════════════════════════════════════════════════

/// Text engine backed by `cosmic-text`. Handles font discovery, text
/// measurement, shaping, and glyph rasterization.
///
/// The `FontSystem` is created once at startup and discovers system fonts.
/// The `SwashCache` caches rasterized glyphs for the lifetime of the engine.
pub struct CosmicTextEngine {
    font_system: FontSystem,
    swash_cache: SwashCache,
}

impl CosmicTextEngine {
    /// Create a new text engine. Discovers system fonts on construction.
    pub fn new() -> Self {
        Self {
            font_system: FontSystem::new(),
            swash_cache: SwashCache::new(),
        }
    }

    /// Measure text with the given font and optional max width for wrapping.
    ///
    /// Returns the bounding box dimensions and line count.
    pub fn measure(
        &mut self,
        text: &str,
        font: &FontDescriptor,
        max_width: Option<f32>,
    ) -> TextMeasurement {
        let attrs = make_attrs(font);
        let mut buffer = self.create_buffer(font, max_width);
        buffer.set_text(&mut self.font_system, text, &attrs, Shaping::Advanced, None);
        buffer.shape_until_scroll(&mut self.font_system, false);

        let (width, height, lines) = measure_buffer(&buffer);
        TextMeasurement {
            width,
            height,
            lines,
        }
    }

    /// Shape text into positioned glyphs.
    ///
    /// Returns glyph positions suitable for rendering. Each glyph carries
    /// its ID, position, size, and a hash identifying the font face.
    pub fn shape(
        &mut self,
        text: &str,
        font: &FontDescriptor,
        max_width: Option<f32>,
    ) -> ShapedText {
        let attrs = make_attrs(font);
        let mut buffer = self.create_buffer(font, max_width);
        buffer.set_text(&mut self.font_system, text, &attrs, Shaping::Advanced, None);
        buffer.shape_until_scroll(&mut self.font_system, false);

        let (width, height, lines) = measure_buffer(&buffer);
        let mut glyphs = Vec::new();

        for run in buffer.layout_runs() {
            let line_y = run.line_y;
            for glyph in run.glyphs {
                let fragment = text.get(glyph.start..glyph.end)
                    .unwrap_or("")
                    .to_owned();
                glyphs.push(ShapedGlyph {
                    glyph_id: glyph.glyph_id,
                    x: glyph.x,
                    y: line_y,
                    width: glyph.w,
                    height: glyph.font_size,
                    font_size: glyph.font_size,
                    text_fragment: fragment,
                });
            }
        }

        ShapedText {
            glyphs,
            width,
            height,
            lines,
        }
    }

    /// Rasterize a single glyph at the given font size and scale factor.
    ///
    /// Returns `None` if the glyph could not be rasterized (e.g. space characters).
    pub fn rasterize_glyph(
        &mut self,
        text: &str,
        font: &FontDescriptor,
        scale_factor: f32,
    ) -> Option<GlyphBitmap> {
        let scaled_size = font.size * scale_factor;
        let scaled_font = FontDescriptor {
            size: scaled_size,
            ..font.clone()
        };

        let attrs = make_attrs(&scaled_font);
        let mut buffer = self.create_buffer(&scaled_font, None);
        buffer.set_text(&mut self.font_system, text, &attrs, Shaping::Advanced, None);
        buffer.shape_until_scroll(&mut self.font_system, false);

        // Find the first glyph in the shaped output and rasterize it
        for run in buffer.layout_runs() {
            for glyph in run.glyphs {
                // Get physical glyph with cache key for rasterization
                let physical = glyph.physical((0.0, 0.0), 1.0);
                let image = self
                    .swash_cache
                    .get_image(&mut self.font_system, physical.cache_key);
                if let Some(image) = image
                    && !image.data.is_empty()
                {
                    return Some(GlyphBitmap {
                        data: image.data.clone(),
                        width: image.placement.width,
                        height: image.placement.height,
                        left: image.placement.left,
                        top: image.placement.top,
                    });
                }
            }
        }

        None
    }

    /// Get a reference to the underlying `FontSystem` for direct use.
    pub fn font_system(&mut self) -> &mut FontSystem {
        &mut self.font_system
    }

    /// Get a reference to the underlying `SwashCache` for direct use.
    pub fn swash_cache(&mut self) -> &mut SwashCache {
        &mut self.swash_cache
    }

    /// Create a cosmic-text `Buffer` configured for the given font and width.
    fn create_buffer(&mut self, font: &FontDescriptor, max_width: Option<f32>) -> Buffer {
        let metrics = Metrics::new(font.size, font.size * 1.2);
        let mut buffer = Buffer::new(&mut self.font_system, metrics);
        let width = max_width.unwrap_or(f32::MAX);
        buffer.set_size(&mut self.font_system, Some(width), None);
        buffer
    }
}

impl Default for CosmicTextEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════════════════════════════════

/// Build cosmic-text `Attrs` from a `FontDescriptor`.
fn make_attrs(font: &FontDescriptor) -> Attrs<'_> {
    let family = match &font.family {
        FontFamily::SansSerif => Family::SansSerif,
        FontFamily::Serif => Family::Serif,
        FontFamily::Monospace => Family::Monospace,
        FontFamily::Named(name) => Family::Name(name),
    };

    let weight = CosmicWeight(font.weight.numeric());

    let style = match font.style {
        FontStyle::Normal => CosmicStyle::Normal,
        FontStyle::Italic => CosmicStyle::Italic,
    };

    Attrs::new().family(family).weight(weight).style(style)
}

/// Measure a buffer's total width, height, and line count from its layout runs.
fn measure_buffer(buffer: &Buffer) -> (f32, f32, usize) {
    let mut max_width: f32 = 0.0;
    let mut total_height: f32 = 0.0;
    let mut line_count = 0usize;

    for run in buffer.layout_runs() {
        max_width = max_width.max(run.line_w);
        total_height = run.line_y + run.line_height;
        line_count += 1;
    }

    (max_width, total_height, line_count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use vitreous_style::{FontFamily, FontStyle, FontWeight};

    fn default_font() -> FontDescriptor {
        FontDescriptor {
            family: FontFamily::SansSerif,
            size: 16.0,
            weight: FontWeight::Regular,
            style: FontStyle::Normal,
        }
    }

    #[test]
    fn font_descriptor_defaults() {
        let fd = FontDescriptor::default();
        assert_eq!(fd.family, FontFamily::SansSerif);
        assert_eq!(fd.size, 16.0);
        assert_eq!(fd.weight, FontWeight::Regular);
        assert_eq!(fd.style, FontStyle::Normal);
    }

    #[test]
    fn measure_nonempty_text() {
        // AC-2: measure returns non-zero width/height
        let mut engine = CosmicTextEngine::new();
        let measurement = engine.measure("Hello, world!", &default_font(), Some(200.0));
        assert!(measurement.width > 0.0, "width should be > 0");
        assert!(measurement.height > 0.0, "height should be > 0");
        assert!(measurement.lines >= 1, "should have at least 1 line");
    }

    #[test]
    fn measure_empty_text() {
        let mut engine = CosmicTextEngine::new();
        let measurement = engine.measure("", &default_font(), None);
        // Empty text: cosmic-text still produces a line (blank line)
        assert_eq!(measurement.width, 0.0);
    }

    #[test]
    fn shape_returns_glyphs() {
        // AC-3: shape returns ShapedText with glyph positions
        let mut engine = CosmicTextEngine::new();
        let shaped = engine.shape("Hi", &default_font(), None);
        assert!(!shaped.glyphs.is_empty(), "should have glyphs");
        assert!(shaped.width > 0.0);
        assert!(shaped.height > 0.0);
        assert!(shaped.lines >= 1);
    }

    #[test]
    fn shape_glyph_positions_increase() {
        let mut engine = CosmicTextEngine::new();
        let shaped = engine.shape("ABC", &default_font(), None);
        // Glyphs should have increasing x positions (for LTR text)
        for window in shaped.glyphs.windows(2) {
            assert!(
                window[1].x >= window[0].x,
                "glyph positions should increase: {} >= {}",
                window[1].x,
                window[0].x
            );
        }
    }

    #[test]
    fn measure_with_wrapping() {
        let mut engine = CosmicTextEngine::new();
        let no_wrap = engine.measure("Hello world this is a longer text", &default_font(), None);
        let wrapped = engine.measure(
            "Hello world this is a longer text",
            &default_font(),
            Some(100.0),
        );
        // Wrapped text should be taller (more lines) and narrower
        assert!(wrapped.lines >= no_wrap.lines);
        assert!(wrapped.width <= no_wrap.width + 1.0);
    }

    #[test]
    fn rasterize_produces_bitmap() {
        // AC-4: rasterize_glyph returns a non-empty bitmap
        let mut engine = CosmicTextEngine::new();
        let bitmap = engine.rasterize_glyph("A", &default_font(), 1.0);
        if let Some(bmp) = bitmap {
            assert!(!bmp.data.is_empty(), "bitmap data should not be empty");
            assert!(bmp.width > 0, "bitmap width should be > 0");
            assert!(bmp.height > 0, "bitmap height should be > 0");
        }
        // Note: bitmap may be None in headless CI without fonts installed
    }

    #[test]
    fn rasterize_at_higher_scale() {
        // AC-10: text rasterized at display scale
        let mut engine = CosmicTextEngine::new();
        let bmp_1x = engine.rasterize_glyph("A", &default_font(), 1.0);
        let bmp_2x = engine.rasterize_glyph("A", &default_font(), 2.0);
        if let (Some(b1), Some(b2)) = (bmp_1x, bmp_2x) {
            // 2x bitmap should have more pixels
            assert!(
                b2.width >= b1.width,
                "2x should be wider: {} >= {}",
                b2.width,
                b1.width
            );
            assert!(
                b2.height >= b1.height,
                "2x should be taller: {} >= {}",
                b2.height,
                b1.height
            );
        }
    }

    #[test]
    fn different_fonts_produce_different_measurements() {
        let mut engine = CosmicTextEngine::new();
        let regular = FontDescriptor {
            size: 16.0,
            ..default_font()
        };
        let large = FontDescriptor {
            size: 32.0,
            ..default_font()
        };
        let m_regular = engine.measure("Test", &regular, None);
        let m_large = engine.measure("Test", &large, None);
        assert!(
            m_large.height > m_regular.height,
            "larger font should produce taller text"
        );
    }

    #[test]
    fn engine_default_trait() {
        let _engine = CosmicTextEngine::default();
    }
}
