pub mod focus;
pub mod keyboard;
pub mod roles;
pub mod tree;
pub mod warnings;

pub use focus::FocusManager;
pub use keyboard::{KeyboardAction, default_keyboard_action, key_event};
pub use roles::{
    AccessibilityAction, AccessibilityInfo, AccessibilityState, CheckedState, ExpandedState,
    LivePoliteness, Role,
};
pub use tree::{A11yNode, generate_accesskit_tree};
pub use warnings::{
    A11yWarning, SrgbColor, WCAG_AA_LARGE_TEXT, WCAG_AA_NORMAL_TEXT, check_contrast, check_tree,
    contrast_ratio, meets_wcag_aa,
};
