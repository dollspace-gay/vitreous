use vitreous_a11y::{AccessibilityAction, CheckedState, Role};
use vitreous_reactive::Signal;
use vitreous_style::{Color, Dimension};

use crate::node::{ImageSource, IntoTextContent, Node, NodeKind, TextContent};

/// A text widget. Accepts both static strings and reactive closures via
/// `IntoTextContent`.
///
/// Default role: `Text`.
pub fn text(content: impl IntoTextContent) -> Node {
    let tc = content.into_text_content();
    let label = match &tc {
        TextContent::Static(s) => Some(s.clone()),
        TextContent::Dynamic(_) => None,
    };
    let mut node = Node::new(NodeKind::Text(tc));
    node.a11y.role = Role::Text;
    node.a11y.label = label;
    node
}

/// A clickable button with a text label.
///
/// Default role: `Button`, focusable, with `Click` action.
pub fn button(label: impl Into<String>) -> Node {
    let label_str = label.into();
    let mut node = Node::new(NodeKind::Text(TextContent::Static(label_str.clone())));
    node.a11y.role = Role::Button;
    node.a11y.label = Some(label_str);
    node.a11y.state.focusable = true;
    node.a11y.actions.push(AccessibilityAction::Click);
    node.style.cursor = Some(vitreous_style::CursorIcon::Pointer);
    node
}

/// A single-line text input field.
///
/// Default role: `TextInput`, focusable.
pub fn text_input(value: Signal<String>, on_change: impl Fn(String) + 'static) -> Node {
    let current = value.get();
    let mut node = Node::new(NodeKind::Text(TextContent::Dynamic(Box::new(move || {
        value.get()
    }))));
    node.a11y.role = Role::TextInput;
    node.a11y.label = Some(current);
    node.a11y.state.focusable = true;
    node.a11y.actions.push(AccessibilityAction::SetValue);
    node.style.cursor = Some(vitreous_style::CursorIcon::Text);

    // Wire on_change into key event handling: each key press appends to the
    // signal value and calls on_change with the updated text. Backspace removes
    // the last character.
    let value_for_handler = value;
    node.event_handlers.on_key_down = Some(Box::new(move |ev: vitreous_events::KeyEvent| {
        let mut current = value_for_handler.get();
        if ev.key == vitreous_events::Key::Backspace {
            current.pop();
        } else if let Some(ref ch) = ev.text
            && !ch.is_empty() && !ev.modifiers.ctrl && !ev.modifiers.meta
        {
            current.push_str(ch);
        }
        value_for_handler.set(current.clone());
        on_change(current);
    }));
    node
}

/// A checkbox with a boolean signal for checked state.
///
/// Default role: `Checkbox`, focusable, with `Toggle` action.
pub fn checkbox(checked: Signal<bool>) -> Node {
    let is_checked = checked.get();
    let mut node = Node::new(NodeKind::Container);
    node.a11y.role = Role::Checkbox;
    node.a11y.state.focusable = true;
    node.a11y.state.checked = Some(if is_checked {
        CheckedState::Checked
    } else {
        CheckedState::Unchecked
    });
    node.a11y.actions.push(AccessibilityAction::Click);
    node.style.cursor = Some(vitreous_style::CursorIcon::Pointer);
    node
}

/// A toggle switch (same semantics as checkbox, different visual).
///
/// Default role: `Switch`, focusable, with `Toggle` action.
pub fn toggle(on: Signal<bool>) -> Node {
    let is_on = on.get();
    let mut node = Node::new(NodeKind::Container);
    node.a11y.role = Role::Switch;
    node.a11y.state.focusable = true;
    node.a11y.state.checked = Some(if is_on {
        CheckedState::Checked
    } else {
        CheckedState::Unchecked
    });
    node.a11y.actions.push(AccessibilityAction::Click);
    node.style.cursor = Some(vitreous_style::CursorIcon::Pointer);
    node
}

/// A slider for selecting a numeric value within a range.
///
/// Default role: `Slider`, focusable, with `Increment`/`Decrement` actions.
pub fn slider(value: Signal<f64>, min: f64, max: f64) -> Node {
    let current = value.get();
    let mut node = Node::new(NodeKind::Container);
    node.a11y.role = Role::Slider;
    node.a11y.state.focusable = true;
    node.a11y.state.value_min = Some(min);
    node.a11y.state.value_max = Some(max);
    node.a11y.state.value_now = Some(current);
    node.a11y.actions.push(AccessibilityAction::Increment);
    node.a11y.actions.push(AccessibilityAction::Decrement);
    node.style.cursor = Some(vitreous_style::CursorIcon::Pointer);
    node
}

