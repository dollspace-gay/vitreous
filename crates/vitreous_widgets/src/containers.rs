use vitreous_a11y::Role;
use vitreous_reactive::provide_context;
use vitreous_style::Overflow;

use crate::into_nodes::IntoNodes;
use crate::node::{FlexDirection, Node, NodeKind};

/// Vertical stack — arranges children top-to-bottom (column direction).
pub fn v_stack(children: impl IntoNodes) -> Node {
    let mut node = Node::with_children(NodeKind::Container, children.into_nodes());
    node.flex_direction = FlexDirection::Column;
    node
}

/// Horizontal stack — arranges children left-to-right (row direction).
pub fn h_stack(children: impl IntoNodes) -> Node {
    let mut node = Node::with_children(NodeKind::Container, children.into_nodes());
    node.flex_direction = FlexDirection::Row;
    node
}

/// Z-stack — overlays children on top of each other (last child on top).
///
/// All children are positioned absolutely within the container.
pub fn z_stack(children: impl IntoNodes) -> Node {
    Node::with_children(NodeKind::Container, children.into_nodes())
}

/// A scrollable container.
///
/// Default role: `ScrollView`, with scroll actions.
pub fn scroll_view(children: impl IntoNodes) -> Node {
    let mut node = Node::with_children(NodeKind::Container, children.into_nodes());
    node.a11y.role = Role::ScrollView;
    node.style.overflow = Overflow::Scroll;
    node.a11y
        .actions
        .push(vitreous_a11y::AccessibilityAction::ScrollUp);
    node.a11y
        .actions
        .push(vitreous_a11y::AccessibilityAction::ScrollDown);
    node
}

/// A generic container node wrapping children.
pub fn container(children: impl IntoNodes) -> Node {
    Node::with_children(NodeKind::Container, children.into_nodes())
}

/// An overlay container — renders children above the normal flow.
///
/// Intended for modals, popovers, and floating UI. The platform layer
/// promotes this to a separate rendering layer.
pub fn overlay(children: impl IntoNodes) -> Node {
    let mut node = Node::with_children(NodeKind::Container, children.into_nodes());
    node.a11y.role = Role::Dialog;
    node.a11y.state.modal = true;
    node
}

/// A tooltip attached to content. First child is the trigger, second is the
/// tooltip content shown on hover.
///
/// Default role: `Tooltip` on the tooltip content.
pub fn tooltip(trigger: Node, tip: Node) -> Node {
    let mut tip_node = tip;
    tip_node.a11y.role = Role::Tooltip;
    Node::with_children(NodeKind::Container, vec![trigger, tip_node])
}

/// Provide a context value to all descendants.
///
/// Wraps `vitreous_reactive::provide_context` and returns a container
/// containing the children. The context is available via `use_context<T>()`
/// in any descendant widget function.
pub fn provider<T: Clone + 'static>(value: T, children: impl IntoNodes) -> Node {
    provide_context(value);
    Node::with_children(NodeKind::Container, children.into_nodes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::{button, text};

    #[test]
    fn v_stack_column_direction() {
        let node = v_stack((text("a"), text("b")));
        assert_eq!(node.flex_direction, FlexDirection::Column);
        assert_eq!(node.children.len(), 2);
    }

    #[test]
    fn h_stack_row_direction() {
        let node = h_stack((text("a"), text("b")));
        assert_eq!(node.flex_direction, FlexDirection::Row);
        assert_eq!(node.children.len(), 2);
    }

    #[test]
    fn z_stack_children() {
        let node = z_stack((text("bg"), text("fg")));
        assert_eq!(node.children.len(), 2);
    }

    #[test]
    fn scroll_view_role_and_overflow() {
        let node = scroll_view(text("content"));
        assert_eq!(node.a11y.role, Role::ScrollView);
        assert_eq!(node.style.overflow, Overflow::Scroll);
        assert_eq!(node.children.len(), 1);
    }

    #[test]
    fn container_wraps_children() {
        let node = container((text("a"), text("b"), text("c")));
        assert_eq!(node.children.len(), 3);
    }

    #[test]
    fn overlay_is_modal_dialog() {
        let node = overlay(text("modal content"));
        assert_eq!(node.a11y.role, Role::Dialog);
        assert!(node.a11y.state.modal);
    }

    #[test]
    fn tooltip_sets_role() {
        let node = tooltip(button("hover me"), text("tip text"));
        assert_eq!(node.children.len(), 2);
        assert_eq!(node.children[1].a11y.role, Role::Tooltip);
    }

    #[test]
    fn v_stack_with_empty_tuple() {
        let node = v_stack(());
        assert!(node.children.is_empty());
    }

    #[test]
    fn v_stack_with_vec() {
        let items: Vec<Node> = vec![text("a"), text("b")];
        let node = v_stack(items);
        assert_eq!(node.children.len(), 2);
    }
}
