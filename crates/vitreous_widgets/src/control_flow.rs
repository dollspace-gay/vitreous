use crate::node::{Key, Node, NodeKind};

/// Conditionally render a node. Returns the rendered node when `when` is true,
/// or an empty container when false.
pub fn show(when: bool, then: impl FnOnce() -> Node) -> Node {
    if when {
        then()
    } else {
        Node::new(NodeKind::Container)
    }
}

/// Conditionally render one of two nodes.
pub fn show_else(
    when: bool,
    then: impl FnOnce() -> Node,
    otherwise: impl FnOnce() -> Node,
) -> Node {
    if when { then() } else { otherwise() }
}

/// Render a list of items with keyed reconciliation.
///
/// - `items`: the current list of data items
/// - `key_fn`: extracts a stable, unique key from each item
/// - `render`: produces a `Node` from each item
///
/// Each rendered node is tagged with the key so the diffing engine can
/// efficiently insert, remove, and reorder nodes without rebuilding the
/// entire list.
pub fn for_each<T, K>(items: Vec<T>, key_fn: impl Fn(&T) -> K, render: impl Fn(&T) -> Node) -> Node
where
    K: Into<Key>,
{
    let children: Vec<Node> = items
        .iter()
        .map(|item| {
            let key = key_fn(item).into();
            render(item).key(key)
        })
        .collect();

    Node::with_children(NodeKind::Container, children)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::text;

    #[test]
    fn show_true_renders() {
        let node = show(true, || text("visible"));
        match &node.kind {
            NodeKind::Text(_) => {}
            _ => panic!("expected text node"),
        }
    }

    #[test]
    fn show_false_empty() {
        let node = show(false, || text("hidden"));
        match &node.kind {
            NodeKind::Container => assert!(node.children.is_empty()),
            _ => panic!("expected empty container"),
        }
    }

    #[test]
    fn show_else_true() {
        let node = show_else(true, || text("yes"), || text("no"));
        match &node.kind {
            NodeKind::Text(tc) => match tc {
                crate::node::TextContent::Static(s) => assert_eq!(s, "yes"),
                _ => panic!("expected static text"),
            },
            _ => panic!("expected text node"),
        }
    }

    #[test]
    fn show_else_false() {
        let node = show_else(false, || text("yes"), || text("no"));
        match &node.kind {
            NodeKind::Text(tc) => match tc {
                crate::node::TextContent::Static(s) => assert_eq!(s, "no"),
                _ => panic!("expected static text"),
            },
            _ => panic!("expected text node"),
        }
    }

    #[test]
    fn for_each_renders_keyed_children() {
        let items = vec!["apple", "banana", "cherry"];
        let node = for_each(items, |item| *item, |item| text(*item));

        assert_eq!(node.children.len(), 3);
        assert_eq!(node.children[0].key, Some(Key::Str("apple".to_owned())));
        assert_eq!(node.children[1].key, Some(Key::Str("banana".to_owned())));
        assert_eq!(node.children[2].key, Some(Key::Str("cherry".to_owned())));
    }

    #[test]
    fn for_each_empty_items() {
        let items: Vec<&str> = vec![];
        let node = for_each(items, |item| *item, |item| text(*item));
        assert!(node.children.is_empty());
    }

    #[test]
    fn for_each_with_integer_keys() {
        #[derive(Clone)]
        struct Item {
            id: usize,
            name: String,
        }

        let items = vec![
            Item {
                id: 1,
                name: "first".into(),
            },
            Item {
                id: 2,
                name: "second".into(),
            },
        ];

        let node = for_each(items, |item| item.id, |item| text(item.name.as_str()));

        assert_eq!(node.children.len(), 2);
        assert_eq!(node.children[0].key, Some(Key::Int(1)));
        assert_eq!(node.children[1].key, Some(Key::Int(2)));
    }

    #[test]
    fn for_each_add_item_inserts_node() {
        let items_v1 = vec!["a", "b"];
        let node_v1 = for_each(items_v1, |i| *i, |i| text(*i));
        assert_eq!(node_v1.children.len(), 2);

        let items_v2 = vec!["a", "b", "c"];
        let node_v2 = for_each(items_v2, |i| *i, |i| text(*i));
        assert_eq!(node_v2.children.len(), 3);
    }

    #[test]
    fn for_each_remove_item_removes_node() {
        let items_v1 = vec!["a", "b", "c"];
        let node_v1 = for_each(items_v1, |i| *i, |i| text(*i));
        assert_eq!(node_v1.children.len(), 3);

        let items_v2 = vec!["a", "c"];
        let node_v2 = for_each(items_v2, |i| *i, |i| text(*i));
        assert_eq!(node_v2.children.len(), 2);
        assert_eq!(node_v2.children[0].key, Some(Key::Str("a".to_owned())));
        assert_eq!(node_v2.children[1].key, Some(Key::Str("c".to_owned())));
    }
}
