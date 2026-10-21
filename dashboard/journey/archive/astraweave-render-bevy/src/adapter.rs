// ECS Adapter - Bridges AstraWeave ECS ↔ Bevy Render Data
// Day 2 implementation

//! Adapter layer for extracting render data from AstraWeave ECS
//!
//! This module provides the translation layer between AstraWeave's custom ECS
//! and Bevy's rendering requirements. It extracts components (Transform, Mesh,
//! Material, Light) and converts them to Bevy-compatible structures.
//!
//! ## Architecture
//!
//! ```text
//! AstraWeave World → RenderAdapter → Bevy Renderer
//!        ↓
//!  Components (Transform, Mesh, Material, Light)
//!        ↓
//!  extract_all()
//!        ↓
//!  Bevy-compatible data structures
//!        ↓
//!  submit_render_data()
//!        ↓
//!  GPU Rendering
//! ```
//!
//! ## Usage
//!
//! ```
//! use astraweave_render_bevy::{RenderAdapter, BevyRenderer};
//! use astraweave_render_bevy::adapter::{RenderTransform, RenderMesh, RenderMaterial};
//! use astraweave_ecs::World;
//!
//! let mut adapter = RenderAdapter::new();
//! let mut world = World::new();
//!
//! // Spawn entities with render components
//! let entity = world.spawn();
//! world.insert(entity, RenderTransform::default());
//! world.insert(entity, RenderMesh { handle: 0 });
//! world.insert(entity, RenderMaterial {
//!     base_color: [1.0, 1.0, 1.0, 1.0],
//!     base_color_texture: None,
//!     normal_texture: None,
//!     metallic_roughness_texture: None,
//!     metallic: 0.0,
//!     roughness: 0.5,
//! });
//!
//! // Extract data every frame
//! adapter.extract_all(&world).unwrap();
//!
//! // Submit to renderer
//! // adapter.submit_render_data(&mut renderer);
//! ```
//!
//! ## Component Mapping
//!
//! | AstraWeave Component | Bevy Equivalent | Purpose |
//! |---------------------|----------------|---------|
//! | `RenderTransform` | `GlobalTransform` | Position/rotation/scale |
//! | `RenderMesh` | `Handle<Mesh>` | Geometry data |
//! | `RenderMaterial` | `Handle<StandardMaterial>` | PBR material |
//! | `DirectionalLight` | `DirectionalLight` | Sun/moon lighting |
//! | `PointLight` | `PointLight` | Local light sources |
//! | `SpotLight` | `SpotLight` | Focused light beams |

use anyhow::Result;
use glam::{Mat4, Quat, Vec3, Vec4};
use thiserror::Error;

use crate::render::light::{
    DirectionalLight as BevyDirectionalLight, PointLight as BevyPointLight,
    SpotLight as BevySpotLight,
};
use crate::render::material::{StandardMaterial, TextureHandle};
use crate::render::mesh::MeshHandle;
use astraweave_ecs::{Entity, World};

// ============================================================================
// AstraWeave Render Components (what users attach to entities)
// ============================================================================

/// Transform component for rendering (world position/rotation/scale)
#[derive(Debug, Clone, Copy)]
pub struct RenderTransform {
    /// Translation (world position)
    pub translation: Vec3,
    /// Rotation (quaternion)
    pub rotation: Quat,
    /// Scale
    pub scale: Vec3,
}

impl Default for RenderTransform {
    fn default() -> Self {
        Self {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }
}

impl RenderTransform {
    /// Create a new transform at position
    pub fn from_translation(translation: Vec3) -> Self {
        Self {
            translation,
            ..Default::default()
        }
    }

    /// Convert to 4x4 matrix
    pub fn to_matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }
}

/// Mesh component (references mesh data)
#[derive(Debug, Clone, Copy)]
pub struct RenderMesh {
    /// Handle to mesh data
    pub handle: u64,
}

/// Material component (PBR material properties)
#[derive(Debug, Clone)]
pub struct RenderMaterial {
    /// Base color (RGBA)
    pub base_color: [f32; 4],
    /// Base color texture (optional)
    pub base_color_texture: Option<u64>,
    /// Normal map texture (optional)
    pub normal_texture: Option<u64>,
    /// Metallic-roughness texture (optional)
    pub metallic_roughness_texture: Option<u64>,
    /// Metallic factor (0.0 = dielectric, 1.0 = conductor)
    pub metallic: f32,
    /// Roughness factor (0.0 = smooth, 1.0 = rough)
    pub roughness: f32,
}

impl Default for RenderMaterial {
    fn default() -> Self {
        Self {
            base_color: [1.0, 1.0, 1.0, 1.0],
            base_color_texture: None,
            normal_texture: None,
            metallic_roughness_texture: None,
            metallic: 0.0,
            roughness: 0.5,
        }
    }
}

