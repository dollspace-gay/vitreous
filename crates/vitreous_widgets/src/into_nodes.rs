use crate::node::Node;

/// Convert a single value into a `Node`.
pub trait IntoNode {
    fn into_node(self) -> Node;
}

impl IntoNode for Node {
    fn into_node(self) -> Node {
        self
    }
}

/// `()` produces an empty container node (useful for conditional rendering).
impl IntoNode for () {
    fn into_node(self) -> Node {
        use crate::node::NodeKind;
        Node::new(NodeKind::Container)
    }
}

/// Convert a value into a `Vec<Node>`.
pub trait IntoNodes {
    fn into_nodes(self) -> Vec<Node>;
}

impl IntoNodes for Node {
    fn into_nodes(self) -> Vec<Node> {
        vec![self]
    }
}

impl IntoNodes for () {
    fn into_nodes(self) -> Vec<Node> {
        Vec::new()
    }
}

impl IntoNodes for Vec<Node> {
    fn into_nodes(self) -> Vec<Node> {
        self
    }
}

impl<I: IntoNode> IntoNodes for std::vec::IntoIter<I> {
    fn into_nodes(self) -> Vec<Node> {
        self.map(IntoNode::into_node).collect()
    }
}

// ---------------------------------------------------------------------------
// Tuple impls via macro (1-tuple through 16-tuple)
// ---------------------------------------------------------------------------

macro_rules! impl_into_nodes_for_tuple {
    ($($idx:tt : $T:ident),+) => {
        impl<$($T: IntoNode),+> IntoNodes for ($($T,)+) {
            fn into_nodes(self) -> Vec<Node> {
                vec![$(self.$idx.into_node(),)+]
            }
        }
    };
}

impl_into_nodes_for_tuple!(0: A);
impl_into_nodes_for_tuple!(0: A, 1: B);
impl_into_nodes_for_tuple!(0: A, 1: B, 2: C);
impl_into_nodes_for_tuple!(0: A, 1: B, 2: C, 3: D);
impl_into_nodes_for_tuple!(0: A, 1: B, 2: C, 3: D, 4: E);
impl_into_nodes_for_tuple!(0: A, 1: B, 2: C, 3: D, 4: E, 5: F);
impl_into_nodes_for_tuple!(0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G);
impl_into_nodes_for_tuple!(0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G, 7: H);
impl_into_nodes_for_tuple!(0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G, 7: H, 8: I);
impl_into_nodes_for_tuple!(0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G, 7: H, 8: I, 9: J);
impl_into_nodes_for_tuple!(0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G, 7: H, 8: I, 9: J, 10: K);
impl_into_nodes_for_tuple!(0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G, 7: H, 8: I, 9: J, 10: K, 11: L);
impl_into_nodes_for_tuple!(0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G, 7: H, 8: I, 9: J, 10: K, 11: L, 12: M);
impl_into_nodes_for_tuple!(0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G, 7: H, 8: I, 9: J, 10: K, 11: L, 12: M, 13: N);
impl_into_nodes_for_tuple!(0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G, 7: H, 8: I, 9: J, 10: K, 11: L, 12: M, 13: N, 14: O);
impl_into_nodes_for_tuple!(0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G, 7: H, 8: I, 9: J, 10: K, 11: L, 12: M, 13: N, 14: O, 15: P);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::{NodeKind, TextContent};

    fn text_node(s: &str) -> Node {
        Node::new(NodeKind::Text(TextContent::Static(s.to_owned())))
    }

    #[test]
    fn unit_into_nodes_empty() {
        let nodes = ().into_nodes();
        assert!(nodes.is_empty());
    }

    #[test]
    fn single_node_into_nodes() {
        let nodes = text_node("a").into_nodes();
        assert_eq!(nodes.len(), 1);
    }

    #[test]
    fn vec_into_nodes() {
        let nodes = vec![text_node("a"), text_node("b")].into_nodes();
        assert_eq!(nodes.len(), 2);
    }

    #[test]
    fn tuple_2_into_nodes() {
        let nodes = (text_node("a"), text_node("b")).into_nodes();
        assert_eq!(nodes.len(), 2);
    }

    #[test]
    fn tuple_3_into_nodes() {
        let nodes = (text_node("a"), text_node("b"), text_node("c")).into_nodes();
        assert_eq!(nodes.len(), 3);
    }

    #[test]
    fn unit_into_node() {
        let node = ().into_node();
        matches!(node.kind, NodeKind::Container);
    }

    #[test]
    fn tuple_16_into_nodes() {
        let nodes = (
            text_node("1"),
            text_node("2"),
            text_node("3"),
            text_node("4"),
            text_node("5"),
            text_node("6"),
            text_node("7"),
            text_node("8"),
            text_node("9"),
            text_node("10"),
            text_node("11"),
            text_node("12"),
            text_node("13"),
            text_node("14"),
            text_node("15"),
            text_node("16"),
        )
            .into_nodes();
        assert_eq!(nodes.len(), 16);
    }
}
