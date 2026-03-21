use taffy;

// ---------------------------------------------------------------------------
// NodeId — our own lightweight node identifier
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub u32);

impl NodeId {
    pub const ROOT: Self = Self(0);
}

// ---------------------------------------------------------------------------
// Dimension (Px / Percent / Auto) — matches vitreous_style's Dimension
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Dimension {
    Px(f32),
    Percent(f32),
    #[default]
    Auto,
}

impl From<f32> for Dimension {
    fn from(px: f32) -> Self {
        Self::Px(px)
    }
}

impl From<i32> for Dimension {
    fn from(px: i32) -> Self {
        Self::Px(px as f32)
    }
}

// ---------------------------------------------------------------------------
// Size — simple width/height pair
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

impl Size {
    pub const ZERO: Self = Self {
        width: 0.0,
        height: 0.0,
    };

    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
}

// ---------------------------------------------------------------------------
// Rect — edges (top, right, bottom, left)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl Rect {
    pub const ZERO: Self = Self {
        top: 0.0,
        right: 0.0,
        bottom: 0.0,
        left: 0.0,
    };

    pub const fn new(top: f32, right: f32, bottom: f32, left: f32) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    pub const fn all(v: f32) -> Self {
        Self {
            top: v,
            right: v,
            bottom: v,
            left: v,
        }
    }

    pub const fn axes(vertical: f32, horizontal: f32) -> Self {
        Self {
            top: vertical,
            right: horizontal,
            bottom: vertical,
            left: horizontal,
        }
    }
}

// ---------------------------------------------------------------------------
// DimensionRect — edges in Dimension units (for padding, margin, inset)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DimensionRect {
    pub top: Dimension,
    pub right: Dimension,
    pub bottom: Dimension,
    pub left: Dimension,
}

impl DimensionRect {
    pub const ZERO: Self = Self {
        top: Dimension::Px(0.0),
        right: Dimension::Px(0.0),
        bottom: Dimension::Px(0.0),
        left: Dimension::Px(0.0),
    };

    pub const AUTO: Self = Self {
        top: Dimension::Auto,
        right: Dimension::Auto,
        bottom: Dimension::Auto,
        left: Dimension::Auto,
    };

    pub const fn all(d: Dimension) -> Self {
        Self {
            top: d,
            right: d,
            bottom: d,
            left: d,
        }
    }

    pub const fn axes(vertical: Dimension, horizontal: Dimension) -> Self {
        Self {
            top: vertical,
            right: horizontal,
            bottom: vertical,
            left: horizontal,
        }
    }
}

impl Default for DimensionRect {
    fn default() -> Self {
        Self::ZERO
    }
}

