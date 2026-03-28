pub mod atlas;
pub mod commands;
pub mod damage;
pub mod diff;
pub mod pipeline;

pub use atlas::{
    AtlasRegion, GlyphAtlas, GlyphBearing, GlyphCacheKey, ImageAtlas, ImageCacheKey, ImageEntry,
};
pub use commands::{
    CommandKind, NodeContent, NodeVisualStyle, PositionedGlyph, RenderCommand, RenderNode,
    TextureId, generate_commands,
};
pub use damage::{DamageRect, DamageTracker};
pub use diff::{commands_equal, diff_commands};
pub use pipeline::{
    BatchBuilder, BatchKind, DrawBatch, Globals, GlyphInstance, GlyphKey, ImageInstance,
    RectInstance, ShadowInstance, count_draw_calls,
};

/// The main renderer that manages the full frame pipeline.
///
/// Lifecycle:
/// 1. Create with `Renderer::new(width, height)`
/// 2. Each frame, call `render_frame(commands)` with the new command list
/// 3. The renderer diffs against the previous frame, computes damage rects,
///    builds batched draw calls, and reports whether GPU submission is needed
/// 4. On resize, call `resize(new_width, new_height)`
///
/// The renderer does not own the wgpu device/surface directly — those are
/// managed by `vitreous_platform`. This struct manages the CPU-side frame
/// pipeline: command diffing, damage tracking, atlas caching, and batch building.
pub struct Renderer {
    width: u32,
    height: u32,
    glyph_atlas: GlyphAtlas,
    image_atlas: ImageAtlas,
    damage_tracker: DamageTracker,
    batch_builder: BatchBuilder,
    prev_commands: Vec<RenderCommand>,
    frame_count: u64,
}

impl Renderer {
    /// Creates a new renderer for the given viewport dimensions.
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            glyph_atlas: GlyphAtlas::new(),
            image_atlas: ImageAtlas::new(),
            damage_tracker: DamageTracker::new(4.0),
            batch_builder: BatchBuilder::new(),
            prev_commands: Vec::new(),
            frame_count: 0,
        }
    }

    /// Updates the viewport dimensions. Marks the entire viewport as damaged.
    pub fn resize(&mut self, new_width: u32, new_height: u32) {
        self.width = new_width;
        self.height = new_height;
        self.invalidate();
    }

    /// Processes a new frame's render commands through the full pipeline:
    ///
    /// 1. Diff new commands against previous frame
    /// 2. Compute damage rects from the diff
    /// 3. If no damage, skip GPU work (idle frame optimization — AC-7)
    /// 4. Build batched draw calls from commands
    /// 5. Store commands for next frame's diff
    ///
    /// Returns a `FrameOutput` describing what needs to be submitted to the GPU.
    pub fn render_frame(&mut self, commands: Vec<RenderCommand>) -> FrameOutput {
        self.frame_count += 1;
        self.damage_tracker.clear();

        // Step 1-2: Diff and compute damage
        if commands_equal(&self.prev_commands, &commands) {
            // AC-7: Frame with no changes produces zero GPU submissions
            self.prev_commands = commands;
            return FrameOutput {
                needs_submit: false,
                damage_rects: Vec::new(),
                draw_call_count: 0,
                frame_number: self.frame_count,
            };
        }

        let damage = diff_commands(&self.prev_commands, &commands);
        for rect in &damage {
            self.damage_tracker.add(*rect);
        }

        let damage_rects = self
            .damage_tracker
            .clipped_rects(self.width as f32, self.height as f32);

        // Step 4: Build batched draw calls
        self.batch_builder.build(&commands);
        let draw_call_count = self.batch_builder.draw_call_count();

        // Step 5: Store for next frame
        self.prev_commands = commands;

        FrameOutput {
            needs_submit: true,
            damage_rects,
            draw_call_count,
            frame_number: self.frame_count,
        }
    }

    /// Returns the current viewport dimensions.
    pub fn viewport(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Returns a reference to the glyph atlas for atlas lookups and insertion.
    pub fn glyph_atlas(&mut self) -> &mut GlyphAtlas {
        &mut self.glyph_atlas
    }

    /// Returns a reference to the image atlas for texture cache management.
    pub fn image_atlas(&mut self) -> &mut ImageAtlas {
        &mut self.image_atlas
    }

    /// Returns a reference to the batch builder (populated after `render_frame`).
    pub fn batch_builder(&self) -> &BatchBuilder {
        &self.batch_builder
    }

    /// Returns a mutable reference to the batch builder for post-processing
    /// (e.g. patching glyph UV coordinates after atlas lookup).
    pub fn batch_builder_mut(&mut self) -> &mut BatchBuilder {
        &mut self.batch_builder
    }

    /// Returns the current frame count.
    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }

    /// Returns the globals uniform data for the current viewport.
    pub fn globals(&self) -> Globals {
        Globals {
            viewport_size: [self.width as f32, self.height as f32],
            _pad: [0.0; 2],
        }
    }

    /// Forces a full redraw on the next frame by clearing the previous command list.
    pub fn invalidate(&mut self) {
        self.prev_commands.clear();
    }
}

