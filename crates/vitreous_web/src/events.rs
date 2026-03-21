use std::rc::Rc;

use vitreous_events::{Key, KeyCode, KeyEvent, Modifiers, MouseButton, MouseEvent, ScrollEvent};
use vitreous_reactive::batch;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{EventTarget, HtmlElement};

/// Extract `Modifiers` from a DOM `KeyboardEvent` or `MouseEvent`.
fn modifiers_from_keyboard(e: &web_sys::KeyboardEvent) -> Modifiers {
    Modifiers {
        shift: e.shift_key(),
        ctrl: e.ctrl_key(),
        alt: e.alt_key(),
        meta: e.meta_key(),
    }
}

fn modifiers_from_mouse(e: &web_sys::MouseEvent) -> Modifiers {
    Modifiers {
        shift: e.shift_key(),
        ctrl: e.ctrl_key(),
        alt: e.alt_key(),
        meta: e.meta_key(),
    }
}

/// Map a DOM `MouseEvent.button` to vitreous `MouseButton`.
fn dom_button(button: i16) -> MouseButton {
    match button {
        0 => MouseButton::Left,
        1 => MouseButton::Middle,
        2 => MouseButton::Right,
        3 => MouseButton::Back,
        4 => MouseButton::Forward,
        _ => MouseButton::Left,
    }
}

/// Map a DOM `KeyboardEvent.key` string to vitreous `Key`.
pub fn dom_key_to_key(key: &str) -> Key {
    match key {
        "Enter" => Key::Enter,
        "Tab" => Key::Tab,
        " " => Key::Space,
        "Backspace" => Key::Backspace,
        "Delete" => Key::Delete,
        "Escape" => Key::Escape,
        "ArrowUp" => Key::ArrowUp,
        "ArrowDown" => Key::ArrowDown,
        "ArrowLeft" => Key::ArrowLeft,
        "ArrowRight" => Key::ArrowRight,
        "Home" => Key::Home,
        "End" => Key::End,
        "PageUp" => Key::PageUp,
        "PageDown" => Key::PageDown,
        "Shift" => Key::Shift,
        "Control" => Key::Control,
        "Alt" => Key::Alt,
        "Meta" => Key::Meta,
        "CapsLock" => Key::CapsLock,
        "NumLock" => Key::NumLock,
        "ScrollLock" => Key::ScrollLock,
        "F1" => Key::F1,
        "F2" => Key::F2,
        "F3" => Key::F3,
        "F4" => Key::F4,
        "F5" => Key::F5,
        "F6" => Key::F6,
        "F7" => Key::F7,
        "F8" => Key::F8,
        "F9" => Key::F9,
        "F10" => Key::F10,
        "F11" => Key::F11,
        "F12" => Key::F12,
        "F13" => Key::F13,
        "F14" => Key::F14,
        "F15" => Key::F15,
        "F16" => Key::F16,
        "F17" => Key::F17,
        "F18" => Key::F18,
        "F19" => Key::F19,
        "F20" => Key::F20,
        "F21" => Key::F21,
        "F22" => Key::F22,
        "F23" => Key::F23,
        "F24" => Key::F24,
        "Insert" => Key::Insert,
        "Cut" => Key::Cut,
        "Copy" => Key::Copy,
        "Paste" => Key::Paste,
        "Undo" => Key::Undo,
        "Redo" => Key::Redo,
        "PrintScreen" => Key::PrintScreen,
        "Pause" => Key::Pause,
        "ContextMenu" => Key::ContextMenu,
        "MediaPlayPause" => Key::MediaPlayPause,
        "MediaStop" => Key::MediaStop,
        "MediaTrackNext" => Key::MediaTrackNext,
        "MediaTrackPrevious" => Key::MediaTrackPrevious,
        "AudioVolumeUp" => Key::AudioVolumeUp,
        "AudioVolumeDown" => Key::AudioVolumeDown,
        "AudioVolumeMute" => Key::AudioVolumeMute,
        s if s.len() == 1 => Key::Character(s.to_owned()),
        other => Key::Other(other.to_owned()),
    }
}