/// Directional light component (sun/moon)
#[derive(Debug, Clone)]
pub struct DirectionalLight {
    /// Light direction (normalized)
    pub direction: Vec3,
    /// Light color (linear RGB)
    pub color: Vec3,
    /// Illuminance in lux
    pub illuminance: f32,
    /// Cast shadows
    pub shadows_enabled: bool,
}

impl Default for DirectionalLight {
    fn default() -> Self {
        Self {
            direction: Vec3::new(-0.5, -1.0, -0.3).normalize(),
            color: Vec3::ONE,
            illuminance: 100000.0,
            shadows_enabled: true,
        }
    }
}

/// Point light component
#[derive(Debug, Clone)]
pub struct PointLight {
    /// Light color (linear RGB)
    pub color: Vec3,
    /// Intensity in lumens
    pub intensity: f32,
    /// Maximum range
    pub range: f32,
    /// Light radius (for soft shadows)
    pub radius: f32,
    /// Cast shadows
    pub shadows_enabled: bool,
}

impl Default for PointLight {
    fn default() -> Self {
        Self {
            color: Vec3::ONE,
            intensity: 800.0,
            range: 20.0,
            radius: 0.1,
            shadows_enabled: false,
        }
    }
}

/// Spot light component
#[derive(Debug, Clone)]
pub struct SpotLight {
    /// Direction (normalized)
    pub direction: Vec3,
    /// Light color (linear RGB)
    pub color: Vec3,
    /// Intensity in lumens
    pub intensity: f32,
    /// Maximum range
    pub range: f32,
    /// Inner cone angle (radians)
    pub inner_angle: f32,
    /// Outer cone angle (radians)
    pub outer_angle: f32,
    /// Cast shadows
    pub shadows_enabled: bool,
}

impl Default for SpotLight {
    fn default() -> Self {
        Self {
            direction: Vec3::NEG_Y,
            color: Vec3::ONE,
            intensity: 1000.0,
            range: 20.0,
            inner_angle: 0.0,
            outer_angle: std::f32::consts::FRAC_PI_4,
            shadows_enabled: false,
        }
    }
}

// ============================================================================
// Errors
// ============================================================================

/// Errors that can occur during render data extraction
#[derive(Error, Debug)]
#[non_exhaustive]
#[must_use]
pub enum RenderExtractError {
    /// Failed to extract meshes
    #[error("Failed to extract meshes: {0}")]
    MeshExtraction(String),

    /// Failed to extract materials
    #[error("Failed to extract materials: {0}")]
    MaterialExtraction(String),

    /// Failed to extract lights
    #[error("Failed to extract lights: {0}")]
    LightExtraction(String),

    /// Component not found
    #[error("Component not found on entity {0:?}: {1}")]
    ComponentNotFound(Entity, String),
}

// ============================================================================
// Extracted Render Data (ready for GPU submission)
// ============================================================================

/// Extracted mesh instance (entity + transform + mesh + material)
#[derive(Debug, Clone)]
pub struct MeshInstance {
    /// Source entity
    pub entity: Entity,
    /// World transform matrix
    pub transform: Mat4,
    /// Mesh handle
    pub mesh: MeshHandle,
    /// Material (converted to Bevy StandardMaterial)
    pub material: StandardMaterial,
}

/// Extracted directional light (converted to Bevy format)
#[derive(Debug, Clone)]
pub struct ExtractedDirectionalLight {
    /// Source entity
    pub entity: Entity,
    /// Bevy directional light
    pub light: BevyDirectionalLight,
}

/// Extracted point light (entity + transform + light)
#[derive(Debug, Clone)]
pub struct ExtractedPointLight {
    /// Source entity
    pub entity: Entity,
    /// World position
    pub position: Vec3,
    /// Bevy point light
    pub light: BevyPointLight,
}

/// Extracted spot light (entity + transform + light)
#[derive(Debug, Clone)]
pub struct ExtractedSpotLight {
    /// Source entity
    pub entity: Entity,
    /// World position
    pub position: Vec3,
    /// Bevy spot light
    pub light: BevySpotLight,
}

// ============================================================================
// Render Adapter (main extraction logic)
// ============================================================================

/// Adapter for extracting render data from AstraWeave ECS
///
/// This is the ONLY bridge between AstraWeave and Bevy rendering.
/// It maintains independence by using component traits and HashMap storage.
pub struct RenderAdapter {
    // Extracted data (ready for GPU submission)
    mesh_instances: Vec<MeshInstance>,
    directional_lights: Vec<ExtractedDirectionalLight>,
    point_lights: Vec<ExtractedPointLight>,
    spot_lights: Vec<ExtractedSpotLight>,

