//! Blueprint Zone system for defining polygon-bounded terrain zones.
//!
//! Zones are 2D polygons (in the XZ plane) that define areas where specific
//! biome presets or .blend scenes should be procedurally generated. Each zone
//! has a placement mode (Replica for exact 1:1 reproduction, Inspired for
//! procedural scatter) and an adaptive scaling system that preserves the
//! "heart and soul" of scenes at any size.

use crate::biome::BiomeType;
use crate::scatter::ScatterConfig;
use glam::Vec2;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

// ============================================================================
// Zone types
// ============================================================================

/// Unique identifier for a blueprint zone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ZoneId(pub u64);

/// How objects should be placed within a zone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlacementMode {
    /// Exact positions from the .blend scene, scaled spatially for zone size.
    /// Produces a faithful 1:1 replica at reference size.
    Replica,
    /// Procedural scatter using .blend as a template for types/density.
    /// More variation, less exact reproduction.
    Inspired,
}

impl Default for PlacementMode {
    fn default() -> Self {
        Self::Inspired
    }
}

/// The source of biome/vegetation data for a zone.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ZoneSource {
    /// Use a built-in biome preset (Grassland, Desert, Forest, etc.).
    BiomePreset(BiomeType),
    /// Use a decomposed .blend scene with its BiomePack data.
    BlendScene {
        /// Path to the BiomePack JSON file.
        pack_path: PathBuf,
        /// Placement mode for this zone.
        placement_mode: PlacementMode,
    },
}

impl Default for ZoneSource {
    fn default() -> Self {
        Self::BiomePreset(BiomeType::Grassland)
    }
}

/// A polygon-bounded terrain zone for procedural generation.
///
/// Vertices define a closed polygon in the world XZ plane. The Y coordinate
/// is sampled from the terrain heightmap at runtime.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueprintZone {
    /// Unique identifier.
    pub id: ZoneId,
    /// Human-readable name.
    pub name: String,
    /// Ordered polygon vertices in world XZ coordinates.
    /// The polygon is implicitly closed (last vertex connects to first).
    pub vertices: Vec<Vec2>,
    /// What to generate inside this zone.
    pub source: ZoneSource,
    /// Priority for overlap resolution (higher wins).
    pub priority: i32,
    /// Whether this zone should be included in generation.
    pub enabled: bool,
    /// Per-zone scatter config overrides (applied on top of source defaults).
    pub scatter_config_override: Option<ScatterConfig>,
    /// Width of the boundary blend margin in world units.
    /// Auto-blend smoothly transitions between zone terrain and surrounding terrain
    /// within this distance from the polygon edge.
    pub blend_margin: f32,
    /// Per-vertex blend mask for manual paint refinement.
    /// Same resolution as the terrain heightmap. Values 0.0 = full zone, 1.0 = full surrounding.
    /// When `None`, only auto-blend is used.
    pub blend_mask: Option<BlendMask>,
    /// Manual override for the scene-to-zone scale ratio.
    /// When `Some(ratio)`, adaptive scaling uses this value instead of computing
    /// from zone area / scene footprint area. Range: 0.25–4.0 (1.0 = exact 1:1).
    /// When `None`, automatic scaling is used.
    #[serde(default)]
    pub adaptive_scale_override: Option<f32>,
}

impl BlueprintZone {
    /// Create a new empty zone with default settings.
    pub fn new(id: ZoneId, name: String) -> Self {
        Self {
            id,
            name,
            vertices: Vec::new(),
            source: ZoneSource::default(),
            priority: 0,
            enabled: true,
            scatter_config_override: None,
            blend_margin: 10.0,
            blend_mask: None,
            adaptive_scale_override: None,
        }
    }

    /// Compute the area of the polygon using the shoelace formula.
    /// Returns 0 if fewer than 3 vertices.
    pub fn area(&self) -> f32 {
        polygon_area(&self.vertices)
    }

    /// Compute the centroid (geometric center) of the polygon.
    /// Returns `Vec2::ZERO` if fewer than 1 vertex.
    pub fn centroid(&self) -> Vec2 {
        polygon_centroid(&self.vertices)
    }

    /// Compute the axis-aligned bounding rectangle of the polygon.
    /// Returns `(min, max)` corners. Returns `(Vec2::ZERO, Vec2::ZERO)` if empty.
    pub fn bounding_rect(&self) -> (Vec2, Vec2) {
        polygon_bounding_rect(&self.vertices)
    }

