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
//! ```text
//! ViewportWidget (egui integration)
//!     ↓
//! ViewportRenderer (rendering coordinator)
//!     ↓
//! ├─ GridRenderer (floor grid + axes)
//! ├─ EntityRenderer (world entities)
//! ├─ GizmoRenderer (transform handles)
//! └─ PhysicsDebugRenderer (collider wireframes)
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

pub mod camera;
#[cfg(feature = "astraweave-render")]
mod engine_adapter;
mod entity_renderer;
mod gizmo_renderer;
mod grid_renderer;
mod physics_renderer;
mod rain_renderer;
mod renderer;
mod scatter_renderer;
mod skybox_renderer;
pub mod terrain_renderer;
pub mod toolbar;
mod water_renderer;
mod widget;

// Physics debug types are exported for external configuration
#[allow(unused_imports)]
pub use physics_renderer::{PhysicsDebugOptions, PhysicsDebugRenderer};
#[allow(unused_imports)]
pub use terrain_renderer::{TerrainFogParams, TerrainRenderer, TerrainVertex};
pub use widget::ViewportWidget;

#[allow(unused_imports)]
pub use camera::OrbitCamera;
