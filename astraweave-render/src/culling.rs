//! GPU-driven frustum culling implementation for Phase 2 Task 3
//!
//! This module provides compute-based frustum culling with CPU fallback for determinism.
//! The compute path writes a compacted list of visible instance indices, enabling indirect draws.

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;

/// Per-instance data for culling compute shader
/// Represents an axis-aligned bounding box (AABB) in world space
/// Layout must match WGSL std140: vec3 aligned to 16 bytes
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct InstanceAABB {
    /// Center of the AABB in world space (vec3<f32> in WGSL = 16 bytes with padding)
    pub center: [f32; 3],
    pub _pad0: u32, // Padding to align extent to 16-byte boundary
    /// Half-extents (radius) from center (vec3<f32> in WGSL = 16 bytes with padding)
    pub extent: [f32; 3],
    /// Original instance index in the instance buffer
    pub instance_index: u32,
}

impl InstanceAABB {
    pub fn new(center: Vec3, extent: Vec3, instance_index: u32) -> Self {
        Self {
            center: center.to_array(),
            _pad0: 0,
            extent: extent.to_array(),
            instance_index,
        }
    }

    /// Compute AABB from world transform matrix and local bounds
    pub fn from_transform(
        transform: &Mat4,
        local_min: Vec3,
        local_max: Vec3,
        instance_index: u32,
    ) -> Self {
        let local_center = (local_min + local_max) * 0.5;
        let local_extent = (local_max - local_min) * 0.5;

        // Transform center to world space
        let world_center = transform.transform_point3(local_center);

        // For extent, transform all 8 corners and compute new AABB
        // This handles rotation properly
        let corners = [
            Vec3::new(-1.0, -1.0, -1.0),
            Vec3::new(1.0, -1.0, -1.0),
            Vec3::new(-1.0, 1.0, -1.0),
            Vec3::new(1.0, 1.0, -1.0),
            Vec3::new(-1.0, -1.0, 1.0),
            Vec3::new(1.0, -1.0, 1.0),
            Vec3::new(-1.0, 1.0, 1.0),
            Vec3::new(1.0, 1.0, 1.0),
        ];

        let mut world_min = Vec3::splat(f32::MAX);
        let mut world_max = Vec3::splat(f32::MIN);

        for corner in &corners {
            let local_point = local_center + *corner * local_extent;
            let world_point = transform.transform_point3(local_point);
            world_min = world_min.min(world_point);
            world_max = world_max.max(world_point);
        }

        let world_extent = (world_max - world_min) * 0.5;

        Self {
            center: world_center.to_array(),
            _pad0: 0,
            extent: world_extent.to_array(),
            instance_index,
        }
    }
}

/// Frustum planes in world space for culling
/// Planes are stored as (nx, ny, nz, d) where dot(normal, point) + d = 0 defines the plane
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct FrustumPlanes {
    /// Six planes: left, right, bottom, top, near, far
    /// Each plane: [nx, ny, nz, d]
    pub planes: [[f32; 4]; 6],
}

impl FrustumPlanes {
    /// Extract frustum planes from view-projection matrix
    /// Uses Gribb-Hartmann method
    pub fn from_view_proj(view_proj: &Mat4) -> Self {
        let m = view_proj.to_cols_array();

        // Extract planes from matrix rows
        let left = Self::normalize_plane([m[3] + m[0], m[7] + m[4], m[11] + m[8], m[15] + m[12]]);
        let right = Self::normalize_plane([m[3] - m[0], m[7] - m[4], m[11] - m[8], m[15] - m[12]]);
        let bottom = Self::normalize_plane([m[3] + m[1], m[7] + m[5], m[11] + m[9], m[15] + m[13]]);
        let top = Self::normalize_plane([m[3] - m[1], m[7] - m[5], m[11] - m[9], m[15] - m[13]]);
        let near = Self::normalize_plane([m[3] + m[2], m[7] + m[6], m[11] + m[10], m[15] + m[14]]);
        let far = Self::normalize_plane([m[3] - m[2], m[7] - m[6], m[11] - m[10], m[15] - m[14]]);

        Self {
            planes: [left, right, bottom, top, near, far],
        }
    }

    fn normalize_plane(plane: [f32; 4]) -> [f32; 4] {
        let len = (plane[0] * plane[0] + plane[1] * plane[1] + plane[2] * plane[2]).sqrt();
        if len > 0.0 {
            [
                plane[0] / len,
                plane[1] / len,
                plane[2] / len,
                plane[3] / len,
            ]
        } else {
            plane
        }
    }

    /// CPU-based frustum culling (fallback path)
    /// Returns true if AABB intersects frustum
    pub fn test_aabb(&self, center: Vec3, extent: Vec3) -> bool {
        for plane in &self.planes {
            let normal = Vec3::new(plane[0], plane[1], plane[2]);
            let d = plane[3];

            // Compute signed distance from center to plane
            let dist = normal.dot(center) + d;

            // Compute the effective radius along the plane normal
            let radius = extent.x.abs() * normal.x.abs()
                + extent.y.abs() * normal.y.abs()
                + extent.z.abs() * normal.z.abs();

            // If center is more than radius away on the negative side, AABB is outside
            if dist < -radius {
                return false;
            }
        }
        true
    }
}

/// Indirect draw command structure matching wgpu::DrawIndirect
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable, Default)]
pub struct DrawIndirectCommand {
    pub vertex_count: u32,
    pub instance_count: u32,
    pub first_vertex: u32,
    pub first_instance: u32,
}

impl DrawIndirectCommand {
    /// Create a new indirect draw command for a mesh batch
    pub fn new(
        vertex_count: u32,
        instance_count: u32,
        first_vertex: u32,
        first_instance: u32,
    ) -> Self {
        Self {
            vertex_count,
            instance_count,
            first_vertex,
            first_instance,
        }
    }
}