    /// Check if a world XZ point is inside this zone's polygon.
    pub fn contains_point(&self, point: Vec2) -> bool {
        point_in_polygon(point, &self.vertices)
    }

    /// Get the minimum distance from a point to any edge of the polygon.
    pub fn distance_to_edge(&self, point: Vec2) -> f32 {
        point_distance_to_polygon_edge(point, &self.vertices)
    }

    /// Check if the polygon has enough vertices to be valid (>= 3).
    pub fn is_valid(&self) -> bool {
        self.vertices.len() >= 3
    }
}

// ============================================================================
// Blend mask
// ============================================================================

/// Per-vertex blend mask for manual boundary blending refinement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlendMask {
    /// Blend weight values. 0.0 = full zone terrain, 1.0 = full surrounding terrain.
    pub data: Vec<f32>,
    /// Resolution of the mask grid (same as heightmap resolution).
    pub resolution: u32,
    /// World-space bounds this mask covers (min_x, min_z, max_x, max_z).
    pub world_bounds: (f32, f32, f32, f32),
}

impl BlendMask {
    /// Create a new blend mask initialized to 0.0 (full zone terrain).
    pub fn new(resolution: u32, world_bounds: (f32, f32, f32, f32)) -> Self {
        let size = (resolution * resolution) as usize;
        Self {
            data: vec![0.0; size],
            resolution,
            world_bounds,
        }
    }

    /// Sample the blend weight at a world XZ position using bilinear interpolation.
    pub fn sample(&self, x: f32, z: f32) -> f32 {
        let (min_x, min_z, max_x, max_z) = self.world_bounds;
        let range_x = max_x - min_x;
        let range_z = max_z - min_z;
        if range_x <= 0.0 || range_z <= 0.0 {
            return 0.0;
        }

        let u = ((x - min_x) / range_x).clamp(0.0, 1.0);
        let v = ((z - min_z) / range_z).clamp(0.0, 1.0);

        let fx = u * (self.resolution - 1) as f32;
        let fz = v * (self.resolution - 1) as f32;
        let ix = fx as u32;
        let iz = fz as u32;
        let fx = fx - ix as f32;
        let fz = fz - iz as f32;

        let ix1 = (ix + 1).min(self.resolution - 1);
        let iz1 = (iz + 1).min(self.resolution - 1);

        let v00 = self.data[(iz * self.resolution + ix) as usize];
        let v10 = self.data[(iz * self.resolution + ix1) as usize];
        let v01 = self.data[(iz1 * self.resolution + ix) as usize];
        let v11 = self.data[(iz1 * self.resolution + ix1) as usize];

        let top = v00 * (1.0 - fx) + v10 * fx;
        let bottom = v01 * (1.0 - fx) + v11 * fx;
        (top * (1.0 - fz) + bottom * fz).clamp(0.0, 1.0)
    }

    /// Set the blend weight at a grid index.
    pub fn set(&mut self, x: u32, z: u32, value: f32) {
        if x < self.resolution && z < self.resolution {
            self.data[(z * self.resolution + x) as usize] = value.clamp(0.0, 1.0);
        }
    }
}

// ============================================================================
// Zone registry
// ============================================================================

/// Manages all blueprint zones in a scene.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoneRegistry {
    zones: Vec<BlueprintZone>,
    next_id: u64,
}

impl ZoneRegistry {
    /// Create an empty zone registry.
    pub fn new() -> Self {
        Self {
            zones: Vec::new(),
            next_id: 1,
        }
    }

    /// Allocate a new unique zone ID.
    pub fn next_zone_id(&mut self) -> ZoneId {
        let id = ZoneId(self.next_id);
        self.next_id += 1;
        id
    }

    /// Add a zone to the registry.
    pub fn add_zone(&mut self, zone: BlueprintZone) {
        // Ensure next_id stays ahead of any manually-set IDs
        if zone.id.0 >= self.next_id {
            self.next_id = zone.id.0 + 1;
        }
        self.zones.push(zone);
    }

