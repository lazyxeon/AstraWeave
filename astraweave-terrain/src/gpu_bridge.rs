//! GPU acceleration bridge for terrain generation.
//!
//! Defines the [`TerrainGpuAccelerator`] trait that allows the terrain crate to
//! dispatch expensive heightmap operations (erosion, noise generation) to GPU
//! compute pipelines without depending on `wgpu` or the render crate directly.
//!
//! # Architecture
//!
//! ```text
//! astraweave-terrain (defines trait)
//!         ↑
//!         │ implements
//!         │
//! astraweave-render (GpuErosionPipeline, GpuNoisePipeline)
//! ```
//!
//! The terrain chunk generator checks for GPU accelerator availability at
//! runtime, falling back to CPU noise/erosion when no GPU backend is registered.
//!
//! # Usage
//!
//! ```rust,ignore
//! use astraweave_terrain::gpu_bridge::{TerrainGpuAccelerator, GpuHeightmapRequest};
//!
//! fn generate_chunk(accel: Option<&dyn TerrainGpuAccelerator>, /* ... */) {
//!     let request = GpuHeightmapRequest {
//!         width: 256, height: 256,
//!         world_origin: [0.0, 0.0],
//!         cell_size: 1.0,
//!     };
//!     if let Some(gpu) = accel {
//!         if let Ok(heightmap) = gpu.generate_noise(&request, &default_noise_params()) {
//!             // Use GPU-generated heightmap
//!         }
//!     }
//! }
//! ```

/// Request describing a heightmap region to generate or erode.
#[derive(Debug, Clone)]
pub struct GpuHeightmapRequest {
    /// Grid width in cells.
    pub width: u32,
    /// Grid height in cells.
    pub height: u32,
    /// World-space origin of the heightmap (x, z).
    pub world_origin: [f32; 2],
    /// World-space size of each cell (meters).
    pub cell_size: f32,
}

/// Noise generation parameters for the GPU backend.
#[derive(Debug, Clone)]
pub struct GpuNoiseRequest {
    /// Base frequency of the noise (smaller = larger features).
    pub frequency: f32,
    /// Output amplitude multiplier.
    pub amplitude: f32,
    /// Number of fBM octaves.
    pub octaves: u32,
    /// Persistence (amplitude falloff per octave, typically 0.5).
    pub persistence: f32,
    /// Lacunarity (frequency multiplier per octave, typically 2.0).
    pub lacunarity: f32,
    /// Random seed for deterministic generation.
    pub seed: u32,
}

impl Default for GpuNoiseRequest {
    fn default() -> Self {
        Self {
            frequency: 0.005,
            amplitude: 50.0,
            octaves: 6,
            persistence: 0.5,
            lacunarity: 2.0,
            seed: 0,
        }
    }
}

/// Erosion parameters for the GPU backend.
#[derive(Debug, Clone)]
pub struct GpuErosionRequest {
    /// Number of simulation iterations.
    pub iterations: u32,
    /// Simulation timestep (seconds).
    pub dt: f32,
    /// Rain rate (water units per cell per second).
    pub rain_rate: f32,
    /// Sediment carrying capacity.
    pub sediment_capacity: f32,
    /// Dissolution rate.
    pub dissolution_rate: f32,
    /// Deposition rate.
    pub deposition_rate: f32,
    /// Evaporation rate per second.
    pub evaporation_rate: f32,
}

impl Default for GpuErosionRequest {
    fn default() -> Self {
        Self {
            iterations: 50,
            dt: 0.02,
            rain_rate: 0.01,
            sediment_capacity: 0.5,
            dissolution_rate: 0.01,
            deposition_rate: 0.01,
            evaporation_rate: 0.01,
        }
    }
}

/// Result of a GPU heightmap operation.
#[derive(Debug, Clone)]
pub struct GpuHeightmapResult {
    /// Row-major heightmap data, `width × height` entries.
    pub heights: Vec<f32>,
    /// Grid width.
    pub width: u32,
    /// Grid height.
    pub height: u32,
}

/// Trait for dispatching terrain compute work to the GPU.
///
/// Implementors live in the render crate and wrap `GpuErosionPipeline` and
/// `GpuNoisePipeline`. The terrain crate calls these through a trait object
/// (`&dyn TerrainGpuAccelerator`) with no compile-time dependency on `wgpu`.
///
/// All methods are synchronous from the caller's perspective — the
/// implementor is responsible for GPU dispatch, readback, and fence waiting.
pub trait TerrainGpuAccelerator: Send + Sync {
    /// Generate a noise heightmap on the GPU.
    ///
    /// Returns `Ok(result)` with the generated heights, or `Err` if the GPU
    /// pipeline is unavailable or dispatch fails.
    fn generate_noise(
        &self,
        request: &GpuHeightmapRequest,
        params: &GpuNoiseRequest,
    ) -> anyhow::Result<GpuHeightmapResult>;

    /// Apply hydraulic erosion to an existing heightmap on the GPU.
    ///
    /// `input_heights` must have exactly `request.width * request.height` entries.
    /// Returns the eroded heightmap.
    fn erode_heightmap(
        &self,
        request: &GpuHeightmapRequest,
        params: &GpuErosionRequest,
        input_heights: &[f32],
    ) -> anyhow::Result<GpuHeightmapResult>;

    /// Check whether this accelerator is operational (GPU device + pipelines ready).
    fn is_available(&self) -> bool;

    /// Human-readable name of the GPU backend (e.g. "wgpu/Vulkan", "wgpu/DX12").
    fn backend_name(&self) -> &str;
}
