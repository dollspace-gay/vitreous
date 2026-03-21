---
title: "Implement vitreous_events — event types, propagation, and hit testing"
tags: [design-doc]
sources: []
contributors: [unknown]
created: 2026-03-21
updated: 2026-03-21
---


## Design Specification

### Summary

Implement all input event types (`MouseEvent`, `KeyEvent`, `ScrollEvent`, `DropEvent`), the bubble-up event propagation model, and the hit testing algorithm that determines which widget is under a given screen coordinate. This crate has zero external dependencies and defines the event vocabulary consumed by widgets and platform backends.

### Requirements

- REQ-1: `MouseEvent` with local (x,y) and global (global_x, global_y) positions, `MouseButton` enum, and `Modifiers`
- REQ-2: `KeyEvent` with logical `Key`, physical `KeyCode`, `Modifiers`, repeat flag, and optional text output
- REQ-3: `ScrollEvent` with delta_x, delta_y, and `Modifiers`
- REQ-4: `DropEvent` with position and `DropData` enum (Files, Text, Custom)
- REQ-5: `Modifiers` struct with shift, ctrl, alt, meta (Cmd/Win) booleans
- REQ-6: `MouseButton` enum: Left, Right, Middle, Back, Forward
- REQ-7: Event propagation follows bubble-up model: deepest hit node handles first, bubbles to parent unless `stop_propagation()` called
- REQ-8: Keyboard events dispatch to focused node first, then bubble up
- REQ-9: Hit testing walks layout tree in reverse paint order (front-to-back), returns deepest node whose layout rect contains the point
- REQ-10: Hit testing respects rounded corners — clicks in the transparent corner area of a rounded-clip element do not hit that element
- REQ-11: `EventHandlers` struct holds optional closures for all event types (on_click, on_mouse_down, on_key_down, etc.)
- REQ-12: `Key` and `KeyCode` enums cover standard keyboard keys (letters, numbers, function keys, modifiers, navigation, media)
- REQ-13: `CursorIcon` enum (if not already in `vitreous_style`) or re-exported from style

### Acceptance Criteria

- [ ] AC-1: `MouseEvent` can be constructed with all fields, `button == MouseButton::Left` comparison works (REQ-1, REQ-6)
- [ ] AC-2: `KeyEvent` with `key: Key::Enter`, `modifiers.ctrl: true` is constructible and matchable (REQ-2, REQ-5)
- [ ] AC-3: Hit test on nested rects: outer 0,0,100,100 containing inner 25,25,50,50 — point (30,30) returns inner, point (10,10) returns outer (REQ-9)
- [ ] AC-4: Hit test with overlapping siblings: later sibling (higher z) occludes earlier — point in overlap region returns later sibling (REQ-9)
- [ ] AC-5: Event propagation test: child handler fires, then parent handler fires for same event (REQ-7)
- [ ] AC-6: `stop_propagation()` prevents parent handler from firing (REQ-7)
- [ ] AC-7: Hit test respects border_radius: point in the transparent corner area of a 50px-radius rounded rect does NOT hit that rect (REQ-10)
- [ ] AC-8: `DropData::Files(vec![PathBuf::from("/test")])` roundtrips correctly (REQ-4)
- [ ] AC-9: `EventHandlers::default()` has all handlers as `None` (REQ-11)
- [ ] AC-10: Keyboard event dispatched to focused node ID, not to hit-test point (REQ-8)

### Architecture

### File Structure

```
crates/vitreous_events/src/
├── lib.rs           # Re-exports all public types
├── types.rs         # MouseEvent, KeyEvent, ScrollEvent, DropEvent, DropData, Modifiers, MouseButton, Key, KeyCode
├── propagation.rs   # EventPropagation trait, bubble-up dispatcher, stop_propagation mechanism
└── hit_test.rs      # hit_test(point, layout_tree) -> Option<NodeId>, rounded-corner-aware
```

### Event Propagation Model

```rust
pub struct PropagationContext {
    stopped: bool,
}

impl PropagationContext {
    pub fn stop_propagation(&mut self) { self.stopped = true; }
    pub fn is_stopped(&self) -> bool { self.stopped }
}
```

The dispatcher walks from the hit node upward through ancestors. At each node, if an appropriate handler exists, it runs with a `&mut PropagationContext`. If `stopped` becomes true, traversal halts.

For keyboard events, the walk starts at the focused node (provided by caller) rather than hit-test result.

### Hit Testing Algorithm

```
fn hit_test(point: Point, nodes: &[LayoutRect], border_radii: &[Corners]) -> Option<NodeId> {
    // Walk in reverse paint order (last painted = frontmost)
    for node in nodes.iter().rev() {
        if node.rect.contains(point) && point_inside_rounded_rect(point, node.rect, border_radii[node.id]) {
            return Some(node.id);
        }
    }
    None
}
```

Rounded corner check: for each corner with radius > 0, check if point is within the quarter-circle. If point is in the corner region AND outside the circle, it misses.

### EventHandlers Struct

```rust
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
```

This is `!Send` and `!Sync` because the closures capture signal handles.

### Out of Scope

- Platform-specific event translation (winit events to vitreous events) — that's `vitreous_platform` (Phase 5)
- DOM event mapping — that's `vitreous_web` (Phase 4B)
- Gesture recognition (pinch, swipe) — post-v1
- Focus management — that's `vitreous_a11y` (Phase 2A)
- Drag-and-drop config types (`DragConfig`) — minimal stub, detailed design in Phase 3

