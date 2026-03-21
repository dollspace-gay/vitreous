use vitreous_a11y::{
    A11yNode, A11yWarning, AccessibilityAction, AccessibilityInfo, AccessibilityState,
    CheckedState, ExpandedState, FocusManager, KeyboardAction, LivePoliteness, Role, SrgbColor,
    check_contrast, check_tree, contrast_ratio, default_keyboard_action, generate_accesskit_tree,
    key_event, meets_wcag_aa,
};
use vitreous_events::{Key, NodeId};

/// Helper: build an A11yNode with defaults.
fn node(id: usize, role: Role, label: Option<&str>, children: Vec<A11yNode>) -> A11yNode {
    let focusable = role.is_default_focusable();
    A11yNode {
        id: NodeId(id),
        info: AccessibilityInfo {
            role,
            label: label.map(|s| s.to_string()),
            state: AccessibilityState {
                focusable,
                ..Default::default()
            },
            ..Default::default()
        },
        children,
    }
}

/// Helper: build a non-focusable node (like plain text).
fn text_node(id: usize, label: &str) -> A11yNode {
    A11yNode {
        id: NodeId(id),
        info: AccessibilityInfo {
            role: Role::Text,
            label: Some(label.to_string()),
            ..Default::default()
        },
        children: vec![],
    }
}

// ---------------------------------------------------------------------------
// AC-1: AccessibilityInfo::default() has all fields as None/empty/false
// ---------------------------------------------------------------------------
#[test]
fn ac1_default_accessibility_info() {
    let info = AccessibilityInfo::default();
    assert_eq!(info.role, Role::None);
    assert!(info.label.is_none());
    assert!(info.description.is_none());
    assert!(info.value.is_none());
    assert_eq!(info.live, LivePoliteness::Off);
    assert!(info.actions.is_empty());

    let state = &info.state;
    assert!(!state.disabled);
    assert!(!state.selected);
    assert!(state.checked.is_none());
    assert!(state.expanded.is_none());
    assert!(!state.has_popup);
    assert!(!state.focusable);
    assert!(!state.focused);
    assert!(!state.read_only);
    assert!(!state.required);
    assert!(!state.invalid);
    assert!(!state.busy);
    assert!(!state.modal);
    assert!(state.level.is_none());
    assert!(state.value_min.is_none());
    assert!(state.value_max.is_none());
    assert!(state.value_now.is_none());
}

// ---------------------------------------------------------------------------
// AC-2: Focus order computation skips non-focusable elements
// v_stack with [button("A"), text("skip"), button("B"), text_input]
// produces focus order [A, B, text_input]
// ---------------------------------------------------------------------------
#[test]
fn ac2_focus_order_skips_non_focusable() {
    let tree = node(
        0,
        Role::Group,
        None,
        vec![
            node(1, Role::Button, Some("A"), vec![]),
            text_node(2, "skip"),
            node(3, Role::Button, Some("B"), vec![]),
            node(4, Role::TextInput, Some("input"), vec![]),
        ],
    );

    let mgr = FocusManager::new(&tree);
    let order = mgr.focus_order();
    assert_eq!(order, &[NodeId(1), NodeId(3), NodeId(4)]);
}

// ---------------------------------------------------------------------------
// AC-3: focus_next() from A -> B, focus_previous() from B -> A
// ---------------------------------------------------------------------------
#[test]
fn ac3_focus_next_and_previous() {
    let tree = node(
        0,
        Role::Group,
        None,
        vec![
            node(1, Role::Button, Some("A"), vec![]),
            text_node(2, "skip"),
            node(3, Role::Button, Some("B"), vec![]),
            node(4, Role::TextInput, Some("input"), vec![]),
        ],
    );

    let mut mgr = FocusManager::new(&tree);

    // focus_next from nothing -> A
    assert_eq!(mgr.focus_next(), Some(NodeId(1)));
    assert_eq!(mgr.focused(), Some(NodeId(1)));

    // focus_next from A -> B
    assert_eq!(mgr.focus_next(), Some(NodeId(3)));
    assert_eq!(mgr.focused(), Some(NodeId(3)));

    // focus_previous from B -> A
    assert_eq!(mgr.focus_previous(), Some(NodeId(1)));
    assert_eq!(mgr.focused(), Some(NodeId(1)));
}

