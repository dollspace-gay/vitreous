use crate::types::NodeId;

/// A 2D point in logical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

/// An axis-aligned rectangle in logical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    /// Returns `true` if the point is inside this rectangle.
    pub fn contains(&self, point: Point) -> bool {
        point.x >= self.x
            && point.x <= self.x + self.width
            && point.y >= self.y
            && point.y <= self.y + self.height
    }
}

/// Border radii for the four corners of a rectangle.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Corners {
    pub top_left: f64,
    pub top_right: f64,
    pub bottom_left: f64,
    pub bottom_right: f64,
}

/// A node's layout rectangle with its identifier.
#[derive(Clone, Copy, Debug)]
pub struct LayoutRect {
    pub id: NodeId,
    pub rect: Rect,
}

/// Returns `true` if `point` is inside a rounded rectangle.
///
/// For each corner with a radius > 0, checks whether the point falls within
/// the quarter-circle. Points in the corner bounding box but outside the
/// arc are rejected.
fn point_inside_rounded_rect(point: Point, rect: Rect, corners: Corners) -> bool {
    let px = point.x - rect.x;
    let py = point.y - rect.y;
    let w = rect.width;
    let h = rect.height;

    // Top-left corner
    if px < corners.top_left && py < corners.top_left {
        let dx = corners.top_left - px;
        let dy = corners.top_left - py;
        if dx * dx + dy * dy > corners.top_left * corners.top_left {
            return false;
        }
    }

    // Top-right corner
    if px > w - corners.top_right && py < corners.top_right {
        let dx = px - (w - corners.top_right);
        let dy = corners.top_right - py;
        if dx * dx + dy * dy > corners.top_right * corners.top_right {
            return false;
        }
    }

    // Bottom-left corner
    if px < corners.bottom_left && py > h - corners.bottom_left {
        let dx = corners.bottom_left - px;
        let dy = py - (h - corners.bottom_left);
        if dx * dx + dy * dy > corners.bottom_left * corners.bottom_left {
            return false;
        }
    }

    // Bottom-right corner
    if px > w - corners.bottom_right && py > h - corners.bottom_right {
        let dx = px - (w - corners.bottom_right);
        let dy = py - (h - corners.bottom_right);
        if dx * dx + dy * dy > corners.bottom_right * corners.bottom_right {
            return false;
        }
    }

    true
}

/// Determines which node is under a given screen coordinate.
///
/// Walks nodes in reverse paint order (last painted = frontmost).
/// Returns the deepest node whose layout rect contains the point,
/// respecting rounded corners.
pub fn hit_test(point: Point, nodes: &[LayoutRect], border_radii: &[Corners]) -> Option<NodeId> {
    for (i, node) in nodes.iter().enumerate().rev() {
        if node.rect.contains(point) && point_inside_rounded_rect(point, node.rect, border_radii[i])
        {
            return Some(node.id);
        }
    }
    None
}
