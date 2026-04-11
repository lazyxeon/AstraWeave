//! Vegetation interaction stamp system.
//!
//! Generates a persistent R8 interaction texture that encodes entity proximity,
//! consumed by the grass blade shader to bend blades away from characters.
//!
//! Each frame:
//! 1. **Decay pass**: fades previous stamps toward zero (trail dissipation).
//! 2. **Stamp pass**: stamps circular footprints at entity world positions.
//!
//! The texture covers a square world region centered on the camera, with
//! extent = 2 × `interaction_radius` in each axis.

use bytemuck::{Pod, Zeroable};

// ── Constants ───────────────────────────────────────────────────────────────

/// WGSL shader source.
const INTERACTION_SHADER: &str = include_str!("../shaders/vegetation_interaction.wgsl");

/// Default interaction texture resolution (128×128 is sufficient for 16m radius).
const DEFAULT_TEX_SIZE: u32 = 128;

/// Maximum entity stamps per frame.
const MAX_ENTITIES: usize = 64;

// ── GPU structs (match WGSL layout) ─────────────────────────────────────────

/// Stamp parameters uniform (32 bytes, matches WGSL `StampParams`).
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct StampParams {
    pub camera_x: f32,
    pub camera_z: f32,
    pub interaction_radius: f32,
    pub tex_size: f32,
    pub decay_rate: f32,
    pub entity_count: u32,
    pub stamp_radius: f32,
    pub stamp_intensity: f32,
}

impl Default for StampParams {
    fn default() -> Self {
        Self {
            camera_x: 0.0,
            camera_z: 0.0,
            interaction_radius: 8.0,
            tex_size: DEFAULT_TEX_SIZE as f32,
            decay_rate: 0.92,
            entity_count: 0,
            stamp_radius: 0.5,
            stamp_intensity: 0.8,
        }
    }
}

/// Entity position for stamping (8 bytes, matches WGSL `EntityPos`).
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct EntityPos {
    pub x: f32,
    pub z: f32,
}

// ── Pipeline ────────────────────────────────────────────────────────────────

/// Manages the vegetation interaction stamp compute pipeline and texture.
pub struct VegetationInteraction {
    decay_pipeline: wgpu::ComputePipeline,
    stamp_pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,

    // R8 interaction texture (persistent across frames).
    interaction_texture: wgpu::Texture,
    interaction_view: wgpu::TextureView,
    tex_size: u32,

    // Uniform + entity storage buffers.
    params_buffer: wgpu::Buffer,
    entity_buffer: wgpu::Buffer,
}

