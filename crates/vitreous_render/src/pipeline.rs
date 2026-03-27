use bytemuck::{Pod, Zeroable};

use crate::commands::{CommandKind, RenderCommand};
use crate::damage::DamageRect;

/// GPU vertex type for rectangle instances (fill + stroke).
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct RectInstance {
    pub pos: [f32; 2],
    pub size: [f32; 2],
    pub color: [f32; 4],
    pub border_radius: [f32; 4], // tl, tr, br, bl
    pub border_color: [f32; 4],
    pub border_width: f32,
    pub _pad: [f32; 3],
}

/// GPU vertex type for glyph instances (text rendering).
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct GlyphInstance {
    pub pos: [f32; 2],
    pub size: [f32; 2],
    pub uv_min: [f32; 2],
    pub uv_max: [f32; 2],
    pub color: [f32; 4],
}

/// GPU vertex type for image instances.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct ImageInstance {
    pub pos: [f32; 2],
    pub size: [f32; 2],
    pub uv_min: [f32; 2],
    pub uv_max: [f32; 2],
    pub opacity: f32,
    pub _pad: [f32; 3],
}

/// GPU vertex type for shadow instances.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct ShadowInstance {
    pub pos: [f32; 2],
    pub size: [f32; 2],
    pub rect_pos: [f32; 2],
    pub rect_size: [f32; 2],
    pub color: [f32; 4],
    pub border_radius: [f32; 4],
    pub blur_radius: f32,
    pub _pad: [f32; 3],
}

/// Globals uniform buffer (shared across all shaders).
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct Globals {
    pub viewport_size: [f32; 2],
    pub _pad: [f32; 2],
}

/// A batched draw call — consecutive commands of the same type merged together.
#[derive(Debug)]
pub struct DrawBatch {
    pub kind: BatchKind,
    pub instance_count: u32,
    pub clip_rect: Option<DamageRect>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatchKind {
    Rect,
    Shadow,
    Text,
    Image,
    PushClip,
    PopClip,
    PushOpacity,
    PopOpacity,
}

/// Collects render commands into GPU instance buffers and batched draw calls.
///
/// Consecutive commands of the same type with compatible state are merged
/// into single batches (draw calls), reducing GPU overhead.
/// Metadata for each glyph instance, used for atlas rasterization.
#[derive(Debug, Clone)]
pub struct GlyphKey {
    pub glyph_id: u16,
    pub font_hash: u64,
    pub font_size: f32,
    pub scale_factor: f32,
    pub text_fragment: String,
}

pub struct BatchBuilder {
    pub rect_instances: Vec<RectInstance>,
    pub shadow_instances: Vec<ShadowInstance>,
    pub glyph_instances: Vec<GlyphInstance>,
    pub image_instances: Vec<ImageInstance>,
    pub batches: Vec<DrawBatch>,
    pub clip_stack: Vec<DamageRect>,
    pub opacity_stack: Vec<f32>,
    /// Parallel to `glyph_instances` — carries rasterization metadata.
    pub glyph_keys: Vec<GlyphKey>,
}

impl BatchBuilder {
    pub fn new() -> Self {
        Self {
            rect_instances: Vec::new(),
            shadow_instances: Vec::new(),
            glyph_instances: Vec::new(),
            image_instances: Vec::new(),
            batches: Vec::new(),
            clip_stack: Vec::new(),
            opacity_stack: Vec::new(),
            glyph_keys: Vec::new(),
        }
    }

    /// Clears all buffers for a new frame.
    pub fn clear(&mut self) {
        self.rect_instances.clear();
        self.shadow_instances.clear();
        self.glyph_instances.clear();
        self.image_instances.clear();
        self.batches.clear();
        self.clip_stack.clear();
        self.opacity_stack.clear();
        self.glyph_keys.clear();
    }

