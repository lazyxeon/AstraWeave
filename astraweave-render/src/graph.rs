//! DAG-based render graph with automatic resource lifetime management.
//!
//! Nodes declare their inputs and outputs as named resource slots. The graph compiler
//! performs topological sorting, validates connectivity, and enables resource aliasing
//! (reusing GPU memory for non-overlapping transient textures).
//!
//! Backward-compatible: the original `RenderNode` trait and linear `add_node()` path
//! continue to work. New code should prefer `add_pass()` with explicit I/O declarations.

use anyhow::Context as _;
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

// ---------------------------------------------------------------------------
// Resource types
// ---------------------------------------------------------------------------

/// Typed GPU resources passed between graph nodes.
#[non_exhaustive]
pub enum Resource {
    Texture(wgpu::Texture),
    View(wgpu::TextureView),
    Buffer(wgpu::Buffer),
    BindGroup(wgpu::BindGroup),
}

/// Descriptor for a transient texture that the graph should create automatically.
#[derive(Debug, Clone)]
pub struct TransientTextureDesc {
    pub label: String,
    pub width: u32,
    pub height: u32,
    pub format: wgpu::TextureFormat,
    pub usage: wgpu::TextureUsages,
    pub mip_level_count: u32,
    pub depth_or_array_layers: u32,
}

impl TransientTextureDesc {
    pub fn color(label: impl Into<String>, width: u32, height: u32) -> Self {
        Self {
            label: label.into(),
            width,
            height,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            mip_level_count: 1,
            depth_or_array_layers: 1,
        }
    }

    pub fn depth(label: impl Into<String>, width: u32, height: u32) -> Self {
        Self {
            label: label.into(),
            width,
            height,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            mip_level_count: 1,
            depth_or_array_layers: 1,
        }
    }

    pub fn with_format(mut self, format: wgpu::TextureFormat) -> Self {
        self.format = format;
        self
    }

    pub fn with_usage(mut self, usage: wgpu::TextureUsages) -> Self {
        self.usage = usage;
        self
    }

    pub fn with_mips(mut self, mip_level_count: u32) -> Self {
        self.mip_level_count = mip_level_count;
        self
    }

    pub fn with_layers(mut self, layers: u32) -> Self {
        self.depth_or_array_layers = layers;
        self
    }

    fn to_wgpu_desc(&self) -> wgpu::TextureDescriptor<'_> {
        wgpu::TextureDescriptor {
            label: Some(&self.label),
            size: wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: self.depth_or_array_layers,
            },
            mip_level_count: self.mip_level_count,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.format,
            usage: self.usage,
            view_formats: &[],
        }
    }

    /// Check if two descriptors are compatible for memory aliasing.
    /// Compatible means same size, format, and usage flags.
    fn alias_compatible(&self, other: &Self) -> bool {
        self.width == other.width
            && self.height == other.height
            && self.format == other.format
            && self.usage == other.usage
            && self.mip_level_count == other.mip_level_count
            && self.depth_or_array_layers == other.depth_or_array_layers
    }
}

/// Declares what a node reads from or writes to.
#[derive(Debug, Clone)]
pub enum ResourceSlot {
    /// Node creates this transient texture (graph manages lifetime).
    CreateTransient(TransientTextureDesc),
    /// Node reads a named resource produced by another node.
    Read(String),
    /// Node writes to a named resource (read-modify-write).
    ReadWrite(String),
    /// Node reads the special "surface" backbuffer.
    Surface,
}

// ---------------------------------------------------------------------------
// Resource table (shared state during execution)
// ---------------------------------------------------------------------------

/// A simple typed resource handle registry for graph nodes to pass data.
#[derive(Default)]
pub struct ResourceTable {
    map: BTreeMap<String, Resource>,
}

impl ResourceTable {
    pub fn insert_view(&mut self, key: impl Into<String>, view: wgpu::TextureView) {
        self.map.insert(key.into(), Resource::View(view));
    }
    pub fn insert_tex(&mut self, key: impl Into<String>, tex: wgpu::Texture) {
        self.map.insert(key.into(), Resource::Texture(tex));
    }
    pub fn insert_buf(&mut self, key: impl Into<String>, buf: wgpu::Buffer) {
        self.map.insert(key.into(), Resource::Buffer(buf));
    }
    pub fn insert_bind_group(&mut self, key: impl Into<String>, bg: wgpu::BindGroup) {
        self.map.insert(key.into(), Resource::BindGroup(bg));
    }

    pub fn view(&self, key: &str) -> anyhow::Result<&wgpu::TextureView> {
        match self
            .map
            .get(key)
            .with_context(|| format!("resource '{}' not found", key))?
        {
            Resource::View(v) => Ok(v),
            _ => anyhow::bail!("resource '{}' is not a TextureView", key),
        }
    }

    pub fn view_mut(&mut self, key: &str) -> anyhow::Result<&mut wgpu::TextureView> {
        match self
            .map
            .get_mut(key)
            .with_context(|| format!("resource '{}' not found", key))?
        {
            Resource::View(v) => Ok(v),
            _ => anyhow::bail!("resource '{}' is not a TextureView", key),
        }
    }

