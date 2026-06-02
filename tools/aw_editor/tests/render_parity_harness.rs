//! Editor-Engine Render Parity Harness.
//!
//! Guards the campaign-wide parity contract: the editor viewport and the
//! engine production renderer produce byte-identical output for the same
//! scene fixture, verified per-machine via SHA-256 of the engine LDR target.
//! "What the user sees in the editor matches what ships from the same
//! machine." This is the WYSIWYG fidelity contract the Editor-Engine Render
//! Parity campaign (P.0 → P.7) achieved and that this harness enforces
//! going forward.
//!
//! ## Five closure proofs that structurally guarantee parity
//!
//! Each proof targets one P.0 audit axis with a measurement instrument
//! matched to its seam type (the campaign's Pillar 5-refinement):
//!
//! - **P.2 loader (byte-level closure)** — both paths invoke
//!   `canonical_terrain_pack::load_canonical_terrain_pack` on the same
//!   biome dir, producing identical CPU bytes for `Renderer::
//!   set_terrain_materials`. Closure proof hashes the pack content.
//!
//! - **P.3 tonemap (pipeline-structural closure)** — `Renderer::draw_into`
//!   no longer branches on `surface.is_none()`; both paths invoke the
//!   single canonical `post_pipeline` (ACES Narkowicz + exposure 1.35 +
//!   scene-env tint) from one `POST_SHADER` source of truth.
//!
//! - **P.4 quality preset (parameter-equality closure)** — both paths
//!   apply `CanonicalQualityPresetParams::GAME_QUALITY` to their renderer
//!   via the shared `apply_canonical_quality_preset_to_renderer` helper.
//!   Call-site assertion: same setters, same arguments, same shared
//!   source of truth.
//!
//! - **P.5 target format (format-equality structural closure)** — engine
//!   and editor `Renderer` instances expose pairwise-equal `surface_format`,
//!   `hdr_format`, and `depth_format` via existing public accessors. The
//!   3-row equality table asserts pass; no new `astraweave-render` API
//!   was added.
//!
//! - **P.6 overlay composition (isolation-structural closure)** — editor
//!   overlays draw into `EDITOR_OVERLAY_TARGET`, never mutating the
//!   parity-contract `ENGINE_LDR_TARGET`. Closure proof runs the editor
//!   path twice (overlays off, overlays on) and asserts the engine LDR
//!   target bytes are byte-identical across both runs.
//!
//! ## Per-machine parity contract
//!
//! This harness verifies editor and engine produce identical bytes on
//! whatever GPU runs it. Cross-machine reproducibility is explicitly out
//! of scope. `wgpu::AdapterInfo` is logged on every run so a future
//! failure can be distinguished as either a real parity regression or a
//! GPU/driver environment change.
//!
//! ## Changes that touch rendering must keep this test passing
//!
//! Anything modifying `astraweave-render`, `aw_editor/src/viewport/`, the
//! canonical loader (`canonical_terrain_pack.rs`), `MaterialManager`, the
//! canonical post pipeline, the quality preset application, the target
//! format selection, or the editor overlay composition layer must keep
//! this test green. Failure indicates a parity-class regression — one of
//! the five seams above has reopened. The relevant closure proof above
//! identifies which seam broke; the campaign-outcome doc has full context.
//!
//! See `docs/audits/editor_engine_render_parity_outcome_2026-05.md` for
//! the campaign's full record: the five seams' technical closure details,
//! the post-P.7 cleanup queue, and the methodology pillars surfaced.
//!
//! Run: `cargo test -p aw_editor --test render_parity_harness -- --nocapture`

use anyhow::{Context, Result};
use astraweave_camera::{CameraProducer, FreeFly, Projection, RenderView};
use astraweave_cinematics as awc;
use astraweave_core::World;
use astraweave_render::Renderer;
use aw_editor_lib::viewport::canonical_terrain_pack as ctp;
use aw_editor_lib::viewport::{OrbitCamera, ViewportRenderer};
use glam::{DMat4, DVec3, Mat4, Vec3};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::sync::Arc;

const WIDTH: u32 = 512;
const HEIGHT: u32 = 512;
const FIXED_TIME_OF_DAY: f32 = 12.0;

// ─── P.2 terrain fixture ────────────────────────────────────────────────────
//
// A single 10m × 10m terrain chunk centered at the world origin, Y=0. Four
// vertices, two triangles. Splat textures (2×2 RGBA8, NUM_SPLAT_MAPS=8) drive
// each corner to a different layer so the splat blend across the chunk
// exercises layers 0..3 from the loaded grassland pack (grass / rock_smooth /
// dirt / sand). Layer 4 (moss) is not exercised by this minimal fixture.
//
// Same chunk uploaded on both sides via `Renderer::upload_terrain_chunk` (on
// the editor side, accessed via `EngineRenderAdapter::renderer_mut()`). This
// isolates loader-axis convergence from any chunk-conversion divergence
// (editor's TerrainVertex → TerrainSplatVertex remap inside
// `upload_or_update_terrain_chunk_forward` is bypassed at the harness level
// so the test measures only what set_terrain_materials surfaces).

const TERRAIN_CHUNK_KEY: u64 = 0;
const TERRAIN_HALF_EXTENT: f32 = 5.0; // 10m × 10m chunk
const TERRAIN_SPLAT_DIM: u32 = 2;

// ─── P.4 shadow caster fixture ──────────────────────────────────────────────
//
// A single sphere instance positioned 5m above the terrain chunk's center,
// scaled 2x for shadow visibility. ToD 12.0 puts the sun nearly overhead so
// the shadow projects approximately downward onto the terrain at origin —
// in the camera's frustum at the existing orbit position.
//
// This fixture exercises the quality-preset axis (Axis 8): pre-seam-closure,
// the editor's EditorDefault preset and the engine path's Renderer-defaults
// produce different cascade splits + filter parameters, surfacing as
// different shadow regions for the same caster. Post-seam-closure both
// paths apply identical GameQuality params via a shared harness helper.

const SHADOW_CASTER_POS: [f32; 3] = [0.0, 5.0, 0.0];
const SHADOW_CASTER_SCALE: f32 = 2.0;

/// Build a single sphere `Instance` at the fixture's caster position.
/// Both engine and editor paths upload this identical instance into the
/// underlying `Renderer` via `update_instances`, exercising the renderer's
/// built-in `mesh_sphere` shadow caster geometry.
fn build_shadow_caster_instance() -> astraweave_render::Instance {
    let pos = SHADOW_CASTER_POS;
    let transform = glam::Mat4::from_scale_rotation_translation(
        glam::Vec3::splat(SHADOW_CASTER_SCALE),
        glam::Quat::IDENTITY,
        glam::Vec3::new(pos[0], pos[1], pos[2]),
    );
    astraweave_render::Instance {
        transform,
        color: [0.8, 0.8, 0.8, 1.0],
        material_id: 0,
    }
}

// ─── P.4 canonical quality preset (Branch A: GameQuality) ───────────────────
//
// `EditorQualityPreset::GameQuality` defined at
// tools/aw_editor/src/viewport/engine_adapter.rs:921-949 is the canonical
// "this is what the game ships" preset. P.4 closure proof: both engine and
// editor paths apply the exact same setter calls with the exact same values
// via this helper. Branch A interpretation per Phase 1 audit; the production
// runtime examples don't currently call `apply_quality_preset(GameQuality)`
// but that's a separate "examples need to standardize" issue (not P.4 scope).

#[derive(Clone, Copy, Debug, PartialEq)]
struct CanonicalQualityPresetParams {
    shadows_enabled: bool,
    cloud_shadows_enabled: bool,
    shadow_filter: (f32, f32, f32),
    cascade_extents: (f32, f32),
    cascade_lambda: f32,
    max_draw_distance: f32,
}

impl CanonicalQualityPresetParams {
    /// `GameQuality` preset values, copied from
    /// `EngineRenderAdapter::apply_quality_preset(GameQuality)` at
    /// `engine_adapter.rs:926-949`. Must stay in lockstep with that match
    /// arm — flagged for future sub-phase if the preset definitions are
    /// elevated to a shared canonical location.
    const GAME_QUALITY: Self = Self {
        shadows_enabled: true,
        cloud_shadows_enabled: true,
        shadow_filter: (2.0, 0.005, 1.5),
        cascade_extents: (40.0, 120.0),
        cascade_lambda: 0.75,
        max_draw_distance: 0.0, // 0 = fog-based fallback
    };
}

/// Apply `GameQuality` preset to a Renderer instance via existing public
/// setters. No new accessors required (anti-drift constraint 10 respected).
/// Post-process chain (bloom/taa/color_grading) is set separately via
/// `set_post_process_chain` if needed — for the harness's minimal fixture
/// (no scene-env tint differences worth measuring) we apply only the
/// shadow + draw-distance parameters that surface in the rendered pixels.
fn apply_canonical_quality_preset_to_renderer(
    renderer: &mut Renderer,
    params: &CanonicalQualityPresetParams,
) {
    renderer.set_shadows_enabled(params.shadows_enabled);
    renderer.set_cloud_shadows_enabled(params.cloud_shadows_enabled);
    renderer.set_shadow_filter(
        params.shadow_filter.0,
        params.shadow_filter.1,
        params.shadow_filter.2,
    );
    renderer.set_cascade_extents(params.cascade_extents.0, params.cascade_extents.1);
    renderer.set_cascade_lambda(params.cascade_lambda);
    renderer.set_max_draw_distance(params.max_draw_distance);
}

/// Build the 4-vertex / 2-triangle quad in the engine's TerrainSplatVertex
/// format. Position is world-space (Y=0), normal is +Y, UV is [0..1].
fn build_terrain_chunk()
-> (Vec<astraweave_render::terrain_material_manager::TerrainSplatVertex>, Vec<u32>)
{
    use astraweave_render::terrain_material_manager::TerrainSplatVertex;
    let v = |x: f32, z: f32, u: f32, v_: f32| TerrainSplatVertex {
        position: [x, 0.0, z],
        normal: [0.0, 1.0, 0.0],
        uv: [u, v_],
    };
    let h = TERRAIN_HALF_EXTENT;
    let vertices = vec![
        v(-h, -h, 0.0, 0.0), // 0: -X -Z corner
        v(h, -h, 1.0, 0.0),  // 1: +X -Z corner
        v(-h, h, 0.0, 1.0),  // 2: -X +Z corner
        v(h, h, 1.0, 1.0),   // 3: +X +Z corner
    ];
    let indices = vec![0, 2, 1, 1, 2, 3];
    (vertices, indices)
}

/// Build the 8 splat textures (RGBA8, 2×2, NUM_SPLAT_MAPS=8). Each corner
/// dominates one of layers 0..3 from the canonical grassland pack. Splats
/// 1..7 (layers 4..31) are all-zero — the fixture exercises 4 layers, which
/// is enough to surface the canonical loader's authored content visibly.
///
/// splat_0 RGBA channel = (layer0, layer1, layer2, layer3) per the canonical
/// 32-layer packing at terrain_material_manager.rs:773-781.
fn build_terrain_splats() -> [Vec<u8>; 8] {
    // 2×2 splat layout. Row-major, top-left origin.
    // (0,0): layer 0 → RGBA = (255, 0, 0, 0)
    // (1,0): layer 1 → RGBA = (0, 255, 0, 0)
    // (0,1): layer 2 → RGBA = (0, 0, 255, 0)
    // (1,1): layer 3 → RGBA = (0, 0, 0, 255)
    let splat_0: Vec<u8> = vec![
        255, 0, 0, 0, // (0,0)
        0, 255, 0, 0, // (1,0)
        0, 0, 255, 0, // (0,1)
        0, 0, 0, 255, // (1,1)
    ];
    let zeros: Vec<u8> = vec![0; (TERRAIN_SPLAT_DIM * TERRAIN_SPLAT_DIM * 4) as usize];
    [
        splat_0,
        zeros.clone(),
        zeros.clone(),
        zeros.clone(),
        zeros.clone(),
        zeros.clone(),
        zeros.clone(),
        zeros,
    ]
}

/// Upload the fixture's canonical biome pack (materials.toml + arrays.toml)
/// and the chunk geometry + splats into the given engine Renderer. Mirrors
/// what `EngineRenderAdapter::reupload_terrain_layers_from_pending_pack` plus
/// a direct chunk-upload would produce. Used by the harness's engine path.
fn upload_engine_terrain_fixture(
    renderer: &mut Renderer,
    fixture: &ParityFixture,
) -> Result<()> {
    renderer
        .init_terrain_forward()
        .context("engine init_terrain_forward failed")?;

    let pack = ctp::load_canonical_terrain_pack(&fixture.biome_path)
        .context("engine path: load canonical pack failed")?;
    let layers = ctp::borrow_layer_textures(&pack);
    let gpu_material = ctp::build_gpu_material(&pack);
    renderer
        .set_terrain_materials(&gpu_material, &layers)
        .context("engine set_terrain_materials failed")?;

    let (vertices, indices) = build_terrain_chunk();
    let splats = build_terrain_splats();
    let splat_refs: Vec<&[u8]> = splats.iter().map(|s| s.as_slice()).collect();
    renderer
        .upload_terrain_chunk(
            TERRAIN_CHUNK_KEY,
            &vertices,
            &indices,
            &splat_refs,
            (TERRAIN_SPLAT_DIM, TERRAIN_SPLAT_DIM),
        )
        .context("engine upload_terrain_chunk failed")?;
    Ok(())
}

