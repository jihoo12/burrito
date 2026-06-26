use super::{mesh::Mesh, vertex::Vertex};

/// HTML5 Canvas 2D-like drawing API backed by wgpu-compatible Mesh geometry.
///
/// # Example
/// ```ignore
/// let mut c = Canvas2D::new();
/// c.set_fill_style([1.0, 0.0, 0.0]);
/// c.fill_rect(0.0, 0.0, 100.0, 50.0);
/// c.set_stroke_style([0.0, 0.0, 1.0]);
/// c.set_line_width(2.0);
/// c.begin_path();
/// c.move_to(0.0, 0.0);
/// c.line_to(100.0, 50.0);
/// c.stroke();
/// let mesh = c.build();
/// ```
pub struct Canvas2D {
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
    subpaths: Vec<Vec<[f32; 2]>>,
    current: Vec<[f32; 2]>,
    stroke_color: [f32; 3],
    fill_color: [f32; 3],
    line_width: f32,
}

impl Default for Canvas2D {
    fn default() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            subpaths: Vec::new(),
            current: Vec::new(),
            stroke_color: [0.0, 0.0, 0.0],
            fill_color: [0.0, 0.0, 0.0],
            line_width: 1.0,
        }
    }
}

impl Canvas2D {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_stroke_style(&mut self, color: [f32; 3]) {
        self.stroke_color = color;
    }

    pub fn set_fill_style(&mut self, color: [f32; 3]) {
        self.fill_color = color;
    }

    pub fn set_line_width(&mut self, width: f32) {
        self.line_width = width;
    }

    #[allow(dead_code)]
    pub fn fill_rect(&mut self, x: f32, y: f32, w: f32, h: f32) {
        let base = self.vertices.len() as u32;
        self.vertices.extend([
            Vertex::new([x, y, 0.0], self.fill_color),
            Vertex::new([x + w, y, 0.0], self.fill_color),
            Vertex::new([x + w, y + h, 0.0], self.fill_color),
            Vertex::new([x, y + h, 0.0], self.fill_color),
        ]);
        self.indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    #[allow(dead_code)]
    pub fn stroke_rect(&mut self, x: f32, y: f32, w: f32, h: f32) {
        let c = self.stroke_color;
        let lw = self.line_width;
        self.add_thick_line(x, y, x + w, y, lw, c);
        self.add_thick_line(x + w, y, x + w, y + h, lw, c);
        self.add_thick_line(x + w, y + h, x, y + h, lw, c);
        self.add_thick_line(x, y + h, x, y, lw, c);
    }

    pub fn begin_path(&mut self) {
        self.subpaths.clear();
        self.current.clear();
    }

    pub fn move_to(&mut self, x: f32, y: f32) {
        if !self.current.is_empty() {
            self.subpaths.push(std::mem::take(&mut self.current));
        }
        self.current.push([x, y]);
    }

    pub fn line_to(&mut self, x: f32, y: f32) {
        self.current.push([x, y]);
    }

    pub fn close_path(&mut self) {
        if self.current.len() >= 2 {
            let first = self.current[0];
            self.current.push(first);
        }
    }

    pub fn stroke(&mut self) {
        self.finish_current_subpath();
        let c = self.stroke_color;
        let lw = self.line_width;
        let subs: Vec<Vec<[f32; 2]>> = std::mem::take(&mut self.subpaths);
        for sub in &subs {
            for i in 0..sub.len().saturating_sub(1) {
                let [x1, y1] = sub[i];
                let [x2, y2] = sub[i + 1];
                self.add_thick_line(x1, y1, x2, y2, lw, c);
            }
        }
        self.current.clear();
    }

    pub fn fill(&mut self) {
        self.finish_current_subpath();
        let c = self.fill_color;
        let mut all_points: Vec<[f32; 2]> = self.subpaths.iter().flat_map(|s| s.iter().copied()).collect();
        // Remove trailing point that duplicates the first (from close_path)
        if all_points.len() > 3 && all_points.first() == all_points.last() {
            all_points.pop();
        }
        if all_points.len() >= 3 {
            let base = self.vertices.len() as u32;
            for &p in &all_points {
                self.vertices.push(Vertex::new([p[0], p[1], 0.0], c));
            }
            for i in 1..all_points.len() as u32 - 1 {
                self.indices.extend([base, base + i, base + i + 1]);
            }
        }
        self.subpaths.clear();
        self.current.clear();
    }

    #[allow(dead_code)]
    pub fn rect(&mut self, x: f32, y: f32, w: f32, h: f32) {
        self.finish_current_subpath();
        self.current.push([x, y]);
        self.current.push([x + w, y]);
        self.current.push([x + w, y + h]);
        self.current.push([x, y + h]);
        self.current.push([x, y]);
    }

    pub fn arc(&mut self, x: f32, y: f32, r: f32, start_angle: f32, end_angle: f32) {
        let da = (end_angle - start_angle).abs();
        let segments = ((r * da * 0.5).ceil().max(4.0) as usize).min(128);
        for i in 0..=segments {
            let t = i as f32 / segments as f32;
            let a = start_angle * (1.0 - t) + end_angle * t;
            self.current.push([x + r * a.cos(), y + r * a.sin()]);
        }
        if self.subpaths.is_empty() && self.current.len() > 1 {
            self.first_subpath_start();
        }
    }

    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.vertices.clear();
        self.indices.clear();
        self.subpaths.clear();
        self.current.clear();
    }

