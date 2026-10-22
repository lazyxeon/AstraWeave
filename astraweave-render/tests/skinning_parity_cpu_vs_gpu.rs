//! CPU/GPU Parity Tests for Skeletal Animation
//!
//! Phase 2 Task 5 (Phase E): Validates that CPU and GPU skinning produce
//! equivalent results within floating-point tolerance.
//!
//! **NOTE**: These tests require a GPU context and are marked `#[ignore]` for CI.
//! Run locally with: `cargo test -p astraweave-render --tests --features skinning-gpu -- --ignored`

#[cfg(target_os = "linux")]
#[path = "support/linux_test_threads.rs"]
mod linux_test_threads;

use astraweave_render::animation::*;
#[cfg(feature = "skinning-gpu")]
use astraweave_render::skinning_gpu::{JointPaletteManager, SKINNING_GPU_SHADER};
use glam::{Mat4, Quat, Vec3};
#[cfg(feature = "skinning-gpu")]
use wgpu::util::DeviceExt;

/// Create a test skeleton for parity validation
fn create_parity_test_skeleton() -> Skeleton {
    Skeleton {
        root_indices: vec![0],
        joints: vec![
            Joint {
                name: "root".to_string(),
                parent_index: None,
                inverse_bind_matrix: Mat4::IDENTITY,
                local_transform: Transform::default(),
            },
            Joint {
                name: "spine".to_string(),
                parent_index: Some(0),
                inverse_bind_matrix: Mat4::from_translation(Vec3::new(0.0, -1.0, 0.0)),
                local_transform: Transform {
                    translation: Vec3::new(0.0, 1.0, 0.0),
                    ..Default::default()
                },
            },
            Joint {
                name: "shoulder".to_string(),
                parent_index: Some(1),
                inverse_bind_matrix: Mat4::from_translation(Vec3::new(0.0, -2.0, 0.0)),
                local_transform: Transform {
                    translation: Vec3::new(0.0, 1.0, 0.0),
                    ..Default::default()
                },
            },
        ],
    }
}

/// Create test animation with rotation
fn create_parity_animation() -> AnimationClip {
    AnimationClip {
        name: "test_anim".to_string(),
        duration: 1.0,
        channels: vec![AnimationChannel {
            target_joint_index: 1, // Animate spine
            times: vec![0.0, 0.5, 1.0],
            interpolation: Interpolation::Linear,
            data: ChannelData::Rotation(vec![
                Quat::IDENTITY,
                Quat::from_rotation_z(std::f32::consts::FRAC_PI_4), // 45°
                Quat::from_rotation_z(std::f32::consts::FRAC_PI_2), // 90°
            ]),
        }],
    }
}

/// Test vertices spanning all joints
fn create_test_vertices() -> Vec<(Vec3, Vec3, [u16; 4], [f32; 4])> {
    vec![
        // Vertex 0: 100% root joint
        (
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::Y,
            [0, 0, 0, 0],
            [1.0, 0.0, 0.0, 0.0],
        ),
        // Vertex 1: 100% spine joint
        (
            Vec3::new(0.0, 1.5, 0.0),
            Vec3::Y,
            [1, 0, 0, 0],
            [1.0, 0.0, 0.0, 0.0],
        ),
        // Vertex 2: 50/50 blend between spine and shoulder
        (
            Vec3::new(0.0, 2.0, 0.0),
            Vec3::Y,
            [1, 2, 0, 0],
            [0.5, 0.5, 0.0, 0.0],
        ),
        // Vertex 3: 100% shoulder joint
        (
            Vec3::new(0.0, 2.5, 0.0),
            Vec3::Y,
            [2, 0, 0, 0],
            [1.0, 0.0, 0.0, 0.0],
        ),
    ]
}

/// CPU skinning reference implementation
fn skin_vertices_cpu(
    vertices: &[(Vec3, Vec3, [u16; 4], [f32; 4])],
    joint_matrices: &[Mat4],
) -> Vec<(Vec3, Vec3)> {
    vertices
        .iter()
        .map(|(pos, normal, joints, weights)| {
            skin_vertex_cpu(*pos, *normal, *joints, *weights, joint_matrices)
        })
        .collect()
}

