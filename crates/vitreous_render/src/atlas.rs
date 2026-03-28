use rustc_hash::FxHashMap;

use crate::commands::TextureId;

/// Default atlas texture dimensions (2048x2048).
const DEFAULT_ATLAS_SIZE: u32 = 2048;

/// Cache key for glyph atlas lookups.
///
/// Quantized size and scale factor avoid floating-point key fragmentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GlyphCacheKey {
    pub glyph_id: u16,
    pub font_hash: u64,
    /// Font size quantized to 1/4 pixel (size * 4.0 as u32).
    pub size_q: u32,
    /// Scale factor quantized to 1/4 (scale * 4.0 as u32).
    pub scale_q: u32,
}

impl GlyphCacheKey {
    pub fn new(glyph_id: u16, font_hash: u64, font_size: f32, scale_factor: f32) -> Self {
        Self {
            glyph_id,
            font_hash,
            size_q: (font_size * 4.0) as u32,
            scale_q: (scale_factor * 4.0) as u32,
        }
    }
}

/// UV coordinates for a region within an atlas texture.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AtlasRegion {
    pub page: u32,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl AtlasRegion {
    /// Returns UV coordinates as (u_min, v_min, u_max, v_max) normalized to atlas size.
    pub fn uv(&self, atlas_size: u32) -> (f32, f32, f32, f32) {
        let s = atlas_size as f32;
        (
            self.x as f32 / s,
            self.y as f32 / s,
            (self.x + self.width) as f32 / s,
            (self.y + self.height) as f32 / s,
        )
    }
}

/// A shelf in the shelf-based rectangle packing algorithm.
struct Shelf {
    y: u32,
    height: u32,
    cursor_x: u32,
}

/// A single page (texture) in the atlas.
struct AtlasPage {
    size: u32,
    shelves: Vec<Shelf>,
    next_y: u32,
}

impl AtlasPage {
    fn new(size: u32) -> Self {
        Self {
            size,
            shelves: Vec::new(),
            next_y: 0,
        }
    }

    /// Tries to allocate a region of the given dimensions. Returns the position if successful.
    fn allocate(&mut self, width: u32, height: u32) -> Option<(u32, u32)> {
        // Try to fit in an existing shelf
        for shelf in &mut self.shelves {
            if height <= shelf.height && shelf.cursor_x + width <= self.size {
                let pos = (shelf.cursor_x, shelf.y);
                shelf.cursor_x += width;
                return Some(pos);
            }
        }

        // Create a new shelf
        if self.next_y + height <= self.size {
            let y = self.next_y;
            self.shelves.push(Shelf {
                y,
                height,
                cursor_x: width,
            });
            self.next_y += height;
            Some((0, y))
        } else {
            None
        }
    }
}

/// Bearing and bitmap metrics for a cached glyph, used to correct quad
/// position and size after rasterization.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlyphBearing {
    /// Horizontal bearing (pixels from pen position to left edge of bitmap).
    pub left: f32,
    /// Vertical bearing (pixels from baseline to top edge of bitmap).
    pub top: f32,
    /// Actual bitmap width in logical pixels.
    pub width: f32,
    /// Actual bitmap height in logical pixels.
    pub height: f32,
}

/// Glyph texture atlas using shelf-based rectangle packing.
///
/// Rasterized glyph bitmaps are cached in GPU textures. When a page fills up,
/// a new page is created. Lookups are keyed by `GlyphCacheKey`.
pub struct GlyphAtlas {
    pages: Vec<AtlasPage>,
    cache: FxHashMap<GlyphCacheKey, AtlasRegion>,
    bearings: FxHashMap<GlyphCacheKey, GlyphBearing>,
    atlas_size: u32,
    lookup_count: u64,
    cache_hit_count: u64,
}

impl GlyphAtlas {
    pub fn new() -> Self {
        Self::with_size(DEFAULT_ATLAS_SIZE)
    }

    pub fn with_size(atlas_size: u32) -> Self {
        Self {
            pages: vec![AtlasPage::new(atlas_size)],
            cache: FxHashMap::default(),
            bearings: FxHashMap::default(),
            atlas_size,
            lookup_count: 0,
            cache_hit_count: 0,
        }
    }

    /// Looks up a glyph in the cache.
    pub fn get(&mut self, key: GlyphCacheKey) -> Option<AtlasRegion> {
        self.lookup_count += 1;
        let result = self.cache.get(&key).copied();
        if result.is_some() {
            self.cache_hit_count += 1;
        }
        result
    }

