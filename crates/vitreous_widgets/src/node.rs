use std::fmt;
use std::time::Duration;

use vitreous_a11y::{AccessibilityInfo, LivePoliteness, Role};
use vitreous_events::{DragConfig, DropEvent, EventHandlers, KeyEvent, MouseEvent, ScrollEvent};
use vitreous_style::{
    Animation, Color, Corners, CursorIcon, Dimension, Edges, FontFamily, FontWeight, Shadow, Style,
    TextAlign, TextOverflow, Transition,
};

// ---------------------------------------------------------------------------
// Key — stable identity for diffing
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Key {
    Str(String),
    Int(u64),
}

impl From<&str> for Key {
    fn from(s: &str) -> Self {
        Key::Str(s.to_owned())
    }
}

impl From<String> for Key {
    fn from(s: String) -> Self {
        Key::Str(s)
    }
}

impl From<u64> for Key {
    fn from(n: u64) -> Self {
        Key::Int(n)
    }
}

impl From<usize> for Key {
    fn from(n: usize) -> Self {
        Key::Int(n as u64)
    }
}

impl From<i32> for Key {
    fn from(n: i32) -> Self {
        Key::Int(n as u64)
    }
}

// ---------------------------------------------------------------------------
// TextContent — static or reactive text
// ---------------------------------------------------------------------------

pub enum TextContent {
    Static(String),
    Dynamic(Box<dyn Fn() -> String>),
}

impl fmt::Debug for TextContent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TextContent::Static(s) => write!(f, "TextContent::Static({s:?})"),
            TextContent::Dynamic(_) => write!(f, "TextContent::Dynamic(<fn>)"),
        }
    }
}

/// Trait for converting into text content — supports both static strings and
/// reactive closures.
pub trait IntoTextContent {
    fn into_text_content(self) -> TextContent;
}

impl IntoTextContent for &str {
    fn into_text_content(self) -> TextContent {
        TextContent::Static(self.to_owned())
    }
}

impl IntoTextContent for String {
    fn into_text_content(self) -> TextContent {
        TextContent::Static(self)
    }
}

impl<F: Fn() -> String + 'static> IntoTextContent for F {
    fn into_text_content(self) -> TextContent {
        TextContent::Dynamic(Box::new(self))
    }
}

// ---------------------------------------------------------------------------
// ImageSource — where an image comes from
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum ImageSource {
    Path(String),
    Url(String),
    Bytes(Vec<u8>),
}

impl From<&str> for ImageSource {
    fn from(s: &str) -> Self {
        if s.starts_with("http://") || s.starts_with("https://") {
            ImageSource::Url(s.to_owned())
        } else {
            ImageSource::Path(s.to_owned())
        }
    }
}

impl From<String> for ImageSource {
    fn from(s: String) -> Self {
        if s.starts_with("http://") || s.starts_with("https://") {
            ImageSource::Url(s)
        } else {
            ImageSource::Path(s)
        }
    }
}

impl From<Vec<u8>> for ImageSource {
    fn from(bytes: Vec<u8>) -> Self {
        ImageSource::Bytes(bytes)
    }
}

// ---------------------------------------------------------------------------
// CanvasPaintFn — platform-specific canvas callback
// ---------------------------------------------------------------------------

pub type CanvasPaintFn = Box<dyn Fn() + 'static>;

// ---------------------------------------------------------------------------
// NativeViewDescriptor — opaque handle for platform-native views
// ---------------------------------------------------------------------------

pub struct NativeViewDescriptor {
    pub type_name: String,
    pub properties: Vec<(String, String)>,
}

impl fmt::Debug for NativeViewDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NativeViewDescriptor")
            .field("type_name", &self.type_name)
            .finish()
    }
}

// ---------------------------------------------------------------------------
// ComponentFn — a component function that produces a Node
// ---------------------------------------------------------------------------

pub struct ComponentFn {
    pub name: String,
    pub render: Box<dyn Fn() -> Node>,
}

impl fmt::Debug for ComponentFn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ComponentFn({:?})", self.name)
    }
}

// ---------------------------------------------------------------------------
// AlignSelf — local enum to avoid layout dependency
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AlignSelf {
    Start,
    End,
    FlexStart,
    FlexEnd,
    Center,
    Baseline,
    Stretch,
}

