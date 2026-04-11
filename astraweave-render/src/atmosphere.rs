//! Bruneton Physically-Based Atmosphere Rendering.
//!
//! Implements a complete atmosphere system based on Bruneton & Neyret 2008
//! (refined by Hillaire 2020):
//!
//! 1. **Transmittance LUT** — precomputed optical depth for (cos_zenith, altitude)
//! 2. **Sky Render** — single-scattering integration with sun/moon disc and stars
//! 3. **Aerial Perspective** — depth-based atmosphere composited onto scene geometry
//!
//! All three passes run as GPU compute shaders.

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;

// ---------------------------------------------------------------------------
// GPU types
// ---------------------------------------------------------------------------

/// Atmosphere parameters for LUT generation.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct AtmosphereParams {
    pub planet_radius: f32,         // 4
    pub atmosphere_height: f32,     // 8
    pub rayleigh_scale_h: f32,      // 12
    pub _pad0: f32,                 // 16 (align rayleigh_scatter to 16)
    pub rayleigh_scatter: [f32; 3], // 28
    pub mie_scale_h: f32,           // 32
    pub mie_scatter: f32,           // 36
    pub mie_absorption: f32,        // 40
    pub mie_g: f32,                 // 44
    pub _pad1: f32,                 // 48 (align ozone block)
    pub ozone_center_h: f32,        // 52
    pub ozone_width: f32,           // 56
    pub ozone_absorption: [f32; 3], // 68
    pub _pad2: f32,                 // 72
    pub lut_width: u32,             // 76
    pub lut_height: u32,            // 80
    pub _pad3: u32,                 // 84
    pub _pad4: u32,                 // 88
    pub _pad5: u32,                 // 92
    pub _pad6: u32,                 // 96
}

/// Sky render pass parameters.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct SkyRenderParams {
    pub inv_view_proj: [[f32; 4]; 4],
    pub view_pos: [f32; 3],
    pub planet_radius: f32,
    pub atmosphere_height: f32,
    pub rayleigh_scale_h: f32,
    pub mie_scale_h: f32,
    pub mie_g: f32,
    pub rayleigh_scatter: [f32; 3],
    pub mie_scatter: f32,
    pub sun_dir: [f32; 3],
    pub sun_intensity: f32,
    pub sun_color: [f32; 3],
    pub sun_disk_size: f32,
    pub moon_dir: [f32; 3],
    pub moon_intensity: f32,
    pub moon_color: [f32; 3],
    pub exposure: f32,
    pub resolution: [f32; 2],
    pub inv_resolution: [f32; 2],
    pub ozone_center_h: f32,
    pub ozone_width: f32,
    pub _pad0: f32,
    pub _pad1: f32,
    pub ozone_absorption: [f32; 3],
    pub mie_absorption: f32,
}

/// Aerial perspective pass parameters.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct AerialParams {
    pub inv_view_proj: [[f32; 4]; 4],
    pub view_pos: [f32; 3],
    pub planet_radius: f32,
    pub atmosphere_height: f32,
    pub rayleigh_scale_h: f32,
    pub mie_scale_h: f32,
    pub mie_g: f32,
    pub rayleigh_scatter: [f32; 3],
    pub mie_scatter: f32,
    pub sun_dir: [f32; 3],
    pub sun_intensity: f32,
    pub sun_color: [f32; 3],
    pub max_distance: f32,
    pub resolution: [f32; 2],
    pub inv_resolution: [f32; 2],
    pub near_plane: f32,
    pub far_plane: f32,
    pub ozone_center_h: f32,
    pub ozone_width: f32,
    pub ozone_absorption: [f32; 3],
    pub mie_absorption: f32,
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Physical atmosphere configuration.
#[derive(Debug, Clone)]
pub struct AtmosphereConfig {
    pub enabled: bool,
    /// Planet radius in km (Earth ≈ 6371).
    pub planet_radius: f32,
    /// Atmosphere thickness in km (Earth ≈ 100).
    pub atmosphere_height: f32,
    /// Rayleigh scattering scale height in km (Earth ≈ 8.0).
    pub rayleigh_scale_h: f32,
    /// Rayleigh scattering coefficients at sea level.
    pub rayleigh_scatter: Vec3,
    /// Mie scattering scale height in km (Earth ≈ 1.2).
    pub mie_scale_h: f32,
    /// Mie scattering coefficient at sea level.
    pub mie_scatter: f32,
    /// Mie absorption coefficient.
    pub mie_absorption: f32,
    /// Henyey-Greenstein asymmetry (Earth ≈ 0.8).
    pub mie_g: f32,
    /// Ozone layer center altitude in km (Earth ≈ 25).
    pub ozone_center_h: f32,
    /// Ozone layer width in km (Earth ≈ 15).
    pub ozone_width: f32,
    /// Ozone absorption coefficients.
    pub ozone_absorption: Vec3,
    /// Sun angular radius in radians (Earth ≈ 0.00935).
    pub sun_disk_size: f32,
    /// Sun intensity multiplier.
    pub sun_intensity: f32,
    /// Sun color tint (typically near white).
    pub sun_color: Vec3,
    /// Moon intensity multiplier.
    pub moon_intensity: f32,
    /// Moon color.
    pub moon_color: Vec3,
    /// Exposure multiplier.
    pub exposure: f32,
    /// Max aerial perspective distance in km.
    pub max_aerial_distance: f32,
    /// Transmittance LUT size.
    pub lut_width: u32,
    pub lut_height: u32,
}

