# Feature: Implement vitreous_render — desktop GPU renderer via wgpu

## Summary

Implement the desktop rendering backend that takes a `LayoutOutput` tree with styles and produces GPU-rendered frames via wgpu. Includes the `RenderCommand` list abstraction, SDF-based rounded rectangle shaders, glyph/image texture atlas, damage tracking for partial re-render, and frame-to-frame command diffing.

## Requirements

- REQ-1: `RenderCommand` enum with variants: FillRect, StrokeRect, Shadow, Text, Image, PushClip, PopClip, PushOpacity, PopOpacity
- REQ-2: wgpu render pipeline with WGSL shaders for rectangles (with border radius via SDF), text (textured quads from glyph atlas), images (textured quads), and shadows (blurred SDF)
- REQ-3: Glyph texture atlas: rasterized glyphs cached in a GPU texture, looked up by (glyph_id, font, size, scale_factor)
- REQ-4: Image texture atlas or individual textures: images uploaded to GPU on first use, cached by `ImageSource`
- REQ-5: Clipping via stencil buffer (not scissor rects) to support rounded clip regions
- REQ-6: Damage tracking: maintain damage rect list, only submit draw calls for regions that changed since last frame
- REQ-7: Frame-to-frame command diffing: compare new command list against previous to compute damage rects
- REQ-8: Rounded rectangles rendered with crisp edges at any resolution via SDF fragment shaders
- REQ-9: Draw call batching: consecutive commands of the same type with compatible state are merged into single draw calls
- REQ-10: Frame pipeline: dirty signals -> rebuild dirty subtrees -> merge into tree -> layout -> generate commands -> diff -> submit GPU commands -> present

## Acceptance Criteria

- [ ] AC-1: `RenderCommand::FillRect` with border_radius > 0 produces a visually rounded rectangle (verified by screenshot test) (REQ-1, REQ-8)
- [ ] AC-2: `RenderCommand::Text` with positioned glyphs renders readable text at 1x and 2x scale factors (REQ-2, REQ-3)
- [ ] AC-3: Same glyph rendered twice uses cached atlas entry (atlas lookup count test) (REQ-3)
- [ ] AC-4: `RenderCommand::Image` with texture_id renders the correct image (REQ-4)
- [ ] AC-5: PushClip with rounded rect clips child content to the rounded region (REQ-5)
- [ ] AC-6: Changing one node's background color produces a damage rect covering only that node, not the entire window (REQ-6, REQ-7)
- [ ] AC-7: Frame with no changes produces zero GPU submissions (idle frame optimization) (REQ-6)
- [ ] AC-8: 10 consecutive FillRect commands with same texture/shader state are batched into 1 draw call (REQ-9)
- [ ] AC-9: Command generation from 1,000-node layout tree completes in < 2ms (REQ-10)
- [ ] AC-10: Full frame render (1,000 nodes, 10% dirty) completes in < 4ms (REQ-10)
- [ ] AC-11: Rect SDF shader produces anti-aliased edges (no jagged pixels at non-integer positions) (REQ-8)

## Architecture

### File Structure

```
crates/vitreous_render/src/
├── lib.rs          # Re-exports, Renderer struct with init/render_frame methods
├── commands.rs     # RenderCommand enum, command list generation from LayoutOutput + Style
├── pipeline.rs     # wgpu render pipeline setup, shader compilation, vertex/index buffers
├── atlas.rs        # GlyphAtlas + ImageAtlas — texture packing, cache lookup, GPU upload
├── damage.rs       # DamageTracker — damage rect computation, region merging
├── diff.rs         # Frame diffing — compare old/new command lists to find changed regions
└── shaders/
    ├── rect.wgsl   # Rounded rectangle SDF shader (fill + stroke + border radius)
    ├── text.wgsl   # Textured quad shader for glyph rendering
    ├── image.wgsl  # Textured quad shader for image rendering
    └── shadow.wgsl # Box shadow shader (blurred SDF)
```

### Dependencies

- `vitreous_layout` — LayoutOutput for position/size data
- `wgpu` (29) — GPU rendering
- `cosmic-text` (0.18) — glyph rasterization for atlas population (text shaping happens in `vitreous_platform`, but the renderer needs to rasterize shaped glyphs)

### Renderer Lifecycle

```rust
pub struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface,
    pipeline: RenderPipeline,
    glyph_atlas: GlyphAtlas,
    image_atlas: ImageAtlas,
    damage_tracker: DamageTracker,
    prev_commands: Vec<RenderCommand>,
}

impl Renderer {
    pub fn new(window: &impl HasRawWindowHandle, size: (u32, u32)) -> Self;
    pub fn resize(&mut self, new_size: (u32, u32));
    pub fn render_frame(&mut self, commands: Vec<RenderCommand>);
}
```

### SDF Rounded Rectangle

The fragment shader computes signed distance from the pixel to the rounded rectangle boundary:
- For each corner with radius r, the SDF is `length(max(abs(p - corner_center) - half_size + r, 0.0)) - r`
- Distance < 0 = inside, > 0 = outside
- Anti-aliasing via `smoothstep` over a 1-pixel band at the boundary
- Border: two SDF evaluations (outer - border_width = inner), fill between

### Damage Tracking

The `DamageTracker` maintains the previous frame's command list. On new frame:
1. Walk new and old command lists in parallel
2. Commands that changed → add their rect to damage list
3. Commands that were added/removed → add their rect
4. Merge overlapping damage rects (union with a small margin)
5. Clip GPU rendering to damage rects via viewport/scissor

### Glyph Atlas

Rectangle-packing algorithm (shelf-based or skyline) allocates glyph bitmaps into a single GPU texture (e.g., 2048x2048). When atlas is full, create a new atlas texture page. Cache key: `(glyph_id, font_hash, size_quantized, scale_factor_quantized)`.

## Open Questions

None — rendering architecture follows established patterns (similar to vello, femtovg, iced_wgpu).

## Out of Scope

- Subpixel text rendering (defer to cosmic-text's default behavior)
- GPU-accelerated SVG rendering
- 3D rendering or perspective transforms
- Custom shader injection by users (post-v1, via Canvas node)
- Multi-window rendering (single window for v1)
