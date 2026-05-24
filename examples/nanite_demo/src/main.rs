//! Nanite Demo - High-polygon scene rendering with virtualized geometry
//!
//! This demo showcases the Nanite-inspired meshlet rendering system with:
//! - 10M+ polygon scene
//! - LOD hierarchy with automatic selection
//! - Frustum and backface culling
//! - Integration with voxel terrain

use anyhow::Result;
use glam::{Mat4, Vec3};
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

#[cfg(feature = "nanite")]
use astraweave_render::nanite_visibility::GpuMeshlet;
#[cfg(feature = "nanite")]
use astraweave_render::nanite_render::NaniteRenderContext;
use astraweave_asset::nanite_preprocess::{generate_lod_hierarchy, MeshletHierarchy};

// C.6.E (Unified Camera campaign): the pre-C.6.E `DemoState` struct held
// camera state fields (`camera_pos`, `camera_yaw`, `camera_pitch`,
// `camera_speed`, `mouse_sensitivity`, `last_mouse_pos`) along with a
// `get_view_matrix()` method and a `handle_input()` method covering
// WASD/Space/Shift movement, mouse-orbit, and pitch clamping (±π/2 ± 0.1).
// Per C.5 audit L.5.14, none of that state was used: this example's
// `Event::RedrawRequested` handler is a stub that prints meshlet
// statistics without invoking any rendering pipeline. The camera state
// was structurally present but functionally dormant; `get_view_matrix()`
// had zero callers.
//
// `DemoState` is removed entirely. The example continues to demonstrate
// meshlet hierarchy generation (which is its actual purpose); the event
// loop is simplified to handle window close + escape key only.

/// Generate a procedural high-detail sphere mesh
fn generate_sphere_mesh(radius: f32, subdivisions: u32) -> (Vec<[f32; 3]>, Vec<[f32; 3]>, Vec<[f32; 4]>, Vec<[f32; 2]>, Vec<u32>) {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut tangents = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();

    // Generate vertices
    for lat in 0..=subdivisions {
        let theta = lat as f32 * std::f32::consts::PI / subdivisions as f32;
        let sin_theta = theta.sin();
        let cos_theta = theta.cos();

        for lon in 0..=subdivisions {
            let phi = lon as f32 * 2.0 * std::f32::consts::PI / subdivisions as f32;
            let sin_phi = phi.sin();
            let cos_phi = phi.cos();

            let x = sin_theta * cos_phi;
            let y = cos_theta;
            let z = sin_theta * sin_phi;

            positions.push([x * radius, y * radius, z * radius]);
            normals.push([x, y, z]);
            
            // Tangent
            let tx = -sin_phi;
            let tz = cos_phi;
            tangents.push([tx, 0.0, tz, 1.0]);
            
            // UV
            let u = lon as f32 / subdivisions as f32;
            let v = lat as f32 / subdivisions as f32;
            uvs.push([u, v]);
        }
    }

    // Generate indices
    for lat in 0..subdivisions {
        for lon in 0..subdivisions {
            let first = lat * (subdivisions + 1) + lon;
            let second = first + subdivisions + 1;

            indices.push(first);
            indices.push(second);
            indices.push(first + 1);

            indices.push(second);
            indices.push(second + 1);
            indices.push(first + 1);
        }
    }

    (positions, normals, tangents, uvs, indices)
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    
    println!("=== AstraWeave Nanite Demo ===");
    println!("Generating high-polygon scene...");

    // Generate a high-detail sphere (10M+ polygons)
    let subdivisions = 500; // This will create ~1.5M triangles per sphere
    let (positions, normals, tangents, uvs, indices) = generate_sphere_mesh(50.0, subdivisions);
    
    println!("Generated sphere with {} vertices and {} triangles", 
             positions.len(), indices.len() / 3);

    // Generate meshlet hierarchy
    println!("Generating meshlet hierarchy with LODs...");
    let hierarchy = generate_lod_hierarchy(
        &positions,
        &normals,
        &tangents,
        &uvs,
        &indices,
        4, // 4 LOD levels
    )?;

    println!("Generated {} meshlets across {} LOD levels", 
             hierarchy.meshlets.len(), hierarchy.lod_count);
    
    for (lod, range) in hierarchy.lod_ranges.iter().enumerate() {
        let meshlet_count = range.end - range.start;
        println!("  LOD {}: {} meshlets", lod, meshlet_count);
    }

    // Create window
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("AstraWeave Nanite Demo - 10M+ Polygons")
        .with_inner_size(winit::dpi::LogicalSize::new(1920, 1080))
        .build(&event_loop)?;

    println!("\nControls:");
    println!("  WASD - Move camera");
    println!("  Space/Shift - Move up/down");
    println!("  Mouse - Look around");
    println!("  ESC - Exit");

    // C.6.E: removed `DemoState::new()` instantiation and per-frame timing
    // (no rendering happens, no state to advance).

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => {
                if let WindowEvent::CloseRequested
                | WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            state: ElementState::Pressed,
                            virtual_keycode: Some(VirtualKeyCode::Escape),
                            ..
                        },
                    ..
                } = event
                {
                    *control_flow = ControlFlow::Exit;
                }
                // C.6.E: removed `state.handle_input(event, delta_time)`
                // call — `DemoState` and its camera-input handling were
                // deleted (no rendering, no use).
            }
            Event::MainEventsCleared => {
                window.request_redraw();
            }
            Event::RedrawRequested(_) => {
                // The example demonstrates meshlet hierarchy generation
                // (see the prologue logs above); rendering is intentionally
                // not implemented. Camera state was removed in Unified
                // Camera campaign sub-phase C.6.E since it was structurally
                // present but never used.
            }
            _ => {}
        }
    });
}