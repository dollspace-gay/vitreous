use std::sync::Arc;
use vitreous_render::pipeline::{
    BatchBuilder, BatchKind, Globals, GlyphInstance, RectInstance, ShadowInstance,
};
use wgpu::util::DeviceExt;

/// Errors from frame presentation.
#[derive(Debug)]
pub enum PresentError {
    SurfaceLost,
    Validation,
}

/// GPU rendering context — owns the wgpu device, surface, pipelines, and buffers.
pub struct GpuContext {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    rect_pipeline: wgpu::RenderPipeline,
    text_pipeline: wgpu::RenderPipeline,
    shadow_pipeline: wgpu::RenderPipeline,
    globals_buffer: wgpu::Buffer,
    globals_bind_group: wgpu::BindGroup,
    #[allow(dead_code)]
    globals_bind_group_layout: wgpu::BindGroupLayout,
    #[allow(dead_code)]
    glyph_atlas_texture: wgpu::Texture,
    #[allow(dead_code)]
    glyph_atlas_view: wgpu::TextureView,
    glyph_atlas_bind_group: wgpu::BindGroup,
    #[allow(dead_code)]
    text_bind_group_layout: wgpu::BindGroupLayout,
}

impl GpuContext {
    /// Initialize wgpu and create all render pipelines.
    pub fn new(window: Arc<winit::window::Window>) -> Self {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            flags: wgpu::InstanceFlags::default(),
            memory_budget_thresholds: wgpu::MemoryBudgetThresholds::default(),
            backend_options: wgpu::BackendOptions::default(),
            display: None,
        });

        let surface = instance
            .create_surface(window.clone())
            .expect("failed to create wgpu surface");

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .expect("failed to find a suitable GPU adapter");

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("vitreous"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                ..Default::default()
            },
        ))
        .expect("failed to create wgpu device");

        let size = window.inner_size();
        let capabilities = surface.get_capabilities(&adapter);
        let format = capabilities
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(capabilities.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: capabilities.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        // ── Globals uniform ──────────────────────────────────────────────
        let globals = Globals {
            viewport_size: [size.width as f32, size.height as f32],
            _pad: [0.0; 2],
        };
        let globals_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("globals"),
            contents: bytemuck::cast_slice(&[globals]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let globals_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("globals_layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let globals_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("globals_bg"),
            layout: &globals_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: globals_buffer.as_entire_binding(),
            }],
        });

        // ── Glyph atlas texture ──────────────────────────────────────────
        let text_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("text_layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let (glyph_atlas_texture, glyph_atlas_view, glyph_atlas_bind_group) =
            create_atlas_texture(&device, &text_bind_group_layout);

        // ── Pipelines ────────────────────────────────────────────────────
        let rect_pipeline = create_rect_pipeline(&device, format, &globals_bind_group_layout);
        let text_pipeline =
            create_text_pipeline(&device, format, &globals_bind_group_layout, &text_bind_group_layout);
        let shadow_pipeline = create_shadow_pipeline(&device, format, &globals_bind_group_layout);

        Self {
            surface,
            device,
            queue,
            config,
            rect_pipeline,
            text_pipeline,
            shadow_pipeline,
            globals_buffer,
            globals_bind_group,
            globals_bind_group_layout,
            glyph_atlas_texture,
            glyph_atlas_view,
            glyph_atlas_bind_group,
            text_bind_group_layout,
        }
    }

    /// Resize the GPU surface with physical pixel dimensions.
    ///
    /// This reconfigures the wgpu surface for the new physical size but does
    /// NOT update the globals uniform. Call [`set_logical_size`] separately
    /// to update the viewport used by shaders.
    pub fn resize(&mut self, physical_width: u32, physical_height: u32) {
        if physical_width == 0 || physical_height == 0 {
            return;
        }
        self.config.width = physical_width;
        self.config.height = physical_height;
        self.surface.configure(&self.device, &self.config);
    }

    /// Update the globals uniform with logical pixel dimensions.
    ///
    /// The shader converts positions from logical pixels to NDC using
    /// `viewport_size`, so this must match the coordinate system used by the
    /// layout engine (logical pixels).
    pub fn set_logical_size(&mut self, logical_width: u32, logical_height: u32) {
        let globals = Globals {
            viewport_size: [logical_width as f32, logical_height as f32],
            _pad: [0.0; 2],
        };
        self.queue
            .write_buffer(&self.globals_buffer, 0, bytemuck::cast_slice(&[globals]));
    }

    /// Render a frame from the batch builder and present to the surface.
    pub fn present_frame(
        &mut self,
        batch_builder: &BatchBuilder,
        clear_color: [f32; 4],
    ) -> Result<(), PresentError> {
        let surface_texture = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(tex)
            | wgpu::CurrentSurfaceTexture::Suboptimal(tex) => tex,
            wgpu::CurrentSurfaceTexture::Timeout
            | wgpu::CurrentSurfaceTexture::Occluded => return Ok(()),
            wgpu::CurrentSurfaceTexture::Outdated
            | wgpu::CurrentSurfaceTexture::Lost => return Err(PresentError::SurfaceLost),
            wgpu::CurrentSurfaceTexture::Validation => return Err(PresentError::Validation),
        };

        let view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame"),
            });

        // Upload instance buffers
        let rect_buf = if !batch_builder.rect_instances.is_empty() {
            Some(self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("rects"),
                contents: bytemuck::cast_slice(&batch_builder.rect_instances),
                usage: wgpu::BufferUsages::VERTEX,
            }))
        } else {
            None
        };

        let glyph_buf = if !batch_builder.glyph_instances.is_empty() {
            Some(self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("glyphs"),
                contents: bytemuck::cast_slice(&batch_builder.glyph_instances),
                usage: wgpu::BufferUsages::VERTEX,
            }))
        } else {
            None
        };

        let shadow_buf = if !batch_builder.shadow_instances.is_empty() {
            Some(self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("shadows"),
                contents: bytemuck::cast_slice(&batch_builder.shadow_instances),
                usage: wgpu::BufferUsages::VERTEX,
            }))
        } else {
            None
        };

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("main"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: clear_color[0] as f64,
                            g: clear_color[1] as f64,
                            b: clear_color[2] as f64,
                            a: clear_color[3] as f64,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            let mut rect_offset: u32 = 0;
            let mut glyph_offset: u32 = 0;
            let mut shadow_offset: u32 = 0;

            for batch in &batch_builder.batches {
                match batch.kind {
                    BatchKind::Rect => {
                        if let Some(ref buf) = rect_buf {
                            pass.set_pipeline(&self.rect_pipeline);
                            pass.set_bind_group(0, Some(&self.globals_bind_group), &[]);
                            pass.set_vertex_buffer(0, buf.slice(..));
                            pass.draw(0..6, rect_offset..rect_offset + batch.instance_count);
                            rect_offset += batch.instance_count;
                        }
                    }
                    BatchKind::Text => {
                        if let Some(ref buf) = glyph_buf {
                            pass.set_pipeline(&self.text_pipeline);
                            pass.set_bind_group(0, Some(&self.globals_bind_group), &[]);
                            pass.set_bind_group(1, Some(&self.glyph_atlas_bind_group), &[]);
                            pass.set_vertex_buffer(0, buf.slice(..));
                            pass.draw(0..6, glyph_offset..glyph_offset + batch.instance_count);
                            glyph_offset += batch.instance_count;
                        }
                    }
                    BatchKind::Shadow => {
                        if let Some(ref buf) = shadow_buf {
                            pass.set_pipeline(&self.shadow_pipeline);
                            pass.set_bind_group(0, Some(&self.globals_bind_group), &[]);
                            pass.set_vertex_buffer(0, buf.slice(..));
                            pass.draw(0..6, shadow_offset..shadow_offset + batch.instance_count);
                            shadow_offset += batch.instance_count;
                        }
                    }
                    _ => {}
                }
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        surface_texture.present();
        Ok(())
    }

    /// Upload glyph bitmap data to a region of the atlas texture.
    pub fn upload_glyph(&self, data: &[u8], x: u32, y: u32, width: u32, height: u32) {
        if width == 0 || height == 0 || data.is_empty() {
            return;
        }
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.glyph_atlas_texture,
                mip_level: 0,
                origin: wgpu::Origin3d { x, y, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width),
                rows_per_image: None,
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
    }
}