    // Statistics (for debugging/profiling)
    stats: ExtractionStats,
}

/// Extraction statistics
#[derive(Debug, Default, Clone)]
pub struct ExtractionStats {
    /// Number of mesh instances extracted
    pub mesh_instances: usize,
    /// Number of directional lights extracted
    pub directional_lights: usize,
    /// Number of point lights extracted
    pub point_lights: usize,
    /// Number of spot lights extracted
    pub spot_lights: usize,
    /// Last extraction time (microseconds)
    pub extraction_time_us: u64,
}

impl RenderAdapter {
    /// Create a new render adapter
    pub fn new() -> Self {
        Self {
            mesh_instances: Vec::new(),
            directional_lights: Vec::new(),
            point_lights: Vec::new(),
            spot_lights: Vec::new(),
            stats: ExtractionStats::default(),
        }
    }

    /// Extract all render data from World
    ///
    /// This is the main entry point called every frame.
    pub fn extract_all(&mut self, world: &World) -> Result<()> {
        let start = std::time::Instant::now();

        // Clear previous frame data
        self.mesh_instances.clear();
        self.directional_lights.clear();
        self.point_lights.clear();
        self.spot_lights.clear();

        // Extract each component type
        self.extract_meshes(world)?;
        self.extract_directional_lights(world)?;
        self.extract_point_lights(world)?;
        self.extract_spot_lights(world)?;

        // Update stats
        self.stats.mesh_instances = self.mesh_instances.len();
        self.stats.directional_lights = self.directional_lights.len();
        self.stats.point_lights = self.point_lights.len();
        self.stats.spot_lights = self.spot_lights.len();
        self.stats.extraction_time_us = start.elapsed().as_micros() as u64;

        Ok(())
    }

    /// Extract mesh instances (entities with Transform + Mesh + Material)
    fn extract_meshes(&mut self, world: &World) -> Result<()> {
        use astraweave_ecs::Query;

        // Query for all entities with RenderMesh
        let query = Query::<RenderMesh>::new(world);

        for (entity, mesh) in query {
            // Check for required Transform and Material components
            if let Some(transform) = world.get::<RenderTransform>(entity) {
                if let Some(material) = world.get::<RenderMaterial>(entity) {
                    // Convert to Bevy-compatible mesh instance
                    let transform_matrix = transform.to_matrix();
                    let bevy_material = self.convert_material(material);

                    self.mesh_instances.push(MeshInstance {
                        entity,
                        transform: transform_matrix,
                        mesh: MeshHandle(mesh.handle),
                        material: bevy_material,
                    });
                }
            }
        }

        Ok(())
    }

    /// Extract directional lights
    fn extract_directional_lights(&mut self, world: &World) -> Result<()> {
        use astraweave_ecs::Query;

        let query = Query::<DirectionalLight>::new(world);

        for (entity, light) in query {
            let bevy_light = self.convert_directional_light(light);
            self.directional_lights.push(ExtractedDirectionalLight {
                entity,
                light: bevy_light,
            });
        }

        Ok(())
    }

    /// Extract point lights (requires Transform for position)
    fn extract_point_lights(&mut self, world: &World) -> Result<()> {
        use astraweave_ecs::Query;

        let query = Query::<PointLight>::new(world);

        for (entity, light) in query {
            if let Some(transform) = world.get::<RenderTransform>(entity) {
                let bevy_light = self.convert_point_light(light);
                self.point_lights.push(ExtractedPointLight {
                    entity,
                    position: transform.translation,
                    light: bevy_light,
                });
            }
        }

        Ok(())
    }

    /// Extract spot lights (requires Transform for position + rotation)
    fn extract_spot_lights(&mut self, world: &World) -> Result<()> {
        use astraweave_ecs::Query;

        let query = Query::<SpotLight>::new(world);

        for (entity, light) in query {
            if let Some(transform) = world.get::<RenderTransform>(entity) {
                // Calculate light direction from rotation
                let direction = transform.rotation * Vec3::NEG_Y;

                let bevy_light = self.convert_spot_light(light, direction);
                self.spot_lights.push(ExtractedSpotLight {
                    entity,
                    position: transform.translation,
                    light: bevy_light,
                });
            }
        }

        Ok(())
    }

    // ========================================================================
    // Conversion Methods (AstraWeave → Bevy)
    // ========================================================================

