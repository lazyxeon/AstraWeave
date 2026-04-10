//! Weighted Blended Order-Independent Transparency (WBOIT)
//!
//! McGuire & Bavoil (2013) — "Weighted Blended Order-Independent Transparency"
//!
//! Two-pass algorithm:
//! 1. **Accumulation**: Render all transparent geometry into two render targets
//!    with specific blend states. Each fragment computes a depth-dependent weight
//!    and outputs weighted premultiplied color + revealage.
//! 2. **Composite**: Full-screen resolve pass blends accumulated transparency
//!    over the opaque scene.

use wgpu;

// ─── Constants ───

/// Texture format for the accumulation buffer (weighted premultiplied color + alpha).
pub const ACCUM_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

/// Texture format for the revealage buffer (product of `1 − αᵢ`).
pub const REVEALAGE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R8Unorm;

// ─── Blend States ───

/// Blend state for the accumulation render target (additive: `dst += src`).
pub fn accum_blend_state() -> wgpu::BlendState {
    wgpu::BlendState {
        color: wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::One,
            operation: wgpu::BlendOperation::Add,
        },
        alpha: wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::One,
            operation: wgpu::BlendOperation::Add,
        },
    }
}

/// Blend state for the revealage render target (multiplicative: `dst *= (1 − src.r)`).
///
/// Clear the revealage texture to `1.0` before use.
pub fn revealage_blend_state() -> wgpu::BlendState {
    wgpu::BlendState {
        color: wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::Zero,
            dst_factor: wgpu::BlendFactor::OneMinusSrc,
            operation: wgpu::BlendOperation::Add,
        },
        alpha: wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::Zero,
            dst_factor: wgpu::BlendFactor::OneMinusSrc,
            operation: wgpu::BlendOperation::Add,
        },
    }
}

/// Blend state for the composite pass (standard alpha-over blending).
pub fn composite_blend_state() -> wgpu::BlendState {
    wgpu::BlendState {
        color: wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::SrcAlpha,
            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
            operation: wgpu::BlendOperation::Add,
        },
        alpha: wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
            operation: wgpu::BlendOperation::Add,
        },
    }
}

// ─── Weight Function (CPU mirror of WGSL `oit_weight`) ───

/// Compute the depth-dependent weight for a transparent fragment.
///
/// This is the CPU-side mirror of the WGSL `oit_weight()` function.
/// Higher weights are assigned to closer fragments and higher alpha values.
#[allow(clippy::manual_clamp)]
pub fn oit_weight(z: f32, alpha: f32) -> f32 {
    let clamped_z = z.abs().clamp(0.01, 3000.0);
    let w = alpha
        * (10.0 / (0.00001 + (clamped_z / 5.0).powi(2) + (clamped_z / 200.0).powi(6)))
            .max(0.01)
            .min(3000.0);
    w.clamp(0.001, 300.0)
}

// ─── OIT Buffers ───

/// GPU accumulation buffers for a single frame of WBOIT.
pub struct OitBuffers {
    #[allow(dead_code)] // texture must be kept alive for view to remain valid
    accum_texture: wgpu::Texture,
    accum_view: wgpu::TextureView,
    #[allow(dead_code)] // texture must be kept alive for view to remain valid
    revealage_texture: wgpu::Texture,
    revealage_view: wgpu::TextureView,
    width: u32,
    height: u32,
}

impl OitBuffers {
    /// Create accumulation buffers for a given viewport size.
    pub fn new(device: &wgpu::Device, width: u32, height: u32) -> Self {
        let accum_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("wboit_accum"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: ACCUM_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let accum_view = accum_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let revealage_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("wboit_revealage"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: REVEALAGE_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let revealage_view = revealage_texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self {
            accum_texture,
            accum_view,
            revealage_texture,
            revealage_view,
            width,
            height,
        }
    }

    /// Width in pixels.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Height in pixels.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// View into the accumulation texture (Rgba16Float).
    pub fn accum_view(&self) -> &wgpu::TextureView {
        &self.accum_view
    }

    /// View into the revealage texture (R8Unorm).
    pub fn revealage_view(&self) -> &wgpu::TextureView {
        &self.revealage_view
    }

    /// Returns `wgpu::RenderPassColorAttachment` entries for the accumulation pass.
    ///
    /// Index 0 = accumulation (clear to black-transparent),
    /// Index 1 = revealage (clear to 1.0).
    pub fn accumulation_attachments(&self) -> [wgpu::RenderPassColorAttachment<'_>; 2] {
        [
            wgpu::RenderPassColorAttachment {
                view: &self.accum_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 0.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            },
            wgpu::RenderPassColorAttachment {
                view: &self.revealage_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 1.0,
                        g: 1.0,
                        b: 1.0,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            },
        ]
    }
}

// ─── Composite Pipeline ───

/// Weighted Blended OIT renderer.
///
/// Manages accumulation buffers and provides a composite pass to resolve
/// transparent fragments over the opaque scene.
pub struct WboitRenderer {
    buffers: Option<OitBuffers>,
    composite_pipeline: wgpu::RenderPipeline,
    composite_bgl: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    target_format: wgpu::TextureFormat,
}

impl WboitRenderer {
    /// Create a new WBOIT renderer.
    ///
    /// `target_format` is the swap-chain / final output format (e.g. Bgra8Unorm).
    pub fn new(device: &wgpu::Device, target_format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("wboit_composite_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/oit_wboit.wgsl").into()),
        });

