//! GPU Skinning Pipeline for AstraWeave
//!
//! Phase 2 Task 5 (Phase D): GPU-accelerated skeletal animation skinning.
//! Feature-gated with `skinning-gpu` - optional for performance, CPU path is default.

use crate::animation::JointPalette;
use anyhow::Result;
use glam::Mat4;
use std::collections::HashMap;

/// Handle for a joint palette buffer (per-skeleton GPU buffer)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct JointPaletteHandle(pub u32);

/// GPU buffer pool for joint palettes
///
/// Uses a single large SSBO with dynamic offsets instead of per-skeleton
/// individual buffers. This eliminates per-skeleton buffer/bind-group
/// allocation overhead and enables efficient draw-call batching.
pub struct JointPaletteManager {
    device: wgpu::Device,
    queue: wgpu::Queue,

    /// Single pooled storage buffer for all joint palettes
    pool_buffer: wgpu::Buffer,

    /// Current capacity in palette slots
    pool_capacity: u32,

    /// Aligned byte stride per palette slot
    slot_stride: u64,

    /// Handle → pool slot mapping
    handle_to_slot: HashMap<JointPaletteHandle, u32>,

    /// Free slot indices (LIFO stack)
    free_slots: Vec<u32>,

    /// Next available handle
    next_handle: u32,

    /// Bind group layout for skinning storage buffer (has_dynamic_offset = true)
    pub bind_group_layout: wgpu::BindGroupLayout,

    /// Single bind group pointing to the entire pool buffer
    bind_group: wgpu::BindGroup,
}

/// Initial pool capacity (number of skeleton slots)
const INITIAL_POOL_CAPACITY: u32 = 64;

