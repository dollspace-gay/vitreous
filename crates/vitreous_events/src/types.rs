use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Geometry primitives (zero-dep, minimal — just what hit_test needs)
// ---------------------------------------------------------------------------

/// Opaque identifier for a node in the widget/layout tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NodeId(pub usize);

/// A 2D point in screen coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// An axis-aligned rectangle defined by its top-left corner and size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Returns `true` if `point` lies within (or on the boundary of) this rectangle.
    pub fn contains(&self, point: Point) -> bool {
        point.x >= self.x
            && point.x <= self.x + self.width
            && point.y >= self.y
            && point.y <= self.y + self.height
    }
}

/// Per-corner border radii (top-left, top-right, bottom-right, bottom-left).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Corners {
    pub top_left: f64,
    pub top_right: f64,
    pub bottom_right: f64,
    pub bottom_left: f64,
}

impl Corners {
    pub fn new(top_left: f64, top_right: f64, bottom_right: f64, bottom_left: f64) -> Self {
        Self {
            top_left,
            top_right,
            bottom_right,
            bottom_left,
        }
    }

    pub fn all(radius: f64) -> Self {
        Self {
            top_left: radius,
            top_right: radius,
            bottom_right: radius,
            bottom_left: radius,
        }
    }

    pub fn zero() -> Self {
        Self::all(0.0)
    }
}

/// A layout node as seen by the hit-test algorithm: its id, bounding rect,
/// and border radii. Nodes are stored in paint order (first = backmost).
#[derive(Clone, Debug)]
pub struct LayoutNode {
    pub id: NodeId,
    pub rect: Rect,
    pub corners: Corners,
}

// ---------------------------------------------------------------------------
// Modifier keys
// ---------------------------------------------------------------------------

/// State of modifier keys at the time of an event.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    /// Cmd on macOS, Win/Super on Windows/Linux.
    pub meta: bool,
}

impl Modifiers {
    pub fn none() -> Self {
        Self::default()
    }
}

// ---------------------------------------------------------------------------
// Mouse
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Back,
    Forward,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MouseEvent {
    /// Position relative to the receiving widget.
    pub x: f64,
    pub y: f64,
    /// Position relative to the window.
    pub global_x: f64,
    pub global_y: f64,
    pub button: MouseButton,
    pub modifiers: Modifiers,
}

// ---------------------------------------------------------------------------
// Keyboard
// ---------------------------------------------------------------------------

/// Logical key value — what the key *means* (layout-dependent).
///
/// Mirrors the W3C `KeyboardEvent.key` model. Only standard keys are included;
/// extend with `Other(String)` for platform-specific or exotic keys.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Key {
    // Characters
    Character(String),

    // Whitespace / editing
    Enter,
    Tab,
    Space,
    Backspace,
    Delete,
    Escape,

    // Navigation
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Home,
    End,
    PageUp,
    PageDown,

    // Modifiers (when the modifier key itself is pressed)
    Shift,
    Control,
    Alt,
    Meta,
    CapsLock,
    NumLock,
    ScrollLock,

    // Function keys
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    F13,
    F14,
    F15,
    F16,
    F17,
    F18,
    F19,
    F20,
    F21,
    F22,
    F23,
    F24,

    // Editing
    Insert,
    Cut,
    Copy,
    Paste,
    Undo,
    Redo,
    SelectAll,

    // Media
    MediaPlayPause,
    MediaStop,
    MediaTrackNext,
    MediaTrackPrevious,
    AudioVolumeUp,
    AudioVolumeDown,
    AudioVolumeMute,

    // Misc
    PrintScreen,
    Pause,
    ContextMenu,

    // Catch-all
    Other(String),
}