    /// Remove a zone by ID. Returns the removed zone if found.
    pub fn remove_zone(&mut self, id: ZoneId) -> Option<BlueprintZone> {
        if let Some(pos) = self.zones.iter().position(|z| z.id == id) {
            Some(self.zones.remove(pos))
        } else {
            None
        }
    }

    /// Get an immutable reference to a zone by ID.
    pub fn get_zone(&self, id: ZoneId) -> Option<&BlueprintZone> {
        self.zones.iter().find(|z| z.id == id)
    }

    /// Get a mutable reference to a zone by ID.
    pub fn get_zone_mut(&mut self, id: ZoneId) -> Option<&mut BlueprintZone> {
        self.zones.iter_mut().find(|z| z.id == id)
    }

    /// Get all zones that contain the given world XZ point, sorted by priority (highest first).
    pub fn get_zones_at_point(&self, point: Vec2) -> Vec<&BlueprintZone> {
        let mut zones: Vec<&BlueprintZone> = self
            .zones
            .iter()
            .filter(|z| z.enabled && z.contains_point(point))
            .collect();
        zones.sort_by(|a, b| b.priority.cmp(&a.priority));
        zones
    }

    /// Get all zones whose bounding rect overlaps with the given chunk bounds.
    pub fn zones_overlapping_rect(&self, min: Vec2, max: Vec2) -> Vec<&BlueprintZone> {
        self.zones
            .iter()
            .filter(|z| {
                if !z.enabled || z.vertices.len() < 3 {
                    return false;
                }
                let (z_min, z_max) = z.bounding_rect();
                // AABB overlap test
                z_min.x <= max.x && z_max.x >= min.x && z_min.y <= max.y && z_max.y >= min.y
            })
            .collect()
    }

    /// Get all zones in the registry.
    pub fn zones(&self) -> &[BlueprintZone] {
        &self.zones
    }

    /// Get mutable access to all zones.
    pub fn zones_mut(&mut self) -> &mut Vec<BlueprintZone> {
        &mut self.zones
    }

    /// Number of zones in the registry.
    pub fn len(&self) -> usize {
        self.zones.len()
    }

    /// Check if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.zones.is_empty()
    }

    /// Save the zone registry to a JSON file.
    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Load a zone registry from a JSON file.
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(&content)?)
    }
}

impl Default for ZoneRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Adaptive scaling
// ============================================================================

/// Parameters for adaptively scaling .blend scene content to fit a zone.
///
/// Uses a hybrid approach: both object density and individual object scale
/// are adjusted, so smaller areas feel coherent rather than just sparse.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct AdaptiveScaleParams {
    /// Area of the original .blend scene footprint.
    pub reference_area: f32,
    /// Area of the target blueprint zone polygon.
    pub zone_area: f32,
    /// Density multiplier: `sqrt(ratio)`. More/fewer objects.
    pub density_multiplier: f32,
    /// Object scale multiplier: `ratio^0.25`. Gentle size adjustment (0.5–2.0×).
    pub scale_multiplier: f32,
    /// Position scale for Replica mode: `sqrt(ratio)`. Spatial compression/expansion.
    pub position_scale: f32,
}

impl AdaptiveScaleParams {
    /// Compute adaptive scaling from reference and zone areas.
    pub fn compute(reference_area: f32, zone_area: f32) -> Self {
        let reference_area = reference_area.max(1.0); // Avoid division by zero
        let zone_area = zone_area.max(1.0);
        let ratio = zone_area / reference_area;

        Self {
            reference_area,
            zone_area,
            density_multiplier: ratio.sqrt().clamp(0.1, 10.0),
            scale_multiplier: ratio.powf(0.25).clamp(0.5, 2.0),
            position_scale: ratio.sqrt(),
        }
    }

    /// Identity scaling — no adjustment (1:1 match).
    pub fn identity() -> Self {
        Self {
            reference_area: 1.0,
            zone_area: 1.0,
            density_multiplier: 1.0,
            scale_multiplier: 1.0,
            position_scale: 1.0,
        }
    }
}

// ============================================================================
// Polygon math utilities
// ============================================================================

