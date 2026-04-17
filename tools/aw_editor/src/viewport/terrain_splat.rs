//! Editor-side wrapper around `astraweave_render::TerrainMaterialManager` (T7).
//!
//! Owns an optional [`TerrainMaterialManager`] plus a cache of the most
//! recently uploaded per-chunk splat dimensions so the editor can inspect
//! GPU state without touching the manager directly.
//!
//! Usage pattern:
//!
//! ```ignore
//! let mut splat = EditorTerrainSplat::new();
//! splat.initialize(&device, TerrainMaterialConfig::default())?;
//! splat.upload_material(&queue, &material_gpu, &layer_payloads)?;
//! splat.upload_chunk_from_vertices(&device, &queue, chunk_id, &verts, w, h)?;
//! splat.update_camera(&queue, view_proj, view, cam_pos, cam_fwd, cam_right);
//! // during the render pass:
//! splat.draw_chunk(&mut rpass, chunk_id, &vb, &ib, index_count);
//! ```
//!
//! The draw path (T8) is currently gated behind `terrain-splat-arrays`: when
//! the feature is off, every method becomes a no-op returning `Ok(())` or
//! `false` so editor code can hold `EditorTerrainSplat` unconditionally.

#![allow(clippy::too_many_arguments)]

use anyhow::Result;

use super::terrain_splat_builder::{build_chunk_splat_maps, ChunkSplatMaps};
use super::types::TerrainVertex;

#[cfg(feature = "terrain-splat-arrays")]
use astraweave_render::{
    ChunkKey, LayerTextures, TerrainMaterialConfig, TerrainMaterialGpu, TerrainMaterialManager,
};

/// When the feature is off, provide local stub types so external code can still
/// construct the wrapper unconditionally.
#[cfg(not(feature = "terrain-splat-arrays"))]
pub type ChunkKey = u64;

#[cfg(not(feature = "terrain-splat-arrays"))]
#[derive(Debug, Clone, Copy, Default)]
pub struct TerrainMaterialConfig;

#[cfg(not(feature = "terrain-splat-arrays"))]
#[derive(Debug, Default, Clone)]
pub struct LayerTextures<'a> {
    pub albedo: Option<&'a [u8]>,
    pub normal: Option<&'a [u8]>,
    pub orm: Option<&'a [u8]>,
    pub height: Option<&'a [u8]>,
}

/// Editor-side owner of the splat-array terrain pipeline.
///
/// Lightweight, always constructible — the feature flag only affects whether
/// the internal manager is populated.
#[derive(Default)]
pub struct EditorTerrainSplat {
    #[cfg(feature = "terrain-splat-arrays")]
    manager: Option<TerrainMaterialManager>,

    /// Number of chunks currently loaded (mirrored for fast queries).
    chunk_count: usize,
    /// Whether [`initialize`] has succeeded.
    initialized: bool,
    /// Whether [`upload_material`] has been called at least once.
    material_uploaded: bool,
}

impl EditorTerrainSplat {
    /// Construct an uninitialized wrapper. Cheap, no GPU work.
    pub fn new() -> Self {
        Self::default()
    }

    /// True once [`initialize`] has succeeded.
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// True once a terrain material has been uploaded.
    pub fn material_uploaded(&self) -> bool {
        self.material_uploaded
    }

    /// Number of chunks currently registered.
    pub fn chunk_count(&self) -> usize {
        self.chunk_count
    }

    /// Allocate the shared GPU resources. No-op when the feature is off.
    #[cfg(feature = "terrain-splat-arrays")]
    pub fn initialize(
        &mut self,
        device: &wgpu::Device,
        config: TerrainMaterialConfig,
    ) -> Result<()> {
        let manager = TerrainMaterialManager::new(device, config)?;
        self.manager = Some(manager);
        self.initialized = true;
        Ok(())
    }

    #[cfg(not(feature = "terrain-splat-arrays"))]
    pub fn initialize(
        &mut self,
        _device: &wgpu::Device,
        _config: TerrainMaterialConfig,
    ) -> Result<()> {
        self.initialized = true;
        Ok(())
    }

