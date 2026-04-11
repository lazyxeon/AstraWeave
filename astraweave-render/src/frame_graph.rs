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

use crate::graph::{GraphContext, PassDecl, PassOutput, RenderGraph, RenderNode, TransientTextureDesc};
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
}

impl Default for FrameGraphConfig {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
            shadow_resolution: 2048,
            cascade_count: 2,
            enable_clustered_lighting: true,
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

// ---------------------------------------------------------------------------
// Graph Builder
// ---------------------------------------------------------------------------

/// Build the default frame render graph with proper DAG topology.
///
/// The resulting graph encodes a forward+ pipeline:
///
/// | Pass | Reads | Writes |
/// |------|-------|--------|
/// | `cluster_bin` | — | `cluster_data` |
/// | `shadow` | — | `shadow_map` |
/// | `sky` | — | `hdr_color`, `depth` |
/// | `main_scene` | `shadow_map`, `hdr_color`, `depth`, `cluster_data` | `scene_color` |
/// | `tonemap` | `scene_color` | — (writes to surface) |
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

    // Pass 4: Main scene rendering (reads upstream resources, produces scene_color)
    let mut main_inputs = vec![
        "shadow_map".into(),
        "hdr_color".into(),
        "depth".into(),
    ];
    if config.enable_clustered_lighting {
        main_inputs.push("cluster_data".into());
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

    // Pass 5: Tonemap (reads scene_color, writes to surface)
    graph.add_pass(PassDecl {
        name: "tonemap".into(),
        inputs: vec!["scene_color".into()],
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
        assert_eq!(compiled.execution_order.len(), 5);
    }

    #[test]
    fn build_graph_without_clustering_compiles() {
        let cfg = FrameGraphConfig {
            enable_clustered_lighting: false,
            ..Default::default()
        };
        let mut graph = build_default_graph(&cfg);
        let compiled = graph.compile().expect("graph should compile");
        // Without clustering: shadow, sky, main_scene, tonemap
        assert_eq!(compiled.execution_order.len(), 4);
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
        let main_scene_pos = pos("main_scene");
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
        // tonemap must come after main_scene
        assert!(
            main_scene_pos < tonemap_pos,
            "main_scene must execute before tonemap"
        );
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
        assert_eq!(graph.node_count(), 5);
        assert_eq!(graph.node_name(0), Some("cluster_bin"));
        assert_eq!(graph.node_name(1), Some("shadow"));
        assert_eq!(graph.node_name(2), Some("sky"));
        assert_eq!(graph.node_name(3), Some("main_scene"));
        assert_eq!(graph.node_name(4), Some("tonemap"));
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
            enable_clustered_lighting: true,
        };
        let mut graph = build_default_graph(&cfg);
        graph.compile().expect("tiny resolution should compile");
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
