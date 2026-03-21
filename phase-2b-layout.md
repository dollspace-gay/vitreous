---
title: "Implement vitreous_layout — flexbox layout engine wrapping taffy"
tags: [design-doc]
sources: []
contributors: [unknown]
created: 2026-03-21
updated: 2026-03-21
---


## Design Specification

### Summary

Implement the layout engine that converts a tree of styled nodes into concrete screen positions and sizes. Wraps the `taffy` crate for flexbox (and optional CSS Grid) computation. Provides `LayoutInput`/`LayoutOutput` types, `MeasureFn` for text/image leaf node sizing, and layout boundary optimization for incremental re-layout.

### Requirements

- REQ-1: `LayoutInput` struct representing a node's layout style and children, convertible to taffy's internal representation
- REQ-2: `LayoutOutput` struct with resolved x, y, width, height for each node, plus content_width/content_height for scrollable regions
- REQ-3: `LayoutStyle` struct mapping to flexbox properties: display, flex_direction, flex_wrap, justify_content, align_items, align_self, flex_grow/shrink/basis, width/height/min/max dimensions, padding, margin, gap, aspect_ratio, overflow, position, inset
- REQ-4: `MeasureFn` callback type for leaf nodes (text, images) to report intrinsic size given constraints
- REQ-5: Layout computation via taffy 0.9 — flexbox algorithm with correct space distribution, wrapping, and alignment
- REQ-6: Layout boundary optimization: nodes with explicit width AND height set skip upward propagation during partial re-layout
- REQ-7: Percentage dimensions resolve relative to parent container size
- REQ-8: Absolute positioning via `Position::Absolute` with `inset` (top/right/bottom/left) relative to nearest positioned ancestor
- REQ-9: Overflow: Scroll nodes report content_width/content_height for scrollable area calculation

### Acceptance Criteria

- [ ] AC-1: Column layout with two children (50px and 30px height) in a 200px-wide container: child 0 at y=0 h=50, child 1 at y=50 h=30 (REQ-5)
- [ ] AC-2: Row layout with flex_grow 1.0 and 2.0 children in 300px container: widths are 100px and 200px (REQ-5)
- [ ] AC-3: Text wrapping: leaf with MeasureFn reporting 200px unconstrained width in 100px container produces 2 lines at 40px height (20px per line) (REQ-4, REQ-5)
- [ ] AC-4: Percentage dimensions: 50% width and 25% height child in 400x300 parent produces 200x75 (REQ-7)
- [ ] AC-5: `JustifyContent::Center` with single 100px child in 300px container places child at x=100 (REQ-5)
- [ ] AC-6: `AlignItems::Center` with 50px-tall child in 200px-tall container places child at y=75 (REQ-5)
- [ ] AC-7: Gap of 8px between 3 children in column: positions are 0, h0+8, h0+h1+16 (REQ-5)
- [ ] AC-8: Layout boundary: node with explicit 200x100 dimensions does not trigger parent re-layout when its children change (REQ-6)
- [ ] AC-9: Scroll overflow: container 100px tall with 300px of content reports content_height=300 (REQ-9)
- [ ] AC-10: Property test: random layout trees with random flex properties — no child extends beyond parent (unless overflow), all sizes non-negative, flex space fully distributed (REQ-5)
- [ ] AC-11: Benchmark: layout of 1,000 nodes completes in < 1ms (REQ-5)

### Architecture

### File Structure

```
crates/vitreous_layout/src/
├── lib.rs          # Re-exports, compute_layout entry point
├── tree.rs         # LayoutInput, LayoutOutput, LayoutStyle, MeasureFn, conversion to/from taffy
├── compute.rs      # compute_layout() — builds taffy tree, runs layout, extracts results
└── boundary.rs     # Layout boundary detection and partial re-layout optimization
```

### Dependencies

- `taffy` (0.9) — the layout algorithm itself

### Taffy Integration

`compute_layout()` does:
1. Build a `taffy::TaffyTree` from `LayoutInput` nodes
2. Map `LayoutStyle` fields to `taffy::Style`
3. Register `MeasureFn` callbacks as taffy leaf measure functions
4. Call `taffy::compute_layout()` with root node and available space
5. Walk the taffy output tree, extract positions/sizes into `LayoutOutput`

The taffy tree is rebuilt each frame (for simplicity in v1). Future optimization: maintain a persistent taffy tree and only update dirty nodes.

### Layout Boundary Detection

A node is a layout boundary if:
- It has `width` set to `Px(...)` or `Percent(...)` (not `Auto`) AND
- It has `height` set to `Px(...)` or `Percent(...)` (not `Auto`)

When re-layout is triggered by a dirty subtree, traversal stops at the nearest layout boundary ancestor and only recomputes from there downward. The boundary node's own size is known, so parent layout is unchanged.

### MeasureFn Contract

```rust
pub type MeasureFn = Box<dyn Fn(MeasureConstraint) -> Size>;

pub struct MeasureConstraint {
    pub max_width: Option<f32>,   // None = unconstrained
    pub max_height: Option<f32>,
}
```

Text nodes provide a MeasureFn that calls into the platform's text measurement API. Image nodes provide a MeasureFn that returns intrinsic image dimensions (or scales to fit constraints).

### Out of Scope

- CSS Grid (taffy supports it, but only flexbox for v1; grid exposure is post-v1)
- Constraint-based layout (auto-layout ala Figma)
- Custom layout algorithms beyond flexbox
- Text measurement implementation (that's `vitreous_platform` Phase 5)