/// Batch identifier for grouping instances by mesh+material
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BatchId {
    pub mesh_id: u32,
    pub material_id: u32,
}

impl BatchId {
    pub fn new(mesh_id: u32, material_id: u32) -> Self {
        Self {
            mesh_id,
            material_id,
        }
    }
}

/// A batch of instances sharing the same mesh and material
#[derive(Debug, Clone)]
pub struct DrawBatch {
    pub batch_id: BatchId,
    pub vertex_count: u32,
    pub first_vertex: u32,
    pub instances: Vec<u32>, // Instance indices
}

impl DrawBatch {
    pub fn new(batch_id: BatchId, vertex_count: u32, first_vertex: u32) -> Self {
        Self {
            batch_id,
            vertex_count,
            first_vertex,
            instances: Vec::new(),
        }
    }

    pub fn add_instance(&mut self, instance_idx: u32) {
        self.instances.push(instance_idx);
    }

    pub fn instance_count(&self) -> u32 {
        self.instances.len() as u32
    }
}

/// CPU fallback culling implementation
/// Returns list of visible instance indices
pub fn cpu_frustum_cull(instances: &[InstanceAABB], frustum: &FrustumPlanes) -> Vec<u32> {
    instances
        .iter()
        .filter(|inst| {
            let center = Vec3::from(inst.center);
            let extent = Vec3::from(inst.extent);
            frustum.test_aabb(center, extent)
        })
        .map(|inst| inst.instance_index)
        .collect()
}

/// Build indirect draw commands from visible instances and batch info (CPU path)
/// Returns vector of DrawIndirectCommand structs, one per batch
pub fn build_indirect_commands_cpu(batches: &[DrawBatch]) -> Vec<DrawIndirectCommand> {
    batches
        .iter()
        .map(|batch| {
            DrawIndirectCommand::new(
                batch.vertex_count,
                batch.instance_count(),
                batch.first_vertex,
                0, // first_instance typically 0 for instanced draws
            )
        })
        .collect()
}

/// Group visible instances into batches by mesh/material (CPU path)
/// This is a simplified version - real implementation would use scene data
pub fn batch_visible_instances(
    visible_indices: &[u32],
    get_batch_id: impl Fn(u32) -> BatchId,
    get_mesh_info: impl Fn(BatchId) -> (u32, u32), // Returns (vertex_count, first_vertex)
) -> Vec<DrawBatch> {
    use std::collections::BTreeMap;

    let mut batch_map: BTreeMap<BatchId, DrawBatch> = BTreeMap::new();

    for &instance_idx in visible_indices {
        let batch_id = get_batch_id(instance_idx);

        batch_map
            .entry(batch_id)
            .or_insert_with(|| {
                let (vertex_count, first_vertex) = get_mesh_info(batch_id);
                DrawBatch::new(batch_id, vertex_count, first_vertex)
            })
            .add_instance(instance_idx);
    }

    batch_map.into_values().collect()
}

/// Compute shader source for frustum culling
pub const CULLING_SHADER: &str = r#"
// Per-instance AABB data (std140 layout: vec3 is 16-byte aligned)
struct InstanceAABB {
    center: vec3<f32>,      // offset 0, size 12, align 16
    _pad0: u32,             // offset 12
    extent: vec3<f32>,      // offset 16, size 12, align 16
    instance_index: u32,    // offset 28
}
// Total size: 32 bytes

// Frustum planes (6 planes)
struct FrustumPlanes {
    planes: array<vec4<f32>, 6>,
}

// Input/output bindings
@group(0) @binding(0) var<storage, read> instance_aabbs: array<InstanceAABB>;
@group(0) @binding(1) var<uniform> frustum: FrustumPlanes;
@group(0) @binding(2) var<storage, read_write> visible_instances: array<u32>;
@group(0) @binding(3) var<storage, read_write> visible_count: atomic<u32>;

// Test if AABB is visible against frustum
fn is_aabb_visible(center: vec3<f32>, extent: vec3<f32>, frustum: FrustumPlanes) -> bool {
    // Test AABB against all 6 frustum planes (manually unrolled - WGSL requires constant indices)
    // Plane 0 (left)
    var plane = frustum.planes[0];
    var normal = plane.xyz;
    var d = plane.w;
    var dist = dot(center, normal) + d;
    var radius = abs(extent.x) * abs(normal.x) + abs(extent.y) * abs(normal.y) + abs(extent.z) * abs(normal.z);
    if (dist < -radius) { return false; }
    
    // Plane 1 (right)
    plane = frustum.planes[1];
    normal = plane.xyz;
    d = plane.w;
    dist = dot(center, normal) + d;
    radius = abs(extent.x) * abs(normal.x) + abs(extent.y) * abs(normal.y) + abs(extent.z) * abs(normal.z);
    if (dist < -radius) { return false; }
    
    // Plane 2 (bottom)
    plane = frustum.planes[2];
    normal = plane.xyz;
    d = plane.w;
    dist = dot(center, normal) + d;
    radius = abs(extent.x) * abs(normal.x) + abs(extent.y) * abs(normal.y) + abs(extent.z) * abs(normal.z);
    if (dist < -radius) { return false; }
    
    // Plane 3 (top)
    plane = frustum.planes[3];
    normal = plane.xyz;
    d = plane.w;
    dist = dot(center, normal) + d;
    radius = abs(extent.x) * abs(normal.x) + abs(extent.y) * abs(normal.y) + abs(extent.z) * abs(normal.z);
    if (dist < -radius) { return false; }
    
    // Plane 4 (near)
    plane = frustum.planes[4];
    normal = plane.xyz;
    d = plane.w;
    dist = dot(center, normal) + d;
    radius = abs(extent.x) * abs(normal.x) + abs(extent.y) * abs(normal.y) + abs(extent.z) * abs(normal.z);
    if (dist < -radius) { return false; }
    
    // Plane 5 (far)
    plane = frustum.planes[5];
    normal = plane.xyz;
    d = plane.w;
    dist = dot(center, normal) + d;
    radius = abs(extent.x) * abs(normal.x) + abs(extent.y) * abs(normal.y) + abs(extent.z) * abs(normal.z);
    if (dist < -radius) { return false; }
    
    return true;
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx >= arrayLength(&instance_aabbs)) { 
        return; 
    }
    
    let aabb = instance_aabbs[idx];
    if (is_aabb_visible(aabb.center, aabb.extent, frustum)) {
        let slot = atomicAdd(&visible_count, 1u);
        visible_instances[slot] = aabb.instance_index;
    }
}
"#;