/// Test if a point is inside a polygon using the ray-casting algorithm.
///
/// Casts a ray from the point in the +X direction and counts edge crossings.
/// An odd count means the point is inside.
pub fn point_in_polygon(point: Vec2, vertices: &[Vec2]) -> bool {
    let n = vertices.len();
    if n < 3 {
        return false;
    }

    let mut inside = false;
    let mut j = n - 1;
    for i in 0..n {
        let vi = vertices[i];
        let vj = vertices[j];

        // Check if the ray crosses this edge
        if (vi.y > point.y) != (vj.y > point.y) {
            let x_intersect = vj.x + (point.y - vj.y) / (vi.y - vj.y) * (vi.x - vj.x);
            if point.x < x_intersect {
                inside = !inside;
            }
        }
        j = i;
    }
    inside
}

/// Compute the area of a polygon using the shoelace formula.
/// Returns the absolute area (always positive).
pub fn polygon_area(vertices: &[Vec2]) -> f32 {
    let n = vertices.len();
    if n < 3 {
        return 0.0;
    }

    let mut area = 0.0f32;
    let mut j = n - 1;
    for i in 0..n {
        area += vertices[j].x * vertices[i].y;
        area -= vertices[i].x * vertices[j].y;
        j = i;
    }
    area.abs() * 0.5
}

/// Compute the centroid of a polygon.
/// Returns `Vec2::ZERO` if the polygon is degenerate.
pub fn polygon_centroid(vertices: &[Vec2]) -> Vec2 {
    let n = vertices.len();
    if n == 0 {
        return Vec2::ZERO;
    }
    if n == 1 {
        return vertices[0];
    }
    if n == 2 {
        return (vertices[0] + vertices[1]) * 0.5;
    }

    let mut cx = 0.0f32;
    let mut cy = 0.0f32;
    let mut signed_area = 0.0f32;

    let mut j = n - 1;
    for i in 0..n {
        let cross = vertices[j].x * vertices[i].y - vertices[i].x * vertices[j].y;
        signed_area += cross;
        cx += (vertices[j].x + vertices[i].x) * cross;
        cy += (vertices[j].y + vertices[i].y) * cross;
        j = i;
    }

    signed_area *= 0.5;
    if signed_area.abs() < f32::EPSILON {
        // Degenerate polygon — fall back to simple average
        let sum: Vec2 = vertices.iter().copied().sum();
        return sum / n as f32;
    }

    let factor = 1.0 / (6.0 * signed_area);
    Vec2::new(cx * factor, cy * factor)
}

/// Compute the axis-aligned bounding rectangle of a polygon.
/// Returns `(min_corner, max_corner)`.
pub fn polygon_bounding_rect(vertices: &[Vec2]) -> (Vec2, Vec2) {
    if vertices.is_empty() {
        return (Vec2::ZERO, Vec2::ZERO);
    }

    let mut min = vertices[0];
    let mut max = vertices[0];
    for &v in &vertices[1..] {
        min = min.min(v);
        max = max.max(v);
    }
    (min, max)
}

/// Compute the minimum distance from a point to any edge of the polygon.
///
/// Returns `f32::MAX` for empty or single-vertex polygons.
pub fn point_distance_to_polygon_edge(point: Vec2, vertices: &[Vec2]) -> f32 {
    let n = vertices.len();
    if n < 2 {
        return f32::MAX;
    }

    let mut min_dist = f32::MAX;
    for i in 0..n {
        let j = (i + 1) % n;
        let dist = point_to_segment_distance(point, vertices[i], vertices[j]);
        if dist < min_dist {
            min_dist = dist;
        }
    }
    min_dist
}

/// Minimum distance from a point to a line segment.
fn point_to_segment_distance(point: Vec2, a: Vec2, b: Vec2) -> f32 {
    let ab = b - a;
    let len_sq = ab.length_squared();
    if len_sq < f32::EPSILON {
        return point.distance(a);
    }

    let t = ((point - a).dot(ab) / len_sq).clamp(0.0, 1.0);
    let projection = a + ab * t;
    point.distance(projection)
}