/// Compare two vertex arrays with tolerance
#[allow(dead_code)]
fn assert_vertices_close(
    cpu_verts: &[(Vec3, Vec3)],
    gpu_verts: &[(Vec3, Vec3)],
    pos_tolerance: f32,
    normal_tolerance: f32,
) {
    assert_eq!(cpu_verts.len(), gpu_verts.len(), "Vertex count mismatch");

    let mut max_pos_diff = 0.0f32;
    let mut max_normal_diff = 0.0f32;
    let mut pos_diff_vertex = 0;
    let mut normal_diff_vertex = 0;

    for (i, ((cpu_pos, cpu_norm), (gpu_pos, gpu_norm))) in
        cpu_verts.iter().zip(gpu_verts.iter()).enumerate()
    {
        let pos_diff = (*cpu_pos - *gpu_pos).length();
        let normal_diff = (*cpu_norm - *gpu_norm).length();

        if pos_diff > max_pos_diff {
            max_pos_diff = pos_diff;
            pos_diff_vertex = i;
        }
        if normal_diff > max_normal_diff {
            max_normal_diff = normal_diff;
            normal_diff_vertex = i;
        }

        assert!(
            pos_diff < pos_tolerance,
            "Vertex {} position diff {} exceeds tolerance {}. CPU: {:?}, GPU: {:?}",
            i,
            pos_diff,
            pos_tolerance,
            cpu_pos,
            gpu_pos
        );

        assert!(
            normal_diff < normal_tolerance,
            "Vertex {} normal diff {} exceeds tolerance {}. CPU: {:?}, GPU: {:?}",
            i,
            normal_diff,
            normal_tolerance,
            cpu_norm,
            gpu_norm
        );
    }

    eprintln!(
        "✅ Parity verified: {} vertices within tolerance",
        cpu_verts.len()
    );
    eprintln!(
        "   Max position diff: {:.6} at vertex {} (tolerance: {})",
        max_pos_diff, pos_diff_vertex, pos_tolerance
    );
    eprintln!(
        "   Max normal diff: {:.6} at vertex {} (tolerance: {})",
        max_normal_diff, normal_diff_vertex, normal_tolerance
    );
}

/// Helper to run GPU skinning via compute shader for parity testing
#[cfg(feature = "skinning-gpu")]
async fn run_gpu_skinning(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    vertices: &[(Vec3, Vec3, [u16; 4], [f32; 4])],
    joint_matrices: &[Mat4],
) -> Vec<(Vec3, Vec3)> {
    // 1. Create Joint Palette
    let mut palette_manager = JointPaletteManager::new(device, queue);
    let handle = palette_manager.allocate();
    palette_manager
        .upload_matrices(handle, joint_matrices)
        .unwrap();
    let palette_bind_group = palette_manager.get_bind_group(handle).unwrap();

    // 2. Create Vertex Buffer
    #[repr(C)]
    #[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
    struct GpuVertex {
        pos: [f32; 3],
        _pad1: f32, // alignment
        normal: [f32; 3],
        _pad2: f32,
        tangent: [f32; 4],
        joints: [u32; 4],
        weights: [f32; 4],
    }

    let gpu_vertices: Vec<GpuVertex> = vertices
        .iter()
        .map(|(p, n, j, w)| GpuVertex {
            pos: p.to_array(),
            _pad1: 0.0,
            normal: n.to_array(),
            _pad2: 0.0,
            tangent: [0.0; 4],
            joints: [j[0] as u32, j[1] as u32, j[2] as u32, j[3] as u32],
            weights: *w,
        })
        .collect();

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Input Vertex Buffer"),
        contents: bytemuck::cast_slice(&gpu_vertices),
        usage: wgpu::BufferUsages::STORAGE,
    });

    // 3. Create Output Buffer
    #[repr(C)]
    #[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
    struct GpuOutput {
        pos: [f32; 4],
        normal: [f32; 4],
    }

    let output_size = (std::mem::size_of::<GpuOutput>() * vertices.len()) as u64;
    let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Output Buffer"),
        size: output_size,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });

    // 4. Create Compute Shader
    let compute_src = format!(
        "{}\n{}",
        SKINNING_GPU_SHADER.replace("@group(4)", "@group(1)"),
        r#"
        struct VertexInput {
            pos: vec3<f32>,
            _pad1: f32,
            normal: vec3<f32>,
            _pad2: f32,
            tangent: vec4<f32>,
            joints: vec4<u32>,
            weights: vec4<f32>,
        }
        
        struct VertexBuffer {
            vertices: array<VertexInput>,
        }

        struct OutputItem {
            pos: vec4<f32>,
            normal: vec4<f32>,
        }

        struct OutputBuffer {
            items: array<OutputItem>,
        }

        @group(0) @binding(0) var<storage, read> input_buffer: VertexBuffer;
        @group(0) @binding(1) var<storage, read_write> output_buffer: OutputBuffer;

        @compute @workgroup_size(64)
        fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
            let index = global_id.x;
            if (index >= arrayLength(&input_buffer.vertices)) {
                return;
            }
            
            let v_in = input_buffer.vertices[index];
            
            // Map to SkinnedVertexInput expected by library function
            var input: SkinnedVertexInput;
            input.position = v_in.pos;
            input.normal = v_in.normal;
            input.tangent = v_in.tangent;
            input.joints = v_in.joints;
            input.weights = v_in.weights;
            
            let pos = apply_skinning(input);
            let nrm = apply_skinning_normal(input);
            
            output_buffer.items[index].pos = pos;
            output_buffer.items[index].normal = vec4<f32>(nrm, 0.0);
        }
        "#
    );

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Skinning Compute Shader"),
        source: wgpu::ShaderSource::Wgsl(compute_src.into()),
    });

    // 5. Create Pipeline
    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Compute Bind Group Layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Compute Pipeline Layout"),
        bind_group_layouts: &[&bind_group_layout, &palette_manager.bind_group_layout],
        push_constant_ranges: &[],
    });

    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Skinning Pipeline"),
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Compute Bind Group"),
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: vertex_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: output_buffer.as_entire_binding(),
            },
        ],
    });

    // 6. Dispatch
    let mut encoder =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    {
        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Skinning Pass"),
            timestamp_writes: None,
        });
        cpass.set_pipeline(&pipeline);
        cpass.set_bind_group(0, &bind_group, &[]);
        cpass.set_bind_group(1, palette_bind_group, &[]);
        let workgroups = (vertices.len() as f32 / 64.0).ceil() as u32;
        cpass.dispatch_workgroups(workgroups, 1, 1);
    }

    // 7. Readback
    let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Staging Buffer"),
        size: output_size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    encoder.copy_buffer_to_buffer(&output_buffer, 0, &staging_buffer, 0, output_size);
    queue.submit(Some(encoder.finish()));

    let buffer_slice = staging_buffer.slice(..);
    let (sender, receiver) = tokio::sync::oneshot::channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |v| sender.send(v).unwrap());
    let _ = device.poll(wgpu::MaintainBase::Wait);
    receiver.await.unwrap().unwrap();

    let data = buffer_slice.get_mapped_range();
    let result: &[GpuOutput] = bytemuck::cast_slice(&data);

    let mut output = Vec::new();
    for item in result {
        output.push((
            Vec3::new(item.pos[0], item.pos[1], item.pos[2]),
            Vec3::new(item.normal[0], item.normal[1], item.normal[2]),
        ));
    }

    drop(data);
    staging_buffer.unmap();

    output
}