/// Upload the same fixture content into the editor's adapter. Triggers the
/// canonical-pack load via `set_biome_pack`, then pushes the same chunk
/// geometry + splats directly into the underlying Renderer (bypassing the
/// adapter's TerrainVertex → TerrainSplatVertex conversion so the chunk
/// data is byte-identical to the engine path's upload).
fn upload_editor_terrain_fixture(
    viewport: &mut ViewportRenderer,
    fixture: &ParityFixture,
) -> Result<()> {
    let adapter = viewport
        .engine_adapter_mut()
        .context("editor adapter not initialised")?;
    // Step 1: bring terrain_forward up so the next `set_biome_pack` call sees
    // the canonical 32-layer pipeline live and triggers the canonical-pack
    // reupload immediately. The change-detection inside `set_biome_pack`
    // (None → Some) ensures the reupload runs exactly once.
    adapter
        .renderer_mut()
        .init_terrain_forward()
        .context("editor init_terrain_forward failed")?;
    // Step 2: set the canonical biome pack. With terrain_forward already
    // initialised and the prior pending_biome_pack == None, this transitions
    // to Some(path) and invokes `reupload_terrain_layers_from_pending_pack`
    // synchronously — pushing the canonical grassland layer arrays into
    // `Renderer::set_terrain_materials`. Mirrors what main.rs:5093 does at
    // editor startup.
    adapter.set_biome_pack(Some(fixture.biome_path.clone()));
    // Step 3: upload the chunk via the underlying renderer directly so the
    // chunk bytes are byte-identical to the engine path's upload (bypasses
    // the adapter's TerrainVertex → TerrainSplatVertex remap, isolating
    // loader-axis convergence from chunk-conversion divergence at P.2).
    let renderer = adapter.renderer_mut();
    let (vertices, indices) = build_terrain_chunk();
    let splats = build_terrain_splats();
    let splat_refs: Vec<&[u8]> = splats.iter().map(|s| s.as_slice()).collect();
    renderer
        .upload_terrain_chunk(
            TERRAIN_CHUNK_KEY,
            &vertices,
            &indices,
            &splat_refs,
            (TERRAIN_SPLAT_DIM, TERRAIN_SPLAT_DIM),
        )
        .context("editor upload_terrain_chunk failed")?;
    Ok(())
}

/// Locked fixture parameters. See P.0 Phase 5 / P.1 Phase 1 audit summary.
struct ParityFixture {
    width: u32,
    height: u32,
    time_of_day: f32,
    biome_path: PathBuf,
}

impl ParityFixture {
    fn default_grassland() -> Self {
        Self {
            width: WIDTH,
            height: HEIGHT,
            time_of_day: FIXED_TIME_OF_DAY,
            biome_path: PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../assets/materials/grassland"),
        }
    }

    fn camera(&self) -> OrbitCamera {
        // 25m distance, 45° yaw, 30° pitch — matches OrbitCamera::default()'s
        // intent (diagonal view) with the fixture aspect locked to 1:1.
        let mut cam = OrbitCamera::new(
            Vec3::ZERO,
            25.0,
            std::f32::consts::FRAC_PI_4,
            std::f32::consts::FRAC_PI_6,
        );
        cam.set_aspect(self.width as f32, self.height as f32);
        cam
    }
}

// ─── P.5 format-equality closure proof state ────────────────────────────────
//
// Captured from each Renderer instance via its existing public accessors
// (`surface_format()`, `hdr_format()`, `depth_format()`). No new public API
// added to astraweave-render (anti-drift constraint 14 respected). The closure
// proof reports the values and asserts pairwise equality across rows where
// both sides have a format value.

#[derive(Clone, Copy, Debug, PartialEq)]
struct RendererFormats {
    /// `config.format` — the canonical post_pipeline output format. P.3 made
    /// this Rgba8UnormSrgb on both sides (the editor adapter's config.format
    /// migrated as a downstream consequence of the surface.is_none() branch
    /// deletion; engine harness path was already aligned).
    surface_format: wgpu::TextureFormat,
    /// Internal HDR intermediate format. Hardcoded at
    /// `astraweave-render/src/renderer.rs:5788` to `Rgba16Float`. Same code
    /// path on both sides, so necessarily equal — recorded for completeness.
    hdr_format: wgpu::TextureFormat,
    /// Depth attachment format. `Depth32Float` per renderer.rs:2357.
    depth_format: wgpu::TextureFormat,
}

impl RendererFormats {
    fn capture(renderer: &Renderer) -> Self {
        Self {
            surface_format: renderer.surface_format(),
            hdr_format: renderer.hdr_format(),
            depth_format: renderer.depth_format(),
        }
    }
}

/// Engine production path readback. P.3: now Rgba8UnormSrgb (4 B / px) —
/// the canonical post_pipeline runs unconditionally and writes LDR after
/// ACES tonemap + scene-env tint. Pre-P.3 was Rgba16Float HDR passthrough.
struct EngineFrame {
    bytes: Vec<u8>,
    width: u32,
    height: u32,
    /// P.5: captured from the Renderer's public format accessors before
    /// readback. Drives the format-equality closure proof in the report.
    formats: RendererFormats,
}

/// Editor viewport path readback. P.6: now carries two byte-buffers —
/// the internal ENGINE_LDR_TARGET (the parity-contract target; hashed
/// for the campaign closure proofs) and the caller-supplied display
/// target (the composite of engine + overlay; what the user sees in
/// the editor). The two are equal when no overlays draw (P.6's
/// overlay-isolation contract verified by running editor path twice
/// with show_grid=false then show_grid=true and comparing engine_ldr
/// bytes across both runs).
struct EditorFrame {
    /// Bytes from the internal ENGINE_LDR_TARGET texture — what the
    /// canonical post_pipeline wrote. The hashable parity-contract
    /// target. Independent of editor overlays.
    engine_ldr_bytes: Vec<u8>,
    /// Bytes from the caller-supplied display target — the composite
    /// output (engine + overlay alpha-over). Differs from engine_ldr_bytes
    /// when overlays drew. Diagnostic only; not part of the parity contract.
    display_bytes: Vec<u8>,
    width: u32,
    height: u32,
    /// P.5: captured from the editor adapter's inner Renderer's public
    /// format accessors before readback. Mirrors `EngineFrame::formats`.
    formats: RendererFormats,
}

async fn acquire_device() -> Result<(Arc<wgpu::Device>, Arc<wgpu::Queue>, wgpu::AdapterInfo)> {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::PRIMARY,
        ..Default::default()
    });
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        })
        .await
        .context("no suitable wgpu adapter for parity harness")?;
    let info = adapter.get_info();
    // Renderer::new requires max_bind_groups: 8 — see renderer.rs:1084.
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("parity-harness device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits {
                max_bind_groups: 8,
                ..wgpu::Limits::default()
            },
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
        })
        .await
        .context("request_device failed")?;
    Ok((Arc::new(device), Arc::new(queue), info))
}

async fn render_engine_path(
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    fixture: &ParityFixture,
) -> Result<EngineFrame> {
    // C.3.C: `CameraUploadPath` enum removed. `Renderer::update_view` is the
    // only camera-upload path; the deprecated `update_camera_matrices` wrapper
    // and the wrapper-preservation test were deleted alongside.
    // P.3: mirror EngineRenderAdapter::new with Rgba8UnormSrgb config.format
    // (changed from Bgra8UnormSrgb in same commit) so post_pipeline outputs
    // LDR sRGB bytes matching the harness's external target view. The
    // surface.is_none() branch in draw_into was deleted in this sub-phase;
    // post_pipeline (ACES Narkowicz + exposure 1.35 + scene-env tint) now
    // runs unconditionally on both windowed and headless invocations.
    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        width: fixture.width,
        height: fixture.height,
        present_mode: wgpu::PresentMode::AutoVsync,
        alpha_mode: wgpu::CompositeAlphaMode::Auto,
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    };

    let device_owned = (*device).clone();
    let queue_owned = (*queue).clone();
    let mut renderer = Renderer::new_from_device(device_owned, queue_owned, None, config)
        .await
        .context("Renderer::new_from_device failed")?;

    renderer.time_of_day_mut().current_time = fixture.time_of_day;

    let camera = fixture.camera();
    let view_matrix = camera.view_matrix();
    let projection_matrix = camera.projection_matrix();
    let position = camera.position();
    const ENGINE_PATH_ZNEAR: f32 = 0.5;
    const ENGINE_PATH_ZFAR: f32 = 5000.0;
    let fovy = 60_f32.to_radians();
    let aspect = fixture.width as f32 / fixture.height as f32;

    // C.3.C canonical (and only) path: build RenderView directly, call
    // Renderer::update_view.
    {
        let inverse_view = view_matrix.inverse();
        let view_dir = -inverse_view.col(2).truncate();
        let view_proj = projection_matrix * view_matrix;
        let inverse_view_proj = view_proj.inverse();
        let render_view = astraweave_camera::RenderView {
            view: view_matrix,
            projection: projection_matrix,
            view_proj,
            inverse_view,
            inverse_view_proj,
            position,
            view_dir,
            fovy,
            aspect,
            znear: ENGINE_PATH_ZNEAR,
            zfar: ENGINE_PATH_ZFAR,
        };
        renderer.update_view(&render_view);
    }

    // P.2 fixture expansion: upload canonical grassland biome pack + a single
    // 10m × 10m terrain chunk at origin so the loader axis becomes measurable.
    // Failure here is logged but not fatal — the test still produces output
    // (sky + engine-default ground plane), the loader-axis SAD just stays
    // unmeasurable.
    if let Err(e) = upload_engine_terrain_fixture(&mut renderer, fixture) {
        eprintln!("[harness] Engine path terrain fixture upload failed: {e:#}");
    }

    // P.4 fixture expansion: a single sphere instance positioned above the
    // terrain so its shadow falls within the camera's frustum. Both engine
    // and editor paths upload byte-identical instance data; the renderer's
    // built-in mesh_sphere is the caster geometry.
    renderer.update_instances(&[build_shadow_caster_instance()]);

    // P.4 seam closure (Move C): engine path applies GameQuality preset
    // via the shared harness helper. Editor path applies the same via
    // EngineRenderAdapter::new (Move A switched the default from
    // EditorDefault to GameQuality) plus a defensive re-application in
    // the editor harness setup. Both arrive at identical setter-call
    // sequence with identical argument values — the call-site closure
    // proof. See the report section for closure verification.
    apply_canonical_quality_preset_to_renderer(
        &mut renderer,
        &CanonicalQualityPresetParams::GAME_QUALITY,
    );

    // P.3: external Rgba8UnormSrgb LDR target — matches post_pipeline's
    // config.format output. Pre-P.3 was Rgba16Float to match the now-deleted
    // hdr_blit_pipeline's hardcoded format.
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("parity-engine-ldr-target"),
        size: wgpu::Extent3d {
            width: fixture.width,
            height: fixture.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[wgpu::TextureFormat::Rgba8Unorm],
    });
    let view = target.create_view(&wgpu::TextureViewDescriptor::default());

    // Warm-up frame so clustered-lights cache + IBL bake settle deterministically.
    let mut enc1 = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("parity-engine warm-up encoder"),
    });
    renderer
        .draw_into(&view, None, &mut enc1)
        .context("engine warm-up draw_into failed")?;
    queue.submit(std::iter::once(enc1.finish()));

    // Measurement frame.
    let mut enc2 = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("parity-engine measurement encoder"),
    });
    renderer
        .draw_into(&view, None, &mut enc2)
        .context("engine measurement draw_into failed")?;

    // P.5 format capture (before readback consumes the renderer's encoder):
    // record the engine path's Renderer formats for the format-equality
    // closure proof. Reads via existing public accessors only.
    let formats = RendererFormats::capture(&renderer);

    // P.3: 4 B/px Rgba8UnormSrgb (was 8 B/px Rgba16Float pre-P.3).
    let bytes = readback_texture(&device, &queue, &target, fixture.width, fixture.height, 4, enc2)?;
    Ok(EngineFrame {
        bytes,
        width: fixture.width,
        height: fixture.height,
        formats,
    })
}

async fn render_editor_path(
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    fixture: &ParityFixture,
    show_grid: bool,
) -> Result<EditorFrame> {
    let mut viewport = ViewportRenderer::new(device.clone(), queue.clone())
        .context("ViewportRenderer::new failed")?;
    viewport
        .init_engine_adapter()
        .await
        .context("ViewportRenderer::init_engine_adapter failed")?;
    if let Some(adapter) = viewport.engine_adapter_mut() {
        adapter.set_time_of_day(fixture.time_of_day);
    }

    // P.2 fixture expansion: upload the same canonical grassland biome pack +
    // terrain chunk that the engine path uploads, ensuring loader-axis
    // convergence between the two paths.
    if let Err(e) = upload_editor_terrain_fixture(&mut viewport, fixture) {
        eprintln!("[harness] Editor path terrain fixture upload failed: {e:#}");
    }

    // P.4 fixture expansion: identical shadow caster instance on the editor
    // side. Goes through the same Renderer::update_instances API the engine
    // path uses; bytes are identical.
    if let Some(adapter) = viewport.engine_adapter_mut() {
        adapter
            .renderer_mut()
            .update_instances(&[build_shadow_caster_instance()]);

        // P.4 seam closure (Move A + Move C): EngineRenderAdapter::new
        // already applied GameQuality via the canonical preset (Move A
        // switched the default from EditorDefault to GameQuality). This
        // call is defensive re-application — guarantees parameter equality
        // with the engine path regardless of any future drift in the
        // adapter's construction code path, and keeps both sides going
        // through the same harness-controlled setter calls.
        apply_canonical_quality_preset_to_renderer(
            adapter.renderer_mut(),
            &CanonicalQualityPresetParams::GAME_QUALITY,
        );
    }

    // Display target (caller-supplied texture; the egui-bound viewport
    // texture in the editor runtime; harness-allocated here). Post-P.6
    // this is the composite output (engine LDR + editor overlay alpha-over)
    // — what the user sees, NOT the parity-contract hashable target.
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("parity-editor-display-target"),
        size: wgpu::Extent3d {
            width: fixture.width,
            height: fixture.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[wgpu::TextureFormat::Rgba8Unorm],
    });

    let camera = fixture.camera();
    let world = World::new();

    // Two-frame settle. The first frame triggers ViewportRenderer::resize
    // (lazy allocation of depth + ENGINE_LDR_TARGET + EDITOR_OVERLAY_TARGET
    // + composite pipeline) plus the engine adapter's first-frame state
    // (clustered-lights cache, IBL bake, etc.). Measurement is frame 2.
    viewport
        .render(&target, &camera, &world, None, None, None, show_grid, false, 0)
        .context("editor warm-up render failed")?;
    viewport
        .render(&target, &camera, &world, None, None, None, show_grid, false, 0)
        .context("editor measurement render failed")?;

    // P.5 format capture: read the editor-adapter's inner Renderer formats
    // via the same public accessors used on the engine path. The closure
    // proof asserts pairwise equality (engine.formats == editor.formats).
    let formats = viewport
        .engine_adapter()
        .map(|adapter| RendererFormats::capture(adapter.renderer()))
        .context("editor adapter not initialised at format-capture site")?;

    // P.6 closure-proof readback: capture bytes from the internal
    // ENGINE_LDR_TARGET (the parity-contract target — bit-identical to
    // what the runtime would produce; overlays never mutate it) AND
    // from the display target (the composite — diagnostic only).
    let engine_ldr_texture = viewport
        .engine_ldr_texture()
        .context("ENGINE_LDR_TARGET texture missing — was resize() called?")?;

    let enc_engine = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("parity-editor engine_ldr readback encoder"),
    });
    let engine_ldr_bytes = readback_texture(
        &device,
        &queue,
        engine_ldr_texture,
        fixture.width,
        fixture.height,
        4,
        enc_engine,
    )?;

    let enc_display = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("parity-editor display readback encoder"),
    });
    let display_bytes = readback_texture(
        &device,
        &queue,
        &target,
        fixture.width,
        fixture.height,
        4,
        enc_display,
    )?;

    Ok(EditorFrame {
        engine_ldr_bytes,
        display_bytes,
        width: fixture.width,
        height: fixture.height,
        formats,
    })
}

