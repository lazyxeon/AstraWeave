//! Median `draw_into` time for the step-8 scene.
//!
//! This does not decide the 11 ms budget. Lavapipe time is not that budget.
//! On a GTX 1660 Ti-class GPU the printed median is the measurement.
//!
//! ```text
//! cargo run -p astraweave-render --release --features gpu-frame-budget --bin gpu_frame_budget -- --scene heavy --debug-timestamps --glb "assets/The Complete KayKit Collection v4/KayKit Adventurers 2.0/Characters/gltf/Rogue.glb"
//! ```

use anyhow::Context;
use std::collections::BTreeMap;
use std::future::Future;
use std::path::PathBuf;
use std::sync::mpsc;
use std::task::Wake;
use std::time::{Duration, Instant};

use astraweave_camera::{Projection, RenderView};
use astraweave_render::gpu_particles::EmitterParams;
use astraweave_render::mesh_gltf::{load_gltf, GltfOptions};
use astraweave_render::{Instance, Renderer};
use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;

const GPU_EXEC_MAGIC: u32 = astraweave_render::gpu_profiler::FRAME_EXEC_MAGIC;

const WIDTH: u32 = 1920;
const HEIGHT: u32 = 1080;
const WARMUP: usize = 30;
const SAMPLES: usize = 120;

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    let glb = glb_path();
    let heavy = scene_is_heavy();
    let debug_timestamps = debug_timestamps();
    let fresh_query_set = fresh_query_set();
    if !glb.is_file() {
        anyhow::bail!(
            "textured GLB not found at {}. Pass --glb <path>. No frame time is invented.",
            glb.display()
        );
    }

    let mut renderer = pollster::block_on(Renderer::new_headless(WIDTH, HEIGHT))?;
    // Replaces the log-only handler installed by `new_headless`. Without a
    // logger that handler was silent, and an invalid command buffer was dropped.
    renderer.device().on_uncaptured_error(Box::new(|error| {
        eprintln!("uncaptured_wgpu_error={error}");
        std::process::exit(1);
    }));
    if let Some(err) = astraweave_render::gpu_fault::take() {
        println!("init_wgpu_error={err}");
    } else {
        println!("init_wgpu_error=none");
    }
    println!("adapter={}", renderer.gpu_adapter_name());
    println!("backend={}", renderer.gpu_backend());
    println!(
        "wgpu_backend_env={}",
        std::env::var("WGPU_BACKEND").unwrap_or_else(|_| "unset".to_string())
    );
    println!("features={:?}", renderer.gpu_features());
    print_device_limits(renderer.device());
    print_adapter_scan(renderer.device());
    println!(
        "fresh_query_set={}",
        if fresh_query_set { "on" } else { "off" }
    );
    if let Some(profiler) = renderer.gpu_profiler() {
        println!("timestamp_period_ns={}", profiler.timestamp_period_ns());
    }
    let timestamps = renderer.timestamps_supported();
    if timestamps
        && renderer
            .gpu_features()
            .contains(wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS)
    {
        renderer
            .device()
            .push_error_scope(wgpu::ErrorFilter::Validation);
        renderer
            .device()
            .push_error_scope(wgpu::ErrorFilter::OutOfMemory);
        match astraweave_render::GpuProfiler::prove_readback(renderer.device(), renderer.queue()) {
            Ok(report) => {
                let (validation, oom) = pop_frame_scopes(renderer.device())?;
                println!(
                    "self_test_scope validation={} out_of_memory={}",
                    scope_text(&validation),
                    scope_text(&oom)
                );
                if validation.is_some() || oom.is_some() {
                    anyhow::bail!("timestamp self-test validation\n{report}");
                }
                println!("timestamp_self_test=ok");
                println!("{report}");
            }
            Err(err) => {
                let (validation, oom) = pop_frame_scopes(renderer.device())?;
                anyhow::bail!(
                    "timestamp self-test failed before the benchmark.\n{err}\nvalidation={}\nout_of_memory={}",
                    scope_text(&validation),
                    scope_text(&oom)
                );
            }
        }
    } else if timestamps {
        println!("timestamp_self_test=skipped missing TIMESTAMP_QUERY_INSIDE_ENCODERS");
    }
    let timestamps = renderer.timestamps_supported();
    if !timestamps {
        eprintln!(
            "GPU timestamps are unsupported on this adapter (need TIMESTAMP_QUERY and a positive timestamp period)."
        );
    }
    let hdri = renderer.bake_showcase_environment(astraweave_render::ibl::IblQuality::Medium)?;
    let meshes = load_gltf(&glb, &GltfOptions::default())?;
    if meshes.is_empty() {
        anyhow::bail!("{} produced no meshes", glb.display());
    }
    let instances = if heavy {
        heavy_instances()
    } else {
        vec![Instance::from_pos_scale_color(
            Vec3::ZERO,
            Vec3::ONE,
            [1.0, 1.0, 1.0, 1.0],
        )]
    };
    if instances.len() <= 1 {
        renderer.add_composite_model("budget-model", meshes, &instances);
    } else {
        renderer.add_composite_model("budget-0", meshes, &instances[..1]);
        for (i, instance) in instances.iter().enumerate().skip(1) {
            if !renderer.duplicate_model("budget-0", format!("budget-{i}"), instance.clone()) {
                anyhow::bail!("failed to place heavy model {i}");
            }
        }
    }
    renderer.set_shadows_enabled(true);
    add_heavy_terrain(&mut renderer)?;
    renderer.set_particle_emitter(Some(emitter()))?;
    // Cloud shadows stay at the renderer default. Volumetric fog and god rays
    // are not called. GPU indirect culling is not the draw path yet: each
    // model still binds group 3, so the CPU frustum remains the filter.
    let gpu_cull = "cpu_fallback";

    let device = renderer.device().clone();
    let exec_src = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("frame exec sentinel src"),
        contents: &GPU_EXEC_MAGIC.to_le_bytes(),
        usage: wgpu::BufferUsages::COPY_SRC,
    });
    let exec_dst = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("frame exec sentinel dst"),
        size: 4,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let color = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("frame budget color"),
        size: wgpu::Extent3d {
            width: WIDTH,
            height: HEIGHT,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: renderer.surface_format(),
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = color.create_view(&wgpu::TextureViewDescriptor::default());
    let camera = fixed_camera();

    let frame = |renderer: &mut Renderer| -> anyhow::Result<FrameTimes> {
        device.push_error_scope(wgpu::ErrorFilter::Validation);
        device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
        let recorded = (|| -> anyhow::Result<FrameTimes> {
            if fresh_query_set {
                renderer.recreate_timestamp_set()?;
            }
            renderer.update_view(&camera);
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame budget"),
            });
            let encode_started = Instant::now();
            renderer.draw_into(&view, None, &mut encoder)?;
            let encode_ms = encode_started.elapsed().as_secs_f64() * 1_000.0;
            // Same encoder as the timestamp writes. If wgpu drops this buffer
            // at finish or submit, the destination stays 0.
            encoder.copy_buffer_to_buffer(&exec_src, 0, &exec_dst, 0, 4);
            let frame_started = Instant::now();
            renderer.submit_timestamped(encoder.finish())?;
            let (write_wait_ms, copy_wait_ms) = renderer
                .gpu_profiler()
                .map(|profiler| {
                    (
                        f64::from(profiler.write_wait_ms()),
                        f64::from(profiler.copy_wait_ms()),
                    )
                })
                .unwrap_or((0.0, 0.0));
            if timestamps {
                renderer.request_gpu_profiler_readback();
            }
            let readback_started = Instant::now();
            astraweave_render::gpu_profiler::wait_for_gpu(&device, "frame gpu")
                .map_err(anyhow::Error::msg)?;
            if timestamps {
                if let Some(profiler) = renderer.gpu_profiler_mut() {
                    profiler
                        .block_on_readback(&device)
                        .map_err(anyhow::Error::msg)?;
                }
            }
            let readback_ms = readback_started.elapsed().as_secs_f64() * 1_000.0;
            if debug_timestamps {
                if let Some(profiler) = renderer.gpu_profiler() {
                    println!("{}", profiler.submission_report());
                }
            }
            // Record time plus the waits inside submit and the map. Those waits
            // block on the GPU that just ran the frame. They are not extra GPU work.
            let cpu_frame_ms = frame_started.elapsed().as_secs_f64() * 1_000.0 + encode_ms;
            Ok(FrameTimes {
                encode_ms,
                cpu_frame_ms,
                write_wait_ms,
                copy_wait_ms,
                readback_ms,
            })
        })();
        let (validation, oom) = pop_frame_scopes(&device)?;
        println!(
            "frame_scope validation={} out_of_memory={}",
            scope_text(&validation),
            scope_text(&oom)
        );
        let exec = read_exec_word(&device, &exec_dst);
        match &exec {
            Ok(word) => println!("gpu_exec=0x{word:08X}"),
            Err(err) => println!("gpu_exec=err {err}"),
        }
        if validation.is_some() || oom.is_some() {
            anyhow::bail!("frame command buffer was rejected");
        }
        let word = exec.map_err(anyhow::Error::msg)?;
        if word != GPU_EXEC_MAGIC {
            anyhow::bail!(
                "frame command buffer did not run: gpu_exec=0x{word:08X}, expected 0x{GPU_EXEC_MAGIC:08X}"
            );
        }
        recorded
    };

    for i in 0..WARMUP {
        if debug_timestamps {
            println!("warmup {}/{WARMUP}", i + 1);
        }
        frame(&mut renderer)?;
    }
    let mut encode = Vec::with_capacity(SAMPLES);
    let mut cpu_frame = Vec::with_capacity(SAMPLES);
    let mut write_wait = Vec::with_capacity(SAMPLES);
    let mut copy_wait = Vec::with_capacity(SAMPLES);
    let mut readback = Vec::with_capacity(SAMPLES);
    let mut gpu = Vec::new();
    let mut gpu_frame = Vec::new();
    let mut pass_samples: BTreeMap<String, Vec<f64>> = BTreeMap::new();
    let mut empty_passes = String::new();
    let mut gpu_error = None;
    for sample_index in 0..SAMPLES {
        if debug_timestamps {
            println!("sample {}/{SAMPLES}", sample_index + 1);
        }
        let times = frame(&mut renderer)?;
        encode.push(times.encode_ms);
        cpu_frame.push(times.cpu_frame_ms);
        write_wait.push(times.write_wait_ms);
        copy_wait.push(times.copy_wait_ms);
        readback.push(times.readback_ms);
        if !timestamps {
            continue;
        }
        let profiler = renderer
            .gpu_profiler()
            .context("GPU profiler disappeared after a supported device")?;
        match profiler.validated_stamp() {
            Ok(summary) => {
                if debug_timestamps {
                    println!("sample={sample_index}\n{}", summary.report);
                }
                gpu.push(f64::from(summary.pass_sum_ms));
                if let Some(ms) = summary.frame_ms {
                    gpu_frame.push(f64::from(ms));
                }
                for timing in profiler.results() {
                    pass_samples
                        .entry(timing.name.clone())
                        .or_default()
                        .push(f64::from(timing.duration_ms));
                }
                if empty_passes.is_empty() && !summary.empty.is_empty() {
                    empty_passes = summary.empty.join(",");
                }
            }
            Err(report) => {
                let trace = renderer
                    .gpu_profiler()
                    .map(|profiler| profiler.submission_report())
                    .unwrap_or_default();
                println!("sample={sample_index}\n{report}\n{trace}");
                gpu_error = Some(report);
                break;
            }
        }
    }

    let scene = if heavy { "heavy" } else { "light" };
    println!("scene={scene} resolution=1920x1080 warmup={WARMUP} samples={SAMPLES}");
    println!("hdri={}", hdri.display());
    println!("glb={}", glb.display());
    println!(
        "instances={} rendered_models={} particles=on shadows=on volumetrics=off gpu_cull={gpu_cull} water_drawn={}",
        instances.len(),
        renderer.rendered_model_count(),
        if renderer.water_is_drawn() { "yes" } else { "no" }
    );
    println!("median_draw_into_ms={:.3}", median(&mut encode));
    println!("median_cpu_frame_ms={:.3}", median(&mut cpu_frame));
    println!("median_cpu_write_wait_ms={:.3}", median(&mut write_wait));
    println!("median_cpu_copy_wait_ms={:.3}", median(&mut copy_wait));
    println!("median_cpu_readback_ms={:.3}", median(&mut readback));
    println!("cpu_frame_includes=draw_into_record+gpu_write_wait+timestamp_copy_wait+readback_map");
    println!("cpu_frame_waits_for_gpu=yes");
    println!("cluster_bin=stamped_only_when_light_bins_change");
    if !timestamps {
        anyhow::bail!(
            "GPU timestamps are unsupported. median_cpu_frame_ms is the CPU fallback and is not the 11 ms GPU budget."
        );
    }
    if let Some(report) = gpu_error {
        anyhow::bail!(
            "GPU timestamp read failed. median_cpu_frame_ms is the CPU fallback.\n{report}"
        );
    }
    let gpu_median = median(&mut gpu);
    println!(
        "median_gpu_pass_sum_ms={:.3} gpu_samples={} empty={empty_passes}",
        gpu_median,
        gpu.len()
    );
    if !gpu_frame.is_empty() {
        let frame_median = median(&mut gpu_frame);
        println!(
            "median_gpu_frame_ms={:.3} frame_samples={}",
            frame_median,
            gpu_frame.len()
        );
        println!(
            "median_gpu_frame_minus_pass_sum_ms={:.3}",
            frame_median - gpu_median
        );
    }
    for (name, samples) in &mut pass_samples {
        println!(
            "median_pass {name}={:.3} n={}",
            median(samples),
            samples.len()
        );
    }
    let frame_positive = gpu_frame.iter().any(|ms| *ms > 0.0);
    if gpu_median.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) && !frame_positive {
        anyhow::bail!("no positive GPU duration. median_cpu_frame_ms is the CPU fallback.");
    }
    println!("budget_basis=gpu_frame");
    println!(
        "budget_basis_note=median_gpu_frame_ms is the clock. median_cpu_frame_ms waits for that GPU submission, so it is not the budget and not max(cpu, gpu)."
    );
    println!("budget_11ms=not_decided");
    Ok(())
}

