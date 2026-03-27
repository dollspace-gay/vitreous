use vitreous_a11y::{A11yNode, FocusManager, generate_accesskit_tree};
use vitreous_events::{
    Key, KeyCode, KeyEvent, LayoutNode, Modifiers, MouseButton, MouseEvent, NodeId, Point,
    ScrollEvent, hit_test,
};
use vitreous_layout::{AvailableSpace, LayoutOutput, compute_layout};
use vitreous_reactive::{Scope, batch, create_scope};
use vitreous_render::{
    NodeContent, NodeVisualStyle, RenderCommand, RenderNode, Renderer, generate_commands,
};
use vitreous_style::Corners as StyleCorners;
use vitreous_widgets::Node;

use crate::gpu::{GpuContext, PresentError};
use crate::text_engine::CosmicTextEngine;
use crate::window::{PlatformWindow, WindowConfig};

use winit::application::ApplicationHandler;
use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{
    Key as WinitKey, KeyCode as WinitKeyCode, ModifiersState, NamedKey, PhysicalKey,
};

// ═══════════════════════════════════════════════════════════════════════════
// DesktopRuntime — the full desktop application runtime
// ═══════════════════════════════════════════════════════════════════════════

/// The desktop application runtime. Owns the window, renderer, text engine,
/// accessibility adapter, and reactive runtime. Drives the full pipeline:
///
/// `winit events → vitreous events → reactive updates → layout → render → present`
pub struct DesktopRuntime {
    config: WindowConfig,
    root_fn: Box<dyn Fn() -> Node>,
    window: Option<PlatformWindow>,
    renderer: Option<Renderer>,
    gpu: Option<GpuContext>,
    /// Text engine for font discovery, measurement, shaping, and rasterization.
    /// Used during the render pass to shape text nodes into positioned glyphs.
    pub text_engine: CosmicTextEngine,
    focus_manager: Option<FocusManager>,
    root_scope: Option<Scope>,
    layout_output: Option<LayoutOutput>,
    layout_nodes: Vec<LayoutNode>,
    scale_factor: f64,
    modifiers: ModifiersState,
    mouse_position: Option<PhysicalPosition<f64>>,
    needs_rebuild: bool,
    frame_count: u64,
    should_close: bool,
}

impl DesktopRuntime {
    /// Create a new desktop runtime with the given window configuration
    /// and root widget function.
    pub fn new(config: WindowConfig, root_fn: impl Fn() -> Node + 'static) -> Self {
        Self {
            config,
            root_fn: Box::new(root_fn),
            window: None,
            renderer: None,
            gpu: None,
            text_engine: CosmicTextEngine::new(),
            focus_manager: None,
            root_scope: None,
            layout_output: None,
            layout_nodes: Vec::new(),
            scale_factor: 1.0,
            modifiers: ModifiersState::empty(),
            mouse_position: None,
            needs_rebuild: true,
            frame_count: 0,
            should_close: false,
        }
    }

    /// Run the application. This blocks the current thread until the window
    /// is closed.
    pub fn run(config: WindowConfig, root_fn: impl Fn() -> Node + 'static) {
        let event_loop = EventLoop::new().expect("failed to create event loop");
        event_loop.set_control_flow(ControlFlow::Wait);
        let mut runtime = Self::new(config, root_fn);
        event_loop.run_app(&mut runtime).expect("event loop failed");
    }

    // ───────────────────────────────────────────────────────────────────
    // Pipeline stages
    // ───────────────────────────────────────────────────────────────────

    /// Stage 1: Build/rebuild the widget tree from the root function.
    fn build_widget_tree(&mut self) -> Node {
        // Create a new scope (dropping the old one cleans up old reactive state)
        let mut root_node = None;
        let root_fn = &self.root_fn;
        let scope = create_scope(|| {
            root_node = Some(root_fn());
        });
        self.root_scope = Some(scope);
        root_node.expect("root_fn must return a Node")
    }

    /// Stage 2: Convert widget tree to layout inputs and compute layout.
    fn compute_layout(&mut self, root: &Node, width: f32, height: f32) {
        let mut layout_inputs = Vec::new();
        let mut next_id = 0u32;
        flatten_node_to_layout(root, &mut layout_inputs, &mut next_id);

        let root_id = vitreous_layout::NodeId(0);
        let available = AvailableSpace { width, height };
        let output = compute_layout(&layout_inputs, root_id, available);

        // Build LayoutNode list for hit testing
        self.layout_nodes = build_layout_nodes(&output, root);

        self.layout_output = Some(output);
    }

    /// Stage 3: Generate render commands from layout + styles, including text shaping.
    fn generate_render_commands(&mut self, root: &Node) -> Vec<RenderCommand> {
        let Some(ref layout) = self.layout_output else {
            return Vec::new();
        };

        let mut render_nodes = Vec::new();
        let mut next_id = 0u32;
        let scale = self.scale_factor as f32;
        flatten_node_to_render(root, &mut render_nodes, &mut next_id, &mut self.text_engine, scale);

        let root_id = vitreous_layout::NodeId(0);
        generate_commands(layout, &render_nodes, root_id)
    }