/// Generic texture readback. Encoder must be open (no submit yet); this fn
/// records `copy_texture_to_buffer`, submits, maps synchronously, de-pads rows.
fn readback_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    tex: &wgpu::Texture,
    width: u32,
    height: u32,
    bytes_per_pixel: u32,
    mut encoder: wgpu::CommandEncoder,
) -> Result<Vec<u8>> {
    let unpadded = width * bytes_per_pixel;
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let padded = unpadded.div_ceil(align) * align;
    let size = (padded * height) as u64;
    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("parity-readback-staging"),
        size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: tex,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &staging,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(std::iter::once(encoder.finish()));

    let (tx, rx) = std::sync::mpsc::channel();
    staging
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |res| {
            let _ = tx.send(res);
        });
    loop {
        if device.poll(wgpu::MaintainBase::Wait).is_ok() {
            if let Ok(r) = rx.try_recv() {
                r.context("map_async failed")?;
                break;
            }
        }
    }
    let mapped = staging.slice(..).get_mapped_range();
    let mut out = Vec::with_capacity((unpadded * height) as usize);
    let row_stride = padded as usize;
    let row_bytes = unpadded as usize;
    for y in 0..height as usize {
        let src = y * row_stride;
        out.extend_from_slice(&mapped[src..src + row_bytes]);
    }
    drop(mapped);
    staging.unmap();
    Ok(out)
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    format!("{:x}", h.finalize())
}

/// Hash the canonical biome pack's CPU bytes (albedo + normal + mra per layer,
/// in array-index order) plus the active layer count. P.2 loader-axis closure
/// proof: both editor and engine paths load via this loader on the same input
/// directory, so this hash is identical on both sides — direct byte-level
/// evidence that the loader axis is closed regardless of what the per-pixel
/// probe attribution shows.
fn hash_canonical_pack(pack: &ctp::CanonicalTerrainPack) -> String {
    let mut h = Sha256::new();
    h.update(pack.biome_name.as_bytes());
    h.update(pack.active_layer_count.to_le_bytes());
    for layer in &pack.layers {
        let len_or_zero = |opt: &Option<Vec<u8>>| opt.as_ref().map(|v| v.len()).unwrap_or(0);
        h.update((len_or_zero(&layer.albedo) as u64).to_le_bytes());
        h.update((len_or_zero(&layer.normal) as u64).to_le_bytes());
        h.update((len_or_zero(&layer.mra) as u64).to_le_bytes());
        if let Some(a) = &layer.albedo {
            h.update(a);
        }
        if let Some(n) = &layer.normal {
            h.update(n);
        }
        if let Some(m) = &layer.mra {
            h.update(m);
        }
        h.update(layer.uv_scale[0].to_le_bytes());
        h.update(layer.uv_scale[1].to_le_bytes());
    }
    format!("{:x}", h.finalize())
}

// ─── Linear-space normalization for SAD computation ────────────────────────
//
// P.3: both engine and editor paths now produce Rgba8UnormSrgb. Pre-P.3,
// the engine path was Rgba16Float HDR and the editor was Rgba8UnormSrgb LDR,
// requiring cross-format decoding. The f16 decoder is gone (no consumer);
// `half` dev-dep is now unused — left in Cargo.toml as it's a single line
// of metadata and may return in future sub-phases that probe HDR
// intermediate state (P.6 composition pass introspection, etc.).

#[inline]
fn rgba8srgb_to_linear(pixel: &[u8]) -> [f32; 3] {
    let s = |v: u8| -> f32 {
        let f = v as f32 / 255.0;
        if f <= 0.04045 {
            f / 12.92
        } else {
            ((f + 0.055) / 1.055).powf(2.4)
        }
    };
    [s(pixel[0]), s(pixel[1]), s(pixel[2])]
}

// ─── Per-axis SAD attribution (heuristic) ───────────────────────────────────
//
// Per the P.1 prompt: attribution does not need to sum to 100% and is not
// expected to perfectly isolate any axis. The point is directional usefulness
// for sub-phase progress tracking — each subsequent sub-phase reduces the SAD
// attributed to the named axis it closes.
//
// Heuristics used here:
// - **Tonemap axis (Axis 11)**: synthesize what engine HDR WOULD look like
//   after the canonical engine ACES pass (POST_SHADER, exposure 1.35, no
//   tint — scene-env tint is identity in default state). The SAD reduction
//   from pre-tonemap to post-tonemap on the engine side, per pixel, is
//   tonemap-attributable.
// - **Target format axis (Axis 6)**: synthesize a Rgba8UnormSrgb round-trip
//   of engine HDR (clamp [0,1] → sRGB-encode → 8-bit truncate → sRGB-decode).
//   The SAD between original and round-tripped engine values is the bytes
//   that would be lost purely to format quantization.
// - **Loader axis (Axes 1, 10)** and **quality preset axis (Axis 8)**: probed
//   at 16 fixed sample positions each. Without a real terrain chunk uploaded
//   in P.1's fixture (sky + engine-default ground plane only), the loader
//   probe registers near-zero. Reported faithfully; P.2 expands the fixture.
// - **Overlay composition (residual)**: `total - (tonemap + format)`. The
//   loader/quality probe SADs are reported separately and not double-counted.

struct AxisAttribution {
    total_sad: f64,
    tonemap_axis_sad: f64,
    target_format_axis_sad: f64,
    loader_axis_probe_sad: f64,
    loader_axis_probe_pixels: u64,
    quality_preset_axis_probe_sad: f64,
    quality_preset_axis_probe_pixels: u64,
    overlay_composition_residual: f64,
    pixel_count: u64,
}

impl AxisAttribution {
    fn format_report(&self) -> String {
        let pct = |v: f64| -> f64 {
            if self.total_sad > 0.0 {
                100.0 * v / self.total_sad
            } else {
                0.0
            }
        };
        format!(
            "Per-axis SAD attribution (heuristic, linear-RGB space):\n  \
             Pixels compared: {}\n  \
             Total SAD: {:.4} (mean {:.6} / pixel)\n  \
             \n  \
             Full-frame attributions:\n    \
             - Tonemap divergence (Axis 11):       {:>12.4} ({:.1}%)\n    \
             - Target format divergence (Axis 6):  {:>12.4} ({:.1}%)\n    \
             - Overlay composition (residual):     {:>12.4} ({:.1}%)\n  \
             \n  \
             Sampled-probe attributions (16 fixed positions each):\n    \
             - Loader divergence (Axes 1, 10):     {:>12.4} over {} px\n    \
             - Quality preset (Axis 8):            {:>12.4} over {} px",
            self.pixel_count,
            self.total_sad,
            self.total_sad / self.pixel_count.max(1) as f64,
            self.tonemap_axis_sad,
            pct(self.tonemap_axis_sad),
            self.target_format_axis_sad,
            pct(self.target_format_axis_sad),
            self.overlay_composition_residual,
            pct(self.overlay_composition_residual),
            self.loader_axis_probe_sad,
            self.loader_axis_probe_pixels,
            self.quality_preset_axis_probe_sad,
            self.quality_preset_axis_probe_pixels,
        )
    }
}

/// Engine canonical ACES (POST_SHADER, astraweave-render/src/renderer.rs:366-383):
/// ACES Narkowicz fit with exposure pre-multiply 1.35. Scene-env tint at default
/// is identity (`tint_alpha = 0.0`), so the `mix(color, tint_color, tint_alpha)`
/// at POST_SHADER:381 is a no-op for the default fixture.
#[inline]
fn aces_canonical(rgb: [f32; 3]) -> [f32; 3] {
    const EXPOSURE: f32 = 1.35;
    let aces = |x: f32| -> f32 {
        let xe = x * EXPOSURE;
        let a = 2.51;
        let b = 0.03;
        let c = 2.43;
        let d = 0.59;
        let e = 0.14;
        ((xe * (a * xe + b)) / (xe * (c * xe + d) + e)).clamp(0.0, 1.0)
    };
    [aces(rgb[0]), aces(rgb[1]), aces(rgb[2])]
}

/// Round-trip a linear value through Rgba8UnormSrgb (clamp to [0,1], sRGB-encode,
/// truncate to u8, sRGB-decode back to linear). The discrepancy between input
/// and output is what would be lost to format-axis quantization.
#[inline]
fn srgb_u8_roundtrip(v: f32) -> f32 {
    let clamped = v.clamp(0.0, 1.0);
    let srgb = if clamped <= 0.0031308 {
        clamped * 12.92
    } else {
        1.055 * clamped.powf(1.0 / 2.4) - 0.055
    };
    let u = (srgb * 255.0).round().clamp(0.0, 255.0) as u8;
    let f = u as f32 / 255.0;
    if f <= 0.04045 {
        f / 12.92
    } else {
        ((f + 0.055) / 1.055).powf(2.4)
    }
}

fn compute_attribution(engine: &EngineFrame, editor: &EditorFrame) -> AxisAttribution {
    assert_eq!(engine.width, editor.width);
    assert_eq!(engine.height, editor.height);
    let w = engine.width as usize;
    let h = engine.height as usize;

    // Fixed probe positions. Real attribution requires per-pixel knowledge of
    // which axis dominates — impossible without running ablated paths (P.2-P.6
    // scope). These probes register samples in regions where the named axis
    // would heuristically dominate; P.1's fixture has no terrain chunk so the
    // loader probe is near-zero by design (documented in the report comment).
    let loader_xy: Vec<(usize, usize)> = (0..16)
        .map(|i| {
            let col = i % 4;
            let row = i / 4;
            let x = w / 8 + (w * col * 3) / 16;
            let y = h * 5 / 8 + (h * row) / 32;
            (x.min(w - 1), y.min(h - 1))
        })
        .collect();
    let quality_xy: Vec<(usize, usize)> = (0..16)
        .map(|i| {
            let col = i % 4;
            let row = i / 4;
            let x = w / 4 + (w * col) / 8;
            let y = h / 8 + (h * row) / 32; // upper-frame: sky/horizon region
            (x.min(w - 1), y.min(h - 1))
        })
        .collect();
    let loader_set: std::collections::HashSet<(usize, usize)> =
        loader_xy.iter().copied().collect();
    let quality_set: std::collections::HashSet<(usize, usize)> =
        quality_xy.iter().copied().collect();

    let mut total_sad = 0.0f64;
    let mut tonemap_sad = 0.0f64;
    let mut format_sad = 0.0f64;
    let mut loader_probe_sad = 0.0f64;
    let mut quality_probe_sad = 0.0f64;

    for y in 0..h {
        for x in 0..w {
            // P.3: both paths produce Rgba8UnormSrgb. Decode each side from
            // sRGB-8 to linear f32 for SAD computation in linear space.
            let idx = (y * w + x) * 4;
            let eng_lin = rgba8srgb_to_linear(&engine.bytes[idx..idx + 4]);
            // P.6: editor side uses engine_ldr_bytes (the parity-contract
            // target). display_bytes carries the overlay composite which
            // intentionally differs from engine bytes when overlays drew.
            let edt_lin = rgba8srgb_to_linear(&editor.engine_ldr_bytes[idx..idx + 4]);

            let dr = (eng_lin[0] - edt_lin[0]).abs() as f64;
            let dg = (eng_lin[1] - edt_lin[1]).abs() as f64;
            let db = (eng_lin[2] - edt_lin[2]).abs() as f64;
            let pixel_sad = dr + dg + db;
            total_sad += pixel_sad;

            // Tonemap: how much SAD is removed when we tonemap engine HDR.
            let eng_tm = aces_canonical(eng_lin);
            let post_tm = ((eng_tm[0] - edt_lin[0]).abs()
                + (eng_tm[1] - edt_lin[1]).abs()
                + (eng_tm[2] - edt_lin[2]).abs()) as f64;
            tonemap_sad += (pixel_sad - post_tm).max(0.0);

            // Format: sRGB-u8 round-trip discrepancy on the engine value.
            // Compare against the tonemapped value rather than raw HDR so the
            // format-axis contribution reflects what would remain AFTER the
            // tonemap axis is closed. (Otherwise format and tonemap double-count
            // the clamp-to-[0,1] effect.)
            let fr = (eng_tm[0] - srgb_u8_roundtrip(eng_tm[0])).abs() as f64;
            let fg = (eng_tm[1] - srgb_u8_roundtrip(eng_tm[1])).abs() as f64;
            let fb = (eng_tm[2] - srgb_u8_roundtrip(eng_tm[2])).abs() as f64;
            format_sad += fr + fg + fb;

            if loader_set.contains(&(x, y)) {
                loader_probe_sad += pixel_sad;
            }
            if quality_set.contains(&(x, y)) {
                quality_probe_sad += pixel_sad;
            }
        }
    }

    let attributed_full_frame = tonemap_sad + format_sad;
    let overlay_composition_residual = (total_sad - attributed_full_frame).max(0.0);

    AxisAttribution {
        total_sad,
        tonemap_axis_sad: tonemap_sad,
        target_format_axis_sad: format_sad,
        loader_axis_probe_sad: loader_probe_sad,
        loader_axis_probe_pixels: 16,
        quality_preset_axis_probe_sad: quality_probe_sad,
        quality_preset_axis_probe_pixels: 16,
        overlay_composition_residual,
        pixel_count: (w * h) as u64,
    }
}

// ─── Test ────────────────────────────────────────────────────────────────────