/// Map a DOM `KeyboardEvent.code` string to vitreous `KeyCode`.
pub fn dom_code_to_keycode(code: &str) -> KeyCode {
    match code {
        "KeyA" => KeyCode::KeyA,
        "KeyB" => KeyCode::KeyB,
        "KeyC" => KeyCode::KeyC,
        "KeyD" => KeyCode::KeyD,
        "KeyE" => KeyCode::KeyE,
        "KeyF" => KeyCode::KeyF,
        "KeyG" => KeyCode::KeyG,
        "KeyH" => KeyCode::KeyH,
        "KeyI" => KeyCode::KeyI,
        "KeyJ" => KeyCode::KeyJ,
        "KeyK" => KeyCode::KeyK,
        "KeyL" => KeyCode::KeyL,
        "KeyM" => KeyCode::KeyM,
        "KeyN" => KeyCode::KeyN,
        "KeyO" => KeyCode::KeyO,
        "KeyP" => KeyCode::KeyP,
        "KeyQ" => KeyCode::KeyQ,
        "KeyR" => KeyCode::KeyR,
        "KeyS" => KeyCode::KeyS,
        "KeyT" => KeyCode::KeyT,
        "KeyU" => KeyCode::KeyU,
        "KeyV" => KeyCode::KeyV,
        "KeyW" => KeyCode::KeyW,
        "KeyX" => KeyCode::KeyX,
        "KeyY" => KeyCode::KeyY,
        "KeyZ" => KeyCode::KeyZ,
        "Digit0" => KeyCode::Digit0,
        "Digit1" => KeyCode::Digit1,
        "Digit2" => KeyCode::Digit2,
        "Digit3" => KeyCode::Digit3,
        "Digit4" => KeyCode::Digit4,
        "Digit5" => KeyCode::Digit5,
        "Digit6" => KeyCode::Digit6,
        "Digit7" => KeyCode::Digit7,
        "Digit8" => KeyCode::Digit8,
        "Digit9" => KeyCode::Digit9,
        "Numpad0" => KeyCode::Numpad0,
        "Numpad1" => KeyCode::Numpad1,
        "Numpad2" => KeyCode::Numpad2,
        "Numpad3" => KeyCode::Numpad3,
        "Numpad4" => KeyCode::Numpad4,
        "Numpad5" => KeyCode::Numpad5,
        "Numpad6" => KeyCode::Numpad6,
        "Numpad7" => KeyCode::Numpad7,
        "Numpad8" => KeyCode::Numpad8,
        "Numpad9" => KeyCode::Numpad9,
        "NumpadAdd" => KeyCode::NumpadAdd,
        "NumpadSubtract" => KeyCode::NumpadSubtract,
        "NumpadMultiply" => KeyCode::NumpadMultiply,
        "NumpadDivide" => KeyCode::NumpadDivide,
        "NumpadDecimal" => KeyCode::NumpadDecimal,
        "NumpadEnter" => KeyCode::NumpadEnter,
        "F1" => KeyCode::F1,
        "F2" => KeyCode::F2,
        "F3" => KeyCode::F3,
        "F4" => KeyCode::F4,
        "F5" => KeyCode::F5,
        "F6" => KeyCode::F6,
        "F7" => KeyCode::F7,
        "F8" => KeyCode::F8,
        "F9" => KeyCode::F9,
        "F10" => KeyCode::F10,
        "F11" => KeyCode::F11,
        "F12" => KeyCode::F12,
        "F13" => KeyCode::F13,
        "F14" => KeyCode::F14,
        "F15" => KeyCode::F15,
        "F16" => KeyCode::F16,
        "F17" => KeyCode::F17,
        "F18" => KeyCode::F18,
        "F19" => KeyCode::F19,
        "F20" => KeyCode::F20,
        "F21" => KeyCode::F21,
        "F22" => KeyCode::F22,
        "F23" => KeyCode::F23,
        "F24" => KeyCode::F24,
        "ShiftLeft" => KeyCode::ShiftLeft,
        "ShiftRight" => KeyCode::ShiftRight,
        "ControlLeft" => KeyCode::ControlLeft,
        "ControlRight" => KeyCode::ControlRight,
        "AltLeft" => KeyCode::AltLeft,
        "AltRight" => KeyCode::AltRight,
        "MetaLeft" => KeyCode::MetaLeft,
        "MetaRight" => KeyCode::MetaRight,
        "CapsLock" => KeyCode::CapsLock,
        "NumLock" => KeyCode::NumLock,
        "ScrollLock" => KeyCode::ScrollLock,
        "Enter" => KeyCode::Enter,
        "Tab" => KeyCode::Tab,
        "Space" => KeyCode::Space,
        "Backspace" => KeyCode::Backspace,
        "Delete" => KeyCode::Delete,
        "Insert" => KeyCode::Insert,
        "Escape" => KeyCode::Escape,
        "ArrowUp" => KeyCode::ArrowUp,
        "ArrowDown" => KeyCode::ArrowDown,
        "ArrowLeft" => KeyCode::ArrowLeft,
        "ArrowRight" => KeyCode::ArrowRight,
        "Home" => KeyCode::Home,
        "End" => KeyCode::End,
        "PageUp" => KeyCode::PageUp,
        "PageDown" => KeyCode::PageDown,
        "Minus" => KeyCode::Minus,
        "Equal" => KeyCode::Equal,
        "BracketLeft" => KeyCode::BracketLeft,
        "BracketRight" => KeyCode::BracketRight,
        "Backslash" => KeyCode::Backslash,
        "Semicolon" => KeyCode::Semicolon,
        "Quote" => KeyCode::Quote,
        "Backquote" => KeyCode::Backquote,
        "Comma" => KeyCode::Comma,
        "Period" => KeyCode::Period,
        "Slash" => KeyCode::Slash,
        "PrintScreen" => KeyCode::PrintScreen,
        "Pause" => KeyCode::Pause,
        "ContextMenu" => KeyCode::ContextMenu,
        _ => KeyCode::Unidentified,
    }
}