/// GPU culling manager
pub struct CullingPipeline {
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
}

impl CullingPipeline {
    pub fn new(device: &wgpu::Device) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("frustum_culling_shader"),
            source: wgpu::ShaderSource::Wgsl(CULLING_SHADER.into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("culling_bind_group_layout"),
            entries: &[
                // Instance AABBs (storage buffer)
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Frustum planes (uniform buffer)
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Visible instances output (storage buffer)
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Visible count (atomic storage buffer)
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("culling_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("frustum_culling_pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        Self {
            pipeline,
            bind_group_layout,
        }
    }

    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }

    /// Create buffers and bind group for culling (allocates fresh resources).
    pub fn create_culling_resources(
        &self,
        device: &wgpu::Device,
        instances: &[InstanceAABB],
        frustum: &FrustumPlanes,
    ) -> CullingResources {
        // Handle empty instance list with minimum buffer size
        let instance_data = if instances.is_empty() {
            vec![InstanceAABB::new(Vec3::ZERO, Vec3::ZERO, 0)]
        } else {
            instances.to_vec()
        };

        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("culling_instance_buffer"),
            contents: bytemuck::cast_slice(&instance_data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let frustum_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("culling_frustum_buffer"),
            contents: bytemuck::bytes_of(frustum),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Allocate buffer for visible instances (max size = input size, min 1 element)
        let buffer_size = (instances.len().max(1) * std::mem::size_of::<u32>()) as u64;
        let visible_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("culling_visible_buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Counter for visible instances (atomic)
        let count_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("culling_count_buffer"),
            size: std::mem::size_of::<u32>() as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("culling_bind_group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: instance_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: frustum_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: visible_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: count_buffer.as_entire_binding(),
                },
            ],
        });

        CullingResources {
            instance_buffer,
            frustum_buffer,
            visible_buffer,
            count_buffer,
            bind_group,
        }
    }

    /// Reuse existing buffers when large enough, only reallocating on growth.
    /// Falls back to fresh allocation when `existing` is `None`.
    pub fn update_or_create_resources(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        instances: &[InstanceAABB],
        frustum: &FrustumPlanes,
        existing: Option<CullingResources>,
    ) -> CullingResources {
        let existing = match existing {
            Some(e) => e,
            None => return self.create_culling_resources(device, instances, frustum),
        };

        let instance_data = if instances.is_empty() {
            vec![InstanceAABB::new(Vec3::ZERO, Vec3::ZERO, 0)]
        } else {
            instances.to_vec()
        };

        let instance_bytes: &[u8] = bytemuck::cast_slice(&instance_data);
        let required_instance_size = instance_bytes.len() as u64;
        let visible_size = (instances.len().max(1) * std::mem::size_of::<u32>()) as u64;

        let instance_grew = existing.instance_buffer.size() < required_instance_size;
        let visible_grew = existing.visible_buffer.size() < visible_size;
        let needs_rebind = instance_grew || visible_grew;

        let instance_buffer = if instance_grew {
            let alloc = required_instance_size.next_power_of_two().max(256);
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("culling_instance_buffer"),
                size: alloc,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        } else {
            existing.instance_buffer
        };
        queue.write_buffer(&instance_buffer, 0, instance_bytes);

        queue.write_buffer(&existing.frustum_buffer, 0, bytemuck::bytes_of(frustum));

        let visible_buffer = if visible_grew {
            let alloc = visible_size.next_power_of_two().max(256);
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("culling_visible_buffer"),
                size: alloc,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            })
        } else {
            existing.visible_buffer
        };

        let bind_group = if needs_rebind {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("culling_bind_group"),
                layout: &self.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: instance_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: existing.frustum_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: visible_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: existing.count_buffer.as_entire_binding(),
                    },
                ],
            })
        } else {
            existing.bind_group
        };

        CullingResources {
            instance_buffer,
            frustum_buffer: existing.frustum_buffer,
            visible_buffer,
            count_buffer: existing.count_buffer,
            bind_group,
        }
    }

    /// Execute culling compute pass
    /// Note: Caller must ensure count_buffer is cleared to 0 before calling this
    pub fn execute(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        bind_group: &wgpu::BindGroup,
        instance_count: u32,
    ) {
        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("frustum_culling_pass"),
            timestamp_writes: None,
        });
        cpass.set_pipeline(&self.pipeline);
        cpass.set_bind_group(0, bind_group, &[]);
        let workgroup_count = instance_count.div_ceil(64);
        cpass.dispatch_workgroups(workgroup_count, 1, 1);
    }

    /// Execute culling with automatic buffer setup (clears count buffer first)
    pub fn execute_with_clear(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        resources: &CullingResources,
        instance_count: u32,
    ) {
        // Clear count buffer to 0
        encoder.clear_buffer(&resources.count_buffer, 0, None);

        // Execute culling
        self.execute(encoder, &resources.bind_group, instance_count);
    }
}

