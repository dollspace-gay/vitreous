use vitreous_events::NodeId;

use crate::roles::Role;
use crate::tree::A11yNode;

/// An accessibility warning detected during tree validation.
///
/// Warnings are collected during tree traversal in debug builds only.
/// They carry the offending node's ID and a description of the issue.
#[derive(Clone, Debug, PartialEq)]
pub enum A11yWarning {
    /// An interactive element or image has no accessible label.
    MissingLabel { node_id: NodeId, role: Role },
    /// Foreground/background color contrast is below WCAG AA threshold.
    InsufficientContrast {
        node_id: NodeId,
        ratio: f64,
        required: f64,
    },
    /// A focusable element has no way out (all focus_next paths lead back to it).
    FocusTrap { node_id: NodeId },
}

/// An sRGB color used for contrast checking.
///
/// Components are in the range `0.0..=1.0`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SrgbColor {
    pub r: f64,
    pub g: f64,
    pub b: f64,
}

impl SrgbColor {
    /// Create a color from 8-bit components.
    pub fn from_u8(r: u8, g: u8, b: u8) -> Self {
        Self {
            r: r as f64 / 255.0,
            g: g as f64 / 255.0,
            b: b as f64 / 255.0,
        }
    }

    /// Compute relative luminance per WCAG 2.x.
    ///
    /// Linearizes sRGB components then applies the luminance formula:
    /// `L = 0.2126*R + 0.7152*G + 0.0722*B`
    pub fn relative_luminance(&self) -> f64 {
        fn linearize(c: f64) -> f64 {
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        }
        let r = linearize(self.r);
        let g = linearize(self.g);
        let b = linearize(self.b);
        0.2126 * r + 0.7152 * g + 0.0722 * b
    }
}

/// Compute the WCAG contrast ratio between two colors.
///
/// Returns a value >= 1.0. A ratio of 1:1 means identical colors.
/// WCAG AA requires 4.5:1 for normal text, 3:1 for large text.
pub fn contrast_ratio(fg: SrgbColor, bg: SrgbColor) -> f64 {
    let l1 = fg.relative_luminance();
    let l2 = bg.relative_luminance();
    let (lighter, darker) = if l1 > l2 { (l1, l2) } else { (l2, l1) };
    (lighter + 0.05) / (darker + 0.05)
}

/// WCAG AA minimum contrast ratio for normal text (< 18pt).
pub const WCAG_AA_NORMAL_TEXT: f64 = 4.5;

/// WCAG AA minimum contrast ratio for large text (>= 18pt or >= 14pt bold).
pub const WCAG_AA_LARGE_TEXT: f64 = 3.0;

/// Check whether a contrast ratio meets WCAG AA for a given text size.
pub fn meets_wcag_aa(ratio: f64, is_large_text: bool) -> bool {
    let threshold = if is_large_text {
        WCAG_AA_LARGE_TEXT
    } else {
        WCAG_AA_NORMAL_TEXT
    };
    ratio >= threshold
}

/// Check a single node for missing labels.
///
/// Returns `Some(warning)` if the node requires a label but doesn't have one.
fn check_missing_label(node: &A11yNode) -> Option<A11yWarning> {
    let needs_label = matches!(
        node.info.role,
        Role::Image
            | Role::Button
            | Role::Checkbox
            | Role::Switch
            | Role::Slider
            | Role::TextInput
            | Role::Link
            | Role::RadioButton
            | Role::MenuItem
            | Role::Tab
    );

    if needs_label && node.info.label.is_none() {
        Some(A11yWarning::MissingLabel {
            node_id: node.id,
            role: node.info.role,
        })
    } else {
        Option::None
    }
}

/// Collect accessibility warnings from an entire tree.
///
/// Walks the tree depth-first and checks each node for:
/// - Missing labels on interactive elements and images
///
/// Contrast and focus trap checks require additional context (colors, focus order)
/// and are available as standalone functions.
pub fn check_tree(root: &A11yNode) -> Vec<A11yWarning> {
    let mut warnings = Vec::new();
    check_tree_recursive(root, &mut warnings);
    warnings
}

fn check_tree_recursive(node: &A11yNode, warnings: &mut Vec<A11yWarning>) {
    if let Some(warning) = check_missing_label(node) {
        warnings.push(warning);
    }
    for child in &node.children {
        check_tree_recursive(child, warnings);
    }
}

/// Check a specific foreground/background pair for WCAG AA compliance.
///
/// Returns `Some(warning)` if contrast is insufficient.
pub fn check_contrast(
    node_id: NodeId,
    fg: SrgbColor,
    bg: SrgbColor,
    is_large_text: bool,
) -> Option<A11yWarning> {
    let ratio = contrast_ratio(fg, bg);
    let required = if is_large_text {
        WCAG_AA_LARGE_TEXT
    } else {
        WCAG_AA_NORMAL_TEXT
    };
    if !meets_wcag_aa(ratio, is_large_text) {
        Some(A11yWarning::InsufficientContrast {
            node_id,
            ratio,
            required,
        })
    } else {
        Option::None
    }
}