impl VegetationInteraction {
    /// Create the interaction stamp system.
    ///
    /// `tex_size`: interaction texture resolution (default 128).
    pub fn new(device: &wgpu::Device, tex_size: Option<u32>) -> Self {
        let tex_size = tex_size.unwrap_or(DEFAULT_TEX_SIZE);

        // ── Shader ──────────────────────────────────────────────────────
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("vegetation_interaction.wgsl"),
            source: wgpu::ShaderSource::Wgsl(INTERACTION_SHADER.into()),
        });

        // ── Bind group layout ───────────────────────────────────────────
        let bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("VegInteraction BGL"),
                entries: &[
                    // 0: StampParams uniform
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // 1: EntityPos storage (read)
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // 2: Interaction texture (storage, read_write)
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::StorageTexture {
                            access: wgpu::StorageTextureAccess::ReadWrite,
                            format: wgpu::TextureFormat::R8Unorm,
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                ],
            });

        // ── Pipeline layout ─────────────────────────────────────────────
        let pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("VegInteraction PipelineLayout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

        // ── Compute pipelines ───────────────────────────────────────────
        let decay_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("VegInteraction Decay"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: Some("decay"),
                compilation_options: Default::default(),
                cache: None,
            });

        let stamp_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("VegInteraction Stamp"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: Some("stamp"),
                compilation_options: Default::default(),
                cache: None,
            });

        // ── Interaction texture ─────────────────────────────────────────
        let interaction_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("VegInteraction Texture"),
            size: wgpu::Extent3d {
                width: tex_size,
                height: tex_size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let interaction_view =
            interaction_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // ── Buffers ─────────────────────────────────────────────────────
        let params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("VegInteraction Params"),
            size: std::mem::size_of::<StampParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let entity_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("VegInteraction Entities"),
            size: (MAX_ENTITIES * std::mem::size_of::<EntityPos>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            decay_pipeline,
            stamp_pipeline,
            bind_group_layout,
            interaction_texture,
            interaction_view,
            tex_size,
            params_buffer,
            entity_buffer,
        }
    }

    /// Dispatch decay + stamp passes for this frame.
    ///
    /// `camera_xz`: camera world position (x, z).
    /// `entity_positions`: world XZ positions of entities that bend grass.
    /// `interaction_radius`: world-space radius covered by the texture.
    /// `decay_rate`: per-frame decay multiplier (0.9–0.98 typical).
    #[allow(clippy::too_many_arguments)]
    pub fn dispatch(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        camera_xz: [f32; 2],
        entity_positions: &[EntityPos],
        interaction_radius: f32,
        decay_rate: f32,
    ) {
        let entity_count = entity_positions.len().min(MAX_ENTITIES);

        let params = StampParams {
            camera_x: camera_xz[0],
            camera_z: camera_xz[1],
            interaction_radius,
            tex_size: self.tex_size as f32,
            decay_rate,
            entity_count: entity_count as u32,
            stamp_radius: 0.5,
            stamp_intensity: 0.8,
        };

        queue.write_buffer(&self.params_buffer, 0, bytemuck::bytes_of(&params));

        if entity_count > 0 {
            queue.write_buffer(
                &self.entity_buffer,
                0,
                bytemuck::cast_slice(&entity_positions[..entity_count]),
            );
        }

        // Build bind group (shared by both passes).
        let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("VegInteraction BG"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.entity_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&self.interaction_view),
                },
            ],
        });

        let wg_x = self.tex_size.div_ceil(8);
        let wg_y = self.tex_size.div_ceil(8);

        // Pass 1: Decay
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("VegInteraction Decay"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.decay_pipeline);
            pass.set_bind_group(0, &bg, &[]);
            pass.dispatch_workgroups(wg_x, wg_y, 1);
        }

        // Pass 2: Stamp entities
        if entity_count > 0 {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("VegInteraction Stamp"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.stamp_pipeline);
            pass.set_bind_group(0, &bg, &[]);
            pass.dispatch_workgroups(wg_x, wg_y, 1);
        }
    }

    /// Get the interaction texture view for binding to the grass blade shader.
    pub fn interaction_view(&self) -> &wgpu::TextureView {
        &self.interaction_view
    }

    /// Get the raw texture (for external sampling or debugging).
    pub fn interaction_texture(&self) -> &wgpu::Texture {
        &self.interaction_texture
    }

    /// Texture resolution.
    pub fn tex_size(&self) -> u32 {
        self.tex_size
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stamp_params_size() {
        assert_eq!(std::mem::size_of::<StampParams>(), 32);
    }

    #[test]
    fn test_entity_pos_size() {
        assert_eq!(std::mem::size_of::<EntityPos>(), 8);
    }

    #[test]
    fn test_stamp_params_default() {
        let p = StampParams::default();
        assert!(p.decay_rate > 0.0 && p.decay_rate < 1.0);
        assert!(p.interaction_radius > 0.0);
        assert!(p.stamp_radius > 0.0);
        assert!(p.stamp_intensity > 0.0 && p.stamp_intensity <= 1.0);
        assert_eq!(p.entity_count, 0);
    }

    #[test]
    fn test_interaction_shader_present() {
        assert!(
            INTERACTION_SHADER.contains("decay"),
            "shader must contain decay entry point"
        );
        assert!(
            INTERACTION_SHADER.contains("stamp"),
            "shader must contain stamp entry point"
        );
        assert!(
            INTERACTION_SHADER.contains("StampParams"),
            "shader must contain StampParams struct"
        );
        assert!(
            INTERACTION_SHADER.contains("interaction_tex"),
            "shader must reference interaction texture"
        );
    }

    #[test]
    fn test_max_entities_reasonable() {
        // 64 entities × 8 bytes = 512 bytes storage buffer
        assert!(MAX_ENTITIES >= 16 && MAX_ENTITIES <= 256);
    }
}