// ---------------------------------------------------------------------------
// AC-4: focus_next() from last wraps to first
// ---------------------------------------------------------------------------
#[test]
fn ac4_focus_next_wraps() {
    let tree = node(
        0,
        Role::Group,
        None,
        vec![
            node(1, Role::Button, Some("A"), vec![]),
            node(2, Role::Button, Some("B"), vec![]),
        ],
    );

    let mut mgr = FocusManager::new(&tree);
    mgr.focus_next(); // -> A (index 0)
    mgr.focus_next(); // -> B (index 1)
    let wrapped = mgr.focus_next(); // -> wraps to A
    assert_eq!(wrapped, Some(NodeId(1)));
}

// ---------------------------------------------------------------------------
// AC-5: AccessKit tree for v_stack((text("Heading").role(Heading), button("Click")))
// has 3 nodes (root + 2 children) with correct roles
// ---------------------------------------------------------------------------
#[test]
fn ac5_accesskit_tree_structure_and_roles() {
    let tree = node(
        0,
        Role::Group,
        None,
        vec![
            {
                let mut n = text_node(1, "Heading");
                n.info.role = Role::Heading;
                n
            },
            node(2, Role::Button, Some("Click"), vec![]),
        ],
    );

    let update = generate_accesskit_tree(&tree, NodeId(0), true);

    // Should have 3 nodes
    assert_eq!(update.nodes.len(), 3);

    // Tree should be set (initial)
    assert!(update.tree.is_some());

    // Check roles
    let find_role = |id: u64| -> accesskit::Role {
        update
            .nodes
            .iter()
            .find(|(nid, _)| nid.0 == id)
            .map(|(_, n)| n.role())
            .unwrap()
    };

    assert_eq!(find_role(0), accesskit::Role::Group);
    assert_eq!(find_role(1), accesskit::Role::Heading);
    assert_eq!(find_role(2), accesskit::Role::Button);

    // Root should have 2 children
    let root_node = &update.nodes.iter().find(|(nid, _)| nid.0 == 0).unwrap().1;
    assert_eq!(root_node.children().len(), 2);
}

// ---------------------------------------------------------------------------
// AC-6: Button responds to Enter and Space by producing Click action
// ---------------------------------------------------------------------------
#[test]
fn ac6_button_enter_space_click() {
    let enter_action = default_keyboard_action(&Role::Button, &key_event(Key::Enter));
    assert_eq!(enter_action, KeyboardAction::Click);

    let space_action = default_keyboard_action(&Role::Button, &key_event(Key::Space));
    assert_eq!(space_action, KeyboardAction::Click);

    // Other keys produce no action
    let arrow_action = default_keyboard_action(&Role::Button, &key_event(Key::ArrowDown));
    assert_eq!(arrow_action, KeyboardAction::None);
}

// ---------------------------------------------------------------------------
// AC-7: Checkbox responds to Space by toggling
// ---------------------------------------------------------------------------
#[test]
fn ac7_checkbox_space_toggle() {
    let action = default_keyboard_action(&Role::Checkbox, &key_event(Key::Space));
    assert_eq!(action, KeyboardAction::Toggle);

    // Enter does NOT toggle checkbox (per ARIA pattern)
    let enter_action = default_keyboard_action(&Role::Checkbox, &key_event(Key::Enter));
    assert_eq!(enter_action, KeyboardAction::None);
}

// ---------------------------------------------------------------------------
// AC-8: Slider responds to Left/Right arrows
// ---------------------------------------------------------------------------
#[test]
fn ac8_slider_arrow_keys() {
    let right = default_keyboard_action(&Role::Slider, &key_event(Key::ArrowRight));
    assert_eq!(right, KeyboardAction::Increment);

    let left = default_keyboard_action(&Role::Slider, &key_event(Key::ArrowLeft));
    assert_eq!(left, KeyboardAction::Decrement);

    let up = default_keyboard_action(&Role::Slider, &key_event(Key::ArrowUp));
    assert_eq!(up, KeyboardAction::Increment);

    let down = default_keyboard_action(&Role::Slider, &key_event(Key::ArrowDown));
    assert_eq!(down, KeyboardAction::Decrement);
}