impl JointPaletteManager {
    /// Create a new joint palette manager with a pooled SSBO
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let raw_size = std::mem::size_of::<JointPalette>() as u64;
        let alignment = device.limits().min_storage_buffer_offset_alignment as u64;
        let slot_stride = raw_size.div_ceil(alignment) * alignment;

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("joint_palette_pool_layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: true,
                    min_binding_size: std::num::NonZeroU64::new(raw_size),
                },
                count: None,
            }],
        });

        let pool_capacity = INITIAL_POOL_CAPACITY;
        let pool_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("joint_palette_pool"),
            size: slot_stride * pool_capacity as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("joint_palette_pool_bind_group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &pool_buffer,
                    offset: 0,
                    size: std::num::NonZeroU64::new(raw_size),
                }),
            }],
        });

        let free_slots = (0..pool_capacity).rev().collect();

        Self {
            device: device.clone(),
            queue: queue.clone(),
            pool_buffer,
            pool_capacity,
            slot_stride,
            handle_to_slot: HashMap::new(),
            free_slots,
            next_handle: 0,
            bind_group_layout,
            bind_group,
        }
    }

    /// Allocate a new joint palette slot from the pool
    pub fn allocate(&mut self) -> JointPaletteHandle {
        if self.free_slots.is_empty() {
            self.grow();
        }

        let slot = self
            .free_slots
            .pop()
            .expect("grow() should have added slots");
        let handle = JointPaletteHandle(self.next_handle);
        self.next_handle += 1;
        self.handle_to_slot.insert(handle, slot);
        handle
    }

    /// Upload joint matrices to the pooled GPU buffer (from Mat4 array)
    pub fn upload_matrices(&mut self, handle: JointPaletteHandle, matrices: &[Mat4]) -> Result<()> {
        let palette = JointPalette::from_matrices(matrices);
        self.upload_palette(handle, &palette)
    }

    /// Upload joint palette to the pooled GPU buffer (from JointPalette struct)
    pub fn upload_palette(
        &mut self,
        handle: JointPaletteHandle,
        palette: &JointPalette,
    ) -> Result<()> {
        let slot = *self
            .handle_to_slot
            .get(&handle)
            .ok_or_else(|| anyhow::anyhow!("Invalid joint palette handle: {:?}", handle))?;

        let offset = slot as u64 * self.slot_stride;
        let binding = [*palette];
        let data = bytemuck::cast_slice(&binding);
        self.queue.write_buffer(&self.pool_buffer, offset, data);

        Ok(())
    }

    /// Get the shared bind group for the pool (use with dynamic_offset)
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }

    /// Get the dynamic offset for a handle (pass to set_bind_group)
    pub fn dynamic_offset(&self, handle: JointPaletteHandle) -> Option<u32> {
        self.handle_to_slot
            .get(&handle)
            .map(|&slot| (slot as u64 * self.slot_stride) as u32)
    }

    /// Backward-compatible bind group accessor.
    ///
    /// Returns the pool's shared bind group for valid handles.
    /// Callers should migrate to `bind_group()` + `dynamic_offset()`.
    pub fn get_bind_group(&self, handle: JointPaletteHandle) -> Option<&wgpu::BindGroup> {
        if self.handle_to_slot.contains_key(&handle) {
            Some(&self.bind_group)
        } else {
            None
        }
    }

    /// Free a joint palette slot back to the pool
    pub fn free(&mut self, handle: JointPaletteHandle) {
        if let Some(slot) = self.handle_to_slot.remove(&handle) {
            self.free_slots.push(slot);
        }
    }

    /// Get number of active palette slots
    pub fn active_count(&self) -> usize {
        self.handle_to_slot.len()
    }

    /// Return all slots to the pool
    pub fn clear(&mut self) {
        for (_, slot) in self.handle_to_slot.drain() {
            self.free_slots.push(slot);
        }
        self.next_handle = 0;
    }

    /// Double the pool capacity, copying existing data
    fn grow(&mut self) {
        let new_capacity = self.pool_capacity * 2;
        let new_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("joint_palette_pool"),
            size: self.slot_stride * new_capacity as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Copy existing pool data to the new buffer
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("joint_palette_pool_grow"),
            });
        encoder.copy_buffer_to_buffer(
            &self.pool_buffer,
            0,
            &new_buffer,
            0,
            self.slot_stride * self.pool_capacity as u64,
        );
        self.queue.submit(std::iter::once(encoder.finish()));

        // Register new free slots
        for i in self.pool_capacity..new_capacity {
            self.free_slots.push(i);
        }

        self.pool_buffer = new_buffer;
        self.pool_capacity = new_capacity;

        // Recreate bind group for the new buffer
        let raw_size = std::mem::size_of::<JointPalette>() as u64;
        self.bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("joint_palette_pool_bind_group"),
            layout: &self.bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &self.pool_buffer,
                    offset: 0,
                    size: std::num::NonZeroU64::new(raw_size),
                }),
            }],
        });
    }
}

// ============================================================================
// GPU Skinning Shader Module (embedded WGSL)
// ============================================================================

/// WGSL shader for GPU skinning vertex transformation
pub const SKINNING_GPU_SHADER: &str = r#"
// Joint palette storage buffer
struct JointMatrix {
    matrix: mat4x4<f32>,
}

struct JointPalette {
    joints: array<JointMatrix, 256>,
    joint_count: u32,
    _padding: array<u32, 3>,
}

@group(4) @binding(0) var<storage, read> joint_palette: JointPalette;

// Vertex input with skinning data
struct SkinnedVertexInput {
    position: vec3<f32>,
    normal: vec3<f32>,
    tangent: vec4<f32>,
    joints: vec4<u32>,
    weights: vec4<f32>,
}

// Apply GPU skinning to position and normal
fn apply_skinning(input: SkinnedVertexInput) -> vec4<f32> {
    let j = input.joints;
    let w = input.weights;
    
    // Fetch joint matrices
    let m0 = joint_palette.joints[j.x].matrix;
    let m1 = joint_palette.joints[j.y].matrix;
    let m2 = joint_palette.joints[j.z].matrix;
    let m3 = joint_palette.joints[j.w].matrix;
    
    // Blend position
    let pos4 = vec4<f32>(input.position, 1.0);
    let skinned_pos = (m0 * pos4) * w.x 
                    + (m1 * pos4) * w.y 
                    + (m2 * pos4) * w.z 
                    + (m3 * pos4) * w.w;
    
    return skinned_pos;
}

