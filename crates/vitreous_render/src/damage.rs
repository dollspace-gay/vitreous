/// Axis-aligned damage rectangle in logical pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DamageRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl DamageRect {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Returns true if this rect overlaps another (with a margin for merging).
    fn overlaps_with_margin(&self, other: &Self, margin: f32) -> bool {
        self.x - margin < other.x + other.width + margin
            && self.x + self.width + margin > other.x - margin
            && self.y - margin < other.y + other.height + margin
            && self.y + self.height + margin > other.y - margin
    }

    /// Computes the union (bounding box) of this rect with another.
    fn union(&self, other: &Self) -> Self {
        let min_x = self.x.min(other.x);
        let min_y = self.y.min(other.y);
        let max_x = (self.x + self.width).max(other.x + other.width);
        let max_y = (self.y + self.height).max(other.y + other.height);
        Self {
            x: min_x,
            y: min_y,
            width: max_x - min_x,
            height: max_y - min_y,
        }
    }

    /// Returns true if this rect has zero or negative area.
    pub fn is_empty(&self) -> bool {
        self.width <= 0.0 || self.height <= 0.0
    }

    /// Clips this rect to be within the given viewport bounds.
    pub fn clip_to(&self, vp_width: f32, vp_height: f32) -> Self {
        let x0 = self.x.max(0.0);
        let y0 = self.y.max(0.0);
        let x1 = (self.x + self.width).min(vp_width);
        let y1 = (self.y + self.height).min(vp_height);
        Self {
            x: x0,
            y: y0,
            width: (x1 - x0).max(0.0),
            height: (y1 - y0).max(0.0),
        }
    }
}

/// Tracks damaged (changed) screen regions across frames.
///
/// Accepts raw damage rects, merges overlapping ones (with a small margin
/// to avoid excessive fragmentation), and provides the merged list for
/// scissor-rect GPU rendering.
pub struct DamageTracker {
    rects: Vec<DamageRect>,
    merge_margin: f32,
}

impl DamageTracker {
    /// Creates a new tracker. `merge_margin` controls how aggressively
    /// nearby rects are merged (in logical pixels).
    pub fn new(merge_margin: f32) -> Self {
        Self {
            rects: Vec::new(),
            merge_margin,
        }
    }

    /// Clears all tracked damage (call at frame start).
    pub fn clear(&mut self) {
        self.rects.clear();
    }

    /// Adds a damage region. Will be merged with existing rects if overlapping.
    pub fn add(&mut self, rect: DamageRect) {
        if rect.is_empty() {
            return;
        }
        self.rects.push(rect);
    }

    /// Returns true if no damage has been recorded this frame.
    pub fn is_clean(&self) -> bool {
        self.rects.is_empty()
    }

    /// Merges overlapping rects and returns the final damage list.
    ///
    /// Uses a greedy merging pass: repeatedly merge any two rects that
    /// overlap (within `merge_margin`) until no more merges occur.
    pub fn merged_rects(&self) -> Vec<DamageRect> {
        if self.rects.is_empty() {
            return Vec::new();
        }

        let mut merged = self.rects.clone();
        let mut changed = true;

        while changed {
            changed = false;
            let mut i = 0;
            while i < merged.len() {
                let mut j = i + 1;
                while j < merged.len() {
                    if merged[i].overlaps_with_margin(&merged[j], self.merge_margin) {
                        let union = merged[i].union(&merged[j]);
                        merged[i] = union;
                        merged.swap_remove(j);
                        changed = true;
                    } else {
                        j += 1;
                    }
                }
                i += 1;
            }
        }

        merged
    }

