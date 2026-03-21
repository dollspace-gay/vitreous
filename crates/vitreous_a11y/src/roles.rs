/// Semantic role of a widget in the accessibility tree.
///
/// Maps to `accesskit::Role` for platform accessibility APIs.
/// `Role::None` marks a node as presentational (excluded from assistive technology).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Role {
    Button,
    Checkbox,
    Dialog,
    Grid,
    GridCell,
    Heading,
    Image,
    Link,
    List,
    ListItem,
    Menu,
    MenuItem,
    ProgressBar,
    RadioButton,
    ScrollView,
    Slider,
    Switch,
    Tab,
    TabList,
    TabPanel,
    TextInput,
    Text,
    Toolbar,
    Tooltip,
    Tree,
    TreeItem,
    Window,
    Group,
    /// Presentational — excluded from the accessibility tree.
    #[default]
    None,
}

impl Role {
    /// Returns `true` if this role is focusable by default.
    ///
    /// Interactive widgets (buttons, inputs, etc.) are focusable.
    /// Static content (text, images, containers) is not.
    pub fn is_default_focusable(&self) -> bool {
        matches!(
            self,
            Role::Button
                | Role::TextInput
                | Role::Checkbox
                | Role::Switch
                | Role::Slider
                | Role::Tab
                | Role::Link
                | Role::RadioButton
                | Role::MenuItem
        )
    }
}

/// Tri-state checked value for checkboxes and similar controls.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CheckedState {
    Unchecked,
    Checked,
    Mixed,
}

/// Tri-state expanded value for collapsible sections, tree items, etc.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ExpandedState {
    Collapsed,
    Expanded,
}

/// Live region politeness level for dynamic content announcements.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum LivePoliteness {
    /// No live announcements.
    #[default]
    Off,
    /// Announced when the user is idle.
    Polite,
    /// Announced immediately, interrupting current speech.
    Assertive,
}

/// An action that can be performed on an accessible node.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AccessibilityAction {
    Click,
    Focus,
    Blur,
    Increment,
    Decrement,
    ScrollUp,
    ScrollDown,
    ScrollLeft,
    ScrollRight,
    Expand,
    Collapse,
    SetValue,
}

/// Accessibility state properties for a node.
///
/// All fields default to their "not set" state (`false`, `None`, etc.).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AccessibilityState {
    pub disabled: bool,
    pub selected: bool,
    pub checked: Option<CheckedState>,
    pub expanded: Option<ExpandedState>,
    pub has_popup: bool,
    pub focusable: bool,
    pub focused: bool,
    pub read_only: bool,
    pub required: bool,
    pub invalid: bool,
    pub busy: bool,
    pub modal: bool,
    pub level: Option<u32>,
    pub value_min: Option<f64>,
    pub value_max: Option<f64>,
    pub value_now: Option<f64>,
}

/// Complete accessibility metadata for a single node.
///
/// Attached to every widget node. Fields are optional — unset fields
/// produce no corresponding AccessKit properties.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AccessibilityInfo {
    pub role: Role,
    pub label: Option<String>,
    pub description: Option<String>,
    pub value: Option<String>,
    pub live: LivePoliteness,
    pub state: AccessibilityState,
    pub actions: Vec<AccessibilityAction>,
}
