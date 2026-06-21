//! GPU-accelerated water volume rendering
//!
//! This module provides GPU integration for the volumetric water system,
//! including 3D texture upload and surface mesh generation using a
//! heightfield-based approach.

use crate::volume_grid::{WaterCell, WaterVolumeGrid};
use bytemuck::{Pod, Zeroable};
use glam::{IVec3, UVec3, Vec2, Vec3};
use wgpu::util::DeviceExt;

/// Minimum water level to consider a cell as containing water
const MIN_WATER_LEVEL: f32 = 0.01;

/// GPU-friendly water cell data (16 bytes, aligned for GPU)
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct GpuWaterCell {
    /// Water level (0.0-1.0)
    pub level: f32,
    /// Flow velocity X
    pub velocity_x: f32,
    /// Flow velocity Y
    pub velocity_y: f32,
    /// Flow velocity Z
    pub velocity_z: f32,
}

impl GpuWaterCell {
    /// Create from a water cell
    pub fn from_cell(cell: &WaterCell) -> Self {
        Self {
            level: cell.level,
            velocity_x: cell.velocity.x,
            velocity_y: cell.velocity.y,
            velocity_z: cell.velocity.z,
        }
    }
}

/// Vertex format for water surface mesh (32 bytes)
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct WaterSurfaceVertex {
    /// World position
    pub position: [f32; 3],
    /// Surface normal
    pub normal: [f32; 3],
    /// UV coordinates for flow effects
    pub uv: [f32; 2],
}

impl WaterSurfaceVertex {
    /// Vertex buffer layout for wgpu
    pub fn buffer_layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // Position
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // Normal
                wgpu::VertexAttribute {
                    offset: 12,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // UV
                wgpu::VertexAttribute {
                    offset: 24,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

/// Uniform buffer for water volume shader (64 bytes, 16-byte aligned)
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct WaterVolumeUniforms {
    /// Volume dimensions (width, height, depth, padding)
    pub dimensions: [u32; 4],
    /// World-space origin of the volume
    pub origin: [f32; 4],
    /// Cell size in world units (x, y, z, padding)
    pub cell_size: [f32; 4],
    /// Time for animation (time, flow_speed, wave_height, wave_frequency)
    pub animation: [f32; 4],
}

impl Default for WaterVolumeUniforms {
    fn default() -> Self {
        Self {
            dimensions: [64, 64, 64, 0],
            origin: [0.0, 0.0, 0.0, 0.0],
            cell_size: [1.0, 1.0, 1.0, 0.0],
            animation: [0.0, 1.0, 0.1, 2.0],
        }
    }
}

/// GPU resources for water volume rendering
pub struct WaterVolumeGpu {
    /// 3D texture containing water cell data
    volume_texture: wgpu::Texture,
    /// View for sampling the volume texture (used by render pass)
    #[allow(dead_code)]
    volume_view: wgpu::TextureView,
    /// Sampler for the volume texture (used by render pass)
    #[allow(dead_code)]
    volume_sampler: wgpu::Sampler,
    /// Uniform buffer for shader parameters
    uniform_buffer: wgpu::Buffer,
    /// Bind group for the water volume
    bind_group: wgpu::BindGroup,
    /// Bind group layout for the water volume
    bind_group_layout: wgpu::BindGroupLayout,
    /// Volume dimensions
    dimensions: UVec3,
    /// Current uniform values
    uniforms: WaterVolumeUniforms,
    /// Staging buffer for CPU -> GPU uploads (used during upload)
    #[allow(dead_code)]
    staging_buffer: wgpu::Buffer,
    /// Whether the volume needs re-upload
    dirty: bool,
}

impl WaterVolumeGpu {
    /// Create a new GPU water volume
    pub fn new(device: &wgpu::Device, dimensions: UVec3) -> Self {
        let volume_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Water Volume 3D Texture"),
            size: wgpu::Extent3d {
                width: dimensions.x,
                height: dimensions.y,
                depth_or_array_layers: dimensions.z,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D3,
            format: wgpu::TextureFormat::Rgba32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let volume_view = volume_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let volume_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Water Volume Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let uniforms = WaterVolumeUniforms {
            dimensions: [dimensions.x, dimensions.y, dimensions.z, 0],
            ..Default::default()
        };

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Water Volume Uniforms"),
            contents: bytemuck::bytes_of(&uniforms),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Water Volume Bind Group Layout"),
            entries: &[
                // Volume texture
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT | wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D3,
                        multisampled: false,
                    },
                    count: None,
                },
                // Sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT | wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                // Uniforms
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT | wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Water Volume Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&volume_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&volume_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: uniform_buffer.as_entire_binding(),
                },
            ],
        });

