use vitreous_layout::{LayoutOutput, NodeId};
use vitreous_style::{Color, Corners, Shadow};

/// A unique identifier for a pre-uploaded image texture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextureId(pub u32);

/// A positioned glyph for text rendering.
#[derive(Debug, Clone, PartialEq)]
pub struct PositionedGlyph {
    pub glyph_id: u16,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub font_hash: u64,
    pub font_size: f32,
    pub scale_factor: f32,
    /// The text fragment this glyph represents (for rasterization fallback).
    pub text_fragment: String,
}

/// Visual style information needed to generate render commands for a node.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeVisualStyle {
    pub background: Option<Color>,
    pub foreground: Option<Color>,
    pub border_color: Option<Color>,
    pub border_width: vitreous_style::Edges,
    pub border_radius: Corners,
    pub opacity: f32,
    pub shadow: Option<Shadow>,
    pub clip_content: bool,
}

impl Default for NodeVisualStyle {
    fn default() -> Self {
        Self {
            background: None,
            foreground: None,
            border_color: None,
            border_width: vitreous_style::Edges::all(0.0),
            border_radius: Corners::all(0.0),
            opacity: 1.0,
            shadow: None,
            clip_content: false,
        }
    }
}

/// Content that a node may contain for rendering.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum NodeContent {
    #[default]
    None,
    Text(Vec<PositionedGlyph>, Color),
    Image(TextureId),
}

/// Input to the command generator: layout + visual style + content for each node.
#[derive(Debug, Clone)]
pub struct RenderNode {
    pub id: NodeId,
    pub style: NodeVisualStyle,
    pub content: NodeContent,
    pub children: Vec<NodeId>,
}

/// A render command representing a single drawing operation.
///
/// Commands are generated in painter's order (back-to-front) and consumed
/// by the render pipeline for GPU submission.
#[derive(Debug, Clone, PartialEq)]
pub enum RenderCommand {
    FillRect {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: Color,
        border_radius: Corners,
    },
    StrokeRect {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: Color,
        border_radius: Corners,
        stroke_width: f32,
    },
    Shadow {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        border_radius: Corners,
        shadow: vitreous_style::Shadow,
    },
    Text {
        glyphs: Vec<PositionedGlyph>,
        color: Color,
    },
    Image {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        texture_id: TextureId,
    },
    PushClip {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        border_radius: Corners,
    },
    PopClip,
    PushOpacity {
        opacity: f32,
    },
    PopOpacity,
}

impl RenderCommand {
    /// Returns the bounding rectangle of this command, if it has one.
    pub fn bounds(&self) -> Option<(f32, f32, f32, f32)> {
        match self {
            Self::FillRect {
                x,
                y,
                width,
                height,
                ..
            }
            | Self::StrokeRect {
                x,
                y,
                width,
                height,
                ..
            }
            | Self::Image {
                x,
                y,
                width,
                height,
                ..
            }
            | Self::PushClip {
                x,
                y,
                width,
                height,
                ..
            } => Some((*x, *y, *width, *height)),
            Self::Shadow {
                x,
                y,
                width,
                height,
                shadow,
                ..
            } => {
                let expand = shadow.blur_radius + shadow.spread_radius;
                Some((
                    *x + shadow.offset_x - expand,
                    *y + shadow.offset_y - expand,
                    *width + expand * 2.0,
                    *height + expand * 2.0,
                ))
            }
            Self::Text { glyphs, .. } => {
                if glyphs.is_empty() {
                    return None;
                }
                let mut min_x = f32::MAX;
                let mut min_y = f32::MAX;
                let mut max_x = f32::MIN;
                let mut max_y = f32::MIN;
                for g in glyphs {
                    min_x = min_x.min(g.x);
                    min_y = min_y.min(g.y);
                    max_x = max_x.max(g.x + g.width);
                    max_y = max_y.max(g.y + g.height);
                }
                Some((min_x, min_y, max_x - min_x, max_y - min_y))
            }
            Self::PopClip | Self::PushOpacity { .. } | Self::PopOpacity => None,
        }
    }