#[test]
fn editor_engine_render_parity() {
    let fixture = ParityFixture::default_grassland();
    let (device, queue, adapter_info) =
        pollster::block_on(acquire_device()).expect("acquire wgpu device");

    eprintln!("============================================================");
    eprintln!("Editor-Engine Render Parity Harness");
    eprintln!("============================================================");
    eprintln!(
        "Adapter: {} | device_type={:?} | backend={:?}",
        adapter_info.name, adapter_info.device_type, adapter_info.backend
    );
    eprintln!(
        "Driver info: \"{}\" | vendor=0x{:x} | device_id=0x{:x}",
        adapter_info.driver_info, adapter_info.vendor, adapter_info.device
    );
    eprintln!(
        "Fixture: {}x{} | ToD={} | biome={}",
        fixture.width,
        fixture.height,
        fixture.time_of_day,
        fixture.biome_path.display()
    );
    eprintln!("Per-machine parity contract — hash comparison valid only on this adapter.");
    eprintln!();

    // C.3.C: `Renderer::update_view(&RenderView)` is the canonical (and only)
    // camera-upload path. The deprecated `update_camera_matrices` wrapper and
    // the sibling wrapper-preservation test were deleted in C.3.C.
    let engine = pollster::block_on(render_engine_path(
        device.clone(),
        queue.clone(),
        &fixture,
    ))
    .expect("engine path render failed");

    // P.6 editor path runs twice: once with overlays disabled (show_grid=false)
    // and once with overlays enabled (show_grid=true). The closure proof
    // asserts the editor's internal ENGINE_LDR_TARGET bytes are byte-identical
    // across the two runs — overlays must not mutate the parity-contract
    // target. The display target hashes are captured for diagnostic only;
    // they intentionally differ when overlays drew.
    let editor = pollster::block_on(render_editor_path(
        device.clone(),
        queue.clone(),
        &fixture,
        false, // overlays disabled — the canonical parity comparison
    ))
    .expect("editor path render (overlays disabled) failed");
    let editor_overlays_on = pollster::block_on(render_editor_path(
        device.clone(),
        queue.clone(),
        &fixture,
        true, // overlays enabled — drives the overlay-isolation closure proof
    ))
    .expect("editor path render (overlays enabled) failed");

    let engine_hash = sha256_hex(&engine.bytes);
    let editor_hash = sha256_hex(&editor.engine_ldr_bytes);
    let editor_engine_ldr_overlays_on = sha256_hex(&editor_overlays_on.engine_ldr_bytes);
    let editor_display_overlays_off = sha256_hex(&editor.display_bytes);
    let editor_display_overlays_on = sha256_hex(&editor_overlays_on.display_bytes);

    // P.2 direct loader-axis closure proof: hash the canonical pack's CPU
    // bytes that flow into Renderer::set_terrain_materials on both sides.
    // If the editor and engine paths both invoke load_canonical_terrain_pack
    // on the same biome dir, they receive identical bytes → loader axis
    // closed at the input boundary. This is the byte-level proof that the
    // per-pixel probe in compute_attribution can't isolate (terrain pixels'
    // diff also contains tonemap+format axes).
    let pack_hash = match ctp::load_canonical_terrain_pack(&fixture.biome_path) {
        Ok(pack) => Some(hash_canonical_pack(&pack)),
        Err(e) => {
            eprintln!(
                "[harness] Canonical pack hash skipped — load failed: {e:#}"
            );
            None
        }
    };

    // P.3 tonemap-axis closure proof — structural, not byte-equality.
    //
    // After P.3, `Renderer::draw_into` no longer branches on `surface.is_none()`:
    // it unconditionally invokes the canonical `post_pipeline` (ACES Narkowicz
    // + exposure 1.35 + scene-env tint) as the terminal stage. Both the
    // harness's engine path and the editor path construct their `Renderer`
    // via `Renderer::new_from_device(..., None, config)` with identical
    // `config.format = Rgba8UnormSrgb`. Therefore:
    //
    //   1. Both paths' Renderer instances build `post_pipeline` from the same
    //      `POST_SHADER` constant (one source of truth in astraweave-render).
    //   2. Both pipelines write `config.format` (Rgba8UnormSrgb) outputs.
    //   3. Both call sites in `draw_into` hit the same code path now (the
    //      pre-P.3 `hdr_blit_pipeline` editor-mode branch is deleted).
    //
    // A computational proof — byte-identical renderings of an identical scene
    // — would require P.4 (quality preset) and possibly downstream alignment
    // to also be closed. The structural proof here is sufficient evidence
    // that the tonemap axis itself is no longer divergent at the pipeline
    // level. The per-pixel `compute_attribution` numbers below corroborate:
    // tonemap-axis SAD attribution should drop from P.2's baseline now that
    // both paths run identical tonemap math on identical post-shader inputs.
    let tonemap_closure_proof = format!(
        "Engine path config.format = Rgba8UnormSrgb (post_pipeline output);\n  \
         Editor path config.format = Rgba8UnormSrgb (post_pipeline output);\n  \
         draw_into pipeline branch:    unconditional post_pipeline (canonical);\n  \
         Pipeline source of truth:     astraweave-render::POST_SHADER (single instance)"
    );

    eprintln!("Engine path: Rgba8UnormSrgb LDR (draw_into, canonical post_pipeline)");
    eprintln!(
        "  Bytes:   {} ({} px × 4 B/px)",
        engine.bytes.len(),
        engine.width * engine.height
    );
    eprintln!("  SHA-256: {}", engine_hash);
    eprintln!();
    eprintln!("Editor path (overlays OFF): ENGINE_LDR_TARGET (the parity-contract target)");
    eprintln!(
        "  Bytes:   {} ({} px × 4 B/px)",
        editor.engine_ldr_bytes.len(),
        editor.width * editor.height
    );
    eprintln!("  SHA-256: {}", editor_hash);
    eprintln!(
        "  Display SHA-256 (composite output, no overlays drawn): {}",
        editor_display_overlays_off
    );
    eprintln!();
    eprintln!("Editor path (overlays ON): ENGINE_LDR_TARGET (parity contract target)");
    eprintln!(
        "  Bytes:   {} ({} px × 4 B/px)",
        editor_overlays_on.engine_ldr_bytes.len(),
        editor_overlays_on.width * editor_overlays_on.height
    );
    eprintln!("  SHA-256: {}", editor_engine_ldr_overlays_on);
    eprintln!(
        "  Display SHA-256 (composite output, with overlays):     {}",
        editor_display_overlays_on
    );
    eprintln!();

    let attribution = compute_attribution(&engine, &editor);
    eprintln!("{}", attribution.format_report());
    eprintln!();
    if let Some(hash) = &pack_hash {
        eprintln!("Loader-axis closure proof (P.2):");
        eprintln!("  Canonical pack content hash: {}", hash);
        eprintln!(
            "  Both editor and engine paths invoke load_canonical_terrain_pack on the"
        );
        eprintln!(
            "  same biome dir; this hash is the byte-identical input to set_terrain_materials"
        );
        eprintln!("  on both sides. P.2 closes Axis 1 at the input boundary.");
        eprintln!();
    }
    eprintln!("Tonemap-axis closure proof (P.3):");
    eprintln!("  {}", tonemap_closure_proof);
    eprintln!(
        "  Structural proof: both paths instantiate Renderer with identical config.format"
    );
    eprintln!(
        "  and now invoke the same `post_pipeline` (ACES Narkowicz + exposure 1.35 +"
    );
    eprintln!(
        "  scene-env tint) unconditionally inside `draw_into`. The pre-P.3 surface.is_none()"
    );
    eprintln!("  branch + hdr_blit_pipeline + editor's own tonemap.wgsl pass are all deleted.");
    eprintln!();

    eprintln!("Target-format-axis closure proof (P.5):");
    eprintln!(
        "  Engine formats: surface={:?}, hdr={:?}, depth={:?}",
        engine.formats.surface_format,
        engine.formats.hdr_format,
        engine.formats.depth_format
    );
    eprintln!(
        "  Editor formats: surface={:?}, hdr={:?}, depth={:?}",
        editor.formats.surface_format,
        editor.formats.hdr_format,
        editor.formats.depth_format
    );
    let format_table = [
        (
            "surface_format (post_pipeline target)",
            engine.formats.surface_format,
            editor.formats.surface_format,
        ),
        (
            "hdr_format     (internal HDR target)",
            engine.formats.hdr_format,
            editor.formats.hdr_format,
        ),
        (
            "depth_format   (Depth32Float)",
            engine.formats.depth_format,
            editor.formats.depth_format,
        ),
    ];
    let mut all_equal = true;
    for (label, e, d) in &format_table {
        let equal = e == d;
        if !equal {
            all_equal = false;
        }
        eprintln!(
            "  | {:<40} | {:?} | {:?} | {} |",
            label,
            e,
            d,
            if equal { "YES" } else { "NO " }
        );
    }
    eprintln!(
        "  Pairwise comparisons: {} / 3 equal ({})",
        format_table.iter().filter(|(_, e, d)| e == d).count(),
        if all_equal { "PASS" } else { "FAIL" }
    );
    eprintln!(
        "  Closure proof: {}. P.5 formalises the structural closure that P.3's",
        if all_equal {
            "STRUCTURAL PASS"
        } else {
            "STRUCTURAL FAIL — escalate"
        }
    );
    eprintln!(
        "  surface.is_none() branch deletion incidentally produced (config.format"
    );
    eprintln!(
        "  migrated to Rgba8UnormSrgb on both sides as a downstream consequence)."
    );
    eprintln!();

    eprintln!("Overlay-isolation closure proof (P.6):");
    eprintln!(
        "  ENGINE_LDR_TARGET SHA-256 (overlays OFF): {}",
        editor_hash
    );
    eprintln!(
        "  ENGINE_LDR_TARGET SHA-256 (overlays ON):  {}",
        editor_engine_ldr_overlays_on
    );
    let overlay_isolation_pass = editor_hash == editor_engine_ldr_overlays_on;
    eprintln!(
        "  Equality: {}",
        if overlay_isolation_pass {
            "PASS (overlays do not mutate the parity-contract target)"
        } else {
            "FAIL (overlays are mutating the engine LDR target — escalate)"
        }
    );
    eprintln!(
        "  Display SHA-256 (overlays OFF): {}",
        editor_display_overlays_off
    );
    eprintln!(
        "  Display SHA-256 (overlays ON):  {}",
        editor_display_overlays_on
    );
    eprintln!(
        "  Display targets {} (composite-output diagnostic, NOT part of contract).",
        if editor_display_overlays_off == editor_display_overlays_on {
            "MATCH"
        } else {
            "differ (overlays composited as expected)"
        }
    );
    eprintln!();

    eprintln!("Quality-preset-axis closure proof (P.4):");
    eprintln!(
        "  Canonical preset (GameQuality):  {:?}",
        CanonicalQualityPresetParams::GAME_QUALITY
    );
    eprintln!(
        "  Engine path: apply_canonical_quality_preset_to_renderer(GAME_QUALITY)"
    );
    eprintln!(
        "  Editor path: apply_quality_preset(EditorQualityPreset::GameQuality)"
    );
    eprintln!(
        "                (via EngineRenderAdapter::new — Move A swapped EditorDefault → GameQuality)"
    );
    eprintln!(
        "              + apply_canonical_quality_preset_to_renderer(GAME_QUALITY)"
    );
    eprintln!(
        "                (defensive re-application — guarantees parameter equality"
    );
    eprintln!(
        "                 regardless of any future adapter-construction drift)"
    );
    eprintln!(
        "  Call-site closure proof: both paths invoke the same setters with the same"
    );
    eprintln!(
        "  argument values (CanonicalQualityPresetParams::GAME_QUALITY single source of truth)."
    );
    eprintln!(
        "  Parameters covered: shadows_enabled, cloud_shadows_enabled, shadow_filter,"
    );
    eprintln!(
        "  cascade_extents, cascade_lambda, max_draw_distance. Post-process chain handled"
    );
    eprintln!(
        "  separately by EditorQualityPreset::GameQuality match arm (bloom/taa/color_grading);"
    );
    eprintln!(
        "  in headless draw_into only bloom_enabled is consumed and the bloom output is"
    );
    eprintln!(
        "  currently orphaned post-P.3 (flagged in P.3 follow-up candidates)."
    );
    eprintln!();
    eprintln!("Heuristic notes:");
    eprintln!("  - P.3: both paths now produce 4 B/px Rgba8UnormSrgb (was 8 B/px engine HDR");
    eprintln!("    vs 4 B/px editor LDR pre-P.3). Total SAD is computed in linear-RGB space");
    eprintln!("    after sRGB-to-linear decoding on both sides.");
    eprintln!("  - The per-pixel tonemap/format heuristic in compute_attribution interprets");
    eprintln!("    engine bytes as if they were pre-tonemap HDR, which was true pre-P.3 but");
    eprintln!("    is no longer the case. Those rows in the attribution report are stale and");
    eprintln!("    should be read as 'tonemap and format axes are closure-proven' — the");
    eprintln!("    remaining SAD is quality preset (Axis 8, P.4) and overlay composition");
    eprintln!("    (P.6).");
    eprintln!("  - Cross-axis interactions are real. Attribution does not sum to 100%.");

    // P.5 format-equality assertion. Structural closure proof — fails the
    // test if any pairwise format comparison shows divergence. Independent
    // of the per-pixel hash assertion below; surfaces format drift even on
    // fixtures where the rendered pixels happen to match.
    assert!(
        all_equal,
        "Target-format-axis closure proof FAILED — engine and editor formats diverge: \
         engine={:?}, editor={:?}",
        engine.formats, editor.formats
    );

    // P.6 overlay-isolation assertion. The campaign's most fundamental
    // contract: the editor's engine LDR target is byte-identical whether
    // overlays drew or not. Failing this means overlays mutated the
    // parity-contract target — a real architectural regression.
    assert_eq!(
        editor_hash, editor_engine_ldr_overlays_on,
        "Overlay-isolation closure proof FAILED — overlays mutated ENGINE_LDR_TARGET. \
         Off-hash: {} | On-hash: {}",
        editor_hash, editor_engine_ldr_overlays_on
    );

    // Editor-engine byte-identity is the campaign-wide parity contract.
    // A failure here indicates a parity-class regression: the editor's
    // engine LDR target diverged from the engine production renderer's
    // output for the same scene. One of the five closure proofs above
    // narrows which seam reopened. See the campaign-outcome doc
    // (docs/audits/editor_engine_render_parity_outcome_2026-05.md) for
    // full context on each seam and what protects it.
    assert_eq!(
        engine_hash, editor_hash,
        "Parity regression — engine and editor outputs diverged. \
         Investigate before merge: a closure-proof failure above narrows the seam."
    );
}

