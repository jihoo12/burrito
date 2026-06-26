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

pub fn plot_2d_scatter(points: &[[f32; 2]], color: [f32; 3]) -> Mesh {
    let vertices: Vec<Vertex> = points
        .iter()
        .map(|&[x, y]| Vertex::new([x, y, 0.0], color))
        .collect();
    let indices: Vec<u32> = (0..points.len() as u32).collect();
    Mesh::new(vertices, indices)
}

pub fn create_2d_axes(
    x_min: f32,
    x_max: f32,
    y_min: f32,
    y_max: f32,
    x_ticks: usize,
    y_ticks: usize,
    show_grid: bool,
) -> Mesh {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    let spine = [0.15, 0.15, 0.15];
    let grid = [0.85, 0.85, 0.85];
    let tick = [0.15, 0.15, 0.15];

    let x_step = (x_max - x_min) / x_ticks.max(1) as f32;
    let y_step = (y_max - y_min) / y_ticks.max(1) as f32;
    let tick_len = (y_max - y_min) * 0.015;

    // Four spines
    add_line(&mut vertices, &mut indices, [x_min, y_min, 0.0], [x_max, y_min, 0.0], spine);
    add_line(&mut vertices, &mut indices, [x_min, y_min, 0.0], [x_min, y_max, 0.0], spine);
    add_line(&mut vertices, &mut indices, [x_min, y_max, 0.0], [x_max, y_max, 0.0], spine);
    add_line(&mut vertices, &mut indices, [x_max, y_min, 0.0], [x_max, y_max, 0.0], spine);

    if show_grid {
        for i in 1..x_ticks {
            let x = x_min + i as f32 * x_step;
            add_line(&mut vertices, &mut indices, [x, y_min, 0.0], [x, y_max, 0.0], grid);
        }
        for i in 1..y_ticks {
            let y = y_min + i as f32 * y_step;
            add_line(&mut vertices, &mut indices, [x_min, y, 0.0], [x_max, y, 0.0], grid);
        }
    }

    // Tick marks on bottom spine
    for i in 0..=x_ticks {
        let x = x_min + i as f32 * x_step;
        add_line(&mut vertices, &mut indices, [x, y_min, 0.0], [x, y_min - tick_len, 0.0], tick);
    }
    // Tick marks on left spine
    for i in 0..=y_ticks {
        let y = y_min + i as f32 * y_step;
        add_line(&mut vertices, &mut indices, [x_min, y, 0.0], [x_min - tick_len, y, 0.0], tick);
    }

    // Axis lines at origin if within bounds
    if x_min < 0.0 && x_max > 0.0 {
        add_line(&mut vertices, &mut indices, [0.0, y_min, 0.0], [0.0, y_max, 0.0], [0.6, 0.6, 0.6]);
    }
    if y_min < 0.0 && y_max > 0.0 {
        add_line(&mut vertices, &mut indices, [x_min, 0.0, 0.0], [x_max, 0.0, 0.0], [0.6, 0.6, 0.6]);
    }

    Mesh::new(vertices, indices)
}

/// Format a tick value for display.
pub fn format_tick(v: f32) -> String {
    if (v - v.round()).abs() < 1e-4 {
        format!("{}", v as i32)
    } else {
        format!("{:.1}", v)
    }
}