    /// Processes a list of render commands into batched draw calls.
    pub fn build(&mut self, commands: &[RenderCommand]) {
        self.clear();

        for cmd in commands {
            match cmd {
                RenderCommand::FillRect {
                    x,
                    y,
                    width,
                    height,
                    color,
                    border_radius,
                } => {
                    let opacity = self.current_opacity();
                    self.push_rect_instance(RectInstance {
                        pos: [*x, *y],
                        size: [*width, *height],
                        color: [
                            color.r * opacity,
                            color.g * opacity,
                            color.b * opacity,
                            color.a * opacity,
                        ],
                        border_radius: [
                            border_radius.top_left,
                            border_radius.top_right,
                            border_radius.bottom_right,
                            border_radius.bottom_left,
                        ],
                        border_color: [0.0; 4],
                        border_width: 0.0,
                        _pad: [0.0; 3],
                    });
                }
                RenderCommand::StrokeRect {
                    x,
                    y,
                    width,
                    height,
                    color,
                    border_radius,
                    stroke_width,
                } => {
                    let opacity = self.current_opacity();
                    self.push_rect_instance(RectInstance {
                        pos: [*x, *y],
                        size: [*width, *height],
                        color: [0.0, 0.0, 0.0, 0.0], // no fill
                        border_radius: [
                            border_radius.top_left,
                            border_radius.top_right,
                            border_radius.bottom_right,
                            border_radius.bottom_left,
                        ],
                        border_color: [
                            color.r * opacity,
                            color.g * opacity,
                            color.b * opacity,
                            color.a * opacity,
                        ],
                        border_width: *stroke_width,
                        _pad: [0.0; 3],
                    });
                }
                RenderCommand::Shadow {
                    x,
                    y,
                    width,
                    height,
                    border_radius,
                    shadow,
                } => {
                    let opacity = self.current_opacity();
                    let expand = shadow.blur_radius + shadow.spread_radius;
                    let shadow_x = *x + shadow.offset_x - expand;
                    let shadow_y = *y + shadow.offset_y - expand;
                    let shadow_w = *width + expand * 2.0;
                    let shadow_h = *height + expand * 2.0;
                    let rect_w = *width + shadow.spread_radius * 2.0;
                    let rect_h = *height + shadow.spread_radius * 2.0;
                    let rect_x = *x + shadow.offset_x - shadow.spread_radius;
                    let rect_y = *y + shadow.offset_y - shadow.spread_radius;

                    self.push_shadow_instance(ShadowInstance {
                        pos: [shadow_x, shadow_y],
                        size: [shadow_w, shadow_h],
                        rect_pos: [rect_x, rect_y],
                        rect_size: [rect_w, rect_h],
                        color: [
                            shadow.color.r * opacity,
                            shadow.color.g * opacity,
                            shadow.color.b * opacity,
                            shadow.color.a * opacity,
                        ],
                        border_radius: [
                            border_radius.top_left,
                            border_radius.top_right,
                            border_radius.bottom_right,
                            border_radius.bottom_left,
                        ],
                        blur_radius: shadow.blur_radius,
                        _pad: [0.0; 3],
                    });
                }
                RenderCommand::Text { glyphs, color } => {
                    let opacity = self.current_opacity();
                    for glyph in glyphs {
                        self.glyph_instances.push(GlyphInstance {
                            pos: [glyph.x, glyph.y],
                            size: [glyph.width, glyph.height],
                            uv_min: [0.0, 0.0], // patched after atlas rasterization
                            uv_max: [1.0, 1.0],
                            color: [
                                color.r * opacity,
                                color.g * opacity,
                                color.b * opacity,
                                color.a * opacity,
                            ],
                        });
                        self.glyph_keys.push(GlyphKey {
                            glyph_id: glyph.glyph_id,
                            font_hash: glyph.font_hash,
                            font_size: glyph.font_size,
                            scale_factor: glyph.scale_factor,
                            text_fragment: glyph.text_fragment.clone(),
                        });
                    }
                    let count = glyphs.len() as u32;
                    if count > 0 {
                        self.push_batch(BatchKind::Text, count);
                    }
                }
                RenderCommand::Image {
                    x,
                    y,
                    width,
                    height,
                    texture_id: _,
                } => {
                    let opacity = self.current_opacity();
                    self.image_instances.push(ImageInstance {
                        pos: [*x, *y],
                        size: [*width, *height],
                        uv_min: [0.0, 0.0],
                        uv_max: [1.0, 1.0],
                        opacity,
                        _pad: [0.0; 3],
                    });
                    self.push_batch(BatchKind::Image, 1);
                }
                RenderCommand::PushClip {
                    x,
                    y,
                    width,
                    height,
                    ..
                } => {
                    self.clip_stack
                        .push(DamageRect::new(*x, *y, *width, *height));
                    self.batches.push(DrawBatch {
                        kind: BatchKind::PushClip,
                        instance_count: 0,
                        clip_rect: Some(DamageRect::new(*x, *y, *width, *height)),
                    });
                }
                RenderCommand::PopClip => {
                    self.clip_stack.pop();
                    self.batches.push(DrawBatch {
                        kind: BatchKind::PopClip,
                        instance_count: 0,
                        clip_rect: None,
                    });
                }
                RenderCommand::PushOpacity { opacity } => {
                    self.opacity_stack.push(*opacity);
                    self.batches.push(DrawBatch {
                        kind: BatchKind::PushOpacity,
                        instance_count: 0,
                        clip_rect: None,
                    });
                }
                RenderCommand::PopOpacity => {
                    self.opacity_stack.pop();
                    self.batches.push(DrawBatch {
                        kind: BatchKind::PopOpacity,
                        instance_count: 0,
                        clip_rect: None,
                    });
                }
            }
        }
    }