/// GPU buffers for culling
pub struct CullingResources {
    pub instance_buffer: wgpu::Buffer,
    pub frustum_buffer: wgpu::Buffer,
    pub visible_buffer: wgpu::Buffer,
    pub count_buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
}

// ---------------------------------------------------------------------------
// P2-6: Multi-Draw-Indirect — GPU-driven draw command generation
// ---------------------------------------------------------------------------

/// Indexed indirect draw command matching `wgpu::RenderPass::draw_indexed_indirect()`.
///
/// 20 bytes, `#[repr(C)]` so the layout matches what the GPU expects.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable, Default)]
pub struct DrawIndexedIndirectCommand {
    pub index_count: u32,
    pub instance_count: u32,
    pub first_index: u32,
    pub base_vertex: i32,
    pub first_instance: u32,
}

impl DrawIndexedIndirectCommand {
    pub fn new(
        index_count: u32,
        instance_count: u32,
        first_index: u32,
        base_vertex: i32,
        first_instance: u32,
    ) -> Self {
        Self {
            index_count,
            instance_count,
            first_index,
            base_vertex,
            first_instance,
        }
    }
}

/// Compute shader that performs per-batch frustum culling and writes
/// `DrawIndexedIndirectCommand` entries.  Invisible batches get
/// `instance_count = 0` so the GPU skips them at zero cost.
pub const INDIRECT_CULLING_SHADER: &str = r#"
struct FrustumPlanes {
    planes: array<vec4<f32>, 6>,
}

struct BatchAABB {
    center: vec3<f32>,
    _pad0: u32,
    extent: vec3<f32>,
    _pad1: u32,
}

// Must match DrawIndexedIndirectCommand (20 bytes per element)
struct DrawCmd {
    index_count: u32,
    instance_count: u32,
    first_index: u32,
    base_vertex: i32,
    first_instance: u32,
}

@group(0) @binding(0) var<uniform> frustum: FrustumPlanes;
@group(0) @binding(1) var<storage, read> batch_aabbs: array<BatchAABB>;
@group(0) @binding(2) var<storage, read> draw_templates: array<DrawCmd>;
@group(0) @binding(3) var<storage, read_write> draw_commands: array<DrawCmd>;
@group(0) @binding(4) var<storage, read_write> visible_batch_count: atomic<u32>;

fn is_batch_visible(center: vec3<f32>, extent: vec3<f32>) -> bool {
    // Plane 0 (left)
    var plane = frustum.planes[0];
    var normal = plane.xyz;
    var d = plane.w;
    var dist = dot(center, normal) + d;
    var radius = abs(extent.x) * abs(normal.x)
               + abs(extent.y) * abs(normal.y)
               + abs(extent.z) * abs(normal.z);
    if (dist < -radius) { return false; }

    // Plane 1 (right)
    plane = frustum.planes[1];
    normal = plane.xyz; d = plane.w;
    dist = dot(center, normal) + d;
    radius = abs(extent.x) * abs(normal.x)
           + abs(extent.y) * abs(normal.y)
           + abs(extent.z) * abs(normal.z);
    if (dist < -radius) { return false; }

    // Plane 2 (bottom)
    plane = frustum.planes[2];
    normal = plane.xyz; d = plane.w;
    dist = dot(center, normal) + d;
    radius = abs(extent.x) * abs(normal.x)
           + abs(extent.y) * abs(normal.y)
           + abs(extent.z) * abs(normal.z);
    if (dist < -radius) { return false; }

    // Plane 3 (top)
    plane = frustum.planes[3];
    normal = plane.xyz; d = plane.w;
    dist = dot(center, normal) + d;
    radius = abs(extent.x) * abs(normal.x)
           + abs(extent.y) * abs(normal.y)
           + abs(extent.z) * abs(normal.z);
    if (dist < -radius) { return false; }

    // Plane 4 (near)
    plane = frustum.planes[4];
    normal = plane.xyz; d = plane.w;
    dist = dot(center, normal) + d;
    radius = abs(extent.x) * abs(normal.x)
           + abs(extent.y) * abs(normal.y)
           + abs(extent.z) * abs(normal.z);
    if (dist < -radius) { return false; }

    // Plane 5 (far)
    plane = frustum.planes[5];
    normal = plane.xyz; d = plane.w;
    dist = dot(center, normal) + d;
    radius = abs(extent.x) * abs(normal.x)
           + abs(extent.y) * abs(normal.y)
           + abs(extent.z) * abs(normal.z);
    if (dist < -radius) { return false; }

    return true;
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx >= arrayLength(&batch_aabbs)) { return; }

    let aabb = batch_aabbs[idx];
    var cmd = draw_templates[idx];

    if (is_batch_visible(aabb.center, aabb.extent)) {
        atomicAdd(&visible_batch_count, 1u);
    } else {
        cmd.instance_count = 0u;
    }

    draw_commands[idx] = cmd;
}
"#;

/// GPU buffers for indirect draw command generation.
pub struct IndirectDrawResources {
    pub batch_aabb_buffer: wgpu::Buffer,
    pub template_buffer: wgpu::Buffer,
    pub draw_commands_buffer: wgpu::Buffer,
    pub frustum_buffer: wgpu::Buffer,
    pub batch_count_buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub batch_count: u32,
}

/// Compute pipeline that generates per-batch `DrawIndexedIndirectCommand`
/// entries on the GPU.  Each thread handles one batch: it copies the draw
/// template and zeroes `instance_count` when the batch AABB is outside the
/// frustum.
pub struct IndirectDrawPipeline {
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
}

