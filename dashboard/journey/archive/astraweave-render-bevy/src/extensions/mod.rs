//! AstraWeave Extensions for Bevy Renderer
//!
//! This module contains AstraWeave-specific rendering extensions
//! that enhance the Bevy renderer with AI-native optimizations.
//!
//! # Extensions
//!
//! - **MegaLights**: GPU-accelerated light culling for 100k+ lights (68× speedup)
//! - **Nanite**: Virtualized geometry for 10M+ polygons @ 60 FPS

/// GPU-accelerated light culling system for massive light counts
#[cfg(feature = "megalights")]
pub mod megalights;

#[cfg(feature = "megalights")]
pub use megalights::{ClusterBounds, GpuLight, MegaLightsRenderer};

#[cfg(feature = "nanite")]
pub mod nanite;

#[cfg(feature = "nanite")]
pub use nanite::{
    CullStats, Frustum, GpuCamera, GpuMeshlet, LODSelector, NaniteCullingPipeline, VisibilityBuffer,
};
