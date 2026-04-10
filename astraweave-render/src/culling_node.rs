//! Culling node for render graph integration (Phase 2 Task 3)
//!
//! Supports two modes:
//! - **Per-instance culling**: Compacts visible instance indices (original path).
//! - **Indirect draw culling** (P2-6): Generates `DrawIndexedIndirectCommand`
//!   entries per batch on the GPU, enabling `draw_indexed_indirect` dispatch
//!   without CPU readback.

use super::culling::{
    CullingPipeline, CullingResources, DrawIndexedIndirectCommand, FrustumPlanes,
    IndirectDrawPipeline, IndirectDrawResources, InstanceAABB,
};
use super::graph::{GraphContext, RenderNode};
use anyhow::Context;

/// Render graph node for GPU-driven frustum culling
pub struct CullingNode {
    name: String,
    pipeline: CullingPipeline,
    resources: Option<CullingResources>,
    instance_count: u32,
}

impl CullingNode {
    pub fn new(device: &wgpu::Device, name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            pipeline: CullingPipeline::new(device),
            resources: None,
            instance_count: 0,
        }
    }

    /// Prepare culling data before graph execution
    /// This must be called before run() with the instances to cull
    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        instances: &[InstanceAABB],
        frustum: &FrustumPlanes,
    ) {
        self.instance_count = instances.len() as u32;
        self.resources = Some(self.pipeline.update_or_create_resources(
            device,
            queue,
            instances,
            frustum,
            self.resources.take(),
        ));
    }

    /// Get reference to culling resources (for accessing buffers)
    pub fn resources(&self) -> Option<&CullingResources> {
        self.resources.as_ref()
    }
}

impl RenderNode for CullingNode {
    fn name(&self) -> &str {
        &self.name
    }

    fn run(&mut self, ctx: &mut GraphContext) -> anyhow::Result<()> {
        let encoder = ctx
            .encoder
            .as_deref_mut()
            .context("CullingNode requires encoder")?;

        let resources = self
            .resources
            .as_ref()
            .context("CullingNode::prepare() must be called before run()")?;

        // Reset count buffer to 0
        encoder.clear_buffer(&resources.count_buffer, 0, None);

        // Execute compute culling
        self.pipeline
            .execute(encoder, &resources.bind_group, self.instance_count);

        // Note: Resources remain owned by CullingNode for lifetime management
        // Downstream nodes can access buffers via resources() method or
        // by implementing a resource sharing mechanism in GraphContext

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// P2-6: IndirectCullingNode — GPU-driven draw command generation
// ---------------------------------------------------------------------------

/// Render graph node that generates `DrawIndexedIndirectCommand` entries on the
/// GPU.  Each draw batch is frustum-tested; invisible batches have their
/// `instance_count` zeroed so the GPU skips them.
pub struct IndirectCullingNode {
    name: String,
    pipeline: IndirectDrawPipeline,
    resources: Option<IndirectDrawResources>,
}

impl IndirectCullingNode {
    pub fn new(device: &wgpu::Device, name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            pipeline: IndirectDrawPipeline::new(device),
            resources: None,
        }
    }

    /// Upload batch AABBs and draw-command templates before graph execution.
    ///
    /// * `batch_aabbs` – one world-space AABB per draw batch
    /// * `templates`   – one `DrawIndexedIndirectCommand` per batch with full
    ///   `instance_count` (the compute shader zeros it for culled batches)
    /// * `frustum`     – current frame frustum planes
    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        batch_aabbs: &[InstanceAABB],
        templates: &[DrawIndexedIndirectCommand],
        frustum: &FrustumPlanes,
    ) {
        self.resources = Some(self.pipeline.update_or_create_resources(
            device,
            queue,
            batch_aabbs,
            templates,
            frustum,
            self.resources.take(),
        ));
    }

    /// Access the GPU-generated draw commands buffer and batch count.
    pub fn resources(&self) -> Option<&IndirectDrawResources> {
        self.resources.as_ref()
    }
}

impl RenderNode for IndirectCullingNode {
    fn name(&self) -> &str {
        &self.name
    }

    fn run(&mut self, ctx: &mut GraphContext) -> anyhow::Result<()> {
        let encoder = ctx
            .encoder
            .as_deref_mut()
            .context("IndirectCullingNode requires encoder")?;

        let resources = self
            .resources
            .as_ref()
            .context("IndirectCullingNode::prepare() must be called before run()")?;

        self.pipeline.execute(encoder, resources);

        Ok(())
    }
}