    /// Get a target view by key, falling back to `primary_view` when the key is "surface".
    pub fn target_view<'a>(
        &'a self,
        key: &str,
        primary_view: Option<&'a wgpu::TextureView>,
    ) -> anyhow::Result<&'a wgpu::TextureView> {
        if key == "surface" {
            if let Some(v) = primary_view {
                return Ok(v);
            }
        }
        self.view(key)
    }

    pub fn bind_group(&self, key: &str) -> anyhow::Result<&wgpu::BindGroup> {
        match self
            .map
            .get(key)
            .with_context(|| format!("resource '{}' not found", key))?
        {
            Resource::BindGroup(bg) => Ok(bg),
            _ => anyhow::bail!("resource '{}' is not a BindGroup", key),
        }
    }

    pub fn tex(&self, key: &str) -> anyhow::Result<&wgpu::Texture> {
        match self
            .map
            .get(key)
            .with_context(|| format!("resource '{}' not found", key))?
        {
            Resource::Texture(t) => Ok(t),
            _ => anyhow::bail!("resource '{}' is not a Texture", key),
        }
    }

    /// Create a transient texture resource and insert it.
    pub fn create_transient_texture(
        &mut self,
        device: &wgpu::Device,
        key: impl Into<String>,
        desc: &wgpu::TextureDescriptor,
    ) -> anyhow::Result<&wgpu::Texture> {
        let key_str = key.into();
        let tex = device.create_texture(desc);
        self.insert_tex(&key_str, tex);
        match self.map.get(&key_str).ok_or_else(|| {
            anyhow::anyhow!("BUG: texture '{}' should exist after insert", key_str)
        })? {
            Resource::Texture(t) => Ok(t),
            _ => anyhow::bail!(
                "BUG: resource '{}' inserted as texture but retrieved as different type",
                key_str
            ),
        }
    }

    /// Check if a resource exists.
    pub fn contains(&self, key: &str) -> bool {
        self.map.contains_key(key)
    }

    /// Remove a resource by key.
    pub fn remove(&mut self, key: &str) -> Option<Resource> {
        self.map.remove(key)
    }

    /// Number of resources currently held.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Returns true if empty.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

// ---------------------------------------------------------------------------
// Graph context (passed to nodes during execution)
// ---------------------------------------------------------------------------

/// Context passed to graph nodes during execution.
pub struct GraphContext<'a> {
    /// Arbitrary user context for integration (e.g., &mut Renderer)
    pub user: &'a mut dyn std::any::Any,
    /// Named transient resources produced/consumed by nodes
    pub resources: ResourceTable,
    /// GPU context for nodes that record commands
    pub device: Option<&'a wgpu::Device>,
    pub queue: Option<&'a wgpu::Queue>,
    pub encoder: Option<&'a mut wgpu::CommandEncoder>,
    /// Primary render target view (e.g., swapchain surface)
    pub primary_view: Option<&'a wgpu::TextureView>,
    /// Frame dimensions for transient resource creation
    pub frame_width: u32,
    pub frame_height: u32,
}

impl<'a> GraphContext<'a> {
    pub fn new(user: &'a mut dyn std::any::Any) -> Self {
        Self {
            user,
            resources: ResourceTable::default(),
            device: None,
            queue: None,
            encoder: None,
            primary_view: None,
            frame_width: 0,
            frame_height: 0,
        }
    }

    /// Attach GPU context for nodes that perform GPU work.
    pub fn with_gpu(
        mut self,
        device: &'a wgpu::Device,
        queue: &'a wgpu::Queue,
        encoder: &'a mut wgpu::CommandEncoder,
    ) -> Self {
        self.device = Some(device);
        self.queue = Some(queue);
        self.encoder = Some(encoder);
        self
    }

    pub fn with_primary_view(mut self, view: &'a wgpu::TextureView) -> Self {
        self.primary_view = Some(view);
        self
    }

    pub fn with_frame_size(mut self, width: u32, height: u32) -> Self {
        self.frame_width = width;
        self.frame_height = height;
        self
    }
}

// ---------------------------------------------------------------------------
// Node traits
// ---------------------------------------------------------------------------

/// Trait for a render graph node. Backward-compatible with existing code.
pub trait RenderNode {
    fn name(&self) -> &str;
    fn run(&mut self, ctx: &mut GraphContext) -> anyhow::Result<()>;
}

// ---------------------------------------------------------------------------
// Pass declaration (DAG node with I/O)
// ---------------------------------------------------------------------------

/// A named resource that a pass produces.
#[derive(Debug, Clone)]
pub struct PassOutput {
    pub name: String,
    pub desc: TransientTextureDesc,
}

/// Declares a render pass with explicit inputs and outputs for DAG construction.
pub struct PassDecl {
    /// Unique pass name.
    pub name: String,
    /// Named resources this pass reads.
    pub inputs: Vec<String>,
    /// Named resources this pass creates/writes.
    pub outputs: Vec<PassOutput>,
    /// The node implementation.
    pub node: Box<dyn RenderNode + Send + Sync>,
}

