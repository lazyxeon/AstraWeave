//! Benchmarks for Render Graph, Mesh Operations, Material System, and Texture Operations
//!
//! This benchmark suite covers core rendering infrastructure:
//! - Render graph node execution and resource management
//! - Mesh vertex/index operations and AABB computation
//! - Material GPU data packing and parameter management
//! - Texture usage classification and mipmap calculations
//!
//! v5.33: ~65 benchmarks covering fundamental rendering pipeline infrastructure

use criterion::{
    criterion_group, criterion_main, BenchmarkGroup, BenchmarkId, Criterion, SamplingMode,
    Throughput,
};
use glam::{Vec2, Vec3, Vec4};
use std::collections::{BTreeMap, HashMap};
use std::hint::black_box;
use std::time::Duration;

// Run 39100674332 shard 13 killed this target at 600s. Default sample_size
// 100 is 5_050 linear iterations plus 3s warm-up and 5s measurement per
// function. Flat sampling still runs every function. The 600s cap stays.
fn ci_budget<'a>(
    c: &'a mut Criterion,
    name: &str,
) -> BenchmarkGroup<'a, criterion::measurement::WallTime> {
    let mut group = c.benchmark_group(name);
    group.sampling_mode(SamplingMode::Flat);
    group.sample_size(10);
    group.warm_up_time(Duration::from_millis(1));
    group.measurement_time(Duration::from_secs(1));
    group
}

// ============================================================================
// MOCK IMPLEMENTATIONS (CPU-side benchmarks without GPU dependencies)
// ============================================================================

/// Mock render graph resource types
#[derive(Clone)]
pub enum MockResource {
    Texture {
        width: u32,
        height: u32,
        format: u32,
    },
    View {
        texture_id: u32,
    },
    Buffer {
        size: u64,
    },
    BindGroup {
        bindings: u32,
    },
}

/// Mock resource table for graph nodes
#[derive(Default, Clone)]
pub struct MockResourceTable {
    map: BTreeMap<String, MockResource>,
}

impl MockResourceTable {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, key: impl Into<String>, resource: MockResource) {
        self.map.insert(key.into(), resource);
    }

    pub fn get(&self, key: &str) -> Option<&MockResource> {
        self.map.get(key)
    }

    pub fn remove(&mut self, key: &str) -> Option<MockResource> {
        self.map.remove(key)
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn clear(&mut self) {
        self.map.clear();
    }

    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.map.keys()
    }
}

/// Mock graph context
pub struct MockGraphContext {
    pub resources: MockResourceTable,
    pub frame_index: u64,
    pub delta_time: f32,
}

impl MockGraphContext {
    pub fn new(frame_index: u64) -> Self {
        Self {
            resources: MockResourceTable::new(),
            frame_index,
            delta_time: 1.0 / 60.0,
        }
    }
}

/// Mock render node trait
pub trait MockRenderNode {
    fn name(&self) -> &str;
    fn execute(&mut self, ctx: &mut MockGraphContext) -> Result<(), &'static str>;
}

/// Clear node implementation
pub struct MockClearNode {
    name: String,
    target_key: String,
    color: [f32; 4],
}

impl MockClearNode {
    pub fn new(name: impl Into<String>, target_key: impl Into<String>, color: [f32; 4]) -> Self {
        Self {
            name: name.into(),
            target_key: target_key.into(),
            color,
        }
    }
}

impl MockRenderNode for MockClearNode {
    fn name(&self) -> &str {
        &self.name
    }

    fn execute(&mut self, ctx: &mut MockGraphContext) -> Result<(), &'static str> {
        // Simulate clear operation - verify target exists
        let _ = ctx
            .resources
            .get(&self.target_key)
            .ok_or("target not found")?;
        black_box(&self.color);
        Ok(())
    }
}

/// Main scene render node
pub struct MockSceneNode {
    name: String,
    target_key: String,
    depth_key: String,
}

impl MockSceneNode {
    pub fn new(
        name: impl Into<String>,
        target_key: impl Into<String>,
        depth_key: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            target_key: target_key.into(),
            depth_key: depth_key.into(),
        }
    }
}

impl MockRenderNode for MockSceneNode {
    fn name(&self) -> &str {
        &self.name
    }

    fn execute(&mut self, ctx: &mut MockGraphContext) -> Result<(), &'static str> {
        let _ = ctx
            .resources
            .get(&self.target_key)
            .ok_or("target not found")?;
        let _ = ctx
            .resources
            .get(&self.depth_key)
            .ok_or("depth not found")?;
        Ok(())
    }
}

/// Post-process node
pub struct MockPostProcessNode {
    name: String,
    input_key: String,
    output_key: String,
    params: [f32; 8],
}

impl MockPostProcessNode {
    pub fn new(
        name: impl Into<String>,
        input_key: impl Into<String>,
        output_key: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            input_key: input_key.into(),
            output_key: output_key.into(),
            params: [1.0, 1.0, 0.0, 0.0, 1.0, 0.5, 0.0, 1.0],
        }
    }
}

impl MockRenderNode for MockPostProcessNode {
    fn name(&self) -> &str {
        &self.name
    }

    fn execute(&mut self, ctx: &mut MockGraphContext) -> Result<(), &'static str> {
        let _ = ctx
            .resources
            .get(&self.input_key)
            .ok_or("input not found")?;
        black_box(&self.params);
        // Create output resource
        ctx.resources.insert(
            self.output_key.clone(),
            MockResource::View {
                texture_id: ctx.frame_index as u32,
            },
        );
        Ok(())
    }
}