/// Output from a frame render pass.
#[derive(Debug)]
pub struct FrameOutput {
    /// Whether the GPU needs to submit draw calls this frame.
    /// False when the frame is identical to the previous one (idle optimization).
    pub needs_submit: bool,
    /// Damage rects clipped to viewport. Use these as scissor rects for partial re-render.
    pub damage_rects: Vec<DamageRect>,
    /// Number of batched draw calls.
    pub draw_call_count: usize,
    /// Monotonically increasing frame number.
    pub frame_number: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use vitreous_layout::{LayoutOutput, NodeId, NodeLayout, Rect};
    use vitreous_style::{Color, Corners, Shadow};

    fn fill(x: f32, y: f32, w: f32, h: f32, color: Color) -> RenderCommand {
        RenderCommand::FillRect {
            x,
            y,
            width: w,
            height: h,
            color,
            border_radius: Corners::all(0.0),
        }
    }

    fn make_node_layout(x: f32, y: f32, w: f32, h: f32) -> NodeLayout {
        NodeLayout {
            x,
            y,
            width: w,
            height: h,
            content_width: w,
            content_height: h,
            padding: Rect::ZERO,
            border: Rect::ZERO,
            margin: Rect::ZERO,
        }
    }

    #[test]
    fn idle_frame_no_submit() {
        // AC-7: Frame with no changes produces zero GPU submissions
        let mut renderer = Renderer::new(800, 600);
        let commands = vec![fill(0.0, 0.0, 100.0, 50.0, Color::RED)];

        let output1 = renderer.render_frame(commands.clone());
        assert!(output1.needs_submit);
        assert_eq!(output1.frame_number, 1);

        let output2 = renderer.render_frame(commands);
        assert!(!output2.needs_submit);
        assert!(output2.damage_rects.is_empty());
        assert_eq!(output2.draw_call_count, 0);
        assert_eq!(output2.frame_number, 2);
    }

    #[test]
    fn color_change_localized_damage() {
        // AC-6: Changing one node's background color produces damage covering
        // only that node, not the entire window
        let mut renderer = Renderer::new(800, 600);

        let frame1 = vec![
            fill(0.0, 0.0, 800.0, 600.0, Color::WHITE),
            fill(100.0, 100.0, 200.0, 100.0, Color::RED),
        ];
        renderer.render_frame(frame1);

        let frame2 = vec![
            fill(0.0, 0.0, 800.0, 600.0, Color::WHITE),
            fill(100.0, 100.0, 200.0, 100.0, Color::BLUE), // changed
        ];
        let output = renderer.render_frame(frame2);

        assert!(output.needs_submit);
        // Damage should NOT cover the full window
        for rect in &output.damage_rects {
            assert!(rect.width < 800.0 || rect.height < 600.0);
        }
    }

    #[test]
    fn ten_fill_rects_one_draw_call() {
        // AC-8: 10 consecutive FillRect commands batched into 1 draw call
        let mut renderer = Renderer::new(800, 600);
        let commands: Vec<_> = (0..10)
            .map(|i| fill(i as f32 * 50.0, 0.0, 40.0, 40.0, Color::RED))
            .collect();

        let output = renderer.render_frame(commands);
        assert!(output.needs_submit);
        assert_eq!(output.draw_call_count, 1);
    }