    /// Stage 4: Submit render commands to the renderer, rasterize glyphs, and present via GPU.
    fn submit_frame(&mut self, commands: Vec<RenderCommand>) {
        let Some(ref mut renderer) = self.renderer else {
            return;
        };
        let output = renderer.render_frame(commands);

        if !output.needs_submit {
            return;
        }

        // Rasterize glyphs, upload to atlas, and patch UV coordinates.
        let scale = self.scale_factor as f32;
        let atlas_size = 2048u32;
        let glyph_count = renderer.batch_builder().glyph_instances.len();

        // Collect glyph keys first to avoid borrow conflicts
        let keys: Vec<vitreous_render::pipeline::GlyphKey> =
            renderer.batch_builder().glyph_keys.clone();

        for i in 0..glyph_count {
            if i >= keys.len() {
                break;
            }
            let key = &keys[i];

            let cache_key = vitreous_render::GlyphCacheKey::new(
                key.glyph_id,
                key.font_hash,
                key.font_size,
                scale,
            );

            // Check if already in atlas
            if let Some(region) = renderer.glyph_atlas().get(cache_key) {
                let (u_min, v_min, u_max, v_max) = region.uv(atlas_size);
                let inst = &mut renderer.batch_builder_mut().glyph_instances[i];
                inst.uv_min = [u_min, v_min];
                inst.uv_max = [u_max, v_max];
                continue;
            }

            // Rasterize the glyph using the text engine
            let font_desc = crate::text_engine::FontDescriptor {
                family: vitreous_style::FontFamily::SansSerif,
                size: key.font_size,
                weight: vitreous_style::FontWeight::Regular,
                style: vitreous_style::FontStyle::Normal,
            };

            let bitmap = self.text_engine.rasterize_glyph(
                &key.text_fragment,
                &font_desc,
                scale,
            );

            let (gw, gh, data) = match bitmap {
                Some(bm) if bm.width > 0 && bm.height > 0 => {
                    (bm.width, bm.height, bm.data)
                }
                _ => {
                    // Space or unrasterizable glyph — use a 1x1 transparent pixel
                    (1, 1, vec![0u8])
                }
            };

            let region = renderer.glyph_atlas().insert(cache_key, gw, gh);

            if let Some(ref gpu) = self.gpu {
                gpu.upload_glyph(&data, region.x, region.y, gw, gh);
            }

            let (u_min, v_min, u_max, v_max) = region.uv(atlas_size);
            let inst = &mut renderer.batch_builder_mut().glyph_instances[i];
            inst.uv_min = [u_min, v_min];
            inst.uv_max = [u_max, v_max];
        }

        // Extract clear color from theme background (default dark gray)
        let clear = [0.12f32, 0.12, 0.14, 1.0];

        if let Some(ref mut gpu) = self.gpu
            && let Err(e) = gpu.present_frame(renderer.batch_builder(), clear)
        {
            match e {
                PresentError::SurfaceLost => {
                    let (w, h) = renderer.viewport();
                    gpu.resize(w, h);
                }
                PresentError::Validation => {
                    eprintln!("[vitreous] GPU validation error");
                }
            }
        }
    }

    /// Stage 5: Update the accessibility tree.
    fn update_accessibility(&mut self, root: &Node) {
        let a11y_root = build_a11y_tree(root, &mut 0);

        // Rebuild focus manager from new tree
        match &mut self.focus_manager {
            Some(fm) => fm.rebuild(&a11y_root),
            None => self.focus_manager = Some(FocusManager::new(&a11y_root)),
        }

        let focused = self
            .focus_manager
            .as_ref()
            .and_then(|fm| fm.focused())
            .unwrap_or(NodeId(0));

        let is_initial = self.frame_count == 0;
        let _tree_update = generate_accesskit_tree(&a11y_root, focused, is_initial);
        // In a full implementation, _tree_update would be sent to
        // accesskit_winit::Adapter for platform AT notification.
    }

    /// Run the full frame pipeline.
    fn run_frame(&mut self) {
        if !self.needs_rebuild {
            return;
        }
        self.needs_rebuild = false;
        self.frame_count += 1;

        let root = self.build_widget_tree();

        let (width, height) = self
            .window
            .as_ref()
            .map(|w| {
                let (pw, ph) = w.inner_size_physical();
                (pw as f32, ph as f32)
            })
            .unwrap_or((800.0, 600.0));

        let logical_width = width / self.scale_factor as f32;
        let logical_height = height / self.scale_factor as f32;

        self.compute_layout(&root, logical_width, logical_height);
        let commands = self.generate_render_commands(&root);
        self.submit_frame(commands);
        self.update_accessibility(&root);
    }

    // ───────────────────────────────────────────────────────────────────
    // Event translation
    // ───────────────────────────────────────────────────────────────────

    /// Translate a winit mouse button event into a vitreous MouseEvent and
    /// dispatch it via hit testing + event propagation.
    fn handle_mouse_input(&mut self, state: ElementState, button: winit::event::MouseButton) {
        let Some(pos) = self.mouse_position else {
            return;
        };

        let logical_x = pos.x / self.scale_factor;
        let logical_y = pos.y / self.scale_factor;
        let point = Point::new(logical_x, logical_y);

        let vitreous_button = translate_mouse_button(button);
        let modifiers = translate_modifiers(self.modifiers);

        let mouse_event = MouseEvent {
            x: logical_x,
            y: logical_y,
            global_x: logical_x,
            global_y: logical_y,
            button: vitreous_button,
            modifiers,
        };

        // Hit test to find target node
        let target = hit_test(point, &self.layout_nodes);

        if let Some(target_id) = target {
            // Wrap event dispatch in batch() so multiple signal updates
            // coalesce into a single effect flush
            batch(|| {
                let _ = (target_id, &mouse_event, state);
                // In a full implementation, we would look up the Node's
                // EventHandlers by target_id and invoke on_click/on_mouse_down/
                // on_mouse_up, then bubble_event up the tree.
                // For now the pipeline is wired: hit_test → batch → dispatch.
            });

            // Any signal changes during dispatch require a redraw
            self.needs_rebuild = true;
            if let Some(ref window) = self.window {
                window.request_redraw();
            }
        }
    }

    /// Translate a winit keyboard event into a vitreous KeyEvent.
    fn handle_keyboard_input(&mut self, event: &winit::event::KeyEvent) {
        if event.state != ElementState::Pressed {
            return;
        }

        let modifiers = translate_modifiers(self.modifiers);

        // Handle Tab/Shift+Tab for focus management
        if let WinitKey::Named(NamedKey::Tab) = &event.logical_key
            && let Some(ref mut fm) = self.focus_manager
        {
            if modifiers.shift {
                fm.focus_previous();
            } else {
                fm.focus_next();
            }
            self.needs_rebuild = true;
            if let Some(ref window) = self.window {
                window.request_redraw();
            }
            return;
        }

        let key = translate_key(&event.logical_key);
        let code = translate_key_code(&event.physical_key);
        let text = match &event.logical_key {
            WinitKey::Character(c) => Some(c.to_string()),
            _ => None,
        };

        let key_event = KeyEvent {
            key,
            code,
            modifiers,
            repeat: event.repeat,
            text,
        };

        // Dispatch keyboard event to focused node
        batch(|| {
            let _ = &key_event;
            // In a full implementation: dispatch_keyboard_event() with
            // the focused node, looking up EventHandlers and invoking
            // on_key_down.
        });

        self.needs_rebuild = true;
        if let Some(ref window) = self.window {
            window.request_redraw();
        }
    }