// ---------------------------------------------------------------------------
// JustifyContent — main-axis alignment
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

// ---------------------------------------------------------------------------
// AlignItems — cross-axis alignment
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AlignItems {
    Start,
    End,
    FlexStart,
    FlexEnd,
    Center,
    Baseline,
    Stretch,
}

// ---------------------------------------------------------------------------
// FlexWrap — whether flex items wrap
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FlexWrap {
    #[default]
    NoWrap,
    Wrap,
    WrapReverse,
}

// ---------------------------------------------------------------------------
// FlexDirection — needed for stack containers
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FlexDirection {
    Row,
    #[default]
    Column,
}

// ---------------------------------------------------------------------------
// Position — relative or absolute positioning
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Position {
    #[default]
    Relative,
    Absolute,
}

// ---------------------------------------------------------------------------
// NodeKind — what kind of node this is
// ---------------------------------------------------------------------------

pub enum NodeKind {
    Container,
    Text(TextContent),
    Image(ImageSource),
    Canvas(CanvasPaintFn),
    NativeEmbed(NativeViewDescriptor),
    Component(ComponentFn),
}

impl fmt::Debug for NodeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NodeKind::Container => write!(f, "Container"),
            NodeKind::Text(tc) => write!(f, "Text({tc:?})"),
            NodeKind::Image(src) => write!(f, "Image({src:?})"),
            NodeKind::Canvas(_) => write!(f, "Canvas(<fn>)"),
            NodeKind::NativeEmbed(desc) => write!(f, "NativeEmbed({desc:?})"),
            NodeKind::Component(comp) => write!(f, "Component({comp:?})"),
        }
    }
}

// ---------------------------------------------------------------------------
// Node — the core UI tree element
// ---------------------------------------------------------------------------

pub struct Node {
    pub kind: NodeKind,
    pub style: Style,
    pub a11y: AccessibilityInfo,
    pub event_handlers: EventHandlers,
    pub children: Vec<Node>,
    pub key: Option<Key>,

    // Layout flex properties (not in vitreous_style::Style)
    pub flex_direction: FlexDirection,
    pub flex_wrap: FlexWrap,
    pub justify_content: Option<JustifyContent>,
    pub align_items: Option<AlignItems>,
    pub flex_grow: f32,
    pub flex_shrink: f32,
    pub flex_basis: Dimension,
    pub align_self: Option<AlignSelf>,
    pub gap: f32,
    pub aspect_ratio: Option<f32>,
    pub position: Position,

    // Animation
    pub animations: Vec<Animation>,
}

impl fmt::Debug for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Node")
            .field("kind", &self.kind)
            .field("key", &self.key)
            .field("children", &self.children.len())
            .finish()
    }
}

impl Node {
    /// Create a new container node with no children.
    pub fn new(kind: NodeKind) -> Self {
        Self {
            kind,
            style: Style::default(),
            a11y: AccessibilityInfo::default(),
            event_handlers: EventHandlers::default(),
            children: Vec::new(),
            key: None,
            flex_direction: FlexDirection::default(),
            flex_wrap: FlexWrap::default(),
            justify_content: None,
            align_items: None,
            flex_grow: 0.0,
            flex_shrink: 1.0,
            flex_basis: Dimension::Auto,
            align_self: None,
            gap: 0.0,
            aspect_ratio: None,
            position: Position::default(),
            animations: Vec::new(),
        }
    }

    /// Create a container node with children.
    pub fn with_children(kind: NodeKind, children: Vec<Node>) -> Self {
        let mut node = Self::new(kind);
        node.children = children;
        node
    }

    // -----------------------------------------------------------------------
    // Layout modifiers (REQ-3)
    // -----------------------------------------------------------------------

    pub fn width(mut self, w: impl Into<Dimension>) -> Self {
        self.style.width = w.into();
        self
    }

    pub fn height(mut self, h: impl Into<Dimension>) -> Self {
        self.style.height = h.into();
        self
    }

    pub fn min_width(mut self, w: impl Into<Dimension>) -> Self {
        self.style.min_width = w.into();
        self
    }

    pub fn max_width(mut self, w: impl Into<Dimension>) -> Self {
        self.style.max_width = w.into();
        self
    }