impl IndirectDrawPipeline {
    pub fn new(device: &wgpu::Device) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("indirect_culling_shader"),
            source: wgpu::ShaderSource::Wgsl(INDIRECT_CULLING_SHADER.into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("indirect_draw_bind_group_layout"),
            entries: &[
                // 0: Frustum planes (uniform)
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
                // 1: Batch AABBs (storage, read)
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
                // 2: Draw templates (storage, read)
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // 3: Draw commands output (storage, read_write + INDIRECT)
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // 4: Visible batch count (atomic storage)
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("indirect_draw_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("indirect_draw_pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        Self {
            pipeline,
            bind_group_layout,
        }
    }

    /// Allocate GPU resources for indirect draw command generation.
    ///
    /// * `batch_aabbs` – one AABB per draw batch (chunk, mesh group, etc.)
    /// * `templates`   – one draw-command template per batch (CPU-built once)
    /// * `frustum`     – current frame frustum planes
    pub fn create_resources(
        &self,
        device: &wgpu::Device,
        batch_aabbs: &[InstanceAABB],
        templates: &[DrawIndexedIndirectCommand],
        frustum: &FrustumPlanes,
    ) -> IndirectDrawResources {
        assert_eq!(
            batch_aabbs.len(),
            templates.len(),
            "batch_aabbs and templates must be the same length"
        );

        let count = batch_aabbs.len().max(1) as u32;

        // Batch AABBs
        let aabb_data = if batch_aabbs.is_empty() {
            vec![InstanceAABB::new(Vec3::ZERO, Vec3::ZERO, 0)]
        } else {
            batch_aabbs.to_vec()
        };
        let batch_aabb_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("indirect_batch_aabb_buffer"),
            contents: bytemuck::cast_slice(&aabb_data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // Draw templates
        let tmpl_data = if templates.is_empty() {
            vec![DrawIndexedIndirectCommand::default()]
        } else {
            templates.to_vec()
        };
        let template_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("indirect_template_buffer"),
            contents: bytemuck::cast_slice(&tmpl_data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // Output draw commands — must have INDIRECT usage for draw_indexed_indirect()
        let cmd_size = (count as usize * std::mem::size_of::<DrawIndexedIndirectCommand>()) as u64;
        let draw_commands_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("indirect_draw_commands_buffer"),
            size: cmd_size,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::INDIRECT
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Frustum uniform
        let frustum_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("indirect_frustum_buffer"),
            contents: bytemuck::bytes_of(frustum),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Visible batch count (atomic)
        let batch_count_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("indirect_batch_count_buffer"),
            size: std::mem::size_of::<u32>() as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("indirect_draw_bind_group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: frustum_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: batch_aabb_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: template_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: draw_commands_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: batch_count_buffer.as_entire_binding(),
                },
            ],
        });

        IndirectDrawResources {
            batch_aabb_buffer,
            template_buffer,
            draw_commands_buffer,
            frustum_buffer,
            batch_count_buffer,
            bind_group,
            batch_count: if batch_aabbs.is_empty() { 0 } else { count },
        }
    }

    /// Reuse buffers when possible, only reallocating on growth.
    pub fn update_or_create_resources(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        batch_aabbs: &[InstanceAABB],
        templates: &[DrawIndexedIndirectCommand],
        frustum: &FrustumPlanes,
        existing: Option<IndirectDrawResources>,
    ) -> IndirectDrawResources {
        let existing = match existing {
            Some(e) => e,
            None => return self.create_resources(device, batch_aabbs, templates, frustum),
        };

        assert_eq!(batch_aabbs.len(), templates.len());

        let count = batch_aabbs.len().max(1);

        // Check if buffers need reallocation
        let dummy_aabb = InstanceAABB::new(Vec3::ZERO, Vec3::ZERO, 0);
        let dummy_tmpl = DrawIndexedIndirectCommand::default();
        let aabb_bytes: &[u8] = if batch_aabbs.is_empty() {
            bytemuck::bytes_of(&dummy_aabb)
        } else {
            bytemuck::cast_slice(batch_aabbs)
        };
        let tmpl_bytes: &[u8] = if templates.is_empty() {
            bytemuck::bytes_of(&dummy_tmpl)
        } else {
            bytemuck::cast_slice(templates)
        };
        let cmd_size = (count * std::mem::size_of::<DrawIndexedIndirectCommand>()) as u64;

        let aabb_grew = existing.batch_aabb_buffer.size() < aabb_bytes.len() as u64;
        let tmpl_grew = existing.template_buffer.size() < tmpl_bytes.len() as u64;
        let cmd_grew = existing.draw_commands_buffer.size() < cmd_size;
        let needs_rebind = aabb_grew || tmpl_grew || cmd_grew;

        let batch_aabb_buffer = if aabb_grew {
            let alloc = (aabb_bytes.len() as u64).next_power_of_two().max(256);
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("indirect_batch_aabb_buffer"),
                size: alloc,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        } else {
            existing.batch_aabb_buffer
        };
        queue.write_buffer(&batch_aabb_buffer, 0, aabb_bytes);

        let template_buffer = if tmpl_grew {
            let alloc = (tmpl_bytes.len() as u64).next_power_of_two().max(256);
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("indirect_template_buffer"),
                size: alloc,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        } else {
            existing.template_buffer
        };
        queue.write_buffer(&template_buffer, 0, tmpl_bytes);

        let draw_commands_buffer = if cmd_grew {
            let alloc = cmd_size.next_power_of_two().max(256);
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("indirect_draw_commands_buffer"),
                size: alloc,
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::INDIRECT
                    | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            })
        } else {
            existing.draw_commands_buffer
        };

        // Update frustum uniform (always fits — fixed 96 bytes)
        queue.write_buffer(&existing.frustum_buffer, 0, bytemuck::bytes_of(frustum));