    /// Allocates space for a glyph and inserts it into the cache.
    /// Returns the allocated region. The caller is responsible for uploading
    /// the actual bitmap data to the GPU texture at this position.
    pub fn insert(&mut self, key: GlyphCacheKey, width: u32, height: u32) -> AtlasRegion {
        if let Some(existing) = self.cache.get(&key) {
            return *existing;
        }

        let page_count = self.pages.len();
        for (page_idx, page) in self.pages.iter_mut().enumerate() {
            if let Some((x, y)) = page.allocate(width, height) {
                let region = AtlasRegion {
                    page: page_idx as u32,
                    x,
                    y,
                    width,
                    height,
                };
                self.cache.insert(key, region);
                return region;
            }
        }

        // All pages full — create a new one
        let mut new_page = AtlasPage::new(self.atlas_size);
        let (x, y) = new_page
            .allocate(width, height)
            .expect("glyph too large for atlas page");
        let region = AtlasRegion {
            page: page_count as u32,
            x,
            y,
            width,
            height,
        };
        self.pages.push(new_page);
        self.cache.insert(key, region);
        region
    }

    /// Store bearing metrics for a cached glyph.
    pub fn insert_bearing(&mut self, key: GlyphCacheKey, bearing: GlyphBearing) {
        self.bearings.insert(key, bearing);
    }

    /// Retrieve bearing metrics for a cached glyph.
    pub fn get_bearing(&self, key: GlyphCacheKey) -> Option<GlyphBearing> {
        self.bearings.get(&key).copied()
    }

    /// Returns the number of atlas pages currently allocated.
    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    /// Returns the atlas texture size (square, in pixels).
    pub fn atlas_size(&self) -> u32 {
        self.atlas_size
    }

    /// Returns (total_lookups, cache_hits) for diagnostic purposes.
    pub fn stats(&self) -> (u64, u64) {
        (self.lookup_count, self.cache_hit_count)
    }

    /// Clears the entire atlas (e.g., on font change or scale factor change).
    pub fn clear(&mut self) {
        self.pages.clear();
        self.pages.push(AtlasPage::new(self.atlas_size));
        self.cache.clear();
        self.bearings.clear();
        self.lookup_count = 0;
        self.cache_hit_count = 0;
    }
}

impl Default for GlyphAtlas {
    fn default() -> Self {
        Self::new()
    }
}

/// Cache key for image atlas lookups.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ImageCacheKey(pub TextureId);

/// Image texture cache. Unlike the glyph atlas which packs many small glyphs,
/// images are stored as individual GPU textures since they vary widely in size.
pub struct ImageAtlas {
    cache: FxHashMap<ImageCacheKey, ImageEntry>,
}

/// Metadata for a cached image texture.
#[derive(Debug, Clone, Copy)]
pub struct ImageEntry {
    pub width: u32,
    pub height: u32,
    pub uploaded: bool,
}

impl ImageAtlas {
    pub fn new() -> Self {
        Self {
            cache: FxHashMap::default(),
        }
    }

    /// Checks if an image is already cached.
    pub fn contains(&self, key: ImageCacheKey) -> bool {
        self.cache.contains_key(&key)
    }

    /// Registers an image in the cache. Returns true if this is a new entry.
    pub fn insert(&mut self, key: ImageCacheKey, width: u32, height: u32) -> bool {
        if self.cache.contains_key(&key) {
            return false;
        }
        self.cache.insert(
            key,
            ImageEntry {
                width,
                height,
                uploaded: false,
            },
        );
        true
    }

    /// Marks an image as uploaded to the GPU.
    pub fn mark_uploaded(&mut self, key: ImageCacheKey) {
        if let Some(entry) = self.cache.get_mut(&key) {
            entry.uploaded = true;
        }
    }

    /// Returns the image entry if it exists.
    pub fn get(&self, key: ImageCacheKey) -> Option<&ImageEntry> {
        self.cache.get(&key)
    }

    /// Removes an image from the cache.
    pub fn remove(&mut self, key: ImageCacheKey) -> bool {
        self.cache.remove(&key).is_some()
    }

    /// Returns the number of cached images.
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Returns true if the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    /// Clears the entire image cache.
    pub fn clear(&mut self) {
        self.cache.clear();
    }
}

impl Default for ImageAtlas {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glyph_atlas_insert_and_lookup() {
        let mut atlas = GlyphAtlas::with_size(256);
        let key = GlyphCacheKey::new(65, 1234, 16.0, 1.0);

        assert!(atlas.get(key).is_none());

        let region = atlas.insert(key, 12, 16);
        assert_eq!(region.page, 0);
        assert_eq!(region.x, 0);
        assert_eq!(region.y, 0);
        assert_eq!(region.width, 12);
        assert_eq!(region.height, 16);

        // Second lookup should be a cache hit
        let cached = atlas.get(key);
        assert!(cached.is_some());
        assert_eq!(cached.unwrap(), region);

        let (lookups, hits) = atlas.stats();
        assert_eq!(lookups, 2);
        assert_eq!(hits, 1);
    }

    #[test]
    fn glyph_atlas_duplicate_insert_returns_existing() {
        let mut atlas = GlyphAtlas::with_size(256);
        let key = GlyphCacheKey::new(65, 1234, 16.0, 1.0);

        let r1 = atlas.insert(key, 12, 16);
        let r2 = atlas.insert(key, 12, 16);
        assert_eq!(r1, r2);
    }

