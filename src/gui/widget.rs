use super::Gui;

impl Gui {
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
            [0.45, 0.55, 0.70, 1.0]
        } else if over {
            [0.35, 0.40, 0.50, 1.0]
        } else {
            [0.20, 0.22, 0.27, 1.0]
        };
        self.add_rounded_rect(x, y, w, h, 4.0, color);
        self.add_border(x, y, w, h, [0.40, 0.40, 0.42, 1.0]);
        self.add_text(label, x + 6.0, y + 3.0, [0.9, 0.9, 0.9, 1.0]);
        clicked
    }

    #[allow(dead_code)]
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
            [0.16, 0.16, 0.20, 1.0]
        };
        self.add_rounded_rect(x, y, sz, sz, 3.0, bg);
        self.add_border(x, y, sz, sz, [0.50, 0.50, 0.52, 1.0]);
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

        self.add_rounded_rect(x, y, w, h, 3.0, [0.10, 0.10, 0.13, 1.0]);
        let t = ((*value - min) / (max - min)).clamp(0.0, 1.0);
        let fill = if self.active == id {
            [0.40, 0.60, 0.85, 1.0]
        } else {
            [0.25, 0.45, 0.70, 1.0]
        };
        self.add_rounded_rect(x, y, w * t.max(4.0), h, 3.0, fill);
        self.add_border(x, y, w, h, [0.30, 0.30, 0.30, 1.0]);
        self.add_text(
            &format!("{:.1}", value),
            x + w + 6.0,
            y - 1.0,
            [0.7, 0.7, 0.7, 1.0],
        );
        if !label.is_empty() {
            self.add_text(label, x, y + h + 2.0, [0.60, 0.60, 0.65, 1.0]);
        }
    }

    pub fn group(&mut self, title: &str, x: f32, y: f32, w: f32, h: f32, f: impl FnOnce(&mut Self)) {
        self.add_rounded_rect(x, y, w, h, 6.0, [0.06, 0.06, 0.08, 0.92]);
        self.add_border(x, y, w, h, [0.20, 0.20, 0.25, 1.0]);
        self.add_text(title, x + 6.0, y + 4.0, [0.55, 0.55, 0.65, 1.0]);
        self.add_rect(x + 4.0, y + 22.0, w - 8.0, 1.0, [0.18, 0.18, 0.22, 1.0]);
        f(self);
    }

    pub fn add_border(&mut self, x: f32, y: f32, w: f32, h: f32, color: [f32; 4]) {
        let t = 1.0;
        self.add_rect(x, y, w, t, color);
        self.add_rect(x, y + h - t, w, t, color);
        self.add_rect(x, y, t, h, color);
        self.add_rect(x + w - t, y, t, h, color);
    }
}
