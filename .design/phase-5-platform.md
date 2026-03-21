# Feature: Implement vitreous_platform — desktop platform abstraction layer

## Summary

Wire up all platform-specific desktop functionality: winit window creation and event loop, cosmic-text text measurement and glyph shaping, AccessKit/winit accessibility bridge, rfd file dialogs, arboard clipboard, and the full desktop runtime pipeline (winit events -> vitreous events -> reactive updates -> layout -> render -> present). This is the integration crate that makes desktop apps actually run.

## Requirements

- REQ-1: `WindowConfig` struct and `PlatformWindow` trait wrapping winit window creation with title, size, min/max size, resizable, decorations, transparent, always_on_top, icon, and theme
- REQ-2: `PlatformWindow` methods: create, request_redraw, set_title, set_size, inner_size, scale_factor, set_cursor, set_fullscreen, close, is_focused, theme
- REQ-3: `TextEngine` trait with `measure()`, `shape()`, and `rasterize_glyph()` backed by cosmic-text 0.18
- REQ-4: `FontDescriptor` with family, size, weight, style; `TextMeasurement` with width, height, lines; `ShapedText` with positioned glyphs
- REQ-5: `PlatformDialogs` trait with `open_file`, `open_files`, `save_file`, `open_directory`, `message_box` backed by rfd 0.17
- REQ-6: `PlatformClipboard` trait with `read_text`, `write_text`, `read_image`, `write_image` backed by arboard 3.6
- REQ-7: `PlatformInfo` trait with `os()`, `theme()`, `locale()`, `scale_factor()`, `accent_color()`
- REQ-8: Event loop integration: winit event loop drives the application, translating winit events to vitreous events, running the reactive/layout/render pipeline on each frame
- REQ-9: AccessKit integration via accesskit_winit 0.32: accessibility tree updates piped to platform AT on every frame
- REQ-10: DPI-aware rendering: scale factor applied to layout and rendering, text rasterized at display scale

## Acceptance Criteria

- [ ] AC-1: `PlatformWindow::create(WindowConfig { title: "Test", width: 800, height: 600, .. })` opens a visible window on Linux/macOS/Windows (REQ-1, REQ-2)
- [ ] AC-2: `TextEngine::measure("Hello, world!", font_desc, Some(200.0))` returns non-zero width/height (REQ-3)
- [ ] AC-3: `TextEngine::shape("Hi", font_desc, None)` returns a `ShapedText` with glyph positions (REQ-3, REQ-4)
- [ ] AC-4: `TextEngine::rasterize_glyph(glyph_id, font_desc, 16.0, 1.0)` returns a non-empty bitmap (REQ-3)
- [ ] AC-5: `PlatformDialogs::open_file` shows a native file picker dialog (manual test, not automated) (REQ-5)
- [ ] AC-6: `PlatformClipboard::write_text("test")` followed by `read_text()` returns `Some("test")` (REQ-6)
- [ ] AC-7: Winit mouse click event translates to vitreous `MouseEvent` with correct position and button (REQ-8)
- [ ] AC-8: Winit keyboard event translates to vitreous `KeyEvent` with correct key and modifiers (REQ-8)
- [ ] AC-9: AccessKit tree update is sent to the platform on every frame where the widget tree changed (REQ-9)
- [ ] AC-10: On a 2x DPI display, text is rasterized at 2x and layout positions are in logical pixels (REQ-10)
- [ ] AC-11: Counter example app: click increment button -> text updates -> frame renders with new value, all within 16ms (REQ-8)
- [ ] AC-12: Window resize triggers layout recomputation at new dimensions (REQ-8)

## Architecture

### File Structure

```
crates/vitreous_platform/src/
├── lib.rs            # Re-exports, DesktopRuntime::run() entry point
├── window.rs         # WindowConfig, PlatformWindow wrapping winit::Window
├── text_engine.rs    # TextEngine implementation via cosmic-text
├── dialogs.rs        # PlatformDialogs via rfd
├── clipboard.rs      # PlatformClipboard via arboard
├── event_loop.rs     # Winit event loop adapter: winit events -> vitreous events -> reactive -> layout -> render
└── system_info.rs    # PlatformInfo (OS detection, theme detection, locale)
```

### Dependencies

- `vitreous_render` — Renderer for GPU frame submission
- `vitreous_a11y` — AccessibilityInfo for AccessKit tree generation
- `winit` (0.30) — window creation, event loop
- `rfd` (0.17) — native file dialogs
- `arboard` (3.6) — clipboard
- `cosmic-text` (0.18) — text measurement, shaping, glyph rasterization
- `accesskit_winit` (0.32) — AccessKit integration with winit

### Desktop Runtime Pipeline

```rust
pub struct DesktopRuntime {
    window: winit::window::Window,
    renderer: Renderer,
    text_engine: CosmicTextEngine,
    a11y_adapter: accesskit_winit::Adapter,
    reactive_runtime: ReactiveRuntime, // thread-local
    root_scope: Scope,
    root_node: Node,
    layout_tree: LayoutOutput,
    prev_layout: LayoutOutput,
}
```

The event loop:
```
winit::EventLoop::run(|event, target| {
    match event {
        WindowEvent::RedrawRequested => {
            // 1. Rebuild dirty widget subtrees
            // 2. Compute layout (incremental)
            // 3. Generate render commands
            // 4. Submit to GPU
            // 5. Update AccessKit tree
            // 6. Present frame
        },
        WindowEvent::MouseInput { .. } => {
            // 1. Hit test to find target node
            // 2. Wrap in batch()
            // 3. Translate to vitreous MouseEvent
            // 4. Dispatch through event propagation
            // 5. request_redraw() if any signals changed
        },
        // ... other events
    }
});
```

### cosmic-text Integration

cosmic-text provides a `FontSystem` (discovers system fonts) and `SwashCache` (rasterizes glyphs). The `TextEngine` wraps these:

- `measure()`: Create a `Buffer`, set text + font + max_width, run layout, read line metrics
- `shape()`: Same as measure but also extract glyph positions
- `rasterize_glyph()`: Use `SwashCache::get_image()` to get a bitmap

The `FontSystem` is created once at startup and reused for the lifetime of the app.

## Open Questions

None — all platform integrations follow documented patterns from their respective crates.

## Out of Scope

- Multi-window management (single window for v1)
- Tray icon / system tray integration
- Native menu bar
- Touch input / gesture recognition
- Gamepad input
- Custom window chrome (decorations: false + manual title bar)