/// Linear render graph
pub struct MockRenderGraph {
    nodes: Vec<Box<dyn MockRenderNode + Send + Sync>>,
}

impl MockRenderGraph {
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    pub fn add_node<N: MockRenderNode + Send + Sync + 'static>(&mut self, node: N) {
        self.nodes.push(Box::new(node));
    }

    pub fn execute(&mut self, ctx: &mut MockGraphContext) -> Result<(), &'static str> {
        for node in &mut self.nodes {
            node.execute(ctx)?;
        }
        Ok(())
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
}

impl Default for MockRenderGraph {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// MESH VERTEX AND OPERATIONS
// ============================================================================

/// Mesh vertex with position, normal, tangent, UV
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct MeshVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tangent: [f32; 4],
    pub uv: [f32; 2],
}

impl MeshVertex {
    pub fn new(position: Vec3, normal: Vec3, tangent: Vec4, uv: Vec2) -> Self {
        Self {
            position: position.to_array(),
            normal: normal.to_array(),
            tangent: tangent.to_array(),
            uv: uv.to_array(),
        }
    }

    pub fn from_arrays(
        position: [f32; 3],
        normal: [f32; 3],
        tangent: [f32; 4],
        uv: [f32; 2],
    ) -> Self {
        Self {
            position,
            normal,
            tangent,
            uv,
        }
    }

    pub const STRIDE: usize = 48; // 12 floats * 4 bytes
}

/// CPU mesh representation
#[derive(Clone, Debug, Default)]
pub struct CpuMesh {
    pub vertices: Vec<MeshVertex>,
    pub indices: Vec<u32>,
}

impl CpuMesh {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(vertex_count: usize, index_count: usize) -> Self {
        Self {
            vertices: Vec::with_capacity(vertex_count),
            indices: Vec::with_capacity(index_count),
        }
    }

    pub fn aabb(&self) -> Option<(Vec3, Vec3)> {
        if self.vertices.is_empty() {
            return None;
        }
        let mut min = Vec3::splat(f32::INFINITY);
        let mut max = Vec3::splat(f32::NEG_INFINITY);
        for v in &self.vertices {
            let p = Vec3::from_array(v.position);
            min = min.min(p);
            max = max.max(p);
        }
        Some((min, max))
    }

    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    pub fn memory_size(&self) -> usize {
        self.vertices.len() * MeshVertex::STRIDE + self.indices.len() * 4
    }
}

/// Compute tangents for a mesh (MikkTSpace-like approximation)
pub fn compute_tangents(mesh: &mut CpuMesh) {
    if mesh.indices.len() % 3 != 0 || mesh.vertices.is_empty() {
        return;
    }

    let v = &mut mesh.vertices;
    let idx = &mesh.indices;
    let mut tan1: Vec<Vec3> = vec![Vec3::ZERO; v.len()];
    let mut tan2: Vec<Vec3> = vec![Vec3::ZERO; v.len()];

    for tri in idx.chunks_exact(3) {
        let (i0, i1, i2) = (tri[0] as usize, tri[1] as usize, tri[2] as usize);
        let p0 = Vec3::from_array(v[i0].position);
        let p1 = Vec3::from_array(v[i1].position);
        let p2 = Vec3::from_array(v[i2].position);
        let uv0 = Vec2::from_array(v[i0].uv);
        let uv1 = Vec2::from_array(v[i1].uv);
        let uv2 = Vec2::from_array(v[i2].uv);

        let dp1 = p1 - p0;
        let dp2 = p2 - p0;
        let duv1 = uv1 - uv0;
        let duv2 = uv2 - uv0;

        let r = 1.0 / (duv1.x * duv2.y - duv1.y * duv2.x).max(1e-8);
        let sdir = (dp1 * duv2.y - dp2 * duv1.y) * r;
        let tdir = (dp2 * duv1.x - dp1 * duv2.x) * r;

        tan1[i0] += sdir;
        tan1[i1] += sdir;
        tan1[i2] += sdir;
        tan2[i0] += tdir;
        tan2[i1] += tdir;
        tan2[i2] += tdir;
    }

    for i in 0..v.len() {
        let n = Vec3::from_array(v[i].normal).normalize_or_zero();
        let t = tan1[i];
        let tangent = (t - n * n.dot(t)).normalize_or_zero();
        let w = if n.cross(t).dot(tan2[i]) < 0.0 {
            -1.0
        } else {
            1.0
        };
        v[i].tangent = [tangent.x, tangent.y, tangent.z, w];
    }
}

/// Generate a simple quad mesh
pub fn generate_quad() -> CpuMesh {
    let mut mesh = CpuMesh::with_capacity(4, 6);
    mesh.vertices = vec![
        MeshVertex::from_arrays(
            [-0.5, 0.0, -0.5],
            [0.0, 1.0, 0.0],
            [1.0, 0.0, 0.0, 1.0],
            [0.0, 0.0],
        ),
        MeshVertex::from_arrays(
            [0.5, 0.0, -0.5],
            [0.0, 1.0, 0.0],
            [1.0, 0.0, 0.0, 1.0],
            [1.0, 0.0],
        ),
        MeshVertex::from_arrays(
            [0.5, 0.0, 0.5],
            [0.0, 1.0, 0.0],
            [1.0, 0.0, 0.0, 1.0],
            [1.0, 1.0],
        ),
        MeshVertex::from_arrays(
            [-0.5, 0.0, 0.5],
            [0.0, 1.0, 0.0],
            [1.0, 0.0, 0.0, 1.0],
            [0.0, 1.0],
        ),
    ];
    mesh.indices = vec![0, 1, 2, 0, 2, 3];
    mesh
}