struct FrameTimes {
    encode_ms: f64,
    cpu_frame_ms: f64,
    write_wait_ms: f64,
    copy_wait_ms: f64,
    readback_ms: f64,
}

struct ScopeWaker;

impl Wake for ScopeWaker {
    fn wake(self: std::sync::Arc<Self>) {}
}

fn scope_text(value: &Option<String>) -> &str {
    value.as_deref().unwrap_or("none")
}

fn pop_frame_scopes(device: &wgpu::Device) -> anyhow::Result<(Option<String>, Option<String>)> {
    let oom = pop_one_scope(device, "OutOfMemory");
    let validation = pop_one_scope(device, "Validation");
    match (validation, oom) {
        (Ok(validation), Ok(oom)) => Ok((validation, oom)),
        (validation, oom) => anyhow::bail!(
            "error scope pop failed validation={} out_of_memory={}",
            pop_failure(&validation),
            pop_failure(&oom)
        ),
    }
}

fn pop_failure(result: &Result<Option<String>, String>) -> String {
    match result {
        Ok(Some(err)) => err.clone(),
        Ok(None) => "none".to_string(),
        Err(err) => err.clone(),
    }
}

fn pop_one_scope(device: &wgpu::Device, label: &str) -> Result<Option<String>, String> {
    let waker = std::task::Waker::from(std::sync::Arc::new(ScopeWaker));
    let mut future = std::pin::pin!(device.pop_error_scope());
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let mut context = std::task::Context::from_waker(&waker);
        match future.as_mut().poll(&mut context) {
            std::task::Poll::Ready(Some(err)) => return Ok(Some(err.to_string())),
            std::task::Poll::Ready(None) => return Ok(None),
            std::task::Poll::Pending => {
                device
                    .poll(wgpu::PollType::Poll)
                    .map_err(|err| format!("device.poll failed while popping {label}: {err}"))?;
                if Instant::now() >= deadline {
                    return Err(format!("error scope {label} did not resolve within 5s"));
                }
                std::thread::sleep(Duration::from_millis(1));
            }
        }
    }
}