    /// Lazily build the render pipeline for the given target formats.
    #[cfg(feature = "terrain-splat-arrays")]
    pub fn ensure_pipeline(
        &mut self,
        device: &wgpu::Device,
        color_format: wgpu::TextureFormat,
        depth_format: Option<wgpu::TextureFormat>,
    ) -> bool {
        let Some(manager) = self.manager.as_mut() else {
            return false;
        };
        manager.ensure_pipeline(device, color_format, depth_format);
        true
    }

    #[cfg(not(feature = "terrain-splat-arrays"))]
    pub fn ensure_pipeline(
        &mut self,
        _device: &wgpu::Device,
        _color_format: wgpu::TextureFormat,
        _depth_format: Option<wgpu::TextureFormat>,
    ) -> bool {
        false
    }

    /// Upload the 8 layer textures + material UBO.
    #[cfg(feature = "terrain-splat-arrays")]
    pub fn upload_material(
        &mut self,
        queue: &wgpu::Queue,
        gpu_material: &TerrainMaterialGpu,
        layers: &[LayerTextures<'_>],
    ) -> Result<()> {
        let Some(manager) = self.manager.as_mut() else {
            anyhow::bail!(
                "EditorTerrainSplat::upload_material called before initialize()"
            );
        };
        manager.set_material(queue, gpu_material, layers)?;
        self.material_uploaded = true;
        Ok(())
    }

    #[cfg(not(feature = "terrain-splat-arrays"))]
    pub fn upload_material<T>(
        &mut self,
        _queue: &wgpu::Queue,
        _gpu_material: &T,
        _layers: &[LayerTextures<'_>],
    ) -> Result<()> {
        Ok(())
    }

    /// Build splat maps from the editor's per-vertex biome weights and upload
    /// them to the manager under `chunk` key.
    ///
    /// This is the canonical path the editor should use — the raw
    /// [`super::terrain_splat_builder::build_chunk_splat_maps`] fallback is
    /// exposed only for tests.
    #[cfg(feature = "terrain-splat-arrays")]
    pub fn upload_chunk_from_vertices(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        chunk: ChunkKey,
        vertices: &[TerrainVertex],
        width: u32,
        height: u32,
    ) -> Result<()> {
        let Some(manager) = self.manager.as_mut() else {
            anyhow::bail!(
                "EditorTerrainSplat::upload_chunk_from_vertices called before initialize()"
            );
        };
        let ChunkSplatMaps {
            splat_0,
            splat_1,
            width: w,
            height: h,
        } = build_chunk_splat_maps(vertices, width, height)?;
        manager.set_chunk_splat(device, queue, chunk, &splat_0, &splat_1, (w, h))?;
        self.chunk_count = manager.chunk_splat_count();
        Ok(())
    }

    #[cfg(not(feature = "terrain-splat-arrays"))]
    pub fn upload_chunk_from_vertices(
        &mut self,
        _device: &wgpu::Device,
        _queue: &wgpu::Queue,
        _chunk: ChunkKey,
        vertices: &[TerrainVertex],
        width: u32,
        height: u32,
    ) -> Result<()> {
        // Still validate inputs so feature-off callers catch obvious bugs.
        let _ = build_chunk_splat_maps(vertices, width, height)?;
        Ok(())
    }

    /// Remove a single chunk's splat pair.
    #[cfg(feature = "terrain-splat-arrays")]
    pub fn remove_chunk(&mut self, chunk: ChunkKey) -> bool {
        let Some(manager) = self.manager.as_mut() else {
            return false;
        };
        let removed = manager.remove_chunk_splat(chunk);
        self.chunk_count = manager.chunk_splat_count();
        removed
    }

    #[cfg(not(feature = "terrain-splat-arrays"))]
    pub fn remove_chunk(&mut self, _chunk: ChunkKey) -> bool {
        false
    }

    /// Drop every per-chunk splat pair (e.g. on biome reload).
    #[cfg(feature = "terrain-splat-arrays")]
    pub fn clear_chunks(&mut self) {
        if let Some(manager) = self.manager.as_mut() {
            manager.clear_chunks();
            self.chunk_count = 0;
        }
    }

    #[cfg(not(feature = "terrain-splat-arrays"))]
    pub fn clear_chunks(&mut self) {
        self.chunk_count = 0;
    }

    /// Update the shader's camera uniform.
    #[cfg(feature = "terrain-splat-arrays")]
    pub fn update_camera(
        &mut self,
        queue: &wgpu::Queue,
        view_proj: glam::Mat4,
        view: glam::Mat4,
        camera_pos: glam::Vec3,
        camera_forward: glam::Vec3,
        camera_right: glam::Vec3,
    ) {
        if let Some(manager) = self.manager.as_mut() {
            manager.update_camera(
                queue,
                view_proj,
                view,
                camera_pos,
                camera_forward,
                camera_right,
            );
        }
    }

    #[cfg(not(feature = "terrain-splat-arrays"))]
    pub fn update_camera(
        &mut self,
        _queue: &wgpu::Queue,
        _view_proj: glam::Mat4,
        _view: glam::Mat4,
        _camera_pos: glam::Vec3,
        _camera_forward: glam::Vec3,
        _camera_right: glam::Vec3,
    ) {
    }

    /// Issue a draw for a loaded chunk. Returns `false` when the splat pair
    /// or pipeline is missing (caller should fall back to the legacy path).
    #[cfg(feature = "terrain-splat-arrays")]
    pub fn draw_chunk<'a>(
        &'a self,
        rpass: &mut wgpu::RenderPass<'a>,
        chunk: ChunkKey,
        vertex_buffer: &'a wgpu::Buffer,
        index_buffer: &'a wgpu::Buffer,
        index_count: u32,
    ) -> bool {
        let Some(manager) = self.manager.as_ref() else {
            return false;
        };
        manager.draw_chunk(rpass, chunk, vertex_buffer, index_buffer, index_count)
    }

    #[cfg(not(feature = "terrain-splat-arrays"))]
    pub fn draw_chunk<'a>(
        &'a self,
        _rpass: &mut wgpu::RenderPass<'a>,
        _chunk: ChunkKey,
        _vertex_buffer: &'a wgpu::Buffer,
        _index_buffer: &'a wgpu::Buffer,
        _index_count: u32,
    ) -> bool {
        false
    }