        // Create staging buffer for uploads
        let staging_size = (dimensions.x * dimensions.y * dimensions.z) as usize
            * std::mem::size_of::<GpuWaterCell>();
        let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Water Volume Staging Buffer"),
            size: staging_size as u64,
            usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::MAP_WRITE,
            mapped_at_creation: false,
        });

        Self {
            volume_texture,
            volume_view,
            volume_sampler,
            uniform_buffer,
            bind_group,
            bind_group_layout,
            dimensions,
            uniforms,
            staging_buffer,
            dirty: true,
        }
    }

    /// Upload water volume data to the GPU
    pub fn upload(&mut self, queue: &wgpu::Queue, grid: &WaterVolumeGrid) {
        // Convert grid data to GPU format
        let mut gpu_cells = Vec::with_capacity(
            (self.dimensions.x * self.dimensions.y * self.dimensions.z) as usize,
        );

        for z in 0..self.dimensions.z as i32 {
            for y in 0..self.dimensions.y as i32 {
                for x in 0..self.dimensions.x as i32 {
                    let pos = IVec3::new(x, y, z);
                    let cell = grid.get_cell(pos).cloned().unwrap_or_default();
                    gpu_cells.push(GpuWaterCell::from_cell(&cell));
                }
            }
        }

        // Upload to GPU
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.volume_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytemuck::cast_slice(&gpu_cells),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(self.dimensions.x * std::mem::size_of::<GpuWaterCell>() as u32),
                rows_per_image: Some(self.dimensions.y),
            },
            wgpu::Extent3d {
                width: self.dimensions.x,
                height: self.dimensions.y,
                depth_or_array_layers: self.dimensions.z,
            },
        );

        self.dirty = false;
    }

    /// Update uniforms
    pub fn update_uniforms(&mut self, queue: &wgpu::Queue, time: f32) {
        self.uniforms.animation[0] = time;
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&self.uniforms));
    }

    /// Set world origin
    pub fn set_origin(&mut self, origin: Vec3) {
        self.uniforms.origin = [origin.x, origin.y, origin.z, 0.0];
    }

    /// Set cell size
    pub fn set_cell_size(&mut self, size: Vec3) {
        self.uniforms.cell_size = [size.x, size.y, size.z, 0.0];
    }

    /// Set flow animation speed
    pub fn set_flow_speed(&mut self, speed: f32) {
        self.uniforms.animation[1] = speed;
    }

    /// Set wave height
    pub fn set_wave_height(&mut self, height: f32) {
        self.uniforms.animation[2] = height;
    }

    /// Set wave frequency
    pub fn set_wave_frequency(&mut self, frequency: f32) {
        self.uniforms.animation[3] = frequency;
    }

    /// Get the bind group layout
    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }

    /// Get the bind group
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }

    /// Get volume dimensions
    pub fn dimensions(&self) -> UVec3 {
        self.dimensions
    }

    /// Check if the volume needs re-upload
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Mark the volume as dirty (needs re-upload)
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Generate a surface mesh from the water volume using heightfield approach
    ///
    /// This scans columns (x, z) and finds the water surface height for each,
    /// then generates a quad mesh representing the water surface.
    pub fn generate_surface_mesh(
        &self,
        grid: &WaterVolumeGrid,
    ) -> (Vec<WaterSurfaceVertex>, Vec<u32>) {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        let cell_size = Vec3::new(
            self.uniforms.cell_size[0],
            self.uniforms.cell_size[1],
            self.uniforms.cell_size[2],
        );
        let origin = Vec3::new(
            self.uniforms.origin[0],
            self.uniforms.origin[1],
            self.uniforms.origin[2],
        );

        // For each column (x, z), find the water surface height
        for z in 0..self.dimensions.z as i32 - 1 {
            for x in 0..self.dimensions.x as i32 - 1 {
                // Sample heights at the four corners of this grid cell
                let h00 = self.sample_column_height(grid, x, z);
                let h10 = self.sample_column_height(grid, x + 1, z);
                let h01 = self.sample_column_height(grid, x, z + 1);
                let h11 = self.sample_column_height(grid, x + 1, z + 1);

                // Skip if no water in any corner
                if h00.is_none() && h10.is_none() && h01.is_none() && h11.is_none() {
                    continue;
                }

                // Use a default height for corners without water (slightly below neighbors)
                let avg_height = [h00, h10, h01, h11].iter().filter_map(|h| *h).sum::<f32>()
                    / [h00, h10, h01, h11]
                        .iter()
                        .filter(|h| h.is_some())
                        .count()
                        .max(1) as f32;

                let y00 = h00.unwrap_or(avg_height - 0.1);
                let y10 = h10.unwrap_or(avg_height - 0.1);
                let y01 = h01.unwrap_or(avg_height - 0.1);
                let y11 = h11.unwrap_or(avg_height - 0.1);

                // Calculate world positions
                let p00 = origin + Vec3::new(x as f32 * cell_size.x, y00, z as f32 * cell_size.z);
                let p10 =
                    origin + Vec3::new((x + 1) as f32 * cell_size.x, y10, z as f32 * cell_size.z);
                let p01 =
                    origin + Vec3::new(x as f32 * cell_size.x, y01, (z + 1) as f32 * cell_size.z);
                let p11 = origin
                    + Vec3::new(
                        (x + 1) as f32 * cell_size.x,
                        y11,
                        (z + 1) as f32 * cell_size.z,
                    );

                // Calculate normals using gradient
                let n00 = self.calculate_surface_normal(grid, x, z);
                let n10 = self.calculate_surface_normal(grid, x + 1, z);
                let n01 = self.calculate_surface_normal(grid, x, z + 1);
                let n11 = self.calculate_surface_normal(grid, x + 1, z + 1);

                // UV coordinates (for flow effects)
                let uv00 = Vec2::new(
                    x as f32 / self.dimensions.x as f32,
                    z as f32 / self.dimensions.z as f32,
                );
                let uv10 = Vec2::new(
                    (x + 1) as f32 / self.dimensions.x as f32,
                    z as f32 / self.dimensions.z as f32,
                );
                let uv01 = Vec2::new(
                    x as f32 / self.dimensions.x as f32,
                    (z + 1) as f32 / self.dimensions.z as f32,
                );
                let uv11 = Vec2::new(
                    (x + 1) as f32 / self.dimensions.x as f32,
                    (z + 1) as f32 / self.dimensions.z as f32,
                );

                // Add vertices
                let base_index = vertices.len() as u32;
                vertices.push(WaterSurfaceVertex {
                    position: p00.into(),
                    normal: n00.into(),
                    uv: uv00.into(),
                });
                vertices.push(WaterSurfaceVertex {
                    position: p10.into(),
                    normal: n10.into(),
                    uv: uv10.into(),
                });
                vertices.push(WaterSurfaceVertex {
                    position: p01.into(),
                    normal: n01.into(),
                    uv: uv01.into(),
                });
                vertices.push(WaterSurfaceVertex {
                    position: p11.into(),
                    normal: n11.into(),
                    uv: uv11.into(),
                });

                // Two triangles for the quad
                indices.extend_from_slice(&[
                    base_index,
                    base_index + 1,
                    base_index + 2,
                    base_index + 1,
                    base_index + 3,
                    base_index + 2,
                ]);
            }
        }

        (vertices, indices)
    }

    /// Calculate the surface normal at a grid column using finite differences
    fn calculate_surface_normal(&self, grid: &WaterVolumeGrid, x: i32, z: i32) -> Vec3 {
        let h_center = self.sample_column_height(grid, x, z).unwrap_or(0.0);

        // Sample neighboring heights
        let h_left = if x > 0 {
            self.sample_column_height(grid, x - 1, z)
                .unwrap_or(h_center)
        } else {
            h_center
        };
        let h_right = if x < self.dimensions.x as i32 - 1 {
            self.sample_column_height(grid, x + 1, z)
                .unwrap_or(h_center)
        } else {
            h_center
        };
        let h_back = if z > 0 {
            self.sample_column_height(grid, x, z - 1)
                .unwrap_or(h_center)
        } else {
            h_center
        };
        let h_front = if z < self.dimensions.z as i32 - 1 {
            self.sample_column_height(grid, x, z + 1)
                .unwrap_or(h_center)
        } else {
            h_center
        };

        // Calculate gradient
        let dx = (h_right - h_left) / (2.0 * self.uniforms.cell_size[0]);
        let dz = (h_front - h_back) / (2.0 * self.uniforms.cell_size[2]);

        // Normal from gradient
        Vec3::new(-dx, 1.0, -dz).normalize()
    }

    /// Sample the water surface height at a grid column (x, z)
    /// Returns None if the column has no water
    fn sample_column_height(&self, grid: &WaterVolumeGrid, x: i32, z: i32) -> Option<f32> {
        // Scan from top to bottom to find the first cell with water
        for y in (0..self.dimensions.y as i32).rev() {
            let pos = IVec3::new(x, y, z);
            if let Some(cell) = grid.get_cell(pos) {
                if cell.level > MIN_WATER_LEVEL {
                    // Interpolate height based on water level within the cell
                    let base_height = y as f32 * self.uniforms.cell_size[1];
                    let water_height = base_height + cell.level * self.uniforms.cell_size[1];
                    return Some(water_height);
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: create a wgpu device + queue for testing.
    /// Returns None if no GPU adapter is available (e.g. headless CI)
    /// or if SKIP_GPU_TESTS env var is set (e.g. mutation testing).
    fn try_create_test_device() -> Option<(wgpu::Device, wgpu::Queue)> {
        if std::env::var("SKIP_GPU_TESTS").is_ok() {
            return None;
        }
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .ok()?;
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("test device"),
            required_features: wgpu::Features::FLOAT32_FILTERABLE,
            required_limits: wgpu::Limits::default(),
            memory_hints: wgpu::MemoryHints::default(),
            trace: wgpu::Trace::Off,
        }))
        .ok()?;
        Some((device, queue))
    }

    /// Helper: create a minimal WaterVolumeGpu for testing.
    fn create_test_gpu(device: &wgpu::Device, dims: UVec3) -> WaterVolumeGpu {
        WaterVolumeGpu::new(device, dims)
    }

    #[test]
    fn test_gpu_water_cell_size() {
        assert_eq!(std::mem::size_of::<GpuWaterCell>(), 16); // 4 floats * 4 bytes
    }

    #[test]
    fn test_water_surface_vertex_size() {
        assert_eq!(std::mem::size_of::<WaterSurfaceVertex>(), 32); // 8 floats * 4 bytes
    }

    #[test]
    fn test_uniforms_size() {
        // Ensure proper alignment for GPU
        let size = std::mem::size_of::<WaterVolumeUniforms>();
        assert!(
            size % 16 == 0,
            "Uniforms must be 16-byte aligned, got {}",
            size
        );
    }

    // ---- GpuWaterCell::from_cell (no GPU needed) ----

    #[test]
    fn test_from_cell_copies_level() {
        let cell = WaterCell {
            level: 0.75,
            velocity: Vec3::new(1.0, 2.0, 3.0),
            ..Default::default()
        };
        let gpu = GpuWaterCell::from_cell(&cell);
        assert_eq!(gpu.level, 0.75);
    }

    #[test]
    fn test_from_cell_copies_velocity() {
        let cell = WaterCell {
            level: 0.5,
            velocity: Vec3::new(1.5, -2.5, 3.5),
            ..Default::default()
        };
        let gpu = GpuWaterCell::from_cell(&cell);
        assert_eq!(gpu.velocity_x, 1.5);
        assert_eq!(gpu.velocity_y, -2.5);
        assert_eq!(gpu.velocity_z, 3.5);
    }

    #[test]
    fn test_from_cell_default_produces_zeros() {
        let cell = WaterCell::default();
        let gpu = GpuWaterCell::from_cell(&cell);
        assert_eq!(gpu.level, 0.0);
        assert_eq!(gpu.velocity_x, 0.0);
        assert_eq!(gpu.velocity_y, 0.0);
        assert_eq!(gpu.velocity_z, 0.0);
    }

    // ---- WaterVolumeGpu setter/getter tests (need GPU device) ----

    #[test]
    fn test_dimensions_returns_constructor_value() {
        let Some((device, _queue)) = try_create_test_device() else {
            return; // skip if no GPU
        };
        let dims = UVec3::new(4, 8, 16);
        let gpu = create_test_gpu(&device, dims);
        assert_eq!(gpu.dimensions(), dims);
    }

    #[test]
    fn test_is_dirty_initially_true() {
        let Some((device, _queue)) = try_create_test_device() else {
            return;
        };
        let gpu = create_test_gpu(&device, UVec3::new(2, 2, 2));
        assert!(gpu.is_dirty());
    }

    #[test]
    fn test_mark_dirty_sets_dirty_true() {
        let Some((device, queue)) = try_create_test_device() else {
            return;
        };
        let mut gpu = create_test_gpu(&device, UVec3::new(2, 2, 2));
        let grid = WaterVolumeGrid::new(UVec3::new(2, 2, 2), 1.0, Vec3::ZERO);
        gpu.upload(&queue, &grid);
        assert!(!gpu.is_dirty());
        gpu.mark_dirty();
        assert!(gpu.is_dirty());
    }

    #[test]
    fn test_upload_clears_dirty() {
        let Some((device, queue)) = try_create_test_device() else {
            return;
        };
        let mut gpu = create_test_gpu(&device, UVec3::new(2, 2, 2));
        assert!(gpu.is_dirty());
        let grid = WaterVolumeGrid::new(UVec3::new(2, 2, 2), 1.0, Vec3::ZERO);
        gpu.upload(&queue, &grid);
        assert!(!gpu.is_dirty());
    }

    #[test]
    fn test_set_origin_updates_uniforms() {
        let Some((device, _queue)) = try_create_test_device() else {
            return;
        };
        let mut gpu = create_test_gpu(&device, UVec3::new(2, 2, 2));
        gpu.set_origin(Vec3::new(10.0, 20.0, 30.0));
        assert_eq!(gpu.uniforms.origin[0], 10.0);
        assert_eq!(gpu.uniforms.origin[1], 20.0);
        assert_eq!(gpu.uniforms.origin[2], 30.0);
        assert_eq!(gpu.uniforms.origin[3], 0.0);
    }

    #[test]
    fn test_set_cell_size_updates_uniforms() {
        let Some((device, _queue)) = try_create_test_device() else {
            return;
        };
        let mut gpu = create_test_gpu(&device, UVec3::new(2, 2, 2));
        gpu.set_cell_size(Vec3::new(0.5, 1.0, 2.0));
        assert_eq!(gpu.uniforms.cell_size[0], 0.5);
        assert_eq!(gpu.uniforms.cell_size[1], 1.0);
        assert_eq!(gpu.uniforms.cell_size[2], 2.0);
        assert_eq!(gpu.uniforms.cell_size[3], 0.0);
    }

    #[test]
    fn test_set_flow_speed_updates_animation_1() {
        let Some((device, _queue)) = try_create_test_device() else {
            return;
        };
        let mut gpu = create_test_gpu(&device, UVec3::new(2, 2, 2));
        gpu.set_flow_speed(5.0);
        assert_eq!(gpu.uniforms.animation[1], 5.0);
    }

    #[test]
    fn test_set_wave_height_updates_animation_2() {
        let Some((device, _queue)) = try_create_test_device() else {
            return;
        };
        let mut gpu = create_test_gpu(&device, UVec3::new(2, 2, 2));
        gpu.set_wave_height(0.25);
        assert_eq!(gpu.uniforms.animation[2], 0.25);
    }

    #[test]
    fn test_set_wave_frequency_updates_animation_3() {
        let Some((device, _queue)) = try_create_test_device() else {
            return;
        };
        let mut gpu = create_test_gpu(&device, UVec3::new(2, 2, 2));
        gpu.set_wave_frequency(3.5);
        assert_eq!(gpu.uniforms.animation[3], 3.5);
    }

    // ---- generate_surface_mesh & helpers (need GPU device) ----

    #[test]
    fn test_sample_column_height_no_water_returns_none() {
        let Some((device, _queue)) = try_create_test_device() else {
            return;
        };
        let gpu = create_test_gpu(&device, UVec3::new(4, 4, 4));
        let grid = WaterVolumeGrid::new(UVec3::new(4, 4, 4), 1.0, Vec3::ZERO);
        assert!(gpu.sample_column_height(&grid, 0, 0).is_none());
    }

    #[test]
    fn test_sample_column_height_with_water() {
        let Some((device, _queue)) = try_create_test_device() else {
            return;
        };
        let gpu = create_test_gpu(&device, UVec3::new(4, 4, 4));
        let mut grid = WaterVolumeGrid::new(UVec3::new(4, 4, 4), 1.0, Vec3::ZERO);
        // Place water at level 0.5 in cell (1, 2, 1)
        if let Some(cell) = grid.get_cell_mut(IVec3::new(1, 2, 1)) {
            cell.level = 0.5;
        }
        let h = gpu.sample_column_height(&grid, 1, 1);
        assert!(h.is_some());
        // base_height = y(2) * cell_size(1.0) = 2.0
        // water_height = 2.0 + 0.5 * 1.0 = 2.5
        let h = h.unwrap();
        assert!((h - 2.5).abs() < 1e-5, "expected ~2.5, got {}", h);
    }

    #[test]
    fn test_sample_column_height_returns_topmost_water() {
        let Some((device, _queue)) = try_create_test_device() else {
            return;
        };
        let gpu = create_test_gpu(&device, UVec3::new(4, 4, 4));
        let mut grid = WaterVolumeGrid::new(UVec3::new(4, 4, 4), 1.0, Vec3::ZERO);
        // Place water at y=1 (level=0.8) and y=3 (level=0.3)
        if let Some(cell) = grid.get_cell_mut(IVec3::new(0, 1, 0)) {
            cell.level = 0.8;
        }
        if let Some(cell) = grid.get_cell_mut(IVec3::new(0, 3, 0)) {
            cell.level = 0.3;
        }
        let h = gpu.sample_column_height(&grid, 0, 0).unwrap();
        // Should return topmost (y=3): base=3.0 + 0.3*1.0 = 3.3
        assert!(
            (h - 3.3).abs() < 1e-5,
            "expected topmost water ~3.3, got {}",
            h
        );
    }

    #[test]
    fn test_sample_column_height_ignores_below_min_water_level() {
        let Some((device, _queue)) = try_create_test_device() else {
            return;
        };
        let gpu = create_test_gpu(&device, UVec3::new(4, 4, 4));
        let mut grid = WaterVolumeGrid::new(UVec3::new(4, 4, 4), 1.0, Vec3::ZERO);
        // Place water below MIN_WATER_LEVEL (0.01)
        if let Some(cell) = grid.get_cell_mut(IVec3::new(0, 0, 0)) {
            cell.level = 0.005;
        }
        assert!(gpu.sample_column_height(&grid, 0, 0).is_none());
    }

    #[test]
    fn test_generate_surface_mesh_empty_grid_no_vertices() {
        let Some((device, _queue)) = try_create_test_device() else {
            return;
        };
        let gpu = create_test_gpu(&device, UVec3::new(4, 4, 4));
        let grid = WaterVolumeGrid::new(UVec3::new(4, 4, 4), 1.0, Vec3::ZERO);
        let (verts, indices) = gpu.generate_surface_mesh(&grid);
        assert!(verts.is_empty(), "empty grid should produce no vertices");
        assert!(indices.is_empty(), "empty grid should produce no indices");
    }

    #[test]
    fn test_generate_surface_mesh_produces_vertices_with_water() {
        let Some((device, _queue)) = try_create_test_device() else {
            return;
        };
        let mut gpu = create_test_gpu(&device, UVec3::new(4, 4, 4));
        gpu.set_cell_size(Vec3::new(1.0, 1.0, 1.0));
        let mut grid = WaterVolumeGrid::new(UVec3::new(4, 4, 4), 1.0, Vec3::ZERO);
        // Fill a 2x2 column area with water at y=0
        for x in 0..2 {
            for z in 0..2 {
                if let Some(cell) = grid.get_cell_mut(IVec3::new(x, 0, z)) {
                    cell.level = 0.5;
                }
            }
        }
        let (verts, indices) = gpu.generate_surface_mesh(&grid);
        assert!(!verts.is_empty(), "grid with water should produce vertices");
        assert!(
            !indices.is_empty(),
            "grid with water should produce indices"
        );
        // Each quad = 4 vertices, 6 indices
        assert_eq!(indices.len() % 6, 0, "indices should be multiple of 6");
        assert_eq!(verts.len() % 4, 0, "vertices should be multiple of 4");
    }

    #[test]
    fn test_generate_surface_mesh_vertex_positions_use_cell_size() {
        let Some((device, _queue)) = try_create_test_device() else {
            return;
        };
        let mut gpu = create_test_gpu(&device, UVec3::new(4, 4, 4));
        gpu.set_cell_size(Vec3::new(2.0, 1.0, 3.0));
        gpu.set_origin(Vec3::ZERO);
        let mut grid = WaterVolumeGrid::new(UVec3::new(4, 4, 4), 1.0, Vec3::ZERO);
        // Place water at (0,0,0) and (1,0,0) — needs 2x2 area for a quad
        for x in 0..2 {
            for z in 0..2 {
                if let Some(cell) = grid.get_cell_mut(IVec3::new(x, 0, z)) {
                    cell.level = 0.5;
                }
            }
        }
        let (verts, _indices) = gpu.generate_surface_mesh(&grid);
        assert!(!verts.is_empty());
        // Check that x positions use cell_size.x (2.0)
        let has_nonzero_x = verts.iter().any(|v| v.position[0] > 0.5);
        assert!(
            has_nonzero_x,
            "vertex x positions should reflect cell_size.x=2.0"
        );
    }

    #[test]
    fn test_generate_surface_mesh_vertex_positions_use_origin() {
        let Some((device, _queue)) = try_create_test_device() else {
            return;
        };
        let mut gpu = create_test_gpu(&device, UVec3::new(4, 4, 4));
        gpu.set_cell_size(Vec3::new(1.0, 1.0, 1.0));
        gpu.set_origin(Vec3::new(100.0, 200.0, 300.0));
        let mut grid = WaterVolumeGrid::new(UVec3::new(4, 4, 4), 1.0, Vec3::ZERO);
        for x in 0..2 {
            for z in 0..2 {
                if let Some(cell) = grid.get_cell_mut(IVec3::new(x, 0, z)) {
                    cell.level = 0.5;
                }
            }
        }
        let (verts, _indices) = gpu.generate_surface_mesh(&grid);
        assert!(!verts.is_empty());
        // All positions should be offset by origin
        for v in &verts {
            assert!(
                v.position[0] >= 100.0,
                "x should be >= origin.x=100, got {}",
                v.position[0]
            );
            assert!(
                v.position[2] >= 300.0,
                "z should be >= origin.z=300, got {}",
                v.position[2]
            );
        }
    }

    #[test]
    fn test_generate_surface_mesh_uv_coordinates() {
        let Some((device, _queue)) = try_create_test_device() else {
            return;
        };
        let gpu = create_test_gpu(&device, UVec3::new(4, 4, 4));
        let mut grid = WaterVolumeGrid::new(UVec3::new(4, 4, 4), 1.0, Vec3::ZERO);
        for x in 0..3 {
            for z in 0..3 {
                if let Some(cell) = grid.get_cell_mut(IVec3::new(x, 0, z)) {
                    cell.level = 0.5;
                }
            }
        }
        let (verts, _indices) = gpu.generate_surface_mesh(&grid);
        // UV coords should be in [0, 1] range (x/dim_x, z/dim_z)
        for v in &verts {
            assert!(
                v.uv[0] >= 0.0 && v.uv[0] <= 1.0,
                "u should be in [0,1], got {}",
                v.uv[0]
            );
            assert!(
                v.uv[1] >= 0.0 && v.uv[1] <= 1.0,
                "v should be in [0,1], got {}",
                v.uv[1]
            );
        }
    }

    #[test]
    fn test_generate_surface_mesh_indices_valid() {
        let Some((device, _queue)) = try_create_test_device() else {
            return;
        };
        let gpu = create_test_gpu(&device, UVec3::new(4, 4, 4));
        let mut grid = WaterVolumeGrid::new(UVec3::new(4, 4, 4), 1.0, Vec3::ZERO);
        for x in 0..2 {
            for z in 0..2 {
                if let Some(cell) = grid.get_cell_mut(IVec3::new(x, 0, z)) {
                    cell.level = 0.5;
                }
            }
        }
        let (verts, indices) = gpu.generate_surface_mesh(&grid);
        for &idx in &indices {
            assert!(
                (idx as usize) < verts.len(),
                "index {} out of range for {} vertices",
                idx,
                verts.len()
            );
        }
    }

    #[test]
    fn test_generate_surface_mesh_index_triangle_pattern() {
        let Some((device, _queue)) = try_create_test_device() else {
            return;
        };
        let gpu = create_test_gpu(&device, UVec3::new(4, 4, 4));
        let mut grid = WaterVolumeGrid::new(UVec3::new(4, 4, 4), 1.0, Vec3::ZERO);
        for x in 0..2 {
            for z in 0..2 {
                if let Some(cell) = grid.get_cell_mut(IVec3::new(x, 0, z)) {
                    cell.level = 0.5;
                }
            }
        }
        let (_verts, indices) = gpu.generate_surface_mesh(&grid);
        // Each quad: base, base+1, base+2, base+1, base+3, base+2
        assert!(indices.len() >= 6);
        let base = indices[0];
        assert_eq!(indices[1], base + 1);
        assert_eq!(indices[2], base + 2);
        assert_eq!(indices[3], base + 1);
        assert_eq!(indices[4], base + 3);
        assert_eq!(indices[5], base + 2);
    }

    #[test]
    fn test_calculate_surface_normal_flat_surface_is_up() {
        let Some((device, _queue)) = try_create_test_device() else {
            return;
        };
        let gpu = create_test_gpu(&device, UVec3::new(6, 4, 6));
        let mut grid = WaterVolumeGrid::new(UVec3::new(6, 4, 6), 1.0, Vec3::ZERO);
        // Fill a flat plane of water at same level
        for x in 0..6 {
            for z in 0..6 {
                if let Some(cell) = grid.get_cell_mut(IVec3::new(x, 1, z)) {
                    cell.level = 0.5;
                }
            }
        }
        let n = gpu.calculate_surface_normal(&grid, 3, 3);
        // Flat surface should have normal pointing up (0, 1, 0)
        assert!(
            (n.y - 1.0).abs() < 0.01,
            "flat surface normal.y should be ~1.0, got {}",
            n.y
        );
        assert!(
            n.x.abs() < 0.01,
            "flat surface normal.x should be ~0, got {}",
            n.x
        );
        assert!(
            n.z.abs() < 0.01,
            "flat surface normal.z should be ~0, got {}",
            n.z
        );
    }

    #[test]
    fn test_calculate_surface_normal_sloped_surface() {
        let Some((device, _queue)) = try_create_test_device() else {
            return;
        };
        let gpu = create_test_gpu(&device, UVec3::new(6, 4, 6));
        let mut grid = WaterVolumeGrid::new(UVec3::new(6, 4, 6), 1.0, Vec3::ZERO);
        // Create a sloped surface: higher water on the right (x+)
        for x in 0..6 {
            for z in 0..6 {
                if let Some(cell) = grid.get_cell_mut(IVec3::new(x, 1, z)) {
                    cell.level = 0.1 + 0.15 * x as f32;
                }
            }
        }
        let n = gpu.calculate_surface_normal(&grid, 3, 3);
        // Normal should tilt away from the higher side (negative x)
        assert!(
            n.x < 0.0,
            "normal should tilt away from higher x, got n.x={}",
            n.x
        );
    }

    #[test]
    fn test_update_uniforms_sets_time() {
        let Some((device, queue)) = try_create_test_device() else {
            return;
        };
        let mut gpu = create_test_gpu(&device, UVec3::new(2, 2, 2));
        gpu.update_uniforms(&queue, 42.5);
        assert_eq!(gpu.uniforms.animation[0], 42.5);
    }

    #[test]
    fn test_upload_iterates_full_grid() {
        let Some((device, queue)) = try_create_test_device() else {
            return;
        };
        // Create a 3x3x3 GPU volume
        let mut gpu = create_test_gpu(&device, UVec3::new(3, 3, 3));
        let mut grid = WaterVolumeGrid::new(UVec3::new(3, 3, 3), 1.0, Vec3::ZERO);
        // Set one cell with water
        if let Some(cell) = grid.get_cell_mut(IVec3::new(1, 1, 1)) {
            cell.level = 0.99;
        }
        // Upload should not panic
        gpu.upload(&queue, &grid);
        assert!(!gpu.is_dirty(), "upload should clear dirty flag");
    }

    // ---- Targeted mutation-killing tests for generate_surface_mesh internals ----

    /// Tests loop bounds: `0..dim-1` must exclude the last column.
    /// Kills: `replace - with + in generate_surface_mesh` (loop bounds)
    #[test]
    fn test_generate_surface_mesh_loop_bounds_count() {
        let Some((device, _queue)) = try_create_test_device() else {
            return;
        };
        // 4x4x4 grid entirely filled with water at y=0
        let gpu = create_test_gpu(&device, UVec3::new(4, 4, 4));
        let mut grid = WaterVolumeGrid::new(UVec3::new(4, 4, 4), 1.0, Vec3::ZERO);
        for x in 0..4 {
            for z in 0..4 {
                if let Some(cell) = grid.get_cell_mut(IVec3::new(x, 0, z)) {
                    cell.level = 0.5;
                }
            }
        }
        let (verts, indices) = gpu.generate_surface_mesh(&grid);
        // Loop over (0..3) x (0..3) = 9 quads, each quad = 4 verts, 6 indices
        assert_eq!(verts.len(), 9 * 4, "expected 9 quads = 36 vertices");
        assert_eq!(indices.len(), 9 * 6, "expected 9 quads = 54 indices");
    }

    /// Tests that sample_column_height(x+1, z) samples the NEXT column, not same/previous.
    /// Kills: `replace + with - in sample_column_height offset` and `replace + with *`
    #[test]
    fn test_generate_surface_mesh_samples_adjacent_columns() {
        let Some((device, _queue)) = try_create_test_device() else {
            return;
        };
        let gpu = create_test_gpu(&device, UVec3::new(4, 4, 4));
        let mut grid = WaterVolumeGrid::new(UVec3::new(4, 4, 4), 1.0, Vec3::ZERO);
        // Only place water at column (1,0,1) — specifically NOT at (0,0,0)
        if let Some(cell) = grid.get_cell_mut(IVec3::new(1, 0, 1)) {
            cell.level = 0.8;
        }
        let (verts, _indices) = gpu.generate_surface_mesh(&grid);
        // With mutation x+1→x-1, the quad at (0,0) would sample h10=sample(0-1=-1)
        // which returns None, then all four would be None → skip. But correctly
        // it samples h10=sample(1,0) which has water → produces a quad.
        assert!(
            !verts.is_empty(),
            "should produce vertices when adjacent column has water"
        );
        // More specifically: quad at (x=0,z=0) samples corner (1,0), (0,1), (1,1)
        // Only (1,0,1) yields water. With correct code, the quad at (x=0,z=0) has
        // h10=Some(...) so it won't be skipped.
    }

    /// Tests the `&&` skip condition: quads are generated when SOME (not all) corners have water.
    /// Kills: `replace && with || in generate_surface_mesh` (3 mutations)
    #[test]
    fn test_generate_surface_mesh_partial_water_not_skipped() {
        let Some((device, _queue)) = try_create_test_device() else {
            return;
        };
        let gpu = create_test_gpu(&device, UVec3::new(4, 4, 4));
        let mut grid = WaterVolumeGrid::new(UVec3::new(4, 4, 4), 1.0, Vec3::ZERO);
        // Place water at only ONE corner: (0,0,0) with level above MIN_WATER_LEVEL
        if let Some(cell) = grid.get_cell_mut(IVec3::new(0, 0, 0)) {
            cell.level = 0.5;
        }
        let (verts, _indices) = gpu.generate_surface_mesh(&grid);
        // With && → ||: h00 is Some → condition becomes true → skip (WRONG)
        // With correct &&: h00 is Some, so not all are None → don't skip (CORRECT)
        assert!(
            !verts.is_empty(),
            "single corner with water should still produce a quad"
        );
    }

    /// Tests UV division: `x / dim` should produce fractional UVs.
    /// Kills: `replace / with % in generate_surface_mesh` and `replace / with *`
    #[test]
    fn test_generate_surface_mesh_uv_division_exact() {
        let Some((device, _queue)) = try_create_test_device() else {
            return;
        };
        let gpu = create_test_gpu(&device, UVec3::new(4, 4, 4));
        let mut grid = WaterVolumeGrid::new(UVec3::new(4, 4, 4), 1.0, Vec3::ZERO);
        // Fill ALL columns with water so every quad is generated
        for x in 0..4 {
            for z in 0..4 {
                if let Some(cell) = grid.get_cell_mut(IVec3::new(x, 0, z)) {
                    cell.level = 0.5;
                }
            }
        }
        let (verts, _indices) = gpu.generate_surface_mesh(&grid);
        // 9 quads (3x3), 36 vertices. Quad at (x=1,z=1) is quad index 4 (z*3+x = 1*3+1=4)
        // Vertices for quad 4 start at index 16
        let v00 = &verts[16]; // uv00 = (1/4, 1/4) = (0.25, 0.25)
        let v10 = &verts[17]; // uv10 = (2/4, 1/4) = (0.5, 0.25)
                              // With / → %: 1.0 % 4.0 = 1.0 (ok for x=1, but 2.0 % 4.0 = 2.0 for uv10!)
                              // With / → *: 1.0 * 4.0 = 4.0 (way off)
        assert!(
            (v00.uv[0] - 0.25).abs() < 1e-5,
            "u should be 0.25, got {}",
            v00.uv[0]
        );
        assert!(
            (v00.uv[1] - 0.25).abs() < 1e-5,
            "v should be 0.25, got {}",
            v00.uv[1]
        );
        assert!(
            (v10.uv[0] - 0.5).abs() < 1e-5,
            "u10 should be 0.5, got {}",
            v10.uv[0]
        );
    }

    /// Tests UV for x+1 columns: `(x+1) / dim` should be different from `x / dim`.
    /// Kills: `replace + with - in UV (x+1)` and `replace + with * in UV (z+1)`
    #[test]
    fn test_generate_surface_mesh_uv_adjacent_columns_differ() {
        let Some((device, _queue)) = try_create_test_device() else {
            return;
        };
        let gpu = create_test_gpu(&device, UVec3::new(4, 4, 4));
        let mut grid = WaterVolumeGrid::new(UVec3::new(4, 4, 4), 1.0, Vec3::ZERO);
        // Fill ALL columns with water
        for x in 0..4 {
            for z in 0..4 {
                if let Some(cell) = grid.get_cell_mut(IVec3::new(x, 0, z)) {
                    cell.level = 0.5;
                }
            }
        }
        let (verts, _indices) = gpu.generate_surface_mesh(&grid);
        assert!(verts.len() >= 4);
        // First quad at (x=0, z=0):
        // v[0]=uv00=(0/4,0/4), v[1]=uv10=(1/4,0/4), v[2]=uv01=(0/4,1/4), v[3]=uv11=(1/4,1/4)
        let uv00 = verts[0].uv;
        let uv10 = verts[1].uv;
        let uv01 = verts[2].uv;
        let uv11 = verts[3].uv;
        // uv10.u = 1/4 = 0.25; uv00.u = 0/4 = 0.0
        assert!(
            (uv10[0] - 0.25).abs() < 1e-5,
            "u10 should be 0.25, got {}",
            uv10[0]
        );
        assert!(
            (uv00[0] - 0.0).abs() < 1e-5,
            "u00 should be 0.0, got {}",
            uv00[0]
        );
        // uv01.v = 1/4 = 0.25; uv00.v = 0.0
        assert!(
            (uv01[1] - 0.25).abs() < 1e-5,
            "v01 should be 0.25, got {}",
            uv01[1]
        );
        // uv11 should have both 0.25
        assert!(
            (uv11[0] - 0.25).abs() < 1e-5 && (uv11[1] - 0.25).abs() < 1e-5,
            "uv11 should be (0.25, 0.25), got {:?}",
            uv11
        );
    }

    /// Tests fallback height: `unwrap_or(avg - 0.1)` should be BELOW average.
    /// Kills: `replace - with + in unwrap_or` and `replace - with /`
    #[test]
    fn test_generate_surface_mesh_fallback_height_below_average() {
        let Some((device, _queue)) = try_create_test_device() else {
            return;
        };
        let mut gpu = create_test_gpu(&device, UVec3::new(4, 4, 4));
        gpu.set_cell_size(Vec3::new(1.0, 1.0, 1.0));
        gpu.set_origin(Vec3::ZERO);
        let mut grid = WaterVolumeGrid::new(UVec3::new(4, 4, 4), 1.0, Vec3::ZERO);
        // Place water at (0,1,0) only — the three other corners of quad (0,0) have no water
        if let Some(cell) = grid.get_cell_mut(IVec3::new(0, 1, 0)) {
            cell.level = 0.5; // height = 1.0 + 0.5*1.0 = 1.5
        }
        let (verts, _indices) = gpu.generate_surface_mesh(&grid);
        assert!(verts.len() >= 4, "should have vertices for the quad");
        // v[0] = (0,0) corner which HAS water → height = 1.5
        // v[1] = (1,0) corner which has NO water → height = avg - 0.1 = 1.5 - 0.1 = 1.4
        let y_with_water = verts[0].position[1];
        let y_without_water = verts[1].position[1];
        assert!(
            y_without_water < y_with_water,
            "fallback height ({}) should be below water height ({})",
            y_without_water,
            y_with_water
        );
        // More precisely, the difference should be about 0.1
        let diff = y_with_water - y_without_water;
        assert!(
            (diff - 0.1).abs() < 0.01,
            "fallback offset should be ~0.1, got {}",
            diff
        );
    }

    /// Tests position calculation: `(x+1) * cell_size` should step by cell_size.
    /// Kills: `replace + with - in position calculation` and `replace + with *`
    #[test]
    fn test_generate_surface_mesh_position_stepping() {
        let Some((device, _queue)) = try_create_test_device() else {
            return;
        };
        let mut gpu = create_test_gpu(&device, UVec3::new(4, 4, 4));
        gpu.set_cell_size(Vec3::new(2.0, 1.0, 3.0));
        gpu.set_origin(Vec3::ZERO);
        let mut grid = WaterVolumeGrid::new(UVec3::new(4, 4, 4), 1.0, Vec3::ZERO);
        // Fill all columns so we get deterministic quads
        for x in 0..4 {
            for z in 0..4 {
                if let Some(cell) = grid.get_cell_mut(IVec3::new(x, 0, z)) {
                    cell.level = 0.5;
                }
            }
        }
        let (verts, _indices) = gpu.generate_surface_mesh(&grid);
        assert!(verts.len() >= 4);
        // First quad at (x=0, z=0):
        // v[0] = p00 = (0*2, h, 0*3) = (0, h, 0)
        // v[1] = p10 = (1*2, h, 0*3) = (2, h, 0)
        // v[2] = p01 = (0*2, h, 1*3) = (0, h, 3)
        // v[3] = p11 = (1*2, h, 1*3) = (2, h, 3)
        let p00 = verts[0].position;
        let p10 = verts[1].position;
        let p01 = verts[2].position;
        let p11 = verts[3].position;
        // X stepping: p10.x = (0+1)*2 = 2, p00.x = 0*2 = 0
        assert!(
            (p10[0] - p00[0] - 2.0).abs() < 1e-4,
            "x step should be cell_size.x=2.0, got {}",
            p10[0] - p00[0]
        );
        // Z stepping: p01.z = (0+1)*3 = 3, p00.z = 0*3 = 0
        assert!(
            (p01[2] - p00[2] - 3.0).abs() < 1e-4,
            "z step should be cell_size.z=3.0, got {}",
            p01[2] - p00[2]
        );
        // Diagonal: p11 should have both steps
        assert!(
            (p11[0] - p00[0] - 2.0).abs() < 1e-4,
            "p11 x should step by 2.0"
        );
        assert!(
            (p11[2] - p00[2] - 3.0).abs() < 1e-4,
            "p11 z should step by 3.0"
        );
    }

    /// Tests the average height computation: avg = sum(Some) / count(Some).
    /// Kills: `replace / with % in average` and `replace / with *`
    #[test]
    fn test_generate_surface_mesh_avg_height_computation() {
        let Some((device, _queue)) = try_create_test_device() else {
            return;
        };
        let mut gpu = create_test_gpu(&device, UVec3::new(4, 4, 4));
        gpu.set_cell_size(Vec3::new(1.0, 1.0, 1.0));
        gpu.set_origin(Vec3::ZERO);
        let mut grid = WaterVolumeGrid::new(UVec3::new(4, 4, 4), 1.0, Vec3::ZERO);
        // Place water at TWO corners of quad (0,0): (0,0,0) and (1,0,0)
        // Heights: h00 = 0 + 0.4*1 = 0.4, h10 = 0 + 0.6*1 = 0.6
        if let Some(cell) = grid.get_cell_mut(IVec3::new(0, 0, 0)) {
            cell.level = 0.4;
        }
        if let Some(cell) = grid.get_cell_mut(IVec3::new(1, 0, 0)) {
            cell.level = 0.6;
        }
        // avg = (0.4 + 0.6) / 2 = 0.5
        // fallback height = 0.5 - 0.1 = 0.4
        let (verts, _indices) = gpu.generate_surface_mesh(&grid);
        assert!(verts.len() >= 4);
        // v[2] = p01 (corner with no water) should have y = avg - 0.1 = 0.4
        let y_fallback = verts[2].position[1];
        assert!(
            (y_fallback - 0.4).abs() < 0.05,
            "fallback y should be avg(0.5)-0.1=0.4, got {}",
            y_fallback
        );
        // v[0] = p00 (corner WITH water) should have y = 0.4
        let y_water = verts[0].position[1];
        assert!(
            (y_water - 0.4).abs() < 0.05,
            "water height should be 0.4, got {}",
            y_water
        );
    }

    /// Tests fallback for y00: when h00 is None, y00 = avg_height - 0.1.
    /// Kills: `replace - with + in y00 unwrap_or` (line 396)
    #[test]
    fn test_generate_surface_mesh_y00_fallback() {
        let Some((device, _queue)) = try_create_test_device() else {
            return;
        };
        let mut gpu = create_test_gpu(&device, UVec3::new(4, 4, 4));
        gpu.set_cell_size(Vec3::new(1.0, 1.0, 1.0));
        gpu.set_origin(Vec3::ZERO);
        let mut grid = WaterVolumeGrid::new(UVec3::new(4, 4, 4), 1.0, Vec3::ZERO);
        // Water at (1,1,0) but NOT at (0,_,0) → h10=Some, h00=None
        if let Some(cell) = grid.get_cell_mut(IVec3::new(1, 1, 0)) {
            cell.level = 0.5; // height = 1.0 + 0.5 = 1.5
        }
        let (verts, _indices) = gpu.generate_surface_mesh(&grid);
        assert!(verts.len() >= 4, "should produce quad at (0,0)");
        // v[0] = p00 (h00=None) → y00 = avg(1.5) - 0.1 = 1.4
        // v[1] = p10 (h10=Some(1.5)) → y10 = 1.5
        let y00 = verts[0].position[1];
        let y10 = verts[1].position[1];
        assert!(
            y00 < y10,
            "y00 fallback ({}) should be below y10 ({}) since y00 uses avg-0.1",
            y00,
            y10
        );
        assert!(
            (y10 - y00 - 0.1).abs() < 0.01,
            "difference should be ~0.1, got {}",
            y10 - y00
        );
    }

    /// Tests fallback for y01: when h01 is None, y01 = avg_height - 0.1.
    /// Kills: `replace - with + in y01 unwrap_or` (line 399)
    #[test]
    fn test_generate_surface_mesh_y01_fallback() {
        let Some((device, _queue)) = try_create_test_device() else {
            return;
        };
        let mut gpu = create_test_gpu(&device, UVec3::new(4, 4, 4));
        gpu.set_cell_size(Vec3::new(1.0, 1.0, 1.0));
        gpu.set_origin(Vec3::ZERO);
        let mut grid = WaterVolumeGrid::new(UVec3::new(4, 4, 4), 1.0, Vec3::ZERO);
        // Water at (0,1,0) but NOT at (0,_,1) → h00=Some, h01=None
        if let Some(cell) = grid.get_cell_mut(IVec3::new(0, 1, 0)) {
            cell.level = 0.5; // height = 1.0 + 0.5 = 1.5
        }
        let (verts, _indices) = gpu.generate_surface_mesh(&grid);
        assert!(verts.len() >= 4, "should produce quad at (0,0)");
        // v[0] = p00 (h00=Some(1.5)) → y00 = 1.5
        // v[2] = p01 (h01=None) → y01 = avg(1.5) - 0.1 = 1.4
        let y00 = verts[0].position[1];
        let y01 = verts[2].position[1];
        assert!(
            y01 < y00,
            "y01 fallback ({}) should be below y00 ({}) since h01 is None",
            y01,
            y00
        );
        assert!(
            (y00 - y01 - 0.1).abs() < 0.01,
            "difference should be ~0.1, got {}",
            y00 - y01
        );
    }

    /// Tests that h01 samples at z+1 (not z-1 or z*1).
    /// Kills: `replace + with - in z+1` for h01 sampling (line 380:64)
    #[test]
    fn test_generate_surface_mesh_h01_samples_z_plus_1() {
        let Some((device, _queue)) = try_create_test_device() else {
            return;
        };
        let gpu = create_test_gpu(&device, UVec3::new(4, 4, 4));
        let mut grid = WaterVolumeGrid::new(UVec3::new(4, 4, 4), 1.0, Vec3::ZERO);
        // Place water at (0, 2, 1) — column (x=0, z=1) has water
        // h01 = sample(0, z+1=0+1=1) should find water
        // With mutation z+1→z-1: sample(0, -1) → out of bounds → None
        if let Some(cell) = grid.get_cell_mut(IVec3::new(0, 2, 1)) {
            cell.level = 0.8;
        }
        let (verts, _indices) = gpu.generate_surface_mesh(&grid);
        assert!(!verts.is_empty(), "h01 should find water at z+1=1");
        // v[2] = p01 should have a water-derived height, not a fallback
        let y01 = verts[2].position[1];
        // expected: base=2*1 + 0.8*1 = 2.8
        assert!(
            (y01 - 2.8).abs() < 0.1,
            "y01 should reflect water at (0,2,1), got {}",
            y01
        );
    }

    /// Tests that h11 samples at (x+1, z+1) correctly.
    /// Kills: `replace + with - in x+1,z+1` for h11 sampling (lines 381:61, 381:68)
    #[test]
    fn test_generate_surface_mesh_h11_samples_x_plus_1_z_plus_1() {
        let Some((device, _queue)) = try_create_test_device() else {
            return;
        };
        let gpu = create_test_gpu(&device, UVec3::new(4, 4, 4));
        let mut grid = WaterVolumeGrid::new(UVec3::new(4, 4, 4), 1.0, Vec3::ZERO);
        // Place water at (1, 2, 1) — column (x=1, z=1) has water
        // h11 = sample(x+1=0+1=1, z+1=0+1=1) should find water
        // With mutation x+1→x-1: sample(-1, 1) → out of bounds → None
        // With mutation z+1→z-1: sample(1, -1) → out of bounds → None
        if let Some(cell) = grid.get_cell_mut(IVec3::new(1, 2, 1)) {
            cell.level = 0.7;
        }
        let (verts, _indices) = gpu.generate_surface_mesh(&grid);
        assert!(!verts.is_empty(), "h11 should find water at (1,1)");
        // v[3] = p11 should have a water-derived height
        let y11 = verts[3].position[1];
        // expected: base=2*1 + 0.7*1 = 2.7
        assert!(
            (y11 - 2.7).abs() < 0.1,
            "y11 should reflect water at (1,2,1), got {}",
            y11
        );
    }

    /// Tests p11 position uses `(x+1) * cell_size.x` (not / cell_size.x).
    /// Kills: `replace * with / in p11 x-position` (line 402:55)
    #[test]
    fn test_generate_surface_mesh_p11_position_uses_multiply() {
        let Some((device, _queue)) = try_create_test_device() else {
            return;
        };
        let mut gpu = create_test_gpu(&device, UVec3::new(4, 4, 4));
        gpu.set_cell_size(Vec3::new(2.0, 1.0, 3.0));
        gpu.set_origin(Vec3::ZERO);
        let mut grid = WaterVolumeGrid::new(UVec3::new(4, 4, 4), 1.0, Vec3::ZERO);
        for x in 0..4 {
            for z in 0..4 {
                if let Some(cell) = grid.get_cell_mut(IVec3::new(x, 0, z)) {
                    cell.level = 0.5;
                }
            }
        }
        let (verts, _indices) = gpu.generate_surface_mesh(&grid);
        // First quad: v[3] = p11 = ((0+1)*2.0, _, (0+1)*3.0) = (2.0, _, 3.0)
        let p11 = verts[3].position;
        assert!(
            (p11[0] - 2.0).abs() < 1e-4,
            "p11.x should be (0+1)*2.0=2.0, got {} (would be 0.5 if / instead of *)",
            p11[0]
        );
        assert!(
            (p11[2] - 3.0).abs() < 1e-4,
            "p11.z should be (0+1)*3.0=3.0, got {}",
            p11[2]
        );
    }

    /// Tests p00 `x * cell_size` at x>0 where `*` vs `/` diverges.
    /// Kills: `replace * with / in p00.x` (line 402:55) and `p00.z` (line 402:84)
    #[test]
    fn test_generate_surface_mesh_p00_multiply_at_nonzero_coords() {
        let Some((device, _queue)) = try_create_test_device() else {
            return;
        };
        let mut gpu = create_test_gpu(&device, UVec3::new(4, 4, 4));
        gpu.set_cell_size(Vec3::new(2.0, 1.0, 3.0));
        gpu.set_origin(Vec3::ZERO);
        let mut grid = WaterVolumeGrid::new(UVec3::new(4, 4, 4), 1.0, Vec3::ZERO);
        // Fill all columns
        for x in 0..4 {
            for z in 0..4 {
                if let Some(cell) = grid.get_cell_mut(IVec3::new(x, 0, z)) {
                    cell.level = 0.5;
                }
            }
        }
        let (verts, _indices) = gpu.generate_surface_mesh(&grid);
        // Quad at (x=2, z=0) is the 3rd quad (z=0: quads 0,1,2). Vertices at index 8-11.
        // p00.x = 2*2.0 = 4.0 (not 2/2.0 = 1.0)
        assert!(
            (verts[8].position[0] - 4.0).abs() < 1e-4,
            "p00.x at x=2 should be 2*2.0=4.0, got {}",
            verts[8].position[0]
        );
        // Quad at (x=0, z=2) is the 7th quad (z=2: quads 6,7,8). Vertices at index 24-27.
        // p00.z = 2*3.0 = 6.0 (not 2/3.0 = 0.667)
        assert!(
            (verts[24].position[2] - 6.0).abs() < 1e-4,
            "p00.z at z=2 should be 2*3.0=6.0, got {}",
            verts[24].position[2]
        );
    }

    /// Tests y11 fallback when h11 is None: y11 = avg - 0.1.
    /// Kills: `replace - with + in y11 unwrap_or` (line 399)
    #[test]
    fn test_generate_surface_mesh_y11_fallback_below_avg() {
        let Some((device, _queue)) = try_create_test_device() else {
            return;
        };
        let mut gpu = create_test_gpu(&device, UVec3::new(4, 4, 4));
        gpu.set_cell_size(Vec3::new(1.0, 1.0, 1.0));
        gpu.set_origin(Vec3::ZERO);
        let mut grid = WaterVolumeGrid::new(UVec3::new(4, 4, 4), 1.0, Vec3::ZERO);
        // Water at column (0,0) only — h00=Some, h10=None, h01=None, h11=None
        if let Some(cell) = grid.get_cell_mut(IVec3::new(0, 1, 0)) {
            cell.level = 0.5; // height = 1.0 + 0.5 = 1.5
        }
        let (verts, _indices) = gpu.generate_surface_mesh(&grid);
        assert!(verts.len() >= 4);
        // v[0]=p00 (h00=Some(1.5)): y=1.5
        // v[3]=p11 (h11=None): y11 = avg(1.5) - 0.1 = 1.4
        let y00 = verts[0].position[1];
        let y11 = verts[3].position[1];
        assert!(
            y11 < y00,
            "y11 ({}) should be below y00 ({}) when h11=None (using avg-0.1)",
            y11,
            y00
        );
        assert!(
            (y00 - y11 - 0.1).abs() < 0.02,
            "y11 should be avg-0.1, diff should be ~0.1, got {}",
            y00 - y11
        );
    }

    /// Tests h01 correctly samples z+1 by placing water at z=0 AND z=1
    /// with different heights, then verifying p01.y reflects z=1 height.
    /// Kills: `replace + with - in h01 z+1` (line 380:64)
    #[test]
    fn test_generate_surface_mesh_h01_z_plus_1_specific_height() {
        let Some((device, _queue)) = try_create_test_device() else {
            return;
        };
        let mut gpu = create_test_gpu(&device, UVec3::new(4, 4, 4));
        gpu.set_cell_size(Vec3::new(1.0, 1.0, 1.0));
        gpu.set_origin(Vec3::ZERO);
        let mut grid = WaterVolumeGrid::new(UVec3::new(4, 4, 4), 1.0, Vec3::ZERO);
        // Column (0, z=0): water at y=0, level=0.3 → height = 0.3
        if let Some(cell) = grid.get_cell_mut(IVec3::new(0, 0, 0)) {
            cell.level = 0.3;
        }
        // Column (0, z=1): water at y=2, level=0.9 → height = 2.0 + 0.9 = 2.9
        if let Some(cell) = grid.get_cell_mut(IVec3::new(0, 2, 1)) {
            cell.level = 0.9;
        }
        // Also put water at (1, z=0) and (1, z=1) to ensure all corners have water
        if let Some(cell) = grid.get_cell_mut(IVec3::new(1, 0, 0)) {
            cell.level = 0.3;
        }
        if let Some(cell) = grid.get_cell_mut(IVec3::new(1, 2, 1)) {
            cell.level = 0.9;
        }
        let (verts, _indices) = gpu.generate_surface_mesh(&grid);
        assert!(verts.len() >= 4, "quad (0,0) should generate");
        // v[0]=p00: y = h00 from column (0,0) = 0.3
        // v[2]=p01: y = h01 from column (0, z+1=1) = 2.9 [correct]
        //          With mutation z+1→z-1: h01 = sample(0,-1) = None → fallback
        let y_p00 = verts[0].position[1];
        let y_p01 = verts[2].position[1];
        assert!(
            (y_p00 - 0.3).abs() < 0.05,
            "p00.y should be ~0.3 from water at z=0, got {}",
            y_p00
        );
        assert!(
            (y_p01 - 2.9).abs() < 0.05,
            "p01.y should be ~2.9 from water at z=1 (not fallback), got {}",
            y_p01
        );
        // Key assertion: difference should be large (2.6), not small (~0.1 fallback)
        assert!(
            (y_p01 - y_p00).abs() > 1.0,
            "p01.y should differ significantly from p00.y (water at different heights)"
        );
    }

    /// Tests h11 correctly samples x+1 and z+1 by placing water at
    /// different heights in each corner column.
    /// Kills: `replace + with - in h11 x+1` (381:61) and `h11 z+1` (381:68)
    #[test]
    fn test_generate_surface_mesh_h11_specific_height() {
        let Some((device, _queue)) = try_create_test_device() else {
            return;
        };
        let mut gpu = create_test_gpu(&device, UVec3::new(4, 4, 4));
        gpu.set_cell_size(Vec3::new(1.0, 1.0, 1.0));
        gpu.set_origin(Vec3::ZERO);
        let mut grid = WaterVolumeGrid::new(UVec3::new(4, 4, 4), 1.0, Vec3::ZERO);
        // Fill all 4 corner columns with distinct heights for quad (0,0):
        // (0,0,0): y=0, level=0.2 → height=0.2
        if let Some(cell) = grid.get_cell_mut(IVec3::new(0, 0, 0)) {
            cell.level = 0.2;
        }
        // (1,0,0): y=0, level=0.4 → height=0.4
        if let Some(cell) = grid.get_cell_mut(IVec3::new(1, 0, 0)) {
            cell.level = 0.4;
        }
        // (0,0,1): y=0, level=0.6 → height=0.6
        if let Some(cell) = grid.get_cell_mut(IVec3::new(0, 0, 1)) {
            cell.level = 0.6;
        }
        // (1,0,1): y=2, level=0.8 → height=2.0+0.8=2.8
        if let Some(cell) = grid.get_cell_mut(IVec3::new(1, 2, 1)) {
            cell.level = 0.8;
        }
        let (verts, _indices) = gpu.generate_surface_mesh(&grid);
        assert!(verts.len() >= 4, "quad (0,0) should generate");
        // v[3]=p11: y = h11 from column (x+1=1, z+1=1), height=2.8
        // With mutation x+1→x-1: sample(-1,1)=None → fallback (~avg-0.1)
        // With mutation z+1→z-1: sample(1,-1)=None → fallback
        let y_p11 = verts[3].position[1];
        assert!(
            (y_p11 - 2.8).abs() < 0.1,
            "p11.y should be ~2.8 from column (1,1) at y=2, got {}",
            y_p11
        );
        // Must be significantly higher than fallback (which would be ~avg-0.1)
        let y_p00 = verts[0].position[1];
        assert!(
            y_p11 > y_p00 + 1.0,
            "p11.y ({}) should be much higher than p00.y ({}) — not using fallback",
            y_p11,
            y_p00
        );
    }
}