impl Default for AtmosphereConfig {
    fn default() -> Self {
        Self::earth()
    }
}

impl AtmosphereConfig {
    /// Earth-like atmosphere parameters.
    pub fn earth() -> Self {
        Self {
            enabled: true,
            planet_radius: 6371.0,
            atmosphere_height: 100.0,
            rayleigh_scale_h: 8.0,
            // Rayleigh scattering coefficients for Earth (wavelength-dependent)
            rayleigh_scatter: Vec3::new(5.802e-3, 13.558e-3, 33.1e-3),
            mie_scale_h: 1.2,
            mie_scatter: 3.996e-3,
            mie_absorption: 4.4e-4,
            mie_g: 0.8,
            ozone_center_h: 25.0,
            ozone_width: 15.0,
            ozone_absorption: Vec3::new(0.65e-3, 1.881e-3, 0.085e-3),
            sun_disk_size: 0.00935,
            sun_intensity: 20.0,
            sun_color: Vec3::new(1.0, 0.98, 0.95),
            moon_intensity: 0.3,
            moon_color: Vec3::new(0.8, 0.85, 1.0),
            exposure: 10.0,
            max_aerial_distance: 50.0,
            lut_width: 256,
            lut_height: 64,
        }
    }

    /// Mars-like atmosphere (thin, reddish).
    pub fn mars() -> Self {
        Self {
            planet_radius: 3390.0,
            atmosphere_height: 80.0,
            rayleigh_scale_h: 11.0,
            rayleigh_scatter: Vec3::new(19.918e-3, 13.57e-3, 5.75e-3), // red-dominant
            mie_scale_h: 1.5,
            mie_scatter: 21.0e-3, // heavy dust
            mie_absorption: 5.0e-3,
            mie_g: 0.65,
            ozone_center_h: 30.0,
            ozone_width: 10.0,
            ozone_absorption: Vec3::ZERO,
            sun_intensity: 10.0,
            exposure: 15.0,
            ..Self::earth()
        }
    }

    /// Alien atmosphere (thick, green-purple).
    pub fn alien() -> Self {
        Self {
            planet_radius: 8000.0,
            atmosphere_height: 150.0,
            rayleigh_scale_h: 12.0,
            rayleigh_scatter: Vec3::new(8.0e-3, 25.0e-3, 6.0e-3), // green-dominant
            mie_scatter: 8.0e-3,
            mie_g: 0.7,
            exposure: 8.0,
            ..Self::earth()
        }
    }
}

// ---------------------------------------------------------------------------
// Atmosphere Pass
// ---------------------------------------------------------------------------

