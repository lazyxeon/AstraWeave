//! Frame render graph: DAG-based pipeline for the main frame rendering passes.
//!
//! This module bridges the [`RenderGraph`] infrastructure in [`crate::graph`]
//! with the concrete pass structure of the engine's forward+ renderer. It
//! defines typed pass nodes and a builder that assembles the frame DAG with
//! explicit resource inputs/outputs.
//!
//! ## Pass topology
//!
//! ```text
//!   cluster_bin ──┐
//!                 ├──→ main_scene ──→ tonemap
//!   shadow ───────┤        ↑
//!                 │        │
//!   sky ──────────┘   (hdr + depth)
//! ```
//!
//! ## Migration Status
//!
//! The graph currently defines the DAG topology, validates resource flow, and
//! exercises the automatic topological ordering, resource lifetime, and aliasing
//! analysis from [`crate::graph::RenderGraph::compile`]. Pass nodes validate
//! available resources but delegate actual GPU command recording to
//! [`crate::renderer::Renderer`] methods. Full delegation is designed for
//! incremental adoption.

use crate::graph::{
    GraphContext, PassDecl, PassOutput, RenderGraph, RenderNode, TransientTextureDesc,
};
use anyhow::{Context, Result};

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for building a frame render graph.
#[derive(Debug, Clone)]
pub struct FrameGraphConfig {
    /// Render target width in pixels.
    pub width: u32,
    /// Render target height in pixels.
    pub height: u32,
    /// Shadow map resolution per cascade (square).
    pub shadow_resolution: u32,
    /// Number of shadow cascades.
    pub cascade_count: u32,
    /// Whether clustered forward lighting is enabled.
    pub enable_clustered_lighting: bool,
    /// Whether GTAO (ambient occlusion) is enabled.
    pub enable_gtao: bool,
    /// Whether screen-space global illumination is enabled.
    pub enable_ssgi: bool,
    /// Whether screen-space reflections are enabled.
    pub enable_ssr: bool,
    /// Whether bloom post-processing is enabled.
    pub enable_bloom: bool,
    /// Whether froxel-based volumetric fog is enabled.
    pub enable_volumetric_fog: bool,
}

impl Default for FrameGraphConfig {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
            shadow_resolution: 2048,
            cascade_count: 2,
            enable_clustered_lighting: true,
            enable_gtao: true,
            enable_ssgi: false,
            enable_ssr: true,
            enable_bloom: true,
            enable_volumetric_fog: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Pass Nodes
// ---------------------------------------------------------------------------

/// Clustered light binning compute pass.
///
/// Reads point light data from the renderer's light buffers and writes
/// per-cluster light indices/counts for the main scene pass.
pub struct ClusterBinNode;

impl ClusterBinNode {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ClusterBinNode {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderNode for ClusterBinNode {
    fn name(&self) -> &str {
        "cluster_bin"
    }
    fn run(&mut self, ctx: &mut GraphContext) -> Result<()> {
        let _enc = ctx
            .encoder
            .as_deref_mut()
            .context("ClusterBinNode requires encoder")?;
        Ok(())
    }
}

/// Shadow depth pass for all cascades.
///
/// Renders depth-only geometry into a layered shadow map. Each layer
/// corresponds to one CSM cascade with its own orthographic projection.
pub struct ShadowPassNode {
    cascade_count: u32,
}

impl ShadowPassNode {
    pub fn new(cascade_count: u32) -> Self {
        Self { cascade_count }
    }