    pub fn min_height(mut self, h: impl Into<Dimension>) -> Self {
        self.style.min_height = h.into();
        self
    }

    pub fn max_height(mut self, h: impl Into<Dimension>) -> Self {
        self.style.max_height = h.into();
        self
    }

    pub fn padding(mut self, p: impl Into<Edges>) -> Self {
        self.style.padding = p.into();
        self
    }

    pub fn padding_x(mut self, px: f32) -> Self {
        self.style.padding.left = px;
        self.style.padding.right = px;
        self
    }

    pub fn padding_y(mut self, py: f32) -> Self {
        self.style.padding.top = py;
        self.style.padding.bottom = py;
        self
    }

    pub fn margin(mut self, m: impl Into<Edges>) -> Self {
        self.style.margin = m.into();
        self
    }

    pub fn flex_grow(mut self, grow: f32) -> Self {
        self.flex_grow = grow;
        self
    }

    pub fn flex_shrink(mut self, shrink: f32) -> Self {
        self.flex_shrink = shrink;
        self
    }

    pub fn flex_basis(mut self, basis: impl Into<Dimension>) -> Self {
        self.flex_basis = basis.into();
        self
    }

    pub fn flex_wrap(mut self, wrap: FlexWrap) -> Self {
        self.flex_wrap = wrap;
        self
    }

    pub fn justify_content(mut self, jc: JustifyContent) -> Self {
        self.justify_content = Some(jc);
        self
    }

    pub fn align_items(mut self, ai: AlignItems) -> Self {
        self.align_items = Some(ai);
        self
    }

    pub fn align_self(mut self, align: AlignSelf) -> Self {
        self.align_self = Some(align);
        self
    }

    pub fn position(mut self, pos: Position) -> Self {
        self.position = pos;
        self
    }

    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    pub fn aspect_ratio(mut self, ratio: f32) -> Self {
        self.aspect_ratio = Some(ratio);
        self
    }

    // -----------------------------------------------------------------------
    // Visual modifiers (REQ-4)
    // -----------------------------------------------------------------------

    pub fn background(mut self, color: impl Into<Color>) -> Self {
        self.style.background = Some(color.into());
        self
    }

    pub fn foreground(mut self, color: impl Into<Color>) -> Self {
        self.style.foreground = Some(color.into());
        self
    }

    pub fn border(mut self, width: f32, color: impl Into<Color>) -> Self {
        self.style.border_width = Edges {
            top: width,
            right: width,
            bottom: width,
            left: width,
        };
        self.style.border_color = Some(color.into());
        self
    }

    pub fn border_radius(mut self, radius: impl Into<Corners>) -> Self {
        self.style.border_radius = radius.into();
        self
    }

    pub fn shadow(mut self, shadow: Shadow) -> Self {
        self.style.shadow = Some(shadow);
        self
    }

    pub fn opacity(mut self, opacity: f32) -> Self {
        self.style.opacity = opacity;
        self
    }

    pub fn clip(mut self) -> Self {
        self.style.clip_content = true;
        self
    }

    // -----------------------------------------------------------------------
    // Text modifiers (REQ-5)
    // -----------------------------------------------------------------------

    pub fn font_size(mut self, size: f32) -> Self {
        self.style.font_size = Some(size);
        self
    }

    pub fn font_weight(mut self, weight: FontWeight) -> Self {
        self.style.font_weight = Some(weight);
        self
    }

    pub fn font_family(mut self, family: FontFamily) -> Self {
        self.style.font_family = Some(family);
        self
    }

    pub fn text_align(mut self, align: TextAlign) -> Self {
        self.style.text_align = Some(align);
        self
    }

    pub fn line_height(mut self, lh: f32) -> Self {
        self.style.line_height = Some(lh);
        self
    }

    pub fn text_overflow(mut self, overflow: TextOverflow) -> Self {
        self.style.text_overflow = Some(overflow);
        self
    }

    // -----------------------------------------------------------------------
    // Interaction modifiers (REQ-6)
    // -----------------------------------------------------------------------

    pub fn on_click(mut self, handler: impl Fn() + 'static) -> Self {
        self.event_handlers.on_click = Some(Box::new(handler));
        self
    }

    pub fn on_double_click(mut self, handler: impl Fn() + 'static) -> Self {
        self.event_handlers.on_double_click = Some(Box::new(handler));
        self
    }