fn read_exec_word(device: &wgpu::Device, buffer: &wgpu::Buffer) -> Result<u32, String> {
    let (sender, receiver) = mpsc::channel();
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result.map_err(|err| err.to_string()));
        });
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        device
            .poll(wgpu::PollType::Poll)
            .map_err(|err| format!("device.poll failed while mapping the exec sentinel: {err}"))?;
        match receiver.try_recv() {
            Ok(Ok(())) => break,
            Ok(Err(err)) => return Err(err),
            Err(mpsc::TryRecvError::Empty) => {
                if Instant::now() >= deadline {
                    return Err("frame exec map timed out after 5s".to_string());
                }
                std::thread::sleep(Duration::from_millis(1));
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                return Err("frame exec map closed before the callback".to_string());
            }
        }
    }
    let data = buffer.slice(..).get_mapped_range();
    if data.len() < 4 {
        let len = data.len();
        drop(data);
        buffer.unmap();
        return Err(format!("frame exec map is {len} bytes"));
    }
    let word = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    drop(data);
    buffer.unmap();
    Ok(word)
}

fn print_device_limits(device: &wgpu::Device) {
    let limits = device.limits();
    println!(
        "device_limits bind_groups={} bindings_per_group={} sampled_textures={} samplers={} storage_buffers={} storage_textures={} uniform_buffers={} uniform_binding={} storage_binding={} buffer_bytes={} tex2d={} color_attachments={} binding_array={} vertex_buffers={} vertex_attributes={} inter_stage={}",
        limits.max_bind_groups,
        limits.max_bindings_per_bind_group,
        limits.max_sampled_textures_per_shader_stage,
        limits.max_samplers_per_shader_stage,
        limits.max_storage_buffers_per_shader_stage,
        limits.max_storage_textures_per_shader_stage,
        limits.max_uniform_buffers_per_shader_stage,
        limits.max_uniform_buffer_binding_size,
        limits.max_storage_buffer_binding_size,
        limits.max_buffer_size,
        limits.max_texture_dimension_2d,
        limits.max_color_attachments,
        limits.max_binding_array_elements_per_shader_stage,
        limits.max_vertex_buffers,
        limits.max_vertex_attributes,
        limits.max_inter_stage_shader_components,
    );
    println!("renderer_requests max_bind_groups=8");
}

