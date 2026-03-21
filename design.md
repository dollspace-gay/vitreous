# vitreous: Design Document

**Codename:** vitreous
**Version:** 0.1.0 (Design Phase)
**Target:** Desktop (macOS, Windows, Linux) + Web (WASM)
**Language:** Rust (stable toolchain, no nightly required)
**License:** MIT / Apache-2.0 (dual-license, standard Rust convention)

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Problem Statement](#2-problem-statement)
3. [Design Principles](#3-design-principles)
4. [Architecture Overview](#4-architecture-overview)
5. [Module Specifications](#5-module-specifications)
   - 5.1 [Reactivity System (`vitreous_reactive`)](#51-reactivity-system)
   - 5.2 [Widget System (`vitreous_widgets`)](#52-widget-system)
   - 5.3 [Layout Engine (`vitreous_layout`)](#53-layout-engine)
   - 5.4 [Rendering Abstraction (`vitreous_render`)](#54-rendering-abstraction)
   - 5.5 [Styling System (`vitreous_style`)](#55-styling-system)
   - 5.6 [Accessibility (`vitreous_a11y`)](#56-accessibility)
   - 5.7 [Platform Abstraction Layer (`vitreous_platform`)](#57-platform-abstraction-layer)
   - 5.8 [Event System (`vitreous_events`)](#58-event-system)
   - 5.9 [Web Backend (`vitreous_web`)](#59-web-backend)
6. [API Surface](#6-api-surface)
7. [Example App Walkthroughs](#7-example-app-walkthroughs)
8. [Crate Structure & Module Map](#8-crate-structure--module-map)
9. [Agent Implementation Strategy](#9-agent-implementation-strategy)
10. [Test Strategy](#10-test-strategy)
11. [Performance Targets](#11-performance-targets)
12. [Open Questions & Decision Log](#12-open-questions--decision-log)

---

## 1. Executive Summary

vitreous is a Rust-native GUI framework designed to be the missing "just works" option for building cross-platform desktop and web applications. It targets the gap between immediate-mode GUIs (fast to prototype, hard to polish) and heavyweight framework bindings (mature but painful FFI and non-Rustic APIs).

The key bets vitreous makes:

- **Fine-grained signals** as the reactivity primitive, not virtual DOM diffing, not Elm-style message passing, not immediate-mode redraw-everything. Signals map naturally onto Rust's ownership model and enable surgical UI updates.
- **Own the layout, delegate the pixels.** vitreous runs its own flexbox-based layout engine but delegates text shaping, GPU rendering, and accessibility to platform-appropriate backends (wgpu + native text on desktop, DOM on web).
- **Accessibility is the architecture.** The widget tree *is* the accessibility tree. Every widget has a semantic role, every interaction has a keyboard equivalent, every state change is announced. This isn't a feature — it's the skeleton the rest hangs on.
- **Typed styling with theme injection.** No CSS parsing, no stringly-typed properties, no runtime style errors. Styles are Rust types, themes are injected via context, and the compiler catches your mistakes.
- **Hot reload for markup and styles.** Logic changes require recompilation. Layout and style changes don't.

The framework is designed to be implementable by a multi-agent system working from this document. Each module has clear boundaries, defined interfaces, and can be built and tested independently.

---

## 2. Problem Statement

### 2.1 Why Existing Crates Fail

Every Rust GUI crate makes a tradeoff that blocks mainstream adoption. Here is a specific accounting:

**Ownership vs. GUI patterns mismatch.** Traditional GUI frameworks assume a mutable tree of widgets with shared state, bidirectional parent-child references, and closures that capture mutable state freely. Rust's borrow checker forbids all three. Every existing crate is a different workaround:

| Crate | Workaround | Cost |
|-------|-----------|------|
| egui | Immediate mode — no persistent widget tree | No accessibility, no retained state, redraws everything every frame |
| iced | Elm architecture — message passing | Verbose for complex state, message explosion, hard to compose |
| Dioxus | Virtual DOM with `Rc<RefCell<>>` / signals hybrid | Cloning overhead, lifetime confusion, web-first ergonomics that don't translate perfectly to native |
| Slint | Custom DSL (`.slint` files) | Not Rust — separate language, separate tooling, limited expressiveness |
| Druid/Xilem | Lens-based data binding | Steep learning curve, Druid abandoned, Xilem still experimental |
| gtk-rs | FFI bindings to C library | Not Rustic API, GTK looks wrong on macOS/Windows, massive dependency |
| Tauri | Web view — HTML/CSS/JS | You're writing a web app, not a Rust GUI |

**No accessibility.** egui, iced (until very recently), and most custom-rendered frameworks have no screen reader support, no keyboard navigation, and no semantic tree. This makes them unusable for any application that needs to meet WCAG/Section 508 compliance, which includes most enterprise and government software.

**No consensus means no ecosystem.** Because no crate has "won," third-party widget authors don't know what to build for. There's no equivalent of Material UI, no rich date pickers, no data grids, no charting widgets. Every app author builds everything from scratch.

**Compile times kill iteration speed.** A typical Rust GUI app takes 10-30 seconds to recompile after a small change. Web developers get sub-second hot reload. This isn't just annoying — it fundamentally changes how people design UIs. You can't experiment with layout and color when each experiment costs 20 seconds.

### 2.2 What Winning Looks Like

vitreous succeeds if:

1. A developer with React/SwiftUI experience can build a functioning app in under 30 minutes by reading the docs.
2. The app is accessible by default — screen readers work, keyboard navigation works, focus management works — without the developer doing anything special.
3. The app looks acceptable on macOS, Windows, Linux, and web without platform-specific code.
4. Style and layout changes are visible in under 1 second (hot reload).
5. Third-party widget authors can publish widgets to crates.io that "just work" when added as a dependency.
6. The framework can be adopted incrementally — you can embed an vitreous view inside an existing application, or embed native views inside vitreous.

---

## 3. Design Principles

### P1: Signals, Not Messages

State is expressed as signals. A signal is an owned, observable value. Derived state is expressed as computed signals (memos). Side effects are expressed as effects that subscribe to signals. There is no message enum, no reducer, no `Msg` type. State mutations are direct assignments.

```rust
let count = create_signal(0);
let doubled = create_memo(move || count.get() * 2);

create_effect(move || {
    println!("Count is now: {}", count.get());
});

count.set(5); // Effect fires, memo updates, dependent UI rebuilds
```

**Rationale:** Message-passing (Elm architecture) scales poorly — every new interaction requires a new message variant, a match arm, and wiring. Signals scale linearly with state, not with interactions. They also map well onto Rust ownership: the signal owns the value, readers borrow it.

### P2: The Widget Tree Is the Accessibility Tree

Every widget has a `Role` (button, text field, list, etc.), a `Label` (human-readable description), and a `State` (enabled, disabled, checked, expanded, etc.). The accessibility tree is not a parallel structure — it IS the widget tree, projected through a platform-specific accessibility API.

**Rationale:** Bolting on accessibility later is architecturally impossible to do well. It must be the skeleton.

### P3: Typed Styles, Not Strings

There is no CSS. There is no style string. Every visual property is a Rust type with a known set of valid values. The compiler rejects invalid styles. Themes are a struct, not a stylesheet.

### P4: Own the Layout, Delegate the Pixels

vitreous computes layout (position, size) itself using a flexbox-compatible algorithm. It then hands rendering instructions to a backend: wgpu for desktop, DOM manipulation for web. Text shaping is delegated to platform text engines (CoreText, DirectWrite, fontconfig/harfbuzz, browser).

### P5: Hot Reload Is Not Optional

The declarative UI description (widget tree structure + styles) is designed to be reloadable without recompilation. Logic (signal creation, effects, event handlers) requires recompilation. This means the boundary between "what to render" and "what to do" must be architecturally enforced.

### P6: Composition Over Inheritance

There are no base widget classes. There is no `Widget` trait you must implement to create a new widget. A widget is a function that returns a `Node`. Composition is function composition. Custom widgets are just functions that call other functions.

---

## 4. Architecture Overview

```
┌─────────────────────────────────────────────────────────┐
│                    User Application                      │
│   fn app() -> Node { ... }                              │
├─────────────────────────────────────────────────────────┤
│                   vitreous (facade crate)                │
│   Re-exports everything, provides App::run()            │
├──────────┬──────────┬──────────┬────────────────────────┤
│ reactive │ widgets  │  style   │       events           │
│          │          │          │                        │
│ Signal<T>│ text()   │ Theme    │ on_click, on_input,    │
│ Memo<T>  │ button() │ Style    │ on_key, on_focus,      │
│ Effect   │ stack()  │ Modifier │ on_hover, on_scroll    │
│ Context  │ list()   │          │                        │
├──────────┴──────────┴──────────┴────────────────────────┤
│                      layout                              │
│   Flexbox algorithm: Node → LayoutTree (position, size) │
├─────────────────────────────────────────────────────────┤
│                       a11y                               │
│   Accessibility tree projection, focus management,      │
│   keyboard navigation, screen reader announcements      │
├───────────────────────┬─────────────────────────────────┤
│    vitreous_render    │        vitreous_web              │
│    (Desktop: wgpu)    │     (Web: DOM/WASM)             │
├───────────────────────┤─────────────────────────────────┤
│  vitreous_platform    │    wasm-bindgen / web-sys        │
│  (winit, raw-window,  │                                 │
│   accesskit, native   │                                 │
│   text, file dialogs) │                                 │
└───────────────────────┴─────────────────────────────────┘
```

### Data Flow

The runtime loop is:

1. **Event** arrives (user input, timer, async completion).
2. **Signal** mutations occur in response to the event.
3. **Reactive graph** propagates: memos recompute, effects fire.
4. **Dirty widgets** are identified by tracking which signals each widget's build function reads.
5. **Layout** is recomputed only for the dirty subtree and its ancestors up to the nearest layout boundary.
6. **Render** diffs the new layout against the previous frame and issues minimal draw commands.
7. **Accessibility** tree is updated in sync with the widget tree. Platform a11y APIs are notified of changes.

This is a pull-based, lazy system. Nothing recomputes unless something it depends on changed. Nothing re-renders unless its layout or paint properties changed.

---

## 5. Module Specifications

### 5.1 Reactivity System

**Crate:** `vitreous_reactive`
**Dependencies:** None (zero external dependencies — this is a standalone reactive primitives library)

#### Core Types

```rust
/// A readable, writable reactive value.
/// Owns its data. Notifies subscribers on write.
pub struct Signal<T: 'static> {
    // Internal: id into the reactive runtime's slot map
    id: SignalId,
    // PhantomData for T
}

impl<T: Clone + 'static> Signal<T> {
    /// Read the current value. Registers a dependency if called inside
    /// a reactive context (memo, effect, or widget build function).
    pub fn get(&self) -> T;

    /// Read the current value without registering a dependency.
    pub fn get_untracked(&self) -> T;

    /// Set a new value. Triggers downstream updates if value changed.
    pub fn set(&self, value: T);

    /// Modify the value in place using a closure. Triggers downstream updates.
    pub fn update(&self, f: impl FnOnce(&mut T));

    /// Returns a read-only handle to this signal.
    pub fn read_only(&self) -> ReadSignal<T>;
}

/// A derived reactive value. Recomputes lazily when dependencies change.
pub struct Memo<T: 'static> {
    id: MemoId,
}

impl<T: Clone + PartialEq + 'static> Memo<T> {
    /// Read the current derived value. Recomputes if stale.
    pub fn get(&self) -> T;
}

/// A side-effect that re-runs when its dependencies change.
pub struct Effect {
    id: EffectId,
}

/// A way to pass values down the widget tree without prop drilling.
pub struct Context<T: 'static> {
    id: ContextId,
}
```

#### Creation Functions

These are free functions, not methods, matching the established pattern from Leptos/SolidJS:

```rust
/// Create a new signal with an initial value.
pub fn create_signal<T: Clone + 'static>(initial: T) -> Signal<T>;

/// Create a derived computation that caches its result.
/// Only recomputes when a dependency changes.
/// Uses PartialEq to skip propagation if the result didn't change.
pub fn create_memo<T: Clone + PartialEq + 'static>(
    f: impl Fn() -> T + 'static
) -> Memo<T>;

/// Create a side effect that re-runs when dependencies change.
/// Effects are batched — they run after all synchronous signal updates
/// in the current event handler have completed.
pub fn create_effect(f: impl Fn() + 'static);

/// Create a context value accessible by all descendant widgets.
pub fn provide_context<T: Clone + 'static>(value: T);

/// Read a context value provided by an ancestor widget.
/// Panics if no ancestor provided this context type.
pub fn use_context<T: Clone + 'static>() -> T;

/// Read a context value, returning None if not provided.
pub fn try_use_context<T: Clone + 'static>() -> Option<T>;
```

#### Reactive Runtime

The reactive runtime is thread-local. Each thread (or each web worker) gets its own independent reactive graph. This avoids all `Send + Sync` constraints on signal values while remaining compatible with single-threaded WASM.

```rust
/// The reactive runtime. One per thread. Managed automatically.
pub(crate) struct Runtime {
    /// Slot map of all signals
    signals: SlotMap<SignalId, SignalSlot>,
    /// Slot map of all memos
    memos: SlotMap<MemoId, MemoSlot>,
    /// Slot map of all effects
    effects: SlotMap<EffectId, EffectSlot>,
    /// The currently executing reactive context (if any).
    /// Used to auto-track dependencies.
    observer: Option<ObserverId>,
    /// Batch depth counter. Effects only flush when this reaches 0.
    batch_depth: u32,
    /// Queue of effects waiting to run.
    pending_effects: Vec<EffectId>,
}
```

#### Dependency Tracking Algorithm

When a `Signal::get()` is called:
1. Check if there is a current `observer` (a memo or effect being evaluated).
2. If yes, record `(observer_id, signal_id)` as a dependency edge.
3. Return the value.

When a `Signal::set()` is called:
1. Store the new value.
2. Walk all subscribers (memos and effects that depend on this signal).
3. Mark memos as stale (but do NOT recompute yet — lazy).
4. Add dependent effects to the `pending_effects` queue.
5. If `batch_depth == 0`, flush all pending effects.

When a `Memo::get()` is called:
1. If marked stale, recompute by calling the user's closure (this registers fresh dependencies).
2. Compare new value to cached value via `PartialEq`.
3. If unchanged, mark clean and DO NOT propagate staleness to downstream subscribers.
4. If changed, store new value, mark clean, and propagate staleness downstream.

This is a push-pull hybrid: staleness is pushed eagerly (to know what might change), but values are pulled lazily (to avoid computing values no one reads).

#### Batching

Signal updates within a single event handler are batched automatically:

```rust
/// Group multiple signal updates into a single batch.
/// Effects and UI updates are deferred until the batch completes.
pub fn batch(f: impl FnOnce());
```

All event handlers implicitly wrap their body in a `batch()`. This means setting 10 signals in one click handler results in one UI update, not ten.

#### Resource (Async Data Loading)

```rust
/// An async data source that integrates with the reactive system.
/// Refetches when its source signal changes.
pub struct Resource<S, T>
where
    S: Clone + PartialEq + 'static,
    T: Clone + 'static,
{
    source: Memo<S>,
    data: Signal<Option<T>>,
    loading: Signal<bool>,
    error: Signal<Option<String>>,
}

pub fn create_resource<S, T, Fut>(
    source: impl Fn() -> S + 'static,
    fetcher: impl Fn(S) -> Fut + 'static,
) -> Resource<S, T>
where
    S: Clone + PartialEq + 'static,
    T: Clone + 'static,
    Fut: Future<Output = Result<T, Box<dyn std::error::Error>>> + 'static;
```

`Resource` tracks loading, error, and data states reactively. When the `source` signal changes, it cancels the previous fetch and starts a new one. On web, `Fut` is spawned via `wasm_bindgen_futures::spawn_local`. On desktop, via `tokio::spawn` (or a configurable executor).

---

### 5.2 Widget System

**Crate:** `vitreous_widgets`
**Dependencies:** `vitreous_reactive`, `vitreous_style`, `vitreous_a11y`, `vitreous_events`

#### Core Abstraction: The Node

There is no `Widget` trait. A widget is a function that returns a `Node`. A `Node` is an opaque handle representing a position in the UI tree.

```rust
/// A single node in the UI tree. Produced by widget functions.
/// Can represent a primitive (text, box, image) or a container.
pub struct Node {
    pub(crate) kind: NodeKind,
    pub(crate) style: Style,
    pub(crate) a11y: AccessibilityInfo,
    pub(crate) event_handlers: EventHandlers,
    pub(crate) children: Vec<Node>,
    pub(crate) key: Option<Key>,
}

pub(crate) enum NodeKind {
    /// A block container (like a <div>). Has children, participates in layout.
    Container,
    /// A text run. Leaf node.
    Text(TextContent),
    /// An image. Leaf node.
    Image(ImageSource),
    /// A custom paint node. User provides a paint callback.
    Canvas(CanvasPaintFn),
    /// A platform-native embedded view (escape hatch).
    NativeEmbed(NativeViewDescriptor),
    /// A component boundary. Contains a reactive scope.
    Component(ComponentFn),
}
```

#### Primitive Widget Functions

These are the building blocks. All user-facing widgets are composed from these:

```rust
/// A container that stacks children vertically.
pub fn v_stack(children: impl IntoNodes) -> Node;

/// A container that stacks children horizontally.
pub fn h_stack(children: impl IntoNodes) -> Node;

/// A container that layers children on top of each other (z-axis).
pub fn z_stack(children: impl IntoNodes) -> Node;

/// A text label.
pub fn text(content: impl Into<TextContent>) -> Node;

/// A clickable button with a label.
pub fn button(label: impl Into<TextContent>) -> Node;

/// A text input field.
pub fn text_input(value: Signal<String>) -> Node;

/// A checkbox.
pub fn checkbox(checked: Signal<bool>) -> Node;

/// A toggle/switch.
pub fn toggle(on: Signal<bool>) -> Node;

/// A slider for numeric values.
pub fn slider(value: Signal<f64>, min: f64, max: f64) -> Node;

/// A dropdown/select.
pub fn select<T: Clone + PartialEq + ToString + 'static>(
    selected: Signal<T>,
    options: Vec<SelectOption<T>>,
) -> Node;

/// A scrollable container.
pub fn scroll_view(child: impl IntoNode) -> Node;

/// A lazy list that only instantiates visible items.
/// Critical for performance with large datasets.
pub fn virtual_list<T: Clone + 'static>(
    items: impl Fn() -> Vec<T> + 'static,
    key: impl Fn(&T) -> Key + 'static,
    render: impl Fn(T) -> Node + 'static,
) -> Node;

/// An image from a source (URL, path, or bytes).
pub fn image(source: impl Into<ImageSource>) -> Node;

/// A spacer that expands to fill available space.
pub fn spacer() -> Node;

/// A divider line.
pub fn divider() -> Node;

/// A container with no semantics — just a layout box.
pub fn container(child: impl IntoNode) -> Node;

/// Conditionally render one of two nodes.
pub fn show<N: IntoNode>(
    when: impl Fn() -> bool + 'static,
    then: impl Fn() -> N + 'static,
) -> Node;

/// Conditionally render, with an else branch.
pub fn show_else(
    when: impl Fn() -> bool + 'static,
    then: impl Fn() -> Node + 'static,
    otherwise: impl Fn() -> Node + 'static,
) -> Node;

/// Render a list of items reactively. Diffs by key.
pub fn for_each<T: Clone + 'static>(
    items: impl Fn() -> Vec<T> + 'static,
    key: impl Fn(&T) -> Key + 'static,
    render: impl Fn(T) -> Node + 'static,
) -> Node;

/// Provides a context value to all descendants.
pub fn provider<T: Clone + 'static>(value: T, child: impl IntoNode) -> Node;

/// An overlay/modal that renders above the normal flow.
pub fn overlay(child: impl IntoNode) -> Node;

/// A tooltip attached to a child widget.
pub fn tooltip(content: impl Into<TextContent>, child: impl IntoNode) -> Node;
```

#### The Modifier Pattern

Every `Node` supports chained modifiers that set style, accessibility, and event properties. This is the primary ergonomic surface of the framework:

```rust
impl Node {
    // --- Layout Modifiers ---
    pub fn width(self, w: impl Into<Dimension>) -> Self;
    pub fn height(self, h: impl Into<Dimension>) -> Self;
    pub fn min_width(self, w: impl Into<Dimension>) -> Self;
    pub fn max_width(self, w: impl Into<Dimension>) -> Self;
    pub fn min_height(self, h: impl Into<Dimension>) -> Self;
    pub fn max_height(self, h: impl Into<Dimension>) -> Self;
    pub fn padding(self, p: impl Into<Edges>) -> Self;
    pub fn padding_x(self, p: f32) -> Self;
    pub fn padding_y(self, p: f32) -> Self;
    pub fn margin(self, m: impl Into<Edges>) -> Self;
    pub fn flex_grow(self, g: f32) -> Self;
    pub fn flex_shrink(self, s: f32) -> Self;
    pub fn flex_basis(self, b: impl Into<Dimension>) -> Self;
    pub fn align_self(self, a: Align) -> Self;
    pub fn gap(self, g: f32) -> Self;
    pub fn aspect_ratio(self, ratio: f32) -> Self;

    // --- Visual Modifiers ---
    pub fn background(self, color: impl Into<Color>) -> Self;
    pub fn foreground(self, color: impl Into<Color>) -> Self;
    pub fn border(self, width: f32, color: impl Into<Color>) -> Self;
    pub fn border_radius(self, r: impl Into<Corners>) -> Self;
    pub fn shadow(self, s: Shadow) -> Self;
    pub fn opacity(self, o: f32) -> Self;
    pub fn clip(self) -> Self;

    // --- Text Modifiers ---
    pub fn font_size(self, size: f32) -> Self;
    pub fn font_weight(self, weight: FontWeight) -> Self;
    pub fn font_family(self, family: impl Into<FontFamily>) -> Self;
    pub fn text_align(self, align: TextAlign) -> Self;
    pub fn line_height(self, height: f32) -> Self;
    pub fn text_overflow(self, overflow: TextOverflow) -> Self;

    // --- Interaction Modifiers ---
    pub fn on_click(self, handler: impl Fn() + 'static) -> Self;
    pub fn on_double_click(self, handler: impl Fn() + 'static) -> Self;
    pub fn on_mouse_down(self, handler: impl Fn(MouseEvent) + 'static) -> Self;
    pub fn on_mouse_up(self, handler: impl Fn(MouseEvent) + 'static) -> Self;
    pub fn on_mouse_move(self, handler: impl Fn(MouseEvent) + 'static) -> Self;
    pub fn on_mouse_enter(self, handler: impl Fn() + 'static) -> Self;
    pub fn on_mouse_leave(self, handler: impl Fn() + 'static) -> Self;
    pub fn on_scroll(self, handler: impl Fn(ScrollEvent) + 'static) -> Self;
    pub fn on_key_down(self, handler: impl Fn(KeyEvent) + 'static) -> Self;
    pub fn on_key_up(self, handler: impl Fn(KeyEvent) + 'static) -> Self;
    pub fn on_focus(self, handler: impl Fn() + 'static) -> Self;
    pub fn on_blur(self, handler: impl Fn() + 'static) -> Self;
    pub fn on_drag(self, config: DragConfig) -> Self;
    pub fn on_drop(self, handler: impl Fn(DropEvent) + 'static) -> Self;
    pub fn cursor(self, cursor: CursorIcon) -> Self;
    pub fn focusable(self, focusable: bool) -> Self;
    pub fn disabled(self, disabled: impl Fn() -> bool + 'static) -> Self;

    // --- Accessibility Modifiers ---
    pub fn role(self, role: Role) -> Self;
    pub fn label(self, label: impl Into<TextContent>) -> Self;
    pub fn description(self, desc: impl Into<TextContent>) -> Self;
    pub fn live_region(self, politeness: LivePoliteness) -> Self;

    // --- Animation Modifiers ---
    pub fn transition(self, prop: AnimatableProperty, duration: Duration) -> Self;
    pub fn animate(self, animation: Animation) -> Self;

    // --- Composition ---
    pub fn key(self, key: impl Into<Key>) -> Self;
    pub fn apply(self, modifier: impl Fn(Node) -> Node) -> Self;
    pub fn apply_if(self, condition: bool, modifier: impl Fn(Node) -> Node) -> Self;
}
```

#### Component Functions

A component is a function that takes props and returns a `Node`. There is no trait, no struct, no derive macro required. Props are plain structs.

```rust
// Simple component: just a function
fn greeting(name: &str) -> Node {
    text(format!("Hello, {}!", name))
        .font_size(24.0)
        .font_weight(FontWeight::Bold)
}

// Component with reactive state
fn counter() -> Node {
    let count = create_signal(0);

    v_stack((
        text(move || format!("Count: {}", count.get()))
            .font_size(32.0),
        h_stack((
            button("Decrement")
                .on_click(move || count.update(|c| *c -= 1)),
            button("Increment")
                .on_click(move || count.update(|c| *c += 1)),
        )).gap(8.0),
    ))
    .gap(16.0)
    .padding(24.0)
}

// Component with props
#[derive(Clone)]
struct UserCardProps {
    name: String,
    avatar_url: String,
    role: String,
    on_select: Callback,
}

fn user_card(props: UserCardProps) -> Node {
    h_stack((
        image(&props.avatar_url)
            .width(48.0)
            .height(48.0)
            .border_radius(24.0)
            .clip(),
        v_stack((
            text(&props.name)
                .font_weight(FontWeight::SemiBold),
            text(&props.role)
                .font_size(14.0)
                .foreground(theme().text_secondary),
        )).gap(4.0),
    ))
    .gap(12.0)
    .padding(16.0)
    .background(theme().surface)
    .border_radius(8.0)
    .on_click(move || props.on_select.call(()))
    .role(Role::ListItem)
    .label(format!("{}, {}", props.name, props.role))
}
```

#### Callback Type

A type-erased, cloneable callback for passing event handlers through props:

```rust
/// A cloneable, type-erased callback.
/// Wraps Fn() in an Rc so it can be stored in props.
pub struct Callback<A = (), R = ()> {
    inner: Rc<dyn Fn(A) -> R>,
}

impl<A, R> Callback<A, R> {
    pub fn new(f: impl Fn(A) -> R + 'static) -> Self;
    pub fn call(&self, arg: A) -> R;
}

impl<A, R> Clone for Callback<A, R> { /* Rc clone */ }
```

#### IntoNodes Trait

Allows flexible child passing — a single node, a tuple of nodes, a vec, or an iterator:

```rust
pub trait IntoNodes {
    fn into_nodes(self) -> Vec<Node>;
}

// Implemented for:
impl IntoNode for Node { ... }
impl IntoNodes for () { ... } // empty children
impl IntoNodes for Vec<Node> { ... }
impl<A: IntoNode> IntoNodes for (A,) { ... }
impl<A: IntoNode, B: IntoNode> IntoNodes for (A, B) { ... }
// ... up to 16-tuples via macro
impl<I: Iterator<Item = Node>> IntoNodes for I { ... }
```

---

### 5.3 Layout Engine

**Crate:** `vitreous_layout`
**Dependencies:** None (standalone layout algorithm)

vitreous uses a flexbox-compatible layout algorithm, heavily inspired by [Taffy](https://github.com/DioxusLabs/taffy) (the Dioxus layout engine). In fact, vitreous should depend on `taffy` directly rather than reimplementing flexbox from scratch — it's mature, well-tested, and maintained by the Dioxus team.

#### Layout Tree

```rust
/// The input to the layout algorithm.
pub struct LayoutInput {
    pub children: Vec<LayoutInput>,
    pub style: LayoutStyle,
    pub measure: Option<MeasureFn>, // For leaf nodes (text, images) that self-size
}

/// The output of the layout algorithm — resolved positions and sizes.
pub struct LayoutOutput {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub children: Vec<LayoutOutput>,
    pub content_width: f32,  // Scrollable content dimensions
    pub content_height: f32,
}

/// Layout style properties (maps to flexbox).
pub struct LayoutStyle {
    pub display: Display,              // Flex (default), None
    pub flex_direction: FlexDirection,  // Row, Column
    pub flex_wrap: FlexWrap,
    pub justify_content: JustifyContent,
    pub align_items: AlignItems,
    pub align_self: AlignSelf,
    pub flex_grow: f32,
    pub flex_shrink: f32,
    pub flex_basis: Dimension,
    pub width: Dimension,
    pub height: Dimension,
    pub min_width: Dimension,
    pub max_width: Dimension,
    pub min_height: Dimension,
    pub max_height: Dimension,
    pub padding: EdgesLayout,
    pub margin: EdgesLayout,
    pub gap: f32,
    pub aspect_ratio: Option<f32>,
    pub overflow: Overflow,            // Visible, Hidden, Scroll
    pub position: Position,            // Relative, Absolute
    pub inset: EdgesLayout,            // top, right, bottom, left for absolute positioning
}
```

#### Text Measurement

Text nodes require platform-specific measurement (how wide is this string in this font at this size?). The layout engine accepts a `MeasureFn` callback:

```rust
/// Called by the layout engine to measure a leaf node's intrinsic size.
pub type MeasureFn = Box<dyn Fn(MeasureConstraint) -> Size>;

pub struct MeasureConstraint {
    pub max_width: Option<f32>,
    pub max_height: Option<f32>,
}

pub struct Size {
    pub width: f32,
    pub height: f32,
}
```

The platform layer (5.7) provides the actual text measurement implementation. On desktop this calls into platform text APIs; on web it uses the Canvas measureText API or a hidden DOM element.

#### Layout Boundaries

For performance, the layout engine supports **layout boundaries** — nodes where layout changes are guaranteed not to propagate upward. A node is a layout boundary if it has explicit width AND height set (not `auto`). This allows partial re-layout of only the dirty subtree.

---

### 5.4 Rendering Abstraction

**Crate:** `vitreous_render`
**Dependencies:** `vitreous_layout`, `wgpu`, `cosmic-text` (for text shaping on desktop)

This crate is the desktop rendering backend. The web backend (5.9) is separate.

#### Render Command List

The renderer receives a flat list of draw commands, already in screen-space coordinates (layout has been resolved):

```rust
pub enum RenderCommand {
    /// Fill a rectangle with a color.
    FillRect {
        rect: Rect,
        color: Color,
        border_radius: Corners,
        clip: Option<Rect>,
    },
    /// Stroke (outline) a rectangle.
    StrokeRect {
        rect: Rect,
        color: Color,
        width: f32,
        border_radius: Corners,
        clip: Option<Rect>,
    },
    /// Draw a box shadow.
    Shadow {
        rect: Rect,
        shadow: Shadow,
        border_radius: Corners,
    },
    /// Draw a text run at a position.
    Text {
        glyphs: Vec<PositionedGlyph>,
        color: Color,
        clip: Option<Rect>,
    },
    /// Draw an image.
    Image {
        rect: Rect,
        texture_id: TextureId,
        border_radius: Corners,
        opacity: f32,
        clip: Option<Rect>,
    },
    /// Push a clip rect (affects subsequent commands until matching Pop).
    PushClip(Rect),
    /// Pop the clip rect stack.
    PopClip,
    /// Set opacity for subsequent commands.
    PushOpacity(f32),
    PopOpacity,
}
```

#### wgpu Renderer

The desktop renderer uses `wgpu` for GPU-accelerated rendering. It maintains a texture atlas for glyphs and images, and batches draw calls aggressively.

Key implementation details:

- **Rounded rectangles** are drawn using SDF (Signed Distance Field) fragment shaders, which gives crisp edges at any resolution.
- **Text** is shaped by `cosmic-text` (which wraps platform text APIs), rasterized to the glyph atlas, and drawn as textured quads.
- **Images** are uploaded to GPU textures on first use, cached by `ImageSource`.
- **Clipping** is implemented via the stencil buffer, not scissor rects, to support rounded clip regions.
- **Damage tracking:** Only regions that changed since the last frame are re-rendered. The renderer maintains a damage rect list and only submits draw calls for damaged regions.

#### Frame Pipeline

```
Dirty signals detected
        │
        ▼
Rebuild dirty widget subtrees → new Node tree (partial)
        │
        ▼
Merge into existing Node tree (diffed by key)
        │
        ▼
Run layout on dirty subtrees → LayoutOutput
        │
        ▼
Generate RenderCommand list from LayoutOutput + styles
        │
        ▼
Diff against previous frame's command list → damage rects
        │
        ▼
Submit GPU commands for damaged regions only
        │
        ▼
Present frame
```

---

### 5.5 Styling System

**Crate:** `vitreous_style`
**Dependencies:** None

#### Color

```rust
#[derive(Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f32, // 0.0..=1.0
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self;
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self;
    pub fn hex(hex: &str) -> Self; // "#ff0000", "#f00", "#ff000080"
    pub fn hsl(h: f32, s: f32, l: f32) -> Self;
    pub fn hsla(h: f32, s: f32, l: f32, a: f32) -> Self;
    pub fn with_alpha(self, a: f32) -> Self;
    pub fn lighten(self, amount: f32) -> Self;
    pub fn darken(self, amount: f32) -> Self;
    pub fn mix(self, other: Color, t: f32) -> Self;

    // Named colors
    pub const TRANSPARENT: Self;
    pub const WHITE: Self;
    pub const BLACK: Self;
    // ... standard web colors
}
```

#### Theme

The theme is a struct (not a stylesheet) injected via context:

```rust
#[derive(Clone)]
pub struct Theme {
    // --- Color Palette ---
    pub primary: Color,
    pub primary_hover: Color,
    pub primary_active: Color,
    pub secondary: Color,
    pub accent: Color,
    pub destructive: Color,

    pub background: Color,
    pub surface: Color,
    pub surface_hover: Color,
    pub surface_active: Color,
    pub surface_raised: Color,
    pub surface_overlay: Color,

    pub text_primary: Color,
    pub text_secondary: Color,
    pub text_disabled: Color,
    pub text_on_primary: Color,
    pub text_link: Color,

    pub border: Color,
    pub border_focus: Color,
    pub divider: Color,

    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub info: Color,

    // --- Typography ---
    pub font_family: FontFamily,
    pub font_family_mono: FontFamily,
    pub font_size_xs: f32,   // 12
    pub font_size_sm: f32,   // 14
    pub font_size_md: f32,   // 16
    pub font_size_lg: f32,   // 20
    pub font_size_xl: f32,   // 24
    pub font_size_2xl: f32,  // 32
    pub font_size_3xl: f32,  // 40

    // --- Spacing ---
    pub spacing_xs: f32,     // 4
    pub spacing_sm: f32,     // 8
    pub spacing_md: f32,     // 16
    pub spacing_lg: f32,     // 24
    pub spacing_xl: f32,     // 32
    pub spacing_2xl: f32,    // 48

    // --- Borders ---
    pub border_radius_sm: f32,  // 4
    pub border_radius_md: f32,  // 8
    pub border_radius_lg: f32,  // 12
    pub border_radius_xl: f32,  // 16
    pub border_radius_full: f32, // 9999

    pub border_width: f32,      // 1

    // --- Shadows ---
    pub shadow_sm: Shadow,
    pub shadow_md: Shadow,
    pub shadow_lg: Shadow,

    // --- Animation ---
    pub transition_fast: Duration,   // 100ms
    pub transition_normal: Duration, // 200ms
    pub transition_slow: Duration,   // 300ms

    // --- Platform ---
    pub is_dark: bool,
}

impl Theme {
    /// A clean, modern light theme inspired by macOS / Tailwind defaults.
    pub fn light() -> Self;
    /// A matching dark theme.
    pub fn dark() -> Self;
    /// Follows the OS preference.
    pub fn system() -> Self;
}

/// Access the current theme. Must be called inside a widget tree
/// that has a theme provider ancestor (App provides one by default).
pub fn theme() -> Theme;
```

#### Style Struct

The `Style` struct aggregates all visual properties for a node. It is built up by the modifier chain:

```rust
#[derive(Clone, Default)]
pub struct Style {
    // Layout (delegated to LayoutStyle)
    pub layout: LayoutStyle,
    // Visual
    pub background: Option<Color>,
    pub foreground: Option<Color>,
    pub border_color: Option<Color>,
    pub border_width: f32,
    pub border_radius: Corners,
    pub shadow: Option<Shadow>,
    pub opacity: f32,      // default 1.0
    pub clip_content: bool,
    // Text
    pub font_size: Option<f32>,
    pub font_weight: Option<FontWeight>,
    pub font_family: Option<FontFamily>,
    pub text_align: Option<TextAlign>,
    pub line_height: Option<f32>,
    pub text_overflow: Option<TextOverflow>,
    // Cursor
    pub cursor: Option<CursorIcon>,
    // Transitions
    pub transitions: Vec<Transition>,
}
```

#### Style Composition

Styles can be composed and reused:

```rust
/// A reusable style modifier. Apply with `.apply()`.
pub fn card_style(node: Node) -> Node {
    node.padding(16.0)
        .background(theme().surface)
        .border_radius(theme().border_radius_md)
        .shadow(theme().shadow_sm)
}

/// Conditional styling
fn item(selected: bool) -> Node {
    text("Item")
        .padding(8.0)
        .apply_if(selected, |n| {
            n.background(theme().primary)
             .foreground(theme().text_on_primary)
        })
}
```

---

### 5.6 Accessibility

**Crate:** `vitreous_a11y`
**Dependencies:** `accesskit` (platform accessibility bridge)

#### Architecture

AccessKit is used as the platform bridge. It provides a cross-platform accessibility tree API that maps to:
- NSAccessibility on macOS
- UI Automation on Windows
- AT-SPI on Linux
- ARIA attributes on web

vitreous's job is to produce an AccessKit tree that mirrors the widget tree.

#### Accessibility Info

Every node carries accessibility metadata:

```rust
#[derive(Clone, Default)]
pub struct AccessibilityInfo {
    pub role: Option<Role>,
    pub label: Option<String>,
    pub description: Option<String>,
    pub value: Option<String>,
    pub live: Option<LivePoliteness>,
    pub state: AccessibilityState,
    pub actions: Vec<AccessibilityAction>,
}

#[derive(Clone, Default)]
pub struct AccessibilityState {
    pub disabled: bool,
    pub selected: bool,
    pub checked: Option<bool>, // None = not a checkbox, Some(true/false) = checked state
    pub expanded: Option<bool>,
    pub has_popup: bool,
    pub focusable: bool,
    pub focused: bool,
    pub read_only: bool,
    pub required: bool,
    pub invalid: bool,
    pub busy: bool,
    pub modal: bool,
    pub level: Option<u32>, // Heading level
    pub value_min: Option<f64>,
    pub value_max: Option<f64>,
    pub value_now: Option<f64>,
}

#[derive(Clone, Copy, PartialEq)]
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
    None, // Presentational, not exposed to AT
}

#[derive(Clone, Copy)]
pub enum LivePoliteness {
    Off,
    Polite,
    Assertive,
}
```

#### Default Roles

Built-in widgets set their roles automatically:

| Widget | Default Role | Default Label |
|--------|-------------|---------------|
| `button()` | `Button` | Button text content |
| `text_input()` | `TextInput` | Placeholder or associated label |
| `checkbox()` | `Checkbox` | Adjacent text content |
| `toggle()` | `Switch` | Adjacent text content |
| `slider()` | `Slider` | Adjacent text content + current value |
| `select()` | `Menu` | Selected option text |
| `text()` | `Text` | Text content itself |
| `image()` | `Image` | Requires explicit `.label()` (warns in dev if missing) |
| `scroll_view()` | `ScrollView` | None needed |
| `v_stack()` / `h_stack()` | `Group` or `None` | Depends on context |

#### Focus Management

```rust
/// The focus manager tracks which widget has keyboard focus.
pub struct FocusManager {
    /// Ordered list of focusable node IDs (tab order).
    focus_order: Vec<NodeId>,
    /// Currently focused node.
    current_focus: Option<NodeId>,
}

impl FocusManager {
    /// Move focus to the next focusable element (Tab key).
    pub fn focus_next(&mut self);
    /// Move focus to the previous focusable element (Shift+Tab).
    pub fn focus_previous(&mut self);
    /// Set focus to a specific node.
    pub fn focus(&mut self, id: NodeId);
    /// Remove focus from all nodes.
    pub fn blur(&mut self);
    /// Get the currently focused node.
    pub fn focused(&self) -> Option<NodeId>;
}
```

Tab order follows document order (depth-first traversal of the widget tree) by default. A `tab_index` modifier can override this.

#### Keyboard Navigation

All interactive widgets must respond to keyboard events:

| Widget | Key | Action |
|--------|-----|--------|
| `button` | Enter, Space | Activate |
| `checkbox` | Space | Toggle |
| `toggle` | Space | Toggle |
| `select` | Enter, Space | Open menu |
| `select` (open) | Up/Down | Navigate options |
| `select` (open) | Enter | Select option |
| `select` (open) | Escape | Close menu |
| `slider` | Left/Down | Decrease |
| `slider` | Right/Up | Increase |
| `text_input` | Standard text editing keys | Edit text |
| All focusable | Tab | Move focus forward |
| All focusable | Shift+Tab | Move focus backward |

#### Dev Mode Warnings

In debug builds, vitreous produces warnings for:
- `image()` without a `.label()` modifier
- Interactive elements without a label
- Color contrast ratios below WCAG AA thresholds (4.5:1 for normal text, 3:1 for large text)
- Focus traps (focusable regions with no escape path)

---

### 5.7 Platform Abstraction Layer

**Crate:** `vitreous_platform`
**Dependencies:** `winit`, `wgpu`, `raw-window-handle`, `accesskit_winit`, `rfd` (file dialogs), `arboard` (clipboard), `cosmic-text`

This crate wraps all platform-specific functionality behind trait interfaces.

#### Window Management

```rust
pub struct WindowConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub min_width: Option<u32>,
    pub min_height: Option<u32>,
    pub max_width: Option<u32>,
    pub max_height: Option<u32>,
    pub resizable: bool,
    pub decorations: bool,
    pub transparent: bool,
    pub always_on_top: bool,
    pub icon: Option<ImageSource>,
    pub theme: Option<WindowTheme>, // Light, Dark, System
}

pub trait PlatformWindow {
    fn create(config: WindowConfig) -> Self;
    fn request_redraw(&self);
    fn set_title(&self, title: &str);
    fn set_size(&self, width: u32, height: u32);
    fn inner_size(&self) -> (u32, u32);
    fn scale_factor(&self) -> f64;
    fn set_cursor(&self, cursor: CursorIcon);
    fn set_fullscreen(&self, fullscreen: bool);
    fn close(&self);
    fn is_focused(&self) -> bool;
    fn theme(&self) -> WindowTheme;
}
```

#### Text Measurement & Shaping

```rust
pub trait TextEngine {
    /// Measure text with given constraints.
    fn measure(
        &self,
        text: &str,
        font: &FontDescriptor,
        max_width: Option<f32>,
    ) -> TextMeasurement;

    /// Shape text into positioned glyphs.
    fn shape(
        &self,
        text: &str,
        font: &FontDescriptor,
        max_width: Option<f32>,
    ) -> ShapedText;

    /// Rasterize a glyph to a bitmap for the glyph atlas.
    fn rasterize_glyph(
        &self,
        glyph_id: GlyphId,
        font: &FontDescriptor,
        size: f32,
        scale_factor: f64,
    ) -> GlyphBitmap;
}

pub struct FontDescriptor {
    pub family: FontFamily,
    pub size: f32,
    pub weight: FontWeight,
    pub style: FontStyle, // Normal, Italic
}

pub struct TextMeasurement {
    pub width: f32,
    pub height: f32,
    pub lines: Vec<TextLine>,
}

pub struct TextLine {
    pub width: f32,
    pub baseline: f32,
    pub glyph_count: usize,
}
```

On desktop, the implementation uses `cosmic-text`, which provides a unified text layout and shaping engine that works across all desktop platforms using system fonts.

#### Platform Dialogs

```rust
pub trait PlatformDialogs {
    fn open_file(config: FileDialogConfig) -> Option<PathBuf>;
    fn open_files(config: FileDialogConfig) -> Vec<PathBuf>;
    fn save_file(config: FileDialogConfig) -> Option<PathBuf>;
    fn open_directory(config: FileDialogConfig) -> Option<PathBuf>;
    fn message_box(config: MessageBoxConfig) -> MessageBoxResult;
}

pub struct FileDialogConfig {
    pub title: String,
    pub default_path: Option<PathBuf>,
    pub filters: Vec<FileFilter>,
}

pub struct FileFilter {
    pub name: String,
    pub extensions: Vec<String>,
}
```

#### Clipboard

```rust
pub trait PlatformClipboard {
    fn read_text(&self) -> Option<String>;
    fn write_text(&self, text: &str);
    fn read_image(&self) -> Option<ImageData>;
    fn write_image(&self, image: &ImageData);
}
```

#### System Information

```rust
pub trait PlatformInfo {
    fn os(&self) -> Os;               // MacOS, Windows, Linux
    fn theme(&self) -> WindowTheme;   // Light, Dark
    fn locale(&self) -> String;       // "en-US"
    fn scale_factor(&self) -> f64;
    fn accent_color(&self) -> Option<Color>;
}
```

---

### 5.8 Event System

**Crate:** `vitreous_events`
**Dependencies:** None

#### Event Types

```rust
#[derive(Clone, Debug)]
pub struct MouseEvent {
    pub x: f32,               // Position relative to the target node
    pub y: f32,
    pub global_x: f32,        // Position relative to the window
    pub global_y: f32,
    pub button: MouseButton,
    pub modifiers: Modifiers,
}

#[derive(Clone, Debug)]
pub struct KeyEvent {
    pub key: Key,
    pub code: KeyCode,       // Physical key
    pub modifiers: Modifiers,
    pub repeat: bool,
    pub text: Option<String>, // The text the key would produce
}

#[derive(Clone, Debug)]
pub struct ScrollEvent {
    pub delta_x: f32,
    pub delta_y: f32,
    pub modifiers: Modifiers,
}

#[derive(Clone, Debug)]
pub struct DropEvent {
    pub position: Point,
    pub data: DropData,
}

#[derive(Clone, Debug)]
pub enum DropData {
    Files(Vec<PathBuf>),
    Text(String),
    Custom(Box<dyn Any>),
}

#[derive(Clone, Copy, Debug)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub meta: bool, // Cmd on macOS, Win on Windows
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Back,
    Forward,
}
```

#### Event Propagation

Events follow a bubble-up model (like the DOM):

1. Event is dispatched to the deepest node under the cursor (hit testing).
2. That node's handler runs (if any).
3. If the handler doesn't call `event.stop_propagation()`, the event bubbles to the parent.
4. Repeat until root or propagation is stopped.

For keyboard events, the event is dispatched to the currently focused node first, then bubbles up.

#### Hit Testing

Hit testing walks the layout tree in reverse paint order (front-to-back) and returns the first node whose layout rect contains the point. Rounded corners are respected — a click in the rounded corner area of a clipped element doesn't hit that element.

---

### 5.9 Web Backend

**Crate:** `vitreous_web`
**Dependencies:** `wasm-bindgen`, `web-sys`, `js-sys`, `vitreous_layout`, `vitreous_a11y`

The web backend is an alternative to `vitreous_render` + `vitreous_platform`. It does NOT use wgpu on the web. Instead, it maps vitreous's widget tree directly to DOM elements. This gives us:

- Native text rendering and selection
- Native accessibility (ARIA)
- Native scrolling
- Native input handling (IME, autocomplete, etc.)
- CSS transitions/animations for free
- No WASM bundle size bloat from a GPU renderer

#### DOM Mapping Strategy

Each vitreous `Node` maps to a DOM element:

| vitreous Node | DOM Element |
|--------------|-------------|
| `Container` | `<div>` |
| `Text` | `<span>` or `<p>` |
| `Button` | `<button>` |
| `TextInput` | `<input>` or `<textarea>` |
| `Checkbox` | `<input type="checkbox">` |
| `Image` | `<img>` |
| `ScrollView` | `<div style="overflow:auto">` |
| `Select` | Custom dropdown (not native `<select>` for consistent styling) |
| `Slider` | `<input type="range">` (with custom styling overlay) |

#### Style Application

vitreous styles are converted to inline CSS styles. Layout is done via CSS flexbox (not vitreous's layout engine — on web, we let the browser handle layout for better performance and native behavior).

```rust
fn apply_style_to_element(element: &web_sys::HtmlElement, style: &Style) {
    let css = element.style();
    if let Some(bg) = style.background {
        css.set_property("background-color", &bg.to_css_string()).unwrap();
    }
    if let Some(fg) = style.foreground {
        css.set_property("color", &fg.to_css_string()).unwrap();
    }
    // ... all other properties
}
```

#### Reconciliation

The web backend maintains a mapping from `NodeId → web_sys::Element`. When the reactive system triggers a rebuild:

1. New node tree is diffed against the previous tree (keyed by `Key`).
2. New nodes → `document.createElement()` + mount.
3. Removed nodes → `element.remove()`.
4. Changed nodes → update attributes/styles in place.
5. Moved nodes → `parent.insertBefore()` to reorder.

This is intentionally simpler than a full virtual DOM — because vitreous's reactive system already tells us exactly which nodes are dirty, we don't need to diff the entire tree. We only reconcile the subtrees whose signals changed.

#### ARIA Mapping

Accessibility info is mapped directly to ARIA attributes:

```rust
fn apply_a11y_to_element(element: &web_sys::Element, a11y: &AccessibilityInfo) {
    if let Some(role) = &a11y.role {
        element.set_attribute("role", &role.to_aria_string()).unwrap();
    }
    if let Some(label) = &a11y.label {
        element.set_attribute("aria-label", label).unwrap();
    }
    if let Some(desc) = &a11y.description {
        element.set_attribute("aria-describedby", desc).unwrap();
    }
    if a11y.state.disabled {
        element.set_attribute("aria-disabled", "true").unwrap();
    }
    if let Some(checked) = a11y.state.checked {
        element.set_attribute("aria-checked", &checked.to_string()).unwrap();
    }
    // ... etc
}
```

#### Web-Specific APIs

```rust
/// Web-specific extensions available when compiling for WASM.
pub mod web {
    /// Access the browser's fetch API.
    pub async fn fetch(url: &str, options: FetchOptions) -> Result<Response, FetchError>;

    /// Access localStorage.
    pub fn local_storage() -> Storage;

    /// Access the browser's URL/history.
    pub fn location() -> Location;

    /// Navigate to a URL (SPA-style, using History API).
    pub fn navigate(path: &str);

    /// Get the current URL path.
    pub fn current_path() -> String;

    /// Listen for URL changes (popstate).
    pub fn on_navigate(handler: impl Fn(String) + 'static);
}
```

---

## 6. API Surface

This section shows the complete public API of the `vitreous` facade crate — everything a user imports.

### 6.1 App Entry Point

```rust
use vitreous::*;

fn main() {
    App::new()
        .title("My App")
        .size(800, 600)
        .theme(Theme::system())
        .run(app);
}

fn app() -> Node {
    text("Hello, vitreous!")
}
```

`App` builder:

```rust
pub struct App {
    config: AppConfig,
}

impl App {
    pub fn new() -> Self;
    pub fn title(self, title: &str) -> Self;
    pub fn size(self, width: u32, height: u32) -> Self;
    pub fn min_size(self, width: u32, height: u32) -> Self;
    pub fn max_size(self, width: u32, height: u32) -> Self;
    pub fn resizable(self, resizable: bool) -> Self;
    pub fn theme(self, theme: Theme) -> Self;
    pub fn icon(self, icon: ImageSource) -> Self;

    /// Run the app with the given root component.
    /// This function blocks until the window is closed.
    pub fn run(self, root: fn() -> Node);

    /// Run the app on the current thread's event loop.
    /// Use this on web, where there's no separate main thread.
    #[cfg(target_arch = "wasm32")]
    pub fn mount(self, root: fn() -> Node, element_id: &str);
}
```

### 6.2 Dimension Types

```rust
/// A length that can be pixels, percentage, or auto.
#[derive(Clone, Copy)]
pub enum Dimension {
    Px(f32),
    Percent(f32),
    Auto,
}

// Ergonomic conversions
impl From<f32> for Dimension { /* Px */ }
impl From<i32> for Dimension { /* Px */ }
impl From<u32> for Dimension { /* Px */ }

/// Helper for percentage dimensions.
pub fn pct(value: f32) -> Dimension { Dimension::Percent(value) }

/// Edge insets (padding, margin).
#[derive(Clone, Copy)]
pub struct Edges {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl From<f32> for Edges { /* all sides equal */ }
impl From<(f32, f32)> for Edges { /* (vertical, horizontal) */ }
impl From<(f32, f32, f32, f32)> for Edges { /* (top, right, bottom, left) */ }

/// Corner radii.
#[derive(Clone, Copy)]
pub struct Corners {
    pub top_left: f32,
    pub top_right: f32,
    pub bottom_right: f32,
    pub bottom_left: f32,
}

impl From<f32> for Corners { /* all corners equal */ }
```

### 6.3 Enums

```rust
#[derive(Clone, Copy, Default)]
pub enum FlexDirection { #[default] Column, Row }

#[derive(Clone, Copy, Default)]
pub enum JustifyContent {
    #[default] Start, Center, End, SpaceBetween, SpaceAround, SpaceEvenly,
}

#[derive(Clone, Copy, Default)]
pub enum AlignItems {
    #[default] Stretch, Start, Center, End, Baseline,
}

#[derive(Clone, Copy)]
pub enum Align { Auto, Start, Center, End, Stretch, Baseline }

#[derive(Clone, Copy)]
pub enum TextAlign { Left, Center, Right, Justify }

#[derive(Clone, Copy)]
pub enum TextOverflow { Clip, Ellipsis }

#[derive(Clone, Copy)]
pub enum FontWeight {
    Thin,      // 100
    ExtraLight,// 200
    Light,     // 300
    Regular,   // 400
    Medium,    // 500
    SemiBold,  // 600
    Bold,      // 700
    ExtraBold, // 800
    Black,     // 900
}

#[derive(Clone, Copy)]
pub enum CursorIcon {
    Default, Pointer, Text, Move, NotAllowed, Crosshair,
    Grab, Grabbing, ResizeN, ResizeS, ResizeE, ResizeW,
    ResizeNE, ResizeNW, ResizeSE, ResizeSW, ColResize, RowResize,
}

#[derive(Clone, Copy)]
pub enum Overflow { Visible, Hidden, Scroll }

pub struct Shadow {
    pub x: f32,
    pub y: f32,
    pub blur: f32,
    pub spread: f32,
    pub color: Color,
}
```

### 6.4 Animation

```rust
#[derive(Clone)]
pub struct Transition {
    pub property: AnimatableProperty,
    pub duration: Duration,
    pub easing: Easing,
    pub delay: Duration,
}

#[derive(Clone, Copy)]
pub enum AnimatableProperty {
    Opacity,
    BackgroundColor,
    ForegroundColor,
    BorderColor,
    Width,
    Height,
    Padding,
    Margin,
    BorderRadius,
    Shadow,
    Transform,
}

#[derive(Clone, Copy)]
pub enum Easing {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
    CubicBezier(f32, f32, f32, f32),
    Spring { stiffness: f32, damping: f32, mass: f32 },
}

/// A keyframe-based animation.
#[derive(Clone)]
pub struct Animation {
    pub keyframes: Vec<Keyframe>,
    pub duration: Duration,
    pub easing: Easing,
    pub iterations: AnimationIterations,
    pub direction: AnimationDirection,
}

#[derive(Clone, Copy)]
pub enum AnimationIterations {
    Count(u32),
    Infinite,
}

#[derive(Clone, Copy)]
pub enum AnimationDirection {
    Normal,
    Reverse,
    Alternate,
}
```

### 6.5 Router (for Web SPA + Multi-View Desktop)

```rust
/// Declarative routing.
pub fn router(routes: Vec<Route>) -> Node;

pub struct Route {
    pub path: &'static str,
    pub component: fn() -> Node,
}

/// Navigate programmatically.
pub fn navigate(path: &str);

/// Read the current route path reactively.
pub fn use_route() -> Memo<String>;

/// Extract a route parameter reactively.
pub fn use_param(name: &str) -> Memo<Option<String>>;

// Convenience
pub fn route(path: &'static str, component: fn() -> Node) -> Route {
    Route { path, component }
}

// Usage:
fn app() -> Node {
    router(vec![
        route("/", home),
        route("/users", users_list),
        route("/users/:id", user_detail),
        route("/settings", settings),
    ])
}
```

On web, the router uses the History API. On desktop, it manages an internal navigation stack.

---

## 7. Example App Walkthroughs

### 7.1 Hello World (Counter)

The minimal app that exercises signals, events, and text rendering:

```rust
use vitreous::*;

fn main() {
    App::new()
        .title("Counter")
        .size(400, 200)
        .run(counter);
}

fn counter() -> Node {
    let count = create_signal(0);

    v_stack((
        text(move || format!("Count: {}", count.get()))
            .font_size(theme().font_size_2xl)
            .font_weight(FontWeight::Bold)
            .text_align(TextAlign::Center),
        h_stack((
            button("- Decrement")
                .on_click(move || count.update(|c| *c -= 1)),
            button("Reset")
                .on_click(move || count.set(0)),
            button("+ Increment")
                .on_click(move || count.update(|c| *c += 1)),
        ))
        .gap(theme().spacing_sm)
        .justify_content(JustifyContent::Center),
    ))
    .gap(theme().spacing_lg)
    .padding(theme().spacing_xl)
    .align_items(AlignItems::Center)
    .flex_grow(1.0)
    .justify_content(JustifyContent::Center)
}
```

**What this exercises:**
- `create_signal` for state
- Reactive text (closure passed to `text()` re-evaluates when `count` changes)
- Event handlers (`on_click`)
- Layout (`v_stack`, `h_stack`, gap, padding, centering)
- Theme access (`theme()`)
- Default accessibility (buttons are focusable, have labels, respond to Enter/Space)

### 7.2 Todo App

A more realistic app with lists, text input, conditional rendering, and derived state:

```rust
use vitreous::*;

fn main() {
    App::new()
        .title("Todos")
        .size(600, 500)
        .run(todo_app);
}

#[derive(Clone, PartialEq)]
struct Todo {
    id: u64,
    text: String,
    done: bool,
}

#[derive(Clone, Copy, PartialEq)]
enum Filter { All, Active, Completed }

fn todo_app() -> Node {
    let todos = create_signal(Vec::<Todo>::new());
    let input = create_signal(String::new());
    let filter = create_signal(Filter::All);
    let next_id = create_signal(0u64);

    let filtered_todos = create_memo(move || {
        let all = todos.get();
        match filter.get() {
            Filter::All => all,
            Filter::Active => all.into_iter().filter(|t| !t.done).collect(),
            Filter::Completed => all.into_iter().filter(|t| t.done).collect(),
        }
    });

    let remaining = create_memo(move || {
        todos.get().iter().filter(|t| !t.done).count()
    });

    let add_todo = move || {
        let text = input.get().trim().to_string();
        if !text.is_empty() {
            let id = next_id.get();
            next_id.set(id + 1);
            todos.update(|list| list.push(Todo { id, text, done: false }));
            input.set(String::new());
        }
    };

    v_stack((
        // Header
        text("vitreous Todos")
            .font_size(theme().font_size_2xl)
            .font_weight(FontWeight::Bold),

        // Input row
        h_stack((
            text_input(input.clone())
                .flex_grow(1.0)
                .on_key_down(move |e| {
                    if e.key == Key::Enter { add_todo(); }
                }),
            button("Add")
                .on_click(move || add_todo()),
        ))
        .gap(theme().spacing_sm),

        // Filter tabs
        h_stack((
            filter_tab("All", Filter::All, filter),
            filter_tab("Active", Filter::Active, filter),
            filter_tab("Completed", Filter::Completed, filter),
        ))
        .gap(theme().spacing_xs),

        // Todo list
        scroll_view(
            for_each(
                move || filtered_todos.get(),
                |todo| todo.id.into(),
                move |todo| todo_item(todo, todos),
            )
        )
        .flex_grow(1.0),

        // Footer
        text(move || format!("{} items remaining", remaining.get()))
            .font_size(theme().font_size_sm)
            .foreground(theme().text_secondary),
    ))
    .gap(theme().spacing_md)
    .padding(theme().spacing_xl)
    .max_width(600.0)
}

fn filter_tab(label: &str, value: Filter, current: Signal<Filter>) -> Node {
    let is_active = create_memo(move || current.get() == value);
    let label = label.to_string();

    button(label.clone())
        .on_click(move || current.set(value))
        .apply_if(is_active.get(), |n| {
            n.background(theme().primary)
             .foreground(theme().text_on_primary)
        })
        .font_size(theme().font_size_sm)
        .padding((theme().spacing_xs, theme().spacing_sm))
        .border_radius(theme().border_radius_sm)
}

fn todo_item(todo: Todo, todos: Signal<Vec<Todo>>) -> Node {
    let id = todo.id;

    h_stack((
        checkbox(create_signal(todo.done))
            .on_click(move || {
                todos.update(|list| {
                    if let Some(t) = list.iter_mut().find(|t| t.id == id) {
                        t.done = !t.done;
                    }
                });
            }),
        text(&todo.text)
            .apply_if(todo.done, |n| {
                n.foreground(theme().text_disabled)
            })
            .flex_grow(1.0),
        button("×")
            .on_click(move || {
                todos.update(|list| list.retain(|t| t.id != id));
            })
            .foreground(theme().destructive)
            .label("Delete todo"),
    ))
    .gap(theme().spacing_sm)
    .padding(theme().spacing_sm)
    .align_items(AlignItems::Center)
    .role(Role::ListItem)
}
```

**What this exercises:**
- Multiple signals + memos
- `for_each` with keyed list
- Text input with two-way binding
- Conditional styling (`apply_if`)
- Component decomposition (functions)
- Scroll view
- Keyboard handling (Enter to add)
- Accessibility (list items, checkbox state, delete button labels)

### 7.3 Dashboard with Async Data

Shows integration with async data loading, loading states, and more complex layout:

```rust
use vitreous::*;

fn main() {
    App::new()
        .title("Dashboard")
        .size(1200, 800)
        .run(dashboard);
}

#[derive(Clone, PartialEq)]
struct Metrics {
    total_users: u64,
    active_today: u64,
    revenue: f64,
    conversion_rate: f64,
}

fn dashboard() -> Node {
    let metrics = create_resource(
        || (), // No source signal — fetch once
        |_| async { fetch_metrics().await },
    );

    v_stack((
        // Top bar
        h_stack((
            text("Dashboard")
                .font_size(theme().font_size_xl)
                .font_weight(FontWeight::Bold),
            spacer(),
            button("Refresh")
                .on_click(move || metrics.refetch()),
        ))
        .align_items(AlignItems::Center)
        .padding(theme().spacing_lg),

        // Metric cards
        show_else(
            move || metrics.loading.get(),
            || loading_skeleton(),
            move || {
                match metrics.data.get() {
                    Some(m) => metric_cards(m),
                    None => text("Failed to load metrics")
                        .foreground(theme().error),
                }
            },
        ),

        // Main content
        h_stack((
            // Left column
            v_stack((
                section_header("Recent Activity"),
                activity_list(),
            ))
            .flex_grow(2.0),

            // Right column
            v_stack((
                section_header("Quick Actions"),
                quick_actions(),
            ))
            .flex_grow(1.0),
        ))
        .gap(theme().spacing_lg)
        .padding_x(theme().spacing_lg)
        .flex_grow(1.0),
    ))
}

fn metric_cards(metrics: Metrics) -> Node {
    h_stack((
        metric_card("Total Users", format!("{}", metrics.total_users), theme().primary),
        metric_card("Active Today", format!("{}", metrics.active_today), theme().success),
        metric_card("Revenue", format!("${:.2}", metrics.revenue), theme().accent),
        metric_card("Conversion", format!("{:.1}%", metrics.conversion_rate * 100.0), theme().info),
    ))
    .gap(theme().spacing_md)
    .padding_x(theme().spacing_lg)
}

fn metric_card(title: &str, value: String, accent: Color) -> Node {
    v_stack((
        text(title)
            .font_size(theme().font_size_sm)
            .foreground(theme().text_secondary),
        text(value)
            .font_size(theme().font_size_2xl)
            .font_weight(FontWeight::Bold)
            .foreground(accent),
    ))
    .gap(theme().spacing_xs)
    .padding(theme().spacing_lg)
    .background(theme().surface)
    .border_radius(theme().border_radius_md)
    .shadow(theme().shadow_sm)
    .flex_grow(1.0)
}

fn loading_skeleton() -> Node {
    h_stack((
        skeleton_card(),
        skeleton_card(),
        skeleton_card(),
        skeleton_card(),
    ))
    .gap(theme().spacing_md)
    .padding_x(theme().spacing_lg)
}

fn skeleton_card() -> Node {
    container(
        v_stack((
            container(spacer())
                .height(16.0)
                .width(80.0)
                .background(theme().surface_hover)
                .border_radius(4.0),
            container(spacer())
                .height(32.0)
                .width(120.0)
                .background(theme().surface_hover)
                .border_radius(4.0),
        ))
        .gap(theme().spacing_sm)
    )
    .padding(theme().spacing_lg)
    .background(theme().surface)
    .border_radius(theme().border_radius_md)
    .flex_grow(1.0)
    .animate(Animation {
        keyframes: vec![
            Keyframe { at: 0.0, opacity: Some(0.5), ..Default::default() },
            Keyframe { at: 0.5, opacity: Some(1.0), ..Default::default() },
            Keyframe { at: 1.0, opacity: Some(0.5), ..Default::default() },
        ],
        duration: Duration::from_millis(1500),
        easing: Easing::EaseInOut,
        iterations: AnimationIterations::Infinite,
        direction: AnimationDirection::Normal,
    })
}

fn section_header(title: &str) -> Node {
    text(title)
        .font_size(theme().font_size_lg)
        .font_weight(FontWeight::SemiBold)
        .padding_y(theme().spacing_sm)
}

fn activity_list() -> Node {
    // Placeholder — would use create_resource in a real app
    v_stack((
        activity_item("New user signed up", "2 minutes ago"),
        activity_item("Order #1234 completed", "15 minutes ago"),
        activity_item("Support ticket resolved", "1 hour ago"),
        activity_item("New deployment succeeded", "3 hours ago"),
    ))
    .gap(theme().spacing_xs)
}

fn activity_item(title: &str, time: &str) -> Node {
    h_stack((
        container(spacer())
            .width(8.0)
            .height(8.0)
            .background(theme().primary)
            .border_radius(4.0),
        v_stack((
            text(title)
                .font_size(theme().font_size_sm),
            text(time)
                .font_size(theme().font_size_xs)
                .foreground(theme().text_secondary),
        ))
        .gap(2.0)
        .flex_grow(1.0),
    ))
    .gap(theme().spacing_sm)
    .align_items(AlignItems::Center)
    .padding(theme().spacing_sm)
}

fn quick_actions() -> Node {
    v_stack((
        button("Create New User")
            .width(pct(100.0)),
        button("Export Report")
            .width(pct(100.0)),
        button("View Analytics")
            .width(pct(100.0)),
    ))
    .gap(theme().spacing_sm)
}
```

---

## 8. Crate Structure & Module Map

```
vitreous/                          # Workspace root
├── Cargo.toml                     # Workspace manifest
├── crates/
│   ├── vitreous/                  # Facade crate — re-exports everything
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   ├── vitreous_reactive/         # Signal, Memo, Effect, Resource, Context
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── signal.rs
│   │       ├── memo.rs
│   │       ├── effect.rs
│   │       ├── resource.rs
│   │       ├── context.rs
│   │       ├── runtime.rs         # Thread-local reactive runtime
│   │       ├── batch.rs
│   │       ├── graph.rs           # Dependency graph tracking
│   │       └── scope.rs           # Reactive scope (cleanup on drop)
│   ├── vitreous_widgets/          # All widget functions + Node type
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── node.rs            # Node struct + modifiers
│   │       ├── primitives.rs      # text, button, text_input, etc.
│   │       ├── containers.rs      # v_stack, h_stack, scroll_view, etc.
│   │       ├── control_flow.rs    # show, show_else, for_each
│   │       ├── virtual_list.rs
│   │       ├── callback.rs
│   │       ├── into_nodes.rs      # IntoNode / IntoNodes trait impls
│   │       └── router.rs
│   ├── vitreous_style/            # Color, Theme, Style, Dimension types
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── color.rs
│   │       ├── theme.rs
│   │       ├── style.rs
│   │       ├── dimension.rs
│   │       ├── animation.rs
│   │       └── font.rs
│   ├── vitreous_layout/           # Layout engine (wraps taffy)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── tree.rs
│   │       ├── compute.rs
│   │       └── boundary.rs        # Layout boundary optimization
│   ├── vitreous_a11y/             # Accessibility tree, focus management
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── tree.rs            # AccessKit tree generation
│   │       ├── focus.rs           # Focus manager
│   │       ├── keyboard.rs        # Default keyboard navigation
│   │       ├── roles.rs
│   │       └── warnings.rs        # Dev-mode a11y warnings
│   ├── vitreous_events/           # Event types, propagation, hit testing
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── types.rs
│   │       ├── propagation.rs
│   │       └── hit_test.rs
│   ├── vitreous_render/           # Desktop renderer (wgpu)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── commands.rs        # RenderCommand enum
│   │       ├── pipeline.rs        # wgpu render pipeline
│   │       ├── atlas.rs           # Glyph + image texture atlas
│   │       ├── shaders/
│   │       │   ├── rect.wgsl
│   │       │   ├── text.wgsl
│   │       │   ├── image.wgsl
│   │       │   └── shadow.wgsl
│   │       ├── damage.rs          # Damage tracking
│   │       └── diff.rs            # Frame-to-frame command diffing
│   ├── vitreous_platform/         # Platform abstraction (winit, text, dialogs)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── window.rs
│   │       ├── text_engine.rs     # cosmic-text integration
│   │       ├── dialogs.rs         # rfd integration
│   │       ├── clipboard.rs       # arboard integration
│   │       ├── event_loop.rs      # winit event loop adapter
│   │       └── system_info.rs
│   ├── vitreous_web/              # Web/WASM backend
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── dom.rs             # DOM element creation/reconciliation
│   │       ├── styles.rs          # Style → CSS conversion
│   │       ├── aria.rs            # Accessibility → ARIA mapping
│   │       ├── events.rs          # DOM event → vitreous event mapping
│   │       ├── mount.rs           # App mounting
│   │       └── web_apis.rs        # fetch, localStorage, History wrappers
│   └── vitreous_hot_reload/       # Hot reload protocol
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── server.rs          # File watcher + WebSocket server
│           ├── client.rs          # Runtime receiver
│           └── protocol.rs        # Change message types
├── examples/
│   ├── counter/
│   ├── todo/
│   ├── dashboard/
│   └── file_explorer/
└── tools/
    └── vitreous-cli/              # `cargo install vitreous-cli`
        ├── Cargo.toml
        └── src/
            └── main.rs            # `vitreous new`, `vitreous dev` (hot reload)
```

### Dependency Graph Between Crates

```
vitreous_reactive  ──(none)
vitreous_style     ──(none)
vitreous_events    ──(none)
vitreous_layout    ── taffy
vitreous_a11y      ── vitreous_events, accesskit
vitreous_widgets   ── vitreous_reactive, vitreous_style, vitreous_events, vitreous_a11y
vitreous_render    ── vitreous_layout, wgpu, cosmic-text
vitreous_platform  ── vitreous_render, vitreous_a11y, winit, rfd, arboard, cosmic-text, accesskit_winit
vitreous_web       ── vitreous_reactive, vitreous_widgets, vitreous_layout, vitreous_a11y, wasm-bindgen, web-sys
vitreous           ── vitreous_reactive, vitreous_widgets, vitreous_style, vitreous_events, vitreous_a11y,
                      vitreous_platform (desktop), vitreous_web (wasm32)
```

Note: `vitreous_reactive`, `vitreous_style`, and `vitreous_events` have ZERO inter-dependencies. They can be built and tested in complete isolation. This is critical for the agent implementation strategy.

---

## 9. Agent Implementation Strategy

This section defines how a multi-agent system should implement vitreous. Each phase produces testable, working code. No phase depends on a later phase.

### Phase 0: Workspace Scaffolding

**Agent task:** Create the full workspace directory structure, all `Cargo.toml` files with correct dependencies, and empty `lib.rs` files with the correct module declarations and public re-exports. Every crate should compile (to an empty library) after this phase.

**Deliverable:** `cargo check --workspace` passes.

### Phase 1: Foundation Crates (Parallelizable)

These three crates have zero internal dependencies and can be built simultaneously by separate agents:

**Agent 1A — `vitreous_reactive`**
- Implement `Signal<T>`, `Memo<T>`, `Effect`, `Context<T>`
- Implement `Runtime` with dependency tracking, batching, lazy evaluation
- Implement `Resource<S, T>` for async data
- Write unit tests for: signal creation/read/write, memo auto-tracking, effect firing order, batching, diamond dependency problem, glitch-free propagation, memory cleanup on scope drop

**Agent 1B — `vitreous_style`**
- Implement `Color` with all constructors and manipulation methods
- Implement `Theme` with `light()`, `dark()`, and `system()` presets
- Implement all dimension types (`Dimension`, `Edges`, `Corners`)
- Implement all enums (`FontWeight`, `TextAlign`, `Overflow`, etc.)
- Implement `Shadow`, `Transition`, `Animation`, `Easing`
- Write unit tests for: color conversions, color mixing, theme completeness

**Agent 1C — `vitreous_events`**
- Implement all event types (`MouseEvent`, `KeyEvent`, `ScrollEvent`, etc.)
- Implement event propagation logic (bubble-up, stop propagation)
- Implement hit testing algorithm
- Write unit tests for: hit testing with nested rects, propagation stopping, modifier key tracking

### Phase 2: Accessibility + Layout (After Phase 1)

**Agent 2A — `vitreous_a11y`**
- Implement `AccessibilityInfo`, `AccessibilityState`, `Role` enum
- Implement `FocusManager` with tab-order traversal
- Implement AccessKit tree generation from node tree
- Implement default keyboard navigation table
- Implement dev-mode warnings (missing labels, contrast ratios)
- Write unit tests for: focus order computation, role mapping, keyboard navigation simulation

**Agent 2B — `vitreous_layout`**
- Integrate `taffy` crate
- Implement `LayoutInput` → `LayoutOutput` conversion
- Implement `MeasureFn` interface for text/image leaf nodes
- Implement layout boundary optimization
- Write unit tests for: basic flex layout, nested layouts, text wrapping measurement, percentage dimensions, scroll content sizing

### Phase 3: Widget System (After Phase 1 + 2)

**Agent 3 — `vitreous_widgets`**
- Implement `Node` struct with all modifiers
- Implement all primitive widget functions
- Implement `IntoNode` / `IntoNodes` traits with tuple impls
- Implement `Callback<A, R>`
- Implement `show`, `show_else`, `for_each` control flow
- Implement `virtual_list`
- Implement `router`
- Write tests for: node construction, modifier chaining, tuple children, keyed list diffing, router path matching

### Phase 4: Rendering Backends (Parallelizable, After Phase 3)

**Agent 4A — `vitreous_render` (Desktop)**
- Implement `RenderCommand` enum and command list generation
- Implement wgpu render pipeline with shaders (rect, text, image, shadow)
- Implement glyph/image texture atlas
- Implement damage tracking and partial re-render
- Write tests for: command generation from layout tree, damage rect computation

**Agent 4B — `vitreous_web` (WASM)**
- Implement DOM element creation and reconciliation
- Implement Style → CSS property mapping
- Implement AccessibilityInfo → ARIA attribute mapping
- Implement DOM event → vitreous event mapping
- Implement `mount()` function
- Write tests for: DOM diffing, style application, ARIA mapping, event conversion

### Phase 5: Platform Integration (After Phase 4A)

**Agent 5 — `vitreous_platform`**
- Integrate winit for window creation and event loop
- Integrate cosmic-text for text measurement and shaping
- Integrate accesskit_winit for platform accessibility
- Integrate rfd for file dialogs
- Integrate arboard for clipboard
- Wire up the full desktop pipeline: winit events → vitreous events → reactive updates → layout → render → present
- Write integration tests for: window creation, event handling round-trip, text measurement accuracy

### Phase 6: Facade + Examples (After Phase 4 + 5)

**Agent 6A — `vitreous` facade crate**
- Re-export all public APIs
- Implement `App` builder
- Conditional compilation: desktop backend vs web backend

**Agent 6B — Example apps**
- Implement counter, todo, dashboard, file_explorer examples
- Each example must compile and run on both desktop and web targets
- Write screenshot regression tests for each example

### Phase 7: Hot Reload + CLI (After Phase 6)

**Agent 7 — `vitreous_hot_reload` + `vitreous-cli`**
- File watcher (notify crate) watches `.rs` files
- On change: parse the changed file, extract widget tree descriptions, serialize as a diff
- Runtime client receives diff over WebSocket, patches the live tree
- CLI tool: `vitreous new <name>` scaffolds a project, `vitreous dev` starts hot-reload server

### Phase Dependency Graph

```
Phase 0 (scaffold)
    │
    ├── Phase 1A (reactive) ──┐
    ├── Phase 1B (style)    ──┼── Phase 3 (widgets)
    └── Phase 1C (events)   ──┤         │
                              │    ┌────┴────┐
    Phase 2A (a11y)     ──────┘    │         │
    Phase 2B (layout)   ───────────┤         │
                                   │         │
                            Phase 4A     Phase 4B
                            (render)     (web)
                                │
                            Phase 5
                            (platform)
                                │
                    ┌───────────┴───────────┐
                Phase 6A              Phase 6B
                (facade)              (examples)
                    │
                Phase 7
                (hot reload + CLI)
```

**Maximum parallelism:** 3 agents in Phase 1, 2 agents in Phase 2, 2 agents in Phase 4, 2 agents in Phase 6. Total critical path: 7 phases. With agent parallelism, wall-clock time is dominated by Phase 3 (widgets) and Phase 5 (platform integration) which are the most complex sequential steps.

---

## 10. Test Strategy

### 10.1 Unit Tests (Per-Crate)

Every crate has `#[cfg(test)] mod tests` in each source file. Target: 90%+ line coverage on all crates except `vitreous_render` (GPU code is tested differently).

#### `vitreous_reactive` — Critical Test Cases

```rust
#[test]
fn signal_basic_read_write() {
    let s = create_signal(42);
    assert_eq!(s.get(), 42);
    s.set(100);
    assert_eq!(s.get(), 100);
}

#[test]
fn memo_auto_tracks_dependencies() {
    let a = create_signal(1);
    let b = create_signal(2);
    let sum = create_memo(move || a.get() + b.get());
    assert_eq!(sum.get(), 3);
    a.set(10);
    assert_eq!(sum.get(), 12);
}

#[test]
fn memo_skips_propagation_when_value_unchanged() {
    let a = create_signal(1);
    let clamped = create_memo(move || a.get().min(10)); // Clamps to max 10
    let run_count = create_signal(0u32);
    create_effect(move || {
        let _ = clamped.get();
        run_count.update(|c| *c += 1);
    });
    assert_eq!(run_count.get(), 1); // Initial run
    a.set(5);
    assert_eq!(run_count.get(), 2); // clamped changed: 1 → 5
    a.set(100);
    assert_eq!(run_count.get(), 3); // clamped changed: 5 → 10
    a.set(200);
    assert_eq!(run_count.get(), 3); // clamped still 10, no propagation
}

#[test]
fn diamond_dependency_no_glitch() {
    // a → b, a → c, b+c → d
    // When a changes, d should see consistent b and c, not intermediate states.
    let a = create_signal(1);
    let b = create_memo(move || a.get() * 2);
    let c = create_memo(move || a.get() * 3);
    let d = create_memo(move || b.get() + c.get());
    assert_eq!(d.get(), 5); // 2 + 3
    a.set(2);
    assert_eq!(d.get(), 10); // 4 + 6, NOT 4+3 or 2+6
}

#[test]
fn batch_defers_effects() {
    let s = create_signal(0);
    let log = create_signal(Vec::<i32>::new());
    create_effect(move || {
        log.update(|l| l.push(s.get()));
    });
    batch(|| {
        s.set(1);
        s.set(2);
        s.set(3);
    });
    // Effect should have run once with final value, not three times
    let entries = log.get();
    assert_eq!(entries.len(), 2); // Initial + batch
    assert_eq!(*entries.last().unwrap(), 3);
}

#[test]
fn effect_cleanup_on_scope_drop() {
    let s = create_signal(0);
    let ran = create_signal(false);
    {
        let _scope = create_scope(|| {
            create_effect(move || {
                let _ = s.get();
                ran.set(true);
            });
        });
        ran.set(false);
        s.set(1);
        assert!(ran.get()); // Effect ran
    }
    // Scope dropped — effect should be cleaned up
    ran.set(false);
    s.set(2);
    assert!(!ran.get()); // Effect did NOT run
}

#[test]
fn context_propagation() {
    let _scope = create_scope(|| {
        provide_context(42u32);
        let inner_value = create_scope(|| {
            use_context::<u32>()
        });
        assert_eq!(inner_value, 42);
    });
}

#[test]
fn resource_loading_states() {
    // This requires an async test runtime
    let resource = create_resource(
        || "key",
        |_| async { Ok::<_, Box<dyn std::error::Error>>("data".to_string()) },
    );
    assert!(resource.loading.get());
    assert!(resource.data.get().is_none());
    // After async completion (simulated):
    // assert!(!resource.loading.get());
    // assert_eq!(resource.data.get(), Some("data".to_string()));
}
```

#### `vitreous_layout` — Critical Test Cases

```rust
#[test]
fn basic_column_layout() {
    let tree = layout_node(LayoutStyle {
        flex_direction: FlexDirection::Column,
        width: Dimension::Px(200.0),
        ..Default::default()
    }, vec![
        layout_leaf(LayoutStyle { height: Dimension::Px(50.0), ..Default::default() }),
        layout_leaf(LayoutStyle { height: Dimension::Px(30.0), ..Default::default() }),
    ]);
    let output = compute_layout(tree, Size { width: 200.0, height: 400.0 });
    assert_eq!(output.children[0].y, 0.0);
    assert_eq!(output.children[0].height, 50.0);
    assert_eq!(output.children[1].y, 50.0);
    assert_eq!(output.children[1].height, 30.0);
}

#[test]
fn flex_grow_distributes_space() {
    let tree = layout_node(LayoutStyle {
        flex_direction: FlexDirection::Row,
        width: Dimension::Px(300.0),
        height: Dimension::Px(100.0),
        ..Default::default()
    }, vec![
        layout_leaf(LayoutStyle { flex_grow: 1.0, ..Default::default() }),
        layout_leaf(LayoutStyle { flex_grow: 2.0, ..Default::default() }),
    ]);
    let output = compute_layout(tree, Size { width: 300.0, height: 100.0 });
    assert_eq!(output.children[0].width, 100.0);
    assert_eq!(output.children[1].width, 200.0);
}

#[test]
fn text_wrapping_measurement() {
    let tree = layout_node(LayoutStyle {
        width: Dimension::Px(100.0),
        ..Default::default()
    }, vec![
        layout_leaf_with_measure(|constraint| {
            // Simulate text that is 200px wide if unconstrained,
            // but wraps to 2 lines at 100px width
            let width = constraint.max_width.unwrap_or(200.0).min(200.0);
            let lines = (200.0 / width).ceil();
            Size { width, height: lines * 20.0 }
        }),
    ]);
    let output = compute_layout(tree, Size { width: 100.0, height: f32::INFINITY });
    assert_eq!(output.children[0].width, 100.0);
    assert_eq!(output.children[0].height, 40.0); // 2 lines × 20px
}

#[test]
fn percentage_dimensions() {
    let tree = layout_node(LayoutStyle {
        width: Dimension::Px(400.0),
        height: Dimension::Px(300.0),
        ..Default::default()
    }, vec![
        layout_leaf(LayoutStyle {
            width: Dimension::Percent(50.0),
            height: Dimension::Percent(25.0),
            ..Default::default()
        }),
    ]);
    let output = compute_layout(tree, Size { width: 400.0, height: 300.0 });
    assert_eq!(output.children[0].width, 200.0);
    assert_eq!(output.children[0].height, 75.0);
}
```

#### `vitreous_a11y` — Critical Test Cases

```rust
#[test]
fn focus_order_follows_document_order() {
    let tree = build_test_tree(|| {
        v_stack((
            button("First"),  // focusable
            text("Not focusable"),
            button("Second"), // focusable
            text_input(create_signal(String::new())), // focusable
        ))
    });
    let focus_order = compute_focus_order(&tree);
    assert_eq!(focus_order.len(), 3);
    assert_eq!(focus_order[0].label(), "First");
    assert_eq!(focus_order[1].label(), "Second");
    assert_eq!(focus_order[2].role(), Role::TextInput);
}

#[test]
fn button_responds_to_enter_and_space() {
    let clicked = create_signal(false);
    let tree = build_test_tree(|| {
        button("Test").on_click(move || clicked.set(true))
    });
    simulate_key_press(&tree, Key::Enter);
    assert!(clicked.get());

    clicked.set(false);
    simulate_key_press(&tree, Key::Space);
    assert!(clicked.get());
}

#[test]
fn dev_warning_for_image_without_label() {
    let warnings = collect_a11y_warnings(|| {
        image("test.png") // No .label() modifier
    });
    assert!(warnings.iter().any(|w| matches!(w, A11yWarning::MissingLabel { .. })));
}

#[test]
fn accesskit_tree_matches_widget_tree() {
    let tree = build_test_tree(|| {
        v_stack((
            text("Heading").role(Role::Heading),
            button("Click me"),
            checkbox(create_signal(true)).label("Accept terms"),
        ))
    });
    let ak_tree = generate_accesskit_tree(&tree);
    assert_eq!(ak_tree.nodes.len(), 4); // root + 3 children
    assert_eq!(ak_tree.nodes[1].role(), accesskit::Role::Heading);
    assert_eq!(ak_tree.nodes[2].role(), accesskit::Role::Button);
    assert_eq!(ak_tree.nodes[3].role(), accesskit::Role::Checkbox);
    assert_eq!(ak_tree.nodes[3].toggled(), Some(true));
}
```

### 10.2 Integration Tests

Located in `tests/` directories at the workspace root.

**Desktop integration tests** (require a display server — run in CI with xvfb on Linux):
- Create a window, render a counter app, simulate clicks, verify state changes
- Render text in all supported fonts, compare glyph output against reference bitmaps
- Test that AccessKit events are generated correctly when widgets change state

**Web integration tests** (using wasm-pack test):
- Mount a todo app in a headless browser (via wasm-pack + Chrome/Firefox driver)
- Verify DOM structure matches expected output
- Verify ARIA attributes are set correctly
- Simulate click/keyboard events via `web-sys` and verify state changes

### 10.3 Visual Regression Tests

Use a screenshot comparison approach:

1. Each example app is rendered to an offscreen surface (headless wgpu for desktop, headless browser for web).
2. A PNG screenshot is taken.
3. The screenshot is compared pixel-by-pixel against a reference image stored in the repo.
4. Differences beyond a configurable threshold (to account for font rendering differences across platforms) fail the test.

Tool: `insta` crate for snapshot management, extended with image comparison.

### 10.4 Property-Based Tests

Use `proptest` for:

- **Layout engine:** Generate random layout trees with random flex properties. Verify invariants: no child extends beyond parent bounds (unless overflow), all sizes are non-negative, total flex space is fully distributed.
- **Reactive system:** Generate random sequences of signal create/set/memo-create/effect-create operations. Verify invariants: no glitches (a memo never observes an inconsistent set of dependency values), all effects fire exactly once per batch, all cleanup runs on scope drop.
- **Event propagation:** Generate random widget trees with random event handlers. Verify invariants: events bubble correctly, stop_propagation stops bubbling, hit testing returns the correct deepest node.

### 10.5 Benchmark Tests

Use `criterion` for:

- **Signal throughput:** Time to create N signals, set them, and propagate through M memos.
- **Layout computation:** Time to lay out a tree of N nodes.
- **Render command generation:** Time to generate render commands from a layout tree of N nodes.
- **DOM reconciliation (web):** Time to diff and patch a tree of N nodes with M changes.
- **Virtual list scrolling:** Time to scroll through a list of 100,000 items, measuring frame times.

**Targets (preliminary, to be refined):**
- Signal get/set: < 50ns
- Memo recomputation (single dependency): < 100ns
- Layout of 1,000 nodes: < 1ms
- Full frame (1,000 nodes, 10% dirty): < 4ms (targeting 240fps headroom)
- DOM reconciliation (1,000 nodes, 10% changed): < 2ms

---

## 11. Performance Targets

### Desktop

| Metric | Target | Measurement |
|--------|--------|-------------|
| Time to first frame | < 100ms | From `App::run()` to first pixel on screen |
| Frame time (1K nodes, idle) | < 1ms | No dirty signals, just present previous frame |
| Frame time (1K nodes, 10% dirty) | < 4ms | 10% of nodes rebuilt, relaid out, and re-rendered |
| Frame time (10K nodes, 1% dirty) | < 4ms | Large app, small change |
| Signal set + propagation | < 1µs | Single signal, 5-deep memo chain |
| Memory per node | < 512 bytes | Measured after layout, before render |
| Virtual list scroll (100K items) | 60fps | No frame drops during continuous scrolling |
| Startup memory | < 20MB | Empty app, window + GPU context + font atlas |

### Web (WASM)

| Metric | Target | Measurement |
|--------|--------|-------------|
| WASM bundle size (gzipped) | < 200KB | Core framework, no app code |
| Time to interactive | < 500ms | From page load to first interaction possible |
| DOM reconciliation (1K nodes, 10% changed) | < 2ms | Measured in browser performance API |
| Bundle size with full app (gzipped) | < 500KB | Todo app including framework |

### Hot Reload

| Metric | Target |
|--------|--------|
| Style change → visible update | < 200ms |
| Layout change → visible update | < 500ms |
| Logic change → recompile + update | < 5s (depends on project size) |

---

## 12. Open Questions & Decision Log

### Open Questions

**OQ-1: Should `Node` be `Clone`?**
If `Node` is `Clone`, users can store and reuse nodes. This is convenient but means nodes can appear multiple times in the tree, which complicates layout and accessibility. Current decision: `Node` is NOT `Clone`. Widgets are functions — call them again to get a new node.

**OQ-2: Should text content accept `impl Fn() -> String` or `impl Into<String>`?**
Both. `text("static string")` and `text(move || format!("dynamic: {}", signal.get()))` should both work. Implement via an `IntoTextContent` trait that accepts `&str`, `String`, and `impl Fn() -> String + 'static`.

**OQ-3: Should we use `taffy` or write our own layout engine?**
Current decision: Use `taffy`. It's battle-tested in Dioxus, supports flexbox + CSS grid, and is actively maintained. If we hit limitations, fork and extend rather than rewrite.

**OQ-4: How do we handle multi-window on desktop?**
Defer to post-v1. For v1, a single window per `App::run()`. Multi-window adds significant complexity to the event loop, focus management, and accessibility tree.

**OQ-5: Thread model for desktop — single-threaded or multi-threaded?**
Single-threaded event loop + UI thread (like every other GUI framework). Background work is offloaded via `tokio::spawn_blocking` or dedicated threads, communicating back via signals. The reactive runtime is `!Send` — signals cannot cross thread boundaries. This is the same model as web (where everything is single-threaded in the main thread).

**OQ-6: Custom paint / Canvas API?**
Include a `canvas()` node that gives the user a wgpu surface (desktop) or Canvas2D context (web) for custom drawing. This is the escape hatch for charts, custom visualizations, games, etc. Defer detailed API design to post-Phase 4.

**OQ-7: Animation — interpolation in Rust or delegated to platform?**
On web, delegate to CSS transitions/animations. On desktop, vitreous interpolates in Rust using the easing curves defined in `vitreous_style`. The animation system runs before layout each frame, updating animated style properties.

**OQ-8: Should we support CSS Grid in addition to Flexbox?**
Taffy supports CSS Grid. Expose it as a layout option but don't make it the default. Flexbox covers 95% of use cases. Grid is useful for dashboards and complex two-dimensional layouts.

### Decision Log

| ID | Decision | Rationale | Date |
|----|----------|-----------|------|
| D-1 | Fine-grained signals over virtual DOM | Better Rust ownership mapping, surgical updates, proven in Leptos/SolidJS | Day 0 |
| D-2 | DOM backend for web (not wgpu/canvas) | Native text, native accessibility, native scrolling, smaller bundle | Day 0 |
| D-3 | wgpu for desktop rendering | Cross-platform GPU access, maintained by gfx-rs team, used by Firefox | Day 0 |
| D-4 | AccessKit for platform accessibility | Only mature Rust accessibility library, used by egui and others | Day 0 |
| D-5 | cosmic-text for text shaping | Cross-platform, pure Rust (with system font discovery), used by cosmic DE | Day 0 |
| D-6 | winit for windowing | De facto standard in Rust, maintained, cross-platform | Day 0 |
| D-7 | No nightly features required | Maximizes adoption, avoids stability risks | Day 0 |
| D-8 | Dual MIT/Apache-2.0 license | Standard Rust ecosystem convention, maximizes compatibility | Day 0 |
| D-9 | Functions as components, not traits | Lower barrier, better composition, no boilerplate | Day 0 |
| D-10 | Typed styles over CSS | Compiler catches errors, refactor-friendly, no parsing overhead | Day 0 |

---

## Appendix A: Glossary

| Term | Definition |
|------|-----------|
| **Signal** | A reactive primitive that holds a value and notifies subscribers when it changes |
| **Memo** | A derived reactive value that caches its result and only recomputes when dependencies change |
| **Effect** | A side-effect that re-runs when its reactive dependencies change |
| **Node** | A single element in the UI tree, produced by widget functions |
| **Layout boundary** | A node with fixed dimensions that prevents layout recalculation from propagating upward |
| **Damage rect** | A screen region that needs to be re-rendered because something within it changed |
| **Reconciliation** | The process of diffing old and new widget trees to compute minimal DOM/render updates |
| **AccessKit** | A cross-platform Rust library for building accessibility trees |
| **Hit testing** | Determining which widget is under a given screen coordinate |
| **Glitch-free** | A property of reactive systems where derived values never observe inconsistent intermediate states |

## Appendix B: External Dependencies (Pinned Versions)

| Crate | Version | Purpose |
|-------|---------|---------|
| `taffy` | 0.6+ | Flexbox/Grid layout engine |
| `wgpu` | 24+ | GPU rendering |
| `winit` | 0.30+ | Window creation and event loop |
| `cosmic-text` | 0.12+ | Text shaping and measurement |
| `accesskit` | 0.17+ | Accessibility tree |
| `accesskit_winit` | 0.24+ | AccessKit ↔ winit integration |
| `rfd` | 0.15+ | Native file dialogs |
| `arboard` | 3+ | Clipboard access |
| `wasm-bindgen` | 0.2+ | Rust ↔ JS interop |
| `web-sys` | 0.3+ | Web API bindings |
| `js-sys` | 0.3+ | JavaScript runtime bindings |
| `serde` | 1 | Serialization (for hot reload protocol) |
| `notify` | 7+ | File system watching (hot reload) |
| `tokio-tungstenite` | 0.24+ | WebSocket (hot reload) |
| `criterion` | 0.5+ | Benchmarking |
| `proptest` | 1+ | Property-based testing |
| `insta` | 1+ | Snapshot testing |