    #[test]
    fn command_generation_from_layout_tree() {
        // Test the full pipeline: layout -> commands -> batching
        let layout = LayoutOutput {
            nodes: vec![
                (NodeId(0), make_node_layout(0.0, 0.0, 800.0, 600.0)),
                (NodeId(1), make_node_layout(10.0, 10.0, 200.0, 100.0)),
                (NodeId(2), make_node_layout(220.0, 10.0, 200.0, 100.0)),
            ]
            .into_iter()
            .collect(),
        };

        let nodes = vec![
            RenderNode {
                id: NodeId(0),
                style: NodeVisualStyle {
                    background: Some(Color::WHITE),
                    ..Default::default()
                },
                content: NodeContent::None,
                children: vec![NodeId(1), NodeId(2)],
            },
            RenderNode {
                id: NodeId(1),
                style: NodeVisualStyle {
                    background: Some(Color::RED),
                    border_radius: Corners::all(8.0),
                    ..Default::default()
                },
                content: NodeContent::None,
                children: vec![],
            },
            RenderNode {
                id: NodeId(2),
                style: NodeVisualStyle {
                    background: Some(Color::BLUE),
                    shadow: Some(Shadow {
                        offset_x: 0.0,
                        offset_y: 2.0,
                        blur_radius: 4.0,
                        spread_radius: 0.0,
                        color: Color::BLACK,
                    }),
                    ..Default::default()
                },
                content: NodeContent::None,
                children: vec![],
            },
        ];

        let commands = generate_commands(&layout, &nodes, NodeId(0));
        // Node 0: fill, Node 1: fill, Node 2: shadow + fill = 4 commands
        assert_eq!(commands.len(), 4);

        let mut renderer = Renderer::new(800, 600);
        let output = renderer.render_frame(commands);
        assert!(output.needs_submit);
        // Shadow breaks batch: rects(1), shadow(1), rects(2) = 3 draw calls
        assert_eq!(output.draw_call_count, 3);
    }

    #[test]
    fn resize_updates_viewport() {
        let mut renderer = Renderer::new(800, 600);
        assert_eq!(renderer.viewport(), (800, 600));

        renderer.resize(1920, 1080);
        assert_eq!(renderer.viewport(), (1920, 1080));

        let globals = renderer.globals();
        assert_eq!(globals.viewport_size, [1920.0, 1080.0]);
    }

    #[test]
    fn invalidate_forces_full_redraw() {
        let mut renderer = Renderer::new(800, 600);
        let commands = vec![fill(0.0, 0.0, 100.0, 50.0, Color::RED)];

        renderer.render_frame(commands.clone());
        renderer.invalidate();

        let output = renderer.render_frame(commands);
        assert!(output.needs_submit);
    }

    #[test]
    fn glyph_atlas_accessible() {
        let mut renderer = Renderer::new(800, 600);
        let key = GlyphCacheKey::new(65, 1234, 16.0, 1.0);
        let region = renderer.glyph_atlas().insert(key, 12, 16);
        assert_eq!(region.width, 12);

        let cached = renderer.glyph_atlas().get(key);
        assert!(cached.is_some());
    }

    #[test]
    fn image_atlas_accessible() {
        let mut renderer = Renderer::new(800, 600);
        let key = ImageCacheKey(TextureId(1));
        assert!(renderer.image_atlas().insert(key, 256, 256));
        assert!(renderer.image_atlas().contains(key));
    }

    #[test]
    fn frame_counter_increments() {
        let mut renderer = Renderer::new(800, 600);
        assert_eq!(renderer.frame_count(), 0);

        renderer.render_frame(vec![]);
        assert_eq!(renderer.frame_count(), 1);

        renderer.render_frame(vec![]);
        assert_eq!(renderer.frame_count(), 2);
    }

    #[test]
    fn full_pipeline_with_clip_and_opacity() {
        let layout = LayoutOutput {
            nodes: vec![
                (NodeId(0), make_node_layout(0.0, 0.0, 400.0, 300.0)),
                (NodeId(1), make_node_layout(10.0, 10.0, 100.0, 100.0)),
            ]
            .into_iter()
            .collect(),
        };

        let nodes = vec![
            RenderNode {
                id: NodeId(0),
                style: NodeVisualStyle {
                    background: Some(Color::WHITE),
                    clip_content: true,
                    opacity: 0.8,
                    border_radius: Corners::all(12.0),
                    ..Default::default()
                },
                content: NodeContent::None,
                children: vec![NodeId(1)],
            },
            RenderNode {
                id: NodeId(1),
                style: NodeVisualStyle {
                    background: Some(Color::GREEN),
                    ..Default::default()
                },
                content: NodeContent::None,
                children: vec![],
            },
        ];

        let commands = generate_commands(&layout, &nodes, NodeId(0));
        // PushOpacity, fill(white), PushClip, fill(green), PopClip, PopOpacity
        assert_eq!(commands.len(), 6);
        assert!(matches!(&commands[0], RenderCommand::PushOpacity { .. }));
        assert!(matches!(&commands[1], RenderCommand::FillRect { .. }));
        assert!(matches!(&commands[2], RenderCommand::PushClip { .. }));
        assert!(matches!(&commands[3], RenderCommand::FillRect { .. }));
        assert!(matches!(&commands[4], RenderCommand::PopClip));
        assert!(matches!(&commands[5], RenderCommand::PopOpacity));

        let mut renderer = Renderer::new(400, 300);
        let output = renderer.render_frame(commands);
        assert!(output.needs_submit);
    }
}