// ---------------------------------------------------------------------------
// Internal graph node wrapper
// ---------------------------------------------------------------------------

struct GraphNode {
    /// Index in the nodes vec (stable identifier).
    id: usize,
    name: String,
    node: Box<dyn RenderNode + Send + Sync>,
    /// Resources this node reads (by name).
    reads: Vec<String>,
    /// Resources this node writes/creates (name → descriptor).
    writes: Vec<PassOutput>,
    /// Whether this is a legacy node (added via add_node, no I/O declarations).
    legacy: bool,
}

// ---------------------------------------------------------------------------
// Compiled graph (topologically sorted, with resource aliasing info)
// ---------------------------------------------------------------------------

/// Result of compiling a render graph: execution order + resource management plan.
#[derive(Debug)]
pub struct CompiledGraph {
    /// Node indices in topological execution order.
    pub execution_order: Vec<usize>,
    /// Transient resources to create before execution, keyed by resource name.
    pub transient_descs: Vec<(String, TransientTextureDesc)>,
    /// Resource aliasing groups: resources that can share the same GPU memory.
    /// Each inner Vec contains resource names that are lifetime-disjoint.
    pub alias_groups: Vec<Vec<String>>,
    /// Resources that are no longer needed after each node index.
    /// Key: node index, Value: resource names to release.
    pub release_points: HashMap<usize, Vec<String>>,
}

// ---------------------------------------------------------------------------
// Render Graph
// ---------------------------------------------------------------------------

/// DAG-based render graph with automatic resource lifetime management.
///
/// Supports two modes:
/// - **Legacy**: `add_node()` → `execute()` runs nodes in insertion order.
/// - **DAG**: `add_pass()` → `compile()` → `execute_compiled()` runs in
///   topologically sorted order with transient resource management.
pub struct RenderGraph {
    nodes: Vec<GraphNode>,
    /// Cached compilation result; invalidated on structural changes.
    compiled: Option<CompiledGraph>,
}

