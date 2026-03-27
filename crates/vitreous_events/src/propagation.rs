use crate::types::NodeId;

// ---------------------------------------------------------------------------
// Propagation context — threaded through handler calls
// ---------------------------------------------------------------------------

/// Mutable context passed to event handlers during propagation. Calling
/// [`stop_propagation`](Self::stop_propagation) halts the bubble-up walk so
/// that ancestor handlers are not invoked.
pub struct PropagationContext {
    stopped: bool,
}

impl PropagationContext {
    pub fn new() -> Self {
        Self { stopped: false }
    }

    pub fn stop_propagation(&mut self) {
        self.stopped = true;
    }

    pub fn is_stopped(&self) -> bool {
        self.stopped
    }
}

impl Default for PropagationContext {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tree trait — abstraction over whatever tree the caller owns
// ---------------------------------------------------------------------------

/// Trait that the propagation dispatcher uses to walk up the tree.
///
/// Implementors are typically the layout or widget tree. The trait is kept
/// deliberately minimal so that `vitreous_events` does not depend on any
/// other workspace crate.
pub trait EventTree {
    /// Returns the parent of `node`, or `None` if `node` is the root.
    fn parent(&self, node: NodeId) -> Option<NodeId>;
}

// ---------------------------------------------------------------------------
// Bubble-up dispatcher
// ---------------------------------------------------------------------------

/// Walks from `start` up through ancestors (via [`EventTree::parent`]),
/// invoking `handler` at each node. If the handler sets
/// [`PropagationContext::stop_propagation`], the walk halts immediately.
///
/// Returns the list of node IDs that were walked during propagation.
/// This includes all nodes from `start` up to the root (or until propagation
/// is stopped), regardless of whether a handler existed on each node.
pub fn bubble_event<T: EventTree>(
    tree: &T,
    start: NodeId,
    mut handler: impl FnMut(NodeId, &mut PropagationContext),
) -> Vec<NodeId> {
    let mut ctx = PropagationContext::new();
    let mut visited = Vec::new();
    let mut current = Some(start);

    while let Some(node) = current {
        handler(node, &mut ctx);
        visited.push(node);
        if ctx.is_stopped() {
            break;
        }
        current = tree.parent(node);
    }

    visited
}

/// Dispatches a keyboard event starting from the focused node. Semantically
/// identical to [`bubble_event`] — the only difference is that the starting
/// node is the focused node (supplied by the caller) rather than a hit-test
/// result.
pub fn dispatch_keyboard_event<T: EventTree>(
    tree: &T,
    focused: NodeId,
    handler: impl FnMut(NodeId, &mut PropagationContext),
) -> Vec<NodeId> {
    bubble_event(tree, focused, handler)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    /// A simple linear tree: node 0 is root, node 1's parent is 0, node 2's
    /// parent is 1, etc.
    struct LinearTree {
        /// parent[i] = parent of node i, or None if root.
        parents: Vec<Option<NodeId>>,
    }

    impl LinearTree {
        /// Creates a chain of `depth` nodes: 0 <- 1 <- 2 <- ... <- (depth-1)
        fn new(depth: usize) -> Self {
            let mut parents = Vec::with_capacity(depth);
            for i in 0..depth {
                if i == 0 {
                    parents.push(None);
                } else {
                    parents.push(Some(NodeId(i - 1)));
                }
            }
            Self { parents }
        }
    }

    impl EventTree for LinearTree {
        fn parent(&self, node: NodeId) -> Option<NodeId> {
            self.parents.get(node.0).copied().flatten()
        }
    }

    // AC-5: child handler fires, then parent handler fires for same event
    #[test]
    fn bubble_event_child_then_parent() {
        let tree = LinearTree::new(3); // 0 <- 1 <- 2
        let fired: Rc<RefCell<Vec<usize>>> = Rc::new(RefCell::new(Vec::new()));

        let fired_clone = Rc::clone(&fired);
        let visited = bubble_event(&tree, NodeId(2), |node, _ctx| {
            fired_clone.borrow_mut().push(node.0);
        });

        // Child (2) fires first, then parent (1), then grandparent (0)
        assert_eq!(*fired.borrow(), vec![2, 1, 0]);
        assert_eq!(visited, vec![NodeId(2), NodeId(1), NodeId(0)]);
    }

    // AC-6: stop_propagation prevents parent handler from firing
    #[test]
    fn stop_propagation_halts_bubble() {
        let tree = LinearTree::new(3); // 0 <- 1 <- 2
        let fired: Rc<RefCell<Vec<usize>>> = Rc::new(RefCell::new(Vec::new()));

        let fired_clone = Rc::clone(&fired);
        let visited = bubble_event(&tree, NodeId(2), |node, ctx| {
            fired_clone.borrow_mut().push(node.0);
            if node.0 == 2 {
                ctx.stop_propagation();
            }
        });

        // Only node 2 fires; propagation stopped before reaching 1 or 0
        assert_eq!(*fired.borrow(), vec![2]);
        assert_eq!(visited, vec![NodeId(2)]);
    }

    // AC-10: keyboard event dispatched to focused node ID, not hit-test point
    #[test]
    fn keyboard_dispatch_starts_at_focused_node() {
        let tree = LinearTree::new(4); // 0 <- 1 <- 2 <- 3
        let fired: Rc<RefCell<Vec<usize>>> = Rc::new(RefCell::new(Vec::new()));

        // Focus is on node 1 (not on the deepest node 3)
        let fired_clone = Rc::clone(&fired);
        let visited = dispatch_keyboard_event(&tree, NodeId(1), |node, _ctx| {
            fired_clone.borrow_mut().push(node.0);
        });

        // Starts at focused node 1, bubbles to root 0 — never visits 2 or 3
        assert_eq!(*fired.borrow(), vec![1, 0]);
        assert_eq!(visited, vec![NodeId(1), NodeId(0)]);
    }

    #[test]
    fn bubble_event_single_root_node() {
        let tree = LinearTree::new(1);
        let visited = bubble_event(&tree, NodeId(0), |_node, _ctx| {});
        assert_eq!(visited, vec![NodeId(0)]);
    }

    #[test]
    fn stop_propagation_at_root_still_fires_root() {
        let tree = LinearTree::new(2);
        let fired: Rc<RefCell<Vec<usize>>> = Rc::new(RefCell::new(Vec::new()));

        let fired_clone = Rc::clone(&fired);
        bubble_event(&tree, NodeId(1), |node, ctx| {
            fired_clone.borrow_mut().push(node.0);
            // Stop at node 1 (child)
            if node.0 == 1 {
                ctx.stop_propagation();
            }
        });

        assert_eq!(*fired.borrow(), vec![1]);
    }

    #[test]
    fn propagation_context_default() {
        let ctx = PropagationContext::default();
        assert!(!ctx.is_stopped());
    }
}
