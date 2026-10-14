//! Froxel-based Volumetric Fog with Light Scattering.
//!
//! Implements a 3-pass pipeline:
//!
//! 1. **Density**: Fill a 3D froxel texture with fog density (uniform + height + noise)
//! 2. **Scatter**: Compute in-scattered light per froxel from directional lights + shadows
//! 3. **Integrate**: Front-to-back accumulation along each pixel's view ray
//!
//! The result is composited over the lit scene in a final apply pass.
//! Froxels use exponential depth distribution for better near-plane resolution.

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;

// ---------------------------------------------------------------------------
// GPU types
// ---------------------------------------------------------------------------

/// Density pass uniform parameters.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct FogDensityParams {
    pub inv_view_proj: [[f32; 4]; 4],
    pub view_pos: [f32; 3],
    pub near_plane: f32,
    pub far_plane: f32,
    /// WGSL `vec3` is 16-byte aligned, so `froxel_dims` starts at byte 96.
    pub _pad_froxel: [f32; 3],
    pub froxel_dims: [u32; 3],
    pub base_density: f32,
    pub height_fog_density: f32,
    pub height_fog_falloff: f32,
    pub height_fog_offset: f32,
    pub noise_scale: f32,
    pub noise_intensity: f32,
    pub noise_speed: f32,
    pub time: f32,
    /// `wind_dir` is a `vec3` and starts at byte 144.
    pub _pad_wind: f32,
    pub wind_dir: [f32; 3],
    pub _pad: f32,
}

/// Scatter pass uniform parameters.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct ScatterParams {
    pub inv_view_proj: [[f32; 4]; 4],
    pub view_pos: [f32; 3],
    pub near_plane: f32,
    pub far_plane: f32,
    /// WGSL `vec3` alignment. `froxel_dims` starts at byte 96.
    pub _pad_froxel: [f32; 3],
    pub froxel_dims: [u32; 3],
    pub anisotropy: f32,
    pub sun_dir: [f32; 3],
    pub sun_intensity: f32,
    pub sun_color: [f32; 3],
    pub ambient_intensity: f32,
    pub ambient_color: [f32; 3],
    pub temporal_blend: f32,
    pub frame_index: u32,
    pub _pad0: f32,
    pub _pad1: f32,
    /// Rounds the uniform up to the WGSL struct size, 176.
    pub _pad2: f32,
}

/// Integration pass uniform parameters.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct IntegrateParams {
    pub inv_view_proj: [[f32; 4]; 4],
    pub resolution: [f32; 2],
    pub inv_resolution: [f32; 2],
    pub froxel_dims: [u32; 3],
    pub near_plane: f32,
    pub far_plane: f32,
    pub scatter_strength: f32,
    pub _pad0: f32,
    pub _pad1: f32,
}

/// Apply pass uniform parameters.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct ApplyParams {
    pub resolution: [f32; 2],
    pub inv_resolution: [f32; 2],
}

/// World-space window of the scar volume sampled by the density pass.
///
/// Matches `ScarFogParams` in `fog_density.wgsl`. `vec3+f32` is 16 bytes,
/// the same packing as [`FogDensityParams`].
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct ScarFogParams {
    pub origin: [f32; 3],
    pub enabled: f32,
    pub extent: [f32; 3],
    pub _pad: f32,
}

/// Edge of the scar texture, in texels. 32³ RGBA16F is 256 KiB.
pub const SCAR_VOLUME_DIM: u32 = 32;

/// CPU-baked scar volume. The renderer converts `rgba` to RGBA16F.
#[derive(Clone, Debug)]
pub struct ScarFogVolume {
    pub origin_metres: [i32; 3],
    pub extent_metres: [i32; 3],
    pub dims: [u32; 3],
    /// `x + y*dim + z*dim*dim`. R density, G forward weight, B tint weight, A unused.
    pub rgba: Vec<[f32; 4]>,
    pub any: bool,
}

/// Density and scatter GPU times for one submitted frame, when the device
/// exposes `TIMESTAMP_QUERY`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VolumetricGpuSample {
    pub density_ms: f32,
    pub scatter_ms: f32,
}

/// Cascade shadow data for volumetric scatter.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct CascadeData {
    pub vp: [[f32; 4]; 4],
    pub split: f32,
    /// WGSL `vec3` alignment puts `_pad` at byte 80. Storage size rounds to 96.
    pub _align: [f32; 3],
    pub _pad: [f32; 3],
    pub _tail: f32,
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for volumetric fog.
#[derive(Debug, Clone)]
pub struct VolumetricFogConfig {
    pub enabled: bool,
    /// Froxel grid dimensions (x, y, z). Default: 160×90×64.
    pub froxel_dims: [u32; 3],
    /// Uniform base fog density.
    pub base_density: f32,
    /// Height-based fog density at ground level.
    pub height_fog_density: f32,
    /// Exponential falloff rate for height fog.
    pub height_fog_falloff: f32,
    /// World-space Y offset for height fog (sea level).
    pub height_fog_offset: f32,
    /// Noise detail scale (larger = finer detail).
    pub noise_scale: f32,
    /// Noise contribution intensity.
    pub noise_intensity: f32,
    /// Noise animation speed.
    pub noise_speed: f32,
    /// Wind direction (normalized).
    pub wind_dir: Vec3,
    /// Henyey-Greenstein anisotropy (-1 to 1). Positive = forward scattering.
    pub anisotropy: f32,
    /// Overall scattering strength multiplier.
    pub scatter_strength: f32,
    /// Temporal blend factor for scatter history.
    pub temporal_blend: f32,
}