    pub fn on_mouse_down(mut self, handler: impl Fn(MouseEvent) + 'static) -> Self {
        self.event_handlers.on_mouse_down = Some(Box::new(handler));
        self
    }

    pub fn on_mouse_up(mut self, handler: impl Fn(MouseEvent) + 'static) -> Self {
        self.event_handlers.on_mouse_up = Some(Box::new(handler));
        self
    }

    pub fn on_mouse_move(mut self, handler: impl Fn(MouseEvent) + 'static) -> Self {
        self.event_handlers.on_mouse_move = Some(Box::new(handler));
        self
    }

    pub fn on_mouse_enter(mut self, handler: impl Fn() + 'static) -> Self {
        self.event_handlers.on_mouse_enter = Some(Box::new(handler));
        self
    }

    pub fn on_mouse_leave(mut self, handler: impl Fn() + 'static) -> Self {
        self.event_handlers.on_mouse_leave = Some(Box::new(handler));
        self
    }

    pub fn on_scroll(mut self, handler: impl Fn(ScrollEvent) + 'static) -> Self {
        self.event_handlers.on_scroll = Some(Box::new(handler));
        self
    }

    pub fn on_key_down(mut self, handler: impl Fn(KeyEvent) + 'static) -> Self {
        self.event_handlers.on_key_down = Some(Box::new(handler));
        self
    }

    pub fn on_key_up(mut self, handler: impl Fn(KeyEvent) + 'static) -> Self {
        self.event_handlers.on_key_up = Some(Box::new(handler));
        self
    }

    pub fn on_focus(mut self, handler: impl Fn() + 'static) -> Self {
        self.event_handlers.on_focus = Some(Box::new(handler));
        self
    }

    pub fn on_blur(mut self, handler: impl Fn() + 'static) -> Self {
        self.event_handlers.on_blur = Some(Box::new(handler));
        self
    }

    pub fn on_drag(mut self) -> Self {
        self.event_handlers.on_drag = Some(DragConfig { enabled: true });
        self
    }

    pub fn on_drop(mut self, handler: impl Fn(DropEvent) + 'static) -> Self {
        self.event_handlers.on_drop = Some(Box::new(handler));
        self
    }

    pub fn cursor(mut self, cursor: CursorIcon) -> Self {
        self.style.cursor = Some(cursor);
        self
    }

    pub fn focusable(mut self, focusable: bool) -> Self {
        self.a11y.state.focusable = focusable;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.a11y.state.disabled = disabled;
        if disabled {
            self.a11y.state.focusable = false;
        }
        self
    }

    // -----------------------------------------------------------------------
    // Accessibility modifiers (REQ-7)
    // -----------------------------------------------------------------------

    pub fn role(mut self, role: Role) -> Self {
        self.a11y.role = role;
        self
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.a11y.label = Some(label.into());
        self
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.a11y.description = Some(desc.into());
        self
    }

    pub fn live_region(mut self, politeness: LivePoliteness) -> Self {
        self.a11y.live = politeness;
        self
    }

    // -----------------------------------------------------------------------
    // Animation modifiers (REQ-8)
    // -----------------------------------------------------------------------

    pub fn transition(
        mut self,
        property: vitreous_style::AnimatableProperty,
        duration: Duration,
    ) -> Self {
        self.style
            .transitions
            .push(Transition::new(property, duration));
        self
    }

    pub fn animate(mut self, animation: Animation) -> Self {
        self.animations.push(animation);
        self
    }

    // -----------------------------------------------------------------------
    // Composition modifiers (REQ-9)
    // -----------------------------------------------------------------------

    pub fn key(mut self, key: impl Into<Key>) -> Self {
        self.key = Some(key.into());
        self
    }

    /// Apply a modifier function to this node.
    pub fn apply(self, f: impl FnOnce(Self) -> Self) -> Self {
        f(self)
    }

    /// Conditionally apply a modifier function.
    pub fn apply_if(self, condition: bool, f: impl FnOnce(Self) -> Self) -> Self {
        if condition { f(self) } else { self }
    }
}