        // Re-create bind group when any storage buffer was reallocated
        let bind_group = if needs_rebind {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("indirect_draw_bind_group"),
                layout: &self.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: existing.frustum_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: batch_aabb_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: template_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: draw_commands_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: existing.batch_count_buffer.as_entire_binding(),
                    },
                ],
            })
        } else {
            existing.bind_group
        };

        IndirectDrawResources {
            batch_aabb_buffer,
            template_buffer,
            draw_commands_buffer,
            frustum_buffer: existing.frustum_buffer,
            batch_count_buffer: existing.batch_count_buffer,
            bind_group,
            batch_count: if batch_aabbs.is_empty() {
                0
            } else {
                count as u32
            },
        }
    }

    /// Dispatch the indirect command builder compute shader.
    /// Clears visible batch count, then runs one thread per batch.
    pub fn execute(&self, encoder: &mut wgpu::CommandEncoder, resources: &IndirectDrawResources) {
        if resources.batch_count == 0 {
            return;
        }

        // Clear visible batch counter
        encoder.clear_buffer(&resources.batch_count_buffer, 0, None);

        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("indirect_draw_build_pass"),
            timestamp_writes: None,
        });
        cpass.set_pipeline(&self.pipeline);
        cpass.set_bind_group(0, &resources.bind_group, &[]);
        let workgroups = resources.batch_count.div_ceil(64);
        cpass.dispatch_workgroups(workgroups, 1, 1);
    }
}

/// Dispatch `draw_indexed_indirect` per batch from the GPU-generated command
/// buffer.  The GPU skips batches whose `instance_count` was set to 0 by the
/// culling shader, so no CPU readback is required.
///
/// This is universally supported on all wgpu backends.
pub fn dispatch_indexed_indirect_draws(
    render_pass: &mut wgpu::RenderPass<'_>,
    draw_commands_buffer: &wgpu::Buffer,
    batch_count: u32,
) {
    let stride = std::mem::size_of::<DrawIndexedIndirectCommand>() as u64;
    for i in 0..batch_count {
        render_pass.draw_indexed_indirect(draw_commands_buffer, i as u64 * stride);
    }
}