// ---------------------------------------------------------------------------
// AC-9: image without label produces MissingLabel warning
// ---------------------------------------------------------------------------
#[test]
fn ac9_image_missing_label() {
    let tree = A11yNode {
        id: NodeId(0),
        info: AccessibilityInfo {
            role: Role::Group,
            ..Default::default()
        },
        children: vec![A11yNode {
            id: NodeId(1),
            info: AccessibilityInfo {
                role: Role::Image,
                label: None, // no label
                ..Default::default()
            },
            children: vec![],
        }],
    };

    let warnings = check_tree(&tree);
    assert_eq!(warnings.len(), 1);
    assert_eq!(
        warnings[0],
        A11yWarning::MissingLabel {
            node_id: NodeId(1),
            role: Role::Image,
        }
    );
}

// ---------------------------------------------------------------------------
// AC-10: White text on white background flags as below WCAG AA
// ---------------------------------------------------------------------------
#[test]
fn ac10_white_on_white_fails_contrast() {
    let white = SrgbColor::from_u8(255, 255, 255);
    let ratio = contrast_ratio(white, white);

    // Identical colors → ratio = 1.0
    assert!((ratio - 1.0).abs() < 0.001);
    assert!(!meets_wcag_aa(ratio, false)); // fails normal text
    assert!(!meets_wcag_aa(ratio, true)); // fails large text too

    // check_contrast should produce a warning
    let warning = check_contrast(NodeId(1), white, white, false);
    assert!(warning.is_some());
    match warning.unwrap() {
        A11yWarning::InsufficientContrast {
            node_id,
            ratio: r,
            required,
        } => {
            assert_eq!(node_id, NodeId(1));
            assert!((r - 1.0).abs() < 0.001);
            assert!((required - 4.5).abs() < 0.001);
        }
        other => panic!("Expected InsufficientContrast, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// AC-11: Checkbox checked: Some(true) → accesskit::Toggled::True
// ---------------------------------------------------------------------------
#[test]
fn ac11_checkbox_checked_maps_to_toggled() {
    let tree = A11yNode {
        id: NodeId(0),
        info: AccessibilityInfo {
            role: Role::Checkbox,
            label: Some("Accept".to_string()),
            state: AccessibilityState {
                checked: Some(CheckedState::Checked),
                focusable: true,
                ..Default::default()
            },
            ..Default::default()
        },
        children: vec![],
    };

    let update = generate_accesskit_tree(&tree, NodeId(0), true);
    let checkbox_node = &update.nodes.iter().find(|(id, _)| id.0 == 0).unwrap().1;
    assert_eq!(checkbox_node.toggled(), Some(accesskit::Toggled::True));
}

// ---------------------------------------------------------------------------
// AC-12: Role::None produces a presentational node (hidden from AT)
// ---------------------------------------------------------------------------
#[test]
fn ac12_role_none_is_presentational() {
    let tree = A11yNode {
        id: NodeId(0),
        info: AccessibilityInfo {
            role: Role::None,
            ..Default::default()
        },
        children: vec![],
    };

    let update = generate_accesskit_tree(&tree, NodeId(0), true);
    let node = &update.nodes.iter().find(|(id, _)| id.0 == 0).unwrap().1;

    // GenericContainer role + hidden flag
    assert_eq!(node.role(), accesskit::Role::GenericContainer);
    assert!(node.is_hidden());
}

// ---------------------------------------------------------------------------
// Additional tests for completeness
// ---------------------------------------------------------------------------

#[test]
fn focus_direct_and_blur() {
    let tree = node(
        0,
        Role::Group,
        None,
        vec![
            node(1, Role::Button, Some("A"), vec![]),
            node(2, Role::Button, Some("B"), vec![]),
        ],
    );

    let mut mgr = FocusManager::new(&tree);

    // Direct focus
    assert!(mgr.focus(NodeId(2)));
    assert_eq!(mgr.focused(), Some(NodeId(2)));

    // Focus non-focusable node fails
    assert!(!mgr.focus(NodeId(0))); // Group is not focusable
    // Focus stays on previous
    assert_eq!(mgr.focused(), Some(NodeId(2)));

    // Blur
    mgr.blur();
    assert_eq!(mgr.focused(), None);
}

#[test]
fn focus_previous_wraps_to_last() {
    let tree = node(
        0,
        Role::Group,
        None,
        vec![
            node(1, Role::Button, Some("A"), vec![]),
            node(2, Role::Button, Some("B"), vec![]),
        ],
    );

    let mut mgr = FocusManager::new(&tree);
    // focus_previous from nothing -> last
    let last = mgr.focus_previous();
    assert_eq!(last, Some(NodeId(2)));

    // focus_previous from first -> wraps to last
    mgr.focus(NodeId(1));
    let wrapped = mgr.focus_previous();
    assert_eq!(wrapped, Some(NodeId(2)));
}

#[test]
fn rebuild_preserves_focus() {
    let tree = node(
        0,
        Role::Group,
        None,
        vec![
            node(1, Role::Button, Some("A"), vec![]),
            node(2, Role::Button, Some("B"), vec![]),
        ],
    );

    let mut mgr = FocusManager::new(&tree);
    mgr.focus(NodeId(2));
    assert_eq!(mgr.focused(), Some(NodeId(2)));

    // Rebuild with same tree
    mgr.rebuild(&tree);
    assert_eq!(mgr.focused(), Some(NodeId(2)));
}

#[test]
fn rebuild_clears_focus_if_node_removed() {
    let tree1 = node(
        0,
        Role::Group,
        None,
        vec![
            node(1, Role::Button, Some("A"), vec![]),
            node(2, Role::Button, Some("B"), vec![]),
        ],
    );

    let mut mgr = FocusManager::new(&tree1);
    mgr.focus(NodeId(2));

    // Rebuild with tree missing node 2
    let tree2 = node(
        0,
        Role::Group,
        None,
        vec![node(1, Role::Button, Some("A"), vec![])],
    );
    mgr.rebuild(&tree2);
    assert_eq!(mgr.focused(), None);
}

#[test]
fn accesskit_tree_sets_label_and_description() {
    let tree = A11yNode {
        id: NodeId(0),
        info: AccessibilityInfo {
            role: Role::Button,
            label: Some("Submit".to_string()),
            description: Some("Submit the form".to_string()),
            state: AccessibilityState {
                focusable: true,
                ..Default::default()
            },
            ..Default::default()
        },
        children: vec![],
    };

    let update = generate_accesskit_tree(&tree, NodeId(0), true);
    let btn = &update.nodes[0].1;
    assert_eq!(btn.label(), Some("Submit"));
    assert_eq!(btn.description(), Some("Submit the form"));
}

#[test]
fn accesskit_tree_maps_slider_value_range() {
    let tree = A11yNode {
        id: NodeId(0),
        info: AccessibilityInfo {
            role: Role::Slider,
            label: Some("Volume".to_string()),
            state: AccessibilityState {
                focusable: true,
                value_min: Some(0.0),
                value_max: Some(100.0),
                value_now: Some(50.0),
                ..Default::default()
            },
            actions: vec![
                AccessibilityAction::Increment,
                AccessibilityAction::Decrement,
            ],
            ..Default::default()
        },
        children: vec![],
    };

    let update = generate_accesskit_tree(&tree, NodeId(0), true);
    let slider = &update.nodes[0].1;
    assert_eq!(slider.min_numeric_value(), Some(0.0));
    assert_eq!(slider.max_numeric_value(), Some(100.0));
    assert_eq!(slider.numeric_value(), Some(50.0));
}

#[test]
fn accesskit_tree_maps_expanded_state() {
    let tree = A11yNode {
        id: NodeId(0),
        info: AccessibilityInfo {
            role: Role::TreeItem,
            label: Some("Folder".to_string()),
            state: AccessibilityState {
                expanded: Some(ExpandedState::Collapsed),
                focusable: true,
                ..Default::default()
            },
            ..Default::default()
        },
        children: vec![],
    };

    let update = generate_accesskit_tree(&tree, NodeId(0), true);
    let item = &update.nodes[0].1;
    assert_eq!(item.is_expanded(), Some(false));
}

#[test]
fn accesskit_tree_maps_live_region() {
    let tree = A11yNode {
        id: NodeId(0),
        info: AccessibilityInfo {
            role: Role::Text,
            label: Some("Status".to_string()),
            live: LivePoliteness::Assertive,
            ..Default::default()
        },
        children: vec![],
    };

    let update = generate_accesskit_tree(&tree, NodeId(0), true);
    let node = &update.nodes[0].1;
    assert_eq!(node.live(), Some(accesskit::Live::Assertive));
}

#[test]
fn contrast_ratio_black_on_white() {
    let black = SrgbColor::from_u8(0, 0, 0);
    let white = SrgbColor::from_u8(255, 255, 255);
    let ratio = contrast_ratio(black, white);
    // Black on white should be ~21:1
    assert!(ratio > 20.0 && ratio < 22.0);
    assert!(meets_wcag_aa(ratio, false));
    assert!(meets_wcag_aa(ratio, true));
}

#[test]
fn contrast_good_pair_passes() {
    // Dark gray on white
    let fg = SrgbColor::from_u8(85, 85, 85);
    let bg = SrgbColor::from_u8(255, 255, 255);
    let ratio = contrast_ratio(fg, bg);
    assert!(meets_wcag_aa(ratio, false)); // should pass 4.5:1
}

#[test]
fn missing_label_detected_for_interactive_elements() {
    let tree = node(
        0,
        Role::Group,
        None,
        vec![
            // Button without label → warning
            A11yNode {
                id: NodeId(1),
                info: AccessibilityInfo {
                    role: Role::Button,
                    label: None,
                    ..Default::default()
                },
                children: vec![],
            },
            // Button with label → no warning
            node(2, Role::Button, Some("OK"), vec![]),
            // Text without label → no warning (text doesn't require label)
            A11yNode {
                id: NodeId(3),
                info: AccessibilityInfo {
                    role: Role::Text,
                    label: None,
                    ..Default::default()
                },
                children: vec![],
            },
        ],
    );

    let warnings = check_tree(&tree);
    assert_eq!(warnings.len(), 1);
    assert_eq!(
        warnings[0],
        A11yWarning::MissingLabel {
            node_id: NodeId(1),
            role: Role::Button,
        }
    );
}

#[test]
fn keyboard_modifiers_suppress_default_actions() {
    let mut event = key_event(Key::Enter);
    event.modifiers.ctrl = true;
    let action = default_keyboard_action(&Role::Button, &event);
    assert_eq!(action, KeyboardAction::None);
}

#[test]
fn switch_responds_to_space() {
    let action = default_keyboard_action(&Role::Switch, &key_event(Key::Space));
    assert_eq!(action, KeyboardAction::Toggle);
}

#[test]
fn accesskit_subsequent_update_has_no_tree() {
    let tree = A11yNode {
        id: NodeId(0),
        info: AccessibilityInfo {
            role: Role::Window,
            label: Some("App".to_string()),
            ..Default::default()
        },
        children: vec![],
    };

    let update = generate_accesskit_tree(&tree, NodeId(0), false);
    assert!(update.tree.is_none());
}

#[test]
fn checkbox_mixed_maps_to_toggled_mixed() {
    let tree = A11yNode {
        id: NodeId(0),
        info: AccessibilityInfo {
            role: Role::Checkbox,
            label: Some("Select all".to_string()),
            state: AccessibilityState {
                checked: Some(CheckedState::Mixed),
                focusable: true,
                ..Default::default()
            },
            ..Default::default()
        },
        children: vec![],
    };

    let update = generate_accesskit_tree(&tree, NodeId(0), true);
    let node = &update.nodes[0].1;
    assert_eq!(node.toggled(), Some(accesskit::Toggled::Mixed));
}

#[test]
fn empty_tree_focus_manager() {
    let tree = A11yNode {
        id: NodeId(0),
        info: AccessibilityInfo::default(),
        children: vec![],
    };

    let mut mgr = FocusManager::new(&tree);
    assert!(mgr.focus_order().is_empty());
    assert_eq!(mgr.focused(), None);
    assert_eq!(mgr.focus_next(), None);
    assert_eq!(mgr.focus_previous(), None);
}
