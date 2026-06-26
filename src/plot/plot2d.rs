use super::{mesh::Mesh, vertex::Vertex};

fn add_line(
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u32>,
    p1: [f32; 3],
    p2: [f32; 3],
    color: [f32; 3],
) {
    let base = vertices.len() as u32;
    vertices.push(Vertex::new(p1, color));
    vertices.push(Vertex::new(p2, color));
    indices.push(base);
    indices.push(base + 1);
}

/// 2D line plot: y = f(x)
pub fn plot_2d_line(
    x_range: &[f32],
    func: impl Fn(f32) -> f32,
    color: [f32; 3],
) -> Mesh {
    if x_range.len() < 2 {
        return Mesh::new(vec![], vec![]);
    }
    let vertices: Vec<Vertex> = x_range
        .iter()
        .map(|&x| Vertex::new([x, func(x), 0.0], color))
        .collect();
    let n = vertices.len() as u32;
    let indices: Vec<u32> = (0..n - 1)
        .flat_map(|i| [i, i + 1])
        .collect();
    Mesh::new(vertices, indices)
}

/// 2D scatter plot
pub fn plot_2d_scatter(points: &[[f32; 2]], color: [f32; 3]) -> Mesh {
    let vertices: Vec<Vertex> = points
        .iter()
        .map(|&[x, y]| Vertex::new([x, y, 0.0], color))
        .collect();
    let indices: Vec<u32> = (0..points.len() as u32).collect();
    Mesh::new(vertices, indices)
}