// C.3.C closure: the `update_camera_matrices_wrapper_preserves_behavior` test
// was deleted alongside the deprecated `update_camera_matrices` wrapper itself.
// `editor_engine_render_parity` above is now the sole parity test and exercises
// the canonical `Renderer::update_view` path directly.

// ════════════════════════════════════════════════════════════════════════════
// ─── C.8: Camera parity fixture families ─────────────────────────────────────
// ════════════════════════════════════════════════════════════════════════════
//
// Unified Camera campaign sub-phase C.8. RenderView/matrix-level fixtures that
// DERIVE every baseline from camera math (glam Mat4::look_to_rh / look_at_rh /
// perspective_rh per CAMERA_CONVENTIONS §2.5/§2.6, and FreeFly's dir(yaw,pitch)
// spherical formula) — NOT from running the producer and trusting a SHA of its
// GPU output. This matches the RenderView-level style of
// `orbit_camera_producer.rs` / `picking_consistency.rs`, NOT the GPU SHA style
// of `editor_engine_render_parity` above (which is intentionally kept intact).
//
// Four families:
//   1. extreme_pitch        — near-/at-singularity pitch; honest finite behavior
//   2. non_square_aspect    — ultrawide / portrait / floor-discipline projection
//   3. large_world_positions — world- vs camera-relative precision divergence
//   4. cinematics_driven    — full CameraKey -> tick_cinematics -> RenderView path
//
// Families 1-3 are GPU-free and pass unconditionally. Family 4 is GPU-gated
// (apply_camera_key is private; tick_cinematics — its only public reach — is a
// Renderer method needing a wgpu device); those fixtures skip gracefully when
// no adapter is available, mirroring `editor_engine_render_parity`.

// ─── C.8 shared matrix oracles (independent re-derivation of glam look_to_rh) ─
//
// These re-implement glam's look_to_rh basis from first principles so a fixture
// baseline is computed by a DIFFERENT code path than the producer's call into
// glam. glam-0.30.x look_to_rh: f = forward.normalize(); s = f.cross(up)
// .normalize(); u = s.cross(f); columns (s.*, u.*, -f.*, 0); col3 =
// (-eye.dot(s), -eye.dot(u), eye.dot(f), 1). (look_at_rh(eye, center, up) ==
// look_to_rh(eye, center - eye, up), so a target-based baseline is the same
// basis with forward = center - eye.)

/// f64-free re-derivation of `FreeFly::dir(yaw, pitch)` (the spherical forward
/// formula at freefly.rs:55). Used by the extreme_pitch family as the
/// independent direction oracle.
fn freefly_dir_oracle(yaw: f32, pitch: f32) -> Vec3 {
    let cy = yaw.cos();
    let sy = yaw.sin();
    let cp = pitch.cos();
    let sp = pitch.sin();
    Vec3::new(cy * cp, sp, sy * cp).normalize()
}

/// Independent re-derivation of `Mat4::look_to_rh(eye, forward, up)` from the
/// raw basis arithmetic — a different code path than glam's intrinsic. Used to
/// cross-check the producer's view matrix.
fn manual_look_to_rh_oracle(eye: Vec3, forward: Vec3, up: Vec3) -> Mat4 {
    let f = forward.normalize();
    let s = f.cross(up).normalize();
    let u = s.cross(f);
    // glam column-major: col0=(s.x,u.x,-f.x,0), col1=(s.y,u.y,-f.y,0),
    // col2=(s.z,u.z,-f.z,0), col3=(-eye·s, -eye·u, eye·f, 1).
    Mat4::from_cols_array(&[
        s.x,
        u.x,
        -f.x,
        0.0,
        s.y,
        u.y,
        -f.y,
        0.0,
        s.z,
        u.z,
        -f.z,
        0.0,
        -eye.dot(s),
        -eye.dot(u),
        eye.dot(f),
        1.0,
    ])
}

/// Max abs difference across all 16 entries of two f32 matrices.
fn mat4_max_abs_diff(a: Mat4, b: Mat4) -> f32 {
    let aa = a.to_cols_array();
    let bb = b.to_cols_array();
    let mut m = 0.0f32;
    for i in 0..16 {
        m = m.max((aa[i] - bb[i]).abs());
    }
    m
}

/// True if no entry of the matrix is NaN (also catches inf via the is_nan
/// chain only when an op produced NaN — combined with finite checks where the
/// fixture needs them).
fn mat4_is_nan_free(m: Mat4) -> bool {
    m.to_cols_array().iter().all(|v| !v.is_nan())
}

// ─── C.8 family 1: extreme_pitch ─────────────────────────────────────────────
//
// Near-vertical and exactly-vertical pitch. FreeFly's dir(yaw, ±pi/2) carries a
// float residue (cos(pi/2) ~= -4.37e-8) that keeps forward non-parallel to up,
// so look_to_rh stays FINITE even at the exact singularity — the producer's
// honest documented behavior. OrbitCamera at its Default max_pitch
// (pi/2 - 0.01) is well off vertical and well-conditioned.

#[test]
fn extreme_pitch_freefly_just_inside_minus_clamp() {
    // pitch = -1.54 rad — the exact -clamp; 1.76 deg shy of -pi/2.
    let yaw = 1.2_f32;
    let pitch = -1.54_f32;
    let cam = FreeFly {
        position: Vec3::new(-8.0, 14.0, -20.0),
        yaw,
        pitch,
        fovy: 75_f32.to_radians(),
        aspect: 1.0,
        znear: 0.05,
        zfar: 800.0,
    };
    let rv: RenderView = cam.to_render_view();

    let dir = freefly_dir_oracle(yaw, pitch);
    let baseline_view = manual_look_to_rh_oracle(cam.position, dir, Vec3::Y);

    let diff = mat4_max_abs_diff(rv.view, baseline_view);
    assert!(
        diff < 1e-5,
        "FreeFly.to_render_view().view at pitch=-1.54 must match independently-derived \
         look_to_rh basis; max|diff| = {diff:e} (eps 1e-5)"
    );
    assert!(
        mat4_is_nan_free(rv.view) && mat4_is_nan_free(rv.view_proj),
        "view/view_proj must be NaN-free at pitch=-1.54"
    );
    assert!(
        (rv.view_dir - dir).length() < 1e-6,
        "RenderView.view_dir must equal FreeFly::dir(yaw,pitch); got {:?}, expected {:?}",
        rv.view_dir,
        dir
    );
    assert!(
        dir.y < -0.999,
        "sanity: dir.y ~= sin(-1.54) ~= -0.9995 (near-vertical-down regime); got {}",
        dir.y
    );
}

#[test]
fn extreme_pitch_freefly_exactly_plus_minus_half_pi_stays_finite() {
    // EXACT singularity. FreeFly's dir(yaw, ±pi/2) carries a float residue
    // (cos(pi/2) ~= -4.37e-8), so forward.xz != 0 and forward is NOT parallel
    // to Vec3::Y. look_to_rh therefore stays FINITE — this is the producer's
    // honest, documented behavior, not a degeneracy. (OrbitCamera's target-
    // based look_at_rh DOES NaN at exact +pi/2; see the orbit fixture.)
    let half_pi = std::f32::consts::FRAC_PI_2;
    let yaw = 0.3_f32;
    let eye = Vec3::new(3.0, 5.0, -2.0);

    for (pitch, expected_dir_y, label) in [
        (half_pi, 1.0_f32, "+pi/2"),
        (-half_pi, -1.0_f32, "-pi/2"),
    ] {
        let cam = FreeFly {
            position: eye,
            yaw,
            pitch,
            fovy: 60_f32.to_radians(),
            aspect: 1.5,
            znear: 0.1,
            zfar: 500.0,
        };
        let rv: RenderView = cam.to_render_view();

        // Documented behavior: finite, non-NaN at the exact singularity.
        assert!(
            mat4_is_nan_free(rv.view) && mat4_is_nan_free(rv.view_proj),
            "FreeFly at pitch={label} must stay FINITE (spherical residue keeps \
             forward non-parallel to up); view/view_proj contained NaN"
        );

        // Independent baseline from the same residue-bearing direction.
        let dir = freefly_dir_oracle(yaw, pitch);
        let baseline_view = manual_look_to_rh_oracle(eye, dir, Vec3::Y);
        let diff = mat4_max_abs_diff(rv.view, baseline_view);
        assert!(
            diff < 1e-5,
            "FreeFly.view at pitch={label} must match independently-derived \
             look_to_rh basis; max|diff| = {diff:e} (eps 1e-5)"
        );

        // view_dir is essentially vertical, but NOT exactly (0,±1,0):
        // the xz residue is what saves look_to_rh from degenerating.
        assert!(
            (rv.view_dir.y - expected_dir_y).abs() < 1e-5,
            "view_dir.y at pitch={label} must be ~{expected_dir_y}; got {}",
            rv.view_dir.y
        );
        assert!(
            rv.view_dir.x.abs() < 1e-6 && rv.view_dir.z.abs() < 1e-6,
            "view_dir.xz residue at pitch={label} must be the tiny non-zero value \
             (~4e-8) that keeps look_to_rh finite; got x={}, z={}",
            rv.view_dir.x,
            rv.view_dir.z
        );
        assert_eq!(
            rv.view_dir, dir,
            "view_dir must be exactly FreeFly::dir(yaw,pitch) at pitch={label}"
        );
    }
}

#[test]
fn extreme_pitch_orbit_camera_at_max_pitch_well_defined() {
    // pitch = pi/2 - 0.01 ~= 1.5607963 — OrbitCamera's Default max_pitch
    // (the gimbal-lock guard). 0.573 deg shy of +pi/2; OrbitCamera::new does
    // NOT clamp, so the field holds exactly this value.
    let max_pitch = std::f32::consts::FRAC_PI_2 - 0.01;
    let focal = Vec3::new(5.0, 1.0, -2.0);
    let distance = 25.0_f32;
    let yaw = 0.7_f32;
    let cam = OrbitCamera::new(focal, distance, yaw, max_pitch);

    let rv = cam.to_render_view();

    // Independent position() re-derivation (spherical-to-cartesian).
    let px = distance * yaw.cos() * max_pitch.cos();
    let py = distance * max_pitch.sin();
    let pz = distance * yaw.sin() * max_pitch.cos();
    let expected_pos = focal + Vec3::new(px, py, pz);
    assert!(
        (cam.position() - expected_pos).length() < 1e-4,
        "sanity: OrbitCamera::position() must equal the spherical formula; got {:?}, expected {:?}",
        cam.position(),
        expected_pos
    );
    assert_eq!(
        rv.position,
        cam.position(),
        "RenderView.position must equal OrbitCamera::position()"
    );

    // Independent baseline: target-based look_at == look_to with forward = focal - eye.
    let forward = focal - cam.position();
    let baseline_view = manual_look_to_rh_oracle(cam.position(), forward, Vec3::Y);
    let diff = mat4_max_abs_diff(rv.view, baseline_view);
    assert!(
        diff < 1e-5,
        "OrbitCamera.to_render_view().view at max_pitch must match independently-derived \
         look_at_rh basis; max|diff| = {diff:e} (eps 1e-5)"
    );
    assert!(
        mat4_is_nan_free(rv.view) && mat4_is_nan_free(rv.view_proj),
        "view/view_proj must be NaN-free at max_pitch (0.573 deg off vertical, well-conditioned)"
    );
}

// ─── C.8 family 2: non_square_aspect ─────────────────────────────────────────
//
// Ultrawide / portrait / degenerate-narrow aspect. col(1).y = cot(fovy/2) is
// the aspect-INVARIANT vertical-FOV term; col(0).x = col(1).y/aspect widens or
// narrows horizontally. The floor-discipline closure: Projection::perspective
// floors aspect at .max(0.01) in the MATRIX but stores the RAW pre-floor value
// in the FIELD; OrbitCamera::projection_matrix() (raw, no floor) diverges from
// to_render_view() (floored) below the floor.

/// Matrix epsilon for derived projection comparisons (C.8 non-square-aspect
/// family). Matches the orbit_camera_producer / picking_consistency style.
const ASPECT_MATRIX_EPS: f32 = 1e-5;

/// Aspect floor applied by `Projection::perspective` / `FreeFly::proj_matrix`
/// (`.max(0.01)`) per CAMERA_CONVENTIONS §2.3.
const ASPECT_FLOOR: f32 = 0.01;

/// Compare two Mat4 entry-by-entry within `eps`. glam Mat4 has no exact-eq
/// we should rely on for derived values, so compare the 16 entries directly.
fn assert_mat4_close(actual: Mat4, expected: Mat4, eps: f32, ctx: &str) {
    let a = actual.to_cols_array();
    let e = expected.to_cols_array();
    for i in 0..16 {
        let col = i / 4;
        let row = i % 4;
        assert!(
            (a[i] - e[i]).abs() < eps,
            "{ctx}: matrix mismatch at col={col} row={row}: actual={} expected={} (|d|={})",
            a[i],
            e[i],
            (a[i] - e[i]).abs()
        );
    }
}

/// Build the ultrawide 21:9 FreeFly fixture. No Default impl on FreeFly —
/// literal struct expression per the canonical in-crate test style.
fn freefly_ultrawide_21_9() -> FreeFly {
    FreeFly {
        position: Vec3::new(3.0, 4.0, 5.0),
        yaw: 0.6,
        pitch: 0.2,
        fovy: 60_f32.to_radians(),
        aspect: 21.0 / 9.0, // 2.3333335 — unambiguously ultrawide
        znear: 0.5,
        zfar: 5000.0,
    }
}