    #[test]
    fn glyph_atlas_shelf_packing() {
        let mut atlas = GlyphAtlas::with_size(64);

        // Insert several glyphs that should pack on one shelf
        let k1 = GlyphCacheKey::new(1, 0, 16.0, 1.0);
        let k2 = GlyphCacheKey::new(2, 0, 16.0, 1.0);
        let k3 = GlyphCacheKey::new(3, 0, 16.0, 1.0);

        let r1 = atlas.insert(k1, 10, 12);
        let r2 = atlas.insert(k2, 10, 12);
        let r3 = atlas.insert(k3, 10, 12);

        // All on same page, same shelf (same y), sequential x
        assert_eq!(r1.page, 0);
        assert_eq!(r2.page, 0);
        assert_eq!(r3.page, 0);
        assert_eq!(r1.y, r2.y);
        assert_eq!(r2.y, r3.y);
        assert_eq!(r1.x, 0);
        assert_eq!(r2.x, 10);
        assert_eq!(r3.x, 20);
    }

    #[test]
    fn glyph_atlas_new_shelf_on_overflow() {
        let mut atlas = GlyphAtlas::with_size(32);

        let k1 = GlyphCacheKey::new(1, 0, 16.0, 1.0);
        let k2 = GlyphCacheKey::new(2, 0, 16.0, 1.0);

        let r1 = atlas.insert(k1, 20, 10);
        let r2 = atlas.insert(k2, 20, 10);

        // r2 doesn't fit on the first shelf (20 + 20 > 32), so new shelf
        assert_eq!(r1.y, 0);
        assert_eq!(r2.y, 10);
    }

    #[test]
    fn glyph_atlas_new_page_on_full() {
        let mut atlas = GlyphAtlas::with_size(16);

        let k1 = GlyphCacheKey::new(1, 0, 16.0, 1.0);
        let k2 = GlyphCacheKey::new(2, 0, 16.0, 1.0);

        let r1 = atlas.insert(k1, 16, 16); // fills entire page
        let r2 = atlas.insert(k2, 8, 8); // needs new page

        assert_eq!(r1.page, 0);
        assert_eq!(r2.page, 1);
        assert_eq!(atlas.page_count(), 2);
    }

    #[test]
    fn glyph_atlas_clear() {
        let mut atlas = GlyphAtlas::with_size(256);
        let key = GlyphCacheKey::new(65, 1234, 16.0, 1.0);
        atlas.insert(key, 12, 16);
        atlas.get(key);

        atlas.clear();
        assert_eq!(atlas.page_count(), 1);
        let (lookups, hits) = atlas.stats();
        assert_eq!(lookups, 0);
        assert_eq!(hits, 0);

        // After clear, previously cached glyphs are gone
        assert!(atlas.get(key).is_none());
    }

    #[test]
    fn glyph_cache_key_quantization() {
        // Different float sizes that quantize to the same key
        let k1 = GlyphCacheKey::new(65, 100, 16.0, 1.0);
        let k2 = GlyphCacheKey::new(65, 100, 16.1, 1.0);
        // 16.0 * 4 = 64, 16.1 * 4 = 64.4 -> 64 as u32
        assert_eq!(k1, k2);

        // Different enough to produce different keys
        let k3 = GlyphCacheKey::new(65, 100, 16.5, 1.0);
        // 16.5 * 4 = 66
        assert_ne!(k1, k3);
    }

    #[test]
    fn atlas_region_uv_coordinates() {
        let region = AtlasRegion {
            page: 0,
            x: 0,
            y: 0,
            width: 128,
            height: 64,
        };
        let (u_min, v_min, u_max, v_max) = region.uv(256);
        assert_eq!(u_min, 0.0);
        assert_eq!(v_min, 0.0);
        assert_eq!(u_max, 0.5);
        assert_eq!(v_max, 0.25);
    }

    #[test]
    fn image_atlas_basic_operations() {
        let mut atlas = ImageAtlas::new();
        let key = ImageCacheKey(TextureId(1));

        assert!(!atlas.contains(key));
        assert!(atlas.is_empty());

        assert!(atlas.insert(key, 256, 256));
        assert!(atlas.contains(key));
        assert_eq!(atlas.len(), 1);

        // Duplicate insert returns false
        assert!(!atlas.insert(key, 256, 256));

        let entry = atlas.get(key).unwrap();
        assert_eq!(entry.width, 256);
        assert!(!entry.uploaded);

        atlas.mark_uploaded(key);
        assert!(atlas.get(key).unwrap().uploaded);

        assert!(atlas.remove(key));
        assert!(!atlas.contains(key));
        assert!(atlas.is_empty());
    }

    #[test]
    fn image_atlas_clear() {
        let mut atlas = ImageAtlas::new();
        atlas.insert(ImageCacheKey(TextureId(1)), 64, 64);
        atlas.insert(ImageCacheKey(TextureId(2)), 128, 128);
        assert_eq!(atlas.len(), 2);

        atlas.clear();
        assert!(atlas.is_empty());
    }
}