/// Test: CPU skinning at t=0 (rest pose)
#[test]
fn test_cpu_skinning_rest_pose() {
    let skeleton = create_parity_test_skeleton();
    let vertices = create_test_vertices();

    // Rest pose: use bind pose transforms
    let local_poses: Vec<Transform> = skeleton.joints.iter().map(|j| j.local_transform).collect();
    let joint_matrices = compute_joint_matrices(&skeleton, &local_poses).unwrap();

    // Skin vertices with CPU
    let skinned = skin_vertices_cpu(&vertices, &joint_matrices);

    // Verify no NaN/Inf
    for (i, (pos, normal)) in skinned.iter().enumerate() {
        assert!(
            pos.is_finite(),
            "Vertex {} position not finite: {:?}",
            i,
            pos
        );
        assert!(
            normal.is_finite(),
            "Vertex {} normal not finite: {:?}",
            i,
            normal
        );
    }

    eprintln!(
        "✅ CPU skinning rest pose: {} vertices valid",
        skinned.len()
    );
}

/// Test: CPU skinning at t=0.5 (animated)
#[test]
fn test_cpu_skinning_animated() {
    let skeleton = create_parity_test_skeleton();
    let clip = create_parity_animation();
    let vertices = create_test_vertices();

    // Sample at t=0.5 (45° rotation midpoint)
    let local_poses = clip.sample(0.5, &skeleton);
    let joint_matrices = compute_joint_matrices(&skeleton, &local_poses).unwrap();

    // Skin vertices with CPU
    let skinned = skin_vertices_cpu(&vertices, &joint_matrices);

    // Verify rotation applied (spine joint should rotate vertices)
    let spine_vertex = skinned[1].0; // Vertex 1 is 100% spine
    assert!(
        spine_vertex.x.abs() > 0.1,
        "Expected rotation, got: {:?}",
        spine_vertex
    );

    eprintln!("✅ CPU skinning animated: rotation applied");
}