        let composite_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("wboit_composite_bgl"),
            entries: &[
                // binding 0: accumulation texture
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // binding 1: revealage texture
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // binding 2: sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("wboit_composite_layout"),
            bind_group_layouts: &[&composite_bgl],
            push_constant_ranges: &[],
        });

        let composite_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("wboit_composite_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_composite"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_composite"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: target_format,
                    blend: Some(composite_blend_state()),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("wboit_composite_sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        Self {
            buffers: None,
            composite_pipeline,
            composite_bgl,
            sampler,
            target_format,
        }
    }

    /// (Re)create accumulation buffers for the given viewport size.
    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        // Skip if size hasn't changed
        if let Some(ref buf) = self.buffers {
            if buf.width == width && buf.height == height {
                return;
            }
        }
        self.buffers = Some(OitBuffers::new(device, width, height));
    }

    /// Returns a reference to the current OIT buffers, if allocated.
    pub fn buffers(&self) -> Option<&OitBuffers> {
        self.buffers.as_ref()
    }

    /// The target swap-chain format this renderer was created for.
    pub fn target_format(&self) -> wgpu::TextureFormat {
        self.target_format
    }

    /// The bind group layout for the composite pass.
    pub fn composite_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.composite_bgl
    }

    /// Create a bind group referencing the current accumulation buffers.
    ///
    /// Returns `None` if `resize()` has not been called yet.
    pub fn create_composite_bind_group(&self, device: &wgpu::Device) -> Option<wgpu::BindGroup> {
        let buf = self.buffers.as_ref()?;
        Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("wboit_composite_bg"),
            layout: &self.composite_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&buf.accum_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&buf.revealage_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        }))
    }

    /// Encode the composite pass: resolve accumulated transparency over the opaque scene.
    ///
    /// The `output_view` should be the opaque scene render target (already populated).
    /// Transparent fragments are alpha-blended on top.
    pub fn composite(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        output_view: &wgpu::TextureView,
        device: &wgpu::Device,
    ) {
        let bind_group = match self.create_composite_bind_group(device) {
            Some(bg) => bg,
            None => return, // no buffers allocated yet
        };

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("wboit_composite_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: output_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load, // preserve the opaque scene
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        pass.set_pipeline(&self.composite_pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.draw(0..3, 0..1); // full-screen triangle
    }
}

// ─── Tests ───

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accum_blend_is_additive() {
        let bs = accum_blend_state();
        assert_eq!(bs.color.src_factor, wgpu::BlendFactor::One);
        assert_eq!(bs.color.dst_factor, wgpu::BlendFactor::One);
        assert_eq!(bs.color.operation, wgpu::BlendOperation::Add);
        assert_eq!(bs.alpha.src_factor, wgpu::BlendFactor::One);
        assert_eq!(bs.alpha.dst_factor, wgpu::BlendFactor::One);
    }

    #[test]
    fn revealage_blend_is_multiplicative() {
        let bs = revealage_blend_state();
        assert_eq!(bs.color.src_factor, wgpu::BlendFactor::Zero);
        assert_eq!(bs.color.dst_factor, wgpu::BlendFactor::OneMinusSrc);
        assert_eq!(bs.color.operation, wgpu::BlendOperation::Add);
    }

    #[test]
    fn composite_blend_is_alpha_over() {
        let bs = composite_blend_state();
        assert_eq!(bs.color.src_factor, wgpu::BlendFactor::SrcAlpha);
        assert_eq!(bs.color.dst_factor, wgpu::BlendFactor::OneMinusSrcAlpha);
    }

    #[test]
    fn oit_weight_close_fragments_heavier() {
        let w_near = oit_weight(1.0, 0.5);
        let w_far = oit_weight(100.0, 0.5);
        assert!(w_near > w_far, "near={w_near} should be > far={w_far}");
    }

    #[test]
    fn oit_weight_higher_alpha_heavier() {
        let w_low = oit_weight(10.0, 0.2);
        let w_high = oit_weight(10.0, 0.8);
        assert!(
            w_high > w_low,
            "high_alpha={w_high} should be > low_alpha={w_low}"
        );
    }

    #[test]
    fn oit_weight_zero_alpha() {
        let w = oit_weight(10.0, 0.0);
        assert!(w >= 0.001, "weight should be clamped to 0.001 minimum");
    }

    #[test]
    fn oit_weight_clamped_range() {
        // Extreme near
        let w = oit_weight(0.001, 1.0);
        assert!(w <= 300.0);
        assert!(w >= 0.001);
        // Extreme far
        let w = oit_weight(10000.0, 1.0);
        assert!(w <= 300.0);
        assert!(w >= 0.001);
    }

    #[test]
    fn oit_weight_negative_z_treated_as_positive() {
        let w_pos = oit_weight(10.0, 0.5);
        let w_neg = oit_weight(-10.0, 0.5);
        assert!((w_pos - w_neg).abs() < f32::EPSILON);
    }

    #[test]
    fn parse_oit_wboit_wgsl() {
        let src = include_str!("../shaders/oit_wboit.wgsl");
        let result = naga::front::wgsl::parse_str(src);
        assert!(result.is_ok(), "WGSL parse failed: {result:?}");
    }

    #[test]
    fn oit_buffers_creation() {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = match pollster::block_on(
            instance.request_adapter(&wgpu::RequestAdapterOptions::default()),
        ) {
            Ok(a) => a,
            Err(_) => return,
        };
        let (device, _queue) =
            match pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())) {
                Ok(dq) => dq,
                Err(_) => return,
            };

        let buffers = OitBuffers::new(&device, 1920, 1080);
        assert_eq!(buffers.width(), 1920);
        assert_eq!(buffers.height(), 1080);
        let _av = buffers.accum_view();
        let _rv = buffers.revealage_view();
    }

    #[test]
    fn oit_buffers_accumulation_attachments() {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = match pollster::block_on(
            instance.request_adapter(&wgpu::RequestAdapterOptions::default()),
        ) {
            Ok(a) => a,
            Err(_) => return,
        };
        let (device, _queue) =
            match pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())) {
                Ok(dq) => dq,
                Err(_) => return,
            };

        let buffers = OitBuffers::new(&device, 800, 600);
        let attachments = buffers.accumulation_attachments();
        assert_eq!(attachments.len(), 2);
    }

    #[test]
    fn wboit_renderer_creation() {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = match pollster::block_on(
            instance.request_adapter(&wgpu::RequestAdapterOptions::default()),
        ) {
            Ok(a) => a,
            Err(_) => return,
        };
        let (device, _queue) =
            match pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())) {
                Ok(dq) => dq,
                Err(_) => return,
            };

        let renderer = WboitRenderer::new(&device, wgpu::TextureFormat::Bgra8Unorm);
        assert_eq!(renderer.target_format(), wgpu::TextureFormat::Bgra8Unorm);
        assert!(renderer.buffers().is_none());
    }

    #[test]
    fn wboit_renderer_resize() {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = match pollster::block_on(
            instance.request_adapter(&wgpu::RequestAdapterOptions::default()),
        ) {
            Ok(a) => a,
            Err(_) => return,
        };
        let (device, _queue) =
            match pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())) {
                Ok(dq) => dq,
                Err(_) => return,
            };

        let mut renderer = WboitRenderer::new(&device, wgpu::TextureFormat::Bgra8Unorm);
        renderer.resize(&device, 1280, 720);
        assert!(renderer.buffers().is_some());
        assert_eq!(renderer.buffers().unwrap().width(), 1280);
        assert_eq!(renderer.buffers().unwrap().height(), 720);
    }

    #[test]
    fn wboit_renderer_resize_zero_noop() {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = match pollster::block_on(
            instance.request_adapter(&wgpu::RequestAdapterOptions::default()),
        ) {
            Ok(a) => a,
            Err(_) => return,
        };
        let (device, _queue) =
            match pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())) {
                Ok(dq) => dq,
                Err(_) => return,
            };

        let mut renderer = WboitRenderer::new(&device, wgpu::TextureFormat::Bgra8Unorm);
        renderer.resize(&device, 0, 0);
        assert!(renderer.buffers().is_none()); // zero-sized → no allocation
    }

    #[test]
    fn wboit_composite_bind_group() {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = match pollster::block_on(
            instance.request_adapter(&wgpu::RequestAdapterOptions::default()),
        ) {
            Ok(a) => a,
            Err(_) => return,
        };
        let (device, _queue) =
            match pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())) {
                Ok(dq) => dq,
                Err(_) => return,
            };

        let mut renderer = WboitRenderer::new(&device, wgpu::TextureFormat::Bgra8Unorm);
        // No buffers → None
        assert!(renderer.create_composite_bind_group(&device).is_none());
        // After resize → Some
        renderer.resize(&device, 640, 480);
        assert!(renderer.create_composite_bind_group(&device).is_some());
    }
}