impl Default for VolumetricFogConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            froxel_dims: [160, 90, 64],
            // Phase 1.6-F.4.B.2.H: density scaled ×33 smaller for Target B
            // world extent (10.75 km per side) per
            // `docs/supplemental/WORLD_SCALE_CONVENTIONS.md`.
            //
            // Exponential fog: optical_depth(r) = 1 - exp(-density * r).
            //
            // Pre-F.4.B.2.H (density 0.01): 50% at 69 m, 90% at 230 m —
            // fog wall within 2% of world extent.
            //
            // Post-F.4.B.2.H (density 0.0003): 50% at 2310 m (21% of
            // extent), 90% at 7675 m (71% of extent), 99% at 15350 m (full
            // atmospheric haze at horizon). Matches AAA-typical horizon-
            // fog behavior for ~10 km worlds (see F.4.B.1 diagnostic).
            base_density: 0.0003,
            // Height fog scaled ×25 smaller for the same reason.
            // Pre-F.4.B.2.H (0.05 with falloff 0.3): near-opaque within
            // ~100 m at ground level.
            // Post-F.4.B.2.H (0.002 with falloff 0.2): noticeable ground
            // haze at ~300-500 m, tapering smoothly with altitude.
            height_fog_density: 0.002,
            height_fog_falloff: 0.2,
            height_fog_offset: 0.0,
            noise_scale: 0.05,
            noise_intensity: 0.02,
            noise_speed: 0.5,
            wind_dir: Vec3::new(1.0, 0.0, 0.3).normalize(),
            anisotropy: 0.6,
            scatter_strength: 1.0,
            temporal_blend: 0.9,
        }
    }
}

/// Quality presets for volumetric fog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VolumetricQuality {
    /// 80×45×32 froxels
    Low,
    /// 160×90×64 froxels (default)
    Medium,
    /// 240×135×128 froxels
    High,
}

impl VolumetricQuality {
    pub fn froxel_dims(self) -> [u32; 3] {
        match self {
            VolumetricQuality::Low => [80, 45, 32],
            VolumetricQuality::Medium => [160, 90, 64],
            VolumetricQuality::High => [240, 135, 128],
        }
    }
}

// ---------------------------------------------------------------------------
// Volumetric Fog Pass
// ---------------------------------------------------------------------------

/// Manages the full volumetric fog pipeline (density → scatter → integrate → apply).
pub struct VolumetricFogPass {
    config: VolumetricFogConfig,
    // Pipelines
    density_pipeline: wgpu::ComputePipeline,
    scatter_pipeline: wgpu::ComputePipeline,
    integrate_pipeline: wgpu::ComputePipeline,
    apply_pipeline: wgpu::ComputePipeline,
    // Bind group layouts
    density_bgl: wgpu::BindGroupLayout,
    scatter_bgl: wgpu::BindGroupLayout,
    integrate_bgl: wgpu::BindGroupLayout,
    apply_bgl: wgpu::BindGroupLayout,
    // Uniform buffers
    density_params_buf: wgpu::Buffer,
    scatter_params_buf: wgpu::Buffer,
    integrate_params_buf: wgpu::Buffer,
    apply_params_buf: wgpu::Buffer,
    cascade_buf: wgpu::Buffer,
    // 3D froxel textures
    #[allow(dead_code)] // texture must be kept alive for view to remain valid
    density_texture: wgpu::Texture,
    density_view: wgpu::TextureView,
    scatter_texture: wgpu::Texture,
    scatter_view: wgpu::TextureView,
    scatter_history: wgpu::Texture,
    scatter_history_view: wgpu::TextureView,
    // 2D output textures
    #[allow(dead_code)] // texture must be kept alive for view to remain valid
    integrated_texture: wgpu::Texture,
    integrated_view: wgpu::TextureView,
    #[allow(dead_code)] // texture must be kept alive for view to remain valid
    applied_texture: wgpu::Texture,
    applied_view: wgpu::TextureView,
    // State
    frame_index: u32,
    time: f32,
    width: u32,
    height: u32,
    // Cached samplers (static, created once)
    linear_sampler: wgpu::Sampler,
    shadow_sampler: wgpu::Sampler,
    // Cached bind groups — rebuilt only when resource generation changes.
    cached_density_bg: crate::bind_group_cache::CachedBindGroup,
    cached_scatter_bg: crate::bind_group_cache::CachedBindGroup,
    cached_integrate_bg: crate::bind_group_cache::CachedBindGroup,
    cached_apply_bg: crate::bind_group_cache::CachedBindGroup,
    scar_params_buf: wgpu::Buffer,
    scar_texture: wgpu::Texture,
    scar_view: wgpu::TextureView,
    timestamp_query: Option<wgpu::QuerySet>,
    timestamp_resolve: Option<wgpu::Buffer>,
    timestamp_readback: Option<wgpu::Buffer>,
    timestamp_period_ns: f32,
    timestamp_readback_pending: bool,
    timestamp_map_ready: std::sync::Arc<std::sync::atomic::AtomicBool>,
    last_gpu_sample: Option<VolumetricGpuSample>,
}