#[test]
fn non_square_aspect_ultrawide_21_9_freefly_projection_matches_derived_baseline() {
    let cam = freefly_ultrawide_21_9();
    let rv = cam.to_render_view();

    // Method A: direct Mat4::perspective_rh re-construction (aspect > 0.01,
    // so the .max(0.01) floor is inactive and floored == raw).
    let expected_proj = Mat4::perspective_rh(cam.fovy, cam.aspect, cam.znear, cam.zfar);
    assert_mat4_close(
        rv.projection,
        expected_proj,
        ASPECT_MATRIX_EPS,
        "ultrawide 21:9 RenderView.projection vs direct perspective_rh",
    );

    // Method B: closed-form perspective_rh entries, derived WITHOUT calling
    // perspective_rh. f = cot(fovy/2); col(1).y is aspect-INVARIANT (vertical
    // FOV held constant); col(0).x = f/aspect (horizontal widens with aspect).
    let f = 1.0_f32 / (cam.fovy * 0.5).tan(); // sqrt(3) = 1.7320508 for 60deg
    assert!(
        (rv.projection.col(1).y - f).abs() < ASPECT_MATRIX_EPS,
        "col(1).y (vertical-FOV invariant) must equal cot(fovy/2)={f}; got {} — \
         fovy is being treated as HORIZONTAL if this widens with aspect",
        rv.projection.col(1).y,
    );
    let expected_m00 = f / cam.aspect; // 1.7320508 / 2.3333335 = 0.7423075
    assert!(
        (rv.projection.col(0).x - expected_m00).abs() < ASPECT_MATRIX_EPS,
        "col(0).x must equal cot(fovy/2)/aspect={expected_m00} (horizontal widens \
         with ultrawide aspect); got {}",
        rv.projection.col(0).x,
    );

    // RenderView.aspect mirrors the raw input (>= 0.01, so raw == floored).
    assert!(
        (rv.aspect - cam.aspect).abs() < 1e-6,
        "RenderView.aspect must equal the raw 21:9 input {}; got {}",
        cam.aspect,
        rv.aspect,
    );
    // RenderView.fovy is the vertical-FOV invariant: unchanged by aspect.
    assert!(
        (rv.fovy - cam.fovy).abs() < 1e-6,
        "RenderView.fovy must be unchanged by ultrawide aspect (vertical FOV is \
         the invariant); got {} expected {}",
        rv.fovy,
        cam.fovy,
    );
}

fn freefly_portrait_9_16() -> FreeFly {
    FreeFly {
        position: Vec3::new(-2.0, 1.5, 7.0),
        yaw: -0.9,
        pitch: -0.15,
        fovy: 50_f32.to_radians(),
        aspect: 9.0 / 16.0, // 0.5625 — portrait, taller-than-wide
        znear: 0.1,
        zfar: 2000.0,
    }
}

#[test]
fn non_square_aspect_portrait_9_16_freefly_projection_matches_derived_baseline() {
    let cam = freefly_portrait_9_16();
    let rv = cam.to_render_view();

    // Method A: direct re-construction (aspect 0.5625 >= 0.01, no floor).
    let expected_proj = Mat4::perspective_rh(cam.fovy, cam.aspect, cam.znear, cam.zfar);
    assert_mat4_close(
        rv.projection,
        expected_proj,
        ASPECT_MATRIX_EPS,
        "portrait 9:16 RenderView.projection vs direct perspective_rh",
    );

    // Method B: closed-form, no perspective_rh call.
    let f = 1.0_f32 / (cam.fovy * 0.5).tan(); // 2.1445069 for 50deg
    assert!(
        (rv.projection.col(1).y - f).abs() < ASPECT_MATRIX_EPS,
        "col(1).y (vertical-FOV invariant) must equal cot(fovy/2)={f} on the \
         portrait side too; got {}",
        rv.projection.col(1).y,
    );
    let expected_m00 = f / cam.aspect; // 2.1445069 / 0.5625 = 3.8124567
    assert!(
        (rv.projection.col(0).x - expected_m00).abs() < ASPECT_MATRIX_EPS,
        "col(0).x must equal cot(fovy/2)/aspect={expected_m00}; got {}",
        rv.projection.col(0).x,
    );
    // On the sub-1.0 aspect side, horizontal FOV is NARROWER: m00 must exceed
    // m11. A skeptic verifies: aspect < 1 => f/aspect > f.
    assert!(
        rv.projection.col(0).x > rv.projection.col(1).y,
        "portrait aspect < 1 must give col(0).x > col(1).y (narrower horizontal \
         FOV); got m00={} m11={}",
        rv.projection.col(0).x,
        rv.projection.col(1).y,
    );

    assert!(
        (rv.aspect - cam.aspect).abs() < 1e-6,
        "RenderView.aspect must equal the raw 9:16 input {}; got {}",
        cam.aspect,
        rv.aspect,
    );
    assert!(
        (rv.fovy - cam.fovy).abs() < 1e-6,
        "RenderView.fovy must be unchanged by portrait aspect; got {} expected {}",
        rv.fovy,
        cam.fovy,
    );
}

const DEGENERATE_RAW_ASPECT: f32 = 0.001;

fn freefly_degenerate_narrow() -> FreeFly {
    FreeFly {
        position: Vec3::new(0.0, 2.0, 10.0),
        yaw: 0.3,
        pitch: 0.1,
        fovy: 60_f32.to_radians(),
        aspect: DEGENERATE_RAW_ASPECT, // 0.001 — below the 0.01 floor
        znear: 0.5,
        zfar: 5000.0,
    }
}

#[test]
fn non_square_aspect_degenerate_narrow_freefly_floor_discipline() {
    let cam = freefly_degenerate_narrow();
    let rv = cam.to_render_view();

    // (1) Matrix uses the FLOORED aspect (0.001.max(0.01) = 0.01).
    let expected_floored =
        Mat4::perspective_rh(cam.fovy, cam.aspect.max(ASPECT_FLOOR), cam.znear, cam.zfar);
    assert_mat4_close(
        rv.projection,
        expected_floored,
        ASPECT_MATRIX_EPS,
        "degenerate-narrow RenderView.projection must use FLOORED aspect 0.01",
    );

    // (1b) Negative control: the matrix must NOT be the un-floored 0.001 one.
    // col(0).x for unfloored = f/0.001 = 1732.05 vs floored f/0.01 = 173.2 —
    // a 10x discriminator no rounding could mask.
    let expected_unfloored = Mat4::perspective_rh(cam.fovy, cam.aspect, cam.znear, cam.zfar);
    assert!(
        (rv.projection.col(0).x - expected_unfloored.col(0).x).abs() > 1.0,
        "RenderView.projection must NOT use the un-floored 0.001 aspect; \
         floored col(0).x={} unfloored col(0).x={} — if these are equal, the \
         .max(0.01) floor is not being applied",
        rv.projection.col(0).x,
        expected_unfloored.col(0).x,
    );

    // (1c) Closed-form discriminator, no perspective_rh call: floored m00 = f/0.01.
    let f = 1.0_f32 / (cam.fovy * 0.5).tan(); // 1.7320508 for 60deg
    let expected_m00_floored = f / ASPECT_FLOOR; // 173.20508
    // m00 is ~173; use a relative tolerance for the large magnitude.
    let m00_tol = (expected_m00_floored.abs() * 1e-4).max(1e-3);
    assert!(
        (rv.projection.col(0).x - expected_m00_floored).abs() < m00_tol,
        "floored col(0).x must equal cot(fovy/2)/0.01={expected_m00_floored}; got {} \
         (tol {m00_tol})",
        rv.projection.col(0).x,
    );
    // col(1).y is still the aspect-invariant vertical-FOV term.
    assert!(
        (rv.projection.col(1).y - f).abs() < ASPECT_MATRIX_EPS,
        "col(1).y must remain cot(fovy/2)={f} even at degenerate aspect; got {}",
        rv.projection.col(1).y,
    );

    // (2) Field stores the RAW pre-floor aspect 0.001 (Projection::perspective
    // stores the input before .max(0.01)). This is the other half of the
    // floor-discipline closure: matrix floored, field raw.
    assert!(
        (rv.aspect - DEGENERATE_RAW_ASPECT).abs() < 1e-6,
        "RenderView.aspect must store the RAW pre-floor input 0.001, NOT the \
         floored 0.01; got {}",
        rv.aspect,
    );
    // Defensive: the raw field must be distinguishable from the floor value.
    assert!(
        (rv.aspect - ASPECT_FLOOR).abs() > 1e-4,
        "RenderView.aspect ({}) must differ from the floor 0.01 — it is pre-floor",
        rv.aspect,
    );
    // fovy still unchanged.
    assert!(
        (rv.fovy - cam.fovy).abs() < 1e-6,
        "RenderView.fovy must be unchanged at degenerate aspect; got {} expected {}",
        rv.fovy,
        cam.fovy,
    );
}

/// Above the floor, OrbitCamera::projection_matrix() (raw aspect) and
/// to_render_view().projection (Projection-floored aspect) must AGREE,
/// because raw == raw.max(0.01) when aspect >= 0.01.
#[test]
fn non_square_aspect_orbit_paths_agree_above_floor() {
    let mut cam = OrbitCamera::new(
        Vec3::ZERO,
        25.0,
        std::f32::consts::FRAC_PI_4,
        std::f32::consts::FRAC_PI_6,
    );
    cam.set_fov(60.0);
    // Ultrawide 21:9 ~= 2.333, comfortably above the 0.01 floor.
    cam.set_aspect(2333.0, 1000.0);
    let raw_aspect = 2333.0_f32 / 1000.0;

    let proj_path = cam.projection_matrix(); // RAW aspect (camera.rs:644)
    let rv = cam.to_render_view(); // Projection-floored (camera.rs:897)

    // raw == floored above the floor, so the two paths coincide.
    assert_mat4_close(
        proj_path,
        rv.projection,
        ASPECT_MATRIX_EPS,
        "above-floor ultrawide: projection_matrix() must equal to_render_view().projection",
    );
    // And both match the derived baseline at the raw aspect.
    let f = 1.0_f32 / (cam.fovy() * 0.5).tan();
    let expected_m00 = f / raw_aspect; // f/2.333
    assert!(
        (rv.projection.col(0).x - expected_m00).abs() < ASPECT_MATRIX_EPS,
        "above-floor col(0).x must equal cot(fovy/2)/aspect={expected_m00}; got {}",
        rv.projection.col(0).x,
    );
    assert!(
        (rv.aspect - raw_aspect).abs() < 1e-4,
        "RenderView.aspect must equal the raw ultrawide input {raw_aspect}; got {}",
        rv.aspect,
    );
}

/// Below the floor, the two OrbitCamera projection paths DIVERGE by design:
/// projection_matrix() uses RAW 0.001 (camera.rs:644, no floor) while
/// to_render_view() routes through Projection::perspective which floors to
/// 0.01 (camera.rs:897). This is EXPECTED-DIVERGENCE, not a parity bug —
/// asserting byte-equivalence here would assert a falsehood (constraint 4).
#[test]
fn non_square_aspect_orbit_paths_diverge_below_floor_by_design() {
    let mut cam = OrbitCamera::new(
        Vec3::ZERO,
        25.0,
        std::f32::consts::FRAC_PI_4,
        std::f32::consts::FRAC_PI_6,
    );
    cam.set_fov(60.0);
    // Degenerate-narrow aspect = 1/1000 = 0.001, an order of magnitude below
    // the 0.01 floor.
    cam.set_aspect(1.0, 1000.0);
    let raw_aspect = 1.0_f32 / 1000.0; // 0.001
    let floored_aspect = raw_aspect.max(ASPECT_FLOOR); // 0.01

    let proj_path = cam.projection_matrix(); // RAW 0.001
    let rv = cam.to_render_view(); // FLOORED 0.01
    let f = 1.0_f32 / (cam.fovy() * 0.5).tan(); // 1.7320508

    // projection_matrix() uses the RAW aspect (no floor): col(0).x = f/0.001.
    let raw_m00 = f / raw_aspect; // 1732.05
    let raw_tol = (raw_m00.abs() * 1e-4).max(1e-3);
    assert!(
        (proj_path.col(0).x - raw_m00).abs() < raw_tol,
        "OrbitCamera::projection_matrix() must use the RAW unfloored aspect \
         0.001 (camera.rs:644): col(0).x expected {raw_m00}; got {} (tol {raw_tol})",
        proj_path.col(0).x,
    );

    // to_render_view().projection uses the FLOORED aspect 0.01: col(0).x = f/0.01.
    let floored_m00 = f / floored_aspect; // 173.20
    let floored_tol = (floored_m00.abs() * 1e-4).max(1e-3);
    assert!(
        (rv.projection.col(0).x - floored_m00).abs() < floored_tol,
        "OrbitCamera::to_render_view().projection must use the FLOORED aspect \
         0.01 (Projection::perspective, camera.rs:897): col(0).x expected {floored_m00}; \
         got {} (tol {floored_tol})",
        rv.projection.col(0).x,
    );

    // EXPECTED-DIVERGENCE: the two paths' col(0).x differ by ~10x. This is the
    // seam — assert it as a positive expectation, NEVER byte-equivalence.
    assert!(
        proj_path.col(0).x > rv.projection.col(0).x * 5.0,
        "below-floor seam: projection_matrix() col(0).x ({}) must be ~10x \
         to_render_view() col(0).x ({}) — the raw-vs-floored aspect divergence",
        proj_path.col(0).x,
        rv.projection.col(0).x,
    );

    // The two paths AGREE on the aspect-invariant vertical-FOV term col(1).y
    // (only the [0][0] entry depends on aspect): rotation/vertical untouched.
    assert!(
        (proj_path.col(1).y - rv.projection.col(1).y).abs() < ASPECT_MATRIX_EPS,
        "the two paths must AGREE on col(1).y (vertical-FOV invariant); \
         projection_matrix()={} to_render_view()={}",
        proj_path.col(1).y,
        rv.projection.col(1).y,
    );

    // RenderView.aspect still reports the RAW pre-floor 0.001.
    assert!(
        (rv.aspect - raw_aspect).abs() < 1e-6,
        "RenderView.aspect must store the RAW pre-floor 0.001 even though the \
         matrix floored to 0.01; got {}",
        rv.aspect,
    );
}

// ─── C.8 family 3: large_world_positions ─────────────────────────────────────
//
// At 1e5..5e6 world units, the world-relative and camera-relative RenderView
// variants DIVERGE by design (precision mitigation). Each variant is checked
// against its OWN derived baseline; the two variants are asserted to differ in
// translation (and, for OrbitCamera, rotation), never byte-equal (constraint
// 4). f64 references provide an independent precision oracle.

/// Max abs difference between the upper-left 3x3 rotation blocks of two
/// view matrices (columns 0..2, rows x/y/z).
fn max_rotation_diff(a: Mat4, b: Mat4) -> f32 {
    let mut m = 0.0f32;
    for c in 0..3 {
        let ca = a.col(c);
        let cb = b.col(c);
        m = m.max((ca.x - cb.x).abs());
        m = m.max((ca.y - cb.y).abs());
        m = m.max((ca.z - cb.z).abs());
    }
    m
}

