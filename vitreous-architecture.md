---
title: "Vitreous GUI Framework — Architecture Specification"
tags: [design-doc]
sources: []
contributors: [unknown]
created: 2026-03-21
updated: 2026-03-21
---


## Design Specification

### Summary

Vitreous is a Rust-native GUI framework for cross-platform desktop (macOS, Windows, Linux) and web (WASM) applications. It combines fine-grained signal-based reactivity, accessibility-first widget architecture, typed styling with theme injection, and platform-delegated rendering (wgpu on desktop, DOM on web). This document is the top-level architectural reference; individual phase design docs in `.design/phase-*.md` are the kickoff targets.

### Requirements

- REQ-1: Workspace organized as a multi-crate Cargo workspace with 10 library crates, 1 facade crate, 1 CLI tool crate, and 4 example crates
- REQ-2: Zero inter-dependencies between foundation crates (`vitreous_reactive`, `vitreous_style`, `vitreous_events`) to enable parallel development
- REQ-3: Fine-grained signal-based reactivity (not virtual DOM, not Elm architecture, not immediate mode)
- REQ-4: Widget tree IS the accessibility tree — every widget has a semantic Role, Label, and State
- REQ-5: Typed Rust styles with compile-time validation — no CSS parsing, no stringly-typed properties
- REQ-6: Platform-delegated rendering: wgpu + cosmic-text on desktop, DOM manipulation on web
- REQ-7: Hot reload for markup and styles without recompilation; logic changes require recompile
- REQ-8: Components are plain functions returning `Node` — no trait implementation required
- REQ-9: All dependencies pinned to latest stable versions verified against crates.io as of 2026-03-21
- REQ-10: Stable Rust toolchain only — no nightly features required
- REQ-11: Dual MIT / Apache-2.0 license

### Acceptance Criteria

- [ ] AC-1: `cargo check --workspace` passes with all crates present (REQ-1)
- [ ] AC-2: `vitreous_reactive`, `vitreous_style`, `vitreous_events` compile with zero workspace-internal dependencies (REQ-2)
- [ ] AC-3: Signal set triggers surgical UI update of only dependent widgets, verified by dirty-tracking counter test (REQ-3)
- [ ] AC-4: Every built-in widget emits an AccessKit node with correct Role, verified by `generate_accesskit_tree` integration test (REQ-4)
- [ ] AC-5: Invalid style values rejected at compile time — `Color::hex("not-a-color")` is a runtime error, but `node.font_weight(42)` does not compile (REQ-5)
- [ ] AC-6: Counter example runs on desktop (wgpu) and web (WASM/DOM) from identical source (REQ-6)
- [ ] AC-7: Style change reflected in running app within 200ms via hot reload (REQ-7)
- [ ] AC-8: Custom widget defined as `fn my_widget() -> Node { ... }` with no trait bounds (REQ-8)
- [ ] AC-9: `cargo build --workspace` succeeds on stable Rust toolchain (REQ-10)

### Architecture

### Crate Map

```
vitreous/                          # Workspace root
├── Cargo.toml                     # Workspace manifest
├── crates/
│   ├── vitreous/                  # Facade crate — re-exports everything
│   ├── vitreous_reactive/         # Signal, Memo, Effect, Resource, Context, Scope
│   ├── vitreous_widgets/          # Node type, widget functions, modifiers, router
│   ├── vitreous_style/            # Color, Theme, Style, Dimension, Animation types
│   ├── vitreous_layout/           # Layout engine (wraps taffy)
│   ├── vitreous_a11y/             # Accessibility tree, focus management
│   ├── vitreous_events/           # Event types, propagation, hit testing
│   ├── vitreous_render/           # Desktop renderer (wgpu)
│   ├── vitreous_platform/         # Platform abstraction (winit, text, dialogs)
│   ├── vitreous_web/              # Web/WASM backend
│   └── vitreous_hot_reload/       # Hot reload protocol
├── examples/
│   ├── counter/
│   ├── todo/
│   ├── dashboard/
│   └── file_explorer/
└── tools/
    └── vitreous-cli/              # `vitreous new`, `vitreous dev`
```

### Crate Dependency Graph