struct AdapterDep {
    backend: String,
    name: String,
    fields: Vec<(String, String)>,
}

fn print_adapter_scan(device: &wgpu::Device) {
    println!("adapter_scan=vulkan,dx12");
    let mut found = Vec::new();
    for (label, backends) in [
        ("vulkan", wgpu::Backends::VULKAN),
        ("dx12", wgpu::Backends::DX12),
    ] {
        let mut descriptor = wgpu::InstanceDescriptor::from_env_or_default();
        descriptor.backends = backends;
        let instance = wgpu::Instance::new(&descriptor);
        let mut adapters = instance.enumerate_adapters(backends);
        if adapters.is_empty() {
            if let Ok(adapter) =
                pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::HighPerformance,
                    compatible_surface: None,
                    force_fallback_adapter: false,
                }))
            {
                adapters.push(adapter);
            }
        }
        if adapters.is_empty() {
            println!("adapter_dep backend={label} status=none");
            continue;
        }
        for adapter in adapters {
            let dep = adapter_dep(label, &adapter);
            for (key, value) in &dep.fields {
                println!(
                    "adapter_dep backend={} name=\"{}\" {key}={value}",
                    dep.backend, dep.name
                );
            }
            found.push(dep);
        }
    }
    if found.len() < 2 {
        println!("adapter_diff=single");
    } else {
        diff_adapters(&found);
    }
    let granted = renderer_limits(&device.limits());
    for dep in &found {
        for (name, granted_value) in granted {
            let key = format!("limit_{name}");
            let Some((_, adapter_value)) = dep.fields.iter().find(|(field, _)| field == &key)
            else {
                continue;
            };
            if adapter_value != &granted_value.to_string() {
                println!(
                    "granted_vs_adapter backend={} name=\"{}\" {name} device={granted_value} adapter={adapter_value}",
                    dep.backend, dep.name
                );
            }
        }
    }
}