/// Check if a polygon's bounding rect overlaps with a given axis-aligned rectangle.
pub fn polygon_overlaps_rect(vertices: &[Vec2], rect_min: Vec2, rect_max: Vec2) -> bool {
    if vertices.len() < 3 {
        return false;
    }
    let (poly_min, poly_max) = polygon_bounding_rect(vertices);
    poly_min.x <= rect_max.x
        && poly_max.x >= rect_min.x
        && poly_min.y <= rect_max.y
        && poly_max.y >= rect_min.y
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- Point-in-polygon tests ---

    #[test]
    fn test_point_in_triangle() {
        let triangle = vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 0.0),
            Vec2::new(5.0, 10.0),
        ];
        assert!(point_in_polygon(Vec2::new(5.0, 3.0), &triangle));
        assert!(!point_in_polygon(Vec2::new(-1.0, 0.0), &triangle));
        assert!(!point_in_polygon(Vec2::new(11.0, 0.0), &triangle));
    }

    #[test]
    fn test_point_in_square() {
        let square = vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 0.0),
            Vec2::new(10.0, 10.0),
            Vec2::new(0.0, 10.0),
        ];
        assert!(point_in_polygon(Vec2::new(5.0, 5.0), &square));
        assert!(point_in_polygon(Vec2::new(1.0, 1.0), &square));
        assert!(!point_in_polygon(Vec2::new(-1.0, 5.0), &square));
        assert!(!point_in_polygon(Vec2::new(11.0, 5.0), &square));
    }

    #[test]
    fn test_point_in_concave_polygon() {
        // L-shaped polygon
        let l_shape = vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 0.0),
            Vec2::new(10.0, 5.0),
            Vec2::new(5.0, 5.0),
            Vec2::new(5.0, 10.0),
            Vec2::new(0.0, 10.0),
        ];
        assert!(point_in_polygon(Vec2::new(2.0, 2.0), &l_shape)); // bottom-left arm
        assert!(point_in_polygon(Vec2::new(2.0, 8.0), &l_shape)); // top-left arm
        assert!(!point_in_polygon(Vec2::new(8.0, 8.0), &l_shape)); // outside concavity
    }

    #[test]
    fn test_polygon_degenerate() {
        assert!(!point_in_polygon(Vec2::ZERO, &[]));
        assert!(!point_in_polygon(Vec2::ZERO, &[Vec2::ZERO]));
        assert!(!point_in_polygon(Vec2::ZERO, &[Vec2::ZERO, Vec2::ONE]));
    }

    // --- Area tests ---

    #[test]
    fn test_area_square() {
        let square = vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 0.0),
            Vec2::new(10.0, 10.0),
            Vec2::new(0.0, 10.0),
        ];
        let area = polygon_area(&square);
        assert!((area - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_area_triangle() {
        let triangle = vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 0.0),
            Vec2::new(5.0, 10.0),
        ];
        let area = polygon_area(&triangle);
        assert!((area - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_area_degenerate() {
        assert_eq!(polygon_area(&[]), 0.0);
        assert_eq!(polygon_area(&[Vec2::ZERO]), 0.0);
        assert_eq!(polygon_area(&[Vec2::ZERO, Vec2::ONE]), 0.0);
    }

    // --- Centroid tests ---

    #[test]
    fn test_centroid_square() {
        let square = vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 0.0),
            Vec2::new(10.0, 10.0),
            Vec2::new(0.0, 10.0),
        ];
        let c = polygon_centroid(&square);
        assert!((c.x - 5.0).abs() < 0.01);
        assert!((c.y - 5.0).abs() < 0.01);
    }

    #[test]
    fn test_centroid_degenerate() {
        assert_eq!(polygon_centroid(&[]), Vec2::ZERO);
        assert_eq!(
            polygon_centroid(&[Vec2::new(3.0, 4.0)]),
            Vec2::new(3.0, 4.0)
        );
    }

    // --- Bounding rect tests ---

    #[test]
    fn test_bounding_rect() {
        let poly = vec![
            Vec2::new(-5.0, 2.0),
            Vec2::new(10.0, -3.0),
            Vec2::new(3.0, 15.0),
        ];
        let (min, max) = polygon_bounding_rect(&poly);
        assert_eq!(min.x, -5.0);
        assert_eq!(min.y, -3.0);
        assert_eq!(max.x, 10.0);
        assert_eq!(max.y, 15.0);
    }

    // --- Distance to edge tests ---

    #[test]
    fn test_distance_to_edge_square() {
        let square = vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 0.0),
            Vec2::new(10.0, 10.0),
            Vec2::new(0.0, 10.0),
        ];
        let d = point_distance_to_polygon_edge(Vec2::new(5.0, 5.0), &square);
        assert!(
            (d - 5.0).abs() < 0.01,
            "Center of 10×10 square should be 5 from edge"
        );

        let d2 = point_distance_to_polygon_edge(Vec2::new(1.0, 5.0), &square);
        assert!((d2 - 1.0).abs() < 0.01, "1 unit from left edge");
    }

    #[test]
    fn test_distance_to_segment() {
        let d = point_to_segment_distance(Vec2::new(5.0, 5.0), Vec2::ZERO, Vec2::new(10.0, 0.0));
        assert!((d - 5.0).abs() < 0.01);

        // Point closest to endpoint
        let d2 = point_to_segment_distance(Vec2::new(-3.0, 0.0), Vec2::ZERO, Vec2::new(10.0, 0.0));
        assert!((d2 - 3.0).abs() < 0.01);
    }

    // --- Adaptive scaling tests ---

    #[test]
    fn test_adaptive_scale_identity() {
        let params = AdaptiveScaleParams::compute(100.0, 100.0);
        assert!((params.density_multiplier - 1.0).abs() < 0.01);
        assert!((params.scale_multiplier - 1.0).abs() < 0.01);
        assert!((params.position_scale - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_adaptive_scale_half_area() {
        let params = AdaptiveScaleParams::compute(100.0, 50.0);
        // sqrt(0.5) ≈ 0.707
        assert!((params.density_multiplier - 0.707).abs() < 0.01);
        // 0.5^0.25 ≈ 0.840
        assert!((params.scale_multiplier - 0.840).abs() < 0.01);
    }

    #[test]
    fn test_adaptive_scale_double_area() {
        let params = AdaptiveScaleParams::compute(100.0, 200.0);
        // sqrt(2) ≈ 1.414
        assert!((params.density_multiplier - 1.414).abs() < 0.01);
        // 2^0.25 ≈ 1.189
        assert!((params.scale_multiplier - 1.189).abs() < 0.01);
    }

    #[test]
    fn test_adaptive_scale_clamping() {
        // Very tiny zone
        let params = AdaptiveScaleParams::compute(10000.0, 1.0);
        assert!(params.scale_multiplier >= 0.5);
        assert!(params.density_multiplier >= 0.1);

        // Very large zone
        let params = AdaptiveScaleParams::compute(1.0, 100000.0);
        assert!(params.scale_multiplier <= 2.0);
        assert!(params.density_multiplier <= 10.0);
    }

    // --- Zone registry tests ---

    #[test]
    fn test_zone_registry_crud() {
        let mut reg = ZoneRegistry::new();
        assert!(reg.is_empty());

        let id = reg.next_zone_id();
        let mut zone = BlueprintZone::new(id, "Test Zone".to_string());
        zone.vertices = vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(100.0, 0.0),
            Vec2::new(100.0, 100.0),
            Vec2::new(0.0, 100.0),
        ];
        reg.add_zone(zone);
        assert_eq!(reg.len(), 1);
        assert!(reg.get_zone(id).is_some());

        let removed = reg.remove_zone(id);
        assert!(removed.is_some());
        assert!(reg.is_empty());
    }

    #[test]
    fn test_zone_registry_point_query() {
        let mut reg = ZoneRegistry::new();

        let id1 = reg.next_zone_id();
        let mut z1 = BlueprintZone::new(id1, "Low Priority".to_string());
        z1.priority = 1;
        z1.vertices = vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(100.0, 0.0),
            Vec2::new(100.0, 100.0),
            Vec2::new(0.0, 100.0),
        ];
        reg.add_zone(z1);

        let id2 = reg.next_zone_id();
        let mut z2 = BlueprintZone::new(id2, "High Priority".to_string());
        z2.priority = 10;
        z2.vertices = vec![
            Vec2::new(25.0, 25.0),
            Vec2::new(75.0, 25.0),
            Vec2::new(75.0, 75.0),
            Vec2::new(25.0, 75.0),
        ];
        reg.add_zone(z2);

        // Point in both zones — should return highest priority first
        let zones = reg.get_zones_at_point(Vec2::new(50.0, 50.0));
        assert_eq!(zones.len(), 2);
        assert_eq!(zones[0].id, id2); // Higher priority first

        // Point only in outer zone
        let zones = reg.get_zones_at_point(Vec2::new(5.0, 5.0));
        assert_eq!(zones.len(), 1);
        assert_eq!(zones[0].id, id1);

        // Point outside both zones
        let zones = reg.get_zones_at_point(Vec2::new(200.0, 200.0));
        assert!(zones.is_empty());
    }

    #[test]
    fn test_zone_registry_rect_overlap() {
        let mut reg = ZoneRegistry::new();

        let id = reg.next_zone_id();
        let mut zone = BlueprintZone::new(id, "Zone A".to_string());
        zone.vertices = vec![
            Vec2::new(10.0, 10.0),
            Vec2::new(50.0, 10.0),
            Vec2::new(50.0, 50.0),
            Vec2::new(10.0, 50.0),
        ];
        reg.add_zone(zone);

        // Overlapping rect
        let zones = reg.zones_overlapping_rect(Vec2::new(0.0, 0.0), Vec2::new(20.0, 20.0));
        assert_eq!(zones.len(), 1);

        // Non-overlapping rect
        let zones = reg.zones_overlapping_rect(Vec2::new(60.0, 60.0), Vec2::new(80.0, 80.0));
        assert!(zones.is_empty());
    }

    #[test]
    fn test_zone_serialization_roundtrip() {
        let mut reg = ZoneRegistry::new();
        let id = reg.next_zone_id();
        let mut zone = BlueprintZone::new(id, "SerTest".to_string());
        zone.vertices = vec![
            Vec2::new(1.0, 2.0),
            Vec2::new(3.0, 4.0),
            Vec2::new(5.0, 6.0),
        ];
        zone.source = ZoneSource::BiomePreset(BiomeType::Desert);
        zone.priority = 5;
        reg.add_zone(zone);

        let json = serde_json::to_string(&reg).unwrap();
        let loaded: ZoneRegistry = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded.zones()[0].name, "SerTest");
        assert_eq!(loaded.zones()[0].priority, 5);
    }

    // --- Blend mask tests ---

    #[test]
    fn test_blend_mask_sample() {
        let mut mask = BlendMask::new(2, (0.0, 0.0, 10.0, 10.0));
        // Set corner values: (0,0)=0, (1,0)=1, (0,1)=1, (1,1)=0
        mask.set(0, 0, 0.0);
        mask.set(1, 0, 1.0);
        mask.set(0, 1, 1.0);
        mask.set(1, 1, 0.0);

        // Center should interpolate to ~0.5
        let center = mask.sample(5.0, 5.0);
        assert!(
            center > 0.3 && center < 0.7,
            "Center should be ~0.5, got {center}"
        );

        // Corner samples
        let bottom_left = mask.sample(0.0, 0.0);
        assert!((bottom_left - 0.0).abs() < 0.1);

        let bottom_right = mask.sample(10.0, 0.0);
        assert!((bottom_right - 1.0).abs() < 0.1);
    }

    // --- BlueprintZone method tests ---

    #[test]
    fn test_zone_contains_point() {
        let mut zone = BlueprintZone::new(ZoneId(1), "Test".to_string());
        zone.vertices = vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 0.0),
            Vec2::new(10.0, 10.0),
            Vec2::new(0.0, 10.0),
        ];
        assert!(zone.contains_point(Vec2::new(5.0, 5.0)));
        assert!(!zone.contains_point(Vec2::new(15.0, 5.0)));
    }

    #[test]
    fn test_zone_validity() {
        let mut zone = BlueprintZone::new(ZoneId(1), "Test".to_string());
        assert!(!zone.is_valid()); // No vertices

        zone.vertices.push(Vec2::ZERO);
        zone.vertices.push(Vec2::ONE);
        assert!(!zone.is_valid()); // Only 2 vertices

        zone.vertices.push(Vec2::new(1.0, 0.0));
        assert!(zone.is_valid()); // 3 vertices = valid
    }

    #[test]
    fn test_polygon_overlaps_rect() {
        let poly = vec![
            Vec2::new(5.0, 5.0),
            Vec2::new(15.0, 5.0),
            Vec2::new(15.0, 15.0),
            Vec2::new(5.0, 15.0),
        ];
        assert!(polygon_overlaps_rect(
            &poly,
            Vec2::ZERO,
            Vec2::new(10.0, 10.0)
        ));
        assert!(!polygon_overlaps_rect(
            &poly,
            Vec2::new(20.0, 20.0),
            Vec2::new(30.0, 30.0)
        ));
    }
}