impl VolumetricFogPass {
    pub fn new(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        config: VolumetricFogConfig,
    ) -> Self {
        let [fx, fy, fz] = config.froxel_dims;
        let froxel_fmt = wgpu::TextureFormat::Rgba16Float;

        // 3D textures
        let make_3d = |label: &str| {
            let tex = device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width: fx,
                    height: fy,
                    depth_or_array_layers: fz,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D3,
                format: froxel_fmt,
                usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
            (tex, view)
        };
        let (density_texture, density_view) = make_3d("vol_density");
        let (scatter_texture, scatter_view) = make_3d("vol_scatter");
        let (scatter_history, scatter_history_view) = make_3d("vol_scatter_history");

        // 2D textures
        let make_2d = |label: &str| {
            let tex = device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: froxel_fmt,
                usage: wgpu::TextureUsages::STORAGE_BINDING
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
            let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
            (tex, view)
        };
        let (integrated_texture, integrated_view) = make_2d("vol_integrated");
        let (applied_texture, applied_view) = make_2d("vol_applied");

        // Uniform buffers
        let density_params_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("vol_density_params"),
            contents: &[0u8; std::mem::size_of::<FogDensityParams>()],
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let scatter_params_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("vol_scatter_params"),
            contents: &[0u8; std::mem::size_of::<ScatterParams>()],
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let integrate_params_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("vol_integrate_params"),
            contents: &[0u8; std::mem::size_of::<IntegrateParams>()],
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let apply_params_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("vol_apply_params"),
            contents: &[0u8; std::mem::size_of::<ApplyParams>()],
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let cascade_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("vol_cascades"),
            size: 4 * std::mem::size_of::<CascadeData>() as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let scar_params_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("vol_scar_params"),
            contents: bytemuck::bytes_of(&ScarFogParams {
                origin: [0.0; 3],
                enabled: 0.0,
                extent: [1.0; 3],
                _pad: 0.0,
            }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let scar_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("vol_scar"),
            size: wgpu::Extent3d {
                width: SCAR_VOLUME_DIM,
                height: SCAR_VOLUME_DIM,
                depth_or_array_layers: SCAR_VOLUME_DIM,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D3,
            format: froxel_fmt,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let scar_view = scar_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let timestamps = if device.features().contains(wgpu::Features::TIMESTAMP_QUERY) {
            let query = device.create_query_set(&wgpu::QuerySetDescriptor {
                label: Some("vol_fog_timestamps"),
                ty: wgpu::QueryType::Timestamp,
                count: 4,
            });
            let bytes = 4 * std::mem::size_of::<u64>() as u64;
            let resolve = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("vol_fog_ts_resolve"),
                size: bytes,
                usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            });
            let readback = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("vol_fog_ts_readback"),
                size: bytes,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            Some((query, resolve, readback))
        } else {
            None
        };

        // --- Bind group layouts ---
        let density_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("vol_density_bgl"),
            entries: &[
                bgl_uniform(0),
                bgl_storage_3d_rw(1, froxel_fmt),
                bgl_texture_3d(2),
                bgl_sampler(3),
                bgl_uniform(4),
            ],
        });

        let scatter_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("vol_scatter_bgl"),
            entries: &[
                bgl_uniform(0),                   // params
                bgl_texture_3d(1),                // density
                bgl_depth_texture(2),             // shadow (depth)
                bgl_storage_ro(3),                // cascades
                bgl_sampler(4),                   // linear sampler
                bgl_sampler_comparison(5),        // shadow sampler
                bgl_texture_3d(6),                // history
                bgl_storage_3d_rw(7, froxel_fmt), // output
            ],
        });

        let integrate_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("vol_integrate_bgl"),
            entries: &[
                bgl_uniform(0),
                bgl_texture_3d(1), // scatter
                bgl_texture_2d(2), // depth
                bgl_sampler(3),
                bgl_storage_2d_rw(4, froxel_fmt),
            ],
        });

        let apply_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("vol_apply_bgl"),
            entries: &[
                bgl_uniform(0),
                bgl_texture_2d(1), // scene color
                bgl_texture_2d(2), // volumetric result
                bgl_sampler(3),
                bgl_storage_2d_rw(4, froxel_fmt),
            ],
        });

        // --- Pipelines ---
        let density_pipeline = create_compute_pipeline(
            device,
            &density_bgl,
            "vol_density",
            include_str!("../shaders/volumetrics/fog_density.wgsl"),
            "fog_density_main",
        );
        let scatter_pipeline = create_compute_pipeline(
            device,
            &scatter_bgl,
            "vol_scatter",
            concat!(
                include_str!("../shaders/constants.wgsl"),
                include_str!("../shaders/volumetrics/scatter.wgsl")
            ),
            "scatter_main",
        );
        let integrate_pipeline = create_compute_pipeline(
            device,
            &integrate_bgl,
            "vol_integrate",
            include_str!("../shaders/volumetrics/integrate.wgsl"),
            "integrate_main",
        );
        let apply_pipeline = create_compute_pipeline(
            device,
            &apply_bgl,
            "vol_apply",
            include_str!("../shaders/volumetrics/apply.wgsl"),
            "apply_main",
        );

        let (timestamp_query, timestamp_resolve, timestamp_readback) = match timestamps {
            Some((query, resolve, readback)) => (Some(query), Some(resolve), Some(readback)),
            None => (None, None, None),
        };

        Self {
            config,
            density_pipeline,
            scatter_pipeline,
            integrate_pipeline,
            apply_pipeline,
            density_bgl,
            scatter_bgl,
            integrate_bgl,
            apply_bgl,
            density_params_buf,
            scatter_params_buf,
            integrate_params_buf,
            apply_params_buf,
            cascade_buf,
            density_texture,
            density_view,
            scatter_texture,
            scatter_view,
            scatter_history,
            scatter_history_view,
            integrated_texture,
            integrated_view,
            applied_texture,
            applied_view,
            frame_index: 0,
            time: 0.0,
            width,
            height,
            linear_sampler: device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("vol_linear_sampler"),
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                ..Default::default()
            }),
            shadow_sampler: device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("vol_shadow_sampler"),
                compare: Some(wgpu::CompareFunction::LessEqual),
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            }),
            cached_density_bg: crate::bind_group_cache::CachedBindGroup::new(),
            cached_scatter_bg: crate::bind_group_cache::CachedBindGroup::new(),
            cached_integrate_bg: crate::bind_group_cache::CachedBindGroup::new(),
            cached_apply_bg: crate::bind_group_cache::CachedBindGroup::new(),
            scar_params_buf,
            scar_texture,
            scar_view,
            timestamp_query,
            timestamp_resolve,
            timestamp_readback,
            timestamp_period_ns: 0.0,
            timestamp_readback_pending: false,
            timestamp_map_ready: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            last_gpu_sample: None,
        }
    }

    /// Nanoseconds per timestamp tick. `0` leaves samples unread.
    pub fn set_timestamp_period_ns(&mut self, period_ns: f32) {
        self.timestamp_period_ns = period_ns;
    }

    /// Upload a 32³ scar volume into the density pass. No extra froxel pass.
    pub fn upload_scar_volume(
        &self,
        queue: &wgpu::Queue,
        volume: &ScarFogVolume,
    ) -> Result<(), String> {
        if volume.dims != [SCAR_VOLUME_DIM; 3] {
            return Err(format!(
                "scar volume dims {:?}, the fog pass is {SCAR_VOLUME_DIM}³",
                volume.dims
            ));
        }
        let expected = (SCAR_VOLUME_DIM as usize).pow(3);
        if volume.rgba.len() != expected {
            return Err(format!(
                "scar volume has {} texels, expected {expected}",
                volume.rgba.len()
            ));
        }
        let mut packed = Vec::with_capacity(expected * 8);
        for texel in &volume.rgba {
            for channel in texel {
                let bits = half::f16::from_f32(*channel).to_bits().to_le_bytes();
                packed.extend_from_slice(&bits);
            }
        }
        let row_bytes = SCAR_VOLUME_DIM * 8;
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.scar_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &packed,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row_bytes),
                rows_per_image: Some(SCAR_VOLUME_DIM),
            },
            wgpu::Extent3d {
                width: SCAR_VOLUME_DIM,
                height: SCAR_VOLUME_DIM,
                depth_or_array_layers: SCAR_VOLUME_DIM,
            },
        );
        let params = ScarFogParams {
            origin: volume.origin_metres.map(|v| v as f32),
            enabled: if volume.any { 1.0 } else { 0.0 },
            extent: volume.extent_metres.map(|v| v as f32),
            _pad: 0.0,
        };
        queue.write_buffer(&self.scar_params_buf, 0, bytemuck::bytes_of(&params));
        Ok(())
    }

    pub fn last_gpu_sample(&self) -> Option<VolumetricGpuSample> {
        self.last_gpu_sample
    }

    /// Map the timestamp readback. Call after the encoder that ran [`Self::execute`]
    /// has been submitted.
    pub fn request_timestamp_readback(&mut self) {
        let Some(readback) = self.timestamp_readback.as_ref() else {
            return;
        };
        if self.timestamp_readback_pending || self.timestamp_period_ns <= 0.0 {
            return;
        }
        self.timestamp_readback_pending = true;
        self.timestamp_map_ready
            .store(false, std::sync::atomic::Ordering::Release);
        let flag = self.timestamp_map_ready.clone();
        readback
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |_result| {
                flag.store(true, std::sync::atomic::Ordering::Release);
            });
    }

    /// Poll a completed timestamp readback. Returns the new sample when one lands.
    pub fn poll_timestamps(&mut self, device: &wgpu::Device) -> Option<VolumetricGpuSample> {
        if !self.timestamp_readback_pending {
            return None;
        }
        let _ = device.poll(wgpu::PollType::Poll);
        if !self
            .timestamp_map_ready
            .load(std::sync::atomic::Ordering::Acquire)
        {
            return None;
        }
        let readback = self.timestamp_readback.as_ref()?;
        let slice = readback.slice(..);
        let data = slice.get_mapped_range();
        let timestamps: &[u64] = bytemuck::cast_slice(&data);
        let ns = self.timestamp_period_ns as f64;
        let millis = |begin: usize, end: usize| {
            let delta = timestamps[end].wrapping_sub(timestamps[begin]) as f64 * ns;
            (delta / 1_000_000.0) as f32
        };
        let sample = VolumetricGpuSample {
            density_ms: millis(0, 1),
            scatter_ms: millis(2, 3),
        };
        drop(data);
        readback.unmap();
        self.timestamp_readback_pending = false;
        self.timestamp_map_ready
            .store(false, std::sync::atomic::Ordering::Release);
        self.last_gpu_sample = Some(sample);
        Some(sample)
    }

    pub fn config(&self) -> &VolumetricFogConfig {
        &self.config
    }

    pub fn set_config(&mut self, config: VolumetricFogConfig) {
        self.config = config;
    }

    /// Get the final composited output.
    pub fn output_view(&self) -> &wgpu::TextureView {
        &self.applied_view
    }

    pub fn output_texture(&self) -> &wgpu::Texture {
        &self.applied_texture
    }

    /// Fog for a few hundred metres of view. The froxel grid is the low
    /// preset so a GTX 1660 can run it beside cascaded shadows.
    pub fn gameplay_config() -> VolumetricFogConfig {
        VolumetricFogConfig {
            enabled: true,
            froxel_dims: VolumetricQuality::Low.froxel_dims(),
            base_density: 0.004,
            height_fog_density: 0.012,
            height_fog_falloff: 0.14,
            height_fog_offset: 8.0,
            noise_intensity: 0.4,
            noise_scale: 0.07,
            anisotropy: 0.62,
            scatter_strength: 1.2,
            ..VolumetricFogConfig::default()
        }
    }

    /// Get the integrated volumetric result (before apply).
    pub fn integrated_view(&self) -> &wgpu::TextureView {
        &self.integrated_view
    }

    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Upload cascade shadow data.
    pub fn upload_cascades(&self, queue: &wgpu::Queue, cascades: &[CascadeData]) {
        let count = cascades.len().min(4);
        if count > 0 {
            queue.write_buffer(
                &self.cascade_buf,
                0,
                bytemuck::cast_slice(&cascades[..count]),
            );
        }
    }

    /// Update all uniform buffers for this frame.
    #[allow(clippy::too_many_arguments)]
    pub fn prepare_frame(
        &mut self,
        queue: &wgpu::Queue,
        inv_view_proj: Mat4,
        view_pos: Vec3,
        sun_dir: Vec3,
        sun_color: Vec3,
        sun_intensity: f32,
        ambient_color: Vec3,
        ambient_intensity: f32,
        near: f32,
        far: f32,
        dt: f32,
    ) {
        self.time += dt;

        let [fx, fy, fz] = self.config.froxel_dims;

        // Density params
        let density_params = FogDensityParams {
            inv_view_proj: inv_view_proj.to_cols_array_2d(),
            view_pos: view_pos.to_array(),
            near_plane: near,
            far_plane: far,
            _pad_froxel: [0.0; 3],
            froxel_dims: [fx, fy, fz],
            base_density: self.config.base_density,
            height_fog_density: self.config.height_fog_density,
            height_fog_falloff: self.config.height_fog_falloff,
            height_fog_offset: self.config.height_fog_offset,
            noise_scale: self.config.noise_scale,
            noise_intensity: self.config.noise_intensity,
            noise_speed: self.config.noise_speed,
            time: self.time,
            _pad_wind: 0.0,
            wind_dir: self.config.wind_dir.to_array(),
            _pad: 0.0,
        };
        queue.write_buffer(
            &self.density_params_buf,
            0,
            bytemuck::bytes_of(&density_params),
        );

        // Scatter params
        let scatter_params = ScatterParams {
            inv_view_proj: inv_view_proj.to_cols_array_2d(),
            view_pos: view_pos.to_array(),
            near_plane: near,
            far_plane: far,
            _pad_froxel: [0.0; 3],
            froxel_dims: [fx, fy, fz],
            anisotropy: self.config.anisotropy,
            sun_dir: sun_dir.to_array(),
            sun_intensity,
            sun_color: sun_color.to_array(),
            ambient_intensity,
            ambient_color: ambient_color.to_array(),
            temporal_blend: self.config.temporal_blend,
            frame_index: self.frame_index,
            _pad0: 0.0,
            _pad1: 0.0,
            _pad2: 0.0,
        };
        queue.write_buffer(
            &self.scatter_params_buf,
            0,
            bytemuck::bytes_of(&scatter_params),
        );

        // Integrate params
        let integrate_params = IntegrateParams {
            inv_view_proj: inv_view_proj.to_cols_array_2d(),
            resolution: [self.width as f32, self.height as f32],
            inv_resolution: [1.0 / self.width as f32, 1.0 / self.height as f32],
            froxel_dims: [fx, fy, fz],
            near_plane: near,
            far_plane: far,
            scatter_strength: self.config.scatter_strength,
            _pad0: 0.0,
            _pad1: 0.0,
        };
        queue.write_buffer(
            &self.integrate_params_buf,
            0,
            bytemuck::bytes_of(&integrate_params),
        );

        // Apply params
        let apply_params = ApplyParams {
            resolution: [self.width as f32, self.height as f32],
            inv_resolution: [1.0 / self.width as f32, 1.0 / self.height as f32],
        };
        queue.write_buffer(&self.apply_params_buf, 0, bytemuck::bytes_of(&apply_params));

        self.frame_index = self.frame_index.wrapping_add(1);
    }

    /// Execute the full volumetric fog pipeline.
    ///
    /// `resource_gen` is the renderer's current generation counter; bind groups
    /// are rebuilt only when it changes (e.g., after a resize).
    #[allow(clippy::too_many_arguments)]
    pub fn execute(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        depth_view: &wgpu::TextureView,
        shadow_view: &wgpu::TextureView,
        scene_color_view: &wgpu::TextureView,
        resource_gen: crate::bind_group_cache::Generation,
    ) {
        if !self.config.enabled {
            return;
        }

        let [fx, fy, fz] = self.config.froxel_dims;

        // --- Rebuild cached bind groups when resource generation changes ---

        // Density (only internal resources — could be gen 0, but keep uniform for simplicity)
        if !self.cached_density_bg.is_valid(resource_gen) {
            let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("vol_density_bg"),
                layout: &self.density_bgl,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: self.density_params_buf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&self.density_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&self.scar_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::Sampler(&self.linear_sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: self.scar_params_buf.as_entire_binding(),
                    },
                ],
            });
            self.cached_density_bg =
                crate::bind_group_cache::CachedBindGroup::with_bind_group(bg, resource_gen);
        }

        // Scatter (references external shadow_view)
        if !self.cached_scatter_bg.is_valid(resource_gen) {
            let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("vol_scatter_bg"),
                layout: &self.scatter_bgl,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: self.scatter_params_buf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&self.density_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(shadow_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: self.cascade_buf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::Sampler(&self.linear_sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: wgpu::BindingResource::Sampler(&self.shadow_sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 6,
                        resource: wgpu::BindingResource::TextureView(&self.scatter_history_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 7,
                        resource: wgpu::BindingResource::TextureView(&self.scatter_view),
                    },
                ],
            });
            self.cached_scatter_bg =
                crate::bind_group_cache::CachedBindGroup::with_bind_group(bg, resource_gen);
        }

        // Integrate (references external depth_view)
        if !self.cached_integrate_bg.is_valid(resource_gen) {
            let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("vol_integrate_bg"),
                layout: &self.integrate_bgl,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: self.integrate_params_buf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&self.scatter_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(depth_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::Sampler(&self.linear_sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::TextureView(&self.integrated_view),
                    },
                ],
            });
            self.cached_integrate_bg =
                crate::bind_group_cache::CachedBindGroup::with_bind_group(bg, resource_gen);
        }

        // Apply (references external scene_color_view)
        if !self.cached_apply_bg.is_valid(resource_gen) {
            let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("vol_apply_bg"),
                layout: &self.apply_bgl,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: self.apply_params_buf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(scene_color_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&self.integrated_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::Sampler(&self.linear_sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::TextureView(&self.applied_view),
                    },
                ],
            });
            self.cached_apply_bg =
                crate::bind_group_cache::CachedBindGroup::with_bind_group(bg, resource_gen);
        }

        // --- Dispatch compute passes using cached bind groups ---

        let query = if self.timestamp_readback_pending {
            None
        } else {
            self.timestamp_query.take()
        };

        // Pass 1: Density
        {
            let density_ts = query
                .as_ref()
                .map(|query_set| wgpu::ComputePassTimestampWrites {
                    query_set,
                    beginning_of_pass_write_index: Some(0),
                    end_of_pass_write_index: Some(1),
                });
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("vol_density"),
                timestamp_writes: density_ts,
            });
            pass.set_pipeline(&self.density_pipeline);
            let bg = self
                .cached_density_bg
                .get_or_rebuild(resource_gen, || unreachable!());
            pass.set_bind_group(0, bg, &[]);
            pass.dispatch_workgroups(fx.div_ceil(4), fy.div_ceil(4), fz.div_ceil(4));
        }

        // Pass 2: Scatter
        {
            let scatter_ts = query
                .as_ref()
                .map(|query_set| wgpu::ComputePassTimestampWrites {
                    query_set,
                    beginning_of_pass_write_index: Some(2),
                    end_of_pass_write_index: Some(3),
                });
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("vol_scatter"),
                timestamp_writes: scatter_ts,
            });
            pass.set_pipeline(&self.scatter_pipeline);
            let bg = self
                .cached_scatter_bg
                .get_or_rebuild(resource_gen, || unreachable!());
            pass.set_bind_group(0, bg, &[]);
            pass.dispatch_workgroups(fx.div_ceil(4), fy.div_ceil(4), fz.div_ceil(4));
        }

        // Pass 3: Integrate
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("vol_integrate"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.integrate_pipeline);
            let bg = self
                .cached_integrate_bg
                .get_or_rebuild(resource_gen, || unreachable!());
            pass.set_bind_group(0, bg, &[]);
            pass.dispatch_workgroups(self.width.div_ceil(8), self.height.div_ceil(8), 1);
        }

        // Pass 4: Apply
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("vol_apply"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.apply_pipeline);
            let bg = self
                .cached_apply_bg
                .get_or_rebuild(resource_gen, || unreachable!());
            pass.set_bind_group(0, bg, &[]);
            pass.dispatch_workgroups(self.width.div_ceil(8), self.height.div_ceil(8), 1);
        }

        if let Some(query) = query {
            if let (Some(resolve), Some(readback)) = (
                self.timestamp_resolve.as_ref(),
                self.timestamp_readback.as_ref(),
            ) {
                encoder.resolve_query_set(&query, 0..4, resolve, 0);
                let bytes = 4 * std::mem::size_of::<u64>() as u64;
                encoder.copy_buffer_to_buffer(resolve, 0, readback, 0, bytes);
            }
            self.timestamp_query = Some(query);
        }
    }

    /// Copy scatter output to history for next frame's temporal blend.
    pub fn copy_scatter_to_history(&self, encoder: &mut wgpu::CommandEncoder) {
        let [fx, fy, fz] = self.config.froxel_dims;
        encoder.copy_texture_to_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.scatter_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyTextureInfo {
                texture: &self.scatter_history,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::Extent3d {
                width: fx,
                height: fy,
                depth_or_array_layers: fz,
            },
        );
    }

    /// Resize screen-space textures (does not resize froxel grid).
    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if self.width == width && self.height == height {
            return;
        }
        let period = self.timestamp_period_ns;
        let config = self.config.clone();
        *self = Self::new(device, width, height, config);
        self.timestamp_period_ns = period;
    }
}

