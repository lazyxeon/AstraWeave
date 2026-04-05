//! Example: Cascaded Shadow Mapping Integration
//!
//! This example demonstrates how to integrate the CSM system into your renderer.
//! It shows the complete flow from initialization to rendering.

use astraweave_render::shadow_csm::CsmRenderer;
use glam::{Mat4, Vec3};
use wgpu;

/// Example scene renderer with CSM shadows
pub struct SceneRenderer {
    csm: CsmRenderer,
    // ... your other rendering resources
}

impl SceneRenderer {
    /// Initialize renderer with shadow mapping
    pub fn new(device: &wgpu::Device) -> anyhow::Result<Self> {
        // Create CSM renderer
        let csm = CsmRenderer::new(device)?;

        Ok(Self {
            csm,
            // ... initialize other resources
        })
    }

    /// Render frame with shadows
    ///
    /// This demonstrates the complete shadow rendering pipeline:
    /// 1. Update cascade splits based on camera
    /// 2. Render shadow maps (4 passes)
    /// 3. Render main scene with shadow sampling
    pub fn render_frame(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        output_view: &wgpu::TextureView,
        camera_pos: Vec3,
        camera_view: Mat4,
        camera_proj: Mat4,
        light_dir: Vec3, // Direction TOWARD light (e.g., Vec3::new(0.3, -1.0, 0.5).normalize())
    ) {
        // ========================================================================
        // STEP 1: Update shadow cascades
        // ========================================================================

        let near = 0.1; // Camera near plane
        let far = 1000.0; // Camera far plane

        self.csm
            .update_cascades(camera_pos, camera_view, camera_proj, light_dir, near, far);

        // Upload to GPU
        self.csm.upload_to_gpu(queue, device);

        // ========================================================================
        // STEP 2: Render shadow maps (4 cascade passes)
        // ========================================================================

        // Prepare geometry buffers (position-only for shadow pass)
        // In a real renderer, you'd extract this from your scene graph
        let vertex_buffer = create_scene_vertex_buffer(device);
        let index_buffer = create_scene_index_buffer(device);
        let index_count = get_scene_index_count();

        // Render all 4 cascades to shadow atlas
        self.csm
            .render_shadow_maps(encoder, &vertex_buffer, &index_buffer, index_count);

        // ========================================================================
        // STEP 3: Render main scene with shadows
        // ========================================================================

        // Your main render pass (color + depth)
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Main Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: output_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &create_depth_view(device), // Your main depth buffer
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        // Set your main pipeline
        // render_pass.set_pipeline(&your_main_pipeline);

        // CRITICAL: Bind shadow resources (group 1 in shader)
        if let Some(bind_group) = &self.csm.bind_group {
            render_pass.set_bind_group(1, bind_group, &[]);
        }

        // Draw your scene
        // render_pass.set_vertex_buffer(0, full_vertex_buffer.slice(..));
        // render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        // render_pass.draw_indexed(0..index_count, 0, 0..1);

        drop(render_pass);
    }
}

/// Example fragment shader (paste into your .wgsl file)
const EXAMPLE_FRAGMENT_SHADER: &str = r#"
// Your existing bindings
@group(0) @binding(0) var<uniform> camera: CameraUniforms;
@group(0) @binding(1) var<uniform> light: LightUniforms;

// CSM bindings (group 1)
@group(1) @binding(0) var<uniform> cascades: array<ShadowCascade, 4>;
@group(1) @binding(1) var shadow_atlas: texture_depth_2d;
@group(1) @binding(2) var shadow_sampler: sampler_comparison;

struct FragmentInput {
    @location(0) world_position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

@fragment
fn main_fragment(in: FragmentInput) -> @location(0) vec4<f32> {
    // Your existing material/texture sampling
    let base_color = vec3<f32>(0.8, 0.8, 0.8);
    
    // Calculate view depth for cascade selection
    let view_pos = camera.view * vec4<f32>(in.world_position, 1.0);
    let view_depth = -view_pos.z;
    
    // Sample shadow map (call from shadow_csm.wgsl)
    let shadow_factor = sample_shadow_csm(
        in.world_position,
        view_depth,
        in.normal
    );
    
    // Calculate lighting (simple Lambertian)
    let light_dir = normalize(light.direction.xyz);
    let diffuse = max(dot(in.normal, light_dir), 0.0);
    
    // Apply shadow to diffuse term (preserve ambient)
    let ambient = 0.2;
    let final_color = base_color * (ambient + shadow_factor * diffuse * 0.8);
    
    // Optional: Debug cascade visualization
    // let debug_color = debug_cascade_color(view_depth);
    // final_color = mix(final_color, debug_color, 0.3);
    
    return vec4<f32>(final_color, 1.0);
}
"#;

// ============================================================================
// Helper functions (implement these based on your renderer)
// ============================================================================

fn create_scene_vertex_buffer(device: &wgpu::Device) -> wgpu::Buffer {
    // TODO: Extract position-only vertices from your scene
    // For shadow pass, you only need vec3<f32> positions

    // Example:
    // let positions: Vec<[f32; 3]> = scene.meshes.iter()
    //     .flat_map(|mesh| mesh.vertices.iter().map(|v| v.position))
    //     .collect();

    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Shadow Vertex Buffer"),
        size: 0, // Replace with actual size
        usage: wgpu::BufferUsages::VERTEX,
        mapped_at_creation: false,
    })
}

fn create_scene_index_buffer(device: &wgpu::Device) -> wgpu::Buffer {
    // TODO: Combine all scene indices
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Shadow Index Buffer"),
        size: 0,
        usage: wgpu::BufferUsages::INDEX,
        mapped_at_creation: false,
    })
}

fn get_scene_index_count() -> u32 {
    // TODO: Return total index count for all meshes
    0
}

fn create_depth_view(device: &wgpu::Device) -> wgpu::TextureView {
    // TODO: Return your main depth buffer view
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("Main Depth Buffer"),
            size: wgpu::Extent3d {
                width: 1920,
                height: 1080,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
        .create_view(&wgpu::TextureViewDescriptor::default())
}

// ============================================================================
// Quick Testing
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_csm_initialization() {
        // This test requires a real wgpu device
        // Use pollster::block_on for async device creation

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: None,
        }))
        .expect("Failed to find GPU adapter");

        let (device, _queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
                .expect("Failed to create device");

        // Test CSM creation
        let csm = CsmRenderer::new(&device);
        assert!(csm.is_ok(), "CSM initialization failed");

        let csm = csm.unwrap();

        // Verify cascade count
        assert_eq!(csm.cascades.len(), 4);

        // Verify atlas size
        assert_eq!(csm.atlas_texture.size().width, 4096);
        assert_eq!(csm.atlas_texture.size().height, 4096);
    }
}
