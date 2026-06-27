use std::collections::HashMap;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct GuiVertex {
    pub(crate) position: [f32; 2],
    pub(crate) color: [f32; 4],
}

impl GuiVertex {
    pub(crate) fn desc() -> wgpu::VertexBufferLayout<'static> {
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

pub(crate) struct DrawText {
    pub(crate) text: String,
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) color: [f32; 4],
    pub(crate) clip_left: f32,
    pub(crate) clip_top: f32,
    pub(crate) clip_right: f32,
    pub(crate) clip_bottom: f32,
}

pub struct GuiContext {
    pub(crate) screen_w: f32,
    pub(crate) screen_h: f32,

    mouse_x: f32,
    mouse_y: f32,
    mouse_down: bool,
    pub(crate) mouse_pressed: bool,
    pub(crate) mouse_released: bool,

    pub(crate) focused: u64,
    pub(crate) was_focused: u64,
    input_chars: Vec<char>,
    key_backspace: bool,
    key_enter: bool,
    key_delete: bool,

    pub(crate) scroll_delta: f32,

    id_gen: u64,
    hot: u64,
    pub(crate) active: u64,

    pub(crate) vertices: Vec<GuiVertex>,
    pub(crate) indices: Vec<u32>,
    pub(crate) texts: Vec<DrawText>,

    scroll_offsets: HashMap<u64, f32>,
}

impl GuiContext {
    pub fn new() -> Self {
        Self {
            screen_w: 0.0,
            screen_h: 0.0,
            mouse_x: 0.0,
            mouse_y: 0.0,
            mouse_down: false,
            mouse_pressed: false,
            mouse_released: false,
            focused: 0,
            was_focused: 0,
            input_chars: Vec::new(),
            key_backspace: false,
            key_enter: false,
            key_delete: false,
            scroll_delta: 0.0,
            id_gen: 1,
            hot: 0,
            active: 0,
            vertices: Vec::new(),
            indices: Vec::new(),
            texts: Vec::new(),
            scroll_offsets: HashMap::new(),
        }
    }

    // ── Input ──

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

    pub fn scroll(&mut self, delta: f32) {
        self.scroll_delta += delta;
    }

    pub fn key_event(&mut self, c: Option<char>, backspace: bool, delete: bool, enter: bool) {
        if let Some(c) = c {
            if self.input_chars.len() < 1024 {
                self.input_chars.push(c);
            }
        }
        if backspace {
            self.key_backspace = true;
        }
        if delete {
            self.key_delete = true;
        }
        if enter {
            self.key_enter = true;
        }
    }

    // ── Frame lifecycle ──

    pub fn begin_frame(&mut self, w: u32, h: u32) {
        self.screen_w = w as f32;
        self.screen_h = h as f32;
        self.vertices.clear();
        self.indices.clear();
        self.texts.clear();
        self.hot = 0;
        self.id_gen = 1;

        if self.was_focused == 0 {
            self.input_chars.clear();
            self.key_backspace = false;
            self.key_enter = false;
            self.key_delete = false;
        }
    }

    // ── ID management ──

    fn gen_id(&mut self) -> u64 {
        let id = self.id_gen;
        self.id_gen += 1;
        id
    }

    fn hover(&self, x: f32, y: f32, w: f32, h: f32) -> bool {
        self.mouse_x >= x && self.mouse_x <= x + w && self.mouse_y >= y && self.mouse_y <= y + h
    }

    // ── Drawing primitives ──

