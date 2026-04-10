//! Bindless Material System
//!
//! GPU-driven material pipeline using `TEXTURE_BINDING_ARRAY` to bind an array
//! of individual `texture_2d` resources.  Each material stores indices into that
//! array plus PBR factors in a single storage buffer.
//!
//! This enables rendering all meshes sharing a pipeline with a single draw call,
//! as the shader dynamically fetches the correct material and textures.

use bytemuck::{Pod, Zeroable};
use std::num::NonZeroU32;
use wgpu;

// ─── Flag Constants (mirror WGSL) ───

/// Material has an albedo texture.
pub const MAT_FLAG_HAS_ALBEDO: u32 = 1;
/// Material has a normal map.
pub const MAT_FLAG_HAS_NORMAL: u32 = 2;
/// Material has an ORM (Occlusion/Roughness/Metallic) packed texture.
pub const MAT_FLAG_HAS_ORM: u32 = 4;
/// Material has an emissive texture.
pub const MAT_FLAG_HAS_EMISSIVE: u32 = 8;

// ─── GPU Material Entry ───

/// Per-material GPU data — matches the WGSL `MaterialEntry` struct.
///
/// 64 bytes, stored in a single storage buffer. The `*_index` fields are
/// indices into the `binding_array<texture_2d<f32>>` declared in the shader.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct GpuMaterialEntry {
    pub albedo_index: u32,
    pub normal_index: u32,
    pub orm_index: u32,
    pub emissive_index: u32,
    pub base_color: [f32; 4],
    pub metallic_factor: f32,
    pub roughness_factor: f32,
    pub occlusion_factor: f32,
    pub emissive_strength: f32,
    pub uv_scale: [f32; 2],
    pub flags: u32,
    pub _padding: u32,
}

impl Default for GpuMaterialEntry {
    fn default() -> Self {
        Self {
            albedo_index: 0,
            normal_index: 0,
            orm_index: 0,
            emissive_index: 0,
            base_color: [1.0, 1.0, 1.0, 1.0],
            metallic_factor: 0.0,
            roughness_factor: 0.5,
            occlusion_factor: 1.0,
            emissive_strength: 0.0,
            uv_scale: [1.0, 1.0],
            flags: 0,
            _padding: 0,
        }
    }
}

impl GpuMaterialEntry {
    /// Create a simple PBR material with no textures.
    pub fn solid(base_color: [f32; 4], metallic: f32, roughness: f32) -> Self {
        Self {
            base_color,
            metallic_factor: metallic,
            roughness_factor: roughness,
            ..Default::default()
        }
    }

    /// Create a textured PBR material.
    pub fn textured(albedo_index: u32, normal_index: u32, orm_index: u32) -> Self {
        Self {
            albedo_index,
            normal_index,
            orm_index,
            flags: MAT_FLAG_HAS_ALBEDO | MAT_FLAG_HAS_NORMAL | MAT_FLAG_HAS_ORM,
            ..Default::default()
        }
    }
}

// ─── Configuration ───

/// Configuration for the bindless material system.
#[derive(Debug, Clone)]
pub struct BindlessMaterialConfig {
    /// Maximum number of textures in the binding array.
    pub max_textures: u32,
    /// Maximum number of materials in the SSBO.
    pub max_materials: u32,
}

impl Default for BindlessMaterialConfig {
    fn default() -> Self {
        Self {
            max_textures: 256,
            max_materials: 512,
        }
    }
}

impl BindlessMaterialConfig {
    /// Validate the configuration.
    pub fn validate(&self) -> bool {
        self.max_textures > 0 && self.max_materials > 0
    }
}

// ─── Feature Detection ───

/// Check whether the adapter supports bindless textures.
pub fn supports_bindless(adapter: &wgpu::Adapter) -> bool {
    let features = adapter.features();
    features.contains(wgpu::Features::TEXTURE_BINDING_ARRAY)
        && features
            .contains(wgpu::Features::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING)
}

/// Returns the wgpu features required by the bindless material system.
pub fn required_features() -> wgpu::Features {
    wgpu::Features::TEXTURE_BINDING_ARRAY
        | wgpu::Features::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING
}

/// Returns the minimum limits required for a given texture count.
pub fn required_limits(max_textures: u32) -> wgpu::Limits {
    wgpu::Limits {
        max_binding_array_elements_per_shader_stage: max_textures,
        ..Default::default()
    }
}

// ─── Bindless Material System ───

/// GPU-driven material system with bindless texture arrays.
///
/// All materials live in a single storage buffer. Textures are bound as
/// a `binding_array<texture_2d<f32>>` indexed by material entries.
pub struct BindlessMaterialSystem {
    texture_views: Vec<wgpu::TextureView>,
    materials: Vec<GpuMaterialEntry>,
    material_buffer: wgpu::Buffer,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: Option<wgpu::BindGroup>,
    sampler: wgpu::Sampler,
    config: BindlessMaterialConfig,
    dirty: bool,
    /// 1×1 white fallback texture (used when texture_views is empty or for placeholder slots).
    _fallback_texture: wgpu::Texture,
    fallback_view: wgpu::TextureView,
}

