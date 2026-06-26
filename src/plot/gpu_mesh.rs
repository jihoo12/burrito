use super::mesh::Mesh;
use wgpu::util::DeviceExt;

pub struct GpuMesh {
    pub vertex_buf: wgpu::Buffer,
    pub index_buf: wgpu::Buffer,
    pub index_count: u32,
}

impl GpuMesh {
    pub fn upload(device: &wgpu::Device, mesh: &Mesh) -> Self {
        let make_buf = |data: &[u8], usage: wgpu::BufferUsages| {
            let contents = if data.is_empty() { &[0u8; 4] } else { data };
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents,
                usage,
            })
        };
        Self {
            vertex_buf: make_buf(
                bytemuck::cast_slice(&mesh.vertices),
                wgpu::BufferUsages::VERTEX,
            ),
            index_buf: make_buf(
                bytemuck::cast_slice(&mesh.indices),
                wgpu::BufferUsages::INDEX,
            ),
            index_count: mesh.indices.len() as u32,
        }
    }

    #[allow(dead_code)]
    pub fn upload_dynamic(device: &wgpu::Device, mesh: &Mesh) -> Self {
        let vertex_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("anim vertex"),
            contents: bytemuck::cast_slice(&mesh.vertices),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
        let index_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("anim index"),
            contents: bytemuck::cast_slice(&mesh.indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        Self {
            vertex_buf,
            index_buf,
            index_count: mesh.indices.len() as u32,
        }
    }
}