/// Physical key code — which physical key was pressed (layout-independent).
///
/// Mirrors the W3C `KeyboardEvent.code` model.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum KeyCode {
    // Letters
    KeyA,
    KeyB,
    KeyC,
    KeyD,
    KeyE,
    KeyF,
    KeyG,
    KeyH,
    KeyI,
    KeyJ,
    KeyK,
    KeyL,
    KeyM,
    KeyN,
    KeyO,
    KeyP,
    KeyQ,
    KeyR,
    KeyS,
    KeyT,
    KeyU,
    KeyV,
    KeyW,
    KeyX,
    KeyY,
    KeyZ,

    // Digits (main row)
    Digit0,
    Digit1,
    Digit2,
    Digit3,
    Digit4,
    Digit5,
    Digit6,
    Digit7,
    Digit8,
    Digit9,

    // Numpad
    Numpad0,
    Numpad1,
    Numpad2,
    Numpad3,
    Numpad4,
    Numpad5,
    Numpad6,
    Numpad7,
    Numpad8,
    Numpad9,
    NumpadAdd,
    NumpadSubtract,
    NumpadMultiply,
    NumpadDivide,
    NumpadDecimal,
    NumpadEnter,

    // Function keys
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    F13,
    F14,
    F15,
    F16,
    F17,
    F18,
    F19,
    F20,
    F21,
    F22,
    F23,
    F24,

    // Modifiers
    ShiftLeft,
    ShiftRight,
    ControlLeft,
    ControlRight,
    AltLeft,
    AltRight,
    MetaLeft,
    MetaRight,
    CapsLock,
    NumLock,
    ScrollLock,

    // Whitespace / editing
    Enter,
    Tab,
    Space,
    Backspace,
    Delete,
    Insert,
    Escape,

    // Navigation
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Home,
    End,
    PageUp,
    PageDown,

    // Punctuation / symbols
    Minus,
    Equal,
    BracketLeft,
    BracketRight,
    Backslash,
    Semicolon,
    Quote,
    Backquote,
    Comma,
    Period,
    Slash,

    // Misc
    PrintScreen,
    Pause,
    ContextMenu,

    // Catch-all
    Unidentified,
    Other(u32),
}

#[derive(Clone, Debug, PartialEq)]
pub struct KeyEvent {
    /// The logical key value.
    pub key: Key,
    /// The physical key code.
    pub code: KeyCode,
    pub modifiers: Modifiers,
    /// Whether this is a repeat event from holding down the key.
    pub repeat: bool,
    /// The text that this key press would insert, if any.
    pub text: Option<String>,
}

// ---------------------------------------------------------------------------
// Scroll
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub struct ScrollEvent {
    pub delta_x: f64,
    pub delta_y: f64,
    pub modifiers: Modifiers,
}

// ---------------------------------------------------------------------------
// Drop / Drag
// ---------------------------------------------------------------------------