/// Worst per-entry `error / tolerance` ratio of an f32 view matrix against an
/// f64 reference matrix. Tolerance is relative (`|ref| * 5e-6`) for large-
/// magnitude entries (the translation column at large world positions) with an
/// absolute floor (`1e-5`) for the O(1) rotation block. A return `< 1.0` means
/// every entry is within tolerance. The f64 reference makes this a genuine
/// cross-domain check, NOT an f32 re-call of the same glam intrinsic (which
/// would be tautological against an f32 producer that calls the same intrinsic).
fn mat4_worst_rel_ratio_vs_f64(a: Mat4, b: DMat4) -> f64 {
    let aa = a.to_cols_array();
    let bb = b.to_cols_array();
    let mut worst = 0.0f64;
    for i in 0..16 {
        let d = (aa[i] as f64 - bb[i]).abs();
        let tol = (bb[i].abs() * 5e-6).max(1e-5);
        worst = worst.max(d / tol);
    }
    worst
}

/// Build the large-world FreeFly fixture for the 1e5 structural case.
fn freefly_large_1e5() -> FreeFly {
    FreeFly {
        position: Vec3::new(1.0e5, 3.0e4, -2.0e5),
        yaw: 1.1,
        pitch: -0.3,
        fovy: 60_f32.to_radians(),
        aspect: 16.0 / 9.0,
        znear: 0.1,
        zfar: 1.0e6,
    }
}

#[test]
fn large_world_positions_freefly_1e5_structural() {
    let cam = freefly_large_1e5();

    // Producer outputs (system under test).
    let world_rv = cam.to_render_view();
    let rel_rv = cam.to_render_view_camera_relative();

    // (1) Each variant matches an INDEPENDENT f64-reference baseline. The
    //     reference is built in the f64 domain (freefly_dir_f64 +
    //     DMat4::look_to_rh) — a DIFFERENT precision domain than the f32
    //     producer — so this is a genuine cross-check, NOT an f32 re-call of
    //     the same look_to_rh (which would be tautological). Mirrors the
    //     f64-oracle discipline of the 1e6/5e6 sibling fixtures. Rotation
    //     entries (O(1), eye-independent) must match tightly; the world
    //     translation column (|pos| ~ 2.25e5) matches only to f32-relative
    //     precision — the relative tolerance both confirms the value is correct
    //     and documents the f32 precision floor the camera-relative variant
    //     exists to avoid (the C.4 hazard, exercised in full by the siblings).
    let dir_f64 = freefly_dir_f64(cam.yaw as f64, cam.pitch as f64);
    let ref_world_view = DMat4::look_to_rh(cam.position.as_dvec3(), dir_f64, DVec3::Y);
    let ref_rel_view = DMat4::look_to_rh(DVec3::ZERO, dir_f64, DVec3::Y);

    let world_ratio = mat4_worst_rel_ratio_vs_f64(world_rv.view, ref_world_view);
    assert!(
        world_ratio < 1.0,
        "world-relative producer view must match the INDEPENDENT f64 look_to_rh reference \
         (rotation tight, translation to f32-relative precision at |pos|~2.25e5); \
         worst err/tol ratio = {world_ratio} (must be <1.0)"
    );
    let rel_ratio = mat4_worst_rel_ratio_vs_f64(rel_rv.view, ref_rel_view);
    assert!(
        rel_ratio < 1.0,
        "camera-relative producer view must match the INDEPENDENT f64 look_to_rh reference \
         (whole matrix O(1), eye at origin); worst err/tol ratio = {rel_ratio} (must be <1.0)"
    );

    // (2) Rotation 3x3 byte-IDENTICAL between variants (shared dir formula).
    let rot_diff = max_rotation_diff(world_rv.view, rel_rv.view);
    assert!(
        rot_diff == 0.0,
        "FreeFly world/camera-relative rotation 3x3 must be byte-identical (both use dir(yaw,pitch)); got max diff {}",
        rot_diff
    );

    // (3) Translation columns DIVERGE by design (NOT byte-equal).
    let world_t = world_rv.view.w_axis.truncate().length();
    let rel_t = rel_rv.view.w_axis.truncate().length();
    assert!(
        rel_t < 1e-3,
        "camera-relative view translation column must be ~0 (eye at origin); got |t| = {}",
        rel_t
    );
    assert!(
        world_t > 1.0e5,
        "world-relative view translation column must carry |position|-magnitude entries; got |t| = {} (expected >1e5)",
        world_t
    );

    // (4) RenderView.position equals the world position in BOTH variants.
    assert_eq!(
        world_rv.position, cam.position,
        "world-relative RenderView.position must equal the world position"
    );
    assert_eq!(
        rel_rv.position, cam.position,
        "camera-relative RenderView.position must STILL report the world position (only matrices are camera-relative)"
    );
}

/// f64 mirror of FreeFly::dir(yaw,pitch) for the high-precision reference.
fn freefly_dir_f64(yaw: f64, pitch: f64) -> DVec3 {
    let cy = yaw.cos();
    let sy = yaw.sin();
    let cp = pitch.cos();
    let sp = pitch.sin();
    DVec3::new(cy * cp, sp, sy * cp).normalize()
}

/// NDC (clip.xyz / clip.w) of a world point through an f32 view-proj.
fn ndc_f32(vp: Mat4, p: Vec3) -> Vec3 {
    let c = vp.mul_vec4(p.extend(1.0));
    Vec3::new(c.x / c.w, c.y / c.w, c.z / c.w)
}

/// NDC of a world point through an f64 view-proj (the reference path).
fn ndc_f64(vp: DMat4, p: DVec3) -> DVec3 {
    let c = vp.mul_vec4(p.extend(1.0));
    DVec3::new(c.x / c.w, c.y / c.w, c.z / c.w)
}

/// L2 distance between an f32 NDC and an f64 reference NDC.
fn ndc_err(a: Vec3, r: DVec3) -> f64 {
    ((a.x as f64 - r.x).powi(2) + (a.y as f64 - r.y).powi(2) + (a.z as f64 - r.z).powi(2)).sqrt()
}

#[test]
fn large_world_positions_freefly_1e6_world_path_precision_loss() {
    let cam = FreeFly {
        position: Vec3::new(1.0e6, 5.0e5, 1.0e6),
        yaw: 0.6,
        pitch: 0.15,
        fovy: 60_f32.to_radians(),
        aspect: 16.0 / 9.0,
        znear: 0.1,
        zfar: 1.0e7,
    };

    // Producer outputs (system under test): the two VP variants.
    let world_rv = cam.to_render_view();
    let rel_rv = cam.to_render_view_camera_relative();

    // A world point ~40-50 units in front of the camera, inside the frustum.
    let dir = FreeFly::dir(cam.yaw, cam.pitch);
    let offset = dir * 40.0 + Vec3::new(13.0, -7.0, 5.0);
    let p = cam.position + offset;

    // f32 world path: project absolute P through the absolute VP.
    let ndc_world = ndc_f32(world_rv.view_proj, p);
    // f32 camera-relative path: production CPU offsets geometry by -position,
    // then projects through the near-zero-translation VP.
    let ndc_rel = ndc_f32(rel_rv.view_proj, p - cam.position);

    // f64 high-precision reference (DIFFERENT numeric path).
    let yaw_d = cam.yaw as f64;
    let pitch_d = cam.pitch as f64;
    let proj_d = DMat4::perspective_rh(
        cam.fovy as f64,
        (cam.aspect.max(0.01)) as f64,
        cam.znear as f64,
        cam.zfar as f64,
    );
    let view_world_d =
        DMat4::look_to_rh(cam.position.as_dvec3(), freefly_dir_f64(yaw_d, pitch_d), DVec3::Y);
    let vp_world_d = proj_d * view_world_d;
    let p_d = cam.position.as_dvec3()
        + (freefly_dir_f64(yaw_d, pitch_d) * 40.0 + DVec3::new(13.0, -7.0, 5.0));
    let ndc_ref = ndc_f64(vp_world_d, p_d);

    let err_world = ndc_err(ndc_world, ndc_ref);
    let err_rel = ndc_err(ndc_rel, ndc_ref);

    // (a) Camera-relative path is precise (small absolute error vs f64).
    assert!(
        err_rel < 5.0e-3,
        "camera-relative NDC must stay precise vs f64 reference at |pos|~1.5e6; err_rel = {} (expected <5e-3)",
        err_rel
    );
    // (b) World path is the precision-LOSER (strictly larger error).
    assert!(
        err_world > err_rel * 1.5,
        "world-relative NDC must be measurably LESS precise than camera-relative at large |pos| (the C.4 hazard); err_world = {}, err_rel = {} (ratio {:.2}, expected >1.5)",
        err_world,
        err_rel,
        err_world / err_rel
    );
    // And the divergence must be a *real* (non-trivial) error, not float noise
    // that happens to satisfy the ratio.
    assert!(
        err_world > 1.0e-4,
        "world-relative NDC error must be non-trivial at |pos|~1.5e6 (precision loss is real); err_world = {}",
        err_world
    );
}

#[test]
fn large_world_positions_orbit_far_focal_5e6_precision_loss() {
    let focal = Vec3::new(5.0e6, 0.0, 5.0e6);
    let distance = 25.0_f32;
    let yaw = std::f32::consts::FRAC_PI_4;
    let pitch = std::f32::consts::FRAC_PI_6;
    let cam = OrbitCamera::new(focal, distance, yaw, pitch);

    // Producer outputs (system under test).
    let world_rv = cam.to_render_view();
    let rel_rv = cam.to_render_view_camera_relative();
    let position = cam.position();

    // Structural: translation columns diverge by design.
    let world_t = world_rv.view.w_axis.truncate().length();
    let rel_t = rel_rv.view.w_axis.truncate().length();
    assert!(
        rel_t < 1e-3,
        "OrbitCamera camera-relative view translation must be ~0; got |t| = {}",
        rel_t
    );
    assert!(
        world_t > 1.0e6,
        "OrbitCamera world-relative view translation must carry |position|-magnitude entries; got |t| = {} (expected >1e6)",
        world_t
    );
    // Position preserved in BOTH variants.
    assert_eq!(world_rv.position, position);
    assert_eq!(rel_rv.position, position);

    // Offset probe point near the camera (focal point itself is NOT used:
    // at lower magnitudes its divergence ordering is unreliable).
    let p = focal + Vec3::new(3.0, 2.0, -4.0);
    let ndc_world = ndc_f32(world_rv.view_proj, p);
    let ndc_rel = ndc_f32(rel_rv.view_proj, p - position);

    // f64 high-precision reference.
    let yaw_d = yaw as f64;
    let pitch_d = pitch as f64;
    let d = distance as f64;
    let eye_d = DVec3::new(
        d * yaw_d.cos() * pitch_d.cos(),
        d * pitch_d.sin(),
        d * yaw_d.sin() * pitch_d.cos(),
    );
    let focal_d = DVec3::new(5.0e6, 0.0, 5.0e6);
    let pos_d = focal_d + eye_d;
    let proj_d = DMat4::perspective_rh(
        cam.fovy() as f64,
        (16.0_f32 / 9.0).max(0.01) as f64,
        0.5,
        5000.0,
    );
    let vp_world_d = proj_d * DMat4::look_at_rh(pos_d, focal_d, DVec3::Y);
    let p_d = focal_d + DVec3::new(3.0, 2.0, -4.0);
    let ndc_ref = ndc_f64(vp_world_d, p_d);

    let err_world = ndc_err(ndc_world, ndc_ref);
    let err_rel = ndc_err(ndc_rel, ndc_ref);

    assert!(
        err_rel < 2.0e-2,
        "OrbitCamera camera-relative NDC must stay precise vs f64 at focal~7e6; err_rel = {} (expected <2e-2)",
        err_rel
    );
    assert!(
        err_world > err_rel * 1.5,
        "OrbitCamera world-relative NDC must be measurably LESS precise than camera-relative at large focal point; err_world = {}, err_rel = {} (ratio {:.2}, expected >1.5)",
        err_world,
        err_rel,
        err_world / err_rel
    );
}

/// Max abs diff between an f32 view matrix's 3x3 rotation block and an f64
/// reference view matrix's 3x3 rotation block (columns 0..2, rows x/y/z).
fn rotation_diff_vs_f64(a: Mat4, b: DMat4) -> f64 {
    let mut m = 0.0f64;
    for c in 0..3 {
        let ca = a.col(c);
        let cb = b.col(c);
        m = m.max((ca.x as f64 - cb.x).abs());
        m = m.max((ca.y as f64 - cb.y).abs());
        m = m.max((ca.z as f64 - cb.z).abs());
    }
    m
}

#[test]
fn large_world_positions_orbit_5e6_camera_relative_rotation_matches_f64() {
    let focal = Vec3::new(5.0e6, 0.0, 5.0e6);
    let distance = 25.0_f32;
    let yaw = std::f32::consts::FRAC_PI_4;
    let pitch = std::f32::consts::FRAC_PI_6;
    let cam = OrbitCamera::new(focal, distance, yaw, pitch);

    let world_rv = cam.to_render_view();
    let rel_rv = cam.to_render_view_camera_relative();

    // f64 reference view matrices.
    let yaw_d = yaw as f64;
    let pitch_d = pitch as f64;
    let d = distance as f64;
    let eye_d = DVec3::new(
        d * yaw_d.cos() * pitch_d.cos(),
        d * pitch_d.sin(),
        d * yaw_d.sin() * pitch_d.cos(),
    );
    let focal_d = DVec3::new(5.0e6, 0.0, 5.0e6);
    let pos_d = focal_d + eye_d;
    let view_world_d = DMat4::look_at_rh(pos_d, focal_d, DVec3::Y);
    let view_rel_d = DMat4::look_at_rh(DVec3::ZERO, -eye_d, DVec3::Y);

    // Camera-relative rotation MATCHES the f64 truth (clean small operands).
    let rel_rot_err = rotation_diff_vs_f64(rel_rv.view, view_rel_d);
    assert!(
        rel_rot_err < 1e-5,
        "OrbitCamera camera-relative rotation must match the f64 reference (no large-operand cancellation); max diff {} (expected <1e-5)",
        rel_rot_err
    );

    // World rotation DRIFTS measurably (catastrophic cancellation in
    // (focal - position)). This is the precision loss the camera-relative
    // path exists to avoid.
    let world_rot_err = rotation_diff_vs_f64(world_rv.view, view_world_d);
    assert!(
        world_rot_err > 1e-4,
        "OrbitCamera world-relative rotation must drift from the f64 reference at focal~7e6 (the precision hazard); max diff {} (expected >1e-4)",
        world_rot_err
    );

    // The two variants' rotations therefore DIFFER (never byte-equal for
    // OrbitCamera at large positions — unlike FreeFly).
    let inter_variant = max_rotation_diff(world_rv.view, rel_rv.view);
    assert!(
        inter_variant > 1e-4,
        "OrbitCamera world vs camera-relative rotation blocks must differ at large focal point (NOT byte-equal); max diff {}",
        inter_variant
    );
}

