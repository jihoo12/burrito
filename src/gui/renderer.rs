use glam::Mat4;
use glyphon::{
    Attrs, Buffer as GlyphBuffer, Cache as GlyphCache, Color as GlyphColor, Family, FontSystem,
    Metrics, Resolution, Shaping, SwashCache, TextArea, TextAtlas, TextBounds, TextRenderer,
    Viewport,
};
use wgpu::util::DeviceExt;

use super::context::{GuiContext, GuiVertex};

const SHADER: &str = r#"
    struct Uniforms { projection: mat4x4<f32> }
    @group(0) @binding(0) var<uniform> uniforms: Uniforms;

    struct VIn { @location(0) pos: vec2<f32>, @location(1) col: vec4<f32> }
    struct VOut { @builtin(position) pos: vec4<f32>, @location(0) col: vec4<f32> }

    @vertex
    fn vs_main(in: VIn) -> VOut {
        return VOut(uniforms.projection * vec4<f32>(in.pos, 0.0, 1.0), in.col);
    }

    @fragment
    fn fs_main(in: VOut) -> @location(0) vec4<f32> {
        return in.col;
    }
"#;

pub struct GuiRenderer {
    pipeline: wgpu::RenderPipeline,
    uniform_buf: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    vertex_buf: wgpu::Buffer,
    index_buf: wgpu::Buffer,
    buf_vertex_cap: usize,
    buf_index_cap: usize,

    font_system: FontSystem,
    swash_cache: SwashCache,
    #[allow(dead_code)]
    glyph_cache: GlyphCache,
    text_atlas: TextAtlas,
    text_renderer: TextRenderer,
    viewport: Viewport,
}

impl GuiRenderer {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Gui Shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });

        let uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Gui Uniform Buffer"),
            size: std::mem::size_of::<Mat4>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Gui BGL"),
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

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Gui Bind Group"),
            layout: &bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buf.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Gui Pipeline Layout"),
            bind_group_layouts: &[Some(&bgl)],
            ..Default::default()
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Gui Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[GuiVertex::desc()],
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
        });

        let vertex_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Gui Vertex Buffer"),
            contents: bytemuck::cast_slice(&[GuiVertex {
                position: [0.0; 2],
                color: [0.0; 4],
            }]),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
        let index_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Gui Index Buffer"),
            contents: bytemuck::cast_slice(&[0u32]),
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        });

        let font_system = FontSystem::new();
        let swash_cache = SwashCache::new();
        let glyph_cache = GlyphCache::new(device);
        let mut text_atlas = TextAtlas::new(device, queue, &glyph_cache, format);
        let text_renderer = TextRenderer::new(
            &mut text_atlas,
            device,
            wgpu::MultisampleState::default(),
            None,
        );
        let viewport = Viewport::new(device, &glyph_cache);

        Self {
            pipeline,
            uniform_buf,
            bind_group,
            vertex_buf,
            index_buf,
            buf_vertex_cap: 0,
            buf_index_cap: 0,
            font_system,
            swash_cache,
            glyph_cache,
            text_atlas,
            text_renderer,
            viewport,
        }
    }

    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        ctx: &GuiContext,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if ctx.vertices.is_empty() && ctx.texts.is_empty() {
            return Ok(());
        }

        // Upload shape geometry
        if !ctx.vertices.is_empty() {
            let v_bytes = bytemuck::cast_slice(&ctx.vertices);
            let i_bytes = bytemuck::cast_slice(&ctx.indices);

            if v_bytes.len() > self.buf_vertex_cap {
                self.vertex_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Gui Vertex Buffer"),
                    contents: v_bytes,
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                });
                self.buf_vertex_cap = v_bytes.len();
            } else {
                queue.write_buffer(&self.vertex_buf, 0, v_bytes);
            }

            if i_bytes.len() > self.buf_index_cap {
                self.index_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Gui Index Buffer"),
                    contents: i_bytes,
                    usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                });
                self.buf_index_cap = i_bytes.len();
            } else {
                queue.write_buffer(&self.index_buf, 0, i_bytes);
            }
        }

        // Upload projection
        let proj = Mat4::orthographic_rh_gl(0.0, ctx.screen_w, ctx.screen_h, 0.0, -1.0, 1.0);
        queue.write_buffer(
            &self.uniform_buf,
            0,
            bytemuck::cast_slice(&proj.to_cols_array()),
        );

        // Build glyphon resources for text
        let (mut _text_bufs, mut text_areas) = (vec![], vec![]);
        if !ctx.texts.is_empty() {
            self.viewport.update(
                queue,
                Resolution {
                    width: ctx.screen_w as u32,
                    height: ctx.screen_h as u32,
                },
            );
            _text_bufs = ctx
                .texts
                .iter()
                .map(|t| {
                    let mut buf =
                        GlyphBuffer::new(&mut self.font_system, Metrics::new(14.0, 18.0));
                    buf.set_size(&mut self.font_system, Some(400.0), Some(20.0));
                    buf.set_text(
                        &mut self.font_system,
                        &t.text,
                        &Attrs::new().family(Family::SansSerif),
                        Shaping::Basic,
                        None,
                    );
                    buf
                })
                .collect();
            text_areas = _text_bufs
                .iter()
                .zip(ctx.texts.iter())
                .map(|(buf, t)| TextArea {
                    buffer: buf,
                    left: t.x,
                    top: t.y,
                    scale: 1.0,
                    bounds: TextBounds {
                        left: t.clip_left as i32,
                        top: t.clip_top as i32,
                        right: t.clip_right as i32,
                        bottom: t.clip_bottom as i32,
                    },
                    default_color: GlyphColor::rgba(
                        (t.color[0] * 255.0) as u8,
                        (t.color[1] * 255.0) as u8,
                        (t.color[2] * 255.0) as u8,
                        (t.color[3] * 255.0) as u8,
                    ),
                    custom_glyphs: &[],
                })
                .collect();
            self.text_renderer.prepare(
                device,
                queue,
                &mut self.font_system,
                &mut self.text_atlas,
                &self.viewport,
                text_areas.clone(),
                &mut self.swash_cache,
            )?;
        }

        // Render pass
        {
            let mut rp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Gui Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            if !ctx.vertices.is_empty() {
                rp.set_pipeline(&self.pipeline);
                rp.set_bind_group(0, &self.bind_group, &[]);
                rp.set_vertex_buffer(0, self.vertex_buf.slice(..));
                rp.set_index_buffer(self.index_buf.slice(..), wgpu::IndexFormat::Uint32);
                rp.draw_indexed(0..ctx.indices.len() as u32, 0, 0..1);
            }

            if !text_areas.is_empty() {
                self.text_renderer
                    .render(&self.text_atlas, &self.viewport, &mut rp)?;
            }
        }

        self.text_atlas.trim();

        Ok(())
    }
}
