//! Read-only-depth + same-texture-sample capability probe (W.2b.2 regression guard).
//!
//! REGRESSION GUARD — re-run after any wgpu upgrade. The W.2b water pass relies on
//! binding the scene depth as a READ-ONLY depth attachment (for depth-testing the
//! water) AND sampling that same depth texture in the same pass (for the depth-delta
//! foam / refraction). That capability is wgpu-version-dependent; this probe proves
//! it still holds, with a negative control proving the harness detects the hazard:
//!
//!   ONE COPY (capability holds)  -> the water pass needs only the scene-color copy.
//!   TWO COPIES (capability lost) -> the water pass must add a second depth copy
//!                                   (see docs/campaigns/water-successor/W2B1_RECON.md).
//!
//! If this ever reports TWO COPIES after a wgpu bump, the single-copy design in
//! renderer.rs::run_water_pass must be revisited before shipping.
//!
//! Method: two render passes that both bind the depth texture in a bind group
//! (sampled via textureLoad) while it is the depth-stencil attachment:
//!   (A) NEGATIVE CONTROL — writable depth (store) + sampled  => MUST error
//!       (proves the harness actually detects the read-write hazard).
//!   (B) CANDIDATE        — read-only depth + sampled         => error or not.
//! An uncaptured-error flag captures wgpu validation errors per case.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

const SHADER: &str = r#"
@group(0) @binding(0) var depth_tex: texture_depth_2d;
struct VO { @builtin(position) pos: vec4<f32> };
@vertex fn vs(@builtin(vertex_index) vid: u32) -> VO {
    var p = array<vec2<f32>,3>(vec2<f32>(-1.0,-3.0), vec2<f32>(3.0,1.0), vec2<f32>(-1.0,1.0));
    var o: VO; o.pos = vec4<f32>(p[vid], 0.5, 1.0); return o;
}
@fragment fn fs(in: VO) -> @location(0) vec4<f32> {
    let d = textureLoad(depth_tex, vec2<i32>(in.pos.xy), 0);
    return vec4<f32>(d, d, d, 1.0);
}
"#;

fn main() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .expect("adapter");
        println!("Adapter: {} ({:?})", adapter.get_info().name, adapter.get_info().backend);
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("device");

        let err = Arc::new(AtomicBool::new(false));
        {
            let e = err.clone();
            device.on_uncaptured_error(Box::new(move |error| {
                eprintln!("  wgpu validation error: {error}");
                e.store(true, Ordering::SeqCst);
            }));
        }

        let (w, h) = (256u32, 256u32);
        let depth = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("depth"),
            size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let depth_view = depth.create_view(&wgpu::TextureViewDescriptor::default());
        let color = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("color"),
            size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let color_view = color.create_view(&wgpu::TextureViewDescriptor::default());

        // Seed depth with a write pass so read-only loads have defined contents.
        {
            let mut enc = device.create_command_encoder(&Default::default());
            {
                let _ = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("seed_depth"),
                    color_attachments: &[],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &depth_view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
            }
            queue.submit(Some(enc.finish()));
            let _ = device.poll(wgpu::PollType::Wait);
        }

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("depth_fork_shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("depth_fork_bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Depth,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            }],
        });
        let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("depth_fork_bg"),
            layout: &bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&depth_view),
            }],
        });
        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("depth_fork_pl"),
            bind_group_layouts: &[&bgl],
            push_constant_ranges: &[],
        });
        let make_pipeline = |write: bool| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("depth_fork_pipe"),
                layout: Some(&pl),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: Default::default(),
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: write,
                    depth_compare: wgpu::CompareFunction::LessEqual,
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: Default::default(),
                multiview: None,
                cache: None,
            })
        };

        // depth_ops=None  -> read-only depth attachment; Some{store} -> writable.
        let run_case = |name: &str, depth_ops: Option<wgpu::Operations<f32>>, write: bool| {
            err.store(false, Ordering::SeqCst);
            let pipeline = make_pipeline(write);
            let mut enc = device.create_command_encoder(&Default::default());
            {
                let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some(name),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &color_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &depth_view,
                        depth_ops,
                        stencil_ops: None,
                    }),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
                rp.set_pipeline(&pipeline);
                rp.set_bind_group(0, &bg, &[]);
                rp.draw(0..3, 0..1);
            }
            queue.submit(Some(enc.finish()));
            let _ = device.poll(wgpu::PollType::Wait);
            let errored = err.load(Ordering::SeqCst);
            println!(
                "  CASE {name:<26} depth={} sampled-in-pass -> {}",
                if write { "WRITABLE" } else { "read-only" },
                if errored { "VALIDATION ERROR" } else { "OK (accepted)" }
            );
            !errored
        };

        println!("\n=== depth-copy fork resolution ===");
        let ctrl_ok = run_case(
            "(A) negative control",
            Some(wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store }),
            true,
        );
        let cand_ok = run_case("(B) read-only + sampled", None, false);

        println!("\n=== VERDICT ===");
        if ctrl_ok {
            println!("INCONCLUSIVE: negative control did NOT error — harness is not detecting the\n  read-write hazard, so the candidate result can't be trusted. Default to TWO copies.");
        } else if cand_ok {
            println!("ONE COPY: read-only depth attachment + same-texture sampling is ACCEPTED by\n  this wgpu/backend. Scene-color copy is the only snapshot; depth is read-only-\n  attached and sampled directly.");
        } else {
            println!("TWO COPIES: read-only depth + sampling is REJECTED. A parallel depth copy is\n  required alongside the scene-color copy (the bandwidth line item to watch).");
        }
    });
}
