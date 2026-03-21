use crate::tree::{Dimension, LayoutInput, LayoutStyle, NodeId};

/// Returns `true` if this node is a layout boundary.
///
/// A layout boundary has both explicit width AND explicit height (not `Auto`),
/// meaning its own size is fully determined regardless of its children.
/// When a dirty subtree is below a layout boundary, re-layout can start from
/// the boundary node rather than propagating all the way to the root.
pub fn is_layout_boundary(style: &LayoutStyle) -> bool {
    is_explicit_dimension(&style.width) && is_explicit_dimension(&style.height)
}

fn is_explicit_dimension(d: &Dimension) -> bool {
    matches!(d, Dimension::Px(_) | Dimension::Percent(_))
}

/// Finds the nearest layout boundary ancestor for a given node.
///
/// Walks from `node_id` up through the tree (using parent references derived
/// from children lists) and returns the first ancestor whose style is a layout
/// boundary. Returns `None` if no ancestor is a boundary (re-layout must start
/// from root).
pub fn find_boundary_ancestor(nodes: &[LayoutInput], node_id: NodeId) -> Option<NodeId> {
    // Build a parent map: child -> parent
    let parent_map = build_parent_map(nodes);

    let mut current = node_id;
    while let Some(&parent_id) = parent_map.get(&current) {
        if let Some(parent_node) = nodes.iter().find(|n| n.id == parent_id)
            && is_layout_boundary(&parent_node.style)
        {
            return Some(parent_id);
        }
        current = parent_id;
    }

    None
}

/// Collects the set of node IDs that need re-layout given a set of dirty nodes.
///
/// For each dirty node, walks up to find the nearest layout boundary ancestor.
/// The boundary ancestor (or root if none found) is the re-layout root. Returns
/// all unique re-layout roots — layout only needs to run from each of these
/// downward.
pub fn find_relayout_roots(
    nodes: &[LayoutInput],
    dirty_nodes: &[NodeId],
    root_id: NodeId,
) -> Vec<NodeId> {
    let parent_map = build_parent_map(nodes);
    let mut roots = Vec::new();

    for &dirty_id in dirty_nodes {
        let relayout_root =
            find_boundary_ancestor_with_map(nodes, dirty_id, &parent_map).unwrap_or(root_id);

        if !roots.contains(&relayout_root) {
            roots.push(relayout_root);
        }
    }

    roots
}

fn build_parent_map(nodes: &[LayoutInput]) -> std::collections::HashMap<NodeId, NodeId> {
    let mut parent_map = std::collections::HashMap::new();
    for node in nodes {
        for &child_id in &node.children {
            parent_map.insert(child_id, node.id);
        }
    }
    parent_map
}