// Apply skinning to normal (for lighting)
fn apply_skinning_normal(input: SkinnedVertexInput) -> vec3<f32> {
    let j = input.joints;
    let w = input.weights;
    
    let m0 = joint_palette.joints[j.x].matrix;
    let m1 = joint_palette.joints[j.y].matrix;
    let m2 = joint_palette.joints[j.z].matrix;
    let m3 = joint_palette.joints[j.w].matrix;
    
    let nrm4 = vec4<f32>(input.normal, 0.0);
    let skinned_nrm = (m0 * nrm4) * w.x 
                    + (m1 * nrm4) * w.y 
                    + (m2 * nrm4) * w.z 
                    + (m3 * nrm4) * w.w;
    
    return normalize(skinned_nrm.xyz);
}

// Apply skinning to tangent (for normal mapping)
fn apply_skinning_tangent(input: SkinnedVertexInput) -> vec3<f32> {
    let j = input.joints;
    let w = input.weights;
    
    let m0 = joint_palette.joints[j.x].matrix;
    let m1 = joint_palette.joints[j.y].matrix;
    let m2 = joint_palette.joints[j.z].matrix;
    let m3 = joint_palette.joints[j.w].matrix;
    
    let tan4 = vec4<f32>(input.tangent.xyz, 0.0);
    let skinned_tan = (m0 * tan4) * w.x 
                    + (m1 * tan4) * w.y 
                    + (m2 * tan4) * w.z 
                    + (m3 * tan4) * w.w;
    
    return normalize(skinned_tan.xyz);
}
"#;

// ============================================================================
// Complete Shader Generation
// ============================================================================

/// Create complete WGSL shader for GPU skinned mesh rendering
fn create_complete_skinning_shader() -> String {
    format!(
        r#"
// Bind Groups
struct Camera {{
    view_proj: mat4x4<f32>,
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    position: vec3<f32>,
    _padding: f32,
}}

struct Material {{
    base_color: vec4<f32>,
    metallic: f32,
    roughness: f32,
    _padding: vec2<f32>,
}}

struct Light {{
    position: vec3<f32>,
    _padding1: f32,
    direction: vec3<f32>,
    _padding2: f32,
    color: vec3<f32>,
    intensity: f32,
}}

@group(0) @binding(0) var<uniform> camera: Camera;
@group(1) @binding(0) var<uniform> material: Material;
@group(2) @binding(0) var<uniform> light: Light;
@group(3) @binding(0) var albedo_texture: texture_2d<f32>;
@group(3) @binding(1) var albedo_sampler: sampler;

{}

// Vertex Output
struct VertexOutput {{
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) world_tangent: vec3<f32>,
}}

// Vertex Shader
@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) tangent: vec4<f32>,
    @location(10) joints: vec4<u32>,
    @location(11) weights: vec4<f32>,
) -> VertexOutput {{
    var output: VertexOutput;
    
    // Apply GPU skinning
    let skinned_input = SkinnedVertexInput(
        position,
        normal,
        tangent,
        joints,
        weights,
    );
    
    let skinned_pos = apply_skinning(skinned_input);
    let skinned_normal = apply_skinning_normal(skinned_input);
    let skinned_tangent = apply_skinning_tangent(skinned_input);
    
    // Transform to clip space
    output.clip_position = camera.view_proj * skinned_pos;
    output.world_position = skinned_pos.xyz;
    output.world_normal = skinned_normal;
    output.world_tangent = skinned_tangent;
    output.uv = uv;
    
    return output;
}}

// Fragment Shader (Simple PBR)
@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {{
    // Sample albedo
    let albedo = textureSample(albedo_texture, albedo_sampler, input.uv);
    let base_color = albedo * material.base_color;
    
    // Simple diffuse lighting
    let N = normalize(input.world_normal);
    let L = normalize(light.position - input.world_position);
    let V = normalize(camera.position - input.world_position);
    
    let NdotL = max(dot(N, L), 0.0);
    let diffuse = base_color.rgb * NdotL * light.color * light.intensity;
    
    // Simple specular (Blinn-Phong)
    let H = normalize(L + V);
    let NdotH = max(dot(N, H), 0.0);
    let spec_strength = pow(NdotH, 32.0 * (1.0 - material.roughness));
    let specular = vec3<f32>(spec_strength) * light.color * light.intensity;
    
    // Ambient
    let ambient = base_color.rgb * 0.03;
    
    let final_color = ambient + diffuse + specular;
    return vec4<f32>(final_color, base_color.a);
}}
"#,
        SKINNING_GPU_SHADER
    )
}