/// Generate a grid mesh
pub fn generate_grid(size: u32) -> CpuMesh {
    let vertex_count = ((size + 1) * (size + 1)) as usize;
    let index_count = (size * size * 6) as usize;
    let mut mesh = CpuMesh::with_capacity(vertex_count, index_count);

    let step = 1.0 / size as f32;
    for z in 0..=size {
        for x in 0..=size {
            let px = x as f32 * step - 0.5;
            let pz = z as f32 * step - 0.5;
            let u = x as f32 / size as f32;
            let v = z as f32 / size as f32;
            mesh.vertices.push(MeshVertex::from_arrays(
                [px, 0.0, pz],
                [0.0, 1.0, 0.0],
                [1.0, 0.0, 0.0, 1.0],
                [u, v],
            ));
        }
    }

    for z in 0..size {
        for x in 0..size {
            let i = z * (size + 1) + x;
            mesh.indices.extend_from_slice(&[
                i,
                i + 1,
                i + size + 2,
                i,
                i + size + 2,
                i + size + 1,
            ]);
        }
    }

    mesh
}

// ============================================================================
// MATERIAL SYSTEM
// ============================================================================

/// GPU material representation
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct MaterialGpu {
    pub texture_indices: [u32; 4],
    pub tiling_triplanar: [f32; 4],
    pub factors: [f32; 4],
    pub flags: u32,
    pub _padding: [u32; 3],
}

impl MaterialGpu {
    pub const FLAG_HAS_ALBEDO: u32 = 1 << 0;
    pub const FLAG_HAS_NORMAL: u32 = 1 << 1;
    pub const FLAG_HAS_ORM: u32 = 1 << 2;
    pub const FLAG_TRIPLANAR: u32 = 1 << 3;

    pub fn neutral(layer_idx: u32) -> Self {
        Self {
            texture_indices: [layer_idx, layer_idx, layer_idx, 0],
            tiling_triplanar: [1.0, 1.0, 16.0, 0.0],
            factors: [0.0, 0.5, 1.0, 1.0],
            flags: 0,
            _padding: [0; 3],
        }
    }

    pub fn with_albedo(mut self) -> Self {
        self.flags |= Self::FLAG_HAS_ALBEDO;
        self
    }

    pub fn with_normal(mut self) -> Self {
        self.flags |= Self::FLAG_HAS_NORMAL;
        self
    }

    pub fn with_orm(mut self) -> Self {
        self.flags |= Self::FLAG_HAS_ORM;
        self
    }

    pub fn with_triplanar(mut self) -> Self {
        self.flags |= Self::FLAG_TRIPLANAR;
        self
    }

    pub fn with_tiling(mut self, u: f32, v: f32) -> Self {
        self.tiling_triplanar[0] = u;
        self.tiling_triplanar[1] = v;
        self
    }

    pub fn with_metallic(mut self, metallic: f32) -> Self {
        self.factors[0] = metallic;
        self
    }

    pub fn with_roughness(mut self, roughness: f32) -> Self {
        self.factors[1] = roughness;
        self
    }

    pub const SIZE: usize = 64; // 16 u32s * 4 bytes
}

/// Material layer description
#[derive(Clone, Debug)]
pub struct MaterialLayerDesc {
    pub key: String,
    pub albedo: Option<String>,
    pub normal: Option<String>,
    pub mra: Option<String>,
    pub tiling: [f32; 2],
    pub triplanar_scale: f32,
}

impl Default for MaterialLayerDesc {
    fn default() -> Self {
        Self {
            key: String::new(),
            albedo: None,
            normal: None,
            mra: None,
            tiling: [1.0, 1.0],
            triplanar_scale: 16.0,
        }
    }
}

impl MaterialLayerDesc {
    pub fn new(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            ..Default::default()
        }
    }

    pub fn to_gpu(&self, layer_idx: u32) -> MaterialGpu {
        let mut mat = MaterialGpu::neutral(layer_idx).with_tiling(self.tiling[0], self.tiling[1]);

        if self.albedo.is_some() {
            mat = mat.with_albedo();
        }
        if self.normal.is_some() {
            mat = mat.with_normal();
        }
        if self.mra.is_some() {
            mat = mat.with_orm();
        }

        mat.tiling_triplanar[2] = self.triplanar_scale;
        mat
    }
}

/// Material array layout
#[derive(Clone, Debug, Default)]
pub struct ArrayLayout {
    pub layer_indices: HashMap<String, u32>,
    pub count: u32,
}

impl ArrayLayout {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_layer(&mut self, key: impl Into<String>) -> u32 {
        let idx = self.count;
        self.layer_indices.insert(key.into(), idx);
        self.count += 1;
        idx
    }

    pub fn get_index(&self, key: &str) -> Option<u32> {
        self.layer_indices.get(key).copied()
    }
}

/// Material load statistics
#[derive(Clone, Debug, Default)]
pub struct MaterialLoadStats {
    pub biome: String,
    pub layers_total: usize,
    pub albedo_loaded: usize,
    pub albedo_substituted: usize,
    pub normal_loaded: usize,
    pub normal_substituted: usize,
    pub mra_loaded: usize,
    pub mra_packed: usize,
    pub mra_substituted: usize,
    pub gpu_memory_bytes: u64,
}

