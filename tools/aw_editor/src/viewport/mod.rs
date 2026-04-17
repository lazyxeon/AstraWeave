//! 3D Viewport Module
//!
//! Professional 3D scene viewport for the AstraWeave editor. Provides:
//! - wgpu rendering integration with egui
//! - Orbit camera controls (Unity/Blender-style)
//! - Entity rendering with selection
//! - Visual gizmo manipulation
//! - Grid overlay and debug visualization
//! - Physics debug rendering (collider wireframes)
//!
//! # Architecture
//!
//! The viewport uses a **unified rendering architecture** where the engine
//! renderer (`astraweave-render`) is the default path for terrain, sky, water,
//! weather, scatter, and post-processing. Editor-specific overlays (grid,
//! entity cubes, gizmos, physics debug) are handled by lightweight local
//! renderers.
//!
//! ```text
//! ViewportWidget (egui integration)
//!     ↓
//! ViewportRenderer (rendering coordinator)
//!     ├─ EngineRenderAdapter (terrain, sky, water, weather, scatter, entities, post-FX)
//!     ├─ GridRenderer (floor grid + axes)
//!     ├─ GizmoRenderer (transform handles)
//!     └─ PhysicsDebugRenderer (collider wireframes)
//! ```
//!
//! # Usage
//!
//! ```rust,ignore
//! use aw_editor_lib::viewport::ViewportWidget;
//!
//! // Construct the widget from your eframe creation context.
//! // (The real `cc` comes from eframe; omitted here.)
//! // let viewport = ViewportWidget::new(cc)?;
//!
//! // Then call `viewport.ui(...)` from your App's update loop.
//! ```

pub mod blueprint_overlay;
pub mod camera;
mod engine_adapter;
mod gizmo_renderer;
mod grid_renderer;
/// Phase 5.3 T7 stage 3a: per-scatter-mesh impostor atlas registry
/// (content-hashed lazy bake + on-disk cache). Only compiled when the
/// upstream bake pipeline is enabled via the `impostor-bake` feature.
#[cfg(feature = "impostor-bake")]
pub mod impostor_registry;
/// Phase 5.3 T7 stage 3c.1: editor wiring helpers that bridge scatter LOD3
/// primitives to the shared impostor bake + pass infrastructure. Gated
/// alongside [`impostor_registry`] on the `impostor-bake` feature.
#[cfg(feature = "impostor-bake")]
pub mod impostor_wiring;
mod physics_renderer;
mod renderer;
pub mod toolbar;
/// Shared viewport types (fog params, lighting params, terrain vertex, etc.)
pub mod types;
/// Phase 2.2 / T6: rasterise per-vertex biome weights into RGBA8 splat maps.
pub mod terrain_splat_builder;
/// Phase 2.2 / T7: editor-side wrapper around TerrainMaterialManager.
pub mod terrain_splat;
mod widget;

// Physics debug types are exported for external configuration
#[allow(unused_imports)] // Re-exported for external API consumers
pub use engine_adapter::EditorQualityPreset;
#[allow(unused_imports)]
pub use physics_renderer::{PhysicsDebugOptions, PhysicsDebugRenderer};
#[allow(unused_imports)]
pub use renderer::RenderMode;
// Shared types — canonical exports
#[allow(unused_imports)]
pub use types::{
    GltfAnimChannel, GltfAnimationClip, GltfChannelProperty, GltfInterpolation, GltfJoint,
    GltfSkeleton, SceneLight, TerrainFogParams, TerrainLightingParams, TerrainVertex, WaterStyle,
    WeatherKind, MATERIAL_DISPLAY_NAMES, MATERIAL_NAMES,
};
pub use widget::ViewportLayout;
pub use widget::ViewportWidget;

#[allow(unused_imports)]
pub use blueprint_overlay::{BlueprintOverlay, ZoneOverlayData};
#[allow(unused_imports)]
pub use camera::OrbitCamera;
