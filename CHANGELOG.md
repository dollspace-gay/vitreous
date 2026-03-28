# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [Unreleased]

### Added
- Implement scroll event dispatch to widget event handlers (#27)
- Implement keyboard event dispatch to widget event handlers (#26)
- Implement mouse event dispatch to widget event handlers (#25)
- Wire up text shaping and GPU presentation in the desktop render pipeline (#20)
- Add kitchen-sink example app exercising all framework features (#16)
- Implement vitreous_hot_reload file watcher, WebSocket server/client, and CLI tool (#15)

### Fixed
- Fix navigate() panic and text rendering (pixelated, misplaced glyphs) (#180)
- Fix layout rendering: viewport coordinate mismatch, missing flex properties, z_stack positioning (#179)
- Fix layout positioning — elements rendering on top of each other (#177)
- Fix TextInput value to use input value, not accessibility label (#112)
- Fix ARIA role mapping for ScrollView — use region instead of scrollbar (#107)
- Fix CubicBezier easing to emit proper CSS cubic-bezier() instead of fallback (#106)
- Fix glyph UV resolution — placeholder UVs are never patched (#45)
- Fix bubble_event visited return semantics to track actual handler runs (#122)
- Fix LayoutOutput.get() linear scan to use HashMap for O(1) lookup (#142)
- Fix ARIA attribute cleanup during reconciliation (#115)
- Implement ImageSource::Bytes support on web via Blob URL (#104)
- Implement NavigateGuard Drop to remove popstate event listener (#103)
- Fix DOM reconciler to handle tag changes by replacing elements (#102)
- Fix text_input on_change handler — currently dead code (#59)
- Remove unused wgpu and cosmic-text dependencies from vitreous_render (#44)
- Fix Renderer::resize to invalidate previous commands for correct redraw (#47)
- Fix glyph atlas cache key to use actual glyph_id and font_hash (#24)
- Fix glyph rasterization to use actual glyph bitmaps instead of white rectangles (#23)
- Fix wgpu MissingDisplayHandle error on Windows (#21)
- Fix hot reload client connection spam and improve kitchen-sink layout (#19)
- Fix router context panic in kitchen-sink nav bar (#18)

### Changed