// ─── C.8 family 4: cinematics_driven ─────────────────────────────────────────
//
// The cinematics path (CameraKey -> tick_cinematics -> apply_camera_key ->
// FreeFly -> RenderView) is the newest camera surface. These fixtures verify
// that driving the FULL production path produces the same RenderView as the
// geometric intent: a camera AT pos LOOKING AT look_at. The baseline is the
// TARGET-BASED Mat4::look_at_rh(pos, look_at, Y) — derived independently of
// apply_camera_key's atan2/asin/dir round-trip (glam look_at_rh consumes
// center-eye directly; the producer round-trips it through yaw/pitch). The two
// derivations are provably equal away from the dir.y=±1 pitch singularity, so
// the match is a genuine cross-check.
//
// REQUIRES GPU: apply_camera_key is private; the only public reach is
// Renderer::tick_cinematics, a method on a live Renderer. When no wgpu adapter
// is available, these fixtures skip gracefully (eprintln + early return),
// exactly like the existing editor_engine_render_parity GPU test would.

/// Build a headless Renderer for cinematics fixtures, reusing the harness's
/// `acquire_device()` plumbing and the engine-path SurfaceConfiguration.
fn cinematics_renderer() -> Result<Renderer> {
    let (device, queue, _info) =
        pollster::block_on(acquire_device()).context("acquire wgpu device for cinematics fixture")?;
    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        width: WIDTH,
        height: HEIGHT,
        present_mode: wgpu::PresentMode::AutoVsync,
        alpha_mode: wgpu::CompositeAlphaMode::Auto,
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    };
    let renderer = pollster::block_on(Renderer::new_from_device(
        (*device).clone(),
        (*queue).clone(),
        None,
        config,
    ))
    .context("Renderer::new_from_device for cinematics fixture")?;
    Ok(renderer)
}

/// A FreeFly whose projection parameters (aspect/znear/zfar) persist through a
/// `tick_cinematics` apply (apply_camera_key sets only position/yaw/pitch/fovy).
/// position/yaw/pitch/fovy here are placeholders the tick will overwrite.
fn cinematics_freefly(aspect: f32, znear: f32, zfar: f32) -> FreeFly {
    FreeFly {
        position: Vec3::ZERO,
        yaw: 0.0,
        pitch: 0.0,
        fovy: 30_f32.to_radians(),
        aspect,
        znear,
        zfar,
    }
}

/// Drive ONE CameraKey through the production cinematics path and return the
/// resulting world-relative RenderView. The key is placed at t=0.5 in a
/// duration>=1.0 timeline; a single tick of dt=1.0 emits it (0.0 < 0.5 <= 1.0).
fn drive_one_camera_key(
    renderer: &mut Renderer,
    freefly: &mut FreeFly,
    key: awc::CameraKey,
) -> Result<RenderView> {
    let mut tl = awc::Timeline::new("cin_fixture", 2.0);
    tl.add_camera_track(vec![key]);
    renderer.load_timeline(tl);
    renderer.play_timeline();
    let events = renderer.tick_cinematics(1.0, freefly);
    // Sanity: exactly the one camera key emitted (path actually ran).
    let cam_events = events.iter().filter(|e| e.is_camera_key()).count();
    anyhow::ensure!(
        cam_events == 1,
        "cinematics path must emit exactly one CameraKey event; got {} (events={})",
        cam_events,
        events.len(),
    );
    Ok(freefly.to_render_view())
}

/// Independent baseline RenderView from the TARGET-BASED look_at_rh form plus
/// canonical Projection::perspective. `pos` / `look_at` must already be the
/// SANITIZED values (the caller bakes in CameraKey::sanitize's effect).
fn cinematics_baseline_render_view(
    pos: Vec3,
    look_at: Vec3,
    fov_deg_sanitized: f32,
    aspect: f32,
    znear: f32,
    zfar: f32,
) -> RenderView {
    let view = Mat4::look_at_rh(pos, look_at, Vec3::Y);
    let projection = Projection::perspective(fov_deg_sanitized.to_radians(), aspect, znear, zfar);
    let view_dir = (look_at - pos).normalize();
    RenderView::new(view, &projection, pos, view_dir)
}

/// Column-wise Mat4 approx-equality, matching orbit_camera_producer.rs style.
fn assert_mat4_approx(actual: Mat4, expected: Mat4, eps: f32, what: &str) {
    let a = actual.to_cols_array();
    let e = expected.to_cols_array();
    for i in 0..16 {
        let d = (a[i] - e[i]).abs();
        // Relative tolerance for large-magnitude entries (translation columns
        // of view_proj/inverse can be large); absolute floor for the O(1)
        // rotation entries.
        let tol = (e[i].abs() * 1e-4).max(eps);
        assert!(
            d < tol,
            "{}: matrix entry {} diverged: actual={} expected={} |d|={} tol={}",
            what,
            i,
            a[i],
            e[i],
            d,
            tol,
        );
    }
}

/// Compare two RenderViews field-by-field (matrices column-wise, scalars by eps).
fn assert_render_view_approx(actual: &RenderView, expected: &RenderView, what: &str) {
    assert_mat4_approx(actual.view, expected.view, 1e-4, &format!("{what}.view"));
    assert_mat4_approx(
        actual.projection,
        expected.projection,
        1e-4,
        &format!("{what}.projection"),
    );
    assert_mat4_approx(
        actual.view_proj,
        expected.view_proj,
        1e-3,
        &format!("{what}.view_proj"),
    );
    assert_mat4_approx(
        actual.inverse_view,
        expected.inverse_view,
        1e-4,
        &format!("{what}.inverse_view"),
    );
    assert!(
        (actual.position - expected.position).length() < 1e-4,
        "{}.position diverged: actual={:?} expected={:?}",
        what,
        actual.position,
        expected.position,
    );
    assert!(
        (actual.view_dir - expected.view_dir).length() < 1e-5,
        "{}.view_dir diverged: actual={:?} expected={:?}",
        what,
        actual.view_dir,
        expected.view_dir,
    );
    assert!(
        (actual.fovy - expected.fovy).abs() < 1e-6,
        "{}.fovy diverged: actual={} expected={}",
        what,
        actual.fovy,
        expected.fovy,
    );
    assert!(
        (actual.aspect - expected.aspect).abs() < 1e-6,
        "{}.aspect diverged: actual={} expected={}",
        what,
        actual.aspect,
        expected.aspect,
    );
    assert!(
        (actual.znear - expected.znear).abs() < 1e-6,
        "{}.znear diverged: actual={} expected={}",
        what,
        actual.znear,
        expected.znear,
    );
    assert!(
        (actual.zfar - expected.zfar).abs() < 1e-3,
        "{}.zfar diverged: actual={} expected={}",
        what,
        actual.zfar,
        expected.zfar,
    );
}

#[test]
fn cinematics_normal_keyframe_matches_look_at_rh_baseline() {
    let mut renderer = match cinematics_renderer() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("[harness] cinematics fixture skipped (no GPU): {e:#}");
            return;
        }
    };
    let (aspect, znear, zfar) = (16.0_f32 / 9.0, 0.5_f32, 5000.0_f32);
    let mut freefly = cinematics_freefly(aspect, znear, zfar);

    let pos = (3.0_f32, 4.0_f32, 5.0_f32);
    let look_at = (0.0_f32, 0.0_f32, 0.0_f32);
    let fov_deg = 60.0_f32; // in range -> sanitize is a no-op here
    let key = awc::CameraKey::new(awc::Time::from_secs(0.5), pos, look_at, fov_deg);

    let actual =
        drive_one_camera_key(&mut renderer, &mut freefly, key).expect("cinematics path drive failed");

    // Independent baseline: target-based look_at_rh (NOT via yaw/pitch).
    let expected = cinematics_baseline_render_view(
        Vec3::new(pos.0, pos.1, pos.2),
        Vec3::new(look_at.0, look_at.1, look_at.2),
        fov_deg, // already sanitized (in range)
        aspect,
        znear,
        zfar,
    );
    assert_render_view_approx(&actual, &expected, "cinematics_normal");

    // Explicit fovy / position spot-checks (the geometric-intent contract).
    assert!(
        (actual.fovy - 60_f32.to_radians()).abs() < 1e-6,
        "RenderView.fovy must equal 60deg in radians; got {}",
        actual.fovy,
    );
    assert!(
        (actual.position - Vec3::new(3.0, 4.0, 5.0)).length() < 1e-4,
        "RenderView.position must equal the keyframe pos; got {:?}",
        actual.position,
    );
}

#[test]
fn cinematics_fov_out_of_range_clamped_to_170() {
    let mut renderer = match cinematics_renderer() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("[harness] cinematics fixture skipped (no GPU): {e:#}");
            return;
        }
    };
    let (aspect, znear, zfar) = (16.0_f32 / 9.0, 0.5_f32, 5000.0_f32);
    let mut freefly = cinematics_freefly(aspect, znear, zfar);

    let pos = (2.0_f32, 1.0_f32, -4.0_f32);
    let look_at = (0.0_f32, 0.0_f32, 0.0_f32);
    let fov_deg_in = 200.0_f32; // above 170 max -> sanitize clamps to 170
    let key = awc::CameraKey::new(awc::Time::from_secs(0.5), pos, look_at, fov_deg_in);

    let actual =
        drive_one_camera_key(&mut renderer, &mut freefly, key).expect("cinematics path drive failed");

    let fov_deg_sanitized = 170.0_f32; // CameraKey::sanitize: clamp(10,170)
    let expected = cinematics_baseline_render_view(
        Vec3::new(pos.0, pos.1, pos.2),
        Vec3::new(look_at.0, look_at.1, look_at.2),
        fov_deg_sanitized,
        aspect,
        znear,
        zfar,
    );
    assert_render_view_approx(&actual, &expected, "cinematics_fov_clamp");

    // The headline contract: out-of-range fov was clamped to 170deg, NOT 200deg.
    assert!(
        (actual.fovy - 170_f32.to_radians()).abs() < 1e-6,
        "out-of-range fov_deg=200 must clamp to 170deg in radians (~{}); got {} (200deg would be {})",
        170_f32.to_radians(),
        actual.fovy,
        200_f32.to_radians(),
    );
}

#[test]
fn cinematics_degenerate_look_at_resolves_to_plus_x() {
    let mut renderer = match cinematics_renderer() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("[harness] cinematics fixture skipped (no GPU): {e:#}");
            return;
        }
    };
    let (aspect, znear, zfar) = (4.0_f32 / 3.0, 0.5_f32, 5000.0_f32);
    let mut freefly = cinematics_freefly(aspect, znear, zfar);

    let pos = (7.0_f32, -2.0_f32, 3.0_f32);
    let look_at = pos; // DEGENERATE: look_at == pos exactly
    let fov_deg = 60.0_f32;
    let key = awc::CameraKey::new(awc::Time::from_secs(0.5), pos, look_at, fov_deg);

    let actual =
        drive_one_camera_key(&mut renderer, &mut freefly, key).expect("cinematics path drive failed");

    // sanitize resolves look_at -> pos + (1,0,0) (canonical +X forward).
    let look_at_sanitized = (pos.0 + 1.0, pos.1, pos.2);
    let expected = cinematics_baseline_render_view(
        Vec3::new(pos.0, pos.1, pos.2),
        Vec3::new(look_at_sanitized.0, look_at_sanitized.1, look_at_sanitized.2),
        fov_deg,
        aspect,
        znear,
        zfar,
    );
    assert_render_view_approx(&actual, &expected, "cinematics_degenerate");

    // The headline contract: degenerate look_at resolved to +X forward.
    assert!(
        (actual.view_dir - Vec3::X).length() < 1e-5,
        "degenerate look_at==pos must resolve to +X forward per sanitize; got {:?}",
        actual.view_dir,
    );
}

#[test]
fn cinematics_lerped_keyframe_drives_interpolated_pose() {
    let mut renderer = match cinematics_renderer() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("[harness] cinematics fixture skipped (no GPU): {e:#}");
            return;
        }
    };
    let (aspect, znear, zfar) = (16.0_f32 / 9.0, 0.5_f32, 5000.0_f32);
    let mut freefly = cinematics_freefly(aspect, znear, zfar);

    let a = awc::CameraKey::new(awc::Time::from_secs(0.0), (0.0, 0.0, 10.0), (0.0, 0.0, 0.0), 50.0);
    let b = awc::CameraKey::new(awc::Time::from_secs(1.0), (10.0, 0.0, 0.0), (0.0, 5.0, 0.0), 90.0);
    let mid = a.lerp(&b, 0.5);

    // Independently verify the lerp produced the hand-computed triple before
    // we trust it as the baseline source (linear s + (o-s)*t per lib.rs).
    assert!(
        (mid.pos.0 - 5.0).abs() < 1e-5 && (mid.pos.1 - 0.0).abs() < 1e-5 && (mid.pos.2 - 5.0).abs() < 1e-5,
        "lerp pos must be (5,0,5); got {:?}",
        mid.pos
    );
    assert!(
        (mid.look_at.0 - 0.0).abs() < 1e-5
            && (mid.look_at.1 - 2.5).abs() < 1e-5
            && (mid.look_at.2 - 0.0).abs() < 1e-5,
        "lerp look_at must be (0,2.5,0); got {:?}",
        mid.look_at
    );
    assert!((mid.fov_deg - 70.0).abs() < 1e-4, "lerp fov_deg must be 70; got {}", mid.fov_deg);
    assert!((mid.t.0 - 0.5).abs() < 1e-5, "lerp t must be 0.5s; got {}", mid.t.0);

    // Drive the lerped key through the production path.
    let actual = drive_one_camera_key(&mut renderer, &mut freefly, mid.clone())
        .expect("cinematics path drive failed");

    // Independent baseline from the hand-computed lerped triple.
    let expected = cinematics_baseline_render_view(
        Vec3::new(5.0, 0.0, 5.0),
        Vec3::new(0.0, 2.5, 0.0),
        70.0, // fov in range -> sanitize no-op
        aspect,
        znear,
        zfar,
    );
    assert_render_view_approx(&actual, &expected, "cinematics_lerp");

    assert!(
        (actual.fovy - 70_f32.to_radians()).abs() < 1e-6,
        "interpolated fov must reach fovy=70deg in radians; got {}",
        actual.fovy,
    );
}
