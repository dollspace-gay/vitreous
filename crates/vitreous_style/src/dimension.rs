/// A dimension value that can be pixels, a percentage, or auto-sized.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Dimension {
    Px(f32),
    Percent(f32),
    #[default]
    Auto,
}

impl From<f32> for Dimension {
    fn from(v: f32) -> Self {
        Self::Px(v)
    }
}

impl From<i32> for Dimension {
    fn from(v: i32) -> Self {
        Self::Px(v as f32)
    }
}

impl From<u32> for Dimension {
    fn from(v: u32) -> Self {
        Self::Px(v as f32)
    }
}

/// Create a percentage dimension.
pub fn pct(value: f32) -> Dimension {
    Dimension::Percent(value)
}

/// Spacing values for the four edges of a rectangle (top, right, bottom, left).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Edges {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl Default for Edges {
    fn default() -> Self {
        Self {
            top: 0.0,
            right: 0.0,
            bottom: 0.0,
            left: 0.0,
        }
    }
}

impl Edges {
    pub fn all(value: f32) -> Self {
        Self {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }

    pub fn symmetric(vertical: f32, horizontal: f32) -> Self {
        Self {
            top: vertical,
            right: horizontal,
            bottom: vertical,
            left: horizontal,
        }
    }

    pub fn new(top: f32, right: f32, bottom: f32, left: f32) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }
}

/// `From<f32>` sets all four sides to the same value.
impl From<f32> for Edges {
    fn from(v: f32) -> Self {
        Self::all(v)
    }
}

/// `From<(f32, f32)>` sets vertical (top/bottom) and horizontal (right/left).
impl From<(f32, f32)> for Edges {
    fn from((vertical, horizontal): (f32, f32)) -> Self {
        Self::symmetric(vertical, horizontal)
    }
}

/// `From<(f32, f32, f32, f32)>` sets (top, right, bottom, left).
impl From<(f32, f32, f32, f32)> for Edges {
    fn from((top, right, bottom, left): (f32, f32, f32, f32)) -> Self {
        Self::new(top, right, bottom, left)
    }
}

/// Corner radii for a rectangle (top_left, top_right, bottom_right, bottom_left).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Corners {
    pub top_left: f32,
    pub top_right: f32,
    pub bottom_right: f32,
    pub bottom_left: f32,
}

impl Default for Corners {
    fn default() -> Self {
        Self {
            top_left: 0.0,
            top_right: 0.0,
            bottom_right: 0.0,
            bottom_left: 0.0,
        }
    }
}

impl Corners {
    pub fn all(value: f32) -> Self {
        Self {
            top_left: value,
            top_right: value,
            bottom_right: value,
            bottom_left: value,
        }
    }

    pub fn new(top_left: f32, top_right: f32, bottom_right: f32, bottom_left: f32) -> Self {
        Self {
            top_left,
            top_right,
            bottom_right,
            bottom_left,
        }
    }
}

/// `From<f32>` sets all four corners to the same value.
impl From<f32> for Corners {
    fn from(v: f32) -> Self {
        Self::all(v)
    }
}

/// `From<(f32, f32)>` sets (top_left & bottom_right, top_right & bottom_left).
impl From<(f32, f32)> for Corners {
    fn from((a, b): (f32, f32)) -> Self {
        Self {
            top_left: a,
            top_right: b,
            bottom_right: a,
            bottom_left: b,
        }
    }
}

/// `From<(f32, f32, f32, f32)>` sets (top_left, top_right, bottom_right, bottom_left).
impl From<(f32, f32, f32, f32)> for Corners {
    fn from((tl, tr, br, bl): (f32, f32, f32, f32)) -> Self {
        Self::new(tl, tr, br, bl)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // AC-8
    #[test]
    fn dimension_from_f32() {
        assert_eq!(Dimension::from(16.0f32), Dimension::Px(16.0));
    }

    #[test]
    fn dimension_from_i32() {
        assert_eq!(Dimension::from(16i32), Dimension::Px(16.0));
    }

    #[test]
    fn dimension_from_u32() {
        assert_eq!(Dimension::from(16u32), Dimension::Px(16.0));
    }

    #[test]
    fn pct_helper() {
        assert_eq!(pct(50.0), Dimension::Percent(50.0));
    }

    #[test]
    fn dimension_default() {
        assert_eq!(Dimension::default(), Dimension::Auto);
    }

    // AC-9
    #[test]
    fn edges_from_f32() {
        let e = Edges::from(8.0);
        assert_eq!(e.top, 8.0);
        assert_eq!(e.right, 8.0);
        assert_eq!(e.bottom, 8.0);
        assert_eq!(e.left, 8.0);
    }

    #[test]
    fn edges_from_tuple2() {
        let e = Edges::from((8.0, 16.0));
        assert_eq!(e.top, 8.0);
        assert_eq!(e.bottom, 8.0);
        assert_eq!(e.right, 16.0);
        assert_eq!(e.left, 16.0);
    }

    #[test]
    fn edges_from_tuple4() {
        let e = Edges::from((1.0, 2.0, 3.0, 4.0));
        assert_eq!(e.top, 1.0);
        assert_eq!(e.right, 2.0);
        assert_eq!(e.bottom, 3.0);
        assert_eq!(e.left, 4.0);
    }

    #[test]
    fn edges_default() {
        let e = Edges::default();
        assert_eq!(e.top, 0.0);
        assert_eq!(e.right, 0.0);
        assert_eq!(e.bottom, 0.0);
        assert_eq!(e.left, 0.0);
    }

    #[test]
    fn corners_from_f32() {
        let c = Corners::from(4.0);
        assert_eq!(c.top_left, 4.0);
        assert_eq!(c.top_right, 4.0);
        assert_eq!(c.bottom_right, 4.0);
        assert_eq!(c.bottom_left, 4.0);
    }

    #[test]
    fn corners_from_tuple2() {
        let c = Corners::from((4.0, 8.0));
        assert_eq!(c.top_left, 4.0);
        assert_eq!(c.top_right, 8.0);
        assert_eq!(c.bottom_right, 4.0);
        assert_eq!(c.bottom_left, 8.0);
    }

    #[test]
    fn corners_from_tuple4() {
        let c = Corners::from((1.0, 2.0, 3.0, 4.0));
        assert_eq!(c.top_left, 1.0);
        assert_eq!(c.top_right, 2.0);
        assert_eq!(c.bottom_right, 3.0);
        assert_eq!(c.bottom_left, 4.0);
    }

    #[test]
    fn corners_default() {
        let c = Corners::default();
        assert_eq!(c.top_left, 0.0);
        assert_eq!(c.top_right, 0.0);
        assert_eq!(c.bottom_right, 0.0);
        assert_eq!(c.bottom_left, 0.0);
    }
}