    /// Returns merged rects clipped to the given viewport dimensions.
    pub fn clipped_rects(&self, vp_width: f32, vp_height: f32) -> Vec<DamageRect> {
        self.merged_rects()
            .into_iter()
            .map(|r| r.clip_to(vp_width, vp_height))
            .filter(|r| !r.is_empty())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_tracker_is_clean() {
        let tracker = DamageTracker::new(4.0);
        assert!(tracker.is_clean());
        assert!(tracker.merged_rects().is_empty());
    }

    #[test]
    fn single_rect_passes_through() {
        let mut tracker = DamageTracker::new(4.0);
        tracker.add(DamageRect::new(10.0, 20.0, 100.0, 50.0));
        assert!(!tracker.is_clean());
        let rects = tracker.merged_rects();
        assert_eq!(rects.len(), 1);
        assert_eq!(rects[0].x, 10.0);
    }

    #[test]
    fn overlapping_rects_merged() {
        let mut tracker = DamageTracker::new(0.0);
        tracker.add(DamageRect::new(0.0, 0.0, 50.0, 50.0));
        tracker.add(DamageRect::new(30.0, 30.0, 50.0, 50.0));
        let rects = tracker.merged_rects();
        assert_eq!(rects.len(), 1);
        assert_eq!(rects[0].x, 0.0);
        assert_eq!(rects[0].y, 0.0);
        assert_eq!(rects[0].width, 80.0);
        assert_eq!(rects[0].height, 80.0);
    }

    #[test]
    fn non_overlapping_rects_stay_separate() {
        let mut tracker = DamageTracker::new(0.0);
        tracker.add(DamageRect::new(0.0, 0.0, 10.0, 10.0));
        tracker.add(DamageRect::new(100.0, 100.0, 10.0, 10.0));
        let rects = tracker.merged_rects();
        assert_eq!(rects.len(), 2);
    }

    #[test]
    fn margin_merges_nearby_rects() {
        let mut tracker = DamageTracker::new(10.0);
        // These rects are 5px apart — within the 10px margin
        tracker.add(DamageRect::new(0.0, 0.0, 20.0, 20.0));
        tracker.add(DamageRect::new(25.0, 0.0, 20.0, 20.0));
        let rects = tracker.merged_rects();
        assert_eq!(rects.len(), 1);
    }

    #[test]
    fn clear_resets_state() {
        let mut tracker = DamageTracker::new(4.0);
        tracker.add(DamageRect::new(0.0, 0.0, 50.0, 50.0));
        assert!(!tracker.is_clean());
        tracker.clear();
        assert!(tracker.is_clean());
    }

    #[test]
    fn empty_rects_ignored() {
        let mut tracker = DamageTracker::new(4.0);
        tracker.add(DamageRect::new(0.0, 0.0, 0.0, 50.0));
        tracker.add(DamageRect::new(0.0, 0.0, 50.0, 0.0));
        assert!(tracker.is_clean());
    }

    #[test]
    fn clipped_rects_respect_viewport() {
        let mut tracker = DamageTracker::new(0.0);
        tracker.add(DamageRect::new(-10.0, -10.0, 50.0, 50.0));
        let rects = tracker.clipped_rects(800.0, 600.0);
        assert_eq!(rects.len(), 1);
        assert_eq!(rects[0].x, 0.0);
        assert_eq!(rects[0].y, 0.0);
        assert_eq!(rects[0].width, 40.0);
        assert_eq!(rects[0].height, 40.0);
    }

    #[test]
    fn rect_fully_outside_viewport_filtered() {
        let mut tracker = DamageTracker::new(0.0);
        tracker.add(DamageRect::new(900.0, 700.0, 50.0, 50.0));
        let rects = tracker.clipped_rects(800.0, 600.0);
        assert!(rects.is_empty());
    }

    #[test]
    fn three_overlapping_rects_merge_into_one() {
        let mut tracker = DamageTracker::new(0.0);
        tracker.add(DamageRect::new(0.0, 0.0, 30.0, 30.0));
        tracker.add(DamageRect::new(20.0, 20.0, 30.0, 30.0));
        tracker.add(DamageRect::new(40.0, 40.0, 30.0, 30.0));
        let rects = tracker.merged_rects();
        assert_eq!(rects.len(), 1);
        assert_eq!(rects[0].x, 0.0);
        assert_eq!(rects[0].y, 0.0);
        assert_eq!(rects[0].width, 70.0);
        assert_eq!(rects[0].height, 70.0);
    }
}