// Node is NOT Clone — per architecture decision D-11.
// Widgets are functions: call again for a new node.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_builder_chain() {
        let node = Node::new(NodeKind::Container)
            .width(100.0)
            .height(200.0)
            .background(Color::RED)
            .opacity(0.5);

        assert_eq!(node.style.width, Dimension::Px(100.0));
        assert_eq!(node.style.height, Dimension::Px(200.0));
        assert_eq!(node.style.background, Some(Color::RED));
        assert_eq!(node.style.opacity, 0.5);
    }

    #[test]
    fn key_from_str() {
        let node = Node::new(NodeKind::Container).key("my-key");
        assert_eq!(node.key, Some(Key::Str("my-key".to_owned())));
    }

    #[test]
    fn apply_if_true() {
        let node = Node::new(NodeKind::Container).apply_if(true, |n| n.background(Color::RED));
        assert_eq!(node.style.background, Some(Color::RED));
    }

    #[test]
    fn apply_if_false() {
        let node = Node::new(NodeKind::Container).apply_if(false, |n| n.background(Color::RED));
        assert_eq!(node.style.background, None);
    }

    #[test]
    fn disabled_makes_unfocusable() {
        let node = Node::new(NodeKind::Container).disabled(true);
        assert!(node.a11y.state.disabled);
        assert!(!node.a11y.state.focusable);
    }

    #[test]
    fn into_text_content_static() {
        let tc = "hello".into_text_content();
        match tc {
            TextContent::Static(s) => assert_eq!(s, "hello"),
            TextContent::Dynamic(_) => panic!("expected static"),
        }
    }

    #[test]
    fn into_text_content_dynamic() {
        let tc = (|| "dynamic".to_owned()).into_text_content();
        match tc {
            TextContent::Dynamic(f) => assert_eq!(f(), "dynamic"),
            TextContent::Static(_) => panic!("expected dynamic"),
        }
    }

    #[test]
    fn image_source_from_path() {
        let src = ImageSource::from("test.png");
        match src {
            ImageSource::Path(p) => assert_eq!(p, "test.png"),
            _ => panic!("expected path"),
        }
    }

    #[test]
    fn image_source_from_url() {
        let src = ImageSource::from("https://example.com/img.png");
        match src {
            ImageSource::Url(u) => assert_eq!(u, "https://example.com/img.png"),
            _ => panic!("expected url"),
        }
    }

    #[test]
    fn flex_layout_defaults() {
        let node = Node::new(NodeKind::Container);
        assert_eq!(node.flex_grow, 0.0);
        assert_eq!(node.flex_shrink, 1.0);
        assert_eq!(node.flex_basis, Dimension::Auto);
        assert_eq!(node.align_self, None);
        assert_eq!(node.gap, 0.0);
        assert_eq!(node.aspect_ratio, None);
    }

    #[test]
    fn flex_modifiers() {
        let node = Node::new(NodeKind::Container)
            .flex_grow(1.0)
            .flex_shrink(0.0)
            .flex_basis(200.0)
            .align_self(AlignSelf::Center)
            .gap(8.0)
            .aspect_ratio(16.0 / 9.0);

        assert_eq!(node.flex_grow, 1.0);
        assert_eq!(node.flex_shrink, 0.0);
        assert_eq!(node.flex_basis, Dimension::Px(200.0));
        assert_eq!(node.align_self, Some(AlignSelf::Center));
        assert_eq!(node.gap, 8.0);
        assert!((node.aspect_ratio.unwrap() - 16.0 / 9.0).abs() < f32::EPSILON);
    }

    #[test]
    fn padding_xy() {
        let node = Node::new(NodeKind::Container)
            .padding_x(10.0)
            .padding_y(20.0);

        assert_eq!(node.style.padding.left, 10.0);
        assert_eq!(node.style.padding.right, 10.0);
        assert_eq!(node.style.padding.top, 20.0);
        assert_eq!(node.style.padding.bottom, 20.0);
    }

    #[test]
    fn a11y_modifiers() {
        let node = Node::new(NodeKind::Container)
            .role(Role::Button)
            .label("Click me")
            .description("A button")
            .live_region(LivePoliteness::Polite);

        assert_eq!(node.a11y.role, Role::Button);
        assert_eq!(node.a11y.label, Some("Click me".to_owned()));
        assert_eq!(node.a11y.description, Some("A button".to_owned()));
        assert_eq!(node.a11y.live, LivePoliteness::Polite);
    }
}