/// 2D vertical bar chart
///
/// Each bar is centered at `positions[i]` with height `heights[i]`.
pub fn plot_2d_bar(
    positions: &[f32],
    heights: &[f32],
    bar_width: f32,
    color: [f32; 3],
) -> Mesh {
    let n = positions.len().min(heights.len());
    let mut vertices = Vec::with_capacity(n * 4);
    let mut indices = Vec::with_capacity(n * 6);
    for i in 0..n {
        let cx = positions[i];
        let h = heights[i];
        let half = bar_width * 0.5;
        let x0 = cx - half;
        let x1 = cx + half;
        let y0 = 0.0f32;
        let y1 = h;
        let base = vertices.len() as u32;
        vertices.extend([
            Vertex::new([x0, y0, 0.0], color),
            Vertex::new([x1, y0, 0.0], color),
            Vertex::new([x1, y1, 0.0], color),
            Vertex::new([x0, y1, 0.0], color),
        ]);
        indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    Mesh::new(vertices, indices)
}

/// 2D filled area between two curves
///
/// Fills the region between `lower(x)` and `upper(x)` across `x_range`.
#[allow(dead_code)]
pub fn plot_2d_fill_between(
    x_range: &[f32],
    lower: impl Fn(f32) -> f32,
    upper: impl Fn(f32) -> f32,
    color: [f32; 3],
) -> Mesh {
    if x_range.len() < 2 {
        return Mesh::new(vec![], vec![]);
    }
    let n = x_range.len();
    let mut vertices = Vec::with_capacity(n * 2);
    let mut indices = Vec::with_capacity((n - 1) * 6);
    // Lower boundary (bottom strip)
    for &x in x_range {
        vertices.push(Vertex::new([x, lower(x), 0.0], color));
    }
    // Upper boundary (top strip)
    for &x in x_range {
        vertices.push(Vertex::new([x, upper(x), 0.0], color));
    }
    let n_verts = n as u32;
    for i in 0..n as u32 - 1 {
        let lo = i;
        let hi = i + n_verts;
        indices.extend([lo, hi, hi + 1, lo, hi + 1, lo + 1]);
    }
    Mesh::new(vertices, indices)
}

/// 2D step plot (staircase)
///
/// Draws a stair-step line where the value stays constant between samples.
#[allow(dead_code)]
pub fn plot_2d_step(
    x_range: &[f32],
    func: impl Fn(f32) -> f32,
    color: [f32; 3],
) -> Mesh {
    if x_range.len() < 2 {
        return Mesh::new(vec![], vec![]);
    }
    let mut vertices = Vec::with_capacity(x_range.len() * 2);
    let mut indices = Vec::with_capacity((x_range.len() - 1) * 4);
    let mut idx: u32 = 0;
    for i in 0..x_range.len() - 1 {
        let x0 = x_range[i];
        let x1 = x_range[i + 1];
        let y = func(x0);
        vertices.push(Vertex::new([x0, y, 0.0], color));
        vertices.push(Vertex::new([x1, y, 0.0], color));
        indices.push(idx);
        indices.push(idx + 1);
        idx += 2;
    }
    Mesh::new(vertices, indices)
}

/// 2D stem plot
///
/// Draws vertical lines from zero to each function value.
#[allow(dead_code)]
pub fn plot_2d_stem(
    x_range: &[f32],
    func: impl Fn(f32) -> f32,
    color: [f32; 3],
) -> Mesh {
    let mut vertices = Vec::with_capacity(x_range.len() * 2);
    let mut indices = Vec::with_capacity(x_range.len() * 2);
    let mut idx: u32 = 0;
    for &x in x_range {
        let y = func(x);
        vertices.push(Vertex::new([x, 0.0, 0.0], color));
        vertices.push(Vertex::new([x, y, 0.0], color));
        indices.push(idx);
        indices.push(idx + 1);
        idx += 2;
    }
    Mesh::new(vertices, indices)
}

/// Axes appearance configuration
pub struct AxesConfig {
    pub show_top_spine: bool,
    pub show_right_spine: bool,
    pub show_bottom_spine: bool,
    pub show_left_spine: bool,
    pub show_grid: bool,
    pub spine_color: [f32; 3],
    pub grid_color: [f32; 3],
    pub tick_color: [f32; 3],
    pub origin_axis_color: [f32; 3],
}

impl Default for AxesConfig {
    fn default() -> Self {
        Self {
            show_top_spine: true,
            show_right_spine: true,
            show_bottom_spine: true,
            show_left_spine: true,
            show_grid: true,
            spine_color: [0.15, 0.15, 0.15],
            grid_color: [0.85, 0.85, 0.85],
            tick_color: [0.15, 0.15, 0.15],
            origin_axis_color: [0.6, 0.6, 0.6],
        }
    }
}

/// Create 2D axes with full configuration
pub fn create_2d_axes(
    x_min: f32,
    x_max: f32,
    y_min: f32,
    y_max: f32,
    x_ticks: usize,
    y_ticks: usize,
    show_grid: bool,
) -> Mesh {
    create_2d_axes_with_config(x_min, x_max, y_min, y_max, x_ticks, y_ticks, &AxesConfig {
        show_grid,
        ..Default::default()
    })
}

/// Create 2D axes with custom AxesConfig
pub fn create_2d_axes_with_config(
    x_min: f32,
    x_max: f32,
    y_min: f32,
    y_max: f32,
    x_ticks: usize,
    y_ticks: usize,
    config: &AxesConfig,
) -> Mesh {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    let x_step = (x_max - x_min) / x_ticks.max(1) as f32;
    let y_step = (y_max - y_min) / y_ticks.max(1) as f32;
    let tick_len = (y_max - y_min) * 0.015;

    // Spines
    if config.show_bottom_spine {
        add_line(&mut vertices, &mut indices, [x_min, y_min, 0.0], [x_max, y_min, 0.0], config.spine_color);
    }
    if config.show_left_spine {
        add_line(&mut vertices, &mut indices, [x_min, y_min, 0.0], [x_min, y_max, 0.0], config.spine_color);
    }
    if config.show_top_spine {
        add_line(&mut vertices, &mut indices, [x_min, y_max, 0.0], [x_max, y_max, 0.0], config.spine_color);
    }
    if config.show_right_spine {
        add_line(&mut vertices, &mut indices, [x_max, y_min, 0.0], [x_max, y_max, 0.0], config.spine_color);
    }

    // Grid lines
    if config.show_grid {
        for i in 1..x_ticks {
            let x = x_min + i as f32 * x_step;
            add_line(&mut vertices, &mut indices, [x, y_min, 0.0], [x, y_max, 0.0], config.grid_color);
        }
        for i in 1..y_ticks {
            let y = y_min + i as f32 * y_step;
            add_line(&mut vertices, &mut indices, [x_min, y, 0.0], [x_max, y, 0.0], config.grid_color);
        }
    }

    // Tick marks on bottom spine
    for i in 0..=x_ticks {
        let x = x_min + i as f32 * x_step;
        add_line(&mut vertices, &mut indices, [x, y_min, 0.0], [x, y_min - tick_len, 0.0], config.tick_color);
    }
    // Tick marks on left spine
    for i in 0..=y_ticks {
        let y = y_min + i as f32 * y_step;
        add_line(&mut vertices, &mut indices, [x_min, y, 0.0], [x_min - tick_len, y, 0.0], config.tick_color);
    }

    // Axis lines at origin if within bounds
    if x_min < 0.0 && x_max > 0.0 {
        add_line(&mut vertices, &mut indices, [0.0, y_min, 0.0], [0.0, y_max, 0.0], config.origin_axis_color);
    }
    if y_min < 0.0 && y_max > 0.0 {
        add_line(&mut vertices, &mut indices, [x_min, 0.0, 0.0], [x_max, 0.0, 0.0], config.origin_axis_color);
    }

    Mesh::new(vertices, indices)
}

/// Format a tick value for display
pub fn format_tick(v: f32) -> String {
    if (v - v.round()).abs() < 1e-4 {
        format!("{}", v as i32)
    } else {
        format!("{:.1}", v)
    }
}