/// Manages the full Bruneton atmosphere pipeline.
pub struct AtmospherePass {
    config: AtmosphereConfig,
    // Pipelines
    lut_pipeline: wgpu::ComputePipeline,
    sky_pipeline: wgpu::ComputePipeline,
    aerial_pipeline: wgpu::ComputePipeline,
    // BGLs
    lut_bgl: wgpu::BindGroupLayout,
    sky_bgl: wgpu::BindGroupLayout,
    aerial_bgl: wgpu::BindGroupLayout,
    // Buffers
    lut_params_buf: wgpu::Buffer,
    sky_params_buf: wgpu::Buffer,
    aerial_params_buf: wgpu::Buffer,
    // Textures
    #[allow(dead_code)] // texture must be kept alive for view to remain valid
    transmittance_texture: wgpu::Texture,
    transmittance_view: wgpu::TextureView,
    #[allow(dead_code)] // texture must be kept alive for view to remain valid
    sky_texture: wgpu::Texture,
    sky_view: wgpu::TextureView,
    #[allow(dead_code)] // texture must be kept alive for view to remain valid
    aerial_texture: wgpu::Texture,
    aerial_view: wgpu::TextureView,
    // State
    lut_dirty: bool,
    width: u32,
    height: u32,
    /// Cached sampler (static — never changes).
    sampler: wgpu::Sampler,
    /// Cached bind groups (rebuilt on generation change or lut_dirty).
    cached_lut_bg: crate::bind_group_cache::CachedBindGroup,
    cached_sky_bg: crate::bind_group_cache::CachedBindGroup,
    cached_aerial_bg: crate::bind_group_cache::CachedBindGroup,
}

impl AtmospherePass {
    pub fn new(device: &wgpu::Device, width: u32, height: u32, config: AtmosphereConfig) -> Self {
        let fmt = wgpu::TextureFormat::Rgba16Float;

        // Transmittance LUT
        let transmittance_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("atmo_transmittance_lut"),
            size: wgpu::Extent3d {
                width: config.lut_width,
                height: config.lut_height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: fmt,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let transmittance_view =
            transmittance_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Sky output
        let make_screen_tex = |label: &str| {
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
                format: fmt,
                usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
            (tex, view)
        };
        let (sky_texture, sky_view) = make_screen_tex("atmo_sky");
        let (aerial_texture, aerial_view) = make_screen_tex("atmo_aerial");

        // Uniform buffers
        let lut_params_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("atmo_lut_params"),
            contents: &[0u8; std::mem::size_of::<AtmosphereParams>()],
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let sky_params_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("atmo_sky_params"),
            contents: &[0u8; std::mem::size_of::<SkyRenderParams>()],
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let aerial_params_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("atmo_aerial_params"),
            contents: &[0u8; std::mem::size_of::<AerialParams>()],
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // --- LUT BGL ---
        let lut_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("atmo_lut_bgl"),
            entries: &[bgl_uniform(0), bgl_storage_2d_rw(1, fmt)],
        });