fn find_boundary_ancestor_with_map(
    nodes: &[LayoutInput],
    node_id: NodeId,
    parent_map: &std::collections::HashMap<NodeId, NodeId>,
) -> Option<NodeId> {
    let mut current = node_id;
    while let Some(&parent_id) = parent_map.get(&current) {
        if let Some(parent_node) = nodes.iter().find(|n| n.id == parent_id)
            && is_layout_boundary(&parent_node.style)
        {
            return Some(parent_id);
        }
        current = parent_id;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::{Dimension, LayoutInput, LayoutStyle, NodeId};

    fn px(v: f32) -> Dimension {
        Dimension::Px(v)
    }

    #[test]
    fn test_is_layout_boundary_both_explicit() {
        let style = LayoutStyle {
            width: px(200.0),
            height: px(100.0),
            ..Default::default()
        };
        assert!(is_layout_boundary(&style));
    }

    #[test]
    fn test_is_layout_boundary_percent() {
        let style = LayoutStyle {
            width: Dimension::Percent(50.0),
            height: Dimension::Percent(25.0),
            ..Default::default()
        };
        assert!(is_layout_boundary(&style));
    }

    #[test]
    fn test_is_not_boundary_auto_width() {
        let style = LayoutStyle {
            width: Dimension::Auto,
            height: px(100.0),
            ..Default::default()
        };
        assert!(!is_layout_boundary(&style));
    }

    #[test]
    fn test_is_not_boundary_auto_height() {
        let style = LayoutStyle {
            width: px(200.0),
            height: Dimension::Auto,
            ..Default::default()
        };
        assert!(!is_layout_boundary(&style));
    }

    #[test]
    fn test_is_not_boundary_both_auto() {
        let style = LayoutStyle::default();
        assert!(!is_layout_boundary(&style));
    }

    // AC-8: Layout boundary stops upward propagation
    #[test]
    fn test_find_boundary_ancestor() {
        // Tree: root (auto) -> boundary (200x100) -> parent (auto) -> leaf
        let root =
            LayoutInput::new(NodeId(0), LayoutStyle::default()).with_children(vec![NodeId(1)]);

        let boundary = LayoutInput::new(
            NodeId(1),
            LayoutStyle {
                width: px(200.0),
                height: px(100.0),
                ..Default::default()
            },
        )
        .with_children(vec![NodeId(2)]);

        let parent =
            LayoutInput::new(NodeId(2), LayoutStyle::default()).with_children(vec![NodeId(3)]);

        let leaf = LayoutInput::new(NodeId(3), LayoutStyle::default());

        let nodes = [root, boundary, parent, leaf];

        // From leaf, nearest boundary ancestor should be NodeId(1)
        let ancestor = find_boundary_ancestor(&nodes, NodeId(3));
        assert_eq!(ancestor, Some(NodeId(1)));
    }

    #[test]
    fn test_find_boundary_ancestor_none() {
        // All auto-sized — no boundary
        let root =
            LayoutInput::new(NodeId(0), LayoutStyle::default()).with_children(vec![NodeId(1)]);
        let child = LayoutInput::new(NodeId(1), LayoutStyle::default());

        let nodes = [root, child];
        let ancestor = find_boundary_ancestor(&nodes, NodeId(1));
        assert_eq!(ancestor, None);
    }

    #[test]
    fn test_find_relayout_roots() {
        // root (auto) -> boundary (200x100) -> child (auto)
        let root =
            LayoutInput::new(NodeId(0), LayoutStyle::default()).with_children(vec![NodeId(1)]);
        let boundary = LayoutInput::new(
            NodeId(1),
            LayoutStyle {
                width: px(200.0),
                height: px(100.0),
                ..Default::default()
            },
        )
        .with_children(vec![NodeId(2)]);
        let child = LayoutInput::new(NodeId(2), LayoutStyle::default());

        let nodes = [root, boundary, child];

        // Dirty node below boundary — relayout root should be the boundary
        let roots = find_relayout_roots(&nodes, &[NodeId(2)], NodeId(0));
        assert_eq!(roots, vec![NodeId(1)]);
    }

    #[test]
    fn test_find_relayout_roots_falls_back_to_root() {
        // No boundaries — falls back to root
        let root =
            LayoutInput::new(NodeId(0), LayoutStyle::default()).with_children(vec![NodeId(1)]);
        let child = LayoutInput::new(NodeId(1), LayoutStyle::default());

        let nodes = [root, child];

        let roots = find_relayout_roots(&nodes, &[NodeId(1)], NodeId(0));
        assert_eq!(roots, vec![NodeId(0)]);
    }

    #[test]
    fn test_find_relayout_roots_deduplicates() {
        // Two dirty nodes under same boundary
        let root =
            LayoutInput::new(NodeId(0), LayoutStyle::default()).with_children(vec![NodeId(1)]);
        let boundary = LayoutInput::new(
            NodeId(1),
            LayoutStyle {
                width: px(200.0),
                height: px(100.0),
                ..Default::default()
            },
        )
        .with_children(vec![NodeId(2), NodeId(3)]);
        let child_a = LayoutInput::new(NodeId(2), LayoutStyle::default());
        let child_b = LayoutInput::new(NodeId(3), LayoutStyle::default());

        let nodes = [root, boundary, child_a, child_b];

        let roots = find_relayout_roots(&nodes, &[NodeId(2), NodeId(3)], NodeId(0));
        assert_eq!(roots, vec![NodeId(1)]);
    }
}