/// Test: CPU vs GPU parity at rest pose
/// **Requires GPU context** - marked `#[ignore]` for CI
#[test]
#[ignore = "Requires GPU; run locally with --features skinning-gpu"]
#[cfg(feature = "skinning-gpu")]
fn test_parity_rest_pose() {
    use pollster::FutureExt as _;

    let skeleton = create_parity_test_skeleton();
    let vertices = create_test_vertices();

    // CPU skinning
    let local_poses: Vec<Transform> = skeleton.joints.iter().map(|j| j.local_transform).collect();
    let joint_matrices = compute_joint_matrices(&skeleton, &local_poses).unwrap();
    let cpu_skinned = skin_vertices_cpu(&vertices, &joint_matrices);

    // GPU skinning (requires wgpu setup)
    let (device, queue) = async {
        let instance = wgpu::Instance::default();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: None,
            })
            .await
            .expect("Failed to find adapter");

        adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("Parity Test Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
                trace: wgpu::Trace::Off,
            })
            .await
            .expect("Failed to create device")
    }
    .block_on();

    let gpu_skinned = run_gpu_skinning(&device, &queue, &vertices, &joint_matrices).block_on();

    // Compare with tight tolerance (should be near-identical for rest pose)
    assert_vertices_close(&cpu_skinned, &gpu_skinned, 0.001, 0.001);
}

/// Test: CPU vs GPU parity at animated frame
/// **Requires GPU context** - marked `#[ignore]` for CI
#[test]
#[ignore = "Requires GPU; run locally with --features skinning-gpu"]
#[cfg(feature = "skinning-gpu")]
fn test_parity_animated_frame() {
    use pollster::FutureExt as _;

    let skeleton = create_parity_test_skeleton();
    let clip = create_parity_animation();
    let vertices = create_test_vertices();

    // CPU skinning at t=0.5
    let local_poses = clip.sample(0.5, &skeleton);
    let joint_matrices = compute_joint_matrices(&skeleton, &local_poses).unwrap();
    let cpu_skinned = skin_vertices_cpu(&vertices, &joint_matrices);

    // GPU skinning (requires wgpu setup)
    let (device, queue) = async {
        let instance = wgpu::Instance::default();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: None,
            })
            .await
            .expect("Failed to find adapter");

        adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("Parity Test Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
                trace: wgpu::Trace::Off,
            })
            .await
            .expect("Failed to create device")
    }
    .block_on();

    let gpu_skinned = run_gpu_skinning(&device, &queue, &vertices, &joint_matrices).block_on();

    // Wider tolerance for animated frame (accumulates float errors)
    assert_vertices_close(&cpu_skinned, &gpu_skinned, 0.01, 0.01);
}

/// Test: CPU vs GPU parity with weighted blending
/// **Requires GPU context** - marked `#[ignore]` for CI
#[test]
#[ignore = "Requires GPU; run locally with --features skinning-gpu"]
#[cfg(feature = "skinning-gpu")]
fn test_parity_weighted_blending() {
    use pollster::FutureExt as _;

    let skeleton = create_parity_test_skeleton();
    let clip = create_parity_animation();

    // Vertex with complex blending
    let vertices = vec![(
        Vec3::new(0.0, 1.5, 0.0),
        Vec3::Y,
        [0, 1, 2, 0],
        [0.25, 0.5, 0.25, 0.0], // 3-joint blend
    )];

    // CPU skinning at t=0.75
    let local_poses = clip.sample(0.75, &skeleton);
    let joint_matrices = compute_joint_matrices(&skeleton, &local_poses).unwrap();
    let cpu_skinned = skin_vertices_cpu(&vertices, &joint_matrices);

    // GPU skinning (placeholder)
    let (device, queue) = async {
        let instance = wgpu::Instance::default();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: None,
            })
            .await
            .expect("Failed to find adapter");

        adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("Parity Test Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
                trace: wgpu::Trace::Off,
            })
            .await
            .expect("Failed to create device")
    }
    .block_on();

    let gpu_skinned = run_gpu_skinning(&device, &queue, &vertices, &joint_matrices).block_on();

    // Tolerance for weighted blending
    assert_vertices_close(&cpu_skinned, &gpu_skinned, 0.01, 0.01);
}

#[cfg(test)]
mod tolerance_rationale {
    //! Tolerance Rationale for CPU/GPU Parity
    //!
    //! **Position Tolerance: 0.001 - 0.01**
    //! - Rest pose: 0.001 (tight, no accumulation)
    //! - Animated: 0.01 (allows float precision drift in matrix ops)
    //!
    //! **Normal Tolerance: 0.001 - 0.01**
    //! - Same rationale as position
    //! - Normals are unit length, so absolute diff is meaningful
    //!
    //! **Why not tighter?**
    //! - GPU may use different instruction order (FMA vs separate mul+add)
    //! - f32 precision limits (~7 decimal digits)
    //! - Matrix multiplication accumulates rounding errors
    //!
    //! **Why not looser?**
    //! - Visual artifacts appear above ~0.1 units
    //! - 0.01 is ~1% of typical bone length (1 unit)
    //! - Ensures GPU path is numerically equivalent, not just "close enough"
}
