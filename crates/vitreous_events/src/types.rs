use std::path::PathBuf;

/// Unique identifier for a node in the widget tree.
///
/// Used across the framework for hit testing, focus management,
/// accessibility tree generation, and event dispatching.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct NodeId(pub u64);

/// Logical key identity (what the key means, not where it is physically).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Key {
    // Letters
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,

    // Digits
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

    // Navigation
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Home,
    End,
    PageUp,
    PageDown,

    // Editing
    Backspace,
    Delete,
    Insert,

    // Whitespace / control
    Enter,
    Tab,
    Space,
    Escape,

    // Modifiers (as logical keys)
    Shift,
    Control,
    Alt,
    Meta,

    // Punctuation / symbols
    Comma,
    Period,
    Semicolon,
    Quote,
    BracketLeft,
    BracketRight,
    Backslash,
    Slash,
    Minus,
    Equal,
    Backquote,

    // Media
    MediaPlayPause,
    MediaStop,
    MediaTrackNext,
    MediaTrackPrevious,
    AudioVolumeUp,
    AudioVolumeDown,
    AudioVolumeMute,

    // Other
    CapsLock,
    NumLock,
    ScrollLock,
    PrintScreen,
    Pause,
    ContextMenu,

    /// A character produced by the key that doesn't map to a named variant.
    Character(String),

    /// An unidentified key.
    Unidentified,
}

/// Physical key location on the keyboard.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum KeyCode {
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
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Home,
    End,
    PageUp,
    PageDown,
    Backspace,
    Delete,
    Insert,
    Enter,
    Tab,
    Space,
    Escape,
    ShiftLeft,
    ShiftRight,
    ControlLeft,
    ControlRight,
    AltLeft,
    AltRight,
    MetaLeft,
    MetaRight,
    Comma,
    Period,
    Semicolon,
    Quote,
    BracketLeft,
    BracketRight,
    Backslash,
    Slash,
    Minus,
    Equal,
    Backquote,
    CapsLock,
    NumLock,
    ScrollLock,
    PrintScreen,
    Pause,
    ContextMenu,
    Unidentified,
}

/// Modifier keys held during an event.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub meta: bool,
}

/// A keyboard event.
#[derive(Clone, Debug)]
pub struct KeyEvent {
    /// The logical key identity.
    pub key: Key,
    /// The physical key code.
    pub code: KeyCode,
    /// Active modifier keys.
    pub modifiers: Modifiers,
    /// Whether this is a repeat event from holding the key.
    pub repeat: bool,
    /// Text produced by this key event (if any).
    pub text: Option<String>,
}

/// Mouse button identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Back,
    Forward,
}

/// A mouse event with position and button information.
#[derive(Clone, Debug)]
pub struct MouseEvent {
    /// X position relative to the target element.
    pub x: f64,
    /// Y position relative to the target element.
    pub y: f64,
    /// X position relative to the window.
    pub global_x: f64,
    /// Y position relative to the window.
    pub global_y: f64,
    /// Which button triggered the event.
    pub button: MouseButton,
    /// Active modifier keys.
    pub modifiers: Modifiers,
}

/// A scroll event.
#[derive(Clone, Debug)]
pub struct ScrollEvent {
    /// Horizontal scroll delta.
    pub delta_x: f64,
    /// Vertical scroll delta.
    pub delta_y: f64,
    /// Active modifier keys.
    pub modifiers: Modifiers,
}

/// Data carried by a drop event.
#[derive(Clone, Debug)]
pub enum DropData {
    Files(Vec<PathBuf>),
    Text(String),
    Custom(Vec<u8>),
}

/// A drag-and-drop event.
#[derive(Clone, Debug)]
pub struct DropEvent {
    /// X position of the drop.
    pub x: f64,
    /// Y position of the drop.
    pub y: f64,
    /// The dropped data.
    pub data: DropData,
}

/// Configuration for drag behavior (stub — detailed design in Phase 3).
#[derive(Clone, Debug)]
pub struct DragConfig {
    /// Whether dragging is enabled.
    pub enabled: bool,
}

/// Collection of event handler closures for a node.
///
/// All handlers are optional. This type is `!Send` and `!Sync` because
/// the closures may capture signal handles.
#[derive(Default)]
pub struct EventHandlers {
    pub on_click: Option<Box<dyn Fn() + 'static>>,
    pub on_double_click: Option<Box<dyn Fn() + 'static>>,
    pub on_mouse_down: Option<Box<dyn Fn(MouseEvent) + 'static>>,
    pub on_mouse_up: Option<Box<dyn Fn(MouseEvent) + 'static>>,
    pub on_mouse_move: Option<Box<dyn Fn(MouseEvent) + 'static>>,
    pub on_mouse_enter: Option<Box<dyn Fn() + 'static>>,
    pub on_mouse_leave: Option<Box<dyn Fn() + 'static>>,
    pub on_scroll: Option<Box<dyn Fn(ScrollEvent) + 'static>>,
    pub on_key_down: Option<Box<dyn Fn(KeyEvent) + 'static>>,
    pub on_key_up: Option<Box<dyn Fn(KeyEvent) + 'static>>,
    pub on_focus: Option<Box<dyn Fn() + 'static>>,
    pub on_blur: Option<Box<dyn Fn() + 'static>>,
    pub on_drag: Option<DragConfig>,
    pub on_drop: Option<Box<dyn Fn(DropEvent) + 'static>>,
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
