use accesskit::{Node as AkNode, NodeId as AkNodeId, Toggled, Tree, TreeId, TreeUpdate};
use vitreous_events::NodeId;

use crate::roles::{
    AccessibilityAction, AccessibilityInfo, CheckedState, ExpandedState, LivePoliteness, Role,
};

/// A node in the accessibility tree snapshot.
///
/// The widgets layer constructs this tree from its `Node` tree and passes it
/// to [`generate_accesskit_tree`] for conversion to an `accesskit::TreeUpdate`.
#[derive(Clone, Debug)]
pub struct A11yNode {
    pub id: NodeId,
    pub info: AccessibilityInfo,
    pub children: Vec<A11yNode>,
}

/// Convert a vitreous `Role` to the corresponding `accesskit::Role`.
fn map_role(role: &Role) -> accesskit::Role {
    match role {
        Role::Button => accesskit::Role::Button,
        Role::Checkbox => accesskit::Role::CheckBox,
        Role::Dialog => accesskit::Role::Dialog,
        Role::Grid => accesskit::Role::Grid,
        Role::GridCell => accesskit::Role::GridCell,
        Role::Heading => accesskit::Role::Heading,
        Role::Image => accesskit::Role::Image,
        Role::Link => accesskit::Role::Link,
        Role::List => accesskit::Role::List,
        Role::ListItem => accesskit::Role::ListItem,
        Role::Menu => accesskit::Role::Menu,
        Role::MenuItem => accesskit::Role::MenuItem,
        Role::ProgressBar => accesskit::Role::ProgressIndicator,
        Role::RadioButton => accesskit::Role::RadioButton,
        Role::ScrollView => accesskit::Role::ScrollView,
        Role::Slider => accesskit::Role::Slider,
        Role::Switch => accesskit::Role::Switch,
        Role::Tab => accesskit::Role::Tab,
        Role::TabList => accesskit::Role::TabList,
        Role::TabPanel => accesskit::Role::TabPanel,
        Role::TextInput => accesskit::Role::TextInput,
        Role::Text => accesskit::Role::Label,
        Role::Toolbar => accesskit::Role::Toolbar,
        Role::Tooltip => accesskit::Role::Tooltip,
        Role::Tree => accesskit::Role::Tree,
        Role::TreeItem => accesskit::Role::TreeItem,
        Role::Window => accesskit::Role::Window,
        Role::Group => accesskit::Role::Group,
        Role::None => accesskit::Role::GenericContainer,
    }
}

/// Convert a vitreous `AccessibilityAction` to `accesskit::Action`.
fn map_action(action: &AccessibilityAction) -> accesskit::Action {
    match action {
        AccessibilityAction::Click => accesskit::Action::Click,
        AccessibilityAction::Focus => accesskit::Action::Focus,
        AccessibilityAction::Blur => accesskit::Action::Blur,
        AccessibilityAction::Increment => accesskit::Action::Increment,
        AccessibilityAction::Decrement => accesskit::Action::Decrement,
        AccessibilityAction::ScrollUp => accesskit::Action::ScrollUp,
        AccessibilityAction::ScrollDown => accesskit::Action::ScrollDown,
        AccessibilityAction::ScrollLeft => accesskit::Action::ScrollLeft,
        AccessibilityAction::ScrollRight => accesskit::Action::ScrollRight,
        AccessibilityAction::Expand => accesskit::Action::Expand,
        AccessibilityAction::Collapse => accesskit::Action::Collapse,
        AccessibilityAction::SetValue => accesskit::Action::SetValue,
    }
}

