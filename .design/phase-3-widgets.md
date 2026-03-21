# Feature: Implement vitreous_widgets — Node type, widget functions, modifiers, and router

## Summary

Implement the widget system: the `Node` struct and its modifier chain API, all primitive widget functions (text, button, text_input, checkbox, toggle, slider, select, scroll_view, virtual_list, image, spacer, divider, container, overlay, tooltip), control flow helpers (show, show_else, for_each), the `Callback<A,R>` type, `IntoNode`/`IntoNodes` traits with tuple impls, and declarative routing. This is the primary user-facing API surface of vitreous.

## Requirements

- REQ-1: `Node` struct with `NodeKind`, `Style`, `AccessibilityInfo`, `EventHandlers`, `Vec<Node>` children, and optional `Key`
- REQ-2: `NodeKind` enum: Container, Text(TextContent), Image(ImageSource), Canvas(CanvasPaintFn), NativeEmbed(NativeViewDescriptor), Component(ComponentFn)
- REQ-3: All layout modifiers on `Node`: `width`, `height`, `min_width`, `max_width`, `min_height`, `max_height`, `padding`, `padding_x`, `padding_y`, `margin`, `flex_grow`, `flex_shrink`, `flex_basis`, `align_self`, `gap`, `aspect_ratio`
- REQ-4: All visual modifiers: `background`, `foreground`, `border`, `border_radius`, `shadow`, `opacity`, `clip`
- REQ-5: All text modifiers: `font_size`, `font_weight`, `font_family`, `text_align`, `line_height`, `text_overflow`
- REQ-6: All interaction modifiers: `on_click`, `on_double_click`, `on_mouse_down/up/move`, `on_mouse_enter/leave`, `on_scroll`, `on_key_down/up`, `on_focus`, `on_blur`, `on_drag`, `on_drop`, `cursor`, `focusable`, `disabled`
- REQ-7: All accessibility modifiers: `role`, `label`, `description`, `live_region`
- REQ-8: Animation modifiers: `transition`, `animate`
- REQ-9: Composition modifiers: `key`, `apply`, `apply_if`
- REQ-10: Primitive widget functions: `v_stack`, `h_stack`, `z_stack`, `text`, `button`, `text_input`, `checkbox`, `toggle`, `slider`, `select`, `scroll_view`, `virtual_list`, `image`, `spacer`, `divider`, `container`, `overlay`, `tooltip`
- REQ-11: Control flow: `show(when, then)`, `show_else(when, then, otherwise)`, `for_each(items, key, render)`, `provider`
- REQ-12: `Callback<A, R>` — Rc-wrapped, cloneable, type-erased callback for props
- REQ-13: `IntoNode` trait for single-node conversion, `IntoNodes` trait for multiple-node conversion with impls for `Node`, `()`, `Vec<Node>`, tuples up to 16 elements, and iterators
- REQ-14: Declarative `router(routes)` with `Route { path, component }`, `navigate()`, `use_route()`, `use_param()` for both web SPA and desktop navigation
- REQ-15: `text()` accepts both `impl Into<String>` (static) and `impl Fn() -> String + 'static` (reactive) via `IntoTextContent` trait
- REQ-16: Built-in widgets set correct default accessibility roles automatically (button -> Button, text_input -> TextInput, checkbox -> Checkbox, etc.)
- REQ-17: `virtual_list` only instantiates visible items, recycling nodes as the user scrolls

## Acceptance Criteria

- [ ] AC-1: `v_stack((text("hello"), button("click")))` constructs a Node with 2 children (REQ-1, REQ-10)
- [ ] AC-2: Modifier chain `text("hi").font_size(16.0).foreground(Color::RED).on_click(|| {})` compiles and sets all three properties (REQ-3, REQ-4, REQ-5, REQ-6)
- [ ] AC-3: `text("static")` and `text(move || format!("dynamic {}", sig.get()))` both compile via IntoTextContent (REQ-15)
- [ ] AC-4: `IntoNodes` for `(A, B, C)` where A,B,C: IntoNode produces vec of 3 nodes (REQ-13)
- [ ] AC-5: `IntoNodes` for `()` produces empty vec (REQ-13)
- [ ] AC-6: `for_each` with keyed items: adding an item inserts one node, removing an item removes one node, reordering items reorders nodes (not recreate) (REQ-11)
- [ ] AC-7: `show(move || flag.get(), || text("visible"))` produces a text node when flag is true, empty when false (REQ-11)
- [ ] AC-8: `button("Submit")` has `role: Some(Role::Button)` and `label: Some("Submit")` automatically (REQ-16)
- [ ] AC-9: `image("test.png")` has `role: Some(Role::Image)` automatically (REQ-16)
- [ ] AC-10: `Callback::new(|x: i32| x * 2)` can be cloned and called: `cb.call(5) == 10` (REQ-12)
- [ ] AC-11: Router with routes `["/", "/users", "/users/:id"]`: path "/users/42" matches "/users/:id", `use_param("id")` returns "42" (REQ-14)
- [ ] AC-12: `node.apply_if(true, |n| n.background(Color::RED))` applies background; `apply_if(false, ...)` does not (REQ-9)
- [ ] AC-13: `node.key("my-key")` sets the diffing key (REQ-9)
- [ ] AC-14: `virtual_list` with 100,000 items and 500px viewport only instantiates ~25 nodes (at 20px item height) (REQ-17)
- [ ] AC-15: `disabled(move || !valid.get())` makes node unfocusable and sets `a11y.state.disabled` when signal is true (REQ-6, REQ-7)