impl MaterialLoadStats {
    pub fn concise_summary(&self) -> String {
        format!(
            "[materials] biome={} layers={} | albedo L/S={}/{} | normal L/S={}/{} | mra L+P/S={}+{}/{} | gpu={:.2} MiB",
            self.biome,
            self.layers_total,
            self.albedo_loaded,
            self.albedo_substituted,
            self.normal_loaded,
            self.normal_substituted,
            self.mra_loaded,
            self.mra_packed,
            self.mra_substituted,
            (self.gpu_memory_bytes as f64) / (1024.0 * 1024.0)
        )
    }
}

// ============================================================================
// TEXTURE OPERATIONS
// ============================================================================

/// Texture usage classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureUsage {
    Albedo,
    Normal,
    MRA,
    Emissive,
    Height,
}

impl TextureUsage {
    pub fn format_code(&self) -> u32 {
        match self {
            Self::Albedo | Self::Emissive => 1,           // sRGB
            Self::Normal | Self::MRA | Self::Height => 0, // Linear
        }
    }

    pub fn needs_mipmaps(&self) -> bool {
        match self {
            Self::Albedo | Self::Emissive | Self::MRA => true,
            Self::Normal | Self::Height => false,
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::Albedo => "Albedo (sRGB color)",
            Self::Normal => "Normal Map (linear RGB)",
            Self::MRA => "Metallic/Roughness/AO (linear)",
            Self::Emissive => "Emissive (sRGB color)",
            Self::Height => "Height/Displacement (linear)",
        }
    }
}

/// Calculate mip levels for a texture
pub fn calculate_mip_levels(width: u32, height: u32) -> u32 {
    let max_dim = width.max(height);
    (32 - max_dim.leading_zeros()).max(1)
}

/// Texture descriptor (CPU-side metadata)
#[derive(Clone, Debug)]
pub struct TextureDesc {
    pub width: u32,
    pub height: u32,
    pub usage: TextureUsage,
    pub mip_levels: u32,
    pub label: String,
}

impl TextureDesc {
    pub fn new(width: u32, height: u32, usage: TextureUsage) -> Self {
        let mip_levels = if usage.needs_mipmaps() {
            calculate_mip_levels(width, height)
        } else {
            1
        };
        Self {
            width,
            height,
            usage,
            mip_levels,
            label: String::new(),
        }
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    pub fn memory_size(&self) -> u64 {
        let base_size = (self.width as u64) * (self.height as u64) * 4;
        if self.mip_levels <= 1 {
            base_size
        } else {
            // Approximate: mips add ~1/3 more memory
            base_size + base_size / 3
        }
    }

    pub fn mip_size(&self, level: u32) -> (u32, u32) {
        let w = (self.width >> level).max(1);
        let h = (self.height >> level).max(1);
        (w, h)
    }
}

/// Texture atlas management
#[derive(Clone, Debug)]
pub struct TextureAtlas {
    pub width: u32,
    pub height: u32,
    pub layers: u32,
    pub slots: Vec<AtlasSlot>,
}

#[derive(Clone, Debug)]
pub struct AtlasSlot {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub layer: u32,
    pub occupied: bool,
}

impl TextureAtlas {
    pub fn new(width: u32, height: u32, layers: u32) -> Self {
        Self {
            width,
            height,
            layers,
            slots: Vec::new(),
        }
    }

    pub fn allocate(&mut self, width: u32, height: u32) -> Option<usize> {
        // Simple linear allocation for benchmarking
        let slot_idx = self.slots.len();
        let layer = (slot_idx as u32) % self.layers;
        self.slots.push(AtlasSlot {
            x: 0,
            y: 0,
            width,
            height,
            layer,
            occupied: true,
        });
        Some(slot_idx)
    }

    pub fn free(&mut self, slot_idx: usize) {
        if slot_idx < self.slots.len() {
            self.slots[slot_idx].occupied = false;
        }
    }

    pub fn occupied_count(&self) -> usize {
        self.slots.iter().filter(|s| s.occupied).count()
    }
}

// ============================================================================
// MESH REGISTRY
// ============================================================================

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct MeshKey(pub String);

#[derive(Clone, Debug, Copy, PartialEq, Eq, Hash)]
pub struct MeshHandle(pub u32);

pub struct MeshRegistry {
    next_id: u32,
    map: HashMap<MeshKey, MeshHandle>,
    meshes: HashMap<MeshHandle, CpuMesh>,
}

impl Default for MeshRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl MeshRegistry {
    pub fn new() -> Self {
        Self {
            next_id: 1,
            map: HashMap::new(),
            meshes: HashMap::new(),
        }
    }

    pub fn get(&self, key: &MeshKey) -> Option<MeshHandle> {
        self.map.get(key).copied()
    }

    pub fn register(&mut self, key: MeshKey, mesh: CpuMesh) -> MeshHandle {
        if let Some(h) = self.map.get(&key) {
            return *h;
        }
        let handle = MeshHandle(self.next_id);
        self.next_id += 1;
        self.map.insert(key, handle);
        self.meshes.insert(handle, mesh);
        handle
    }

    pub fn get_mesh(&self, handle: MeshHandle) -> Option<&CpuMesh> {
        self.meshes.get(&handle)
    }

    pub fn mesh_count(&self) -> usize {
        self.meshes.len()
    }

    pub fn total_vertices(&self) -> usize {
        self.meshes.values().map(|m| m.vertex_count()).sum()
    }

