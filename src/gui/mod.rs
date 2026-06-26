use glam::Mat4;
use glyphon::{
    Attrs, Buffer as GlyphBuffer, Cache as GlyphCache, Color as GlyphColor, Family,
    FontSystem, Metrics, Resolution, Shaping, SwashCache, TextArea, TextAtlas, TextBounds,
    TextRenderer, Viewport,
};
use wgpu::util::DeviceExt;

// ---------------------------------------------------------------------------
// Vertex & Shader
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GuiVertex {
    position: [f32; 2],
    color: [f32; 4],
}

impl GuiVertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<GuiVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: 8,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

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

// ---------------------------------------------------------------------------
// Gui
// ---------------------------------------------------------------------------

struct DrawText {
    text: String,
    x: f32,
    y: f32,
    color: [f32; 4],
}

pub struct Gui {
    screen_w: f32,
    screen_h: f32,

    // Input state
    mouse_x: f32,
    mouse_y: f32,
    mouse_down: bool,
    mouse_pressed: bool,
    mouse_released: bool,

    // ID stack
    id_gen: u64,
    hot: u64,
    active: u64,

    // Per-frame draw buffers
    vertices: Vec<GuiVertex>,
    indices: Vec<u32>,
    texts: Vec<DrawText>,

    // GPU resources
    pipeline: wgpu::RenderPipeline,
    uniform_buf: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    vertex_buf: wgpu::Buffer,
    index_buf: wgpu::Buffer,
    buf_vertex_cap: usize,
    buf_index_cap: usize,

    // glyphon
    font_system: FontSystem,
    swash_cache: SwashCache,
    #[allow(dead_code)]
    glyph_cache: GlyphCache,
    text_atlas: TextAtlas,
    text_renderer: TextRenderer,
    viewport: Viewport,
}