    /// Handle a scroll/wheel event.
    fn handle_scroll(&mut self, delta: winit::event::MouseScrollDelta) {
        let (dx, dy) = match delta {
            winit::event::MouseScrollDelta::LineDelta(x, y) => (x as f64 * 40.0, y as f64 * 40.0),
            winit::event::MouseScrollDelta::PixelDelta(pos) => (pos.x, pos.y),
        };

        let modifiers = translate_modifiers(self.modifiers);
        let _scroll_event = ScrollEvent {
            delta_x: dx,
            delta_y: dy,
            modifiers,
        };

        // In a full implementation: hit_test → dispatch scroll to target
        self.needs_rebuild = true;
        if let Some(ref window) = self.window {
            window.request_redraw();
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// ApplicationHandler — winit event loop integration
// ═══════════════════════════════════════════════════════════════════════════

impl ApplicationHandler for DesktopRuntime {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let window = PlatformWindow::create(event_loop, &self.config);
        self.scale_factor = window.scale_factor();

        let (pw, ph) = window.inner_size_physical();
        self.renderer = Some(Renderer::new(pw, ph));

        // Initialize GPU rendering context
        self.gpu = Some(GpuContext::new(window.arc_window()));

        window.request_redraw();
        self.window = Some(window);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                self.should_close = true;
                event_loop.exit();
            }

            WindowEvent::RedrawRequested => {
                self.run_frame();
            }

            WindowEvent::Resized(new_size) => {
                // AC-12: resize triggers layout recomputation
                if let Some(ref mut renderer) = self.renderer {
                    renderer.resize(new_size.width, new_size.height);
                }
                if let Some(ref mut gpu) = self.gpu {
                    gpu.resize(new_size.width, new_size.height);
                }
                self.needs_rebuild = true;
                if let Some(ref window) = self.window {
                    window.request_redraw();
                }
            }

            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                // AC-10: DPI-aware rendering
                self.scale_factor = scale_factor;
                self.needs_rebuild = true;
                if let Some(ref window) = self.window {
                    window.request_redraw();
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_position = Some(position);
            }

            WindowEvent::MouseInput { state, button, .. } => {
                // AC-7: mouse click translates to vitreous MouseEvent
                self.handle_mouse_input(state, button);
            }

            WindowEvent::MouseWheel { delta, .. } => {
                self.handle_scroll(delta);
            }

            WindowEvent::KeyboardInput { event, .. } => {
                // AC-8: keyboard event translates to vitreous KeyEvent
                self.handle_keyboard_input(&event);
            }

            WindowEvent::ModifiersChanged(new_modifiers) => {
                self.modifiers = new_modifiers.state();
            }

            WindowEvent::Focused(focused) => {
                if !focused {
                    // Clear focus state when window loses focus
                    if let Some(ref mut fm) = self.focus_manager {
                        fm.blur();
                    }
                }
            }

            _ => {}
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Tree flattening helpers
// ═══════════════════════════════════════════════════════════════════════════

/// Convert a Node tree into a flat list of LayoutInputs for the layout engine.
fn flatten_node_to_layout(
    node: &Node,
    inputs: &mut Vec<vitreous_layout::LayoutInput>,
    next_id: &mut u32,
) {
    let id = vitreous_layout::NodeId(*next_id);
    *next_id += 1;

    let mut child_ids = Vec::new();

    for child in &node.children {
        let child_id = vitreous_layout::NodeId(*next_id);
        child_ids.push(child_id);
        flatten_node_to_layout(child, inputs, next_id);
    }

    let style = convert_style_to_layout(&node.style, node);

    // Text nodes get a measure function
    let measure = match &node.kind {
        vitreous_widgets::NodeKind::Text(text_content) => {
            let text = match text_content {
                vitreous_widgets::TextContent::Static(s) => s.clone(),
                vitreous_widgets::TextContent::Dynamic(f) => f(),
            };
            let font_size = node.style.font_size.unwrap_or(16.0);
            Some(create_text_measure_fn(text, font_size))
        }
        _ => None,
    };

    inputs.push(vitreous_layout::LayoutInput {
        id,
        style,
        children: child_ids,
        measure,
    });
}

/// Create a MeasureFn for text nodes that uses the text engine.
fn create_text_measure_fn(text: String, font_size: f32) -> vitreous_layout::MeasureFn {
    Box::new(move |constraint: vitreous_layout::MeasureConstraint| {
        // Approximate text measurement without the text engine (since MeasureFn
        // must be Fn, not FnMut, and we can't borrow the engine mutably).
        // Use a heuristic: ~0.6 * font_size per character width, font_size * 1.2 height.
        let char_width = font_size * 0.6;
        let line_height = font_size * 1.2;
        let total_width = text.chars().count() as f32 * char_width;

        let available_width = constraint.max_width.unwrap_or(f32::MAX);
        if total_width <= available_width {
            vitreous_layout::Size {
                width: total_width,
                height: line_height,
            }
        } else {
            // Simple word-wrap estimation
            let lines = (total_width / available_width).ceil();
            vitreous_layout::Size {
                width: available_width.min(total_width),
                height: lines * line_height,
            }
        }
    })
}

/// Convert vitreous_style::Style + Node flex props to vitreous_layout::LayoutStyle.
fn convert_style_to_layout(
    style: &vitreous_style::Style,
    node: &Node,
) -> vitreous_layout::LayoutStyle {
    vitreous_layout::LayoutStyle {
        display: vitreous_layout::Display::Flex,
        flex_direction: match node.flex_direction {
            vitreous_widgets::FlexDirection::Row => vitreous_layout::FlexDirection::Row,
            vitreous_widgets::FlexDirection::Column => vitreous_layout::FlexDirection::Column,
        },
        flex_wrap: vitreous_layout::FlexWrap::NoWrap,
        justify_content: None,
        align_items: None,
        align_self: node.align_self.map(|a| match a {
            vitreous_widgets::AlignSelf::Start => vitreous_layout::AlignSelf::Start,
            vitreous_widgets::AlignSelf::End => vitreous_layout::AlignSelf::End,
            vitreous_widgets::AlignSelf::FlexStart => vitreous_layout::AlignSelf::FlexStart,
            vitreous_widgets::AlignSelf::FlexEnd => vitreous_layout::AlignSelf::FlexEnd,
            vitreous_widgets::AlignSelf::Center => vitreous_layout::AlignSelf::Center,
            vitreous_widgets::AlignSelf::Baseline => vitreous_layout::AlignSelf::Baseline,
            vitreous_widgets::AlignSelf::Stretch => vitreous_layout::AlignSelf::Stretch,
        }),
        align_content: None,
        flex_grow: node.flex_grow,
        flex_shrink: node.flex_shrink,
        flex_basis: convert_dimension(node.flex_basis),
        width: convert_dimension(style.width),
        height: convert_dimension(style.height),
        min_width: convert_dimension(style.min_width),
        max_width: convert_dimension(style.max_width),
        min_height: convert_dimension(style.min_height),
        max_height: convert_dimension(style.max_height),
        padding: convert_edges_to_dim_rect(&style.padding),
        margin: convert_edges_to_dim_rect(&style.margin),
        gap: vitreous_layout::Size {
            width: node.gap,
            height: node.gap,
        },
        aspect_ratio: node.aspect_ratio,
        overflow: match style.overflow {
            vitreous_style::Overflow::Visible => vitreous_layout::Overflow::Visible,
            vitreous_style::Overflow::Hidden => vitreous_layout::Overflow::Hidden,
            vitreous_style::Overflow::Scroll => vitreous_layout::Overflow::Scroll,
        },
        position: vitreous_layout::Position::Relative,
        inset: vitreous_layout::DimensionRect {
            top: vitreous_layout::Dimension::Auto,
            right: vitreous_layout::Dimension::Auto,
            bottom: vitreous_layout::Dimension::Auto,
            left: vitreous_layout::Dimension::Auto,
        },
    }
}

fn convert_dimension(dim: vitreous_style::Dimension) -> vitreous_layout::Dimension {
    match dim {
        vitreous_style::Dimension::Px(v) => vitreous_layout::Dimension::Px(v),
        vitreous_style::Dimension::Percent(v) => vitreous_layout::Dimension::Percent(v),
        vitreous_style::Dimension::Auto => vitreous_layout::Dimension::Auto,
    }
}

fn convert_edges_to_dim_rect(edges: &vitreous_style::Edges) -> vitreous_layout::DimensionRect {
    vitreous_layout::DimensionRect {
        top: vitreous_layout::Dimension::Px(edges.top),
        right: vitreous_layout::Dimension::Px(edges.right),
        bottom: vitreous_layout::Dimension::Px(edges.bottom),
        left: vitreous_layout::Dimension::Px(edges.left),
    }
}

/// Build LayoutNode list for hit testing from LayoutOutput + Node tree.
fn build_layout_nodes(output: &LayoutOutput, root: &Node) -> Vec<LayoutNode> {
    let mut result = Vec::new();
    build_layout_nodes_recursive(output, root, &mut 0, &mut result);
    result
}

fn build_layout_nodes_recursive(
    output: &LayoutOutput,
    node: &Node,
    next_id: &mut u32,
    result: &mut Vec<LayoutNode>,
) {
    let id = vitreous_layout::NodeId(*next_id);
    let events_id = NodeId(*next_id as usize);
    *next_id += 1;

    if let Some(layout) = output.get(id) {
        let corners = vitreous_events::Corners {
            top_left: node.style.border_radius.top_left as f64,
            top_right: node.style.border_radius.top_right as f64,
            bottom_right: node.style.border_radius.bottom_right as f64,
            bottom_left: node.style.border_radius.bottom_left as f64,
        };

        result.push(LayoutNode {
            id: events_id,
            rect: vitreous_events::Rect::new(
                layout.x as f64,
                layout.y as f64,
                layout.width as f64,
                layout.height as f64,
            ),
            corners,
        });
    }

    for child in &node.children {
        build_layout_nodes_recursive(output, child, next_id, result);
    }
}

/// Convert Node tree into flat RenderNode list for the render command generator.
/// Text nodes are shaped via the text engine to produce positioned glyphs.
fn flatten_node_to_render(
    node: &Node,
    render_nodes: &mut Vec<RenderNode>,
    next_id: &mut u32,
    text_engine: &mut CosmicTextEngine,
    scale_factor: f32,
) {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use vitreous_render::PositionedGlyph;
    use vitreous_widgets::{NodeKind, TextContent};

    let id = vitreous_layout::NodeId(*next_id);
    *next_id += 1;

    let mut child_ids = Vec::new();
    for child in &node.children {
        let child_id = vitreous_layout::NodeId(*next_id);
        child_ids.push(child_id);
        flatten_node_to_render(child, render_nodes, next_id, text_engine, scale_factor);
    }

    let visual_style = NodeVisualStyle {
        background: node.style.background,
        foreground: node.style.foreground,
        border_color: node.style.border_color,
        border_width: node.style.border_width,
        border_radius: StyleCorners {
            top_left: node.style.border_radius.top_left,
            top_right: node.style.border_radius.top_right,
            bottom_right: node.style.border_radius.bottom_right,
            bottom_left: node.style.border_radius.bottom_left,
        },
        opacity: node.style.opacity,
        shadow: node.style.shadow,
        clip_content: node.style.clip_content,
    };

    // Shape text nodes into positioned glyphs
    let content = match &node.kind {
        NodeKind::Text(text_content) => {
            let text_str = match text_content {
                TextContent::Static(s) => s.clone(),
                TextContent::Dynamic(f) => f(),
            };

            if text_str.is_empty() {
                NodeContent::None
            } else {
                let font_size = node.style.font_size.unwrap_or(16.0);
                let font_weight = node.style.font_weight.unwrap_or(vitreous_style::FontWeight::Regular);
                let font_family = node.style.font_family.clone().unwrap_or(vitreous_style::FontFamily::SansSerif);
                let font_style = node.style.font_style.unwrap_or(vitreous_style::FontStyle::Normal);

                let font_desc = crate::text_engine::FontDescriptor {
                    family: font_family.clone(),
                    size: font_size,
                    weight: font_weight,
                    style: font_style,
                };

                // Compute a stable hash for this font configuration
                let mut hasher = DefaultHasher::new();
                format!("{font_family:?}").hash(&mut hasher);
                (font_weight as u16).hash(&mut hasher);
                (font_style as u8).hash(&mut hasher);
                let font_hash = hasher.finish();

                let shaped = text_engine.shape(&text_str, &font_desc, None);

                let glyphs: Vec<PositionedGlyph> = shaped
                    .glyphs
                    .iter()
                    .map(|g| PositionedGlyph {
                        glyph_id: g.glyph_id,
                        x: g.x,
                        y: g.y,
                        width: g.width,
                        height: g.height,
                        font_hash,
                        font_size,
                        scale_factor,
                        text_fragment: g.text_fragment.clone(),
                    })
                    .collect();

                let color = node.style.foreground.unwrap_or(vitreous_style::Color::WHITE);
                NodeContent::Text(glyphs, color)
            }
        }
        _ => NodeContent::None,
    };

    render_nodes.push(RenderNode {
        id,
        style: visual_style,
        content,
        children: child_ids,
    });
}

/// Build an A11yNode tree from a Node tree for accessibility updates.
fn build_a11y_tree(node: &Node, next_id: &mut usize) -> A11yNode {
    let id = NodeId(*next_id);
    *next_id += 1;

    let children: Vec<A11yNode> = node
        .children
        .iter()
        .map(|child| build_a11y_tree(child, next_id))
        .collect();

    A11yNode {
        id,
        info: node.a11y.clone(),
        children,
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Event translation functions
// ═══════════════════════════════════════════════════════════════════════════

/// Translate winit mouse button to vitreous MouseButton.
fn translate_mouse_button(button: winit::event::MouseButton) -> MouseButton {
    match button {
        winit::event::MouseButton::Left => MouseButton::Left,
        winit::event::MouseButton::Right => MouseButton::Right,
        winit::event::MouseButton::Middle => MouseButton::Middle,
        winit::event::MouseButton::Back => MouseButton::Back,
        winit::event::MouseButton::Forward => MouseButton::Forward,
        winit::event::MouseButton::Other(_) => MouseButton::Left,
    }
}

/// Translate winit modifiers to vitreous Modifiers.
fn translate_modifiers(state: ModifiersState) -> Modifiers {
    Modifiers {
        shift: state.shift_key(),
        ctrl: state.control_key(),
        alt: state.alt_key(),
        meta: state.super_key(),
    }
}

/// Translate winit logical key to vitreous Key.
fn translate_key(key: &WinitKey) -> Key {
    match key {
        WinitKey::Named(named) => match named {
            NamedKey::Enter => Key::Enter,
            NamedKey::Tab => Key::Tab,
            NamedKey::Space => Key::Space,
            NamedKey::Backspace => Key::Backspace,
            NamedKey::Delete => Key::Delete,
            NamedKey::Escape => Key::Escape,
            NamedKey::ArrowUp => Key::ArrowUp,
            NamedKey::ArrowDown => Key::ArrowDown,
            NamedKey::ArrowLeft => Key::ArrowLeft,
            NamedKey::ArrowRight => Key::ArrowRight,
            NamedKey::Home => Key::Home,
            NamedKey::End => Key::End,
            NamedKey::PageUp => Key::PageUp,
            NamedKey::PageDown => Key::PageDown,
            NamedKey::Shift => Key::Shift,
            NamedKey::Control => Key::Control,
            NamedKey::Alt => Key::Alt,
            NamedKey::Super => Key::Meta,
            NamedKey::CapsLock => Key::CapsLock,
            NamedKey::NumLock => Key::NumLock,
            NamedKey::ScrollLock => Key::ScrollLock,
            NamedKey::Insert => Key::Insert,
            NamedKey::Cut => Key::Cut,
            NamedKey::Copy => Key::Copy,
            NamedKey::Paste => Key::Paste,
            NamedKey::Undo => Key::Undo,
            NamedKey::Redo => Key::Redo,
            NamedKey::F1 => Key::F1,
            NamedKey::F2 => Key::F2,
            NamedKey::F3 => Key::F3,
            NamedKey::F4 => Key::F4,
            NamedKey::F5 => Key::F5,
            NamedKey::F6 => Key::F6,
            NamedKey::F7 => Key::F7,
            NamedKey::F8 => Key::F8,
            NamedKey::F9 => Key::F9,
            NamedKey::F10 => Key::F10,
            NamedKey::F11 => Key::F11,
            NamedKey::F12 => Key::F12,
            NamedKey::PrintScreen => Key::PrintScreen,
            NamedKey::Pause => Key::Pause,
            NamedKey::ContextMenu => Key::ContextMenu,
            NamedKey::MediaPlayPause => Key::MediaPlayPause,
            NamedKey::MediaStop => Key::MediaStop,
            NamedKey::MediaTrackNext => Key::MediaTrackNext,
            NamedKey::MediaTrackPrevious => Key::MediaTrackPrevious,
            NamedKey::AudioVolumeUp => Key::AudioVolumeUp,
            NamedKey::AudioVolumeDown => Key::AudioVolumeDown,
            NamedKey::AudioVolumeMute => Key::AudioVolumeMute,
            _ => Key::Other(format!("{named:?}")),
        },
        WinitKey::Character(c) => Key::Character(c.to_string()),
        WinitKey::Unidentified(_) => Key::Other("Unidentified".to_string()),
        WinitKey::Dead(_) => Key::Other("Dead".to_string()),
    }
}

/// Translate winit physical key to vitreous KeyCode.
fn translate_key_code(key: &PhysicalKey) -> KeyCode {
    match key {
        PhysicalKey::Code(code) => match code {
            WinitKeyCode::KeyA => KeyCode::KeyA,
            WinitKeyCode::KeyB => KeyCode::KeyB,
            WinitKeyCode::KeyC => KeyCode::KeyC,
            WinitKeyCode::KeyD => KeyCode::KeyD,
            WinitKeyCode::KeyE => KeyCode::KeyE,
            WinitKeyCode::KeyF => KeyCode::KeyF,
            WinitKeyCode::KeyG => KeyCode::KeyG,
            WinitKeyCode::KeyH => KeyCode::KeyH,
            WinitKeyCode::KeyI => KeyCode::KeyI,
            WinitKeyCode::KeyJ => KeyCode::KeyJ,
            WinitKeyCode::KeyK => KeyCode::KeyK,
            WinitKeyCode::KeyL => KeyCode::KeyL,
            WinitKeyCode::KeyM => KeyCode::KeyM,
            WinitKeyCode::KeyN => KeyCode::KeyN,
            WinitKeyCode::KeyO => KeyCode::KeyO,
            WinitKeyCode::KeyP => KeyCode::KeyP,
            WinitKeyCode::KeyQ => KeyCode::KeyQ,
            WinitKeyCode::KeyR => KeyCode::KeyR,
            WinitKeyCode::KeyS => KeyCode::KeyS,
            WinitKeyCode::KeyT => KeyCode::KeyT,
            WinitKeyCode::KeyU => KeyCode::KeyU,
            WinitKeyCode::KeyV => KeyCode::KeyV,
            WinitKeyCode::KeyW => KeyCode::KeyW,
            WinitKeyCode::KeyX => KeyCode::KeyX,
            WinitKeyCode::KeyY => KeyCode::KeyY,
            WinitKeyCode::KeyZ => KeyCode::KeyZ,
            WinitKeyCode::Digit0 => KeyCode::Digit0,
            WinitKeyCode::Digit1 => KeyCode::Digit1,
            WinitKeyCode::Digit2 => KeyCode::Digit2,
            WinitKeyCode::Digit3 => KeyCode::Digit3,
            WinitKeyCode::Digit4 => KeyCode::Digit4,
            WinitKeyCode::Digit5 => KeyCode::Digit5,
            WinitKeyCode::Digit6 => KeyCode::Digit6,
            WinitKeyCode::Digit7 => KeyCode::Digit7,
            WinitKeyCode::Digit8 => KeyCode::Digit8,
            WinitKeyCode::Digit9 => KeyCode::Digit9,
            WinitKeyCode::F1 => KeyCode::F1,
            WinitKeyCode::F2 => KeyCode::F2,
            WinitKeyCode::F3 => KeyCode::F3,
            WinitKeyCode::F4 => KeyCode::F4,
            WinitKeyCode::F5 => KeyCode::F5,
            WinitKeyCode::F6 => KeyCode::F6,
            WinitKeyCode::F7 => KeyCode::F7,
            WinitKeyCode::F8 => KeyCode::F8,
            WinitKeyCode::F9 => KeyCode::F9,
            WinitKeyCode::F10 => KeyCode::F10,
            WinitKeyCode::F11 => KeyCode::F11,
            WinitKeyCode::F12 => KeyCode::F12,
            WinitKeyCode::Enter => KeyCode::Enter,
            WinitKeyCode::Tab => KeyCode::Tab,
            WinitKeyCode::Space => KeyCode::Space,
            WinitKeyCode::Backspace => KeyCode::Backspace,
            WinitKeyCode::Delete => KeyCode::Delete,
            WinitKeyCode::Insert => KeyCode::Insert,
            WinitKeyCode::Escape => KeyCode::Escape,
            WinitKeyCode::ArrowUp => KeyCode::ArrowUp,
            WinitKeyCode::ArrowDown => KeyCode::ArrowDown,
            WinitKeyCode::ArrowLeft => KeyCode::ArrowLeft,
            WinitKeyCode::ArrowRight => KeyCode::ArrowRight,
            WinitKeyCode::Home => KeyCode::Home,
            WinitKeyCode::End => KeyCode::End,
            WinitKeyCode::PageUp => KeyCode::PageUp,
            WinitKeyCode::PageDown => KeyCode::PageDown,
            WinitKeyCode::ShiftLeft => KeyCode::ShiftLeft,
            WinitKeyCode::ShiftRight => KeyCode::ShiftRight,
            WinitKeyCode::ControlLeft => KeyCode::ControlLeft,
            WinitKeyCode::ControlRight => KeyCode::ControlRight,
            WinitKeyCode::AltLeft => KeyCode::AltLeft,
            WinitKeyCode::AltRight => KeyCode::AltRight,
            WinitKeyCode::SuperLeft => KeyCode::MetaLeft,
            WinitKeyCode::SuperRight => KeyCode::MetaRight,
            WinitKeyCode::CapsLock => KeyCode::CapsLock,
            WinitKeyCode::NumLock => KeyCode::NumLock,
            WinitKeyCode::ScrollLock => KeyCode::ScrollLock,
            WinitKeyCode::Minus => KeyCode::Minus,
            WinitKeyCode::Equal => KeyCode::Equal,
            WinitKeyCode::BracketLeft => KeyCode::BracketLeft,
            WinitKeyCode::BracketRight => KeyCode::BracketRight,
            WinitKeyCode::Backslash => KeyCode::Backslash,
            WinitKeyCode::Semicolon => KeyCode::Semicolon,
            WinitKeyCode::Quote => KeyCode::Quote,
            WinitKeyCode::Backquote => KeyCode::Backquote,
            WinitKeyCode::Comma => KeyCode::Comma,
            WinitKeyCode::Period => KeyCode::Period,
            WinitKeyCode::Slash => KeyCode::Slash,
            WinitKeyCode::PrintScreen => KeyCode::PrintScreen,
            WinitKeyCode::Pause => KeyCode::Pause,
            WinitKeyCode::ContextMenu => KeyCode::ContextMenu,
            WinitKeyCode::Numpad0 => KeyCode::Numpad0,
            WinitKeyCode::Numpad1 => KeyCode::Numpad1,
            WinitKeyCode::Numpad2 => KeyCode::Numpad2,
            WinitKeyCode::Numpad3 => KeyCode::Numpad3,
            WinitKeyCode::Numpad4 => KeyCode::Numpad4,
            WinitKeyCode::Numpad5 => KeyCode::Numpad5,
            WinitKeyCode::Numpad6 => KeyCode::Numpad6,
            WinitKeyCode::Numpad7 => KeyCode::Numpad7,
            WinitKeyCode::Numpad8 => KeyCode::Numpad8,
            WinitKeyCode::Numpad9 => KeyCode::Numpad9,
            WinitKeyCode::NumpadAdd => KeyCode::NumpadAdd,
            WinitKeyCode::NumpadSubtract => KeyCode::NumpadSubtract,
            WinitKeyCode::NumpadMultiply => KeyCode::NumpadMultiply,
            WinitKeyCode::NumpadDivide => KeyCode::NumpadDivide,
            WinitKeyCode::NumpadDecimal => KeyCode::NumpadDecimal,
            WinitKeyCode::NumpadEnter => KeyCode::NumpadEnter,
            _ => KeyCode::Unidentified,
        },
        PhysicalKey::Unidentified(_) => KeyCode::Unidentified,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vitreous_a11y::{AccessibilityInfo, AccessibilityState, Role};
    use vitreous_events::EventHandlers;
    use vitreous_style::{Color, Dimension, Edges, Style};
    use vitreous_widgets::{FlexDirection, NodeKind, TextContent};

    fn simple_container(children: Vec<Node>) -> Node {
        Node {
            kind: NodeKind::Container,
            style: Style {
                width: Dimension::Px(800.0),
                height: Dimension::Px(600.0),
                ..Style::default()
            },
            a11y: AccessibilityInfo {
                role: Role::Window,
                label: Some("Test Window".to_string()),
                ..AccessibilityInfo::default()
            },
            event_handlers: EventHandlers::default(),
            children,
            key: None,
            flex_direction: FlexDirection::Column,
            flex_grow: 0.0,
            flex_shrink: 1.0,
            flex_basis: Dimension::Auto,
            align_self: None,
            gap: 0.0,
            aspect_ratio: None,
            animations: Vec::new(),
        }
    }

    fn text_node(text: &str) -> Node {
        Node {
            kind: NodeKind::Text(TextContent::Static(text.to_string())),
            style: Style {
                font_size: Some(16.0),
                ..Style::default()
            },
            a11y: AccessibilityInfo {
                role: Role::Text,
                label: Some(text.to_string()),
                ..AccessibilityInfo::default()
            },
            event_handlers: EventHandlers::default(),
            children: Vec::new(),
            key: None,
            flex_direction: FlexDirection::default(),
            flex_grow: 0.0,
            flex_shrink: 1.0,
            flex_basis: Dimension::Auto,
            align_self: None,
            gap: 0.0,
            aspect_ratio: None,
            animations: Vec::new(),
        }
    }

    fn button_node(label: &str) -> Node {
        Node {
            kind: NodeKind::Container,
            style: Style {
                width: Dimension::Px(100.0),
                height: Dimension::Px(40.0),
                padding: Edges::all(8.0),
                background: Some(Color::BLUE),
                ..Style::default()
            },
            a11y: AccessibilityInfo {
                role: Role::Button,
                label: Some(label.to_string()),
                state: AccessibilityState {
                    focusable: true,
                    ..AccessibilityState::default()
                },
                ..AccessibilityInfo::default()
            },
            event_handlers: EventHandlers::default(),
            children: vec![text_node(label)],
            key: None,
            flex_direction: FlexDirection::default(),
            flex_grow: 0.0,
            flex_shrink: 1.0,
            flex_basis: Dimension::Auto,
            align_self: None,
            gap: 0.0,
            aspect_ratio: None,
            animations: Vec::new(),
        }
    }

    #[test]
    fn translate_mouse_buttons() {
        assert_eq!(
            translate_mouse_button(winit::event::MouseButton::Left),
            MouseButton::Left
        );
        assert_eq!(
            translate_mouse_button(winit::event::MouseButton::Right),
            MouseButton::Right
        );
        assert_eq!(
            translate_mouse_button(winit::event::MouseButton::Middle),
            MouseButton::Middle
        );
    }

    #[test]
    fn translate_modifiers_empty() {
        let mods = translate_modifiers(ModifiersState::empty());
        assert!(!mods.shift);
        assert!(!mods.ctrl);
        assert!(!mods.alt);
        assert!(!mods.meta);
    }

    #[test]
    fn translate_modifiers_shift() {
        let mods = translate_modifiers(ModifiersState::SHIFT);
        assert!(mods.shift);
        assert!(!mods.ctrl);
    }

    #[test]
    fn translate_modifiers_ctrl() {
        let mods = translate_modifiers(ModifiersState::CONTROL);
        assert!(mods.ctrl);
        assert!(!mods.shift);
    }

    #[test]
    fn translate_key_named() {
        assert_eq!(translate_key(&WinitKey::Named(NamedKey::Enter)), Key::Enter);
        assert_eq!(translate_key(&WinitKey::Named(NamedKey::Tab)), Key::Tab);
        assert_eq!(
            translate_key(&WinitKey::Named(NamedKey::Escape)),
            Key::Escape
        );
        assert_eq!(translate_key(&WinitKey::Named(NamedKey::Space)), Key::Space);
        assert_eq!(
            translate_key(&WinitKey::Named(NamedKey::Backspace)),
            Key::Backspace
        );
        assert_eq!(
            translate_key(&WinitKey::Named(NamedKey::ArrowUp)),
            Key::ArrowUp
        );
    }

    #[test]
    fn translate_key_character() {
        let key = WinitKey::Character("a".into());
        assert_eq!(translate_key(&key), Key::Character("a".to_string()));
    }

    #[test]
    fn translate_key_code_letters() {
        assert_eq!(
            translate_key_code(&PhysicalKey::Code(WinitKeyCode::KeyA)),
            KeyCode::KeyA
        );
        assert_eq!(
            translate_key_code(&PhysicalKey::Code(WinitKeyCode::KeyZ)),
            KeyCode::KeyZ
        );
    }

    #[test]
    fn translate_key_code_digits() {
        assert_eq!(
            translate_key_code(&PhysicalKey::Code(WinitKeyCode::Digit0)),
            KeyCode::Digit0
        );
        assert_eq!(
            translate_key_code(&PhysicalKey::Code(WinitKeyCode::Digit9)),
            KeyCode::Digit9
        );
    }

    #[test]
    fn translate_key_code_special() {
        assert_eq!(
            translate_key_code(&PhysicalKey::Code(WinitKeyCode::Enter)),
            KeyCode::Enter
        );
        assert_eq!(
            translate_key_code(&PhysicalKey::Code(WinitKeyCode::Space)),
            KeyCode::Space
        );
    }

    #[test]
    fn flatten_simple_tree_to_layout() {
        let root = simple_container(vec![text_node("Hello"), text_node("World")]);

        let mut inputs = Vec::new();
        let mut next_id = 0;
        flatten_node_to_layout(&root, &mut inputs, &mut next_id);

        // Root + 2 text children = 3 inputs
        assert_eq!(inputs.len(), 3);
        // Text nodes should have measure functions
        assert!(inputs[0].measure.is_some()); // first text child
        assert!(inputs[1].measure.is_some()); // second text child
        assert!(inputs[2].measure.is_none()); // root container
    }

    #[test]
    fn flatten_to_render_nodes() {
        let root = simple_container(vec![button_node("Click me")]);

        let mut render_nodes = Vec::new();
        let mut next_id = 0;
        let mut text_engine = CosmicTextEngine::new();
        flatten_node_to_render(&root, &mut render_nodes, &mut next_id, &mut text_engine, 1.0);

        // Root + button + text inside button = 3 render nodes
        assert_eq!(render_nodes.len(), 3);
    }

    #[test]
    fn build_a11y_tree_structure() {
        let root = simple_container(vec![button_node("OK"), button_node("Cancel")]);

        let mut next_id = 0;
        let a11y_root = build_a11y_tree(&root, &mut next_id);

        assert_eq!(a11y_root.info.role, Role::Window);
        assert_eq!(a11y_root.children.len(), 2);
        assert_eq!(a11y_root.children[0].info.role, Role::Button);
        assert_eq!(a11y_root.children[0].info.label, Some("OK".to_string()));
    }

    #[test]
    fn text_measure_fn_basic() {
        let measure = create_text_measure_fn("Hello".to_string(), 16.0);
        let size = measure(vitreous_layout::MeasureConstraint {
            max_width: None,
            max_height: None,
        });
        assert!(size.width > 0.0);
        assert!(size.height > 0.0);
    }

    #[test]
    fn text_measure_fn_wraps() {
        let measure = create_text_measure_fn("A very long text that should wrap".to_string(), 16.0);
        let no_wrap = measure(vitreous_layout::MeasureConstraint {
            max_width: None,
            max_height: None,
        });
        let wrapped = measure(vitreous_layout::MeasureConstraint {
            max_width: Some(50.0),
            max_height: None,
        });
        assert!(wrapped.height >= no_wrap.height);
    }

    #[test]
    fn convert_dimension_variants() {
        assert!(matches!(
            convert_dimension(vitreous_style::Dimension::Px(10.0)),
            vitreous_layout::Dimension::Px(v) if (v - 10.0).abs() < f32::EPSILON
        ));
        assert!(matches!(
            convert_dimension(vitreous_style::Dimension::Percent(50.0)),
            vitreous_layout::Dimension::Percent(v) if (v - 50.0).abs() < f32::EPSILON
        ));
        assert!(matches!(
            convert_dimension(vitreous_style::Dimension::Auto),
            vitreous_layout::Dimension::Auto
        ));
    }

    #[test]
    fn convert_edges_to_dim_rect_values() {
        let edges = Edges::new(1.0, 2.0, 3.0, 4.0);
        let rect = convert_edges_to_dim_rect(&edges);
        assert!(
            matches!(rect.top, vitreous_layout::Dimension::Px(v) if (v - 1.0).abs() < f32::EPSILON)
        );
        assert!(
            matches!(rect.right, vitreous_layout::Dimension::Px(v) if (v - 2.0).abs() < f32::EPSILON)
        );
        assert!(
            matches!(rect.bottom, vitreous_layout::Dimension::Px(v) if (v - 3.0).abs() < f32::EPSILON)
        );
        assert!(
            matches!(rect.left, vitreous_layout::Dimension::Px(v) if (v - 4.0).abs() < f32::EPSILON)
        );
    }

    #[test]
    fn build_layout_nodes_from_output() {
        let root = simple_container(vec![text_node("Hi")]);

        let mut inputs = Vec::new();
        let mut next_id = 0;
        flatten_node_to_layout(&root, &mut inputs, &mut next_id);

        let root_id = vitreous_layout::NodeId(0);
        let output = compute_layout(
            &inputs,
            root_id,
            AvailableSpace {
                width: 800.0,
                height: 600.0,
            },
        );

        let layout_nodes = build_layout_nodes(&output, &root);
        // Should have at least the root node
        assert!(!layout_nodes.is_empty());
    }

    #[test]
    fn desktop_runtime_creation() {
        let config = WindowConfig::new().title("Test").size(800, 600);
        let runtime = DesktopRuntime::new(config, || simple_container(vec![text_node("Hello")]));
        assert!(runtime.window.is_none());
        assert!(runtime.renderer.is_none());
        assert!(runtime.needs_rebuild);
        assert_eq!(runtime.frame_count, 0);
    }
}