/// Payload carried by a drop event.
#[derive(Clone, Debug, PartialEq)]
pub enum DropData {
    Files(Vec<PathBuf>),
    Text(String),
    Custom(Vec<u8>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct DropEvent {
    /// Position relative to the receiving widget.
    pub x: f64,
    pub y: f64,
    pub data: DropData,
}

/// Minimal stub for drag configuration. Detailed design deferred to Phase 3.
#[derive(Clone, Debug, Default)]
pub struct DragConfig {
    pub enabled: bool,
}

// ---------------------------------------------------------------------------
// Cursor
// ---------------------------------------------------------------------------

/// Standard cursor icons. Maps to platform cursors (winit/web).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CursorIcon {
    #[default]
    Default,
    Pointer,
    Text,
    Crosshair,
    Move,
    NotAllowed,
    Grab,
    Grabbing,
    ResizeNorth,
    ResizeSouth,
    ResizeEast,
    ResizeWest,
    ResizeNorthEast,
    ResizeNorthWest,
    ResizeSouthEast,
    ResizeSouthWest,
    ResizeColumn,
    ResizeRow,
    Wait,
    Progress,
    Help,
    ZoomIn,
    ZoomOut,
    None,
}

// ---------------------------------------------------------------------------
// EventHandlers
// ---------------------------------------------------------------------------

/// Holds optional closures for all event types that a widget can handle.
///
/// This type is intentionally `!Send` and `!Sync` because closures may capture
/// signal handles that are tied to a single-threaded reactive runtime.
#[derive(Default)]
pub struct EventHandlers {
    pub on_click: Option<Box<dyn Fn()>>,
    pub on_double_click: Option<Box<dyn Fn()>>,
    pub on_mouse_down: Option<Box<dyn Fn(MouseEvent)>>,
    pub on_mouse_up: Option<Box<dyn Fn(MouseEvent)>>,
    pub on_mouse_move: Option<Box<dyn Fn(MouseEvent)>>,
    pub on_mouse_enter: Option<Box<dyn Fn()>>,
    pub on_mouse_leave: Option<Box<dyn Fn()>>,
    pub on_scroll: Option<Box<dyn Fn(ScrollEvent)>>,
    pub on_key_down: Option<Box<dyn Fn(KeyEvent)>>,
    pub on_key_up: Option<Box<dyn Fn(KeyEvent)>>,
    pub on_focus: Option<Box<dyn Fn()>>,
    pub on_blur: Option<Box<dyn Fn()>>,
    pub on_drag: Option<DragConfig>,
    pub on_drop: Option<Box<dyn Fn(DropEvent)>>,
}

impl std::fmt::Debug for EventHandlers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventHandlers")
            .field("on_click", &self.on_click.as_ref().map(|_| ".."))
            .field(
                "on_double_click",
                &self.on_double_click.as_ref().map(|_| ".."),
            )
            .field("on_mouse_down", &self.on_mouse_down.as_ref().map(|_| ".."))
            .field("on_mouse_up", &self.on_mouse_up.as_ref().map(|_| ".."))
            .field("on_mouse_move", &self.on_mouse_move.as_ref().map(|_| ".."))
            .field(
                "on_mouse_enter",
                &self.on_mouse_enter.as_ref().map(|_| ".."),
            )
            .field(
                "on_mouse_leave",
                &self.on_mouse_leave.as_ref().map(|_| ".."),
            )
            .field("on_scroll", &self.on_scroll.as_ref().map(|_| ".."))
            .field("on_key_down", &self.on_key_down.as_ref().map(|_| ".."))
            .field("on_key_up", &self.on_key_up.as_ref().map(|_| ".."))
            .field("on_focus", &self.on_focus.as_ref().map(|_| ".."))
            .field("on_blur", &self.on_blur.as_ref().map(|_| ".."))
            .field("on_drag", &self.on_drag)
            .field("on_drop", &self.on_drop.as_ref().map(|_| ".."))
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // AC-1: MouseEvent with all fields, button == MouseButton::Left comparison
    #[test]
    fn mouse_event_construction_and_comparison() {
        let event = MouseEvent {
            x: 10.0,
            y: 20.0,
            global_x: 110.0,
            global_y: 220.0,
            button: MouseButton::Left,
            modifiers: Modifiers {
                shift: true,
                ctrl: false,
                alt: false,
                meta: false,
            },
        };
        assert_eq!(event.button, MouseButton::Left);
        assert_ne!(event.button, MouseButton::Right);
        assert!(event.modifiers.shift);
        assert!(!event.modifiers.ctrl);
        assert_eq!(event.x, 10.0);
        assert_eq!(event.global_x, 110.0);
    }

    // AC-2: KeyEvent with key: Key::Enter, modifiers.ctrl: true
    #[test]
    fn key_event_construction_and_matching() {
        let event = KeyEvent {
            key: Key::Enter,
            code: KeyCode::Enter,
            modifiers: Modifiers {
                shift: false,
                ctrl: true,
                alt: false,
                meta: false,
            },
            repeat: false,
            text: None,
        };
        assert_eq!(event.key, Key::Enter);
        assert!(event.modifiers.ctrl);
        assert!(!event.repeat);

        // Pattern matching works
        match &event.key {
            Key::Enter => {}
            _ => panic!("expected Key::Enter"),
        }
    }

    // AC-8: DropData::Files roundtrips correctly
    #[test]
    fn drop_data_files_roundtrip() {
        let data = DropData::Files(vec![PathBuf::from("/test")]);
        let cloned = data.clone();
        assert_eq!(data, cloned);

        match &data {
            DropData::Files(files) => {
                assert_eq!(files.len(), 1);
                assert_eq!(files[0], PathBuf::from("/test"));
            }
            _ => panic!("expected DropData::Files"),
        }
    }