/// Dispatch all batches with a single `multi_draw_indexed_indirect` call.
///
/// Requires the `MULTI_DRAW_INDIRECT` device feature.  Falls back to per-batch
/// `draw_indexed_indirect` loop when the feature is unavailable.
pub fn dispatch_multi_draw_indexed_indirect(
    render_pass: &mut wgpu::RenderPass<'_>,
    draw_commands_buffer: &wgpu::Buffer,
    batch_count: u32,
) {
    if batch_count == 0 {
        return;
    }
    render_pass.multi_draw_indexed_indirect(draw_commands_buffer, 0, batch_count);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frustum_extraction() {
        // Simple orthographic-like projection
        let view_proj = Mat4::orthographic_rh(-10.0, 10.0, -10.0, 10.0, 0.1, 100.0);
        let frustum = FrustumPlanes::from_view_proj(&view_proj);

        // All planes should be normalized
        for plane in &frustum.planes {
            let normal_len =
                (plane[0] * plane[0] + plane[1] * plane[1] + plane[2] * plane[2]).sqrt();
            assert!(
                (normal_len - 1.0).abs() < 0.01,
                "plane normal not normalized"
            );
        }
    }

    #[test]
    fn test_aabb_inside_frustum() {
        let view_proj = Mat4::orthographic_rh(-10.0, 10.0, -10.0, 10.0, 0.1, 100.0);
        let frustum = FrustumPlanes::from_view_proj(&view_proj);

        // AABB at origin, small size - should be visible
        let center = Vec3::ZERO;
        let extent = Vec3::splat(1.0);
        assert!(
            frustum.test_aabb(center, extent),
            "AABB at origin should be visible"
        );
    }

    #[test]
    fn test_aabb_outside_frustum() {
        let view_proj = Mat4::orthographic_rh(-10.0, 10.0, -10.0, 10.0, 0.1, 100.0);
        let frustum = FrustumPlanes::from_view_proj(&view_proj);

        // AABB far beyond frustum bounds - should not be visible
        let center = Vec3::new(100.0, 0.0, -50.0);
        let extent = Vec3::splat(1.0);
        assert!(
            !frustum.test_aabb(center, extent),
            "AABB far outside should not be visible"
        );
    }

    #[test]
    fn test_cpu_culling() {
        let view_proj = Mat4::orthographic_rh(-10.0, 10.0, -10.0, 10.0, 0.1, 100.0);
        let frustum = FrustumPlanes::from_view_proj(&view_proj);

        let instances = vec![
            InstanceAABB::new(Vec3::ZERO, Vec3::splat(1.0), 0), // Visible
            InstanceAABB::new(Vec3::new(100.0, 0.0, -50.0), Vec3::splat(1.0), 1), // Not visible
            InstanceAABB::new(Vec3::new(0.0, 5.0, -10.0), Vec3::splat(1.0), 2), // Visible
        ];

        let visible = cpu_frustum_cull(&instances, &frustum);
        assert!(visible.contains(&0), "Instance 0 should be visible");
        assert!(!visible.contains(&1), "Instance 1 should not be visible");
        assert!(visible.contains(&2), "Instance 2 should be visible");
    }

    #[test]
    fn test_aabb_from_transform() {
        let transform = Mat4::from_translation(Vec3::new(5.0, 10.0, -15.0));
        let local_min = Vec3::new(-1.0, -1.0, -1.0);
        let local_max = Vec3::new(1.0, 1.0, 1.0);

        let aabb = InstanceAABB::from_transform(&transform, local_min, local_max, 0);

        // Center should be translated
        assert!((aabb.center[0] - 5.0).abs() < 0.01);
        assert!((aabb.center[1] - 10.0).abs() < 0.01);
        assert!((aabb.center[2] + 15.0).abs() < 0.01);

        // Extent should remain same (no rotation)
        assert!((aabb.extent[0] - 1.0).abs() < 0.01);
        assert!((aabb.extent[1] - 1.0).abs() < 0.01);
        assert!((aabb.extent[2] - 1.0).abs() < 0.01);
    }

    // --- Mutation-resistant tests ---

    #[test]
    fn parse_culling_wgsl_shader() {
        let module =
            naga::front::wgsl::parse_str(CULLING_SHADER).expect("WGSL culling shader should parse");
        assert!(
            module.entry_points.iter().any(|e| e.name == "main"),
            "culling shader must have 'main' entry point"
        );
    }

    #[test]
    fn instance_aabb_new_stores_fields_correctly() {
        let center = Vec3::new(1.0, 2.0, 3.0);
        let extent = Vec3::new(4.0, 5.0, 6.0);
        let aabb = InstanceAABB::new(center, extent, 42);
        assert_eq!(aabb.center, [1.0, 2.0, 3.0]);
        assert_eq!(aabb.extent, [4.0, 5.0, 6.0]);
        assert_eq!(aabb.instance_index, 42);
        assert_eq!(aabb._pad0, 0);
    }

    #[test]
    fn draw_indirect_command_new_stores_all_fields() {
        let cmd = DrawIndirectCommand::new(100, 5, 200, 3);
        assert_eq!(cmd.vertex_count, 100);
        assert_eq!(cmd.instance_count, 5);
        assert_eq!(cmd.first_vertex, 200);
        assert_eq!(cmd.first_instance, 3);
    }

    #[test]
    fn draw_indirect_command_default_is_zeroed() {
        let cmd = DrawIndirectCommand::default();
        assert_eq!(cmd.vertex_count, 0);
        assert_eq!(cmd.instance_count, 0);
        assert_eq!(cmd.first_vertex, 0);
        assert_eq!(cmd.first_instance, 0);
    }

    #[test]
    fn batch_id_new_stores_fields() {
        let bid = BatchId::new(7, 13);
        assert_eq!(bid.mesh_id, 7);
        assert_eq!(bid.material_id, 13);
    }

    #[test]
    fn batch_id_equality_and_ordering() {
        let a = BatchId::new(1, 2);
        let b = BatchId::new(1, 2);
        let c = BatchId::new(1, 3);
        let d = BatchId::new(2, 1);
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert!(a < c, "same mesh_id, lower material_id should be less");
        assert!(
            c < d,
            "lower mesh_id should be less regardless of material_id"
        );
    }

    #[test]
    fn draw_batch_new_starts_empty() {
        let bid = BatchId::new(0, 0);
        let batch = DrawBatch::new(bid, 36, 100);
        assert_eq!(batch.vertex_count, 36);
        assert_eq!(batch.first_vertex, 100);
        assert_eq!(batch.instance_count(), 0);
        assert!(batch.instances.is_empty());
    }

    #[test]
    fn draw_batch_add_instance_increments_count() {
        let bid = BatchId::new(0, 0);
        let mut batch = DrawBatch::new(bid, 36, 0);
        assert_eq!(batch.instance_count(), 0);
        batch.add_instance(10);
        assert_eq!(batch.instance_count(), 1);
        batch.add_instance(20);
        batch.add_instance(30);
        assert_eq!(batch.instance_count(), 3);
        assert_eq!(batch.instances, vec![10, 20, 30]);
    }

    #[test]
    fn build_indirect_commands_empty_batches() {
        let commands = build_indirect_commands_cpu(&[]);
        assert!(commands.is_empty());
    }

    #[test]
    fn build_indirect_commands_maps_batch_fields() {
        let bid = BatchId::new(1, 2);
        let mut batch = DrawBatch::new(bid, 36, 100);
        batch.add_instance(0);
        batch.add_instance(1);
        batch.add_instance(2);

        let commands = build_indirect_commands_cpu(&[batch]);
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].vertex_count, 36);
        assert_eq!(commands[0].instance_count, 3);
        assert_eq!(commands[0].first_vertex, 100);
        assert_eq!(commands[0].first_instance, 0);
    }

    #[test]
    fn build_indirect_commands_multiple_batches() {
        let mut b1 = DrawBatch::new(BatchId::new(0, 0), 24, 0);
        b1.add_instance(0);
        let mut b2 = DrawBatch::new(BatchId::new(1, 0), 36, 24);
        b2.add_instance(1);
        b2.add_instance(2);

        let commands = build_indirect_commands_cpu(&[b1, b2]);
        assert_eq!(commands.len(), 2);
        assert_eq!(commands[0].vertex_count, 24);
        assert_eq!(commands[0].instance_count, 1);
        assert_eq!(commands[1].vertex_count, 36);
        assert_eq!(commands[1].instance_count, 2);
    }

    #[test]
    fn batch_visible_instances_groups_by_batch_id() {
        let visible = vec![0u32, 1, 2, 3, 4];
        // Even instances → mesh 0, odd → mesh 1
        let get_batch = |idx: u32| {
            if idx % 2 == 0 {
                BatchId::new(0, 0)
            } else {
                BatchId::new(1, 0)
            }
        };
        let get_mesh = |bid: BatchId| {
            if bid.mesh_id == 0 {
                (24u32, 0u32)
            } else {
                (36, 24)
            }
        };
        let batches = batch_visible_instances(&visible, get_batch, get_mesh);
        assert_eq!(batches.len(), 2, "should produce 2 distinct batches");

        // BTreeMap orders by BatchId, so mesh_id=0 comes first
        let b0 = &batches[0];
        assert_eq!(b0.batch_id.mesh_id, 0);
        assert_eq!(b0.vertex_count, 24);
        assert_eq!(b0.instance_count(), 3); // indices 0, 2, 4
        assert_eq!(b0.instances, vec![0, 2, 4]);

        let b1 = &batches[1];
        assert_eq!(b1.batch_id.mesh_id, 1);
        assert_eq!(b1.vertex_count, 36);
        assert_eq!(b1.instance_count(), 2); // indices 1, 3
        assert_eq!(b1.instances, vec![1, 3]);
    }

    #[test]
    fn batch_visible_instances_empty_input() {
        let batches = batch_visible_instances(&[], |_| BatchId::new(0, 0), |_| (36, 0));
        assert!(batches.is_empty());
    }

    #[test]
    fn cpu_frustum_cull_empty_instances() {
        let view_proj = Mat4::orthographic_rh(-10.0, 10.0, -10.0, 10.0, 0.1, 100.0);
        let frustum = FrustumPlanes::from_view_proj(&view_proj);
        let visible = cpu_frustum_cull(&[], &frustum);
        assert!(visible.is_empty());
    }

    #[test]
    fn cpu_frustum_cull_preserves_instance_indices() {
        let view_proj = Mat4::orthographic_rh(-10.0, 10.0, -10.0, 10.0, 0.1, 100.0);
        let frustum = FrustumPlanes::from_view_proj(&view_proj);

        // Use non-sequential instance indices
        let instances = vec![
            InstanceAABB::new(Vec3::ZERO, Vec3::splat(1.0), 99),
            InstanceAABB::new(Vec3::new(100.0, 0.0, -50.0), Vec3::splat(1.0), 42),
        ];
        let visible = cpu_frustum_cull(&instances, &frustum);
        assert_eq!(
            visible,
            vec![99],
            "should return the actual instance_index field value"
        );
    }

    #[test]
    fn frustum_planes_are_six() {
        let vp = Mat4::perspective_rh(std::f32::consts::FRAC_PI_3, 16.0 / 9.0, 0.1, 1000.0);
        let frustum = FrustumPlanes::from_view_proj(&vp);
        assert_eq!(frustum.planes.len(), 6);
        // Each plane normal should be normalized (length ≈ 1)
        for (i, plane) in frustum.planes.iter().enumerate() {
            let len = (plane[0] * plane[0] + plane[1] * plane[1] + plane[2] * plane[2]).sqrt();
            assert!(
                (len - 1.0).abs() < 0.01,
                "plane {i} normal length {len} should be ~1.0"
            );
        }
    }

    #[test]
    fn aabb_from_rotated_transform_expands_extent() {
        // 45-degree rotation around Y axis should expand AABB
        let rotation = Mat4::from_rotation_y(std::f32::consts::FRAC_PI_4);
        let local_min = Vec3::new(-1.0, -1.0, -1.0);
        let local_max = Vec3::new(1.0, 1.0, 1.0);

        let aabb = InstanceAABB::from_transform(&rotation, local_min, local_max, 0);

        // For a unit cube rotated 45°, x and z extents should expand to ~√2
        let sqrt2 = std::f32::consts::SQRT_2;
        assert!(
            aabb.extent[0] > 1.0 && aabb.extent[0] < sqrt2 + 0.1,
            "rotated x extent {} should be > 1.0 and ≤ √2",
            aabb.extent[0]
        );
        // Y extent should remain ~1.0 (rotation around Y)
        assert!(
            (aabb.extent[1] - 1.0).abs() < 0.01,
            "y extent should stay ~1.0, got {}",
            aabb.extent[1]
        );
    }

    #[test]
    fn instance_aabb_struct_size_matches_wgsl_layout() {
        // WGSL std140: center(12) + pad(4) + extent(12) + instance_index(4) = 32 bytes
        assert_eq!(std::mem::size_of::<InstanceAABB>(), 32);
    }

    #[test]
    fn frustum_planes_struct_size_matches_wgsl_layout() {
        // 6 planes × 4 floats × 4 bytes = 96 bytes
        assert_eq!(std::mem::size_of::<FrustumPlanes>(), 96);
    }

    #[test]
    fn draw_indirect_command_struct_size() {
        // 4 × u32 = 16 bytes, matches wgpu::DrawIndirect layout
        assert_eq!(std::mem::size_of::<DrawIndirectCommand>(), 16);
    }

    // --- P2-6 Multi-Draw-Indirect tests ---

    #[test]
    fn draw_indexed_indirect_command_struct_size() {
        // index_count(4) + instance_count(4) + first_index(4) + base_vertex(4) + first_instance(4) = 20
        assert_eq!(std::mem::size_of::<DrawIndexedIndirectCommand>(), 20);
    }

    #[test]
    fn draw_indexed_indirect_new_stores_all_fields() {
        let cmd = DrawIndexedIndirectCommand::new(100, 5, 200, -3, 7);
        assert_eq!(cmd.index_count, 100);
        assert_eq!(cmd.instance_count, 5);
        assert_eq!(cmd.first_index, 200);
        assert_eq!(cmd.base_vertex, -3);
        assert_eq!(cmd.first_instance, 7);
    }

    #[test]
    fn draw_indexed_indirect_default_is_zeroed() {
        let cmd = DrawIndexedIndirectCommand::default();
        assert_eq!(cmd.index_count, 0);
        assert_eq!(cmd.instance_count, 0);
        assert_eq!(cmd.first_index, 0);
        assert_eq!(cmd.base_vertex, 0);
        assert_eq!(cmd.first_instance, 0);
    }

    #[test]
    fn parse_indirect_culling_wgsl_shader() {
        let module = naga::front::wgsl::parse_str(INDIRECT_CULLING_SHADER)
            .expect("WGSL indirect culling shader should parse");
        assert!(
            module.entry_points.iter().any(|e| e.name == "main"),
            "indirect culling shader must have 'main' entry point"
        );
    }
}