/// Build an `accesskit::Node` from vitreous accessibility info.
///
/// Sets role, label, description, value, states, actions, and children.
fn build_accesskit_node(info: &AccessibilityInfo, child_ids: &[AkNodeId]) -> AkNode {
    let ak_role = map_role(&info.role);
    let mut node = AkNode::new(ak_role);

    // Children
    if !child_ids.is_empty() {
        node.set_children(child_ids.to_vec());
    }

    // String properties
    if let Some(label) = &info.label {
        node.set_label(label.clone());
    }
    if let Some(description) = &info.description {
        node.set_description(description.clone());
    }
    if let Some(value) = &info.value {
        node.set_value(value.clone());
    }

    // Live region
    match info.live {
        LivePoliteness::Off => {}
        LivePoliteness::Polite => {
            node.set_live(accesskit::Live::Polite);
        }
        LivePoliteness::Assertive => {
            node.set_live(accesskit::Live::Assertive);
        }
    }

    // State: disabled
    if info.state.disabled {
        node.set_disabled();
    }

    // State: selected
    if info.state.selected {
        node.set_selected(true);
    }

    // State: checked (tri-state → Toggled)
    if let Some(checked) = &info.state.checked {
        let toggled = match checked {
            CheckedState::Unchecked => Toggled::False,
            CheckedState::Checked => Toggled::True,
            CheckedState::Mixed => Toggled::Mixed,
        };
        node.set_toggled(toggled);
    }

    // State: expanded (tri-state)
    if let Some(expanded) = &info.state.expanded {
        match expanded {
            ExpandedState::Collapsed => node.set_expanded(false),
            ExpandedState::Expanded => node.set_expanded(true),
        }
    }

    // State: has_popup
    if info.state.has_popup {
        node.set_has_popup(accesskit::HasPopup::Menu);
    }

    // State: focusable
    if info.state.focusable {
        node.add_action(accesskit::Action::Focus);
    }

    // State: read_only
    if info.state.read_only {
        node.set_read_only();
    }

    // State: required
    if info.state.required {
        node.set_required();
    }

    // State: invalid
    if info.state.invalid {
        node.set_invalid(accesskit::Invalid::True);
    }

    // State: busy
    if info.state.busy {
        node.set_busy();
    }

    // State: modal
    if info.state.modal {
        node.set_modal();
    }

    // State: heading level
    if let Some(level) = info.state.level {
        node.set_level(level as usize);
    }

    // State: numeric value range (for sliders, progress bars)
    if let Some(min) = info.state.value_min {
        node.set_min_numeric_value(min);
    }
    if let Some(max) = info.state.value_max {
        node.set_max_numeric_value(max);
    }
    if let Some(now) = info.state.value_now {
        node.set_numeric_value(now);
    }

    // Actions
    for action in &info.actions {
        node.add_action(map_action(action));
    }

    // Role::None → hidden from AT
    if info.role == Role::None {
        node.set_hidden();
    }

    node
}

/// Convert a vitreous `NodeId` to an `accesskit::NodeId`.
fn to_ak_id(id: NodeId) -> AkNodeId {
    AkNodeId(id.0)
}

/// Recursively collect all nodes from the tree into a flat list of `(NodeId, Node)` pairs.
fn collect_nodes(a11y_node: &A11yNode, output: &mut Vec<(AkNodeId, AkNode)>) {
    let child_ids: Vec<AkNodeId> = a11y_node.children.iter().map(|c| to_ak_id(c.id)).collect();
    let ak_node = build_accesskit_node(&a11y_node.info, &child_ids);
    output.push((to_ak_id(a11y_node.id), ak_node));

    for child in &a11y_node.children {
        collect_nodes(child, output);
    }
}

/// Generate an `accesskit::TreeUpdate` from a vitreous accessibility tree.
///
/// Walks the tree depth-first, converting each node's `AccessibilityInfo`
/// to AccessKit properties. The root node of the tree becomes the AccessKit
/// tree root.
///
/// # Arguments
///
/// * `root` - The root of the accessibility tree snapshot.
/// * `focused` - The `NodeId` of the currently focused node, or the root ID if nothing is focused.
/// * `initial` - If `true`, includes a `Tree` struct (required for the first update).
pub fn generate_accesskit_tree(root: &A11yNode, focused: NodeId, initial: bool) -> TreeUpdate {
    let mut nodes = Vec::new();
    collect_nodes(root, &mut nodes);

    let root_ak_id = to_ak_id(root.id);

    TreeUpdate {
        nodes,
        tree: if initial {
            Some(Tree::new(root_ak_id))
        } else {
            Option::None
        },
        tree_id: TreeId::ROOT,
        focus: to_ak_id(focused),
    }
}
