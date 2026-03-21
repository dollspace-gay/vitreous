use crate::types::{Corners, LayoutNode, NodeId, Point, Rect};

/// Performs hit testing on a flat list of layout nodes in **paint order**
/// (first element = backmost, last element = frontmost).
///
/// Returns the [`NodeId`] of the frontmost node whose (potentially rounded)
/// bounding rect contains `point`, or `None` if no node is hit.
pub fn hit_test(point: Point, nodes: &[LayoutNode]) -> Option<NodeId> {
    // Walk in reverse paint order so the frontmost (last-painted) node wins.
    for node in nodes.iter().rev() {
        if node.rect.contains(point) && point_inside_rounded_rect(point, node.rect, node.corners) {
            return Some(node.id);
        }
    }
    None
}

/// Returns `true` if `point` is inside `rect` accounting for rounded corners
/// defined by `corners`. The caller must already have verified that `point` is
/// within the axis-aligned `rect`.
///
/// For each corner with radius > 0, we check whether the point falls in the
/// square corner region **and** outside the inscribed quarter-circle. If so,
/// the point is in the transparent area clipped by the border radius.
fn point_inside_rounded_rect(point: Point, rect: Rect, corners: Corners) -> bool {
    let left = rect.x;
    let right = rect.x + rect.width;
    let top = rect.y;
    let bottom = rect.y + rect.height;

    // Top-left corner
    if corners.top_left > 0.0 {
        let r = corners.top_left;
        let cx = left + r;
        let cy = top + r;
        if point.x < cx && point.y < cy && !inside_quarter_circle(point, cx, cy, r) {
            return false;
        }
    }

    // Top-right corner
    if corners.top_right > 0.0 {
        let r = corners.top_right;
        let cx = right - r;
        let cy = top + r;
        if point.x > cx && point.y < cy && !inside_quarter_circle(point, cx, cy, r) {
            return false;
        }
    }

    // Bottom-right corner
    if corners.bottom_right > 0.0 {
        let r = corners.bottom_right;
        let cx = right - r;
        let cy = bottom - r;
        if point.x > cx && point.y > cy && !inside_quarter_circle(point, cx, cy, r) {
            return false;
        }
    }

    // Bottom-left corner
    if corners.bottom_left > 0.0 {
        let r = corners.bottom_left;
        let cx = left + r;
        let cy = bottom - r;
        if point.x < cx && point.y > cy && !inside_quarter_circle(point, cx, cy, r) {
            return false;
        }
    }

    true
}

/// Returns `true` if the distance from `point` to `(cx, cy)` is at most `r`.
fn inside_quarter_circle(point: Point, cx: f64, cy: f64, r: f64) -> bool {
    let dx = point.x - cx;
    let dy = point.y - cy;
    dx * dx + dy * dy <= r * r
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Corners;

    fn node(id: usize, x: f64, y: f64, w: f64, h: f64, corners: Corners) -> LayoutNode {
        LayoutNode {
            id: NodeId(id),
            rect: Rect::new(x, y, w, h),
            corners,
        }
    }

    // AC-3: nested rects — point (30,30) hits inner, point (10,10) hits outer
    #[test]
    fn hit_test_nested_rects() {
        let nodes = vec![
            node(0, 0.0, 0.0, 100.0, 100.0, Corners::zero()),
            node(1, 25.0, 25.0, 50.0, 50.0, Corners::zero()),
        ];

        assert_eq!(hit_test(Point::new(30.0, 30.0), &nodes), Some(NodeId(1)));
        assert_eq!(hit_test(Point::new(10.0, 10.0), &nodes), Some(NodeId(0)));
    }

    // AC-4: overlapping siblings — later sibling (higher z) occludes earlier
    #[test]
    fn hit_test_overlapping_siblings() {
        let nodes = vec![
            node(0, 0.0, 0.0, 60.0, 60.0, Corners::zero()),
            node(1, 20.0, 20.0, 60.0, 60.0, Corners::zero()),
        ];

        // Point in overlap region — node 1 was painted later, so it wins
        assert_eq!(hit_test(Point::new(40.0, 40.0), &nodes), Some(NodeId(1)));
        // Point only in node 0
        assert_eq!(hit_test(Point::new(5.0, 5.0), &nodes), Some(NodeId(0)));
        // Point only in node 1
        assert_eq!(hit_test(Point::new(75.0, 75.0), &nodes), Some(NodeId(1)));
    }

    // AC-7: rounded corners — point in transparent corner area does NOT hit
    #[test]
    fn hit_test_respects_border_radius() {
        // A 100x100 rect with 50px radius on all corners (a circle inscribed in the rect).
        let nodes = vec![node(0, 0.0, 0.0, 100.0, 100.0, Corners::all(50.0))];

        // Center — hit
        assert_eq!(hit_test(Point::new(50.0, 50.0), &nodes), Some(NodeId(0)));

        // Near top-left corner — inside the rect but outside the quarter-circle
        // Point (1, 1): distance from center of TL circle (50, 50) = sqrt(49^2 + 49^2) ≈ 69.3 > 50
        assert_eq!(hit_test(Point::new(1.0, 1.0), &nodes), None);

        // Near top-right corner
        assert_eq!(hit_test(Point::new(99.0, 1.0), &nodes), None);

        // Near bottom-right corner
        assert_eq!(hit_test(Point::new(99.0, 99.0), &nodes), None);

        // Near bottom-left corner
        assert_eq!(hit_test(Point::new(1.0, 99.0), &nodes), None);

        // Edge of the circle area — on the circle boundary (should hit)
        // Top center: (50, 0) — distance from TL circle center (50, 50) = 50, exactly on boundary
        assert_eq!(hit_test(Point::new(50.0, 0.0), &nodes), Some(NodeId(0)));
    }

    #[test]
    fn hit_test_miss() {
        let nodes = vec![node(0, 10.0, 10.0, 50.0, 50.0, Corners::zero())];
        assert_eq!(hit_test(Point::new(0.0, 0.0), &nodes), None);
    }

    #[test]
    fn hit_test_empty_nodes() {
        assert_eq!(hit_test(Point::new(0.0, 0.0), &[]), None);
    }

    #[test]
    fn hit_test_partial_rounding() {
        // Only top-left corner is rounded (radius 20)
        let nodes = vec![node(
            0,
            0.0,
            0.0,
            100.0,
            100.0,
            Corners::new(20.0, 0.0, 0.0, 0.0),
        )];

        // Top-left corner area — outside the quarter-circle
        // Point (1,1): distance from (20,20) = sqrt(19^2+19^2) ≈ 26.87 > 20
        assert_eq!(hit_test(Point::new(1.0, 1.0), &nodes), None);

        // Top-right corner — not rounded, so (99, 1) should still hit
        assert_eq!(hit_test(Point::new(99.0, 1.0), &nodes), Some(NodeId(0)));
    }
}