```
vitreous_reactive  ── (none)
vitreous_style     ── (none)
vitreous_events    ── (none)
vitreous_layout    ── taffy
vitreous_a11y      ── vitreous_events, accesskit
vitreous_widgets   ── vitreous_reactive, vitreous_style, vitreous_events, vitreous_a11y
vitreous_render    ── vitreous_layout, wgpu, cosmic-text
vitreous_platform  ── vitreous_render, vitreous_a11y, winit, rfd, arboard, cosmic-text, accesskit_winit
vitreous_web       ── vitreous_reactive, vitreous_widgets, vitreous_layout, vitreous_a11y, wasm-bindgen, web-sys
vitreous           ── all crates (desktop features gate vitreous_platform, wasm32 gates vitreous_web)
```

### Data Flow (Runtime Loop)

1. **Event** arrives (user input, timer, async completion)
2. **Signal** mutations occur in event handler (auto-batched)
3. **Reactive graph** propagates: memos recompute lazily, effects queue
4. **Dirty widgets** identified by signal→widget dependency tracking
5. **Layout** recomputed for dirty subtree + ancestors to nearest layout boundary
6. **Render** diffs layout against previous frame, issues minimal draw commands
7. **Accessibility** tree updated in sync, platform a11y APIs notified

### Implementation Phases

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

Maximum parallelism: 3 agents in Phase 1, 2 in Phase 2, 2 in Phase 4, 2 in Phase 6.

### Out of Scope

- Multi-window support (deferred to post-v1)
- Native platform widget embedding beyond escape-hatch `NativeEmbed` node
- Server-side rendering
- Mobile targets (iOS, Android)
- Built-in internationalization / localization framework
- Built-in form validation library
- Package manager for third-party widgets (crates.io is sufficient)

### design principles

**P1: Signals, Not Messages** — State is signals, derived state is memos, side effects are effects. No message enum, no reducer. Signals map onto Rust ownership: the signal owns the value, readers borrow.

**P2: Widget Tree Is the Accessibility Tree** — Every widget has Role, Label, State. The a11y tree is not parallel — it IS the widget tree projected through platform APIs. Accessibility is the skeleton.

**P3: Typed Styles, Not Strings** — No CSS. Every visual property is a Rust type. Themes are structs, not stylesheets. The compiler rejects invalid styles.

**P4: Own the Layout, Delegate the Pixels** — Vitreous computes layout (flexbox via taffy), delegates rendering to wgpu (desktop) or DOM (web). Text shaping delegated to platform engines.

**P5: Hot Reload Is Not Optional** — Widget tree structure + styles are reloadable without recompilation. Logic requires recompile. The boundary is architecturally enforced.

**P6: Composition Over Inheritance** — No base widget class. No `Widget` trait. A widget is a function returning `Node`. Custom widgets are function composition.

### resolved decisions

| ID | Decision | Rationale |
|----|----------|-----------|
| D-1 | Fine-grained signals over virtual DOM | Better Rust ownership mapping, surgical updates, proven in Leptos/SolidJS |
| D-2 | DOM backend for web (not wgpu/canvas) | Native text, native a11y, native scrolling, smaller bundle |
| D-3 | wgpu for desktop rendering | Cross-platform GPU, maintained by gfx-rs, used by Firefox |
| D-4 | AccessKit for platform accessibility | Only mature Rust a11y library, used by egui |
| D-5 | cosmic-text for text shaping | Cross-platform, pure Rust, used by cosmic DE |
| D-6 | winit for windowing | De facto standard, maintained, cross-platform |
| D-7 | No nightly features required | Maximizes adoption, avoids stability risks |
| D-8 | Dual MIT/Apache-2.0 license | Standard Rust convention, maximizes compatibility |
| D-9 | Functions as components, not traits | Lower barrier, better composition, no boilerplate |
| D-10 | Typed styles over CSS | Compiler catches errors, refactor-friendly, no parsing overhead |
| D-11 | Node is NOT Clone (promoted from OQ-1) | Prevents duplicate tree entries; widgets are functions — call again for new node |
| D-12 | Text content accepts both static and dynamic via IntoTextContent trait (promoted from OQ-2) | `text("static")` and `text(move \|\| format!(...))` both work |
| D-13 | Use taffy for layout, not custom engine (promoted from OQ-3) | Battle-tested in Dioxus, supports flexbox + CSS grid, actively maintained |
| D-14 | Single-window for v1, multi-window deferred (promoted from OQ-4) | Multi-window adds significant complexity to event loop, focus, a11y |
| D-15 | Single-threaded event loop, reactive runtime is !Send (promoted from OQ-5) | Same model as web; background work via tokio::spawn_blocking |
| D-16 | Animation: CSS transitions on web, Rust interpolation on desktop (promoted from OQ-7) | Each platform gets native-feeling animation |
| D-17 | CSS Grid exposed as layout option via taffy, flexbox is default (promoted from OQ-8) | Grid useful for dashboards; flexbox covers 95% of cases |

