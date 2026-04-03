//! Post-Processing Chain for Editor Viewport
//!
//! Manages an HDR intermediate render target and orchestrates production-quality
//! post-processing passes from `astraweave-render`:
//!
//! Pipeline: Scene (HDR) → GTAO → Bloom → Auto-Exposure → Tonemap → LDR Output
//!
//! All scene sub-renderers write to `hdr_target` (Rgba16Float), and the post chain
//! processes it through compute passes before compositing to the final Bgra8UnormSrgb display.

use anyhow::Result;
use glam::{Mat4, Vec3};

use astraweave_render::atmosphere::{AtmosphereConfig, AtmospherePass};
use astraweave_render::auto_exposure::AutoExposurePass;
use astraweave_render::bloom::BloomPass;
use astraweave_render::god_rays::GodRayPass;
use astraweave_render::gtao::GtaoPass;
use astraweave_render::hdr_pipeline::{HdrPipeline, TonemapOperator};
use astraweave_render::volumetric_fog::{VolumetricFogConfig, VolumetricFogPass};

/// Configuration for the editor viewport post-processing chain.
#[derive(Debug, Clone)]
pub struct PostProcessConfig {
    pub gtao_enabled: bool,
    pub bloom_enabled: bool,
    pub auto_exposure_enabled: bool,
    pub atmosphere_enabled: bool,
    pub volumetric_fog_enabled: bool,
    pub god_rays_enabled: bool,
    pub tonemap_operator: TonemapOperator,
    /// Bloom intensity multiplier.
    pub bloom_intensity: f32,
    /// Bloom threshold (luminance above which bloom triggers).
    pub bloom_threshold: f32,
    /// Manual exposure EV when auto-exposure is disabled.
    pub manual_exposure_ev: f32,
}

impl Default for PostProcessConfig {
    fn default() -> Self {
        Self {
            gtao_enabled: true,
            bloom_enabled: true,
            auto_exposure_enabled: false, // manual by default for editor predictability
            atmosphere_enabled: true,
            volumetric_fog_enabled: true,
            god_rays_enabled: true,
            tonemap_operator: TonemapOperator::Aces,
            bloom_intensity: 0.04,
            bloom_threshold: 1.0,
            manual_exposure_ev: 0.0,
        }
    }
}

/// The post-processing chain manages HDR render targets and all post-FX passes.
pub struct PostProcessChain {
    config: PostProcessConfig,
    // HDR scene target (all scene passes render here instead of the final LDR target)
    hdr_texture: wgpu::Texture,
    hdr_view: wgpu::TextureView,
    // Pre-scene passes
    atmosphere: AtmospherePass,
    // Post-FX passes
    gtao: GtaoPass,
    bloom: BloomPass,
    auto_exposure: AutoExposurePass,
    volumetric_fog: VolumetricFogPass,
    god_rays: GodRayPass,
    hdr_pipeline: HdrPipeline,
    // Dimensions
    width: u32,
    height: u32,
}

impl PostProcessChain {
    /// Create the post-processing chain with HDR intermediate target.
    pub fn new(device: &wgpu::Device, width: u32, height: u32) -> Result<Self> {
        let config = PostProcessConfig::default();
        let hdr_fmt = wgpu::TextureFormat::Rgba16Float;

        // HDR scene target — all scene passes render here
        let (hdr_texture, hdr_view) = create_hdr_target(device, width, height, hdr_fmt);

        // Pre-scene: Bruneton atmosphere
        let atmosphere = AtmospherePass::new(device, width, height, AtmosphereConfig::earth());

        // Post-FX passes from astraweave-render
        let gtao = GtaoPass::new(device, width, height);
        let bloom = BloomPass::new(device, width, height);
        let auto_exposure = AutoExposurePass::new(device, width, height);

        // Volumetric fog (small froxels for editor performance)
        let vol_config = VolumetricFogConfig {
            froxel_dims: [80, 45, 32], // Low quality for editor responsiveness
            ..Default::default()
        };
        let volumetric_fog = VolumetricFogPass::new(device, width, height, vol_config);

        // God rays
        let god_rays = GodRayPass::new(device, width, height);

        // HdrPipeline tonemaps HDR → LDR (Bgra8UnormSrgb for editor display)
        let hdr_pipeline =
            HdrPipeline::new(device, width, height, wgpu::TextureFormat::Bgra8UnormSrgb);

        Ok(Self {
            config,
            hdr_texture,
            hdr_view,
            atmosphere,
            gtao,
            bloom,
            auto_exposure,
            volumetric_fog,
            god_rays,
            hdr_pipeline,
            width,
            height,
        })
    }

    /// Get the HDR render target view — scene passes should render to this.
    pub fn hdr_target(&self) -> &wgpu::TextureView {
        &self.hdr_view
    }