    pub fn build(&self) -> Mesh {
        Mesh::new(self.vertices.clone(), self.indices.clone())
    }

    // Internal helpers

    fn finish_current_subpath(&mut self) {
        if !self.current.is_empty() {
            self.subpaths.push(std::mem::take(&mut self.current));
        }
    }

    fn first_subpath_start(&mut self) {}

    fn add_thick_line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, width: f32, color: [f32; 3]) {
        if width <= 0.0 {
            return;
        }
        let dx = x2 - x1;
        let dy = y2 - y1;
        let len = (dx * dx + dy * dy).sqrt();
        if len < 1e-10 {
            return;
        }
        let nx = -dy / len * width * 0.5;
        let ny = dx / len * width * 0.5;

        let base = self.vertices.len() as u32;
        self.vertices.extend([
            Vertex::new([x1 - nx, y1 - ny, 0.0], color),
            Vertex::new([x1 + nx, y1 + ny, 0.0], color),
            Vertex::new([x2 + nx, y2 + ny, 0.0], color),
            Vertex::new([x2 - nx, y2 - ny, 0.0], color),
        ]);
        self.indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fill_rect() {
        let mut c = Canvas2D::new();
        c.set_fill_style([1.0, 0.0, 0.0]);
        c.fill_rect(0.0, 0.0, 10.0, 10.0);
        let m = c.build();
        assert_eq!(m.vertices.len(), 4);
        assert_eq!(m.indices.len(), 6);
    }

    #[test]
    fn test_stroke_rect() {
        let mut c = Canvas2D::new();
        c.set_stroke_style([0.0, 0.0, 1.0]);
        c.set_line_width(2.0);
        c.stroke_rect(0.0, 0.0, 10.0, 10.0);
        let m = c.build();
        assert_eq!(m.vertices.len(), 16);
        assert_eq!(m.indices.len(), 24);
    }

    #[test]
    fn test_path_stroke() {
        let mut c = Canvas2D::new();
        c.set_stroke_style([1.0, 0.0, 0.0]);
        c.begin_path();
        c.move_to(0.0, 0.0);
        c.line_to(10.0, 10.0);
        c.stroke();
        let m = c.build();
        assert!(m.vertices.len() >= 4);
    }

    #[test]
    fn test_path_fill() {
        let mut c = Canvas2D::new();
        c.set_fill_style([1.0, 0.0, 0.0]);
        c.begin_path();
        c.move_to(0.0, 0.0);
        c.line_to(10.0, 0.0);
        c.line_to(5.0, 10.0);
        c.close_path();
        c.fill();
        let m = c.build();
        assert_eq!(m.vertices.len(), 3);
        assert_eq!(m.indices.len(), 3);
    }

    #[test]
    fn test_arc() {
        let mut c = Canvas2D::new();
        c.begin_path();
        c.arc(0.0, 0.0, 5.0, 0.0, std::f32::consts::PI * 2.0);
        assert!(c.current.len() > 4);
    }

    #[test]
    fn test_path_rect() {
        let mut c = Canvas2D::new();
        c.begin_path();
        c.rect(0.0, 0.0, 10.0, 10.0);
        assert_eq!(c.current.len(), 5);
    }

    #[test]
    fn test_clear() {
        let mut c = Canvas2D::new();
        c.fill_rect(0.0, 0.0, 10.0, 10.0);
        assert!(!c.build().is_empty());
        c.clear();
        assert!(c.build().is_empty());
    }
}