    fn current_opacity(&self) -> f32 {
        self.opacity_stack
            .iter()
            .copied()
            .product::<f32>()
            .clamp(0.0, 1.0)
    }

    fn push_rect_instance(&mut self, instance: RectInstance) {
        self.rect_instances.push(instance);
        self.push_batch(BatchKind::Rect, 1);
    }

    fn push_shadow_instance(&mut self, instance: ShadowInstance) {
        self.shadow_instances.push(instance);
        self.push_batch(BatchKind::Shadow, 1);
    }

    /// Merges into the last batch if it's the same kind, otherwise creates a new one.
    fn push_batch(&mut self, kind: BatchKind, count: u32) {
        if let Some(last) = self.batches.last_mut()
            && last.kind == kind
            && is_batchable(kind)
        {
            last.instance_count += count;
            return;
        }
        self.batches.push(DrawBatch {
            kind,
            instance_count: count,
            clip_rect: self.clip_stack.last().copied(),
        });
    }

    /// Returns the total number of draw calls (batches).
    pub fn draw_call_count(&self) -> usize {
        self.batches.len()
    }
}

impl Default for BatchBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Returns true if commands of this kind can be merged into a single batch.
fn is_batchable(kind: BatchKind) -> bool {
    matches!(
        kind,
        BatchKind::Rect | BatchKind::Shadow | BatchKind::Text | BatchKind::Image
    )
}

/// Counts the minimum number of draw calls needed for a command list,
/// after batching consecutive same-type commands.
pub fn count_draw_calls(commands: &[RenderCommand]) -> usize {
    if commands.is_empty() {
        return 0;
    }

    let mut count = 1;
    let mut prev_kind = commands[0].kind();

    for cmd in &commands[1..] {
        let kind = cmd.kind();
        let batchable = matches!(
            kind,
            CommandKind::FillRect
                | CommandKind::StrokeRect
                | CommandKind::Shadow
                | CommandKind::Text
                | CommandKind::Image
        );
        // FillRect and StrokeRect both use the Rect pipeline
        let same_pipeline = match (prev_kind, kind) {
            (CommandKind::FillRect, CommandKind::StrokeRect)
            | (CommandKind::StrokeRect, CommandKind::FillRect)
            | (CommandKind::FillRect, CommandKind::FillRect)
            | (CommandKind::StrokeRect, CommandKind::StrokeRect) => true,
            (a, b) => a == b,
        };

        if !batchable || !same_pipeline {
            count += 1;
        }
        prev_kind = kind;
    }

    count
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::{PositionedGlyph, TextureId};
    use vitreous_style::{Color, Corners, Shadow};

    fn fill(x: f32, y: f32, w: f32, h: f32) -> RenderCommand {
        RenderCommand::FillRect {
            x,
            y,
            width: w,
            height: h,
            color: Color::RED,
            border_radius: Corners::all(0.0),
        }
    }

    fn stroke(x: f32, y: f32, w: f32, h: f32) -> RenderCommand {
        RenderCommand::StrokeRect {
            x,
            y,
            width: w,
            height: h,
            color: Color::BLACK,
            border_radius: Corners::all(0.0),
            stroke_width: 1.0,
        }
    }

    #[test]
    fn ten_fill_rects_batch_into_one_draw_call() {
        // AC-8: 10 consecutive FillRect commands batched into 1 draw call
        let commands: Vec<_> = (0..10)
            .map(|i| fill(i as f32 * 20.0, 0.0, 18.0, 18.0))
            .collect();

        let mut builder = BatchBuilder::new();
        builder.build(&commands);

        assert_eq!(builder.draw_call_count(), 1);
        assert_eq!(builder.rect_instances.len(), 10);
        assert_eq!(builder.batches[0].instance_count, 10);
    }

    #[test]
    fn fill_and_stroke_batch_together() {
        // FillRect and StrokeRect use the same shader pipeline
        let commands = vec![
            fill(0.0, 0.0, 50.0, 50.0),
            stroke(60.0, 0.0, 50.0, 50.0),
            fill(120.0, 0.0, 50.0, 50.0),
        ];

        let mut builder = BatchBuilder::new();
        builder.build(&commands);

        assert_eq!(builder.draw_call_count(), 1);
        assert_eq!(builder.rect_instances.len(), 3);
    }

    #[test]
    fn different_types_break_batch() {
        let commands = vec![
            fill(0.0, 0.0, 50.0, 50.0),
            RenderCommand::Image {
                x: 60.0,
                y: 0.0,
                width: 50.0,
                height: 50.0,
                texture_id: TextureId(1),
            },
            fill(120.0, 0.0, 50.0, 50.0),
        ];

        let mut builder = BatchBuilder::new();
        builder.build(&commands);

        assert_eq!(builder.draw_call_count(), 3);
    }

    #[test]
    fn clip_breaks_batch() {
        let commands = vec![
            fill(0.0, 0.0, 50.0, 50.0),
            RenderCommand::PushClip {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 100.0,
                border_radius: Corners::all(0.0),
            },
            fill(10.0, 10.0, 30.0, 30.0),
            RenderCommand::PopClip,
        ];

        let mut builder = BatchBuilder::new();
        builder.build(&commands);

        // fill, push_clip, fill, pop_clip = 4 batches
        assert_eq!(builder.draw_call_count(), 4);
    }

    #[test]
    fn opacity_applied_to_instances() {
        let commands = vec![
            RenderCommand::PushOpacity { opacity: 0.5 },
            fill(0.0, 0.0, 50.0, 50.0),
            RenderCommand::PopOpacity,
        ];

        let mut builder = BatchBuilder::new();
        builder.build(&commands);

        // The rect instance should have opacity baked in
        assert_eq!(builder.rect_instances.len(), 1);
        let inst = &builder.rect_instances[0];
        // Color::RED = (1, 0, 0, 1), with 0.5 opacity = (0.5, 0, 0, 0.5)
        assert!((inst.color[0] - 0.5).abs() < 0.01);
        assert!((inst.color[3] - 0.5).abs() < 0.01);
    }

    #[test]
    fn count_draw_calls_batching() {
        let commands: Vec<_> = (0..10)
            .map(|i| fill(i as f32 * 20.0, 0.0, 18.0, 18.0))
            .collect();
        assert_eq!(count_draw_calls(&commands), 1);
    }

    #[test]
    fn count_draw_calls_mixed() {
        let commands = vec![
            fill(0.0, 0.0, 50.0, 50.0),
            fill(60.0, 0.0, 50.0, 50.0),
            RenderCommand::Text {
                glyphs: vec![PositionedGlyph {
                    glyph_id: 65,
                    x: 0.0,
                    y: 0.0,
                    width: 8.0,
                    height: 12.0,
                    font_hash: 0,
                    font_size: 16.0,
                    scale_factor: 1.0,
                    text_fragment: "A".to_owned(),
                }],
                color: Color::BLACK,
            },
            fill(120.0, 0.0, 50.0, 50.0),
        ];
        // 2 fills batched, 1 text, 1 fill = 3 draw calls
        assert_eq!(count_draw_calls(&commands), 3);
    }

    #[test]
    fn shadow_instances_generated() {
        let commands = vec![RenderCommand::Shadow {
            x: 10.0,
            y: 10.0,
            width: 100.0,
            height: 50.0,
            border_radius: Corners::all(4.0),
            shadow: Shadow {
                offset_x: 0.0,
                offset_y: 2.0,
                blur_radius: 8.0,
                spread_radius: 0.0,
                color: Color::BLACK,
            },
        }];

        let mut builder = BatchBuilder::new();
        builder.build(&commands);

        assert_eq!(builder.shadow_instances.len(), 1);
        assert_eq!(builder.draw_call_count(), 1);
        let inst = &builder.shadow_instances[0];
        assert_eq!(inst.blur_radius, 8.0);
    }

    #[test]
    fn empty_commands_no_batches() {
        let mut builder = BatchBuilder::new();
        builder.build(&[]);
        assert_eq!(builder.draw_call_count(), 0);
        assert_eq!(count_draw_calls(&[]), 0);
    }

    #[test]
    fn nested_opacity_multiplies() {
        let commands = vec![
            RenderCommand::PushOpacity { opacity: 0.5 },
            RenderCommand::PushOpacity { opacity: 0.5 },
            fill(0.0, 0.0, 50.0, 50.0),
            RenderCommand::PopOpacity,
            RenderCommand::PopOpacity,
        ];

        let mut builder = BatchBuilder::new();
        builder.build(&commands);

        let inst = &builder.rect_instances[0];
        // 0.5 * 0.5 = 0.25
        assert!((inst.color[0] - 0.25).abs() < 0.01);
        assert!((inst.color[3] - 0.25).abs() < 0.01);
    }

    #[test]
    fn bytemuck_repr_c_sizes() {
        // Verify GPU struct sizes are reasonable (no hidden padding issues)
        assert_eq!(std::mem::size_of::<RectInstance>(), 80);
        assert_eq!(std::mem::size_of::<GlyphInstance>(), 48);
        assert_eq!(std::mem::size_of::<ShadowInstance>(), 80);
        assert_eq!(std::mem::size_of::<Globals>(), 16);
    }
}
