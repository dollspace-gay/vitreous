use vitreous_events::{Key, KeyEvent, Modifiers};

use crate::roles::Role;

/// An action produced by a default keyboard interaction.
///
/// The keyboard module maps `(Role, Key, Modifiers)` → `KeyboardAction`.
/// The caller (event loop or widgets layer) is responsible for executing
/// the action on the target node.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum KeyboardAction {
    /// Trigger a click (Button: Enter/Space).
    Click,
    /// Toggle checked state (Checkbox/Switch: Space).
    Toggle,
    /// Increment a numeric value (Slider: Right/Up arrow).
    Increment,
    /// Decrement a numeric value (Slider: Left/Down arrow).
    Decrement,
    /// Select / activate the current item (Select/Menu: Enter/Space).
    Activate,
    /// Dismiss / cancel (Dialog/Menu: Escape).
    Dismiss,
    /// Move to next item in a group (Menu/TabList: Down/Right arrow).
    NextItem,
    /// Move to previous item in a group (Menu/TabList: Up/Left arrow).
    PreviousItem,
    /// No default keyboard action for this role + key combination.
    None,
}

/// Determine the default keyboard action for a role and key event.
///
/// This is a pure mapping — it does not execute any action or modify state.
/// Tab/Shift+Tab focus cycling is handled by `FocusManager`, not here.
pub fn default_keyboard_action(role: &Role, event: &KeyEvent) -> KeyboardAction {
    // Ignore events with Ctrl/Alt/Meta held — those are shortcuts, not widget interactions
    if event.modifiers.ctrl || event.modifiers.alt || event.modifiers.meta {
        return KeyboardAction::None;
    }

    match role {
        Role::Button | Role::Link => match event.key {
            Key::Enter | Key::Space => KeyboardAction::Click,
            _ => KeyboardAction::None,
        },

        Role::Checkbox | Role::Switch => match event.key {
            Key::Space => KeyboardAction::Toggle,
            _ => KeyboardAction::None,
        },

        Role::RadioButton => match event.key {
            Key::Space | Key::Enter => KeyboardAction::Activate,
            Key::ArrowDown | Key::ArrowRight => KeyboardAction::NextItem,
            Key::ArrowUp | Key::ArrowLeft => KeyboardAction::PreviousItem,
            _ => KeyboardAction::None,
        },

        Role::Slider => match event.key {
            Key::ArrowRight | Key::ArrowUp => KeyboardAction::Increment,
            Key::ArrowLeft | Key::ArrowDown => KeyboardAction::Decrement,
            _ => KeyboardAction::None,
        },

        Role::Tab => match event.key {
            Key::Enter | Key::Space => KeyboardAction::Activate,
            Key::ArrowRight | Key::ArrowDown => KeyboardAction::NextItem,
            Key::ArrowLeft | Key::ArrowUp => KeyboardAction::PreviousItem,
            _ => KeyboardAction::None,
        },

        Role::MenuItem => match event.key {
            Key::Enter | Key::Space => KeyboardAction::Activate,
            Key::ArrowDown => KeyboardAction::NextItem,
            Key::ArrowUp => KeyboardAction::PreviousItem,
            Key::Escape => KeyboardAction::Dismiss,
            _ => KeyboardAction::None,
        },

        Role::Menu | Role::TabList => match event.key {
            Key::ArrowDown | Key::ArrowRight => KeyboardAction::NextItem,
            Key::ArrowUp | Key::ArrowLeft => KeyboardAction::PreviousItem,
            Key::Escape => KeyboardAction::Dismiss,
            _ => KeyboardAction::None,
        },

        Role::Dialog => match event.key {
            Key::Escape => KeyboardAction::Dismiss,
            _ => KeyboardAction::None,
        },

        Role::TreeItem => match event.key {
            Key::Enter | Key::Space => KeyboardAction::Activate,
            Key::ArrowDown => KeyboardAction::NextItem,
            Key::ArrowUp => KeyboardAction::PreviousItem,
            Key::ArrowRight => KeyboardAction::Increment, // expand
            Key::ArrowLeft => KeyboardAction::Decrement,  // collapse
            _ => KeyboardAction::None,
        },

        _ => KeyboardAction::None,
    }
}

/// Helper to create a `KeyEvent` for testing keyboard interactions.
pub fn key_event(key: Key) -> KeyEvent {
    KeyEvent {
        key,
        code: vitreous_events::KeyCode::Unidentified,
        modifiers: Modifiers::default(),
        repeat: false,
        text: Option::None,
    }
}