/// A stored event listener closure paired with its event name.
type ListenerEntry = (String, Closure<dyn Fn(web_sys::Event)>);

/// A collection of stored `Closure`s that are attached as DOM event listeners.
/// Dropping this struct removes the listeners from the element.
pub struct EventListeners {
    pub(crate) closures: Vec<ListenerEntry>,
    pub(crate) target: EventTarget,
}

impl EventListeners {
    /// Create an empty listener set (no listeners attached).
    pub fn empty(target: EventTarget) -> Self {
        Self {
            closures: Vec::new(),
            target,
        }
    }
}

impl Drop for EventListeners {
    fn drop(&mut self) {
        for (event_name, closure) in &self.closures {
            let _ = self
                .target
                .remove_event_listener_with_callback(event_name, closure.as_ref().unchecked_ref());
        }
    }
}

/// Attach DOM event listeners from owned `EventHandlers`.
///
/// Takes ownership of the handlers so closures can capture them.
/// Returns an `EventListeners` guard that removes listeners on drop.
pub fn attach_event_listeners_owned(
    element: &HtmlElement,
    handlers: vitreous_events::EventHandlers,
) -> EventListeners {
    let target: EventTarget = element.clone().into();
    let mut closures: Vec<ListenerEntry> = Vec::new();

    // on_click
    if let Some(handler) = handlers.on_click {
        let handler = Rc::new(handler);
        let h = handler.clone();
        let closure = Closure::new(move |_: web_sys::Event| {
            batch(|| h());
        });
        let _ = target.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref());
        closures.push(("click".into(), closure));
    }

    // on_double_click
    if let Some(handler) = handlers.on_double_click {
        let handler = Rc::new(handler);
        let h = handler.clone();
        let closure = Closure::new(move |_: web_sys::Event| {
            batch(|| h());
        });
        let _ =
            target.add_event_listener_with_callback("dblclick", closure.as_ref().unchecked_ref());
        closures.push(("dblclick".into(), closure));
    }

    // on_mouse_down
    if let Some(handler) = handlers.on_mouse_down {
        let handler = Rc::new(handler);
        let h = handler.clone();
        let el = element.clone();
        let closure = Closure::new(move |e: web_sys::Event| {
            if let Some(me) = e.dyn_ref::<web_sys::MouseEvent>() {
                let rect = el.get_bounding_client_rect();
                let event = MouseEvent {
                    x: me.client_x() as f64 - rect.left(),
                    y: me.client_y() as f64 - rect.top(),
                    global_x: me.client_x() as f64,
                    global_y: me.client_y() as f64,
                    button: dom_button(me.button()),
                    modifiers: modifiers_from_mouse(me),
                };
                batch(|| h(event));
            }
        });
        let _ =
            target.add_event_listener_with_callback("mousedown", closure.as_ref().unchecked_ref());
        closures.push(("mousedown".into(), closure));
    }

    // on_mouse_up
    if let Some(handler) = handlers.on_mouse_up {
        let handler = Rc::new(handler);
        let h = handler.clone();
        let el = element.clone();
        let closure = Closure::new(move |e: web_sys::Event| {
            if let Some(me) = e.dyn_ref::<web_sys::MouseEvent>() {
                let rect = el.get_bounding_client_rect();
                let event = MouseEvent {
                    x: me.client_x() as f64 - rect.left(),
                    y: me.client_y() as f64 - rect.top(),
                    global_x: me.client_x() as f64,
                    global_y: me.client_y() as f64,
                    button: dom_button(me.button()),
                    modifiers: modifiers_from_mouse(me),
                };
                batch(|| h(event));
            }
        });
        let _ =
            target.add_event_listener_with_callback("mouseup", closure.as_ref().unchecked_ref());
        closures.push(("mouseup".into(), closure));
    }

    // on_mouse_move
    if let Some(handler) = handlers.on_mouse_move {
        let handler = Rc::new(handler);
        let h = handler.clone();
        let el = element.clone();
        let closure = Closure::new(move |e: web_sys::Event| {
            if let Some(me) = e.dyn_ref::<web_sys::MouseEvent>() {
                let rect = el.get_bounding_client_rect();
                let event = MouseEvent {
                    x: me.client_x() as f64 - rect.left(),
                    y: me.client_y() as f64 - rect.top(),
                    global_x: me.client_x() as f64,
                    global_y: me.client_y() as f64,
                    button: dom_button(me.button()),
                    modifiers: modifiers_from_mouse(me),
                };
                batch(|| h(event));
            }
        });
        let _ =
            target.add_event_listener_with_callback("mousemove", closure.as_ref().unchecked_ref());
        closures.push(("mousemove".into(), closure));
    }

    // on_mouse_enter
    if let Some(handler) = handlers.on_mouse_enter {
        let handler = Rc::new(handler);
        let h = handler.clone();
        let closure = Closure::new(move |_: web_sys::Event| {
            batch(|| h());
        });
        let _ =
            target.add_event_listener_with_callback("mouseenter", closure.as_ref().unchecked_ref());
        closures.push(("mouseenter".into(), closure));
    }

    // on_mouse_leave
    if let Some(handler) = handlers.on_mouse_leave {
        let handler = Rc::new(handler);
        let h = handler.clone();
        let closure = Closure::new(move |_: web_sys::Event| {
            batch(|| h());
        });
        let _ =
            target.add_event_listener_with_callback("mouseleave", closure.as_ref().unchecked_ref());
        closures.push(("mouseleave".into(), closure));
    }

    // on_scroll
    if let Some(handler) = handlers.on_scroll {
        let handler = Rc::new(handler);
        let h = handler.clone();
        let closure = Closure::new(move |e: web_sys::Event| {
            if let Some(we) = e.dyn_ref::<web_sys::WheelEvent>() {
                let event = ScrollEvent {
                    delta_x: we.delta_x(),
                    delta_y: we.delta_y(),
                    modifiers: Modifiers {
                        shift: we.shift_key(),
                        ctrl: we.ctrl_key(),
                        alt: we.alt_key(),
                        meta: we.meta_key(),
                    },
                };
                batch(|| h(event));
            }
        });
        let _ = target.add_event_listener_with_callback("wheel", closure.as_ref().unchecked_ref());
        closures.push(("wheel".into(), closure));
    }

    // on_key_down
    if let Some(handler) = handlers.on_key_down {
        let handler = Rc::new(handler);
        let h = handler.clone();
        let closure = Closure::new(move |e: web_sys::Event| {
            if let Some(ke) = e.dyn_ref::<web_sys::KeyboardEvent>() {
                let event = keyboard_event_from_dom(ke);
                batch(|| h(event));
            }
        });
        let _ =
            target.add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref());
        closures.push(("keydown".into(), closure));
    }

    // on_key_up
    if let Some(handler) = handlers.on_key_up {
        let handler = Rc::new(handler);
        let h = handler.clone();
        let closure = Closure::new(move |e: web_sys::Event| {
            if let Some(ke) = e.dyn_ref::<web_sys::KeyboardEvent>() {
                let event = keyboard_event_from_dom(ke);
                batch(|| h(event));
            }
        });
        let _ = target.add_event_listener_with_callback("keyup", closure.as_ref().unchecked_ref());
        closures.push(("keyup".into(), closure));
    }

    // on_focus
    if let Some(handler) = handlers.on_focus {
        let handler = Rc::new(handler);
        let h = handler.clone();
        let closure = Closure::new(move |_: web_sys::Event| {
            batch(|| h());
        });
        let _ = target.add_event_listener_with_callback("focus", closure.as_ref().unchecked_ref());
        closures.push(("focus".into(), closure));
    }

    // on_blur
    if let Some(handler) = handlers.on_blur {
        let handler = Rc::new(handler);
        let h = handler.clone();
        let closure = Closure::new(move |_: web_sys::Event| {
            batch(|| h());
        });
        let _ = target.add_event_listener_with_callback("blur", closure.as_ref().unchecked_ref());
        closures.push(("blur".into(), closure));
    }

    EventListeners { closures, target }
}