    pub(crate) fn add_rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: [f32; 4]) {
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

    pub(crate) fn add_border(&mut self, x: f32, y: f32, w: f32, h: f32, color: [f32; 4]) {
        let t = 1.0;
        self.add_rect(x, y, w, t, color);
        self.add_rect(x, y + h - t, w, t, color);
        self.add_rect(x, y, t, h, color);
        self.add_rect(x + w - t, y, t, h, color);
    }

    pub(crate) fn add_text(&mut self, text: &str, x: f32, y: f32, color: [f32; 4]) {
        if !text.is_empty() {
            self.texts.push(DrawText {
                text: text.to_string(),
                x,
                y,
                color,
                clip_left: f32::MIN,
                clip_top: f32::MIN,
                clip_right: f32::MAX,
                clip_bottom: f32::MAX,
            });
        }
    }

    // ── Widgets ──

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

        let bg = if over {
            [0.35, 0.35, 0.40, 1.0]
        } else {
            [0.18, 0.18, 0.22, 1.0]
        };
        self.add_rect(x, y, sz, sz, bg);
        self.add_border(x, y, sz, sz, [0.5, 0.5, 0.5, 1.0]);
        if *checked {
            self.add_rect(x + 3.0, y + 3.0, sz - 6.0, sz - 6.0, [0.2, 0.8, 0.3, 1.0]);
        }
        self.add_text(label, x + sz + 6.0, y + 1.0, [0.8, 0.8, 0.8, 1.0]);
    }

    pub fn slider(
        &mut self,
        label: &str,
        x: f32,
        y: f32,
        w: f32,
        value: &mut f32,
        min: f32,
        max: f32,
    ) {
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
        let fill = if self.active == id {
            [0.4, 0.6, 0.8, 1.0]
        } else {
            [0.3, 0.5, 0.7, 1.0]
        };
        self.add_rect(x, y, w * t, h, fill);
        self.add_border(x, y, w, h, [0.3, 0.3, 0.3, 1.0]);
        self.add_text(
            &format!("{:.2}", value),
            x + w + 6.0,
            y - 1.0,
            [0.7, 0.7, 0.7, 1.0],
        );
        self.add_text(label, x, y + h + 2.0, [0.6, 0.6, 0.6, 1.0]);
    }

    pub fn text_area(&mut self, x: f32, y: f32, w: f32, h: f32, text: &mut String) {
        let id = self.gen_id();
        let over = self.hover(x, y, w, h);

        if over && self.mouse_pressed && self.active == 0 {
            self.active = id;
            self.focused = id;
        }
        if self.active == id && self.mouse_released {
            self.active = 0;
        }

        let mut modified = false;
        if self.focused == id {
            for c in self.input_chars.drain(..) {
                text.push(c);
                modified = true;
            }
            if self.key_backspace {
                text.pop();
                self.key_backspace = false;
                modified = true;
            }
            if self.key_delete {
                self.key_delete = false;
                modified = true;
            }
            if self.key_enter {
                text.push('\n');
                self.key_enter = false;
                modified = true;
            }
        }

        let bg = if self.focused == id {
            [0.12, 0.12, 0.16, 1.0]
        } else {
            [0.09, 0.09, 0.11, 1.0]
        };
        self.add_rect(x, y, w, h, bg);
        let bc = if self.focused == id {
            [0.4, 0.6, 0.8, 1.0]
        } else {
            [0.25, 0.25, 0.28, 1.0]
        };
        self.add_border(x, y, w, h, bc);

        let line_h = 16.0;
        let char_w = 7.0;
        let pad_x = 4.0;
        let pad_y = 3.0;
        let max_chars = ((w - pad_x * 2.0) / char_w).floor() as usize;
        let max_chars = max_chars.max(1);

        // Collect raw lines, ensuring at least one empty line
        let mut raw_lines: Vec<&str> = text.lines().collect();
        if text.ends_with('\n') {
            raw_lines.push("");
        }
        if raw_lines.is_empty() {
            raw_lines.push("");
        }

        // Wrap long lines at character boundaries
        let mut lines: Vec<String> = Vec::new();
        for line in raw_lines {
            let chars: Vec<char> = line.chars().collect();
            if chars.len() <= max_chars {
                lines.push(line.to_string());
            } else {
                let mut start = 0;
                while start < chars.len() {
                    let end = (start + max_chars).min(chars.len());
                    lines.push(chars[start..end].iter().collect());
                    start = end;
                }
            }
        }

        // Compute max scroll offset
        let total_lines = lines.len();
        let visible_lines = ((h - pad_y * 2.0) / line_h).floor() as usize;
        let max_scroll = if total_lines > visible_lines {
            (total_lines - visible_lines) as f32 * line_h
        } else {
            0.0
        };

        // Auto-scroll to bottom when modified while focused
        if self.focused == id && modified {
            self.scroll_offsets.insert(id, max_scroll);
        }

        // Manual scroll with mouse wheel
        if self.focused == id && over && self.scroll_delta != 0.0 {
            let off = self.scroll_offsets.get(&id).copied().unwrap_or(0.0);
            let off = (off - self.scroll_delta).clamp(0.0, max_scroll);
            self.scroll_offsets.insert(id, off);
        }

        let scroll_off = self.scroll_offsets.get(&id).copied().unwrap_or(0.0).clamp(0.0, max_scroll);
        let top_clip = y + pad_y;
        let bot_clip = y + h - pad_y;

        // Render only visible (non-clipped) lines
        let mut line_y = y + pad_y - scroll_off;
        for line in &lines {
            if line_y + line_h <= top_clip {
                line_y += line_h;
                continue;
            }
            if line_y >= bot_clip {
                break;
            }
            self.texts.push(DrawText {
                text: line.clone(),
                x: x + pad_x,
                y: line_y,
                color: [0.85, 0.85, 0.85, 1.0],
                clip_left: x,
                clip_top: y,
                clip_right: x + w,
                clip_bottom: y + h,
            });
            line_y += line_h;
        }

        // Cursor at end of text
        if self.focused == id {
            let last = lines.len().saturating_sub(1);
            let cursor_line = &lines[last];
            let cursor_x = x + pad_x + cursor_line.chars().count() as f32 * char_w;
            let cursor_y = y + pad_y + last as f32 * line_h - scroll_off;
            if cursor_x < x + w - pad_x
                && cursor_y >= top_clip
                && cursor_y + line_h <= bot_clip
            {
                self.add_rect(cursor_x, cursor_y, 1.5, line_h - 2.0, [0.8, 0.8, 0.8, 1.0]);
            }
        }
    }

    pub fn group(
        &mut self,
        title: &str,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        f: impl FnOnce(&mut Self),
    ) {
        self.add_rect(x, y, w, h, [0.07, 0.07, 0.09, 0.88]);
        self.add_border(x, y, w, h, [0.22, 0.22, 0.27, 1.0]);
        self.add_text(title, x + 6.0, y + 4.0, [0.6, 0.6, 0.7, 1.0]);
        self.add_rect(x + 4.0, y + 22.0, w - 8.0, 1.0, [0.18, 0.18, 0.22, 1.0]);
        f(self);
    }

    // ── Advanced ──

    #[allow(dead_code)]
    pub fn item_wants_mouse(&self) -> bool {
        self.hot != 0 || self.active != 0
    }

    pub fn is_active(&self) -> bool {
        self.active != 0
    }

    pub fn is_focused(&self) -> bool {
        self.focused != 0
    }

    #[allow(dead_code)]
    pub fn add_line(
        &mut self,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        width: f32,
        color: [f32; 4],
    ) {
        let dx = x2 - x1;
        let dy = y2 - y1;
        let len = (dx * dx + dy * dy).sqrt().max(1e-6);
        let nx = -dy / len * width * 0.5;
        let ny = dx / len * width * 0.5;

        let i = self.vertices.len() as u32;
        self.vertices.extend([
            GuiVertex {
                position: [x1 - nx, y1 - ny],
                color,
            },
            GuiVertex {
                position: [x1 + nx, y1 + ny],
                color,
            },
            GuiVertex {
                position: [x2 + nx, y2 + ny],
                color,
            },
            GuiVertex {
                position: [x2 - nx, y2 - ny],
                color,
            },
        ]);
        self.indices.extend([i, i + 1, i + 2, i, i + 2, i + 3]);
    }
}