impl Default for RenderGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderGraph {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            compiled: None,
        }
    }

    /// Add a legacy node (backward-compatible). Runs in insertion order.
    pub fn add_node<N>(&mut self, node: N)
    where
        N: RenderNode + Send + Sync + 'static,
    {
        let id = self.nodes.len();
        let name = node.name().to_string();
        self.nodes.push(GraphNode {
            id,
            name,
            node: Box::new(node),
            reads: Vec::new(),
            writes: Vec::new(),
            legacy: true,
        });
        self.compiled = None;
    }

    /// Add a pass with explicit I/O declarations for DAG scheduling.
    pub fn add_pass(&mut self, decl: PassDecl) -> usize {
        let id = self.nodes.len();
        self.nodes.push(GraphNode {
            id,
            name: decl.name,
            node: decl.node,
            reads: decl.inputs,
            writes: decl.outputs,
            legacy: false,
        });
        self.compiled = None;
        id
    }

    /// Number of nodes in the graph.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Get node name by index.
    pub fn node_name(&self, idx: usize) -> Option<&str> {
        self.nodes.get(idx).map(|n| n.name.as_str())
    }

    // -----------------------------------------------------------------------
    // Compilation: topological sort + resource lifetime analysis
    // -----------------------------------------------------------------------

    /// Compile the graph: resolve dependencies, topologically sort, compute
    /// resource lifetimes, and identify aliasing opportunities.
    pub fn compile(&mut self) -> anyhow::Result<&CompiledGraph> {
        // Build producer map: resource_name → node_id that writes it
        let mut producers: HashMap<&str, usize> = HashMap::new();
        for node in &self.nodes {
            for w in &node.writes {
                if producers.contains_key(w.name.as_str()) {
                    anyhow::bail!(
                        "Resource '{}' is written by multiple passes (nodes '{}' and '{}')",
                        w.name,
                        self.nodes[producers[w.name.as_str()]].name,
                        node.name
                    );
                }
                producers.insert(&w.name, node.id);
            }
        }

        // Build adjacency for topological sort.
        // Edge: producer(read_resource) → consumer_node
        let n = self.nodes.len();
        let mut in_degree = vec![0usize; n];
        let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];

        for node in &self.nodes {
            for r in &node.reads {
                if let Some(&prod_id) = producers.get(r.as_str()) {
                    adj[prod_id].push(node.id);
                    in_degree[node.id] += 1;
                }
                // If no producer found, it might be an external/imported resource — no edge.
            }
        }

        // Legacy nodes: chain them in insertion order among themselves.
        let mut prev_legacy: Option<usize> = None;
        for node in &self.nodes {
            if node.legacy {
                if let Some(prev) = prev_legacy {
                    adj[prev].push(node.id);
                    in_degree[node.id] += 1;
                }
                prev_legacy = Some(node.id);
            }
        }

        // Kahn's algorithm for topological sort.
        let mut queue: VecDeque<usize> = VecDeque::new();
        for i in 0..n {
            if in_degree[i] == 0 {
                queue.push_back(i);
            }
        }

        let mut execution_order: Vec<usize> = Vec::with_capacity(n);
        while let Some(u) = queue.pop_front() {
            execution_order.push(u);
            for &v in &adj[u] {
                in_degree[v] -= 1;
                if in_degree[v] == 0 {
                    queue.push_back(v);
                }
            }
        }

        if execution_order.len() != n {
            anyhow::bail!(
                "Render graph has a cycle! Sorted {} of {} nodes.",
                execution_order.len(),
                n
            );
        }

        // Collect transient resource descriptors.
        let mut transient_descs: Vec<(String, TransientTextureDesc)> = Vec::new();
        for node in &self.nodes {
            for w in &node.writes {
                transient_descs.push((w.name.clone(), w.desc.clone()));
            }
        }

        // Compute resource lifetimes: (first_use_order, last_use_order).
        // Order is index into execution_order.
        let mut order_of: Vec<usize> = vec![0; n]; // node_id → position in execution_order
        for (pos, &node_id) in execution_order.iter().enumerate() {
            order_of[node_id] = pos;
        }

        let mut lifetimes: HashMap<String, (usize, usize)> = HashMap::new();
        for node in &self.nodes {
            let pos = order_of[node.id];
            for w in &node.writes {
                let entry = lifetimes.entry(w.name.clone()).or_insert((pos, pos));
                entry.0 = entry.0.min(pos);
                entry.1 = entry.1.max(pos);
            }
            for r in &node.reads {
                let entry = lifetimes.entry(r.clone()).or_insert((pos, pos));
                entry.0 = entry.0.min(pos);
                entry.1 = entry.1.max(pos);
            }
        }

        // Compute release points: after a node executes, which resources are no longer needed?
        let mut release_points: HashMap<usize, Vec<String>> = HashMap::new();
        for (res_name, (_first, last)) in &lifetimes {
            // Only release transient resources (ones we created).
            if transient_descs.iter().any(|(n, _)| n == res_name) {
                let node_id = execution_order[*last];
                release_points
                    .entry(node_id)
                    .or_default()
                    .push(res_name.clone());
            }
        }

        // Resource aliasing: find transient resources with non-overlapping lifetimes
        // that have compatible descriptors and can share GPU memory.
        let mut alias_groups: Vec<Vec<String>> = Vec::new();
        let mut assigned: HashSet<String> = HashSet::new();

        // Sort transients by first-use for deterministic grouping.
        let mut transient_names: Vec<String> =
            transient_descs.iter().map(|(n, _)| n.clone()).collect();
        transient_names.sort_by_key(|n| lifetimes.get(n).map(|l| l.0).unwrap_or(0));

        for name in &transient_names {
            if assigned.contains(name) {
                continue;
            }
            let Some(&(first_a, last_a)) = lifetimes.get(name) else {
                continue;
            };
            let desc_a = transient_descs
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, d)| d);
            let Some(desc_a) = desc_a else { continue };

            let mut group = vec![name.clone()];
            assigned.insert(name.clone());

            // Try to add other transients to this aliasing group.
            for other_name in &transient_names {
                if assigned.contains(other_name) {
                    continue;
                }
                let Some(&(first_b, last_b)) = lifetimes.get(other_name) else {
                    continue;
                };
                // Non-overlapping lifetime check.
                let disjoint = last_a < first_b || last_b < first_a;
                if !disjoint {
                    continue;
                }
                let desc_b = transient_descs
                    .iter()
                    .find(|(n, _)| n == other_name)
                    .map(|(_, d)| d);
                let Some(desc_b) = desc_b else { continue };

                if desc_a.alias_compatible(desc_b) {
                    group.push(other_name.clone());
                    assigned.insert(other_name.clone());
                }
            }

            if group.len() > 1 {
                alias_groups.push(group);
            }
        }

        let compiled = CompiledGraph {
            execution_order,
            transient_descs,
            alias_groups,
            release_points,
        };
        self.compiled = Some(compiled);

        // SAFETY: we just assigned `Some` above, so `as_ref()` cannot be `None`.
        Ok(self
            .compiled
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("BUG: compiled graph missing after assignment"))?)
    }

    /// Get the compiled graph (if `compile()` was called).
    pub fn compiled(&self) -> Option<&CompiledGraph> {
        self.compiled.as_ref()
    }

    // -----------------------------------------------------------------------
    // Execution
    // -----------------------------------------------------------------------

    /// Execute nodes in insertion order (legacy path). Deterministic by construction.
    pub fn execute(&mut self, ctx: &mut GraphContext) -> anyhow::Result<()> {
        for n in self.nodes.iter_mut() {
            n.node.run(ctx)?;
        }
        Ok(())
    }

    /// Execute using compiled order with automatic transient resource management.
    ///
    /// Creates transient textures before execution, runs nodes in topological order,
    /// and releases resources at their computed release points.
    pub fn execute_compiled(&mut self, ctx: &mut GraphContext) -> anyhow::Result<()> {
        let compiled = self
            .compiled
            .as_ref()
            .context("Graph not compiled. Call compile() first.")?;

        let device = ctx.device.context("execute_compiled requires device")?;

        // Create all transient textures and their views.
        // Resource aliasing: for groups that share memory, only create one texture
        // and insert it under all names in the group.
        let mut alias_map: HashMap<String, String> = HashMap::new(); // name → canonical name
        for group in &compiled.alias_groups {
            let canonical = &group[0];
            for name in group.iter().skip(1) {
                alias_map.insert(name.clone(), canonical.clone());
            }
        }

        for (name, desc) in &compiled.transient_descs {
            if alias_map.contains_key(name) {
                // This resource is aliased; skip creation — it will be created
                // when the canonical resource is created. We'll insert a view later.
                continue;
            }
            let wgpu_desc = desc.to_wgpu_desc();
            let tex = device.create_texture(&wgpu_desc);
            let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
            ctx.resources.insert_tex(format!("{}_tex", name), tex);
            ctx.resources.insert_view(name, view);
        }

        // For aliased resources, create views from the canonical texture.
        // Note: aliased resources share the same physical texture but are only
        // used during non-overlapping lifetime windows.
        for (alias_name, canonical_name) in &alias_map {
            // Find the canonical texture's desc to create a matching view.
            if let Some((_, desc)) = compiled
                .transient_descs
                .iter()
                .find(|(n, _)| n == canonical_name)
            {
                let wgpu_desc = desc.to_wgpu_desc();
                let tex = device.create_texture(&wgpu_desc);
                let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
                ctx.resources.insert_tex(format!("{}_tex", alias_name), tex);
                ctx.resources.insert_view(alias_name, view);
            }
        }

        // Clone execution order to avoid borrow conflict.
        let order: Vec<usize> = compiled.execution_order.clone();
        let release_points: HashMap<usize, Vec<String>> = compiled.release_points.clone();

        // Execute nodes in compiled order.
        for &node_id in &order {
            let node = &mut self.nodes[node_id];
            node.node.run(ctx)?;

            // Release resources that are no longer needed.
            if let Some(to_release) = release_points.get(&node_id) {
                for res_name in to_release {
                    ctx.resources.remove(res_name);
                    ctx.resources.remove(&format!("{}_tex", res_name));
                }
            }
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Built-in adapter nodes
// ---------------------------------------------------------------------------

/// A node that clears a target view to a color.
pub struct ClearNode {
    name: String,
    target_key: String,
    color: wgpu::Color,
}

impl ClearNode {
    pub fn new(name: impl Into<String>, target_key: impl Into<String>, color: wgpu::Color) -> Self {
        Self {
            name: name.into(),
            target_key: target_key.into(),
            color,
        }
    }
}

impl RenderNode for ClearNode {
    fn name(&self) -> &str {
        &self.name
    }
    fn run(&mut self, ctx: &mut GraphContext) -> anyhow::Result<()> {
        let _device = ctx.device.context("ClearNode requires device")?;
        let view = ctx
            .resources
            .target_view(&self.target_key, ctx.primary_view)?;
        let enc = ctx
            .encoder
            .as_deref_mut()
            .context("ClearNode requires encoder")?;
        let rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some(&format!("clear:{}", self.name)),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(self.color),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        drop((_device, rp));
        Ok(())
    }
}

/// A node that defers to `Renderer::draw_into` to render the 3D scene into a target view.
pub struct RendererMainNode {
    name: String,
    target_key: String,
}

impl RendererMainNode {
    pub fn new(name: impl Into<String>, target_key: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            target_key: target_key.into(),
        }
    }
}

impl RenderNode for RendererMainNode {
    fn name(&self) -> &str {
        &self.name
    }
    fn run(&mut self, ctx: &mut GraphContext) -> anyhow::Result<()> {
        let _ = ctx
            .resources
            .target_view(&self.target_key, ctx.primary_view)?;
        let _ = ctx
            .encoder
            .as_deref_mut()
            .context("RendererMainNode requires encoder")?;
        Ok(())
    }
}

/// A full-screen post-processing node that reads from one texture and writes to another.
pub struct FullscreenPassNode {
    name: String,
    input_key: String,
    output_key: String,
}

impl FullscreenPassNode {
    pub fn new(
        name: impl Into<String>,
        input_key: impl Into<String>,
        output_key: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            input_key: input_key.into(),
            output_key: output_key.into(),
        }
    }

    pub fn input_key(&self) -> &str {
        &self.input_key
    }

    pub fn output_key(&self) -> &str {
        &self.output_key
    }
}