        // --- Sky BGL ---
        let sky_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("atmo_sky_bgl"),
            entries: &[
                bgl_uniform(0),
                bgl_texture_2d(1), // transmittance LUT
                bgl_sampler(2),
                bgl_storage_2d_rw(3, fmt),
            ],
        });

        // --- Aerial BGL ---
        let aerial_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("atmo_aerial_bgl"),
            entries: &[
                bgl_uniform(0),
                bgl_texture_2d(1), // scene
                bgl_texture_2d(2), // depth
                bgl_texture_2d(3), // transmittance LUT
                bgl_sampler(4),
                bgl_storage_2d_rw(5, fmt),
            ],
        });

        // --- Pipelines ---
        let lut_pipeline = create_pipeline(
            device,
            &lut_bgl,
            "atmo_lut",
            concat!(
                include_str!("../shaders/constants.wgsl"),
                include_str!("../shaders/atmosphere/transmittance_lut.wgsl")
            ),
            "transmittance_lut",
        );
        let sky_pipeline = create_pipeline(
            device,
            &sky_bgl,
            "atmo_sky",
            concat!(
                include_str!("../shaders/constants.wgsl"),
                include_str!("../shaders/atmosphere/sky_render.wgsl")
            ),
            "sky_render_main",
        );
        let aerial_pipeline = create_pipeline(
            device,
            &aerial_bgl,
            "atmo_aerial",
            concat!(
                include_str!("../shaders/constants.wgsl"),
                include_str!("../shaders/atmosphere/aerial_perspective.wgsl")
            ),
            "aerial_perspective_main",
        );

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("atmo_sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        Self {
            config,
            lut_pipeline,
            sky_pipeline,
            aerial_pipeline,
            lut_bgl,
            sky_bgl,
            aerial_bgl,
            lut_params_buf,
            sky_params_buf,
            aerial_params_buf,
            transmittance_texture,
            transmittance_view,
            sky_texture,
            sky_view,
            aerial_texture,
            aerial_view,
            lut_dirty: true,
            width,
            height,
            sampler,
            cached_lut_bg: crate::bind_group_cache::CachedBindGroup::new(),
            cached_sky_bg: crate::bind_group_cache::CachedBindGroup::new(),
            cached_aerial_bg: crate::bind_group_cache::CachedBindGroup::new(),
        }
    }

    pub fn config(&self) -> &AtmosphereConfig {
        &self.config
    }

    pub fn set_config(&mut self, config: AtmosphereConfig) {
        self.lut_dirty = true;
        self.cached_lut_bg.invalidate();
        self.cached_sky_bg.invalidate();
        self.cached_aerial_bg.invalidate();
        self.config = config;
    }

    /// Sky output texture view.
    pub fn sky_view(&self) -> &wgpu::TextureView {
        &self.sky_view
    }

    /// Aerial perspective output texture view.
    pub fn aerial_view(&self) -> &wgpu::TextureView {
        &self.aerial_view
    }

    /// Transmittance LUT view (for external use in other passes).
    pub fn transmittance_view(&self) -> &wgpu::TextureView {
        &self.transmittance_view
    }

    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Prepare uniforms for this frame.
    #[allow(clippy::too_many_arguments)]
    pub fn prepare_frame(
        &mut self,
        queue: &wgpu::Queue,
        inv_view_proj: Mat4,
        view_pos: Vec3,
        sun_dir: Vec3,
        moon_dir: Vec3,
        near: f32,
        far: f32,
    ) {
        let c = &self.config;

        // LUT params (only write when dirty)
        if self.lut_dirty {
            let lut_params = AtmosphereParams {
                planet_radius: c.planet_radius,
                atmosphere_height: c.atmosphere_height,
                rayleigh_scale_h: c.rayleigh_scale_h,
                _pad0: 0.0,
                rayleigh_scatter: c.rayleigh_scatter.to_array(),
                mie_scale_h: c.mie_scale_h,
                mie_scatter: c.mie_scatter,
                mie_absorption: c.mie_absorption,
                mie_g: c.mie_g,
                _pad1: 0.0,
                ozone_center_h: c.ozone_center_h,
                ozone_width: c.ozone_width,
                ozone_absorption: c.ozone_absorption.to_array(),
                _pad2: 0.0,
                lut_width: c.lut_width,
                lut_height: c.lut_height,
                _pad3: 0,
                _pad4: 0,
                _pad5: 0,
                _pad6: 0,
            };
            queue.write_buffer(&self.lut_params_buf, 0, bytemuck::bytes_of(&lut_params));
        }

        // Sky params
        let sky_params = SkyRenderParams {
            inv_view_proj: inv_view_proj.to_cols_array_2d(),
            view_pos: view_pos.to_array(),
            planet_radius: c.planet_radius,
            atmosphere_height: c.atmosphere_height,
            rayleigh_scale_h: c.rayleigh_scale_h,
            mie_scale_h: c.mie_scale_h,
            mie_g: c.mie_g,
            rayleigh_scatter: c.rayleigh_scatter.to_array(),
            mie_scatter: c.mie_scatter,
            sun_dir: sun_dir.to_array(),
            sun_intensity: c.sun_intensity,
            sun_color: c.sun_color.to_array(),
            sun_disk_size: c.sun_disk_size,
            moon_dir: moon_dir.to_array(),
            moon_intensity: c.moon_intensity,
            moon_color: c.moon_color.to_array(),
            exposure: c.exposure,
            resolution: [self.width as f32, self.height as f32],
            inv_resolution: [1.0 / self.width as f32, 1.0 / self.height as f32],
            ozone_center_h: c.ozone_center_h,
            ozone_width: c.ozone_width,
            _pad0: 0.0,
            _pad1: 0.0,
            ozone_absorption: c.ozone_absorption.to_array(),
            mie_absorption: c.mie_absorption,
        };
        queue.write_buffer(&self.sky_params_buf, 0, bytemuck::bytes_of(&sky_params));

        // Aerial params
        let aerial_params = AerialParams {
            inv_view_proj: inv_view_proj.to_cols_array_2d(),
            view_pos: view_pos.to_array(),
            planet_radius: c.planet_radius,
            atmosphere_height: c.atmosphere_height,
            rayleigh_scale_h: c.rayleigh_scale_h,
            mie_scale_h: c.mie_scale_h,
            mie_g: c.mie_g,
            rayleigh_scatter: c.rayleigh_scatter.to_array(),
            mie_scatter: c.mie_scatter,
            sun_dir: sun_dir.to_array(),
            sun_intensity: c.sun_intensity,
            sun_color: c.sun_color.to_array(),
            max_distance: c.max_aerial_distance,
            resolution: [self.width as f32, self.height as f32],
            inv_resolution: [1.0 / self.width as f32, 1.0 / self.height as f32],
            near_plane: near,
            far_plane: far,
            ozone_center_h: c.ozone_center_h,
            ozone_width: c.ozone_width,
            ozone_absorption: c.ozone_absorption.to_array(),
            mie_absorption: c.mie_absorption,
        };
        queue.write_buffer(
            &self.aerial_params_buf,
            0,
            bytemuck::bytes_of(&aerial_params),
        );
    }

    /// Execute the atmosphere pipeline: LUT (if dirty) → Sky → Aerial Perspective.
    ///
    /// `resource_gen` is the renderer's generation counter; bind groups are
    /// rebuilt only when it changes (e.g., after a resize) or when the LUT is dirty.
    pub fn execute(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        scene_view: &wgpu::TextureView,
        depth_view: &wgpu::TextureView,
        resource_gen: crate::bind_group_cache::Generation,
    ) {
        if !self.config.enabled {
            return;
        }

        // --- Pass 1: Transmittance LUT (only when dirty) ---
        if self.lut_dirty {
            // Rebuild LUT bind group unconditionally when dirty
            let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("atmo_lut_bg"),
                layout: &self.lut_bgl,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: self.lut_params_buf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&self.transmittance_view),
                    },
                ],
            });
            self.cached_lut_bg =
                crate::bind_group_cache::CachedBindGroup::with_bind_group(bg, resource_gen);

            let wg_x = self.config.lut_width.div_ceil(8);
            let wg_y = self.config.lut_height.div_ceil(8);
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("atmo_transmittance_lut"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.lut_pipeline);
            let lut_bg = self
                .cached_lut_bg
                .get_or_rebuild(resource_gen, || unreachable!());
            pass.set_bind_group(0, lut_bg, &[]);
            pass.dispatch_workgroups(wg_x, wg_y, 1);
            self.lut_dirty = false;

            // LUT changed → sky BG depends on transmittance, must rebuild
            self.cached_sky_bg.invalidate();
            self.cached_aerial_bg.invalidate();
        }

        // --- Pass 2: Sky render ---
        {
            if !self.cached_sky_bg.is_valid(resource_gen) {
                let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("atmo_sky_bg"),
                    layout: &self.sky_bgl,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: self.sky_params_buf.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::TextureView(&self.transmittance_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::Sampler(&self.sampler),
                        },
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: wgpu::BindingResource::TextureView(&self.sky_view),
                        },
                    ],
                });
                self.cached_sky_bg =
                    crate::bind_group_cache::CachedBindGroup::with_bind_group(bg, resource_gen);
            }

            let wg_x = self.width.div_ceil(8);
            let wg_y = self.height.div_ceil(8);
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("atmo_sky_render"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.sky_pipeline);
            let sky_bg = self
                .cached_sky_bg
                .get_or_rebuild(resource_gen, || unreachable!());
            pass.set_bind_group(0, sky_bg, &[]);
            pass.dispatch_workgroups(wg_x, wg_y, 1);
        }

        // --- Pass 3: Aerial perspective ---
        {
            if !self.cached_aerial_bg.is_valid(resource_gen) {
                let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("atmo_aerial_bg"),
                    layout: &self.aerial_bgl,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: self.aerial_params_buf.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::TextureView(scene_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::TextureView(depth_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: wgpu::BindingResource::TextureView(&self.transmittance_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 4,
                            resource: wgpu::BindingResource::Sampler(&self.sampler),
                        },
                        wgpu::BindGroupEntry {
                            binding: 5,
                            resource: wgpu::BindingResource::TextureView(&self.aerial_view),
                        },
                    ],
                });
                self.cached_aerial_bg =
                    crate::bind_group_cache::CachedBindGroup::with_bind_group(bg, resource_gen);
            }

            let wg_x = self.width.div_ceil(8);
            let wg_y = self.height.div_ceil(8);
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("atmo_aerial_perspective"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.aerial_pipeline);
            let aerial_bg = self
                .cached_aerial_bg
                .get_or_rebuild(resource_gen, || unreachable!());
            pass.set_bind_group(0, aerial_bg, &[]);
            pass.dispatch_workgroups(wg_x, wg_y, 1);
        }
    }

    /// Resize screen-space textures (LUT is not resized).
    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if self.width == width && self.height == height {
            return;
        }
        let config = self.config.clone();
        *self = Self::new(device, width, height, config);
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn create_pipeline(
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

fn bgl_sampler(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
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
    fn atmosphere_params_size() {
        let size = std::mem::size_of::<AtmosphereParams>();
        // Should be 16-byte aligned blocks
        assert_eq!(size % 16, 0, "AtmosphereParams not 16-byte aligned: {size}");
    }

    #[test]
    fn sky_render_params_size() {
        let size = std::mem::size_of::<SkyRenderParams>();
        assert_eq!(size % 16, 0, "SkyRenderParams not 16-byte aligned: {size}");
    }

    #[test]
    fn aerial_params_size() {
        let size = std::mem::size_of::<AerialParams>();
        assert_eq!(size % 16, 0, "AerialParams not 16-byte aligned: {size}");
    }

    #[test]
    fn earth_config() {
        let c = AtmosphereConfig::earth();
        assert!(c.enabled);
        assert!((c.planet_radius - 6371.0).abs() < 1.0);
        assert!((c.atmosphere_height - 100.0).abs() < 1.0);
        assert!(c.mie_g > 0.0 && c.mie_g < 1.0);
        assert!(c.rayleigh_scatter.z > c.rayleigh_scatter.x, "Blue > red");
    }

    #[test]
    fn mars_config() {
        let c = AtmosphereConfig::mars();
        assert!(c.planet_radius < 6371.0, "Mars is smaller than Earth");
        assert!(
            c.rayleigh_scatter.x > c.rayleigh_scatter.z,
            "Mars: red > blue"
        );
    }

    #[test]
    fn alien_config() {
        let c = AtmosphereConfig::alien();
        assert!(c.planet_radius > 6371.0, "Alien planet is larger");
        assert!(
            c.rayleigh_scatter.y > c.rayleigh_scatter.x,
            "Alien: green dominant"
        );
    }

    #[test]
    fn atmosphere_pass_creation() {
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

        let config = AtmosphereConfig {
            lut_width: 32,
            lut_height: 16,
            ..AtmosphereConfig::earth()
        };
        let pass = AtmospherePass::new(&device, 320, 240, config);
        assert_eq!(pass.dimensions(), (320, 240));
        assert!(pass.lut_dirty);
    }

    #[test]
    fn rayleigh_wavelength_dependence() {
        // Rayleigh scattering should be strongest for blue (shorter wavelength)
        let c = AtmosphereConfig::earth();
        assert!(c.rayleigh_scatter.x < c.rayleigh_scatter.y);
        assert!(c.rayleigh_scatter.y < c.rayleigh_scatter.z);
    }

    #[test]
    fn sun_disk_size_reasonable() {
        let c = AtmosphereConfig::earth();
        // Sun subtends ~0.53° → ~0.00935 radians
        assert!(c.sun_disk_size > 0.005 && c.sun_disk_size < 0.02);
    }
}
