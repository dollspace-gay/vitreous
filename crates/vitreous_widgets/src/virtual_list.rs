use vitreous_a11y::Role;
use vitreous_style::{Dimension, Overflow};

use crate::node::{Key, Node, NodeKind};

/// A virtualized list that only instantiates visible items.
///
/// Given a total `item_count`, a fixed `item_height`, and the
/// `viewport_height`, this function computes which items are visible
/// (plus a small buffer) and only calls `render` for those items.
/// Each rendered item is keyed by its index for efficient reconciliation.
///
/// # Arguments
///
/// - `item_count` — total number of items in the list
/// - `item_height` — height of each item in pixels
/// - `viewport_height` — visible height of the scroll container in pixels
/// - `scroll_offset` — current vertical scroll offset in pixels
/// - `render` — function that produces a `Node` for a given item index
pub fn virtual_list(
    item_count: usize,
    item_height: f32,
    viewport_height: f32,
    scroll_offset: f32,
    render: impl Fn(usize) -> Node,
) -> Node {
    let buffer = 2; // extra items above and below viewport

    let first_visible = if item_height > 0.0 {
        (scroll_offset / item_height).floor() as usize
    } else {
        0
    };
    let first_visible = first_visible.saturating_sub(buffer);

    let visible_count = if item_height > 0.0 {
        (viewport_height / item_height).ceil() as usize + 1
    } else {
        0
    };
    let last_visible = (first_visible + visible_count + buffer * 2).min(item_count);

    let total_height = item_count as f32 * item_height;

    let mut children = Vec::with_capacity(last_visible - first_visible);
    for i in first_visible..last_visible {
        let mut item_node = render(i);
        item_node.key = Some(Key::Int(i as u64));
        item_node.style.height = Dimension::Px(item_height);
        children.push(item_node);
    }

    // Inner container sized to the full list height, with items offset
    let mut inner = Node::with_children(NodeKind::Container, children);
    inner.style.height = Dimension::Px(total_height);

    // Outer scroll container
    let mut outer = Node::with_children(NodeKind::Container, vec![inner]);
    outer.style.height = Dimension::Px(viewport_height);
    outer.style.overflow = Overflow::Scroll;
    outer.a11y.role = Role::List;
    outer
        .a11y
        .actions
        .push(vitreous_a11y::AccessibilityAction::ScrollUp);
    outer
        .a11y
        .actions
        .push(vitreous_a11y::AccessibilityAction::ScrollDown);

    outer
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::text;

    #[test]
    fn virtual_list_only_instantiates_visible() {
        // 100,000 items, 20px each, 500px viewport, no scroll
        let node = virtual_list(100_000, 20.0, 500.0, 0.0, |i| {
            text(format!("Item {i}").as_str())
        });

        // Outer container
        assert_eq!(node.style.height, Dimension::Px(500.0));
        assert_eq!(node.a11y.role, Role::List);

        // Inner container has the full height
        let inner = &node.children[0];
        assert_eq!(inner.style.height, Dimension::Px(100_000.0 * 20.0));

        // Should only have ~30 items (26 visible + buffer), not 100,000
        let instantiated = inner.children.len();
        assert!(instantiated < 35, "expected ~30 items, got {instantiated}");
        assert!(
            instantiated > 20,
            "expected at least 20 items, got {instantiated}"
        );
    }

    #[test]
    fn virtual_list_scrolled_midway() {
        // Scroll to item 500 (offset = 10000px)
        let node = virtual_list(100_000, 20.0, 500.0, 10_000.0, |i| {
            text(format!("Item {i}").as_str())
        });

        let inner = &node.children[0];
        let first_key = &inner.children[0].key;
        let last_key = &inner.children[inner.children.len() - 1].key;

        // First visible should be around index 498 (500 - 2 buffer)
        match first_key {
            Some(Key::Int(idx)) => assert_eq!(*idx, 498),
            _ => panic!("expected int key"),
        }

        // Last visible should be around 530
        match last_key {
            Some(Key::Int(idx)) => assert!(
                *idx > 520 && *idx < 535,
                "last index {idx} out of expected range"
            ),
            _ => panic!("expected int key"),
        }
    }

    #[test]
    fn virtual_list_items_keyed() {
        let node = virtual_list(10, 20.0, 200.0, 0.0, |i| text(format!("Item {i}").as_str()));

        let inner = &node.children[0];
        for (i, child) in inner.children.iter().enumerate() {
            assert!(child.key.is_some(), "item {i} should have a key");
        }
    }

    #[test]
    fn virtual_list_small_list() {
        // All items fit in viewport
        let node = virtual_list(5, 20.0, 500.0, 0.0, |i| text(format!("Item {i}").as_str()));

        let inner = &node.children[0];
        assert_eq!(inner.children.len(), 5);
    }

    #[test]
    fn virtual_list_zero_items() {
        let node = virtual_list(0, 20.0, 500.0, 0.0, |i| text(format!("Item {i}").as_str()));

        let inner = &node.children[0];
        assert!(inner.children.is_empty());
    }
}
