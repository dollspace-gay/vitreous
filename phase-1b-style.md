---
title: "Implement vitreous_style — typed styling, theming, and animation primitives"
tags: [design-doc]
sources: []
contributors: [unknown]
created: 2026-03-21
updated: 2026-03-21
---


## Design Specification

### Summary

Implement all visual type definitions: `Color` with full color space support, `Theme` with light/dark/system presets, `Style` struct aggregating all visual properties, dimension types (`Dimension`, `Edges`, `Corners`), typography enums, animation primitives (`Transition`, `Animation`, `Easing`), and `Shadow`. This crate has zero external dependencies and defines the type vocabulary used by widgets, rendering, and the web backend.

### Requirements

- REQ-1: `Color` struct with `rgb()`, `rgba()`, `hex()`, `hsl()`, `hsla()` constructors and `with_alpha()`, `lighten()`, `darken()`, `mix()` manipulation methods
- REQ-2: Named color constants (`Color::WHITE`, `Color::BLACK`, `Color::TRANSPARENT`, standard web colors)
- REQ-3: `Theme` struct with complete semantic color palette (primary, secondary, surface, text, border, status colors), typography scale (xs through 3xl), spacing scale, border radii, shadows, and animation durations
- REQ-4: `Theme::light()`, `Theme::dark()`, `Theme::system()` preset constructors
- REQ-5: `Style` struct aggregating layout, visual, text, cursor, and transition properties with `Default` implementation
- REQ-6: `Dimension` enum (`Px`, `Percent`, `Auto`) with ergonomic `From<f32>`, `From<i32>`, `From<u32>` conversions
- REQ-7: `Edges` and `Corners` types with `From<f32>`, `From<(f32, f32)>`, `From<(f32, f32, f32, f32)>` conversions
- REQ-8: Typography enums: `FontWeight` (Thin through Black), `FontFamily`, `FontStyle`, `TextAlign`, `TextOverflow`
- REQ-9: `Shadow` struct, `Transition` struct, `Animation` struct with `Keyframe`, `Easing` enum (Linear, EaseIn/Out/InOut, CubicBezier, Spring), `AnimationIterations`, `AnimationDirection`
- REQ-10: `AnimatableProperty` enum listing all properties that can be animated
- REQ-11: `CursorIcon` enum with all standard cursor types
- REQ-12: `Overflow` enum (`Visible`, `Hidden`, `Scroll`)
- REQ-13: `pct()` helper function for percentage dimensions

### Acceptance Criteria

- [ ] AC-1: `Color::rgb(255, 0, 0)` produces red with r=1.0, g=0.0, b=0.0, a=1.0 (REQ-1)
- [ ] AC-2: `Color::hex("#ff0000")`, `Color::hex("#f00")`, `Color::hex("#ff000080")` all parse correctly (REQ-1)
- [ ] AC-3: `Color::hsl(0.0, 1.0, 0.5)` equals `Color::rgb(255, 0, 0)` (REQ-1)
- [ ] AC-4: `color.lighten(0.2)` produces a lighter color, `color.darken(0.2)` produces a darker color (REQ-1)
- [ ] AC-5: `Color::mix(white, black, 0.5)` produces mid-gray (REQ-1)
- [ ] AC-6: `Theme::light()` and `Theme::dark()` both have all fields populated (no `Option::None` in required fields), and `is_dark` is false/true respectively (REQ-3, REQ-4)
- [ ] AC-7: `Theme::light()` contrast ratios between text_primary/background and text_secondary/background meet WCAG AA (4.5:1 for normal text) (REQ-3)
- [ ] AC-8: `Dimension::from(16.0f32)` equals `Dimension::Px(16.0)`, `pct(50.0)` equals `Dimension::Percent(50.0)` (REQ-6, REQ-13)
- [ ] AC-9: `Edges::from(8.0)` sets all four sides to 8.0, `Edges::from((8.0, 16.0))` sets vertical=8, horizontal=16 (REQ-7)
- [ ] AC-10: `Style::default()` has opacity=1.0, all `Option` fields are `None`, clip_content=false (REQ-5)
- [ ] AC-11: `Easing::CubicBezier(0.25, 0.1, 0.25, 1.0)` can be constructed (REQ-9)
- [ ] AC-12: `Easing::Spring { stiffness: 100.0, damping: 10.0, mass: 1.0 }` can be constructed (REQ-9)
- [ ] AC-13: All `FontWeight` variants map to correct numeric values (Thin=100, Regular=400, Bold=700, Black=900) (REQ-8)

### Architecture

### File Structure

```
crates/vitreous_style/src/
├── lib.rs          # Re-exports everything public
├── color.rs        # Color struct, constructors, manipulation, named constants
├── theme.rs        # Theme struct, light/dark/system presets, theme() accessor
├── style.rs        # Style struct (aggregates all visual properties)
├── dimension.rs    # Dimension, Edges, Corners, From impls, pct()
├── animation.rs    # Transition, Animation, Keyframe, Easing, AnimatableProperty, AnimationIterations, AnimationDirection
└── font.rs         # FontWeight, FontFamily, FontStyle, TextAlign, TextOverflow
```

### Color Internals

`Color` stores RGBA as `f32` (0.0..=1.0) internally. Constructors from u8 values divide by 255.0. HSL conversion uses standard algorithm. `lighten`/`darken` convert to HSL, adjust L, convert back. `mix` does linear interpolation in RGB space.

`hex()` parses at runtime (returns `Color` directly, panics on invalid input in debug, returns black in release). Compile-time hex parsing could be added later via const fn.

### Theme Accessor

`theme()` is a function that calls `use_context::<Theme>()` internally. The `App` builder provides a default theme via `provide_context`. This means `theme()` only works inside the widget tree — calling it outside panics. This function lives in `vitreous_style` but the actual context machinery is in `vitreous_reactive` — so `theme()` is defined in the facade crate (`vitreous`) which depends on both. In `vitreous_style` itself, `Theme` is just a plain struct with no reactive coupling.

### Style Composition Pattern

The `Style` struct is a plain data struct. Widget modifiers (`.background()`, `.font_size()`, etc.) are implemented on `Node` in `vitreous_widgets`, not on `Style`. `Style` just holds the data; `Node` modifiers set fields on it.

### Out of Scope

- CSS parsing or CSS-in-Rust
- Runtime style validation beyond type checking
- Theme switching animation (animated transition between themes)
- Custom font loading (that's `vitreous_platform`)
- The `theme()` context accessor function (that requires `vitreous_reactive`, so it lives in the facade crate)