    pub fn total_memory(&self) -> usize {
        self.meshes.values().map(|m| m.memory_size()).sum()
    }
}

// ============================================================================
// BENCHMARKS: RENDER GRAPH
// ============================================================================

fn bench_render_graph(c: &mut Criterion) {
    let mut group = ci_budget(c, "render_graph");

    // Resource table operations
    group.bench_function("resource_table_insert", |b| {
        b.iter(|| {
            let mut table = MockResourceTable::new();
            table.insert("color_target", MockResource::View { texture_id: 0 });
            table.insert("depth_target", MockResource::View { texture_id: 1 });
            table.insert("gbuffer_albedo", MockResource::View { texture_id: 2 });
            table.insert("gbuffer_normal", MockResource::View { texture_id: 3 });
            black_box(table.len())
        })
    });

    group.bench_function("resource_table_lookup", |b| {
        let mut table = MockResourceTable::new();
        for i in 0..100 {
            table.insert(
                format!("resource_{}", i),
                MockResource::View { texture_id: i },
            );
        }
        b.iter(|| {
            let r1 = table.get("resource_0").is_some();
            let r2 = table.get("resource_50").is_some();
            let r3 = table.get("resource_99").is_some();
            let r4 = table.get("nonexistent").is_some();
            black_box((r1, r2, r3, r4))
        })
    });

    // Graph context creation
    group.bench_function("graph_context_new", |b| {
        b.iter(|| {
            let ctx = MockGraphContext::new(black_box(1234));
            black_box(ctx.frame_index)
        })
    });

    // Node creation
    group.bench_function("clear_node_new", |b| {
        b.iter(|| {
            let node = MockClearNode::new("clear", "color_target", [0.1, 0.2, 0.3, 1.0]);
            black_box(node)
        })
    });

    group.bench_function("scene_node_new", |b| {
        b.iter(|| {
            let node = MockSceneNode::new("scene", "color_target", "depth_target");
            black_box(node)
        })
    });

    group.bench_function("post_process_node_new", |b| {
        b.iter(|| {
            let node = MockPostProcessNode::new("bloom", "hdr_target", "bloom_output");
            black_box(node)
        })
    });

    // Graph execution
    for node_count in [3, 5, 10, 20] {
        group.bench_with_input(
            BenchmarkId::new("graph_execute", node_count),
            &node_count,
            |b, &count| {
                let mut graph = MockRenderGraph::new();
                for i in 0..count {
                    graph.add_node(MockClearNode::new(
                        format!("node_{}", i),
                        format!("target_{}", i),
                        [0.0, 0.0, 0.0, 1.0],
                    ));
                }
                let mut ctx = MockGraphContext::new(0);
                for i in 0..count {
                    ctx.resources.insert(
                        format!("target_{}", i),
                        MockResource::View {
                            texture_id: i as u32,
                        },
                    );
                }
                b.iter(|| {
                    let result = graph.execute(&mut ctx);
                    black_box(result.is_ok())
                })
            },
        );
    }

    // Full pipeline simulation
    group.bench_function("full_pipeline_3_passes", |b| {
        let mut graph = MockRenderGraph::new();
        graph.add_node(MockClearNode::new(
            "shadow_clear",
            "shadow_map",
            [1.0, 1.0, 1.0, 1.0],
        ));
        graph.add_node(MockSceneNode::new("main_scene", "hdr_target", "depth"));
        graph.add_node(MockPostProcessNode::new(
            "tonemap",
            "hdr_target",
            "ldr_output",
        ));

        let mut ctx = MockGraphContext::new(0);
        ctx.resources
            .insert("shadow_map", MockResource::View { texture_id: 0 });
        ctx.resources
            .insert("hdr_target", MockResource::View { texture_id: 1 });
        ctx.resources
            .insert("depth", MockResource::View { texture_id: 2 });

        b.iter(|| {
            let result = graph.execute(&mut ctx);
            black_box(result.is_ok())
        })
    });

    group.finish();
}

// ============================================================================
// BENCHMARKS: MESH OPERATIONS
// ============================================================================

fn bench_mesh_operations(c: &mut Criterion) {
    let mut group = ci_budget(c, "mesh_operations");

    // Vertex creation
    group.bench_function("vertex_new", |b| {
        b.iter(|| {
            let v = MeshVertex::new(
                Vec3::new(1.0, 2.0, 3.0),
                Vec3::new(0.0, 1.0, 0.0),
                Vec4::new(1.0, 0.0, 0.0, 1.0),
                Vec2::new(0.5, 0.5),
            );
            black_box(v)
        })
    });

    group.bench_function("vertex_from_arrays", |b| {
        b.iter(|| {
            let v = MeshVertex::from_arrays(
                [1.0, 2.0, 3.0],
                [0.0, 1.0, 0.0],
                [1.0, 0.0, 0.0, 1.0],
                [0.5, 0.5],
            );
            black_box(v)
        })
    });

    // Mesh generation
    group.bench_function("generate_quad", |b| {
        b.iter(|| {
            let mesh = generate_quad();
            black_box(mesh)
        })
    });

    for size in [8, 16, 32, 64, 128] {
        group.throughput(Throughput::Elements(((size + 1) * (size + 1)) as u64));
        group.bench_with_input(BenchmarkId::new("generate_grid", size), &size, |b, &s| {
            b.iter(|| {
                let mesh = generate_grid(s);
                black_box(mesh)
            })
        });
    }

    // AABB computation
    for vertex_count in [100, 1000, 10000] {
        group.throughput(Throughput::Elements(vertex_count as u64));
        group.bench_with_input(
            BenchmarkId::new("compute_aabb", vertex_count),
            &vertex_count,
            |b, &count| {
                let mut mesh = CpuMesh::with_capacity(count, 0);
                for i in 0..count {
                    let t = i as f32 / count as f32;
                    mesh.vertices.push(MeshVertex::from_arrays(
                        [t * 10.0, (t * std::f32::consts::PI).sin(), t * 5.0],
                        [0.0, 1.0, 0.0],
                        [1.0, 0.0, 0.0, 1.0],
                        [t, t],
                    ));
                }
                b.iter(|| {
                    let aabb = mesh.aabb();
                    black_box(aabb)
                })
            },
        );
    }

    // Tangent computation
    for triangle_count in [100, 500, 1000] {
        group.throughput(Throughput::Elements(triangle_count as u64));
        group.bench_with_input(
            BenchmarkId::new("compute_tangents", triangle_count),
            &triangle_count,
            |b, &tris| {
                b.iter(|| {
                    let mut mesh = CpuMesh::with_capacity(tris * 3, tris * 3);
                    for i in 0..tris {
                        let base = i as f32;
                        for j in 0..3 {
                            let angle = (j as f32) * 2.094;
                            mesh.vertices.push(MeshVertex::from_arrays(
                                [base + angle.cos(), 0.0, base + angle.sin()],
                                [0.0, 1.0, 0.0],
                                [0.0, 0.0, 0.0, 1.0],
                                [(j as f32) / 3.0, 0.5],
                            ));
                            mesh.indices.push((i * 3 + j) as u32);
                        }
                    }
                    compute_tangents(&mut mesh);
                    black_box(mesh.vertex_count())
                })
            },
        );
    }

    // Memory size calculation
    group.bench_function("mesh_memory_size", |b| {
        let mesh = generate_grid(64);
        b.iter(|| {
            let size = mesh.memory_size();
            black_box(size)
        })
    });

    group.finish();
}

// ============================================================================
// BENCHMARKS: MATERIAL SYSTEM
// ============================================================================

fn bench_material_system(c: &mut Criterion) {
    let mut group = ci_budget(c, "material_system");

    // MaterialGpu creation
    group.bench_function("material_gpu_neutral", |b| {
        b.iter(|| {
            let mat = MaterialGpu::neutral(black_box(0));
            black_box(mat)
        })
    });

    group.bench_function("material_gpu_full_config", |b| {
        b.iter(|| {
            let mat = MaterialGpu::neutral(0)
                .with_albedo()
                .with_normal()
                .with_orm()
                .with_tiling(2.0, 2.0)
                .with_metallic(0.8)
                .with_roughness(0.2);
            black_box(mat)
        })
    });

    // MaterialLayerDesc
    group.bench_function("material_layer_new", |b| {
        b.iter(|| {
            let layer = MaterialLayerDesc::new("grass_01");
            black_box(layer)
        })
    });

    group.bench_function("material_layer_to_gpu", |b| {
        let layer = MaterialLayerDesc {
            key: "rock_02".to_string(),
            albedo: Some("rock_albedo.png".to_string()),
            normal: Some("rock_normal.png".to_string()),
            mra: Some("rock_mra.png".to_string()),
            tiling: [4.0, 4.0],
            triplanar_scale: 8.0,
        };
        b.iter(|| {
            let gpu = layer.to_gpu(black_box(5));
            black_box(gpu)
        })
    });

    // Array layout
    group.bench_function("array_layout_add_10", |b| {
        b.iter(|| {
            let mut layout = ArrayLayout::new();
            for i in 0..10 {
                layout.add_layer(format!("layer_{}", i));
            }
            black_box(layout)
        })
    });

    for count in [10, 50, 100] {
        group.bench_with_input(
            BenchmarkId::new("array_layout_lookup", count),
            &count,
            |b, &c| {
                let mut layout = ArrayLayout::new();
                for i in 0..c {
                    layout.add_layer(format!("layer_{}", i));
                }
                b.iter(|| {
                    let idx1 = layout.get_index("layer_0");
                    let idx2 = layout.get_index(&format!("layer_{}", c / 2));
                    let idx3 = layout.get_index("nonexistent");
                    black_box((idx1, idx2, idx3))
                })
            },
        );
    }

    // Batch material conversion
    for count in [10, 50, 100, 500] {
        group.throughput(Throughput::Elements(count as u64));
        group.bench_with_input(BenchmarkId::new("batch_to_gpu", count), &count, |b, &c| {
            let layers: Vec<MaterialLayerDesc> = (0..c)
                .map(|i| MaterialLayerDesc {
                    key: format!("mat_{}", i),
                    albedo: Some(format!("albedo_{}.png", i)),
                    normal: if i % 2 == 0 {
                        Some(format!("normal_{}.png", i))
                    } else {
                        None
                    },
                    mra: if i % 3 == 0 {
                        Some(format!("mra_{}.png", i))
                    } else {
                        None
                    },
                    tiling: [1.0 + (i as f32) * 0.1, 1.0 + (i as f32) * 0.1],
                    triplanar_scale: 16.0,
                })
                .collect();
            b.iter(|| {
                let gpu_mats: Vec<MaterialGpu> = layers
                    .iter()
                    .enumerate()
                    .map(|(i, l)| l.to_gpu(i as u32))
                    .collect();
                black_box(gpu_mats)
            })
        });
    }

    // Load stats summary
    group.bench_function("load_stats_summary", |b| {
        let stats = MaterialLoadStats {
            biome: "forest".to_string(),
            layers_total: 24,
            albedo_loaded: 20,
            albedo_substituted: 4,
            normal_loaded: 18,
            normal_substituted: 6,
            mra_loaded: 15,
            mra_packed: 5,
            mra_substituted: 4,
            gpu_memory_bytes: 128 * 1024 * 1024,
        };
        b.iter(|| {
            let summary = stats.concise_summary();
            black_box(summary)
        })
    });

    group.finish();
}

// ============================================================================
// BENCHMARKS: TEXTURE OPERATIONS
// ============================================================================

fn bench_texture_operations(c: &mut Criterion) {
    let mut group = ci_budget(c, "texture_operations");

    // Texture usage classification
    group.bench_function("texture_usage_format", |b| {
        let usages = [
            TextureUsage::Albedo,
            TextureUsage::Normal,
            TextureUsage::MRA,
            TextureUsage::Emissive,
            TextureUsage::Height,
        ];
        b.iter(|| {
            let formats: Vec<u32> = usages.iter().map(|u| u.format_code()).collect();
            black_box(formats)
        })
    });

    group.bench_function("texture_usage_mipmaps", |b| {
        let usages = [
            TextureUsage::Albedo,
            TextureUsage::Normal,
            TextureUsage::MRA,
            TextureUsage::Emissive,
            TextureUsage::Height,
        ];
        b.iter(|| {
            let needs: Vec<bool> = usages.iter().map(|u| u.needs_mipmaps()).collect();
            black_box(needs)
        })
    });

    // Mip level calculation
    for (width, height) in [
        (256, 256),
        (512, 512),
        (1024, 1024),
        (2048, 2048),
        (4096, 4096),
    ] {
        group.bench_with_input(
            BenchmarkId::new("calculate_mip_levels", format!("{}x{}", width, height)),
            &(width, height),
            |b, &(w, h)| {
                b.iter(|| {
                    let levels = calculate_mip_levels(black_box(w), black_box(h));
                    black_box(levels)
                })
            },
        );
    }

    // Texture descriptor creation
    group.bench_function("texture_desc_new", |b| {
        b.iter(|| {
            let desc = TextureDesc::new(1024, 1024, TextureUsage::Albedo);
            black_box(desc)
        })
    });

    group.bench_function("texture_desc_full", |b| {
        b.iter(|| {
            let desc =
                TextureDesc::new(2048, 2048, TextureUsage::Normal).with_label("terrain_normal");
            black_box(desc)
        })
    });

    // Memory size calculation
    for (width, height) in [(256, 256), (1024, 1024), (4096, 4096)] {
        group.bench_with_input(
            BenchmarkId::new("texture_memory_size", format!("{}x{}", width, height)),
            &(width, height),
            |b, &(w, h)| {
                let desc = TextureDesc::new(w, h, TextureUsage::Albedo);
                b.iter(|| {
                    let size = desc.memory_size();
                    black_box(size)
                })
            },
        );
    }

    // Mip size calculation
    group.bench_function("mip_size_chain", |b| {
        let desc = TextureDesc::new(2048, 2048, TextureUsage::Albedo);
        b.iter(|| {
            let sizes: Vec<(u32, u32)> = (0..desc.mip_levels).map(|l| desc.mip_size(l)).collect();
            black_box(sizes)
        })
    });

    // Texture atlas operations
    group.bench_function("atlas_create", |b| {
        b.iter(|| {
            let atlas = TextureAtlas::new(4096, 4096, 8);
            black_box(atlas)
        })
    });

    for count in [10, 50, 100, 500] {
        group.throughput(Throughput::Elements(count as u64));
        group.bench_with_input(
            BenchmarkId::new("atlas_allocate", count),
            &count,
            |b, &c| {
                b.iter(|| {
                    let mut atlas = TextureAtlas::new(4096, 4096, 16);
                    for i in 0..c {
                        let size = 64 + (i % 4) * 64;
                        atlas.allocate(size as u32, size as u32);
                    }
                    black_box(atlas)
                })
            },
        );
    }

    group.bench_function("atlas_occupied_count", |b| {
        let mut atlas = TextureAtlas::new(4096, 4096, 16);
        for i in 0..200 {
            atlas.allocate(128, 128);
            if i % 3 == 0 {
                atlas.free(i / 3);
            }
        }
        b.iter(|| {
            let count = atlas.occupied_count();
            black_box(count)
        })
    });

    group.finish();
}

// ============================================================================
// BENCHMARKS: MESH REGISTRY
// ============================================================================

fn bench_mesh_registry(c: &mut Criterion) {
    let mut group = ci_budget(c, "mesh_registry");

    group.bench_function("registry_new", |b| {
        b.iter(|| {
            let reg = MeshRegistry::new();
            black_box(reg)
        })
    });

    for count in [10, 50, 100] {
        group.bench_with_input(
            BenchmarkId::new("registry_register", count),
            &count,
            |b, &c| {
                b.iter(|| {
                    let mut reg = MeshRegistry::new();
                    for i in 0..c {
                        let mesh = generate_quad();
                        reg.register(MeshKey(format!("mesh_{}", i)), mesh);
                    }
                    black_box(reg)
                })
            },
        );
    }

    group.bench_function("registry_lookup_existing", |b| {
        let mut reg = MeshRegistry::new();
        for i in 0..100 {
            let mesh = generate_quad();
            reg.register(MeshKey(format!("mesh_{}", i)), mesh);
        }
        b.iter(|| {
            let h1 = reg.get(&MeshKey("mesh_0".to_string()));
            let h2 = reg.get(&MeshKey("mesh_50".to_string()));
            let h3 = reg.get(&MeshKey("mesh_99".to_string()));
            black_box((h1, h2, h3))
        })
    });

    group.bench_function("registry_lookup_missing", |b| {
        let mut reg = MeshRegistry::new();
        for i in 0..100 {
            let mesh = generate_quad();
            reg.register(MeshKey(format!("mesh_{}", i)), mesh);
        }
        b.iter(|| {
            let h = reg.get(&MeshKey("nonexistent".to_string()));
            black_box(h)
        })
    });

    group.bench_function("registry_total_stats", |b| {
        let mut reg = MeshRegistry::new();
        for i in 0..50 {
            let size = 8 + (i % 5) * 8;
            let mesh = generate_grid(size);
            reg.register(MeshKey(format!("grid_{}", i)), mesh);
        }
        b.iter(|| {
            let count = reg.mesh_count();
            let verts = reg.total_vertices();
            let mem = reg.total_memory();
            black_box((count, verts, mem))
        })
    });

    group.finish();
}

// ============================================================================
// BENCHMARKS: COMBINED SCENARIOS
// ============================================================================

fn bench_combined_scenarios(c: &mut Criterion) {
    let mut group = ci_budget(c, "combined_scenarios");

    // Typical frame setup
    group.bench_function("typical_frame_setup", |b| {
        b.iter(|| {
            // Create graph
            let mut graph = MockRenderGraph::new();
            graph.add_node(MockClearNode::new("shadow", "shadow_map", [1.0; 4]));
            graph.add_node(MockSceneNode::new("main", "hdr", "depth"));
            graph.add_node(MockPostProcessNode::new("bloom", "hdr", "bloom_out"));
            graph.add_node(MockPostProcessNode::new("tonemap", "bloom_out", "ldr"));

            // Setup resources
            let mut ctx = MockGraphContext::new(0);
            ctx.resources
                .insert("shadow_map", MockResource::View { texture_id: 0 });
            ctx.resources
                .insert("hdr", MockResource::View { texture_id: 1 });
            ctx.resources
                .insert("depth", MockResource::View { texture_id: 2 });

            black_box((graph, ctx))
        })
    });

    // Material batch loading simulation
    group.bench_function("material_batch_load_24", |b| {
        b.iter(|| {
            let mut layout = ArrayLayout::new();
            let mut gpu_mats = Vec::with_capacity(24);

            for i in 0..24 {
                let layer = MaterialLayerDesc {
                    key: format!("terrain_{}", i),
                    albedo: Some(format!("t_{}_a.png", i)),
                    normal: Some(format!("t_{}_n.png", i)),
                    mra: Some(format!("t_{}_m.png", i)),
                    tiling: [4.0, 4.0],
                    triplanar_scale: 16.0,
                };
                let idx = layout.add_layer(&layer.key);
                gpu_mats.push(layer.to_gpu(idx));
            }

            black_box((layout, gpu_mats))
        })
    });

    // Mesh generation batch
    group.bench_function("mesh_batch_generate_lod_chain", |b| {
        b.iter(|| {
            let lod0 = generate_grid(128);
            let lod1 = generate_grid(64);
            let lod2 = generate_grid(32);
            let lod3 = generate_grid(16);
            black_box((lod0, lod1, lod2, lod3))
        })
    });

    // Texture atlas population
    group.bench_function("atlas_populate_terrain_set", |b| {
        b.iter(|| {
            let mut atlas = TextureAtlas::new(4096, 4096, 8);
            let mut descs = Vec::with_capacity(24);

            for _ in 0..24 {
                atlas.allocate(512, 512);
                descs.push(TextureDesc::new(512, 512, TextureUsage::Albedo));
            }

            let total_memory: u64 = descs.iter().map(|d| d.memory_size()).sum();
            black_box((atlas, total_memory))
        })
    });

    // Full pipeline initialization
    group.bench_function("full_pipeline_init", |b| {
        b.iter(|| {
            // Render graph
            let mut graph = MockRenderGraph::new();
            graph.add_node(MockClearNode::new("shadow_clear", "shadow_map", [1.0; 4]));
            graph.add_node(MockSceneNode::new(
                "gbuffer",
                "gbuffer_color",
                "gbuffer_depth",
            ));
            graph.add_node(MockPostProcessNode::new(
                "ssao",
                "gbuffer_depth",
                "ssao_out",
            ));
            graph.add_node(MockPostProcessNode::new("lighting", "gbuffer_color", "hdr"));
            graph.add_node(MockPostProcessNode::new("bloom", "hdr", "bloom"));
            graph.add_node(MockPostProcessNode::new("tonemap", "bloom", "ldr"));

            // Mesh registry
            let mut mesh_reg = MeshRegistry::new();
            mesh_reg.register(MeshKey("quad".to_string()), generate_quad());
            mesh_reg.register(MeshKey("terrain".to_string()), generate_grid(64));

            // Material layout
            let mut mat_layout = ArrayLayout::new();
            for i in 0..8 {
                mat_layout.add_layer(format!("material_{}", i));
            }

            // Texture atlas
            let mut atlas = TextureAtlas::new(4096, 4096, 8);
            for _ in 0..8 {
                atlas.allocate(512, 512);
            }

            black_box((graph, mesh_reg, mat_layout, atlas))
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_render_graph,
    bench_mesh_operations,
    bench_material_system,
    bench_texture_operations,
    bench_mesh_registry,
    bench_combined_scenarios,
);
criterion_main!(benches);
