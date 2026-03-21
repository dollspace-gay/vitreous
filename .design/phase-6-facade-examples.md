# Feature: Implement vitreous facade crate and example applications

## Summary

Create the `vitreous` facade crate that re-exports all public APIs behind a single `use vitreous::*` import, implements the `App` builder, and uses conditional compilation to select desktop vs web backends. Then build four example apps (counter, todo, dashboard, file_explorer) that exercise the full framework and serve as integration tests and documentation.

## Requirements

- REQ-1: `vitreous` crate re-exports all public types from `vitreous_reactive`, `vitreous_widgets`, `vitreous_style`, `vitreous_events`, `vitreous_a11y`
- REQ-2: `App` builder with `new()`, `title()`, `size()`, `min_size()`, `max_size()`, `resizable()`, `theme()`, `icon()`, `run(root: fn() -> Node)` methods
- REQ-3: `#[cfg(target_arch = "wasm32")]` gates `vitreous_web` backend, `#[cfg(not(target_arch = "wasm32"))]` gates `vitreous_platform` backend
- REQ-4: `App::mount(root, element_id)` available only on wasm32 target
- REQ-5: `theme()` function that reads Theme from reactive context (bridges `vitreous_style::Theme` with `vitreous_reactive::use_context`)
- REQ-6: Counter example: signal-based counter with increment/decrement/reset, exercises signals, events, text, layout, theme
- REQ-7: Todo example: list management with text input, filtering, keyed list, scroll view, conditional styling
- REQ-8: Dashboard example: async data loading with Resource, loading skeletons, multi-column layout, metric cards
- REQ-9: File explorer example: file system navigation (desktop only), tree view, icon display, breadcrumb navigation
- REQ-10: All examples compile and run on desktop; counter/todo/dashboard also compile for wasm32

## Acceptance Criteria

- [ ] AC-1: `use vitreous::*; fn main() { App::new().title("Test").size(400,300).run(|| text("Hello")); }` compiles on desktop (REQ-1, REQ-2)
- [ ] AC-2: Same code with `App::new().mount(|| text("Hello"), "#app")` compiles on wasm32 (REQ-1, REQ-4)
- [ ] AC-3: `theme()` inside a widget function returns the `Theme` provided by `App` (REQ-5)
- [ ] AC-4: Counter example: clicking increment 3 times shows "Count: 3" (REQ-6)
- [ ] AC-5: Todo example: add 3 items, check 1 as done, filter to "Active" shows 2 items (REQ-7)
- [ ] AC-6: Dashboard example: shows loading skeleton, then metric cards after resource resolves (REQ-8)
- [ ] AC-7: `cargo build --example counter` succeeds on desktop (REQ-10)
- [ ] AC-8: `cargo build --example counter --target wasm32-unknown-unknown` succeeds (REQ-10)
- [ ] AC-9: `cargo build --example todo --target wasm32-unknown-unknown` succeeds (REQ-10)
- [ ] AC-10: Visual regression test: counter screenshot matches reference image within 5% pixel tolerance (REQ-6)

## Architecture

### File Structure

```
crates/vitreous/src/
└── lib.rs              # Re-exports + App builder + theme() bridge

examples/
├── counter/
│   ├── Cargo.toml      # depends on vitreous
│   └── src/main.rs     # Counter app (from design.md section 7.1)
├── todo/
│   ├── Cargo.toml
│   └── src/main.rs     # Todo app (from design.md section 7.2)
├── dashboard/
│   ├── Cargo.toml
│   └── src/main.rs     # Dashboard app (from design.md section 7.3)
└── file_explorer/
    ├── Cargo.toml
    └── src/main.rs     # File explorer (desktop only)
```

### Facade Re-export Structure

```rust
// crates/vitreous/src/lib.rs

pub use vitreous_reactive::*;
pub use vitreous_widgets::*;
pub use vitreous_style::*;
pub use vitreous_events::*;
pub use vitreous_a11y::{Role, LivePoliteness, AccessibilityInfo, AccessibilityState};

#[cfg(not(target_arch = "wasm32"))]
pub use vitreous_platform::*;

#[cfg(target_arch = "wasm32")]
pub use vitreous_web::*;

/// Access the current theme from within a widget tree.
pub fn theme() -> Theme {
    use_context::<Theme>()
}
```

### App Builder

```rust
pub struct App { config: AppConfig }

impl App {
    pub fn new() -> Self { /* defaults */ }
    pub fn title(self, t: &str) -> Self { /* set title */ }
    pub fn size(self, w: u32, h: u32) -> Self { /* set size */ }
    // ... other builder methods

    #[cfg(not(target_arch = "wasm32"))]
    pub fn run(self, root: fn() -> Node) {
        // 1. Create reactive scope
        // 2. provide_context(self.config.theme)
        // 3. Call root() to build initial node tree
        // 4. Start DesktopRuntime event loop
    }

    #[cfg(target_arch = "wasm32")]
    pub fn mount(self, root: fn() -> Node, element_id: &str) {
        // 1. Create reactive scope
        // 2. provide_context(self.config.theme)
        // 3. Call root() to build initial node tree
        // 4. Mount via WebApp
    }
}
```

### Visual Regression Testing

Using `insta` with image comparison:
1. Render each example to an offscreen wgpu surface (headless mode)
2. Capture PNG screenshot
3. Compare against reference images in `examples/<name>/snapshots/`
4. Threshold: 5% pixel difference (accounts for font rendering variation across platforms)

## Open Questions

None — facade pattern and examples are fully specified.

## Out of Scope

- Starter project template generation (that's the CLI in Phase 7)
- Benchmarking harness (criterion benchmarks are separate from examples)
- Documentation website
- Published crate metadata (description, keywords, categories) — added at publish time
