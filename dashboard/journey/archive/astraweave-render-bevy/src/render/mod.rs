// Bevy Render Pipeline - Core rendering infrastructure
// Extracted from Bevy v0.14.0

//! Core rendering pipeline
//!
//! This module contains the main renderer and rendering infrastructure
//! extracted from Bevy's bevy_pbr crate.

use anyhow::Result;
use wgpu;

pub mod light;
pub mod material;
pub mod mesh;
pub mod shadow;

/// Renderer configuration
#[derive(Debug, Clone)]
pub struct RenderConfig {
    /// Surface format (usually Bgra8UnormSrgb)
    pub surface_format: wgpu::TextureFormat,

    /// Enable shadows
    pub shadows_enabled: bool,

    /// Shadow map resolution (per cascade)
    pub shadow_map_size: u32,

    /// Number of shadow cascades
    pub num_shadow_cascades: usize,

    /// Enable bloom post-processing
    pub bloom_enabled: bool,

    /// Tonemapping operator
    pub tonemapping: Tonemapping,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            surface_format: wgpu::TextureFormat::Bgra8UnormSrgb,
            shadows_enabled: true,
            shadow_map_size: 2048,
            num_shadow_cascades: 4,
            bloom_enabled: true,
            tonemapping: Tonemapping::AcesFitted,
        }
    }
}

/// Tonemapping operators (from Bevy)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Tonemapping {
    /// No tonemapping (linear)
    None,
    /// Reinhard tonemapping
    Reinhard,
    /// ACES film-like tonemapping
    AcesFitted,
    /// AgX (neutral) tonemapping
    AgX,
    /// Tony McMapface (modern) tonemapping
    TonyMcMapface,
}

/// Main Bevy-based renderer
///
/// This is the core renderer that wraps Bevy's proven rendering pipeline.
pub struct BevyRenderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: RenderConfig,
    // Day 3+: Add pipeline state
    // - shadow_pipeline
    // - pbr_pipeline
    // - post_fx_pipeline
}

impl BevyRenderer {
    /// Create a new Bevy renderer
    pub fn new(device: wgpu::Device, queue: wgpu::Queue, config: RenderConfig) -> Result<Self> {
        Ok(Self {
            device,
            queue,
            config,
        })
    }

    /// Get reference to device
    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    /// Get reference to queue
    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    /// Get configuration
    pub fn config(&self) -> &RenderConfig {
        &self.config
    }
}

// Day 3+: Implement rendering methods
// - render()
// - render_shadows()
// - render_main_pass()
// - render_post_processing()