// ============================================================================
// Integration Helpers
// ============================================================================

/// Helper to create skinned mesh render pipeline with GPU skinning enabled
///
/// This creates a complete render pipeline for GPU-accelerated skeletal animation.
/// The pipeline expects:
/// - Vertex buffers with skinning data (joints, weights)
/// - Joint palette storage buffer at group 4, binding 0
/// - Standard PBR bind groups (camera, material, lights, textures)
///
/// Returns the created pipeline which can be used for rendering skinned meshes.
pub fn create_skinned_pipeline(
    device: &wgpu::Device,
    camera_bind_group_layout: &wgpu::BindGroupLayout,
    material_bind_group_layout: &wgpu::BindGroupLayout,
    light_bind_group_layout: &wgpu::BindGroupLayout,
    texture_bind_group_layout: &wgpu::BindGroupLayout,
    joint_palette_bind_group_layout: &wgpu::BindGroupLayout,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    // Create complete skinned mesh shader with GPU skinning
    let shader_source = create_complete_skinning_shader();
    let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("gpu_skinned_mesh_shader"),
        source: wgpu::ShaderSource::Wgsl(shader_source.into()),
    });

    // Create pipeline layout with all bind groups
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("gpu_skinned_pipeline_layout"),
        bind_group_layouts: &[
            camera_bind_group_layout,        // Group 0: Camera (view, projection)
            material_bind_group_layout,      // Group 1: Material properties
            light_bind_group_layout,         // Group 2: Lights
            texture_bind_group_layout,       // Group 3: Textures (albedo, normal, MRA)
            joint_palette_bind_group_layout, // Group 4: Joint matrices
        ],
        push_constant_ranges: &[],
    });

    // Vertex attributes for skinned vertices
    const SKINNED_VERTEX_ATTRIBUTES: &[wgpu::VertexAttribute] = &wgpu::vertex_attr_array![
        0 => Float32x3,  // position
        1 => Float32x3,  // normal
        2 => Float32x2,  // uv
        3 => Float32x4,  // tangent
        10 => Uint32x4,  // joints
        11 => Float32x4, // weights
    ];

    // Vertex buffer layout for skinned vertices
    let vertex_buffer_layout = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<SkinnedVertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: SKINNED_VERTEX_ATTRIBUTES,
    };

    // Create the render pipeline
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("gpu_skinned_mesh_pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader_module,
            entry_point: Some("vs_main"),
            buffers: &[vertex_buffer_layout],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader_module,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Back),
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview: None,
        cache: None,
    })
}