/// Convert a DOM `KeyboardEvent` to a vitreous `KeyEvent`.
fn keyboard_event_from_dom(ke: &web_sys::KeyboardEvent) -> KeyEvent {
    let key_str = ke.key();
    let code_str = ke.code();
    KeyEvent {
        key: dom_key_to_key(&key_str),
        code: dom_code_to_keycode(&code_str),
        modifiers: modifiers_from_keyboard(ke),
        repeat: ke.repeat(),
        text: {
            let k = &key_str;
            if k.len() == 1 { Some(k.clone()) } else { None }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_enter() {
        assert_eq!(dom_key_to_key("Enter"), Key::Enter);
    }

    #[test]
    fn key_character() {
        assert_eq!(dom_key_to_key("a"), Key::Character("a".into()));
    }

    #[test]
    fn key_space() {
        assert_eq!(dom_key_to_key(" "), Key::Space);
    }

    #[test]
    fn key_unknown() {
        assert_eq!(
            dom_key_to_key("SomeUnknownKey"),
            Key::Other("SomeUnknownKey".into())
        );
    }

    #[test]
    fn keycode_enter() {
        assert_eq!(dom_code_to_keycode("Enter"), KeyCode::Enter);
    }

    #[test]
    fn keycode_key_a() {
        assert_eq!(dom_code_to_keycode("KeyA"), KeyCode::KeyA);
    }

    #[test]
    fn keycode_unknown() {
        assert_eq!(dom_code_to_keycode("UnknownCode"), KeyCode::Unidentified);
    }

    #[test]
    fn dom_button_mapping() {
        assert_eq!(dom_button(0), MouseButton::Left);
        assert_eq!(dom_button(1), MouseButton::Middle);
        assert_eq!(dom_button(2), MouseButton::Right);
        assert_eq!(dom_button(3), MouseButton::Back);
        assert_eq!(dom_button(4), MouseButton::Forward);
        assert_eq!(dom_button(99), MouseButton::Left); // fallback
    }
}