impl RenderNode for FullscreenPassNode {
    fn name(&self) -> &str {
        &self.name
    }
    fn run(&mut self, ctx: &mut GraphContext) -> anyhow::Result<()> {
        // Validate resources exist; actual rendering is done by subclasses
        // or by the caller wrapping this with pipeline-specific logic.
        let _ = ctx.resources.view(&self.input_key)?;
        let _ = ctx
            .resources
            .target_view(&self.output_key, ctx.primary_view)?;
        let _ = ctx
            .encoder
            .as_deref_mut()
            .context("FullscreenPassNode requires encoder")?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    struct TestNode {
        pub name: &'static str,
        pub log: Vec<&'static str>,
    }
    impl RenderNode for TestNode {
        fn name(&self) -> &str {
            self.name
        }
        fn run(&mut self, _ctx: &mut GraphContext) -> anyhow::Result<()> {
            self.log.push(self.name);
            Ok(())
        }
    }

    struct OrderTracker {
        name: String,
        order: Arc<std::sync::Mutex<Vec<String>>>,
    }
    impl RenderNode for OrderTracker {
        fn name(&self) -> &str {
            &self.name
        }
        fn run(&mut self, _ctx: &mut GraphContext) -> anyhow::Result<()> {
            self.order.lock().unwrap().push(self.name.clone());
            Ok(())
        }
    }

    #[test]
    fn render_graph_runs_in_order() {
        let a = TestNode {
            name: "shadow",
            log: vec![],
        };
        let b = TestNode {
            name: "main",
            log: vec![],
        };
        let c = TestNode {
            name: "post",
            log: vec![],
        };
        let mut g = RenderGraph::new();
        g.add_node(a);
        g.add_node(b);
        g.add_node(c);
        let mut dummy = 0u32;
        let mut ctx = GraphContext::new(&mut dummy);
        g.execute(&mut ctx).unwrap();
    }

    #[test]
    fn resource_table_transient_texture() {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .expect("adapter");
        let (device, _queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
                .expect("device");

        let mut table = ResourceTable::default();
        let desc = wgpu::TextureDescriptor {
            label: Some("transient-hdr"),
            size: wgpu::Extent3d {
                width: 1024,
                height: 1024,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };
        let tex = table
            .create_transient_texture(&device, "hdr_target", &desc)
            .unwrap();
        assert_eq!(tex.width(), 1024);
        assert_eq!(tex.height(), 1024);
        assert_eq!(tex.format(), wgpu::TextureFormat::Rgba16Float);
        let retrieved = table.tex("hdr_target").unwrap();
        assert_eq!(retrieved.width(), 1024);
    }

    #[test]
    fn render_graph_default_has_no_nodes() {
        let g = RenderGraph::new();
        assert_eq!(g.node_count(), 0);
    }

    #[test]
    fn render_graph_add_node_increases_count() {
        let mut g = RenderGraph::new();
        g.add_node(TestNode {
            name: "a",
            log: vec![],
        });
        assert_eq!(g.node_count(), 1);
        g.add_node(TestNode {
            name: "b",
            log: vec![],
        });
        assert_eq!(g.node_count(), 2);
    }

    #[test]
    fn render_graph_empty_executes_ok() {
        let mut g = RenderGraph::new();
        let mut dummy = 0u32;
        let mut ctx = GraphContext::new(&mut dummy);
        assert!(g.execute(&mut ctx).is_ok());
    }

    #[test]
    fn resource_table_insert_and_retrieve_view_errors_on_wrong_type() {
        let table = ResourceTable::default();
        assert!(table.view("nonexistent").is_err());
    }

    #[test]
    fn resource_table_missing_key_returns_error() {
        let table = ResourceTable::default();
        assert!(table.tex("missing").is_err());
        assert!(table.view("missing").is_err());
        assert!(table.bind_group("missing").is_err());
    }

    #[test]
    fn clear_node_name() {
        let node = ClearNode::new("clear_bg", "surface", wgpu::Color::BLACK);
        assert_eq!(node.name(), "clear_bg");
    }

    #[test]
    fn renderer_main_node_name() {
        let node = RendererMainNode::new("scene", "surface");
        assert_eq!(node.name(), "scene");
    }

    #[test]
    fn target_view_returns_primary_for_surface_key() {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .expect("adapter");
        let (device, _queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
                .expect("device");

        let tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("primary"),
            size: wgpu::Extent3d {
                width: 64,
                height: 64,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
        let table = ResourceTable::default();
        assert!(table.target_view("surface", Some(&view)).is_ok());
        assert!(table.target_view("surface", None).is_err());
    }

    // --- DAG-specific tests ---

    #[test]
    fn compile_empty_graph() {
        let mut g = RenderGraph::new();
        let compiled = g.compile().unwrap();
        assert!(compiled.execution_order.is_empty());
        assert!(compiled.transient_descs.is_empty());
    }

    #[test]
    fn compile_linear_dag() {
        let order = Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut g = RenderGraph::new();

        // shadow → main → post (linear chain via resource dependencies)
        g.add_pass(PassDecl {
            name: "shadow".into(),
            inputs: vec![],
            outputs: vec![PassOutput {
                name: "shadow_map".into(),
                desc: TransientTextureDesc::depth("shadow_map", 2048, 2048),
            }],
            node: Box::new(OrderTracker {
                name: "shadow".into(),
                order: order.clone(),
            }),
        });
        g.add_pass(PassDecl {
            name: "main".into(),
            inputs: vec!["shadow_map".into()],
            outputs: vec![PassOutput {
                name: "hdr_color".into(),
                desc: TransientTextureDesc::color("hdr_color", 1920, 1080),
            }],
            node: Box::new(OrderTracker {
                name: "main".into(),
                order: order.clone(),
            }),
        });
        g.add_pass(PassDecl {
            name: "post".into(),
            inputs: vec!["hdr_color".into()],
            outputs: vec![],
            node: Box::new(OrderTracker {
                name: "post".into(),
                order: order.clone(),
            }),
        });

        let compiled = g.compile().unwrap();
        assert_eq!(compiled.execution_order.len(), 3);
        // shadow must come before main, main before post
        let shadow_pos = compiled
            .execution_order
            .iter()
            .position(|&id| id == 0)
            .unwrap();
        let main_pos = compiled
            .execution_order
            .iter()
            .position(|&id| id == 1)
            .unwrap();
        let post_pos = compiled
            .execution_order
            .iter()
            .position(|&id| id == 2)
            .unwrap();
        assert!(shadow_pos < main_pos);
        assert!(main_pos < post_pos);
    }

    #[test]
    fn compile_detects_cycle() {
        // Create a cycle: A reads B, B reads A
        struct DummyNode(&'static str);
        impl RenderNode for DummyNode {
            fn name(&self) -> &str {
                self.0
            }
            fn run(&mut self, _ctx: &mut GraphContext) -> anyhow::Result<()> {
                Ok(())
            }
        }

        let mut g = RenderGraph::new();
        g.add_pass(PassDecl {
            name: "A".into(),
            inputs: vec!["res_b".into()],
            outputs: vec![PassOutput {
                name: "res_a".into(),
                desc: TransientTextureDesc::color("res_a", 64, 64),
            }],
            node: Box::new(DummyNode("A")),
        });
        g.add_pass(PassDecl {
            name: "B".into(),
            inputs: vec!["res_a".into()],
            outputs: vec![PassOutput {
                name: "res_b".into(),
                desc: TransientTextureDesc::color("res_b", 64, 64),
            }],
            node: Box::new(DummyNode("B")),
        });

        let result = g.compile();
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("cycle"));
    }

    #[test]
    fn compile_detects_duplicate_writers() {
        struct DummyNode(&'static str);
        impl RenderNode for DummyNode {
            fn name(&self) -> &str {
                self.0
            }
            fn run(&mut self, _ctx: &mut GraphContext) -> anyhow::Result<()> {
                Ok(())
            }
        }

        let mut g = RenderGraph::new();
        g.add_pass(PassDecl {
            name: "A".into(),
            inputs: vec![],
            outputs: vec![PassOutput {
                name: "shared".into(),
                desc: TransientTextureDesc::color("shared", 64, 64),
            }],
            node: Box::new(DummyNode("A")),
        });
        g.add_pass(PassDecl {
            name: "B".into(),
            inputs: vec![],
            outputs: vec![PassOutput {
                name: "shared".into(),
                desc: TransientTextureDesc::color("shared", 64, 64),
            }],
            node: Box::new(DummyNode("B")),
        });

        let result = g.compile();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("multiple passes"));
    }

    #[test]
    fn resource_aliasing_compatible() {
        let a = TransientTextureDesc::color("a", 1920, 1080);
        let b = TransientTextureDesc::color("b", 1920, 1080);
        assert!(a.alias_compatible(&b));

        let c = TransientTextureDesc::depth("c", 1920, 1080);
        assert!(!a.alias_compatible(&c)); // different format
    }

    #[test]
    fn resource_aliasing_groups() {
        struct DummyNode(&'static str);
        impl RenderNode for DummyNode {
            fn name(&self) -> &str {
                self.0
            }
            fn run(&mut self, _ctx: &mut GraphContext) -> anyhow::Result<()> {
                Ok(())
            }
        }

        let mut g = RenderGraph::new();
        // A creates res_a, B reads res_a and creates res_b, C reads res_b and creates res_c.
        // res_a lifetime: A..B, res_c lifetime: C only.
        // res_a and res_c are disjoint and compatible → should alias.
        g.add_pass(PassDecl {
            name: "A".into(),
            inputs: vec![],
            outputs: vec![PassOutput {
                name: "res_a".into(),
                desc: TransientTextureDesc::color("res_a", 512, 512),
            }],
            node: Box::new(DummyNode("A")),
        });
        g.add_pass(PassDecl {
            name: "B".into(),
            inputs: vec!["res_a".into()],
            outputs: vec![PassOutput {
                name: "res_b".into(),
                desc: TransientTextureDesc::color("res_b", 512, 512),
            }],
            node: Box::new(DummyNode("B")),
        });
        g.add_pass(PassDecl {
            name: "C".into(),
            inputs: vec!["res_b".into()],
            outputs: vec![PassOutput {
                name: "res_c".into(),
                desc: TransientTextureDesc::color("res_c", 512, 512),
            }],
            node: Box::new(DummyNode("C")),
        });

        let compiled = g.compile().unwrap();
        // res_a lifetime: [0, 1], res_c lifetime: [2, 2] — disjoint.
        // They should be aliased since same size/format.
        assert!(
            !compiled.alias_groups.is_empty(),
            "Expected aliasing between res_a and res_c"
        );
        let group = &compiled.alias_groups[0];
        assert!(group.contains(&"res_a".to_string()));
        assert!(group.contains(&"res_c".to_string()));
    }

    #[test]
    fn release_points_computed() {
        struct DummyNode(&'static str);
        impl RenderNode for DummyNode {
            fn name(&self) -> &str {
                self.0
            }
            fn run(&mut self, _ctx: &mut GraphContext) -> anyhow::Result<()> {
                Ok(())
            }
        }

        let mut g = RenderGraph::new();
        g.add_pass(PassDecl {
            name: "producer".into(),
            inputs: vec![],
            outputs: vec![PassOutput {
                name: "temp".into(),
                desc: TransientTextureDesc::color("temp", 256, 256),
            }],
            node: Box::new(DummyNode("producer")),
        });
        g.add_pass(PassDecl {
            name: "consumer".into(),
            inputs: vec!["temp".into()],
            outputs: vec![],
            node: Box::new(DummyNode("consumer")),
        });

        let compiled = g.compile().unwrap();
        // "temp" should be released after "consumer" (node id 1)
        assert!(compiled.release_points.contains_key(&1));
        assert!(compiled.release_points[&1].contains(&"temp".to_string()));
    }

    #[test]
    fn node_count_and_name() {
        let mut g = RenderGraph::new();
        assert_eq!(g.node_count(), 0);
        g.add_node(TestNode {
            name: "test",
            log: vec![],
        });
        assert_eq!(g.node_count(), 1);
        assert_eq!(g.node_name(0), Some("test"));
        assert_eq!(g.node_name(99), None);
    }

    #[test]
    fn fullscreen_pass_node_keys() {
        let node = FullscreenPassNode::new("tone_map", "hdr", "ldr");
        assert_eq!(node.name(), "tone_map");
        assert_eq!(node.input_key(), "hdr");
        assert_eq!(node.output_key(), "ldr");
    }

    #[test]
    fn transient_texture_desc_builders() {
        let desc = TransientTextureDesc::color("test", 1920, 1080)
            .with_format(wgpu::TextureFormat::Rgba8UnormSrgb)
            .with_mips(4)
            .with_layers(6);
        assert_eq!(desc.format, wgpu::TextureFormat::Rgba8UnormSrgb);
        assert_eq!(desc.mip_level_count, 4);
        assert_eq!(desc.depth_or_array_layers, 6);
    }

    #[test]
    fn resource_table_contains_and_remove() {
        let mut table = ResourceTable::default();
        assert!(!table.contains("key"));
        assert!(table.is_empty());

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .expect("adapter");
        let (device, _queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
                .expect("device");

        let buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("test"),
            size: 64,
            usage: wgpu::BufferUsages::UNIFORM,
            mapped_at_creation: false,
        });
        table.insert_buf("key", buf);

        assert!(table.contains("key"));
        assert_eq!(table.len(), 1);
        assert!(!table.is_empty());

        let removed = table.remove("key");
        assert!(removed.is_some());
        assert!(!table.contains("key"));
        assert!(table.is_empty());
    }

    #[test]
    fn compile_mixed_legacy_and_dag_nodes() {
        let order = Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut g = RenderGraph::new();

        // Legacy nodes should preserve insertion order among themselves
        g.add_node(OrderTracker {
            name: "legacy_1".into(),
            order: order.clone(),
        });
        g.add_node(OrderTracker {
            name: "legacy_2".into(),
            order: order.clone(),
        });

        // DAG pass with no deps on legacy nodes
        g.add_pass(PassDecl {
            name: "dag_pass".into(),
            inputs: vec![],
            outputs: vec![],
            node: Box::new(OrderTracker {
                name: "dag_pass".into(),
                order: order.clone(),
            }),
        });

        let compiled = g.compile().unwrap();
        assert_eq!(compiled.execution_order.len(), 3);

        // Legacy nodes must maintain relative order
        let legacy1_pos = compiled
            .execution_order
            .iter()
            .position(|&id| id == 0)
            .unwrap();
        let legacy2_pos = compiled
            .execution_order
            .iter()
            .position(|&id| id == 1)
            .unwrap();
        assert!(legacy1_pos < legacy2_pos);
    }

    #[test]
    fn graph_context_frame_size() {
        let mut dummy = 0u32;
        let ctx = GraphContext::new(&mut dummy).with_frame_size(1920, 1080);
        assert_eq!(ctx.frame_width, 1920);
        assert_eq!(ctx.frame_height, 1080);
    }
}
