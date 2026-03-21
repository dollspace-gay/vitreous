use vitreous_events::NodeId;

use crate::tree::A11yNode;

/// Manages keyboard focus within the widget tree.
///
/// Computes tab order via depth-first traversal of nodes where
/// `state.focusable == true`. Provides `focus_next()` / `focus_previous()`
/// for Tab / Shift+Tab cycling, and direct `focus(id)` / `blur()`.
#[derive(Debug)]
pub struct FocusManager {
    /// Ordered list of focusable node IDs (depth-first document order).
    focus_order: Vec<NodeId>,
    /// Index into `focus_order` of the currently focused node, or `None`.
    focused_index: Option<usize>,
}

impl FocusManager {
    /// Build a new `FocusManager` from an accessibility tree.
    ///
    /// Computes tab order by depth-first traversal, keeping only nodes
    /// where `state.focusable == true`.
    pub fn new(root: &A11yNode) -> Self {
        let mut focus_order = Vec::new();
        collect_focusable(root, &mut focus_order);
        Self {
            focus_order,
            focused_index: None,
        }
    }

    /// Returns the `NodeId` of the currently focused node, if any.
    pub fn focused(&self) -> Option<NodeId> {
        self.focused_index.map(|i| self.focus_order[i])
    }

    /// Move focus to the next focusable node in tab order.
    ///
    /// Wraps from the last focusable element back to the first.
    /// If nothing is focused, focuses the first focusable node.
    /// Returns the newly focused `NodeId`, or `None` if there are no focusable nodes.
    pub fn focus_next(&mut self) -> Option<NodeId> {
        if self.focus_order.is_empty() {
            return None;
        }
        let next = match self.focused_index {
            Some(i) => (i + 1) % self.focus_order.len(),
            None => 0,
        };
        self.focused_index = Some(next);
        Some(self.focus_order[next])
    }

    /// Move focus to the previous focusable node in tab order.
    ///
    /// Wraps from the first focusable element to the last.
    /// If nothing is focused, focuses the last focusable node.
    /// Returns the newly focused `NodeId`, or `None` if there are no focusable nodes.
    pub fn focus_previous(&mut self) -> Option<NodeId> {
        if self.focus_order.is_empty() {
            return None;
        }
        let prev = match self.focused_index {
            Some(0) => self.focus_order.len() - 1,
            Some(i) => i - 1,
            None => self.focus_order.len() - 1,
        };
        self.focused_index = Some(prev);
        Some(self.focus_order[prev])
    }

    /// Focus a specific node by ID.
    ///
    /// Returns `true` if the node was found in the focus order and focused.
    /// Returns `false` if the node is not focusable (not in the focus order).
    pub fn focus(&mut self, id: NodeId) -> bool {
        if let Some(i) = self.focus_order.iter().position(|&n| n == id) {
            self.focused_index = Some(i);
            true
        } else {
            false
        }
    }

    /// Remove focus from the current node.
    pub fn blur(&mut self) {
        self.focused_index = None;
    }

    /// Returns the computed focus order (depth-first, focusable nodes only).
    pub fn focus_order(&self) -> &[NodeId] {
        &self.focus_order
    }

    /// Rebuild the focus order from a new tree snapshot.
    ///
    /// Preserves the currently focused node if it still exists in the new order.
    /// Otherwise, clears focus.
    pub fn rebuild(&mut self, root: &A11yNode) {
        let previously_focused = self.focused();
        self.focus_order.clear();
        collect_focusable(root, &mut self.focus_order);

        self.focused_index =
            previously_focused.and_then(|id| self.focus_order.iter().position(|&n| n == id));
    }
}

/// Depth-first traversal collecting focusable node IDs.
fn collect_focusable(node: &A11yNode, out: &mut Vec<NodeId>) {
    if node.info.state.focusable {
        out.push(node.id);
    }
    for child in &node.children {
        collect_focusable(child, out);
    }
}