## Architecture

### File Structure

```
crates/vitreous_widgets/src/
├── lib.rs              # Re-exports all widget functions, Node, Callback, IntoNodes, router
├── node.rs             # Node struct, NodeKind enum, all modifier methods on Node
├── primitives.rs       # text, button, text_input, checkbox, toggle, slider, select, image, spacer, divider
├── containers.rs       # v_stack, h_stack, z_stack, scroll_view, container, overlay, tooltip, provider
├── control_flow.rs     # show, show_else, for_each
├── virtual_list.rs     # virtual_list implementation with viewport-aware instantiation
├── callback.rs         # Callback<A, R> type
├── into_nodes.rs       # IntoNode, IntoNodes traits with tuple macro impls
└── router.rs           # router(), Route, navigate(), use_route(), use_param()
```

### Dependencies

- `vitreous_reactive` — signals for reactive text, disabled state, router state
- `vitreous_style` — Style, Color, Dimension, Theme types
- `vitreous_events` — EventHandlers, event types
- `vitreous_a11y` — AccessibilityInfo, Role, AccessibilityState

### Node Modifier Pattern

All modifiers consume `self` and return `Self` (builder pattern). Modifiers set fields on the node's internal `Style`, `AccessibilityInfo`, or `EventHandlers`:

```rust
impl Node {
    pub fn background(mut self, color: impl Into<Color>) -> Self {
        self.style.background = Some(color.into());
        self
    }
    pub fn on_click(mut self, handler: impl Fn() + 'static) -> Self {
        self.event_handlers.on_click = Some(Box::new(handler));
        self
    }
    // ...
}
```

### IntoNodes Macro

Tuple impls are generated via a declarative macro for 1-tuple through 16-tuple:

```rust
macro_rules! impl_into_nodes_for_tuple {
    ($($T:ident),+) => {
        impl<$($T: IntoNode),+> IntoNodes for ($($T,)+) {
            fn into_nodes(self) -> Vec<Node> {
                let ($($T,)+) = self;
                vec![$($T.into_node(),)+]
            }
        }
    };
}
```

### Router Architecture

The router maintains a `Signal<String>` for the current path. On web, it listens to `popstate` events and pushes via History API. On desktop, it manages an internal navigation stack. Path matching uses simple segment comparison with `:param` capture.

### Keyed Reconciliation (for_each)

`for_each` maintains an internal map from `Key -> (index, Node)`. When the items signal changes:
1. Compute new keys from `key` function
2. Diff against previous keys
3. New keys: call `render`, insert node
4. Removed keys: drop node and its reactive scope
5. Moved keys: reorder existing nodes without rebuild

This is O(n) with a hash map for key lookup.

## Open Questions

### Q1: Canvas paint function signature — RESOLVED
**Decision**: Platform-specific signatures for maximum power.

On desktop:
```rust
pub type CanvasPaintFn = Box<dyn Fn(&wgpu::Device, &wgpu::Queue, &wgpu::TextureView) + 'static>;
```

On web:
```rust
pub type CanvasPaintFn = Box<dyn Fn(&web_sys::CanvasRenderingContext2d) + 'static>;
```

The `canvas()` widget function accepts the appropriate type behind `#[cfg(target_arch)]` gates. Users get direct access to the underlying rendering API with no abstraction overhead. This means canvas code is not cross-platform — users who need both targets write two paint functions. The facade crate can provide a `canvas_cross` helper post-v1 if demand warrants an abstracted `DrawContext` trait.

## Out of Scope

- Built-in rich text editor widget
- Data grid / table widget (third-party crate territory)
- Chart widgets (use Canvas escape hatch)
- Form validation helpers
- Drag-and-drop sortable list (basic drag/drop events exist, but sortable UX is post-v1)