// ── Pipeline creation helpers ────────────────────────────────────────────

fn create_atlas_texture(
    device: &wgpu::Device,
    text_layout: &wgpu::BindGroupLayout,
) -> (wgpu::Texture, wgpu::TextureView, wgpu::BindGroup) {
    let size = wgpu::Extent3d {
        width: 2048,
        height: 2048,
        depth_or_array_layers: 1,
    };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("glyph_atlas"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::R8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("glyph_sampler"),
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("glyph_atlas_bg"),
        layout: text_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
    });
    (texture, view, bind_group)
}

fn create_rect_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    globals_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("rect_shader"),
        source: wgpu::ShaderSource::Wgsl(
            include_str!("../../vitreous_render/src/shaders/rect.wgsl").into(),
        ),
    });

    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("rect_pl"),
        bind_group_layouts: &[Some(globals_layout)],
        immediate_size: 0,
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("rect_pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<RectInstance>() as u64,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &[
                    wgpu::VertexAttribute { offset: 0, shader_location: 0, format: wgpu::VertexFormat::Float32x2 },
                    wgpu::VertexAttribute { offset: 8, shader_location: 1, format: wgpu::VertexFormat::Float32x2 },
                    wgpu::VertexAttribute { offset: 16, shader_location: 2, format: wgpu::VertexFormat::Float32x4 },
                    wgpu::VertexAttribute { offset: 32, shader_location: 3, format: wgpu::VertexFormat::Float32x4 },
                    wgpu::VertexAttribute { offset: 48, shader_location: 4, format: wgpu::VertexFormat::Float32x4 },
                    wgpu::VertexAttribute { offset: 64, shader_location: 5, format: wgpu::VertexFormat::Float32 },
                ],
            }],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            ..Default::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    })
}