// ---------------------------------------------------------------------------
// Pipeline / BGL helpers
// ---------------------------------------------------------------------------

fn create_compute_pipeline(
    device: &wgpu::Device,
    bgl: &wgpu::BindGroupLayout,
    label: &str,
    source: &str,
    entry_point: &str,
) -> wgpu::ComputePipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(label),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(&format!("{label}_pl")),
        bind_group_layouts: &[bgl],
        push_constant_ranges: &[],
    });
    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some(&format!("{label}_pipeline")),
        layout: Some(&pl),
        module: &shader,
        entry_point: Some(entry_point),
        compilation_options: Default::default(),
        cache: None,
    })
}

fn bgl_uniform(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn bgl_texture_2d(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}

fn bgl_texture_3d(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D3,
            multisampled: false,
        },
        count: None,
    }
}

fn bgl_storage_ro(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn bgl_sampler(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        count: None,
    }
}

fn bgl_sampler_comparison(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
        count: None,
    }
}

fn bgl_depth_texture(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Depth,
            view_dimension: wgpu::TextureViewDimension::D2Array,
            multisampled: false,
        },
        count: None,
    }
}

fn bgl_storage_3d_rw(binding: u32, format: wgpu::TextureFormat) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::StorageTexture {
            access: wgpu::StorageTextureAccess::WriteOnly,
            format,
            view_dimension: wgpu::TextureViewDimension::D3,
        },
        count: None,
    }
}