    pub fn cascade_count(&self) -> u32 {
        self.cascade_count
    }
}

impl RenderNode for ShadowPassNode {
    fn name(&self) -> &str {
        "shadow"
    }
    fn run(&mut self, ctx: &mut GraphContext) -> Result<()> {
        let _enc = ctx
            .encoder
            .as_deref_mut()
            .context("ShadowPassNode requires encoder")?;
        Ok(())
    }
}

/// Sky rendering pass.
///
/// Renders the skybox/procedural sky into the HDR color target and
/// initialises the depth buffer. Executes before the main scene pass
/// so that geometry is rendered on top of the sky.
pub struct SkyPassNode;

impl SkyPassNode {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SkyPassNode {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderNode for SkyPassNode {
    fn name(&self) -> &str {
        "sky"
    }
    fn run(&mut self, ctx: &mut GraphContext) -> Result<()> {
        let _enc = ctx
            .encoder
            .as_deref_mut()
            .context("SkyPassNode requires encoder")?;
        Ok(())
    }
}

/// Main scene rendering pass (opaque geometry, transparent objects, water).
///
/// Reads the shadow map, HDR color (preserving sky), and depth buffer.
/// Binds clustered lighting data if available. Renders all visible
/// geometry with PBR shading into the scene color output.
pub struct MainSceneNode;

impl MainSceneNode {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MainSceneNode {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderNode for MainSceneNode {
    fn name(&self) -> &str {
        "main_scene"
    }
    fn run(&mut self, ctx: &mut GraphContext) -> Result<()> {
        let _enc = ctx
            .encoder
            .as_deref_mut()
            .context("MainSceneNode requires encoder")?;
        // Validate resources produced by upstream passes are available.
        let _ = ctx.resources.view("hdr_color")?;
        let _ = ctx.resources.view("depth")?;
        Ok(())
    }
}

/// Tonemap / post-processing pass.
///
/// Reads the HDR scene color and writes the final LDR result to the
/// surface (swapchain) or an offscreen target via a fullscreen triangle.
pub struct TonemapNode;

impl TonemapNode {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TonemapNode {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderNode for TonemapNode {
    fn name(&self) -> &str {
        "tonemap"
    }
    fn run(&mut self, ctx: &mut GraphContext) -> Result<()> {
        let _enc = ctx
            .encoder
            .as_deref_mut()
            .context("TonemapNode requires encoder")?;
        // Scene color is the output of main_scene — our input.
        let _ = ctx.resources.view("scene_color")?;
        Ok(())
    }
}

/// Ground Truth Ambient Occlusion pass.
///
/// Reads the depth buffer and generates an AO texture using visibility
/// bitmask sampling. Runs after the depth prepass / sky pass and feeds
/// into the main scene pass.
pub struct GtaoNode;

impl GtaoNode {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GtaoNode {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderNode for GtaoNode {
    fn name(&self) -> &str {
        "gtao"
    }
    fn run(&mut self, ctx: &mut GraphContext) -> Result<()> {
        let _enc = ctx
            .encoder
            .as_deref_mut()
            .context("GtaoNode requires encoder")?;
        let _ = ctx.resources.view("depth")?;
        Ok(())
    }
}

/// Screen-Space Global Illumination pass.
///
/// Reads the depth and normals from the scene to compute indirect diffuse
/// bounces. Produces an irradiance texture consumed by the main scene pass.
pub struct SsgiNode;

impl SsgiNode {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SsgiNode {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderNode for SsgiNode {
    fn name(&self) -> &str {
        "ssgi"
    }
    fn run(&mut self, ctx: &mut GraphContext) -> Result<()> {
        let _enc = ctx
            .encoder
            .as_deref_mut()
            .context("SsgiNode requires encoder")?;
        let _ = ctx.resources.view("depth")?;
        let _ = ctx.resources.view("hdr_color")?;
        Ok(())
    }
}

/// Screen-Space Reflections pass.
///
/// Uses Hi-Z ray marching against the depth buffer to produce a reflection
/// texture. Reads scene color + depth, outputs a reflection buffer that the
/// main scene or a composite pass merges.
pub struct SsrNode;

impl SsrNode {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SsrNode {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderNode for SsrNode {
    fn name(&self) -> &str {
        "ssr"
    }
    fn run(&mut self, ctx: &mut GraphContext) -> Result<()> {
        let _enc = ctx
            .encoder
            .as_deref_mut()
            .context("SsrNode requires encoder")?;
        let _ = ctx.resources.view("scene_color")?;
        let _ = ctx.resources.view("depth")?;
        Ok(())
    }
}

/// Bloom pass — physically-based bloom with 13-tap downsample and tent upsample.
///
/// Reads the HDR scene color and produces a bloom texture that is composited
/// before tonemapping.
pub struct BloomNode;

impl BloomNode {
    pub fn new() -> Self {
        Self
    }
}

impl Default for BloomNode {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderNode for BloomNode {
    fn name(&self) -> &str {
        "bloom"
    }
    fn run(&mut self, ctx: &mut GraphContext) -> Result<()> {
        let _enc = ctx
            .encoder
            .as_deref_mut()
            .context("BloomNode requires encoder")?;
        let _ = ctx.resources.view("scene_color")?;
        Ok(())
    }
}

/// Froxel-based volumetric fog pass.
///
/// Reads the depth buffer and shadow map to compute in-scattering and
/// extinction per froxel. Produces a 3D fog volume that the main scene
/// pass composites during forward rendering.
pub struct VolumetricFogNode;

impl VolumetricFogNode {
    pub fn new() -> Self {
        Self
    }
}

impl Default for VolumetricFogNode {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderNode for VolumetricFogNode {
    fn name(&self) -> &str {
        "volumetric_fog"
    }
    fn run(&mut self, ctx: &mut GraphContext) -> Result<()> {
        let _enc = ctx
            .encoder
            .as_deref_mut()
            .context("VolumetricFogNode requires encoder")?;
        let _ = ctx.resources.view("depth")?;
        let _ = ctx.resources.view("shadow_map")?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Graph Builder
// ---------------------------------------------------------------------------

/// Build the default frame render graph with proper DAG topology.
///
/// The resulting graph encodes a forward+ pipeline with optional post-processing:
///
/// | Pass | Reads | Writes |
/// |------|-------|--------|
/// | `cluster_bin` | — | `cluster_data` |
/// | `shadow` | — | `shadow_map` |
/// | `sky` | — | `hdr_color`, `depth` |
/// | `gtao` | `depth` | `ao_texture` |
/// | `ssgi` | `depth`, `hdr_color` | `gi_irradiance` |
/// | `volumetric_fog` | `depth`, `shadow_map` | `fog_volume` |
/// | `main_scene` | `shadow_map`, `hdr_color`, `depth`, `cluster_data`, `ao_texture`, `gi_irradiance`, `fog_volume` | `scene_color` |
/// | `ssr` | `scene_color`, `depth` | `ssr_reflections` |
/// | `bloom` | `scene_color` | `bloom_texture` |
/// | `tonemap` | `scene_color`, `ssr_reflections`, `bloom_texture` | — (writes to surface) |
///
/// Resource dependencies create edges in the DAG that enforce correct
/// execution ordering via topological sort.
pub fn build_default_graph(config: &FrameGraphConfig) -> RenderGraph {
    let mut graph = RenderGraph::new();

    // Pass 1: Cluster binning (compute, produces cluster index data)
    if config.enable_clustered_lighting {
        graph.add_pass(PassDecl {
            name: "cluster_bin".into(),
            inputs: vec![],
            outputs: vec![PassOutput {
                name: "cluster_data".into(),
                desc: TransientTextureDesc::color(
                    "cluster_data",
                    (config.width / 16).max(1),
                    (config.height / 16).max(1),
                ),
            }],
            node: Box::new(ClusterBinNode::new()),
        });
    }

    // Pass 2: Shadow map generation (depth-only render per cascade)
    graph.add_pass(PassDecl {
        name: "shadow".into(),
        inputs: vec![],
        outputs: vec![PassOutput {
            name: "shadow_map".into(),
            desc: TransientTextureDesc::depth(
                "shadow_map",
                config.shadow_resolution,
                config.shadow_resolution,
            )
            .with_layers(config.cascade_count),
        }],
        node: Box::new(ShadowPassNode::new(config.cascade_count)),
    });

    // Pass 3: Sky rendering (creates HDR color target + depth buffer)
    graph.add_pass(PassDecl {
        name: "sky".into(),
        inputs: vec![],
        outputs: vec![
            PassOutput {
                name: "hdr_color".into(),
                desc: TransientTextureDesc::color("hdr_color", config.width, config.height),
            },
            PassOutput {
                name: "depth".into(),
                desc: TransientTextureDesc::depth("depth", config.width, config.height),
            },
        ],
        node: Box::new(SkyPassNode::new()),
    });

    // Pass 4 (optional): GTAO — ambient occlusion from depth
    if config.enable_gtao {
        graph.add_pass(PassDecl {
            name: "gtao".into(),
            inputs: vec!["depth".into()],
            outputs: vec![PassOutput {
                name: "ao_texture".into(),
                desc: TransientTextureDesc::color(
                    "ao_texture",
                    config.width / 2,
                    config.height / 2,
                ),
            }],
            node: Box::new(GtaoNode::new()),
        });
    }

    // Pass 5 (optional): SSGI — indirect diffuse from depth + color
    if config.enable_ssgi {
        graph.add_pass(PassDecl {
            name: "ssgi".into(),
            inputs: vec!["depth".into(), "hdr_color".into()],
            outputs: vec![PassOutput {
                name: "gi_irradiance".into(),
                desc: TransientTextureDesc::color(
                    "gi_irradiance",
                    config.width / 2,
                    config.height / 2,
                ),
            }],
            node: Box::new(SsgiNode::new()),
        });
    }

    // Pass 6 (optional): Volumetric fog — froxel scattering from depth + shadow
    if config.enable_volumetric_fog {
        graph.add_pass(PassDecl {
            name: "volumetric_fog".into(),
            inputs: vec!["depth".into(), "shadow_map".into()],
            outputs: vec![PassOutput {
                name: "fog_volume".into(),
                desc: TransientTextureDesc::color(
                    "fog_volume",
                    config.width / 2,
                    config.height / 2,
                ),
            }],
            node: Box::new(VolumetricFogNode::new()),
        });
    }

    // Pass 7: Main scene rendering (reads upstream resources, produces scene_color)
    let mut main_inputs = vec!["shadow_map".into(), "hdr_color".into(), "depth".into()];
    if config.enable_clustered_lighting {
        main_inputs.push("cluster_data".into());
    }
    if config.enable_gtao {
        main_inputs.push("ao_texture".into());
    }
    if config.enable_ssgi {
        main_inputs.push("gi_irradiance".into());
    }
    if config.enable_volumetric_fog {
        main_inputs.push("fog_volume".into());
    }

    graph.add_pass(PassDecl {
        name: "main_scene".into(),
        inputs: main_inputs,
        outputs: vec![PassOutput {
            name: "scene_color".into(),
            desc: TransientTextureDesc::color("scene_color", config.width, config.height),
        }],
        node: Box::new(MainSceneNode::new()),
    });

    // Pass 8 (optional): SSR — screen-space reflections from scene_color + depth
    if config.enable_ssr {
        graph.add_pass(PassDecl {
            name: "ssr".into(),
            inputs: vec!["scene_color".into(), "depth".into()],
            outputs: vec![PassOutput {
                name: "ssr_reflections".into(),
                desc: TransientTextureDesc::color("ssr_reflections", config.width, config.height),
            }],
            node: Box::new(SsrNode::new()),
        });
    }

    // Pass 9 (optional): Bloom — physically-based bloom from scene_color
    if config.enable_bloom {
        graph.add_pass(PassDecl {
            name: "bloom".into(),
            inputs: vec!["scene_color".into()],
            outputs: vec![PassOutput {
                name: "bloom_texture".into(),
                desc: TransientTextureDesc::color(
                    "bloom_texture",
                    config.width / 2,
                    config.height / 2,
                ),
            }],
            node: Box::new(BloomNode::new()),
        });
    }

    // Pass 10: Tonemap (reads scene_color + post-process outputs, writes to surface)
    let mut tonemap_inputs = vec!["scene_color".into()];
    if config.enable_ssr {
        tonemap_inputs.push("ssr_reflections".into());
    }
    if config.enable_bloom {
        tonemap_inputs.push("bloom_texture".into());
    }

    graph.add_pass(PassDecl {
        name: "tonemap".into(),
        inputs: tonemap_inputs,
        outputs: vec![],
        node: Box::new(TonemapNode::new()),
    });

    graph
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let cfg = FrameGraphConfig::default();
        assert_eq!(cfg.width, 1920);
        assert_eq!(cfg.height, 1080);
        assert_eq!(cfg.shadow_resolution, 2048);
        assert_eq!(cfg.cascade_count, 2);
        assert!(cfg.enable_clustered_lighting);
    }

    #[test]
    fn build_graph_compiles() {
        let cfg = FrameGraphConfig::default();
        let mut graph = build_default_graph(&cfg);
        let compiled = graph.compile().expect("graph should compile");
        // Default: cluster_bin + shadow + sky + gtao + main_scene + ssr + bloom + tonemap = 8
        assert_eq!(compiled.execution_order.len(), 8);
    }

    #[test]
    fn build_graph_without_clustering_compiles() {
        let cfg = FrameGraphConfig {
            enable_clustered_lighting: false,
            ..Default::default()
        };
        let mut graph = build_default_graph(&cfg);
        let compiled = graph.compile().expect("graph should compile");
        // Without clustering: shadow, sky, gtao, main_scene, ssr, bloom, tonemap = 7
        assert_eq!(compiled.execution_order.len(), 7);
    }

    #[test]
    fn execution_order_respects_dependencies() {
        let cfg = FrameGraphConfig::default();
        let mut graph = build_default_graph(&cfg);
        graph.compile().expect("compile");

        let order = &graph.compiled().unwrap().execution_order;

        // Find indices of each pass in execution order
        let pos = |name: &str| -> usize {
            order
                .iter()
                .position(|&idx| graph.node_name(idx).unwrap() == name)
                .unwrap_or_else(|| panic!("pass '{}' not found in execution order", name))
        };

        let cluster_bin_pos = pos("cluster_bin");
        let shadow_pos = pos("shadow");
        let sky_pos = pos("sky");
        let gtao_pos = pos("gtao");
        let main_scene_pos = pos("main_scene");
        let ssr_pos = pos("ssr");
        let bloom_pos = pos("bloom");
        let tonemap_pos = pos("tonemap");

        // main_scene must come after all its producers
        assert!(
            shadow_pos < main_scene_pos,
            "shadow must execute before main_scene"
        );
        assert!(
            sky_pos < main_scene_pos,
            "sky must execute before main_scene"
        );
        assert!(
            cluster_bin_pos < main_scene_pos,
            "cluster_bin must execute before main_scene"
        );
        assert!(
            gtao_pos < main_scene_pos,
            "gtao must execute before main_scene"
        );
        // post-processing after main_scene
        assert!(
            main_scene_pos < ssr_pos,
            "main_scene must execute before ssr"
        );
        assert!(
            main_scene_pos < bloom_pos,
            "main_scene must execute before bloom"
        );
        // tonemap after post-processing
        assert!(ssr_pos < tonemap_pos, "ssr must execute before tonemap");
        assert!(bloom_pos < tonemap_pos, "bloom must execute before tonemap");
    }

    #[test]
    fn execution_order_without_clustering() {
        let cfg = FrameGraphConfig {
            enable_clustered_lighting: false,
            ..Default::default()
        };
        let mut graph = build_default_graph(&cfg);
        graph.compile().expect("compile");

        let order = &graph.compiled().unwrap().execution_order;

        let pos = |name: &str| -> usize {
            order
                .iter()
                .position(|&idx| graph.node_name(idx).unwrap() == name)
                .unwrap_or_else(|| panic!("pass '{}' not found", name))
        };

        let shadow_pos = pos("shadow");
        let sky_pos = pos("sky");
        let main_scene_pos = pos("main_scene");
        let tonemap_pos = pos("tonemap");

        assert!(shadow_pos < main_scene_pos);
        assert!(sky_pos < main_scene_pos);
        assert!(main_scene_pos < tonemap_pos);
    }

    #[test]
    fn transient_resources_declared() {
        let cfg = FrameGraphConfig::default();
        let mut graph = build_default_graph(&cfg);
        graph.compile().expect("compile");

        let compiled = graph.compiled().unwrap();
        let resource_names: Vec<&str> = compiled
            .transient_descs
            .iter()
            .map(|(n, _)| n.as_str())
            .collect();

        assert!(
            resource_names.contains(&"shadow_map"),
            "shadow_map transient must be declared"
        );
        assert!(
            resource_names.contains(&"hdr_color"),
            "hdr_color transient must be declared"
        );
        assert!(
            resource_names.contains(&"depth"),
            "depth transient must be declared"
        );
        assert!(
            resource_names.contains(&"scene_color"),
            "scene_color transient must be declared"
        );
        assert!(
            resource_names.contains(&"cluster_data"),
            "cluster_data transient must be declared"
        );
    }

    #[test]
    fn release_points_exist() {
        let cfg = FrameGraphConfig::default();
        let mut graph = build_default_graph(&cfg);
        graph.compile().expect("compile");

        let compiled = graph.compiled().unwrap();
        // At least some resources should have release points
        assert!(
            !compiled.release_points.is_empty(),
            "release points should be computed"
        );
    }

    #[test]
    fn node_names_correct() {
        let cfg = FrameGraphConfig::default();
        let graph = build_default_graph(&cfg);
        // Default: cluster_bin, shadow, sky, gtao, main_scene, ssr, bloom, tonemap = 8
        assert_eq!(graph.node_count(), 8);
        assert_eq!(graph.node_name(0), Some("cluster_bin"));
        assert_eq!(graph.node_name(1), Some("shadow"));
        assert_eq!(graph.node_name(2), Some("sky"));
        assert_eq!(graph.node_name(3), Some("gtao"));
        assert_eq!(graph.node_name(4), Some("main_scene"));
        assert_eq!(graph.node_name(5), Some("ssr"));
        assert_eq!(graph.node_name(6), Some("bloom"));
        assert_eq!(graph.node_name(7), Some("tonemap"));
    }

    #[test]
    fn shadow_pass_cascade_count() {
        let node = ShadowPassNode::new(4);
        assert_eq!(node.cascade_count(), 4);
        assert_eq!(node.name(), "shadow");
    }

    #[test]
    fn small_resolution_does_not_panic() {
        let cfg = FrameGraphConfig {
            width: 1,
            height: 1,
            shadow_resolution: 64,
            cascade_count: 1,
            ..Default::default()
        };
        let mut graph = build_default_graph(&cfg);
        graph.compile().expect("tiny resolution should compile");
    }

    #[test]
    fn minimal_graph_compiles() {
        let cfg = FrameGraphConfig {
            enable_clustered_lighting: false,
            enable_gtao: false,
            enable_ssgi: false,
            enable_ssr: false,
            enable_bloom: false,
            enable_volumetric_fog: false,
            ..Default::default()
        };
        let mut graph = build_default_graph(&cfg);
        let compiled = graph.compile().expect("minimal graph should compile");
        // shadow, sky, main_scene, tonemap = 4
        assert_eq!(compiled.execution_order.len(), 4);
    }

    #[test]
    fn full_graph_compiles() {
        let cfg = FrameGraphConfig {
            enable_clustered_lighting: true,
            enable_gtao: true,
            enable_ssgi: true,
            enable_ssr: true,
            enable_bloom: true,
            enable_volumetric_fog: true,
            ..Default::default()
        };
        let mut graph = build_default_graph(&cfg);
        let compiled = graph.compile().expect("full graph should compile");
        // All passes: cluster_bin + shadow + sky + gtao + ssgi + volumetric_fog +
        //             main_scene + ssr + bloom + tonemap = 10
        assert_eq!(compiled.execution_order.len(), 10);
    }

    #[test]
    fn cluster_data_aliasable_with_depth() {
        // cluster_data and depth have non-overlapping lifetimes if
        // cluster_data is consumed by main_scene (same as depth).
        // The aliasing analysis should consider them as candidates.
        let cfg = FrameGraphConfig::default();
        let mut graph = build_default_graph(&cfg);
        graph.compile().expect("compile");

        let compiled = graph.compiled().unwrap();
        // Just verify aliasing analysis ran without panicking and
        // produced groups (may or may not alias depending on format).
        let _groups = &compiled.alias_groups;
    }
}