fn create_text_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    globals_layout: &wgpu::BindGroupLayout,
    text_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("text_shader"),
        source: wgpu::ShaderSource::Wgsl(
            include_str!("../../vitreous_render/src/shaders/text.wgsl").into(),
        ),
    });

    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("text_pl"),
        bind_group_layouts: &[Some(globals_layout), Some(text_layout)],
        immediate_size: 0,
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("text_pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<GlyphInstance>() as u64,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &[
                    wgpu::VertexAttribute { offset: 0, shader_location: 0, format: wgpu::VertexFormat::Float32x2 },
                    wgpu::VertexAttribute { offset: 8, shader_location: 1, format: wgpu::VertexFormat::Float32x2 },
                    wgpu::VertexAttribute { offset: 16, shader_location: 2, format: wgpu::VertexFormat::Float32x2 },
                    wgpu::VertexAttribute { offset: 24, shader_location: 3, format: wgpu::VertexFormat::Float32x2 },
                    wgpu::VertexAttribute { offset: 32, shader_location: 4, format: wgpu::VertexFormat::Float32x4 },
                ],
            }],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            ..Default::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    })
}

fn create_shadow_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    globals_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("shadow_shader"),
        source: wgpu::ShaderSource::Wgsl(
            include_str!("../../vitreous_render/src/shaders/shadow.wgsl").into(),
        ),
    });

    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("shadow_pl"),
        bind_group_layouts: &[Some(globals_layout)],
        immediate_size: 0,
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("shadow_pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<ShadowInstance>() as u64,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &[
                    wgpu::VertexAttribute { offset: 0, shader_location: 0, format: wgpu::VertexFormat::Float32x2 },
                    wgpu::VertexAttribute { offset: 8, shader_location: 1, format: wgpu::VertexFormat::Float32x2 },
                    wgpu::VertexAttribute { offset: 16, shader_location: 2, format: wgpu::VertexFormat::Float32x2 },
                    wgpu::VertexAttribute { offset: 24, shader_location: 3, format: wgpu::VertexFormat::Float32x2 },
                    wgpu::VertexAttribute { offset: 32, shader_location: 4, format: wgpu::VertexFormat::Float32x4 },
                    wgpu::VertexAttribute { offset: 48, shader_location: 5, format: wgpu::VertexFormat::Float32x4 },
                    wgpu::VertexAttribute { offset: 64, shader_location: 6, format: wgpu::VertexFormat::Float32 },
                ],
            }],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            ..Default::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    })
}