fn diff_adapters(found: &[AdapterDep]) {
    for pair in found.windows(2) {
        let left = &pair[0];
        let right = &pair[1];
        let mut differed = false;
        for (key, left_value) in &left.fields {
            let Some(right_value) = right.fields.iter().find(|(name, _)| name == key) else {
                continue;
            };
            if left_value != &right_value.1 {
                differed = true;
                println!(
                    "adapter_diff {} \"{}\" vs {} \"{}\" {key} {} {}",
                    left.backend, left.name, right.backend, right.name, left_value, right_value.1
                );
            }
        }
        if !differed {
            println!(
                "adapter_diff {} \"{}\" vs {} \"{}\" renderer_dependent=same",
                left.backend, left.name, right.backend, right.name
            );
        }
    }
}

fn adapter_dep(backend: &str, adapter: &wgpu::Adapter) -> AdapterDep {
    let info = adapter.get_info();
    let features = adapter.features();
    let limits = adapter.limits();
    let mut fields = Vec::new();
    for (name, feature) in RENDERER_FEATURES {
        fields.push((
            format!("feature_{name}"),
            if features.contains(*feature) {
                "yes".to_string()
            } else {
                "no".to_string()
            },
        ));
    }
    fields.push((
        "bindless".to_string(),
        if astraweave_render::material_bindless::supports_bindless(adapter) {
            "yes".to_string()
        } else {
            "no".to_string()
        },
    ));
    for (name, value) in renderer_limits(&limits) {
        fields.push((format!("limit_{name}"), value.to_string()));
    }
    for (name, format) in [
        ("Rgba8UnormSrgb", wgpu::TextureFormat::Rgba8UnormSrgb),
        ("Depth32Float", wgpu::TextureFormat::Depth32Float),
        ("Rgba16Float", wgpu::TextureFormat::Rgba16Float),
    ] {
        let format_features = adapter.get_texture_format_features(format);
        fields.push((
            format!("format_{name}"),
            format!(
                "{:?}/{:?}",
                format_features.allowed_usages, format_features.flags
            ),
        ));
    }
    fields.push((
        "downlevel".to_string(),
        format!("{:?}", adapter.get_downlevel_capabilities().flags),
    ));
    AdapterDep {
        backend: backend.to_string(),
        name: info.name,
        fields,
    }
}