    // AC-9: EventHandlers::default() has all handlers as None
    #[test]
    fn event_handlers_default_all_none() {
        let handlers = EventHandlers::default();
        assert!(handlers.on_click.is_none());
        assert!(handlers.on_double_click.is_none());
        assert!(handlers.on_mouse_down.is_none());
        assert!(handlers.on_mouse_up.is_none());
        assert!(handlers.on_mouse_move.is_none());
        assert!(handlers.on_mouse_enter.is_none());
        assert!(handlers.on_mouse_leave.is_none());
        assert!(handlers.on_scroll.is_none());
        assert!(handlers.on_key_down.is_none());
        assert!(handlers.on_key_up.is_none());
        assert!(handlers.on_focus.is_none());
        assert!(handlers.on_blur.is_none());
        assert!(handlers.on_drag.is_none());
        assert!(handlers.on_drop.is_none());
    }

    #[test]
    fn modifiers_none_is_all_false() {
        let m = Modifiers::none();
        assert!(!m.shift);
        assert!(!m.ctrl);
        assert!(!m.alt);
        assert!(!m.meta);
    }

    #[test]
    fn scroll_event_construction() {
        let event = ScrollEvent {
            delta_x: -1.0,
            delta_y: 3.5,
            modifiers: Modifiers::none(),
        };
        assert_eq!(event.delta_x, -1.0);
        assert_eq!(event.delta_y, 3.5);
    }

    #[test]
    fn drop_event_with_text() {
        let event = DropEvent {
            x: 50.0,
            y: 60.0,
            data: DropData::Text("hello".into()),
        };
        assert_eq!(event.data, DropData::Text("hello".into()));
    }

    #[test]
    fn drop_event_with_custom_data() {
        let event = DropEvent {
            x: 0.0,
            y: 0.0,
            data: DropData::Custom(vec![0xDE, 0xAD]),
        };
        match &event.data {
            DropData::Custom(bytes) => assert_eq!(bytes, &[0xDE, 0xAD]),
            _ => panic!("expected DropData::Custom"),
        }
    }

    #[test]
    fn cursor_icon_default_is_default() {
        assert_eq!(CursorIcon::default(), CursorIcon::Default);
    }

    #[test]
    fn rect_contains() {
        let r = Rect::new(10.0, 10.0, 50.0, 50.0);
        assert!(r.contains(Point::new(10.0, 10.0))); // top-left corner (inclusive)
        assert!(r.contains(Point::new(60.0, 60.0))); // bottom-right corner (inclusive)
        assert!(r.contains(Point::new(35.0, 35.0))); // center
        assert!(!r.contains(Point::new(9.0, 10.0))); // just outside left
        assert!(!r.contains(Point::new(10.0, 61.0))); // just outside bottom
    }

    #[test]
    fn corners_constructors() {
        let all = Corners::all(10.0);
        assert_eq!(all.top_left, 10.0);
        assert_eq!(all.bottom_right, 10.0);

        let zero = Corners::zero();
        assert_eq!(zero.top_left, 0.0);

        let custom = Corners::new(1.0, 2.0, 3.0, 4.0);
        assert_eq!(custom.top_left, 1.0);
        assert_eq!(custom.top_right, 2.0);
        assert_eq!(custom.bottom_right, 3.0);
        assert_eq!(custom.bottom_left, 4.0);
    }

    #[test]
    fn key_character_variant() {
        let key = Key::Character("a".into());
        assert_eq!(key, Key::Character("a".into()));
        assert_ne!(key, Key::Character("b".into()));
    }

    #[test]
    fn mouse_button_all_variants() {
        let buttons = [
            MouseButton::Left,
            MouseButton::Right,
            MouseButton::Middle,
            MouseButton::Back,
            MouseButton::Forward,
        ];
        // All distinct
        for (i, a) in buttons.iter().enumerate() {
            for (j, b) in buttons.iter().enumerate() {
                assert_eq!(i == j, a == b);
            }
        }
    }
}