impl Gui {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
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
            contents: bytemuck::cast_slice(&[GuiVertex { position: [0.0; 2], color: [0.0; 4] }]),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
        let index_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Gui Index Buffer"),
            contents: bytemuck::cast_slice(&[0u32]),
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        });

        let font_system = FontSystem::new();
        let swash_cache = SwashCache::new();
        let glyph_cache = GlyphCache::new(&device);
        let mut text_atlas = TextAtlas::new(&device, &queue, &glyph_cache, format);
        let text_renderer = TextRenderer::new(
            &mut text_atlas,
            &device,
            wgpu::MultisampleState::default(),
            None,
        );
        let viewport = Viewport::new(&device, &glyph_cache);

        Self {
            screen_w: 0.0,
            screen_h: 0.0,
            mouse_x: 0.0,
            mouse_y: 0.0,
            mouse_down: false,
            mouse_pressed: false,
            mouse_released: false,
            id_gen: 1,
            hot: 0,
            active: 0,
            vertices: Vec::new(),
            indices: Vec::new(),
            texts: Vec::new(),
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

    // ── Input ──────────────────────────────────────────────────────────────────

    pub fn mouse_press(&mut self, pressed: bool) {
        if pressed && !self.mouse_down {
            self.mouse_pressed = true;
        }
        self.mouse_down = pressed;
        if !pressed {
            self.mouse_released = true;
        }
    }

    pub fn mouse_move(&mut self, x: f64, y: f64) {
        self.mouse_x = x as f32;
        self.mouse_y = y as f32;
    }

    pub fn resize(&mut self, w: u32, h: u32) {
        self.screen_w = w as f32;
        self.screen_h = h as f32;
    }

    // ── Frame lifecycle ────────────────────────────────────────────────────────

    pub fn begin_frame(&mut self, w: u32, h: u32) {
        self.screen_w = w as f32;
        self.screen_h = h as f32;
        self.vertices.clear();
        self.indices.clear();
        self.texts.clear();
        self.hot = 0;
        self.id_gen = 1;
    }

    // ── Internal helpers ───────────────────────────────────────────────────────

    fn gen_id(&mut self) -> u64 {
        let id = self.id_gen;
        self.id_gen += 1;
        id
    }

    fn hover(&self, x: f32, y: f32, w: f32, h: f32) -> bool {
        self.mouse_x >= x && self.mouse_x <= x + w && self.mouse_y >= y && self.mouse_y <= y + h
    }

    fn add_rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: [f32; 4]) {
        if w <= 0.0 || h <= 0.0 {
            return;
        }
        let i = self.vertices.len() as u32;
        self.vertices.extend([
            GuiVertex { position: [x, y], color },
            GuiVertex { position: [x + w, y], color },
            GuiVertex { position: [x + w, y + h], color },
            GuiVertex { position: [x, y + h], color },
        ]);
        self.indices.extend([i, i + 1, i + 2, i, i + 2, i + 3]);
    }

    fn add_border(&mut self, x: f32, y: f32, w: f32, h: f32, color: [f32; 4]) {
        let t = 1.0;
        self.add_rect(x, y, w, t, color);
        self.add_rect(x, y + h - t, w, t, color);
        self.add_rect(x, y, t, h, color);
        self.add_rect(x + w - t, y, t, h, color);
    }

    fn add_text(&mut self, text: &str, x: f32, y: f32, color: [f32; 4]) {
        if !text.is_empty() {
            self.texts.push(DrawText { text: text.to_string(), x, y, color });
        }
    }

    // ── Widgets ────────────────────────────────────────────────────────────────

    pub fn button(&mut self, label: &str, x: f32, y: f32, w: f32, h: f32) -> bool {
        let id = self.gen_id();
        let over = self.hover(x, y, w, h);
        let mut clicked = false;

        if over && self.mouse_pressed && self.active == 0 {
            self.active = id;
        }
        if self.active == id && self.mouse_released {
            if over {
                clicked = true;
            }
            self.active = 0;
        }
        if over || self.active == id {
            self.hot = id;
        }

        let color = if self.active == id {
            [0.45, 0.55, 0.65, 1.0]
        } else if over {
            [0.35, 0.45, 0.55, 1.0]
        } else {
            [0.22, 0.27, 0.32, 1.0]
        };
        self.add_rect(x, y, w, h, color);
        self.add_border(x, y, w, h, [0.45, 0.45, 0.45, 1.0]);
        self.add_text(label, x + 6.0, y + 3.0, [0.9, 0.9, 0.9, 1.0]);
        clicked
    }

    pub fn label(&mut self, text: &str, x: f32, y: f32, color: [f32; 4]) {
        self.add_text(text, x, y, color);
    }

    pub fn checkbox(&mut self, label: &str, x: f32, y: f32, checked: &mut bool) {
        let id = self.gen_id();
        let sz = 16.0;
        let over = self.hover(x, y, sz, sz);

        if over && self.mouse_pressed && self.active == 0 {
            *checked = !*checked;
            self.active = id;
        }
        if self.active == id && self.mouse_released {
            self.active = 0;
        }

        let bg = if over { [0.35, 0.35, 0.40, 1.0] } else { [0.18, 0.18, 0.22, 1.0] };
        self.add_rect(x, y, sz, sz, bg);
        self.add_border(x, y, sz, sz, [0.5, 0.5, 0.5, 1.0]);
        if *checked {
            self.add_rect(x + 3.0, y + 3.0, sz - 6.0, sz - 6.0, [0.2, 0.8, 0.3, 1.0]);
        }
        self.add_text(label, x + sz + 6.0, y + 1.0, [0.8, 0.8, 0.8, 1.0]);
    }

    pub fn slider(&mut self, label: &str, x: f32, y: f32, w: f32, value: &mut f32, min: f32, max: f32) {
        let id = self.gen_id();
        let h = 14.0;
        let over = self.hover(x, y, w, h);

        if over && self.mouse_pressed && self.active == 0 {
            self.active = id;
        }
        if self.active == id && self.mouse_down {
            let t = ((self.mouse_x - x) / w).clamp(0.0, 1.0);
            *value = min + t * (max - min);
        }
        if self.active == id && self.mouse_released {
            self.active = 0;
        }

        self.add_rect(x, y, w, h, [0.10, 0.10, 0.13, 1.0]);
        let t = ((*value - min) / (max - min)).clamp(0.0, 1.0);
        let fill = if self.active == id { [0.4, 0.6, 0.8, 1.0] } else { [0.3, 0.5, 0.7, 1.0] };
        self.add_rect(x, y, w * t, h, fill);
        self.add_border(x, y, w, h, [0.3, 0.3, 0.3, 1.0]);
        self.add_text(&format!("{:.2}", value), x + w + 6.0, y - 1.0, [0.7, 0.7, 0.7, 1.0]);
        self.add_text(label, x, y + h + 2.0, [0.6, 0.6, 0.6, 1.0]);
    }

    pub fn group(&mut self, title: &str, x: f32, y: f32, w: f32, h: f32, f: impl FnOnce(&mut Self)) {
        self.add_rect(x, y, w, h, [0.07, 0.07, 0.09, 0.88]);
        self.add_border(x, y, w, h, [0.22, 0.22, 0.27, 1.0]);
        self.add_text(title, x + 6.0, y + 4.0, [0.6, 0.6, 0.7, 1.0]);
        self.add_rect(x + 4.0, y + 22.0, w - 8.0, 1.0, [0.18, 0.18, 0.22, 1.0]);
        f(self);
    }

    // ── Advanced ───────────────────────────────────────────────────────────────

    #[allow(dead_code)]
    pub fn item_wants_mouse(&self) -> bool {
        self.hot != 0 || self.active != 0
    }

    /// True while a widget is being actively dragged/held (e.g. slider).
    /// Use this to suppress camera events during GUI interaction.
    pub fn is_active(&self) -> bool {
        self.active != 0
    }

    #[allow(dead_code)]
    pub fn add_line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, width: f32, color: [f32; 4]) {
        let dx = x2 - x1;
        let dy = y2 - y1;
        let len = (dx * dx + dy * dy).sqrt().max(1e-6);
        let nx = -dy / len * width * 0.5;
        let ny = dx / len * width * 0.5;

        let i = self.vertices.len() as u32;
        self.vertices.extend([
            GuiVertex { position: [x1 - nx, y1 - ny], color },
            GuiVertex { position: [x1 + nx, y1 + ny], color },
            GuiVertex { position: [x2 + nx, y2 + ny], color },
            GuiVertex { position: [x2 - nx, y2 - ny], color },
        ]);
        self.indices.extend([i, i + 1, i + 2, i, i + 2, i + 3]);
    }

    // ── Render ─────────────────────────────────────────────────────────────────

    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if self.vertices.is_empty() && self.texts.is_empty() {
            return Ok(());
        }

        // Upload shape geometry
        if !self.vertices.is_empty() {
            let v_bytes = bytemuck::cast_slice(&self.vertices);
            let i_bytes = bytemuck::cast_slice(&self.indices);

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
        let proj = Mat4::orthographic_rh_gl(0.0, self.screen_w, self.screen_h, 0.0, -1.0, 1.0);
        queue.write_buffer(&self.uniform_buf, 0, bytemuck::cast_slice(&proj.to_cols_array()));

        // Build glyphon resources for text, keeping them alive through the pass
        let (mut _text_bufs, mut text_areas) = (vec![], vec![]);
        if !self.texts.is_empty() {
            self.viewport.update(
                queue,
                Resolution { width: self.screen_w as u32, height: self.screen_h as u32 },
            );
            _text_bufs = self
                .texts
                .iter()
                .map(|t| {
                    let mut buf = GlyphBuffer::new(&mut self.font_system, Metrics::new(14.0, 18.0));
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
                .zip(self.texts.iter())
                .map(|(buf, t)| TextArea {
                    buffer: buf,
                    left: t.x,
                    top: t.y,
                    scale: 1.0,
                    bounds: TextBounds::default(),
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
        // text_bufs and text_areas are both alive here

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

            if !self.vertices.is_empty() {
                rp.set_pipeline(&self.pipeline);
                rp.set_bind_group(0, &self.bind_group, &[]);
                rp.set_vertex_buffer(0, self.vertex_buf.slice(..));
                rp.set_index_buffer(self.index_buf.slice(..), wgpu::IndexFormat::Uint32);
                rp.draw_indexed(0..self.indices.len() as u32, 0, 0..1);
            }

            if !text_areas.is_empty() {
                self.text_renderer.render(&self.text_atlas, &self.viewport, &mut rp)?;
            }
        }

        self.text_atlas.trim();

        self.mouse_pressed = false;
        self.mouse_released = false;

        Ok(())
    }
}
