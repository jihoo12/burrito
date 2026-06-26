mod camera;
mod config;
mod data;
mod geometry;
mod mesh;
mod plot2d;
mod renderer;
mod vertex;

pub use camera::Camera;
pub use config::{LegendEntry, PlotConfig};
pub use data::{Plot2DLine, Plot2DScatter, PlotData};
pub use geometry::{create_full_grid_data, plot_parametric_curve, plot_scatter, plot_wireframe};
pub use mesh::Mesh;
pub use plot2d::{create_2d_axes, plot_2d_line, plot_2d_scatter};
pub use renderer::App;
pub use vertex::Vertex;