// ---------------------------------------------------------------------------
// Flexbox enums
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Display {
    #[default]
    Flex,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FlexDirection {
    Row,
    #[default]
    Column,
    RowReverse,
    ColumnReverse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FlexWrap {
    #[default]
    NoWrap,
    Wrap,
    WrapReverse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JustifyContent {
    Start,
    End,
    FlexStart,
    FlexEnd,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlignItems {
    Start,
    End,
    FlexStart,
    FlexEnd,
    Center,
    Baseline,
    Stretch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlignSelf {
    Start,
    End,
    FlexStart,
    FlexEnd,
    Center,
    Baseline,
    Stretch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlignContent {
    Start,
    End,
    FlexStart,
    FlexEnd,
    Center,
    Stretch,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Position {
    #[default]
    Relative,
    Absolute,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Overflow {
    #[default]
    Visible,
    Hidden,
    Scroll,
}

// ---------------------------------------------------------------------------
// LayoutStyle — full set of layout properties for a node
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct LayoutStyle {
    pub display: Display,
    pub flex_direction: FlexDirection,
    pub flex_wrap: FlexWrap,
    pub justify_content: Option<JustifyContent>,
    pub align_items: Option<AlignItems>,
    pub align_self: Option<AlignSelf>,
    pub align_content: Option<AlignContent>,
    pub flex_grow: f32,
    pub flex_shrink: f32,
    pub flex_basis: Dimension,
    pub width: Dimension,
    pub height: Dimension,
    pub min_width: Dimension,
    pub max_width: Dimension,
    pub min_height: Dimension,
    pub max_height: Dimension,
    pub padding: DimensionRect,
    pub margin: DimensionRect,
    pub gap: Size,
    pub aspect_ratio: Option<f32>,
    pub overflow: Overflow,
    pub position: Position,
    pub inset: DimensionRect,
}

impl Default for LayoutStyle {
    fn default() -> Self {
        Self {
            display: Display::default(),
            flex_direction: FlexDirection::default(),
            flex_wrap: FlexWrap::default(),
            justify_content: None,
            align_items: None,
            align_self: None,
            align_content: None,
            flex_grow: 0.0,
            flex_shrink: 1.0,
            flex_basis: Dimension::Auto,
            width: Dimension::Auto,
            height: Dimension::Auto,
            min_width: Dimension::Auto,
            max_width: Dimension::Auto,
            min_height: Dimension::Auto,
            max_height: Dimension::Auto,
            padding: DimensionRect::ZERO,
            margin: DimensionRect::ZERO,
            gap: Size::ZERO,
            aspect_ratio: None,
            overflow: Overflow::default(),
            position: Position::default(),
            inset: DimensionRect::AUTO,
        }
    }
}

// ---------------------------------------------------------------------------
// MeasureFn — callback for leaf nodes (text, images) to report intrinsic size
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MeasureConstraint {
    pub max_width: Option<f32>,
    pub max_height: Option<f32>,
}

pub type MeasureFn = Box<dyn Fn(MeasureConstraint) -> Size>;

// ---------------------------------------------------------------------------
// LayoutInput — a node in the input tree
// ---------------------------------------------------------------------------

pub struct LayoutInput {
    pub id: NodeId,
    pub style: LayoutStyle,
    pub children: Vec<NodeId>,
    pub measure: Option<MeasureFn>,
}

impl LayoutInput {
    pub fn new(id: NodeId, style: LayoutStyle) -> Self {
        Self {
            id,
            style,
            children: Vec::new(),
            measure: None,
        }
    }

    pub fn with_children(mut self, children: Vec<NodeId>) -> Self {
        self.children = children;
        self
    }

    pub fn with_measure(mut self, measure: MeasureFn) -> Self {
        self.measure = Some(measure);
        self
    }
}

// ---------------------------------------------------------------------------
// LayoutOutput — resolved layout for each node
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct NodeLayout {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub content_width: f32,
    pub content_height: f32,
    pub padding: Rect,
    pub border: Rect,
    pub margin: Rect,
}

impl NodeLayout {
    pub const ZERO: Self = Self {
        x: 0.0,
        y: 0.0,
        width: 0.0,
        height: 0.0,
        content_width: 0.0,
        content_height: 0.0,
        padding: Rect::ZERO,
        border: Rect::ZERO,
        margin: Rect::ZERO,
    };
}

#[derive(Debug, Clone)]
pub struct LayoutOutput {
    pub nodes: Vec<(NodeId, NodeLayout)>,
}

impl LayoutOutput {
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    pub fn get(&self, id: NodeId) -> Option<&NodeLayout> {
        self.nodes
            .iter()
            .find(|(node_id, _)| *node_id == id)
            .map(|(_, layout)| layout)
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

impl Default for LayoutOutput {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Taffy conversion helpers
// ---------------------------------------------------------------------------

pub(crate) fn dimension_to_taffy(d: Dimension) -> taffy::style::Dimension {
    match d {
        Dimension::Px(v) => taffy::style::Dimension::length(v),
        Dimension::Percent(v) => taffy::style::Dimension::percent(v / 100.0),
        Dimension::Auto => taffy::style::Dimension::auto(),
    }
}

pub(crate) fn dimension_to_length_percentage(d: Dimension) -> taffy::style::LengthPercentage {
    match d {
        Dimension::Px(v) => taffy::style::LengthPercentage::length(v),
        Dimension::Percent(v) => taffy::style::LengthPercentage::percent(v / 100.0),
        Dimension::Auto => taffy::style::LengthPercentage::length(0.0),
    }
}

pub(crate) fn dimension_to_length_percentage_auto(
    d: Dimension,
) -> taffy::style::LengthPercentageAuto {
    match d {
        Dimension::Px(v) => taffy::style::LengthPercentageAuto::length(v),
        Dimension::Percent(v) => taffy::style::LengthPercentageAuto::percent(v / 100.0),
        Dimension::Auto => taffy::style::LengthPercentageAuto::auto(),
    }
}

pub(crate) fn dimension_rect_to_padding(
    dr: &DimensionRect,
) -> taffy::geometry::Rect<taffy::style::LengthPercentage> {
    taffy::geometry::Rect {
        left: dimension_to_length_percentage(dr.left),
        right: dimension_to_length_percentage(dr.right),
        top: dimension_to_length_percentage(dr.top),
        bottom: dimension_to_length_percentage(dr.bottom),
    }
}

pub(crate) fn dimension_rect_to_margin(
    dr: &DimensionRect,
) -> taffy::geometry::Rect<taffy::style::LengthPercentageAuto> {
    taffy::geometry::Rect {
        left: dimension_to_length_percentage_auto(dr.left),
        right: dimension_to_length_percentage_auto(dr.right),
        top: dimension_to_length_percentage_auto(dr.top),
        bottom: dimension_to_length_percentage_auto(dr.bottom),
    }
}

pub(crate) fn layout_style_to_taffy(style: &LayoutStyle) -> taffy::Style {
    taffy::Style {
        display: match style.display {
            Display::Flex => taffy::Display::Flex,
            Display::None => taffy::Display::None,
        },
        flex_direction: match style.flex_direction {
            FlexDirection::Row => taffy::FlexDirection::Row,
            FlexDirection::Column => taffy::FlexDirection::Column,
            FlexDirection::RowReverse => taffy::FlexDirection::RowReverse,
            FlexDirection::ColumnReverse => taffy::FlexDirection::ColumnReverse,
        },
        flex_wrap: match style.flex_wrap {
            FlexWrap::NoWrap => taffy::FlexWrap::NoWrap,
            FlexWrap::Wrap => taffy::FlexWrap::Wrap,
            FlexWrap::WrapReverse => taffy::FlexWrap::WrapReverse,
        },
        justify_content: style.justify_content.map(|jc| match jc {
            JustifyContent::Start => taffy::JustifyContent::Start,
            JustifyContent::End => taffy::JustifyContent::End,
            JustifyContent::FlexStart => taffy::JustifyContent::FlexStart,
            JustifyContent::FlexEnd => taffy::JustifyContent::FlexEnd,
            JustifyContent::Center => taffy::JustifyContent::Center,
            JustifyContent::SpaceBetween => taffy::JustifyContent::SpaceBetween,
            JustifyContent::SpaceAround => taffy::JustifyContent::SpaceAround,
            JustifyContent::SpaceEvenly => taffy::JustifyContent::SpaceEvenly,
        }),
        align_items: style.align_items.map(|ai| match ai {
            AlignItems::Start => taffy::AlignItems::Start,
            AlignItems::End => taffy::AlignItems::End,
            AlignItems::FlexStart => taffy::AlignItems::FlexStart,
            AlignItems::FlexEnd => taffy::AlignItems::FlexEnd,
            AlignItems::Center => taffy::AlignItems::Center,
            AlignItems::Baseline => taffy::AlignItems::Baseline,
            AlignItems::Stretch => taffy::AlignItems::Stretch,
        }),
        align_self: style.align_self.map(|a| match a {
            AlignSelf::Start => taffy::AlignSelf::Start,
            AlignSelf::End => taffy::AlignSelf::End,
            AlignSelf::FlexStart => taffy::AlignSelf::FlexStart,
            AlignSelf::FlexEnd => taffy::AlignSelf::FlexEnd,
            AlignSelf::Center => taffy::AlignSelf::Center,
            AlignSelf::Baseline => taffy::AlignSelf::Baseline,
            AlignSelf::Stretch => taffy::AlignSelf::Stretch,
        }),
        align_content: style.align_content.map(|ac| match ac {
            AlignContent::Start => taffy::AlignContent::Start,
            AlignContent::End => taffy::AlignContent::End,
            AlignContent::FlexStart => taffy::AlignContent::FlexStart,
            AlignContent::FlexEnd => taffy::AlignContent::FlexEnd,
            AlignContent::Center => taffy::AlignContent::Center,
            AlignContent::Stretch => taffy::AlignContent::Stretch,
            AlignContent::SpaceBetween => taffy::AlignContent::SpaceBetween,
            AlignContent::SpaceAround => taffy::AlignContent::SpaceAround,
            AlignContent::SpaceEvenly => taffy::AlignContent::SpaceEvenly,
        }),
        flex_grow: style.flex_grow,
        flex_shrink: style.flex_shrink,
        flex_basis: dimension_to_taffy(style.flex_basis),
        size: taffy::Size {
            width: dimension_to_taffy(style.width),
            height: dimension_to_taffy(style.height),
        },
        min_size: taffy::Size {
            width: dimension_to_taffy(style.min_width),
            height: dimension_to_taffy(style.min_height),
        },
        max_size: taffy::Size {
            width: dimension_to_taffy(style.max_width),
            height: dimension_to_taffy(style.max_height),
        },
        padding: dimension_rect_to_padding(&style.padding),
        margin: dimension_rect_to_margin(&style.margin),
        gap: taffy::Size {
            width: taffy::style::LengthPercentage::length(style.gap.width),
            height: taffy::style::LengthPercentage::length(style.gap.height),
        },
        aspect_ratio: style.aspect_ratio,
        overflow: taffy::Point {
            x: match style.overflow {
                Overflow::Visible => taffy::Overflow::Visible,
                Overflow::Hidden => taffy::Overflow::Hidden,
                Overflow::Scroll => taffy::Overflow::Scroll,
            },
            y: match style.overflow {
                Overflow::Visible => taffy::Overflow::Visible,
                Overflow::Hidden => taffy::Overflow::Hidden,
                Overflow::Scroll => taffy::Overflow::Scroll,
            },
        },
        position: match style.position {
            Position::Relative => taffy::Position::Relative,
            Position::Absolute => taffy::Position::Absolute,
        },
        inset: dimension_rect_to_margin(&style.inset),
        ..taffy::Style::default()
    }
}