const RENDERER_FEATURES: &[(&str, wgpu::Features)] = &[
    ("TIMESTAMP_QUERY", wgpu::Features::TIMESTAMP_QUERY),
    (
        "TIMESTAMP_QUERY_INSIDE_PASSES",
        wgpu::Features::TIMESTAMP_QUERY_INSIDE_PASSES,
    ),
    (
        "TIMESTAMP_QUERY_INSIDE_ENCODERS",
        wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS,
    ),
    ("PIPELINE_CACHE", wgpu::Features::PIPELINE_CACHE),
    (
        "TEXTURE_BINDING_ARRAY",
        wgpu::Features::TEXTURE_BINDING_ARRAY,
    ),
    (
        "NON_UNIFORM_INDEXING",
        wgpu::Features::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING,
    ),
    (
        "TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES",
        wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES,
    ),
];

fn renderer_limits(limits: &wgpu::Limits) -> [(&'static str, u64); 15] {
    [
        ("max_bind_groups", u64::from(limits.max_bind_groups)),
        (
            "max_bindings_per_bind_group",
            u64::from(limits.max_bindings_per_bind_group),
        ),
        (
            "max_sampled_textures_per_shader_stage",
            u64::from(limits.max_sampled_textures_per_shader_stage),
        ),
        (
            "max_samplers_per_shader_stage",
            u64::from(limits.max_samplers_per_shader_stage),
        ),
        (
            "max_storage_buffers_per_shader_stage",
            u64::from(limits.max_storage_buffers_per_shader_stage),
        ),
        (
            "max_storage_textures_per_shader_stage",
            u64::from(limits.max_storage_textures_per_shader_stage),
        ),
        (
            "max_uniform_buffers_per_shader_stage",
            u64::from(limits.max_uniform_buffers_per_shader_stage),
        ),
        (
            "max_uniform_buffer_binding_size",
            u64::from(limits.max_uniform_buffer_binding_size),
        ),
        (
            "max_storage_buffer_binding_size",
            u64::from(limits.max_storage_buffer_binding_size),
        ),
        ("max_buffer_size", limits.max_buffer_size),
        (
            "max_texture_dimension_2d",
            u64::from(limits.max_texture_dimension_2d),
        ),
        (
            "max_color_attachments",
            u64::from(limits.max_color_attachments),
        ),
        (
            "max_binding_array_elements_per_shader_stage",
            u64::from(limits.max_binding_array_elements_per_shader_stage),
        ),
        ("max_vertex_buffers", u64::from(limits.max_vertex_buffers)),
        (
            "max_vertex_attributes",
            u64::from(limits.max_vertex_attributes),
        ),
    ]
}

fn debug_timestamps() -> bool {
    std::env::args().any(|arg| arg == "--debug-timestamps" || arg == "--debug-timestamps=true")
}

fn fresh_query_set() -> bool {
    std::env::args().any(|arg| arg == "--fresh-query-set" || arg == "--fresh-query-set=true")
}

fn scene_is_heavy() -> bool {
    std::env::args().any(|arg| arg == "--scene=heavy" || arg == "heavy")
        || std::env::args()
            .collect::<Vec<_>>()
            .windows(2)
            .any(|pair| pair[0] == "--scene" && pair[1] == "heavy")
}

fn heavy_instances() -> Vec<Instance> {
    let mut instances = Vec::with_capacity(64);
    for z in 0..8 {
        for x in 0..8 {
            instances.push(Instance::from_pos_scale_color(
                Vec3::new((x as f32 - 3.5) * 4.0, 0.0, (z as f32 - 3.5) * 4.0),
                Vec3::ONE,
                [1.0, 1.0, 1.0, 1.0],
            ));
        }
    }
    instances
}

#[cfg(feature = "terrain-splat-arrays")]
fn add_heavy_terrain(renderer: &mut Renderer) -> anyhow::Result<()> {
    if !scene_is_heavy() {
        return Ok(());
    }
    renderer.init_terrain_forward()?;
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let n = 8u32;
    for z in 0..=n {
        for x in 0..=n {
            vertices.push(
                astraweave_render::terrain_material_manager::TerrainSplatVertex {
                    position: [
                        (x as f32 / n as f32) * 64.0 - 32.0,
                        0.0,
                        (z as f32 / n as f32) * 64.0 - 32.0,
                    ],
                    normal: [0.0, 1.0, 0.0],
                    uv: [x as f32 / n as f32, z as f32 / n as f32],
                },
            );
        }
    }
    for z in 0..n {
        for x in 0..n {
            let i = z * (n + 1) + x;
            indices.extend([i, i + n + 1, i + 1, i + 1, i + n + 1, i + n + 2]);
        }
    }
    let texel = [255u8, 0, 0, 255];
    let map = texel.repeat(16);
    let maps = vec![map; 8];
    let refs: Vec<&[u8]> = maps.iter().map(|bytes| bytes.as_slice()).collect();
    renderer.upload_terrain_chunk(1, &vertices, &indices, &refs, (4, 4))?;
    Ok(())
}

#[cfg(not(feature = "terrain-splat-arrays"))]
fn add_heavy_terrain(_renderer: &mut Renderer) -> anyhow::Result<()> {
    Ok(())
}

fn glb_path() -> PathBuf {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--glb" {
            if let Some(path) = args.next() {
                return PathBuf::from(path);
            }
        } else if let Some(path) = arg.strip_prefix("--glb=") {
            return PathBuf::from(path);
        }
    }
    PathBuf::from(
        "assets/The Complete KayKit Collection v4/KayKit Adventurers 2.0/Characters/gltf/Rogue.glb",
    )
}

fn emitter() -> EmitterParams {
    EmitterParams {
        position: [0.0, 1.0, 0.0, 1.0],
        velocity: [0.0, 1.0, 0.0, 0.0],
        emission_rate: 8.0,
        lifetime: 2.0,
        velocity_randomness: 0.25,
        delta_time: 1.0 / 60.0,
        gravity: [0.0, -9.8, 0.0, 0.0],
        particle_count: 32,
        max_particles: 256,
        random_seed: 7,
        _padding: 0,
    }
}

fn fixed_camera() -> RenderView {
    let eye = Vec3::new(4.0, 2.0, 4.0);
    let view = Mat4::look_at_rh(eye, Vec3::ZERO, Vec3::Y);
    let projection = Projection::perspective(
        60_f32.to_radians(),
        WIDTH as f32 / HEIGHT as f32,
        0.1,
        200.0,
    );
    RenderView::new(
        view,
        &projection,
        eye,
        Vec3::new(-1.0, -0.5, -1.0).normalize(),
    )
}

fn median(values: &mut [f64]) -> f64 {
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = values.len() / 2;
    if values.len() % 2 == 0 {
        (values[mid - 1] + values[mid]) * 0.5
    } else {
        values[mid]
    }
}
