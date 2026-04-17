//! Phase 5.3 T2 — impostor atlas bake CLI.
//!
//! Bakes a texture atlas of pre-rendered billboard views for tree / foliage
//! meshes that the LOD3 sampling pipeline (see [`astraweave_render::impostor_lod3`])
//! consumes at draw time. This is the offline counterpart to the lazy-bake
//! path in [`astraweave_render::impostor_bake::load_or_bake_atlas`].
//!
//! # Usage
//!
//! ```text
//! cargo run --release -p astraweave-render --features impostor-bake-cli \
//!     --bin aw-impostor-bake -- \
//!     --input assets/meshes/oak.glb \
//!     --input assets/meshes/pine.glb \
//!     --output-atlas assets/impostors/foliage.png \
//!     --output-sidecar assets/impostors/foliage.toml \
//!     --atlas-width 1024 --atlas-height 512 --angle-count 8
//! ```
//!
//! Each `--input` file contributes one row to the atlas (all primitives in
//! the file are merged). Row `i` is the `i`-th input in command-line order.
//!
//! The CLI is deliberately narrow — it shells out to the same public API
//! (`ImpostorBaker`, `fit_ortho_camera`, `save_atlas_*`) that the runtime
//! lazy-bake path uses. Any mismatch between runtime-baked and CLI-baked
//! atlases indicates a bug in the shared code, not in the CLI.

use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use clap::Parser;

use astraweave_render::impostor_bake::{
    fit_ortho_camera, save_atlas_png, save_atlas_sidecar, upload_simplification_mesh, Aabb,
    ImpostorBaker, ImpostorBakerConfig,
};
use astraweave_render::lod_generator::SimplificationMesh;
use astraweave_render::mesh::{CpuImage, CpuMesh};
use astraweave_render::mesh_gltf::{load_gltf, GltfOptions};
use astraweave_render::vegetation_lod::ImpostorAtlasSpec;

// ────────────────────────────────────────────────────────────────────────────
// CLI definition
// ────────────────────────────────────────────────────────────────────────────

/// Bake an impostor atlas from one or more glTF/GLB meshes.
#[derive(Parser, Debug)]
#[command(name = "aw-impostor-bake", version, about, long_about = None)]
struct Cli {
    /// Path to an input glTF / GLB mesh. May be repeated — one per species.
    /// Species are assigned names from the file stem unless overridden with
    /// repeated `--species-name` flags (same count and order as `--input`).
    #[arg(long = "input", required = true)]
    input: Vec<PathBuf>,

    /// Optional species name override (one per `--input`, same order).
    #[arg(long = "species-name")]
    species_name: Vec<String>,

    /// Output PNG path for the baked atlas.
    #[arg(long = "output-atlas")]
    output_atlas: PathBuf,

    /// Output TOML sidecar path. Stores per-cell UV regions + species order so
    /// the runtime can load the atlas without re-running bake logic.
    #[arg(long = "output-sidecar")]
    output_sidecar: PathBuf,

    /// Atlas width in pixels.
    #[arg(long, default_value_t = 1024)]
    atlas_width: u32,

    /// Atlas height in pixels.
    #[arg(long, default_value_t = 512)]
    atlas_height: u32,

    /// Number of pre-baked angles per species (atlas columns).
    #[arg(long, default_value_t = 8)]
    angle_count: u32,
}

// ────────────────────────────────────────────────────────────────────────────
// Main
// ────────────────────────────────────────────────────────────────────────────

fn main() -> Result<()> {
    let cli = Cli::parse();
    validate_cli(&cli)?;

    let species_names: Vec<String> = cli
        .input
        .iter()
        .enumerate()
        .map(|(i, path)| derive_species_name(path, &cli.species_name, i))
        .collect();

    eprintln!(
        "aw-impostor-bake: {} species × {} angles @ {}×{}",
        species_names.len(),
        cli.angle_count,
        cli.atlas_width,
        cli.atlas_height,
    );

    // 1. Load every input mesh (CPU-side).
    let loaded: Vec<LoadedSpecies> = cli
        .input
        .iter()
        .zip(species_names.iter())
        .map(|(path, name)| load_species(path, name))
        .collect::<Result<Vec<_>>>()?;

    // 2. Build the atlas specification (uniform cell grid).
    let species_refs: Vec<&str> = species_names.iter().map(String::as_str).collect();
    let spec = ImpostorAtlasSpec::uniform(
        cli.atlas_width,
        cli.atlas_height,
        cli.angle_count,
        &species_refs,
    );

    // 3. Spin up a headless wgpu device and create the baker.
    let (device, queue) = pollster::block_on(create_headless_device())?;
    let baker = ImpostorBaker::new(
        &device,
        ImpostorBakerConfig {
            atlas_width: cli.atlas_width,
            atlas_height: cli.atlas_height,
            sample_count: 1,
        },
    )
    .context("failed to create ImpostorBaker")?;
    baker.clear(&device, &queue);

    // 4. For every species × angle, render one atlas cell.
    for (species_idx, species) in loaded.iter().enumerate() {
        eprintln!(
            "  baking species {}/{}: {} ({} verts, {} tris)",
            species_idx + 1,
            loaded.len(),
            species.name,
            species.mesh.positions.len(),
            species.mesh.indices.len() / 3,
        );
        let (vbuf, ibuf, icount) = upload_simplification_mesh(&device, &species.mesh);
        let diffuse_view = upload_diffuse(&device, &queue, species.albedo.as_ref(), &species.name);
        let diffuse_bg = baker.make_diffuse_bind_group(&device, &diffuse_view);
        let aabb = Aabb::from_points(&species.mesh.positions)
            .ok_or_else(|| anyhow!("species '{}' has no vertex positions", species.name))?;

        for angle_idx in 0..cli.angle_count {
            let angle = angle_idx as f32 * std::f32::consts::TAU / cli.angle_count as f32;
            let (proj, view) = fit_ortho_camera(aabb, angle);
            let region = spec
                .lookup(species_idx, angle)
                .copied()
                .ok_or_else(|| {
                    anyhow!(
                        "atlas spec missing cell for species {} angle {}",
                        species_idx,
                        angle_idx
                    )
                })?;
            baker.draw_into_region(
                &device,
                &queue,
                proj * view,
                region,
                &vbuf,
                &ibuf,
                icount,
                &diffuse_bg,
            );
        }
    }

    // 5. Read back the atlas and persist PNG + sidecar.
    let pixels = baker
        .readback_atlas(&device, &queue)
        .context("readback_atlas failed")?;
    assert_eq!(
        pixels.len(),
        (cli.atlas_width * cli.atlas_height * 4) as usize,
        "readback returned {} bytes, expected {}",
        pixels.len(),
        cli.atlas_width * cli.atlas_height * 4
    );

    if let Some(parent) = cli.output_atlas.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create parent dir {}", parent.display()))?;
        }
    }
    if let Some(parent) = cli.output_sidecar.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create parent dir {}", parent.display()))?;
        }
    }

    save_atlas_png(&cli.output_atlas, &pixels, cli.atlas_width, cli.atlas_height)
        .with_context(|| format!("writing atlas PNG to {}", cli.output_atlas.display()))?;
    save_atlas_sidecar(&cli.output_sidecar, &spec)
        .with_context(|| format!("writing sidecar TOML to {}", cli.output_sidecar.display()))?;

    eprintln!(
        "aw-impostor-bake: wrote {} ({} bytes) + {}",
        cli.output_atlas.display(),
        pixels.len(),
        cli.output_sidecar.display(),
    );
    Ok(())
}