### external dependencies (verified 2026-03-21)

| Crate | Version | Purpose |
|-------|---------|---------|
| `taffy` | 0.9 | Flexbox/Grid layout engine |
| `wgpu` | 29 | GPU rendering |
| `winit` | 0.30 | Window creation and event loop |
| `cosmic-text` | 0.18 | Text shaping and measurement |
| `accesskit` | 0.24 | Accessibility tree |
| `accesskit_winit` | 0.32 | AccessKit / winit integration |
| `rfd` | 0.17 | Native file dialogs |
| `arboard` | 3.6 | Clipboard access |
| `wasm-bindgen` | 0.2 | Rust / JS interop |
| `web-sys` | 0.3 | Web API bindings |
| `js-sys` | 0.3 | JavaScript runtime bindings |
| `serde` | 1 | Serialization (hot reload protocol) |
| `notify` | 8 | File system watching (hot reload) |
| `tokio-tungstenite` | 0.29 | WebSocket (hot reload) |
| `criterion` | 0.8 | Benchmarking |
| `proptest` | 1.10 | Property-based testing |
| `insta` | 1.46 | Snapshot testing |

Note: `wgpu` 29 requires Rust 1.87+. All other crates work on current stable.

### performance targets

### Desktop

| Metric | Target |
|--------|--------|
| Time to first frame | < 100ms |
| Frame time (1K nodes, idle) | < 1ms |
| Frame time (1K nodes, 10% dirty) | < 4ms |
| Frame time (10K nodes, 1% dirty) | < 4ms |
| Signal set + propagation (5-deep chain) | < 1us |
| Memory per node | < 512 bytes |
| Virtual list scroll (100K items) | 60fps |
| Startup memory | < 20MB |

### Web (WASM)

| Metric | Target |
|--------|--------|
| WASM bundle size (gzipped) | < 200KB |
| Time to interactive | < 500ms |
| DOM reconciliation (1K nodes, 10% changed) | < 2ms |
| Full app bundle (gzipped) | < 500KB |

### Hot Reload

| Metric | Target |
|--------|--------|
| Style change visible | < 200ms |
| Layout change visible | < 500ms |
| Logic change (recompile) | < 5s |

### glossary

| Term | Definition |
|------|-----------|
| **Signal** | Reactive primitive holding a value, notifies subscribers on change |
| **Memo** | Derived reactive value, caches result, recomputes only when dependencies change |
| **Effect** | Side-effect that re-runs when reactive dependencies change |
| **Scope** | Reactive ownership boundary; cleanup runs on drop |
| **Node** | Single element in the UI tree, produced by widget functions |
| **Layout boundary** | Node with fixed dimensions preventing layout recalc propagation upward |
| **Damage rect** | Screen region needing re-render due to content change |
| **Reconciliation** | Diffing old/new widget trees to compute minimal DOM/render updates |
| **AccessKit** | Cross-platform Rust library for building accessibility trees |
| **Hit testing** | Determining which widget is under a given screen coordinate |
| **Glitch-free** | Property where derived values never observe inconsistent intermediate states |

### phase design documents

| Phase | Document | Kickoff target |
|-------|----------|----------------|
| 0 | `.design/phase-0-workspace-scaffold.md` | Workspace structure |
| 1A | `.design/phase-1a-reactive.md` | `vitreous_reactive` |
| 1B | `.design/phase-1b-style.md` | `vitreous_style` |
| 1C | `.design/phase-1c-events.md` | `vitreous_events` |
| 2A | `.design/phase-2a-accessibility.md` | `vitreous_a11y` |
| 2B | `.design/phase-2b-layout.md` | `vitreous_layout` |
| 3 | `.design/phase-3-widgets.md` | `vitreous_widgets` |
| 4A | `.design/phase-4a-render.md` | `vitreous_render` |
| 4B | `.design/phase-4b-web.md` | `vitreous_web` |
| 5 | `.design/phase-5-platform.md` | `vitreous_platform` |
| 6 | `.design/phase-6-facade-examples.md` | `vitreous` facade + examples |
| 7 | `.design/phase-7-hot-reload-cli.md` | `vitreous_hot_reload` + `vitreous-cli` |