    /// Convert AstraWeave mesh instance to Bevy-compatible format
    #[allow(dead_code)]
    fn convert_mesh_instance(
        &self,
        entity: Entity,
        transform: &RenderTransform,
        mesh: &RenderMesh,
        material: &RenderMaterial,
    ) -> Result<MeshInstance> {
        Ok(MeshInstance {
            entity,
            transform: transform.to_matrix(),
            mesh: MeshHandle(mesh.handle),
            material: self.convert_material(material),
        })
    }

    /// Convert AstraWeave material to Bevy StandardMaterial
    fn convert_material(&self, mat: &RenderMaterial) -> StandardMaterial {
        StandardMaterial {
            base_color: Vec4::from_slice(&mat.base_color),
            base_color_texture: mat.base_color_texture.map(TextureHandle),
            normal_map_texture: mat.normal_texture.map(TextureHandle),
            metallic_roughness_texture: mat.metallic_roughness_texture.map(TextureHandle),
            metallic: mat.metallic,
            perceptual_roughness: mat.roughness,
            reflectance: 0.5,
        }
    }

    /// Convert AstraWeave directional light to Bevy
    #[allow(dead_code)]
    fn convert_directional_light(&self, light: &DirectionalLight) -> BevyDirectionalLight {
        BevyDirectionalLight {
            direction: light.direction,
            color: light.color,
            illuminance: light.illuminance,
            shadows_enabled: light.shadows_enabled,
        }
    }

    /// Convert AstraWeave point light to Bevy
    #[allow(dead_code)]
    fn convert_point_light(&self, light: &PointLight) -> BevyPointLight {
        BevyPointLight {
            position: Vec3::ZERO, // Will be set from transform
            color: light.color,
            intensity: light.intensity,
            range: light.range,
            radius: light.radius,
            shadows_enabled: light.shadows_enabled,
        }
    }

    /// Convert AstraWeave spot light to Bevy
    #[allow(dead_code)]
    fn convert_spot_light(&self, light: &SpotLight, direction: Vec3) -> BevySpotLight {
        BevySpotLight {
            position: Vec3::ZERO, // Will be set from transform
            direction,
            color: light.color,
            intensity: light.intensity,
            range: light.range,
            inner_angle: light.inner_angle,
            outer_angle: light.outer_angle,
            shadows_enabled: light.shadows_enabled,
        }
    }

    // ========================================================================
    // Public Accessors (for renderer to consume extracted data)
    // ========================================================================

    /// Get extracted mesh instances
    pub fn mesh_instances(&self) -> &[MeshInstance] {
        &self.mesh_instances
    }

    /// Get extracted directional lights
    pub fn directional_lights(&self) -> &[ExtractedDirectionalLight] {
        &self.directional_lights
    }

    /// Get extracted point lights
    pub fn point_lights(&self) -> &[ExtractedPointLight] {
        &self.point_lights
    }

    /// Get extracted spot lights
    pub fn spot_lights(&self) -> &[ExtractedSpotLight] {
        &self.spot_lights
    }

    /// Get extraction statistics
    pub fn stats(&self) -> &ExtractionStats {
        &self.stats
    }
}

impl Default for RenderAdapter {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adapter_creation() {
        let adapter = RenderAdapter::new();
        assert_eq!(adapter.mesh_instances().len(), 0);
        assert_eq!(adapter.directional_lights().len(), 0);
        assert_eq!(adapter.point_lights().len(), 0);
        assert_eq!(adapter.spot_lights().len(), 0);
    }

    #[test]
    fn test_material_conversion() {
        let adapter = RenderAdapter::new();
        let astraweave_mat = RenderMaterial {
            base_color: [1.0, 0.5, 0.25, 1.0],
            metallic: 0.8,
            roughness: 0.3,
            ..Default::default()
        };

        let bevy_mat = adapter.convert_material(&astraweave_mat);
        assert_eq!(bevy_mat.base_color, Vec4::new(1.0, 0.5, 0.25, 1.0));
        assert_eq!(bevy_mat.metallic, 0.8);
        assert_eq!(bevy_mat.perceptual_roughness, 0.3);
    }

    #[test]
    fn test_transform_matrix_conversion() {
        let transform = RenderTransform {
            translation: Vec3::new(1.0, 2.0, 3.0),
            rotation: Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
            scale: Vec3::ONE,
        };

        let matrix = transform.to_matrix();
        let expected = Mat4::from_scale_rotation_translation(
            Vec3::ONE,
            Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
            Vec3::new(1.0, 2.0, 3.0),
        );

        assert_eq!(matrix, expected);
    }

    #[test]
    fn test_extract_all_empty_world() {
        let mut adapter = RenderAdapter::new();
        let world = World::new();

        let result = adapter.extract_all(&world);
        assert!(result.is_ok());
        assert_eq!(adapter.stats().mesh_instances, 0);
        assert_eq!(adapter.stats().directional_lights, 0);
    }
}