    /// Get the HDR texture (for passes that need the raw texture, not just the view).
    pub fn hdr_texture(&self) -> &wgpu::Texture {
        &self.hdr_texture
    }

    pub fn config(&self) -> &PostProcessConfig {
        &self.config
    }

    pub fn set_config(&mut self, config: PostProcessConfig) {
        self.config = config;
    }

    /// Get the GTAO output view (AO mask) for use in lighting shaders.
    pub fn gtao_view(&self) -> &wgpu::TextureView {
        self.gtao.ao_view()
    }

    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Get the atmosphere pass for external sky rendering integration.
    pub fn atmosphere(&self) -> &AtmospherePass {
        &self.atmosphere
    }

    /// Prepare atmosphere for this frame. Call before scene rendering.
    pub fn prepare_atmosphere(
        &mut self,
        queue: &wgpu::Queue,
        inv_view_proj: Mat4,
        view_pos: Vec3,
        sun_dir: Vec3,
        moon_dir: Vec3,
        near: f32,
        far: f32,
    ) {
        if self.config.atmosphere_enabled {
            self.atmosphere.prepare_frame(
                queue,
                inv_view_proj,
                view_pos,
                sun_dir,
                moon_dir,
                near,
                far,
            );
        }
    }

    /// Render the atmosphere sky into the HDR target (replaces gradient skybox).
    /// Call AFTER the skybox clears depth but BEFORE scene geometry.
    pub fn render_atmosphere(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        depth_view: &wgpu::TextureView,
    ) {
        if self.config.atmosphere_enabled {
            // The atmosphere pass renders the sky and aerial perspective
            // Sky render writes to atmosphere's internal sky_view texture
            // We use the scene HDR target as both the scene input and aerial output
            self.atmosphere
                .execute(device, encoder, &self.hdr_view, depth_view);
        }
    }

    /// Execute the post-processing chain as screen-space compute effects.
    ///
    /// Scene has already rendered to `scene_view` (Bgra8UnormSrgb).
    /// Compute passes (GTAO, god rays) operate on depth/scene data and
    /// write to their own output textures for later composition.
    ///
    /// NOTE: Bloom and tonemap require an HDR intermediate to be effective.
    /// Currently the scene renders in LDR (Bgra8UnormSrgb) because all
    /// sub-renderer pipelines hard-code that format. Full HDR pipeline
    /// will be enabled when sub-renderers accept configurable formats.
    /// For now, GTAO and god rays run as standalone compute passes.
    #[allow(clippy::too_many_arguments)]
    pub fn execute(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        depth_view: &wgpu::TextureView,
        normal_view: Option<&wgpu::TextureView>,
        scene_view: &wgpu::TextureView,
        near: f32,
        far: f32,
        fov_y: f32,
        aspect: f32,
        sun_dir: Vec3,
        sun_color: Vec3,
        _sun_intensity: f32,
        view_proj: Mat4,
    ) {
        // --- 1. GTAO (Screen-Space Ambient Occlusion) ---
        // Writes AO mask to internal texture; can be sampled by lighting shaders.
        if self.config.gtao_enabled {
            if let Some(normals) = normal_view {
                self.gtao.update_params(queue, near, far, fov_y, aspect);
                self.gtao.execute(device, encoder, depth_view, normals);
            }
        }

        // --- 2. God Rays (screen-space light shafts) ---
        // Writes additive light shafts to internal texture.
        if self.config.god_rays_enabled {
            self.god_rays
                .update_params(queue, view_proj, sun_dir, sun_color);
            self.god_rays
                .execute(device, encoder, depth_view, scene_view);
        }

        // NOTE: Bloom and auto-exposure are deferred until the sub-renderer
        // pipeline format migration to Rgba16Float is complete. They require
        // HDR input to produce correct results.
    }

    /// Resize all post-processing textures.
    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        if self.width == width && self.height == height {
            return;
        }

        let hdr_fmt = wgpu::TextureFormat::Rgba16Float;
        let (hdr_texture, hdr_view) = create_hdr_target(device, width, height, hdr_fmt);
        self.hdr_texture = hdr_texture;
        self.hdr_view = hdr_view;

        self.atmosphere.resize(device, width, height);
        self.gtao.resize(device, width, height);
        self.bloom.resize(device, width, height);
        self.auto_exposure.resize(device, width, height);
        self.volumetric_fog.resize(device, width, height);
        self.god_rays.resize(device, width, height);
        self.hdr_pipeline.resize(device, width, height);

        self.width = width;
        self.height = height;
    }
}

fn create_hdr_target(
    device: &wgpu::Device,
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("PostProcess HDR Target"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::STORAGE_BINDING,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (texture, view)
}