/// A dropdown selection widget.
///
/// Default role: `Menu` (acts as a select/combobox), focusable.
pub fn select(options: Vec<String>, selected: Signal<usize>) -> Node {
    let current_idx = selected.get();
    let label = options.get(current_idx).cloned().unwrap_or_default();
    let mut node = Node::new(NodeKind::Container);
    node.a11y.role = Role::Menu;
    node.a11y.label = Some(label);
    node.a11y.state.focusable = true;
    node.a11y.state.has_popup = true;
    node.a11y.actions.push(AccessibilityAction::Expand);
    node.a11y.actions.push(AccessibilityAction::Click);
    node.style.cursor = Some(vitreous_style::CursorIcon::Pointer);

    // Store option children as menu items
    for (i, opt) in options.into_iter().enumerate() {
        let mut item = Node::new(NodeKind::Text(TextContent::Static(opt.clone())));
        item.a11y.role = Role::MenuItem;
        item.a11y.label = Some(opt);
        item.a11y.state.selected = i == current_idx;
        node.children.push(item);
    }
    node
}

/// An image widget.
///
/// Default role: `Image`.
pub fn image(source: impl Into<ImageSource>) -> Node {
    let mut node = Node::new(NodeKind::Image(source.into()));
    node.a11y.role = Role::Image;
    node
}

/// An invisible spacer that takes up flex space.
pub fn spacer() -> Node {
    let mut node = Node::new(NodeKind::Container);
    node.flex_grow = 1.0;
    node.a11y.role = Role::None;
    node
}

/// A visual divider line.
pub fn divider() -> Node {
    let mut node = Node::new(NodeKind::Container);
    node.a11y.role = Role::None;
    node.style.background = Some(Color::LIGHT_GRAY);
    node.style.height = Dimension::Px(1.0);
    node.style.width = Dimension::Px(f32::INFINITY); // stretch to fill
    node
}

#[cfg(test)]
mod tests {
    use super::*;
    use vitreous_reactive::create_signal;

    #[test]
    fn text_static_sets_role_and_label() {
        let node = text("hello");
        assert_eq!(node.a11y.role, Role::Text);
        assert_eq!(node.a11y.label, Some("hello".to_owned()));
    }

    #[test]
    fn text_dynamic_compiles() {
        let sig = create_signal("world".to_owned());
        let node = text(move || format!("hello {}", sig.get()));
        assert_eq!(node.a11y.role, Role::Text);
        // Dynamic text has no static label
        assert_eq!(node.a11y.label, None);
    }

    #[test]
    fn button_sets_role_label_focusable() {
        let node = button("Submit");
        assert_eq!(node.a11y.role, Role::Button);
        assert_eq!(node.a11y.label, Some("Submit".to_owned()));
        assert!(node.a11y.state.focusable);
        assert!(node.a11y.actions.contains(&AccessibilityAction::Click));
    }

    #[test]
    fn checkbox_sets_role_and_checked_state() {
        let sig = create_signal(true);
        let node = checkbox(sig);
        assert_eq!(node.a11y.role, Role::Checkbox);
        assert_eq!(node.a11y.state.checked, Some(CheckedState::Checked));
        assert!(node.a11y.state.focusable);
    }

    #[test]
    fn toggle_sets_role() {
        let sig = create_signal(false);
        let node = toggle(sig);
        assert_eq!(node.a11y.role, Role::Switch);
        assert_eq!(node.a11y.state.checked, Some(CheckedState::Unchecked));
    }

    #[test]
    fn slider_sets_value_range() {
        let sig = create_signal(50.0);
        let node = slider(sig, 0.0, 100.0);
        assert_eq!(node.a11y.role, Role::Slider);
        assert_eq!(node.a11y.state.value_min, Some(0.0));
        assert_eq!(node.a11y.state.value_max, Some(100.0));
        assert_eq!(node.a11y.state.value_now, Some(50.0));
    }

    #[test]
    fn image_sets_role() {
        let node = image("test.png");
        assert_eq!(node.a11y.role, Role::Image);
    }

    #[test]
    fn spacer_has_flex_grow() {
        let node = spacer();
        assert_eq!(node.flex_grow, 1.0);
        assert_eq!(node.a11y.role, Role::None);
    }

    #[test]
    fn divider_has_background() {
        let node = divider();
        assert!(node.style.background.is_some());
        assert_eq!(node.a11y.role, Role::None);
    }

    #[test]
    fn select_builds_menu_items() {
        let sig = create_signal(1usize);
        let node = select(vec!["Red".into(), "Green".into(), "Blue".into()], sig);
        assert_eq!(node.a11y.role, Role::Menu);
        assert_eq!(node.a11y.label, Some("Green".to_owned()));
        assert_eq!(node.children.len(), 3);
        assert_eq!(node.children[0].a11y.role, Role::MenuItem);
        assert!(!node.children[0].a11y.state.selected);
        assert!(node.children[1].a11y.state.selected);
    }
}