// ────────────────────────────────────────────────────────────────────────────
// Helpers
// ────────────────────────────────────────────────────────────────────────────

fn validate_cli(cli: &Cli) -> Result<()> {
    if !cli.species_name.is_empty() && cli.species_name.len() != cli.input.len() {
        anyhow::bail!(
            "--species-name count ({}) must match --input count ({})",
            cli.species_name.len(),
            cli.input.len()
        );
    }
    if cli.atlas_width == 0 || cli.atlas_height == 0 {
        anyhow::bail!("atlas dimensions must be non-zero");
    }
    if cli.angle_count == 0 {
        anyhow::bail!("--angle-count must be >= 1");
    }
    Ok(())
}

fn derive_species_name(path: &std::path::Path, overrides: &[String], idx: usize) -> String {
    if let Some(name) = overrides.get(idx) {
        return name.clone();
    }
    path.file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("species_{idx}"))
}

struct LoadedSpecies {
    name: String,
    mesh: SimplificationMesh,
    albedo: Option<CpuImage>,
}

/// Load a glTF/GLB file and merge all of its primitives into a single
/// [`SimplificationMesh`]. The first non-empty `albedo_image` on any primitive
/// becomes the species' diffuse texture.
fn load_species(path: &std::path::Path, name: &str) -> Result<LoadedSpecies> {
    let primitives = load_gltf(path, &GltfOptions::default())
        .with_context(|| format!("loading glTF {}", path.display()))?;
    if primitives.is_empty() {
        anyhow::bail!("glTF '{}' contained no primitives", path.display());
    }

    let mut merged = CpuMesh {
        vertices: Vec::new(),
        indices: Vec::new(),
        albedo_image: None,
        texture_source_hint: None,
    };
    for prim in primitives {
        let offset = merged.vertices.len() as u32;
        merged.vertices.extend(prim.vertices);
        merged.indices.extend(prim.indices.iter().map(|i| i + offset));
        if merged.albedo_image.is_none() {
            merged.albedo_image = prim.albedo_image;
        }
    }

    let mesh = SimplificationMesh::from_cpu_mesh(&merged);
    Ok(LoadedSpecies {
        name: name.to_string(),
        mesh,
        albedo: merged.albedo_image,
    })
}

/// Upload the species' albedo to a GPU texture. Falls back to a 1×1 neutral
/// grey placeholder when the mesh has no albedo — keeps bake deterministic
/// instead of silently producing a black atlas cell.
fn upload_diffuse(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    albedo: Option<&CpuImage>,
    species_name: &str,
) -> wgpu::TextureView {
    use wgpu::util::DeviceExt;
    let (w, h, pixels): (u32, u32, std::borrow::Cow<[u8]>) = match albedo {
        Some(img) if !img.pixels.is_empty() => {
            (img.width, img.height, std::borrow::Cow::Borrowed(&img.pixels))
        }
        _ => {
            eprintln!(
                "  [warn] species '{}' has no albedo image; using 1×1 grey placeholder",
                species_name
            );
            (1, 1, std::borrow::Cow::Owned(vec![200u8, 200, 200, 255]))
        }
    };
    let tex = device.create_texture_with_data(
        queue,
        &wgpu::TextureDescriptor {
            label: Some("aw-impostor-bake-diffuse"),
            size: wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        },
        wgpu::util::TextureDataOrder::default(),
        &pixels,
    );
    tex.create_view(&wgpu::TextureViewDescriptor::default())
}

async fn create_headless_device() -> Result<(wgpu::Device, wgpu::Queue)> {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: None,
        })
        .await
        .context("no suitable wgpu adapter found")?;
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("aw-impostor-bake-device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::downlevel_defaults(),
            memory_hints: wgpu::MemoryHints::default(),
            trace: Default::default(),
        })
        .await
        .context("request_device failed")?;
    Ok((device, queue))
}