    /// Returns the command type discriminant for batching comparison.
    pub fn kind(&self) -> CommandKind {
        match self {
            Self::FillRect { .. } => CommandKind::FillRect,
            Self::StrokeRect { .. } => CommandKind::StrokeRect,
            Self::Shadow { .. } => CommandKind::Shadow,
            Self::Text { .. } => CommandKind::Text,
            Self::Image { .. } => CommandKind::Image,
            Self::PushClip { .. } => CommandKind::PushClip,
            Self::PopClip => CommandKind::PopClip,
            Self::PushOpacity { .. } => CommandKind::PushOpacity,
            Self::PopOpacity => CommandKind::PopOpacity,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CommandKind {
    FillRect,
    StrokeRect,
    Shadow,
    Text,
    Image,
    PushClip,
    PopClip,
    PushOpacity,
    PopOpacity,
}

/// Generates a render command list from layout output, visual styles, and content.
///
/// Walks the node tree in depth-first order (painter's order), emitting commands
/// for shadows, backgrounds, borders, content (text/image), and child nodes,
/// wrapped in clip/opacity groups as needed.
pub fn generate_commands(
    layout: &LayoutOutput,
    nodes: &[RenderNode],
    root: NodeId,
) -> Vec<RenderCommand> {
    let mut commands = Vec::new();
    let node_map: rustc_hash::FxHashMap<NodeId, &RenderNode> =
        nodes.iter().map(|n| (n.id, n)).collect();
    emit_node(&mut commands, layout, &node_map, root);
    commands
}

fn emit_node(
    commands: &mut Vec<RenderCommand>,
    layout: &LayoutOutput,
    node_map: &rustc_hash::FxHashMap<NodeId, &RenderNode>,
    id: NodeId,
) {
    let Some(node) = node_map.get(&id) else {
        return;
    };
    let Some(nl) = layout.get(id) else {
        return;
    };

    let style = &node.style;
    let needs_opacity = style.opacity < 1.0;
    let needs_clip = style.clip_content;

    if needs_opacity {
        commands.push(RenderCommand::PushOpacity {
            opacity: style.opacity,
        });
    }

    // Shadow (rendered behind the node)
    if let Some(shadow) = &style.shadow {
        commands.push(RenderCommand::Shadow {
            x: nl.x,
            y: nl.y,
            width: nl.width,
            height: nl.height,
            border_radius: style.border_radius,
            shadow: *shadow,
        });
    }

    // Background fill
    if let Some(bg) = &style.background {
        commands.push(RenderCommand::FillRect {
            x: nl.x,
            y: nl.y,
            width: nl.width,
            height: nl.height,
            color: *bg,
            border_radius: style.border_radius,
        });
    }

    // Border stroke
    if let Some(bc) = &style.border_color {
        let bw = &style.border_width;
        let avg_width = (bw.top + bw.right + bw.bottom + bw.left) / 4.0;
        if avg_width > 0.0 {
            commands.push(RenderCommand::StrokeRect {
                x: nl.x,
                y: nl.y,
                width: nl.width,
                height: nl.height,
                color: *bc,
                border_radius: style.border_radius,
                stroke_width: avg_width,
            });
        }
    }

    // Clip children if needed
    if needs_clip {
        commands.push(RenderCommand::PushClip {
            x: nl.x,
            y: nl.y,
            width: nl.width,
            height: nl.height,
            border_radius: style.border_radius,
        });
    }

    // Node content (text or image)
    match &node.content {
        NodeContent::Text(glyphs, color) => {
            if !glyphs.is_empty() {
                commands.push(RenderCommand::Text {
                    glyphs: glyphs.clone(),
                    color: *color,
                });
            }
        }
        NodeContent::Image(texture_id) => {
            let content_x = nl.x + nl.padding.left + nl.border.left;
            let content_y = nl.y + nl.padding.top + nl.border.top;
            commands.push(RenderCommand::Image {
                x: content_x,
                y: content_y,
                width: nl.content_width,
                height: nl.content_height,
                texture_id: *texture_id,
            });
        }
        NodeContent::None => {}
    }

    // Children (depth-first for painter's order)
    for &child_id in &node.children {
        emit_node(commands, layout, node_map, child_id);
    }

    if needs_clip {
        commands.push(RenderCommand::PopClip);
    }

    if needs_opacity {
        commands.push(RenderCommand::PopOpacity);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vitreous_layout::{NodeLayout, Rect};

    fn make_layout(nodes: Vec<(NodeId, NodeLayout)>) -> LayoutOutput {
        LayoutOutput { nodes }
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
    fn fill_rect_command_generated_for_background() {
        let layout = make_layout(vec![(NodeId(0), make_node_layout(10.0, 20.0, 100.0, 50.0))]);
        let nodes = vec![RenderNode {
            id: NodeId(0),
            style: NodeVisualStyle {
                background: Some(Color::RED),
                ..Default::default()
            },
            content: NodeContent::None,
            children: vec![],
        }];

        let cmds = generate_commands(&layout, &nodes, NodeId(0));
        assert_eq!(cmds.len(), 1);
        assert!(matches!(
            &cmds[0],
            RenderCommand::FillRect { x, y, width, height, .. }
            if *x == 10.0 && *y == 20.0 && *width == 100.0 && *height == 50.0
        ));
    }

    #[test]
    fn shadow_emitted_before_fill() {
        let layout = make_layout(vec![(NodeId(0), make_node_layout(0.0, 0.0, 50.0, 50.0))]);
        let nodes = vec![RenderNode {
            id: NodeId(0),
            style: NodeVisualStyle {
                background: Some(Color::WHITE),
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
        }];

        let cmds = generate_commands(&layout, &nodes, NodeId(0));
        assert_eq!(cmds.len(), 2);
        assert!(matches!(&cmds[0], RenderCommand::Shadow { .. }));
        assert!(matches!(&cmds[1], RenderCommand::FillRect { .. }));
    }

    #[test]
    fn opacity_wraps_content() {
        let layout = make_layout(vec![(NodeId(0), make_node_layout(0.0, 0.0, 50.0, 50.0))]);
        let nodes = vec![RenderNode {
            id: NodeId(0),
            style: NodeVisualStyle {
                background: Some(Color::BLUE),
                opacity: 0.5,
                ..Default::default()
            },
            content: NodeContent::None,
            children: vec![],
        }];

        let cmds = generate_commands(&layout, &nodes, NodeId(0));
        assert_eq!(cmds.len(), 3);
        assert!(matches!(&cmds[0], RenderCommand::PushOpacity { opacity } if *opacity == 0.5));
        assert!(matches!(&cmds[1], RenderCommand::FillRect { .. }));
        assert!(matches!(&cmds[2], RenderCommand::PopOpacity));
    }

    #[test]
    fn clip_wraps_children() {
        let layout = make_layout(vec![
            (NodeId(0), make_node_layout(0.0, 0.0, 100.0, 100.0)),
            (NodeId(1), make_node_layout(10.0, 10.0, 30.0, 30.0)),
        ]);
        let nodes = vec![
            RenderNode {
                id: NodeId(0),
                style: NodeVisualStyle {
                    clip_content: true,
                    border_radius: Corners::all(8.0),
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

        let cmds = generate_commands(&layout, &nodes, NodeId(0));
        assert!(matches!(&cmds[0], RenderCommand::PushClip { .. }));
        assert!(matches!(&cmds[1], RenderCommand::FillRect { .. }));
        assert!(matches!(&cmds[2], RenderCommand::PopClip));
    }

    #[test]
    fn border_generates_stroke_rect() {
        let layout = make_layout(vec![(NodeId(0), make_node_layout(0.0, 0.0, 80.0, 40.0))]);
        let nodes = vec![RenderNode {
            id: NodeId(0),
            style: NodeVisualStyle {
                border_color: Some(Color::BLACK),
                border_width: vitreous_style::Edges::all(2.0),
                ..Default::default()
            },
            content: NodeContent::None,
            children: vec![],
        }];

        let cmds = generate_commands(&layout, &nodes, NodeId(0));
        assert_eq!(cmds.len(), 1);
        assert!(matches!(
            &cmds[0],
            RenderCommand::StrokeRect { stroke_width, .. } if *stroke_width == 2.0
        ));
    }

    #[test]
    fn image_content_generates_image_command() {
        let layout = make_layout(vec![(NodeId(0), make_node_layout(5.0, 5.0, 64.0, 64.0))]);
        let nodes = vec![RenderNode {
            id: NodeId(0),
            style: NodeVisualStyle::default(),
            content: NodeContent::Image(TextureId(42)),
            children: vec![],
        }];

        let cmds = generate_commands(&layout, &nodes, NodeId(0));
        assert_eq!(cmds.len(), 1);
        assert!(matches!(
            &cmds[0],
            RenderCommand::Image { texture_id, .. } if *texture_id == TextureId(42)
        ));
    }

    #[test]
    fn text_content_generates_text_command() {
        let glyphs = vec![PositionedGlyph {
            glyph_id: 65,
            x: 10.0,
            y: 20.0,
            width: 8.0,
            height: 12.0,
            font_hash: 123,
            font_size: 16.0,
            scale_factor: 1.0,
            text_fragment: "A".to_owned(),
        }];
        let layout = make_layout(vec![(NodeId(0), make_node_layout(0.0, 0.0, 100.0, 30.0))]);
        let nodes = vec![RenderNode {
            id: NodeId(0),
            style: NodeVisualStyle::default(),
            content: NodeContent::Text(glyphs, Color::BLACK),
            children: vec![],
        }];

        let cmds = generate_commands(&layout, &nodes, NodeId(0));
        assert_eq!(cmds.len(), 1);
        assert!(matches!(&cmds[0], RenderCommand::Text { glyphs, .. } if glyphs.len() == 1));
    }

    #[test]
    fn bounds_for_shadow_expands_by_blur_and_spread() {
        let cmd = RenderCommand::Shadow {
            x: 10.0,
            y: 10.0,
            width: 100.0,
            height: 50.0,
            border_radius: Corners::all(0.0),
            shadow: Shadow {
                offset_x: 2.0,
                offset_y: 4.0,
                blur_radius: 8.0,
                spread_radius: 2.0,
                color: Color::BLACK,
            },
        };
        let (bx, by, bw, bh) = cmd.bounds().unwrap();
        // x + offset_x - (blur + spread) = 10 + 2 - 10 = 2
        assert_eq!(bx, 2.0);
        // y + offset_y - (blur + spread) = 10 + 4 - 10 = 4
        assert_eq!(by, 4.0);
        assert_eq!(bw, 120.0);
        assert_eq!(bh, 70.0);
    }

    #[test]
    fn empty_text_not_emitted() {
        let layout = make_layout(vec![(NodeId(0), make_node_layout(0.0, 0.0, 100.0, 30.0))]);
        let nodes = vec![RenderNode {
            id: NodeId(0),
            style: NodeVisualStyle::default(),
            content: NodeContent::Text(vec![], Color::BLACK),
            children: vec![],
        }];

        let cmds = generate_commands(&layout, &nodes, NodeId(0));
        assert!(cmds.is_empty());
    }

    #[test]
    fn depth_first_order_parent_before_children() {
        let layout = make_layout(vec![
            (NodeId(0), make_node_layout(0.0, 0.0, 200.0, 200.0)),
            (NodeId(1), make_node_layout(10.0, 10.0, 50.0, 50.0)),
            (NodeId(2), make_node_layout(70.0, 10.0, 50.0, 50.0)),
        ]);
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
                    ..Default::default()
                },
                content: NodeContent::None,
                children: vec![],
            },
            RenderNode {
                id: NodeId(2),
                style: NodeVisualStyle {
                    background: Some(Color::BLUE),
                    ..Default::default()
                },
                content: NodeContent::None,
                children: vec![],
            },
        ];

        let cmds = generate_commands(&layout, &nodes, NodeId(0));
        assert_eq!(cmds.len(), 3);
        assert!(
            matches!(&cmds[0], RenderCommand::FillRect { color, .. } if *color == Color::WHITE)
        );
        assert!(matches!(&cmds[1], RenderCommand::FillRect { color, .. } if *color == Color::RED));
        assert!(matches!(&cmds[2], RenderCommand::FillRect { color, .. } if *color == Color::BLUE));
    }
}