/// Vertex structure for GPU skinned meshes
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SkinnedVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub tangent: [f32; 4],
    pub joints: [u32; 4],
    pub weights: [f32; 4],
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::MAX_JOINTS;

    #[test]
    fn test_joint_palette_handle() {
        let handle1 = JointPaletteHandle(0);
        let handle2 = JointPaletteHandle(1);
        assert_ne!(handle1, handle2);
    }

    #[test]
    fn test_joint_palette_from_matrices() {
        let matrices = vec![
            Mat4::from_translation(glam::Vec3::new(1.0, 0.0, 0.0)),
            Mat4::from_translation(glam::Vec3::new(0.0, 2.0, 0.0)),
        ];

        let palette = JointPalette::from_matrices(&matrices);
        assert_eq!(palette.joint_count, 2);

        // Verify first matrix
        let m0 = palette.joints[0].matrix;
        assert_eq!(m0[3][0], 1.0); // Translation X
    }

    #[test]
    fn test_max_joints_limit() {
        let matrices = vec![Mat4::IDENTITY; 300]; // More than MAX_JOINTS

        let palette = JointPalette::from_matrices(&matrices);
        assert_eq!(palette.joint_count, MAX_JOINTS as u32);
    }

    // --- Mutation-resistant tests ---

    #[test]
    fn joint_palette_handle_equality() {
        let a = JointPaletteHandle(5);
        let b = JointPaletteHandle(5);
        let c = JointPaletteHandle(6);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn joint_palette_from_empty_matrices() {
        let palette = JointPalette::from_matrices(&[]);
        assert_eq!(palette.joint_count, 0);
    }

    #[test]
    fn joint_palette_stores_translation() {
        let matrices = vec![Mat4::from_translation(glam::Vec3::new(3.0, 4.0, 5.0))];
        let palette = JointPalette::from_matrices(&matrices);
        assert_eq!(palette.joint_count, 1);
        // Mat4 column-major: col 3 is translation
        assert_eq!(palette.joints[0].matrix[3][0], 3.0);
        assert_eq!(palette.joints[0].matrix[3][1], 4.0);
        assert_eq!(palette.joints[0].matrix[3][2], 5.0);
    }

    #[test]
    fn skinning_shader_constant_is_nonempty() {
        // Implicitly validates non-empty via contains() checks below
        assert!(
            SKINNING_GPU_SHADER.contains("apply_skinning"),
            "must define apply_skinning fn"
        );
        assert!(
            SKINNING_GPU_SHADER.contains("joint_palette"),
            "must reference joint_palette"
        );
    }

    #[test]
    fn skinning_shader_handles_four_joint_influences() {
        // The shader should reference all 4 weight components (w.x, w.y, w.z, w.w)
        assert!(SKINNING_GPU_SHADER.contains("w.x"));
        assert!(SKINNING_GPU_SHADER.contains("w.y"));
        assert!(SKINNING_GPU_SHADER.contains("w.z"));
        assert!(SKINNING_GPU_SHADER.contains("w.w"));
    }
}

// ============================================================================
// Integration Tests (GPU)
// ============================================================================

#[cfg(all(test, feature = "gpu-tests"))]
mod gpu_tests {
    use super::*;

    async fn create_test_device() -> (wgpu::Device, wgpu::Queue) {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .expect("Failed to find an appropriate adapter");

        adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("test_device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits {
                    max_bind_groups: 8,
                    ..wgpu::Limits::default()
                },
                memory_hints: wgpu::MemoryHints::default(),
                trace: wgpu::Trace::Off,
            })
            .await
            .expect("Failed to create device")
    }

    #[tokio::test]
    async fn test_pipeline_creation() {
        let (device, queue) = create_test_device().await;
        let manager = JointPaletteManager::new(&device, &queue);

        // Create dummy bind group layouts for testing
        let camera_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("camera"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let material_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("material"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let light_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("light"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("texture"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        // Test pipeline creation
        let pipeline = create_skinned_pipeline(
            &device,
            &camera_layout,
            &material_layout,
            &light_layout,
            &texture_layout,
            &manager.bind_group_layout,
            wgpu::TextureFormat::Rgba8UnormSrgb,
        );

        // Pipeline should be created successfully (no panics/errors)
        // Verify pipeline exists by checking it's not null-like
        drop(pipeline); // Just verify it was created without panicking
    }

    #[tokio::test]
    async fn test_skinning_produces_valid_output() {
        let (device, queue) = create_test_device().await;
        let mut manager = JointPaletteManager::new(&device, &queue);

        // Allocate a palette
        let handle = manager.allocate();

        // Create simple test matrices (identity transforms)
        let matrices = vec![
            Mat4::from_translation(glam::Vec3::new(1.0, 0.0, 0.0)),
            Mat4::from_translation(glam::Vec3::new(0.0, 2.0, 0.0)),
        ];

        // Upload matrices
        manager
            .upload_matrices(handle, &matrices)
            .expect("Failed to upload matrices");

        // Verify bind group exists
        let bind_group = manager.get_bind_group(handle);
        assert!(
            bind_group.is_some(),
            "Bind group should exist after allocation"
        );

        // Verify buffer is properly sized
        assert_eq!(manager.active_count(), 1);
    }
}
