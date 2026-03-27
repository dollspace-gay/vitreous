use vitreous_a11y::{AccessibilityInfo, CheckedState, ExpandedState, LivePoliteness, Role};
use web_sys::Element;

/// Map a vitreous `Role` to the corresponding WAI-ARIA role string.
fn role_to_aria(role: &Role) -> Option<&'static str> {
    match role {
        Role::Button => Some("button"),
        Role::Checkbox => Some("checkbox"),
        Role::Dialog => Some("dialog"),
        Role::Grid => Some("grid"),
        Role::GridCell => Some("gridcell"),
        Role::Heading => Some("heading"),
        Role::Image => Some("img"),
        Role::Link => Some("link"),
        Role::List => Some("list"),
        Role::ListItem => Some("listitem"),
        Role::Menu => Some("menu"),
        Role::MenuItem => Some("menuitem"),
        Role::ProgressBar => Some("progressbar"),
        Role::RadioButton => Some("radio"),
        Role::ScrollView => Some("scrollbar"),
        Role::Slider => Some("slider"),
        Role::Switch => Some("switch"),
        Role::Tab => Some("tab"),
        Role::TabList => Some("tablist"),
        Role::TabPanel => Some("tabpanel"),
        Role::TextInput => Some("textbox"),
        Role::Text => None, // native text elements don't need explicit role
        Role::Toolbar => Some("toolbar"),
        Role::Tooltip => Some("tooltip"),
        Role::Tree => Some("tree"),
        Role::TreeItem => Some("treeitem"),
        Role::Window => None, // browser window already has this semantics
        Role::Group => Some("group"),
        Role::None => Some("presentation"),
    }
}

/// Apply all ARIA attributes from `AccessibilityInfo` to a DOM `Element`.
pub fn apply_aria(element: &Element, info: &AccessibilityInfo) {
    // Role
    if let Some(role_str) = role_to_aria(&info.role) {
        let _ = element.set_attribute("role", role_str);
    }

    // Label
    if let Some(label) = &info.label {
        let _ = element.set_attribute("aria-label", label);
    }

    // Description
    if let Some(desc) = &info.description {
        let _ = element.set_attribute("aria-description", desc);
    }

    // Value (text value for inputs/sliders)
    if let Some(value) = &info.value {
        let _ = element.set_attribute("aria-valuetext", value);
    }

    // Live region
    match info.live {
        LivePoliteness::Off => {}
        LivePoliteness::Polite => {
            let _ = element.set_attribute("aria-live", "polite");
        }
        LivePoliteness::Assertive => {
            let _ = element.set_attribute("aria-live", "assertive");
        }
    }

    // State: disabled
    if info.state.disabled {
        let _ = element.set_attribute("aria-disabled", "true");
    }

    // State: selected
    if info.state.selected {
        let _ = element.set_attribute("aria-selected", "true");
    }

    // State: checked
    if let Some(checked) = &info.state.checked {
        let val = match checked {
            CheckedState::Unchecked => "false",
            CheckedState::Checked => "true",
            CheckedState::Mixed => "mixed",
        };
        let _ = element.set_attribute("aria-checked", val);
    }

    // State: expanded
    if let Some(expanded) = &info.state.expanded {
        let val = match expanded {
            ExpandedState::Collapsed => "false",
            ExpandedState::Expanded => "true",
        };
        let _ = element.set_attribute("aria-expanded", val);
    }

    // State: has_popup
    if info.state.has_popup {
        let _ = element.set_attribute("aria-haspopup", "true");
    }

    // State: focusable — set tabindex
    if info.state.focusable {
        let _ = element.set_attribute("tabindex", "0");
    }

    // State: read_only
    if info.state.read_only {
        let _ = element.set_attribute("aria-readonly", "true");
    }

    // State: required
    if info.state.required {
        let _ = element.set_attribute("aria-required", "true");
    }

    // State: invalid
    if info.state.invalid {
        let _ = element.set_attribute("aria-invalid", "true");
    }

    // State: busy
    if info.state.busy {
        let _ = element.set_attribute("aria-busy", "true");
    }

    // State: modal
    if info.state.modal {
        let _ = element.set_attribute("aria-modal", "true");
    }

    // State: heading level
    if let Some(level) = info.state.level {
        let _ = element.set_attribute("aria-level", &level.to_string());
    }

    // State: value range (slider/progressbar)
    if let Some(min) = info.state.value_min {
        let _ = element.set_attribute("aria-valuemin", &min.to_string());
    }
    if let Some(max) = info.state.value_max {
        let _ = element.set_attribute("aria-valuemax", &max.to_string());
    }
    if let Some(now) = info.state.value_now {
        let _ = element.set_attribute("aria-valuenow", &now.to_string());
    }
}

/// Remove all ARIA attributes from an element to prevent stale attributes
/// from accumulating during reconciliation.
pub fn clear_aria_attributes(element: &Element) {
    // Known ARIA attribute names that apply_aria may set
    static ARIA_ATTRS: &[&str] = &[
        "role", "tabindex", "aria-label", "aria-description",
        "aria-live", "aria-disabled", "aria-checked", "aria-selected",
        "aria-expanded", "aria-haspopup", "aria-readonly", "aria-required",
        "aria-invalid", "aria-busy", "aria-modal", "aria-level",
        "aria-valuemin", "aria-valuemax", "aria-valuenow",
    ];
    for attr in ARIA_ATTRS {
        let _ = element.remove_attribute(attr);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_button() {
        assert_eq!(role_to_aria(&Role::Button), Some("button"));
    }

    #[test]
    fn role_checkbox() {
        assert_eq!(role_to_aria(&Role::Checkbox), Some("checkbox"));
    }

    #[test]
    fn role_text_is_none() {
        assert_eq!(role_to_aria(&Role::Text), None);
    }

    #[test]
    fn role_none_is_presentation() {
        assert_eq!(role_to_aria(&Role::None), Some("presentation"));
    }

    #[test]
    fn role_slider() {
        assert_eq!(role_to_aria(&Role::Slider), Some("slider"));
    }

    #[test]
    fn role_textbox() {
        assert_eq!(role_to_aria(&Role::TextInput), Some("textbox"));
    }
}