fn bgl_storage_2d_rw(binding: u32, format: wgpu::TextureFormat) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::StorageTexture {
            access: wgpu::StorageTextureAccess::WriteOnly,
            format,
            view_dimension: wgpu::TextureViewDimension::D2,
        },
        count: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scar_params_size() {
        assert_eq!(std::mem::size_of::<ScarFogParams>(), 32);
    }

    #[test]
    fn scar_shader_samples_world_space_and_has_no_i64() {
        let density = include_str!("../shaders/volumetrics/fog_density.wgsl");
        let scatter = include_str!("../shaders/volumetrics/scatter.wgsl");
        assert!(!density.contains("i64"), "WGSL has no i64");
        assert!(!scatter.contains("i64"));
        assert!(density.contains("t_scar"));
        assert!(density.contains("world_pos"));
        assert!(!density.contains("% 64"));
        assert!(scatter.contains("packed.g"));
        assert!(scatter.contains("packed.b"));
        assert_eq!(VolumetricQuality::Low.froxel_dims(), [80, 45, 32]);
        assert_eq!(SCAR_VOLUME_DIM, 32);
    }

    #[test]
    fn fog_density_params_size() {
        assert_eq!(std::mem::size_of::<FogDensityParams>(), 160);
        assert_eq!(std::mem::offset_of!(FogDensityParams, froxel_dims), 96);
        assert_eq!(std::mem::offset_of!(FogDensityParams, wind_dir), 144);
    }

    #[test]
    fn scatter_params_size() {
        assert_eq!(std::mem::size_of::<ScatterParams>(), 176);
        assert_eq!(std::mem::offset_of!(ScatterParams, froxel_dims), 96);
        assert_eq!(std::mem::offset_of!(ScatterParams, ambient_color), 144);
    }

    #[test]
    fn integrate_params_size() {
        // mat4(64) + 4×f32(16) + uvec3(12)+f32(4) + 4×f32(16) = 112
        assert_eq!(std::mem::size_of::<IntegrateParams>(), 112);
    }

    #[test]
    fn apply_params_size() {
        assert_eq!(std::mem::size_of::<ApplyParams>(), 16);
    }

    #[test]
    fn cascade_data_size() {
        assert_eq!(std::mem::size_of::<CascadeData>(), 96);
        assert_eq!(std::mem::offset_of!(CascadeData, split), 64);
        assert_eq!(std::mem::offset_of!(CascadeData, _pad), 80);
    }

    #[test]
    fn default_config() {
        let c = VolumetricFogConfig::default();
        assert!(c.enabled);
        assert_eq!(c.froxel_dims, [160, 90, 64]);
        assert!(c.anisotropy > 0.0, "Should have forward scattering");
        // Phase 1.6-F.4.B.2.H: base density scaled down for Target B world
        // extent (10.75 km per side). Exponential-fog density 0.0003 gives
        // 50% opacity at ~2310 m (21% of world extent), matching AAA
        // horizon-fog behavior.
        assert!((c.base_density - 0.0003).abs() < 1e-6);
        let game = VolumetricFogPass::gameplay_config();
        assert!(game.enabled);
        assert_eq!(game.froxel_dims, VolumetricQuality::Low.froxel_dims());
        assert!(game.base_density > c.base_density);
    }

    #[test]
    fn quality_presets() {
        let low = VolumetricQuality::Low.froxel_dims();
        let med = VolumetricQuality::Medium.froxel_dims();
        let high = VolumetricQuality::High.froxel_dims();

        let total = |d: [u32; 3]| d[0] * d[1] * d[2];
        assert!(total(low) < total(med));
        assert!(total(med) < total(high));
    }

    #[test]
    fn volumetric_fog_pass_creation() {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .expect("adapter");
        let (device, _) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
                .expect("device");

        let config = VolumetricFogConfig {
            froxel_dims: [8, 8, 8],
            ..Default::default()
        };
        let pass = VolumetricFogPass::new(&device, 320, 240, config);
        assert_eq!(pass.dimensions(), (320, 240));
    }

    #[test]
    fn henyey_greenstein_isotropic() {
        // g=0 should give uniform phase function = 1/(4π)
        let phase = hg_phase(0.5, 0.0);
        let expected = 1.0 / (4.0 * std::f32::consts::PI);
        assert!(
            (phase - expected).abs() < 1e-5,
            "phase={phase}, expected={expected}"
        );
    }

    #[test]
    fn henyey_greenstein_forward() {
        // g>0 forward scattering: phase(cosθ=1) > phase(cosθ=-1)
        let forward = hg_phase(1.0, 0.6);
        let backward = hg_phase(-1.0, 0.6);
        assert!(
            forward > backward,
            "Forward should dominate: f={forward} b={backward}"
        );
    }

    #[test]
    fn scar_upload_is_timed_on_the_low_froxel_grid_when_the_device_can() {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: None,
            force_fallback_adapter: true,
        }));
        let Ok(adapter) = adapter else {
            eprintln!("veil fog timing: no adapter");
            return;
        };
        let mut features = wgpu::Features::empty();
        if adapter.features().contains(wgpu::Features::TIMESTAMP_QUERY) {
            features |= wgpu::Features::TIMESTAMP_QUERY;
        }
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("veil-fog-timing"),
            required_features: features,
            required_limits: wgpu::Limits::default(),
            memory_hints: Default::default(),
            trace: Default::default(),
        }))
        .expect("device");

        let config = VolumetricFogConfig {
            froxel_dims: VolumetricQuality::Low.froxel_dims(),
            ..VolumetricFogPass::gameplay_config()
        };
        let mut pass = VolumetricFogPass::new(&device, 64, 64, config);
        pass.set_timestamp_period_ns(queue.get_timestamp_period());
        let dim = SCAR_VOLUME_DIM as usize;
        let mut rgba = vec![[0.0_f32; 4]; dim * dim * dim];
        let center = dim / 2;
        rgba[center + center * dim + center * dim * dim] = [0.06, 1.0, 1.0, 0.0];
        pass.upload_scar_volume(
            &queue,
            &ScarFogVolume {
                origin_metres: [-64, -64, -64],
                extent_metres: [128, 128, 128],
                dims: [SCAR_VOLUME_DIM; 3],
                rgba,
                any: true,
            },
        )
        .expect("upload");

        let depth = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("test_depth"),
            size: wgpu::Extent3d {
                width: 64,
                height: 64,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let depth_view = depth.create_view(&Default::default());
        let scene = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("test_scene"),
            size: wgpu::Extent3d {
                width: 64,
                height: 64,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let scene_view = scene.create_view(&Default::default());
        let shadow = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("test_shadow"),
            size: wgpu::Extent3d {
                width: 16,
                height: 16,
                depth_or_array_layers: 4,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let shadow_view = shadow.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });

        pass.prepare_frame(
            &queue,
            glam::Mat4::IDENTITY,
            glam::Vec3::ZERO,
            glam::Vec3::Y,
            glam::Vec3::ONE,
            1.0,
            glam::Vec3::ONE,
            0.2,
            0.1,
            200.0,
            1.0 / 60.0,
        );
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("veil_fog_timing"),
        });
        pass.execute(
            &device,
            &mut encoder,
            &depth_view,
            &shadow_view,
            &scene_view,
            1,
        );
        queue.submit(Some(encoder.finish()));
        pass.request_timestamp_readback();

        let mut sample = None;
        for _ in 0..40 {
            let _ = device.poll(wgpu::PollType::Wait);
            if let Some(got) = pass.poll_timestamps(&device) {
                sample = Some(got);
                break;
            }
        }
        let path = std::path::Path::new("/opt/cursor/artifacts/veil_fog_cost.log");
        let line = match sample {
            Some(sample) => format!(
                "low preset 80x45x32 density_ms={:.4} scatter_ms={:.4} sum_ms={:.4} timestamps=yes\n",
                sample.density_ms,
                sample.scatter_ms,
                sample.density_ms + sample.scatter_ms
            ),
            None => "low preset 80x45x32 timestamps=unavailable\n".to_string(),
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("artifacts dir");
        }
        std::fs::write(path, &line).expect("write timing log");
        eprintln!("{line}");
        if let Some(sample) = sample {
            assert!(sample.density_ms.is_finite() && sample.density_ms >= 0.0);
            assert!(sample.scatter_ms.is_finite() && sample.scatter_ms >= 0.0);
        }
    }

    /// CPU implementation of Henyey-Greenstein for test validation.
    fn hg_phase(cos_theta: f32, g: f32) -> f32 {
        let g2 = g * g;
        let denom = 1.0 + g2 - 2.0 * g * cos_theta;
        (1.0 - g2) / (4.0 * std::f32::consts::PI * denom.powf(1.5))
    }
}