impl BindlessMaterialSystem {
    /// Create a new bindless material system.
    ///
    /// The device **must** have been created with [`required_features()`] enabled.
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, config: BindlessMaterialConfig) -> Self {
        let max_textures = config.max_textures.max(1);
        let max_materials = config.max_materials.max(1);

        // Fallback 1×1 white texture
        let fallback_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("bindless_fallback_texture"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &fallback_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &[255u8, 255, 255, 255],
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4),
                rows_per_image: None,
            },
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );
        let fallback_view = fallback_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bindless_material_bgl"),
            entries: &[
                // binding 0: texture array
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: NonZeroU32::new(max_textures),
                },
                // binding 1: sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                // binding 2: material SSBO
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        // Material SSBO
        let material_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bindless_material_ssbo"),
            size: (max_materials as u64) * (std::mem::size_of::<GpuMaterialEntry>() as u64),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("bindless_material_sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            ..Default::default()
        });

        Self {
            texture_views: Vec::new(),
            materials: Vec::new(),
            material_buffer,
            bind_group_layout,
            bind_group: None,
            sampler,
            config: BindlessMaterialConfig {
                max_textures,
                max_materials,
            },
            dirty: true,
            _fallback_texture: fallback_texture,
            fallback_view,
        }
    }

    /// Add a texture view to the array. Returns its index.
    ///
    /// Returns `None` if the array is full.
    pub fn add_texture(&mut self, view: wgpu::TextureView) -> Option<u32> {
        if self.texture_views.len() >= self.config.max_textures as usize {
            return None;
        }
        let idx = self.texture_views.len() as u32;
        self.texture_views.push(view);
        self.dirty = true;
        Some(idx)
    }

    /// Add a material. Returns its index.
    ///
    /// Returns `None` if the material SSBO is full.
    pub fn add_material(&mut self, entry: GpuMaterialEntry) -> Option<u32> {
        if self.materials.len() >= self.config.max_materials as usize {
            return None;
        }
        let idx = self.materials.len() as u32;
        self.materials.push(entry);
        self.dirty = true;
        Some(idx)
    }

    /// Number of textures currently registered.
    pub fn texture_count(&self) -> u32 {
        self.texture_views.len() as u32
    }

    /// Number of materials currently registered.
    pub fn material_count(&self) -> u32 {
        self.materials.len() as u32
    }

    /// Maximum texture slots.
    pub fn max_textures(&self) -> u32 {
        self.config.max_textures
    }

    /// Maximum material slots.
    pub fn max_materials(&self) -> u32 {
        self.config.max_materials
    }

    /// The bind group layout for this system.
    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }

    /// Rebuild the bind group and upload material data if dirty.
    pub fn update(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        if !self.dirty {
            return;
        }
        self.dirty = false;

        // Upload material data
        if !self.materials.is_empty() {
            queue.write_buffer(
                &self.material_buffer,
                0,
                bytemuck::cast_slice(&self.materials),
            );
        }

        // Build texture view references — fill unused slots with fallback
        let max = self.config.max_textures as usize;
        let mut views: Vec<&wgpu::TextureView> = Vec::with_capacity(max);
        for v in &self.texture_views {
            views.push(v);
        }
        // Pad remaining slots with fallback
        for _ in self.texture_views.len()..max {
            views.push(&self.fallback_view);
        }

        self.bind_group = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bindless_material_bg"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureViewArray(&views),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.material_buffer.as_entire_binding(),
                },
            ],
        }));
    }

    /// Get the current bind group (after calling `update()`).
    pub fn bind_group(&self) -> Option<&wgpu::BindGroup> {
        self.bind_group.as_ref()
    }
}