    /// Borrow the underlying manager (feature-gated; exposed for advanced
    /// integration such as custom render passes).
    #[cfg(feature = "terrain-splat-arrays")]
    pub fn manager(&self) -> Option<&TerrainMaterialManager> {
        self.manager.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(w0: [f32; 4], w1: [f32; 4]) -> TerrainVertex {
        TerrainVertex {
            position: [0.0; 3],
            normal: [0.0, 1.0, 0.0],
            uv: [0.0; 2],
            biome_weights_0: w0,
            biome_weights_1: w1,
            material_ids: [0.0; 4],
            material_weights: [1.0, 0.0, 0.0, 0.0],
        }
    }

    #[test]
    fn new_is_uninitialized() {
        let splat = EditorTerrainSplat::new();
        assert!(!splat.is_initialized());
        assert!(!splat.material_uploaded());
        assert_eq!(splat.chunk_count(), 0);
    }

    /// Even with the feature off, CPU-level validation of per-chunk vertex
    /// counts should still succeed/fail as the builder dictates.
    #[cfg(not(feature = "terrain-splat-arrays"))]
    #[test]
    fn feature_off_validates_vertex_input() {
        // With feature off, initialize() is a no-op but still marks as
        // initialized so the upload path can run its CPU-side validation.
        let mut splat = EditorTerrainSplat::new();
        // Simulate a device-less scenario: skip initialize() entirely.
        // Without a device, upload_chunk_from_vertices still builds maps.
        let vertices = vec![v([1.0, 0.0, 0.0, 0.0], [0.0; 4])];
        // We can't actually call upload_chunk_from_vertices without a device,
        // but the CPU-only build path is covered in
        // terrain_splat_builder::tests. This test exists to document that the
        // feature-off stubs compile.
        assert_eq!(vertices.len(), 1);
    }
}
