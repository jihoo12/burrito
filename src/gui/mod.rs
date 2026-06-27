mod context;
mod renderer;

use context::GuiContext;
use renderer::GuiRenderer;

pub struct Gui {
    ctx: GuiContext,
    renderer: GuiRenderer,
}

impl Gui {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
    ) -> Self {
        Self {
            ctx: GuiContext::new(),
            renderer: GuiRenderer::new(device, queue, format),
        }
    }

    // ── Input ──

    pub fn mouse_press(&mut self, pressed: bool) {
        self.ctx.mouse_press(pressed);
    }

    pub fn mouse_move(&mut self, x: f64, y: f64) {
        self.ctx.mouse_move(x, y);
    }

    pub fn resize(&mut self, w: u32, h: u32) {
        self.ctx.resize(w, h);
    }

    pub fn key_event(&mut self, c: Option<char>, backspace: bool, delete: bool, enter: bool) {
        self.ctx.key_event(c, backspace, delete, enter);
    }

    pub fn scroll(&mut self, delta: f32) {
        self.ctx.scroll(delta);
    }

    // ── Frame lifecycle ──

    pub fn begin_frame(&mut self, w: u32, h: u32) {
        self.ctx.begin_frame(w, h);
    }

    // ── Widgets ──

    pub fn group(
        &mut self,
        title: &str,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        f: impl FnOnce(&mut GuiContext),
    ) {
        self.ctx.group(title, x, y, w, h, f);
    }

    // ── State queries ──

    pub fn is_active(&self) -> bool {
        self.ctx.is_active()
    }

    pub fn is_focused(&self) -> bool {
        self.ctx.is_focused()
    }

    // ── Render ──

    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.renderer.render(device, queue, encoder, view, &self.ctx)?;

        self.ctx.was_focused = self.ctx.focused;
        self.ctx.mouse_pressed = false;
        self.ctx.mouse_released = false;
        self.ctx.scroll_delta = 0.0;

        Ok(())
    }
}