// ─── Tests ───

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gpu_material_entry_size() {
        assert_eq!(
            std::mem::size_of::<GpuMaterialEntry>(),
            64,
            "GpuMaterialEntry must be 64 bytes"
        );
    }

    #[test]
    fn gpu_material_entry_default() {
        let mat = GpuMaterialEntry::default();
        assert_eq!(mat.base_color, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(mat.metallic_factor, 0.0);
        assert_eq!(mat.roughness_factor, 0.5);
        assert_eq!(mat.flags, 0);
    }

    #[test]
    fn gpu_material_entry_solid() {
        let mat = GpuMaterialEntry::solid([1.0, 0.0, 0.0, 1.0], 1.0, 0.2);
        assert_eq!(mat.base_color, [1.0, 0.0, 0.0, 1.0]);
        assert_eq!(mat.metallic_factor, 1.0);
        assert_eq!(mat.roughness_factor, 0.2);
        assert_eq!(mat.flags, 0);
    }

    #[test]
    fn gpu_material_entry_textured() {
        let mat = GpuMaterialEntry::textured(0, 1, 2);
        assert_eq!(mat.albedo_index, 0);
        assert_eq!(mat.normal_index, 1);
        assert_eq!(mat.orm_index, 2);
        assert_eq!(
            mat.flags,
            MAT_FLAG_HAS_ALBEDO | MAT_FLAG_HAS_NORMAL | MAT_FLAG_HAS_ORM
        );
    }

    #[test]
    fn config_default_valid() {
        let cfg = BindlessMaterialConfig::default();
        assert!(cfg.validate());
        assert_eq!(cfg.max_textures, 256);
        assert_eq!(cfg.max_materials, 512);
    }

    #[test]
    fn config_validation_rejects_zero() {
        let cfg = BindlessMaterialConfig {
            max_textures: 0,
            max_materials: 512,
        };
        assert!(!cfg.validate());
    }

    #[test]
    fn required_features_includes_binding_array() {
        let f = required_features();
        assert!(f.contains(wgpu::Features::TEXTURE_BINDING_ARRAY));
        assert!(f.contains(
            wgpu::Features::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING
        ));
    }

    #[test]
    fn flag_constants_are_powers_of_two() {
        assert_eq!(MAT_FLAG_HAS_ALBEDO.count_ones(), 1);
        assert_eq!(MAT_FLAG_HAS_NORMAL.count_ones(), 1);
        assert_eq!(MAT_FLAG_HAS_ORM.count_ones(), 1);
        assert_eq!(MAT_FLAG_HAS_EMISSIVE.count_ones(), 1);
        // All different bits
        assert_eq!(
            MAT_FLAG_HAS_ALBEDO | MAT_FLAG_HAS_NORMAL | MAT_FLAG_HAS_ORM | MAT_FLAG_HAS_EMISSIVE,
            0b1111
        );
    }

    #[test]
    fn parse_bindless_material_wgsl() {
        let src = include_str!("../shaders/bindless_material.wgsl");
        let result = naga::front::wgsl::parse_str(src);
        assert!(result.is_ok(), "WGSL parse failed: {result:?}");
    }

    // GPU tests require TEXTURE_BINDING_ARRAY which may not be available in CI.
    // The system construction test uses a real adapter to check feature support.
    #[test]
    fn bindless_system_creation() {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = match pollster::block_on(
            instance.request_adapter(&wgpu::RequestAdapterOptions::default()),
        ) {
            Ok(a) => a,
            Err(_) => return,
        };

        if !supports_bindless(&adapter) {
            return;
        }

        let (device, queue) =
            match pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                label: Some("bindless_test_device"),
                required_features: required_features(),
                required_limits: required_limits(16),
                ..Default::default()
            })) {
                Ok(dq) => dq,
                Err(_) => return,
            };

        let config = BindlessMaterialConfig {
            max_textures: 16,
            max_materials: 32,
        };
        let system = BindlessMaterialSystem::new(&device, &queue, config);
        assert_eq!(system.texture_count(), 0);
        assert_eq!(system.material_count(), 0);
        assert_eq!(system.max_textures(), 16);
        assert_eq!(system.max_materials(), 32);
    }

    #[test]
    fn bindless_add_material() {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = match pollster::block_on(
            instance.request_adapter(&wgpu::RequestAdapterOptions::default()),
        ) {
            Ok(a) => a,
            Err(_) => return,
        };

        if !supports_bindless(&adapter) {
            return;
        }

        let (device, queue) =
            match pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                label: Some("bindless_test_device"),
                required_features: required_features(),
                required_limits: required_limits(4),
                ..Default::default()
            })) {
                Ok(dq) => dq,
                Err(_) => return,
            };

        let config = BindlessMaterialConfig {
            max_textures: 4,
            max_materials: 2,
        };
        let mut system = BindlessMaterialSystem::new(&device, &queue, config);

        let idx0 = system.add_material(GpuMaterialEntry::solid([1.0, 0.0, 0.0, 1.0], 0.0, 0.5));
        assert_eq!(idx0, Some(0));
        let idx1 = system.add_material(GpuMaterialEntry::default());
        assert_eq!(idx1, Some(1));
        // Full
        let idx2 = system.add_material(GpuMaterialEntry::default());
        assert_eq!(idx2, None);
        assert_eq!(system.material_count(), 2);
    }

    #[test]
    fn bindless_update_creates_bind_group() {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = match pollster::block_on(
            instance.request_adapter(&wgpu::RequestAdapterOptions::default()),
        ) {
            Ok(a) => a,
            Err(_) => return,
        };

        if !supports_bindless(&adapter) {
            return;
        }

        let (device, queue) =
            match pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                label: Some("bindless_test_device"),
                required_features: required_features(),
                required_limits: required_limits(4),
                ..Default::default()
            })) {
                Ok(dq) => dq,
                Err(_) => return,
            };

        let config = BindlessMaterialConfig {
            max_textures: 4,
            max_materials: 4,
        };
        let mut system = BindlessMaterialSystem::new(&device, &queue, config);
        system.add_material(GpuMaterialEntry::default());
        assert!(system.bind_group().is_none());
        system.update(&device, &queue);
        assert!(system.bind_group().is_some());
    }
}
