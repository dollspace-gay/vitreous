---
title: "Implement vitreous_web — WASM/DOM backend"
tags: [design-doc]
sources: []
contributors: [unknown]
created: 2026-03-21
updated: 2026-03-21
---


## Design Specification

### Summary

Implement the web backend as an alternative to `vitreous_render` + `vitreous_platform`. Maps vitreous's widget tree directly to DOM elements instead of using wgpu. Handles DOM element creation/reconciliation, Style-to-CSS conversion, AccessibilityInfo-to-ARIA mapping, DOM-to-vitreous event translation, app mounting, and browser API wrappers (fetch, localStorage, History).

### Requirements

- REQ-1: Map each vitreous `NodeKind` to appropriate DOM elements: Container->div, Text->span/p, Button->button, TextInput->input/textarea, Checkbox->input[type=checkbox], Image->img, ScrollView->div[overflow:auto], Slider->input[type=range]
- REQ-2: Convert vitreous `Style` to inline CSS properties on DOM elements
- REQ-3: Convert `AccessibilityInfo` to ARIA attributes: role, aria-label, aria-describedby, aria-disabled, aria-checked, aria-expanded, etc.
- REQ-4: Map DOM events (click, keydown, input, scroll, etc.) to vitreous event types (MouseEvent, KeyEvent, etc.)
- REQ-5: Incremental DOM reconciliation: diff old/new node trees by Key, create/remove/update/reorder DOM elements minimally
- REQ-6: Layout via CSS flexbox (not vitreous_layout) — let the browser handle layout on web for better performance and native behavior
- REQ-7: `mount()` function to attach vitreous app to a DOM element by ID
- REQ-8: Web-specific APIs: `fetch()`, `local_storage()`, `location()`, `navigate()`, `current_path()`, `on_navigate()`
- REQ-9: Select widget uses custom dropdown (not native `<select>`) for consistent cross-browser styling
- REQ-10: WASM bundle size (framework only, gzipped) target: < 200KB

### Acceptance Criteria

- [ ] AC-1: `button("Click me")` produces `<button>Click me</button>` in the DOM (REQ-1)
- [ ] AC-2: `text_input(signal)` produces `<input type="text">` that syncs value with signal on input event (REQ-1, REQ-4)
- [ ] AC-3: `.background(Color::rgb(255, 0, 0))` sets `style="background-color: rgb(255, 0, 0)"` on the element (REQ-2)
- [ ] AC-4: `.role(Role::Button)` sets `role="button"` attribute, `.label("Submit")` sets `aria-label="Submit"` (REQ-3)
- [ ] AC-5: `checkbox(signal)` with `checked: true` sets `aria-checked="true"` on the element (REQ-3)
- [ ] AC-6: DOM click event on a button element triggers the vitreous `on_click` handler (REQ-4)
- [ ] AC-7: DOM keydown event with Enter key produces `KeyEvent { key: Key::Enter, ... }` (REQ-4)
- [ ] AC-8: Adding an item to a `for_each` list inserts one `<div>` in the correct position without recreating siblings (REQ-5)
- [ ] AC-9: Removing a keyed node calls `element.remove()` on the corresponding DOM element (REQ-5)
- [ ] AC-10: `v_stack` produces `<div style="display:flex; flex-direction:column">` (REQ-6)
- [ ] AC-11: `mount()` on element "#app" attaches the rendered tree as children of `document.getElementById("app")` (REQ-7)
- [ ] AC-12: `web::navigate("/users/42")` updates browser URL via History API without page reload (REQ-8)
- [ ] AC-13: Framework-only WASM bundle (no app code) gzipped is < 200KB (REQ-10)

### Architecture

### File Structure

```
crates/vitreous_web/src/
├── lib.rs          # Re-exports, WebApp struct with mount()
├── dom.rs          # DOM element creation, NodeId->Element mapping, reconciliation
├── styles.rs       # Style -> inline CSS property conversion
├── aria.rs         # AccessibilityInfo -> ARIA attribute mapping
├── events.rs       # DOM event listeners -> vitreous event type conversion
├── mount.rs        # mount() function, root element setup, reactive loop integration
└── web_apis.rs     # fetch(), local_storage(), location(), navigate(), on_navigate()
```

### Dependencies

- `vitreous_reactive` — signals for reactive DOM updates
- `vitreous_widgets` — Node, NodeKind for tree structure
- `vitreous_layout` — LayoutStyle types (converted to CSS flex properties)
- `vitreous_a11y` — AccessibilityInfo, Role for ARIA mapping
- `wasm-bindgen` (0.2) — Rust/JS interop
- `web-sys` (0.3) — DOM API bindings
- `js-sys` (0.3) — JS runtime bindings

### Reconciliation Strategy

The reconciler maintains `HashMap<NodeId, web_sys::Element>`. Because vitreous's reactive system identifies exactly which nodes are dirty (via signal tracking), full tree diffing is unnecessary. Only dirty subtrees are reconciled:

1. Signal change triggers widget rebuild for subscribed nodes
2. New subtree is diffed against old subtree by `Key`
3. Matching keys: update attributes/styles in place
4. New keys: `document.createElement()` + mount
5. Missing keys: `element.remove()`
6. Reordered keys: `parent.insertBefore()` to reorder

### Style to CSS Mapping

Direct 1:1 mapping for most properties:
- `background: Color` -> `background-color: rgb(r,g,b)` or `rgba(r,g,b,a)`
- `font_size: f32` -> `font-size: {x}px`
- `padding: Edges` -> `padding: {top}px {right}px {bottom}px {left}px`
- `flex_grow: f32` -> `flex-grow: {x}`
- `border_radius: Corners` -> `border-radius: {tl}px {tr}px {br}px {bl}px`
- `overflow: Scroll` -> `overflow: auto`

Transitions map to CSS transitions for free browser-optimized animation.

### Event Mapping

DOM event listeners are attached during element creation via `web_sys::EventTarget::add_event_listener_with_callback`. Each listener:
1. Extracts relevant fields from the DOM event
2. Constructs the corresponding vitreous event type
3. Wraps in a `batch()` (since this is an event handler entry point)
4. Calls the vitreous handler
5. If `stop_propagation()` was called, calls `event.stop_propagation()` on the DOM event too

### Out of Scope

- Server-side rendering / hydration
- Web Workers integration (signals are !Send, so UI stays on main thread)
- Service worker setup
- PWA manifest generation
- Web component output (vitreous renders into a div, not as custom elements)

