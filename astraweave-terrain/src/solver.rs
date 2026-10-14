//! TerrainSolver - Translates abstract LLM terrain requests into validated world coordinates
//!
//! This module provides the bridge between AI-generated terrain modification requests
//! (using relative locations like "ahead of me" or "to the north") and concrete
//! world coordinates that can be used by the terrain modification system.
//!
//! # Features
//! - Raycast-based location resolution for line-of-sight requests
//! - Cardinal direction offset calculation
//! - Biome compatibility validation
//! - Deterministic seed derivation for reproducible generation
//!
//! # Example
//! ```ignore
//! let solver = TerrainSolver::new(&chunk_manager, camera_pos, camera_dir, 256.0, world_seed);
//! let resolved = solver.resolve_location(&terrain_request)?;
//! if resolved.is_valid() {
//!     // Proceed with terrain modification at resolved.pos
//! }
//! ```

use crate::{BiomeType, ChunkId, ChunkManager};
use astraweave_core::schema::{
    CardinalDirection, DistanceCategory, RelativeLocation, TerrainGenerationRequest,
};
use glam::Vec3;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

/// Result of resolving a terrain generation request to world coordinates
#[derive(Debug, Clone)]
pub struct ResolvedLocation {
    /// The resolved world position
    pub pos: Vec3,
    /// Deterministic seed for procedural generation
    pub seed: u64,
    /// The biome at the resolved location
    pub biome: BiomeType,
    /// Validation status indicating if the location is usable
    pub validation_result: ValidationStatus,
}

impl ResolvedLocation {
    /// Check if the resolved location is valid for terrain modification
    pub fn is_valid(&self) -> bool {
        matches!(self.validation_result, ValidationStatus::Valid)
    }
}

/// Validation status for a resolved terrain location
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ValidationStatus {
    /// Location is valid and can be used for terrain modification
    Valid,
    /// Location is in an incompatible biome (provides the actual biome found)
    BiomeIncompatible(BiomeType),
    /// Location is outside the valid world bounds or loaded chunks
    OutOfBounds,
    /// No solid ground was found at the location (e.g., deep water, void)
    NoSolidGround,
    /// The chunk at this location is not loaded
    ChunkNotLoaded,
}

/// Error types that can occur during terrain solving
#[derive(Debug, Clone, thiserror::Error)]
#[non_exhaustive]
#[must_use]
pub enum SolverError {
    #[error("No chunk loaded at position {0:?}")]
    ChunkNotLoaded(Vec3),
    #[error("Raycast failed to hit terrain within {0} units")]
    RaycastFailed(f32),
    #[error("Invalid relative location configuration")]
    InvalidConfiguration,
}

/// Resolves abstract LLM terrain requests into validated world coordinates
pub struct TerrainSolver<'a> {
    chunk_manager: &'a ChunkManager,
    camera_pos: Vec3,
    camera_dir: Vec3,
    chunk_size: f32,
    world_time_seed: u64,
}

impl<'a> TerrainSolver<'a> {
    /// Create a new TerrainSolver
    ///
    /// # Arguments
    /// * `chunk_manager` - Reference to the chunk manager for terrain queries
    /// * `camera_pos` - Current camera/player position
    /// * `camera_dir` - Current camera/player look direction (normalized)
    /// * `chunk_size` - Size of terrain chunks in world units
    /// * `seed` - Explicit world seed. There is no wall-clock default.
    pub fn new(
        chunk_manager: &'a ChunkManager,
        camera_pos: Vec3,
        camera_dir: Vec3,
        chunk_size: f32,
        seed: u64,
    ) -> Self {
        Self {
            chunk_manager,
            camera_pos,
            camera_dir: camera_dir.normalize_or_zero(),
            chunk_size,
            world_time_seed: seed,
        }
    }

    /// Set a custom world time seed for deterministic operations
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.world_time_seed = seed;
        self
    }

    /// Resolve a terrain generation request to world coordinates
    ///
    /// This is the main entry point for converting LLM terrain requests
    /// into validated world positions.
    pub fn resolve_location(
        &self,
        request: &TerrainGenerationRequest,
    ) -> Result<ResolvedLocation, SolverError> {
        // Calculate raw position based on relative location type
        let raw_pos = match &request.relative_location {
            RelativeLocation::LineOfSight { look_distance } => self
                .raycast_terrain(*look_distance)
                .unwrap_or_else(|| self.fallback_line_of_sight(*look_distance)),
            RelativeLocation::DirectionFrom { cardinal, distance } => {
                self.calculate_direction_offset(*cardinal, *distance)
            }
            RelativeLocation::Coordinates { x, y, z } => Vec3::new(*x, *y, *z),
            // Fallback for future location types
            _ => Vec3::ZERO,
        };

        // Derive seed for deterministic operations
        let seed = self.derive_seed(request, raw_pos);

        // Check if chunk is loaded
        let chunk_id = ChunkId::from_world_pos(raw_pos, self.chunk_size);
        if !self.chunk_manager.has_chunk(chunk_id) {
            return Ok(ResolvedLocation {
                pos: raw_pos,
                seed,
                biome: BiomeType::Grassland, // Default fallback
                validation_result: ValidationStatus::ChunkNotLoaded,
            });
        }

        // Get biome at the resolved position
        let biome = self
            .chunk_manager
            .get_biome_at_world_pos(raw_pos)
            .unwrap_or(BiomeType::Grassland);

        // Validate biome constraints
        if !request.biome_constraints.is_empty() {
            let biome_str = biome.as_str();
            let biome_compatible = request
                .biome_constraints
                .iter()
                .any(|b| b.to_lowercase() == biome_str);

            if !biome_compatible {
                return Ok(ResolvedLocation {
                    pos: raw_pos,
                    seed,
                    biome,
                    validation_result: ValidationStatus::BiomeIncompatible(biome),
                });
            }
        }

        // Check for solid ground
        if self
            .chunk_manager
            .get_height_at_world_pos(raw_pos)
            .is_none()
        {
            return Ok(ResolvedLocation {
                pos: raw_pos,
                seed,
                biome,
                validation_result: ValidationStatus::NoSolidGround,
            });
        }

        // Get actual height and snap position to terrain
        let terrain_height = self
            .chunk_manager
            .get_height_at_world_pos(raw_pos)
            .unwrap_or(0.0);
        let snapped_pos = Vec3::new(raw_pos.x, terrain_height, raw_pos.z);

        Ok(ResolvedLocation {
            pos: snapped_pos,
            seed,
            biome,
            validation_result: ValidationStatus::Valid,
        })
    }

    /// Perform a raycast from the camera position along the look direction
    ///
    /// Returns the terrain hit position, or None if no terrain is hit.
    fn raycast_terrain(&self, max_distance: f32) -> Option<Vec3> {
        const STEP_SIZE: f32 = 2.0;
        let mut distance = 0.0;

        while distance < max_distance {
            let test_pos = self.camera_pos + self.camera_dir * distance;

            // Check if we're below terrain height at this position
            if let Some(terrain_height) = self.chunk_manager.get_height_at_world_pos(test_pos) {
                if test_pos.y <= terrain_height {
                    // Hit terrain - return position at terrain surface
                    return Some(Vec3::new(test_pos.x, terrain_height, test_pos.z));
                }
            }

            distance += STEP_SIZE;
        }

        None
    }

    /// Fallback position if raycast doesn't hit terrain
    ///
    /// Returns a position at ground level in the look direction
    fn fallback_line_of_sight(&self, look_distance: f32) -> Vec3 {
        // Calculate horizontal position
        let horizontal_dir =
            Vec3::new(self.camera_dir.x, 0.0, self.camera_dir.z).normalize_or_zero();
        let horizontal_pos = self.camera_pos + horizontal_dir * look_distance.min(100.0);

        // Try to get terrain height at this position
        let y = self
            .chunk_manager
            .get_height_at_world_pos(horizontal_pos)
            .unwrap_or(self.camera_pos.y);

        Vec3::new(horizontal_pos.x, y, horizontal_pos.z)
    }

    /// Calculate position offset based on cardinal direction and distance category
    fn calculate_direction_offset(
        &self,
        cardinal: CardinalDirection,
        distance: DistanceCategory,
    ) -> Vec3 {
        let (dir_x, dir_z) = cardinal.to_unit_vector();
        let direction = Vec3::new(dir_x, 0.0, dir_z);

        // Get actual distance from category, with some randomization
        let (min_dist, max_dist) = distance.to_range();
        let mut rng = StdRng::seed_from_u64(self.world_time_seed);
        let actual_distance = rng.random_range(min_dist..max_dist);

        let target_pos = self.camera_pos + direction * actual_distance;

        // Snap to terrain height if possible
        let y = self
            .chunk_manager
            .get_height_at_world_pos(target_pos)
            .unwrap_or(self.camera_pos.y);

        Vec3::new(target_pos.x, y, target_pos.z)
    }

    /// Derive a deterministic seed for this request
    ///
    /// Uses the request's explicit seed if provided, otherwise derives from:
    /// - World time seed
    /// - Request ID hash
    /// - Position hash
    fn derive_seed(&self, request: &TerrainGenerationRequest, pos: Vec3) -> u64 {
        if let Some(explicit_seed) = request.seed {
            return explicit_seed;
        }

        // Combine world time, request ID, and position for unique but deterministic seed
        let mut seed = self.world_time_seed;
        for byte in request.request_id.bytes() {
            seed = seed.wrapping_mul(31).wrapping_add(byte as u64);
        }
        seed = seed.wrapping_add((pos.x as i32) as u64);
        seed = seed.wrapping_add(((pos.z as i32) as u64) << 16);

        seed
    }

    /// Get the chunk manager reference
    pub fn chunk_manager(&self) -> &ChunkManager {
        self.chunk_manager
    }

    /// Get the current camera position
    pub fn camera_pos(&self) -> Vec3 {
        self.camera_pos
    }

    /// Get the chunk size
    pub fn chunk_size(&self) -> f32 {
        self.chunk_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Heightmap, HeightmapConfig, TerrainChunk};
    use astraweave_core::schema::{PersistenceMode, TerrainFeatureType};

    fn create_test_chunk_manager() -> ChunkManager {
        let mut manager = ChunkManager::new(256.0, 64);

        // Add a test chunk at (0, 0)
        let heightmap = Heightmap::new(HeightmapConfig::default()).unwrap();
        let resolution = heightmap.resolution() as usize;
        let biome_map = vec![BiomeType::Grassland; resolution * resolution];
        let chunk = TerrainChunk::new(ChunkId::new(0, 0), heightmap, biome_map);
        manager.add_chunk(chunk);

        manager
    }

    fn create_test_request(relative_location: RelativeLocation) -> TerrainGenerationRequest {
        TerrainGenerationRequest {
            request_id: "test-001".to_string(),
            feature_type: TerrainFeatureType::Crater { radius: 10 },
            relative_location,
            intensity: 0.5,
            narrative_reason: "Test terrain modification".to_string(),
            persistence_mode: PersistenceMode::SessionOnly,
            biome_constraints: vec![],
            seed: Some(12345),
        }
    }

    #[test]
    fn test_solver_creation() {
        let manager = create_test_chunk_manager();
        let camera_pos = Vec3::new(128.0, 50.0, 128.0);
        let camera_dir = Vec3::new(1.0, 0.0, 0.0);

        let solver = TerrainSolver::new(&manager, camera_pos, camera_dir, 256.0, 1);

        assert_eq!(solver.camera_pos(), camera_pos);
        assert_eq!(solver.chunk_size(), 256.0);
    }

    #[test]
    fn test_resolve_coordinates() {
        let manager = create_test_chunk_manager();
        let camera_pos = Vec3::new(128.0, 50.0, 128.0);
        let camera_dir = Vec3::new(1.0, 0.0, 0.0);
        let solver = TerrainSolver::new(&manager, camera_pos, camera_dir, 256.0, 1);

        let request = TerrainGenerationRequest {
            request_id: "test-001".to_string(),
            feature_type: TerrainFeatureType::Crater { radius: 10 },
            relative_location: RelativeLocation::Coordinates {
                x: 100.0,
                y: 30.0,
                z: 100.0,
            },
            intensity: 0.5,
            narrative_reason: "Test".to_string(),
            persistence_mode: PersistenceMode::SessionOnly,
            biome_constraints: vec![],
            seed: Some(42),
        };

        let result = solver.resolve_location(&request).unwrap();
        assert_eq!(result.pos.x, 100.0);
        assert_eq!(result.pos.z, 100.0);
        assert_eq!(result.seed, 42);
        assert!(result.is_valid());
    }

    #[test]
    fn test_resolve_direction_from() {
        let manager = create_test_chunk_manager();
        let camera_pos = Vec3::new(128.0, 50.0, 128.0);
        let camera_dir = Vec3::new(1.0, 0.0, 0.0);
        let solver =
            TerrainSolver::new(&manager, camera_pos, camera_dir, 256.0, 1).with_seed(12345);

        let request = create_test_request(RelativeLocation::DirectionFrom {
            cardinal: CardinalDirection::North,
            distance: DistanceCategory::Near,
        });

        let result = solver.resolve_location(&request).unwrap();

        // North is -Z direction
        assert!(
            result.pos.z < camera_pos.z,
            "Position should be north (negative Z)"
        );
    }

    #[test]
    fn test_biome_constraint_compatible() {
        let manager = create_test_chunk_manager();
        let camera_pos = Vec3::new(128.0, 50.0, 128.0);
        let camera_dir = Vec3::new(1.0, 0.0, 0.0);
        let solver = TerrainSolver::new(&manager, camera_pos, camera_dir, 256.0, 1);

        let request = TerrainGenerationRequest {
            request_id: "test-001".to_string(),
            feature_type: TerrainFeatureType::Forest { density: 0.5 },
            relative_location: RelativeLocation::Coordinates {
                x: 100.0,
                y: 30.0,
                z: 100.0,
            },
            intensity: 0.5,
            narrative_reason: "Test".to_string(),
            persistence_mode: PersistenceMode::SessionOnly,
            biome_constraints: vec!["grassland".to_string()], // Our test chunk is Grassland
            seed: None,
        };

        let result = solver.resolve_location(&request).unwrap();
        assert!(result.is_valid());
        assert_eq!(result.biome, BiomeType::Grassland);
    }

    #[test]
    fn test_biome_constraint_incompatible() {
        let manager = create_test_chunk_manager();
        let camera_pos = Vec3::new(128.0, 50.0, 128.0);
        let camera_dir = Vec3::new(1.0, 0.0, 0.0);
        let solver = TerrainSolver::new(&manager, camera_pos, camera_dir, 256.0, 1);

        let request = TerrainGenerationRequest {
            request_id: "test-001".to_string(),
            feature_type: TerrainFeatureType::Lake { depth: 10 },
            relative_location: RelativeLocation::Coordinates {
                x: 100.0,
                y: 30.0,
                z: 100.0,
            },
            intensity: 0.5,
            narrative_reason: "Test".to_string(),
            persistence_mode: PersistenceMode::SessionOnly,
            biome_constraints: vec!["desert".to_string(), "tundra".to_string()], // Not Grassland
            seed: None,
        };

        let result = solver.resolve_location(&request).unwrap();
        assert!(!result.is_valid());
        assert!(matches!(
            result.validation_result,
            ValidationStatus::BiomeIncompatible(BiomeType::Grassland)
        ));
    }

    #[test]
    fn test_chunk_not_loaded() {
        let manager = create_test_chunk_manager();
        let camera_pos = Vec3::new(128.0, 50.0, 128.0);
        let camera_dir = Vec3::new(1.0, 0.0, 0.0);
        let solver = TerrainSolver::new(&manager, camera_pos, camera_dir, 256.0, 1);

        let request = TerrainGenerationRequest {
            request_id: "test-001".to_string(),
            feature_type: TerrainFeatureType::Crater { radius: 10 },
            relative_location: RelativeLocation::Coordinates {
                x: 5000.0, // Far outside loaded chunks
                y: 30.0,
                z: 5000.0,
            },
            intensity: 0.5,
            narrative_reason: "Test".to_string(),
            persistence_mode: PersistenceMode::SessionOnly,
            biome_constraints: vec![],
            seed: None,
        };

        let result = solver.resolve_location(&request).unwrap();
        assert!(!result.is_valid());
        assert_eq!(result.validation_result, ValidationStatus::ChunkNotLoaded);
    }

    #[test]
    fn test_seed_derivation_explicit() {
        let manager = create_test_chunk_manager();
        let camera_pos = Vec3::new(128.0, 50.0, 128.0);
        let camera_dir = Vec3::new(1.0, 0.0, 0.0);
        let solver = TerrainSolver::new(&manager, camera_pos, camera_dir, 256.0, 1);

        let request = TerrainGenerationRequest {
            request_id: "test-001".to_string(),
            feature_type: TerrainFeatureType::Crater { radius: 10 },
            relative_location: RelativeLocation::Coordinates {
                x: 100.0,
                y: 30.0,
                z: 100.0,
            },
            intensity: 0.5,
            narrative_reason: "Test".to_string(),
            persistence_mode: PersistenceMode::SessionOnly,
            biome_constraints: vec![],
            seed: Some(99999),
        };

        let result = solver.resolve_location(&request).unwrap();
        assert_eq!(result.seed, 99999);
    }

    #[test]
    fn test_seed_derivation_deterministic() {
        let manager = create_test_chunk_manager();
        let camera_pos = Vec3::new(128.0, 50.0, 128.0);
        let camera_dir = Vec3::new(1.0, 0.0, 0.0);

        let solver1 =
            TerrainSolver::new(&manager, camera_pos, camera_dir, 256.0, 1).with_seed(12345);
        let solver2 =
            TerrainSolver::new(&manager, camera_pos, camera_dir, 256.0, 1).with_seed(12345);

        let request = TerrainGenerationRequest {
            request_id: "test-001".to_string(),
            feature_type: TerrainFeatureType::Crater { radius: 10 },
            relative_location: RelativeLocation::Coordinates {
                x: 100.0,
                y: 30.0,
                z: 100.0,
            },
            intensity: 0.5,
            narrative_reason: "Test".to_string(),
            persistence_mode: PersistenceMode::SessionOnly,
            biome_constraints: vec![],
            seed: None, // No explicit seed
        };

        let result1 = solver1.resolve_location(&request).unwrap();
        let result2 = solver2.resolve_location(&request).unwrap();

        assert_eq!(result1.seed, result2.seed, "Seeds should be deterministic");
    }

    #[test]
    fn test_cardinal_direction_offsets() {
        let manager = create_test_chunk_manager();
        let camera_pos = Vec3::new(128.0, 50.0, 128.0);
        let camera_dir = Vec3::new(1.0, 0.0, 0.0);
        let solver = TerrainSolver::new(&manager, camera_pos, camera_dir, 256.0, 1).with_seed(42);

        // Test North (should move in -Z direction)
        let request_north = create_test_request(RelativeLocation::DirectionFrom {
            cardinal: CardinalDirection::North,
            distance: DistanceCategory::Near,
        });
        let result_north = solver.resolve_location(&request_north).unwrap();
        assert!(result_north.pos.z < camera_pos.z, "North should be -Z");

        // Test South (should move in +Z direction)
        let request_south = create_test_request(RelativeLocation::DirectionFrom {
            cardinal: CardinalDirection::South,
            distance: DistanceCategory::Near,
        });
        let result_south = solver.resolve_location(&request_south).unwrap();
        assert!(result_south.pos.z > camera_pos.z, "South should be +Z");

        // Test East (should move in +X direction)
        let request_east = create_test_request(RelativeLocation::DirectionFrom {
            cardinal: CardinalDirection::East,
            distance: DistanceCategory::Near,
        });
        let result_east = solver.resolve_location(&request_east).unwrap();
        assert!(result_east.pos.x > camera_pos.x, "East should be +X");

        // Test West (should move in -X direction)
        let request_west = create_test_request(RelativeLocation::DirectionFrom {
            cardinal: CardinalDirection::West,
            distance: DistanceCategory::Near,
        });
        let result_west = solver.resolve_location(&request_west).unwrap();
        assert!(result_west.pos.x < camera_pos.x, "West should be -X");
    }

    #[test]
    fn test_line_of_sight_fallback() {
        let manager = create_test_chunk_manager();
        let camera_pos = Vec3::new(128.0, 100.0, 128.0); // High above terrain
        let camera_dir = Vec3::new(1.0, 0.0, 0.0); // Looking horizontally
        let solver = TerrainSolver::new(&manager, camera_pos, camera_dir, 256.0, 1);

        let request = create_test_request(RelativeLocation::LineOfSight {
            look_distance: 50.0,
        });

        let result = solver.resolve_location(&request).unwrap();

        // Should use fallback and place on terrain
        assert!(
            result.pos.x > camera_pos.x,
            "Position should be in look direction"
        );
    }

    #[test]
    fn test_validation_status_equality() {
        assert_eq!(ValidationStatus::Valid, ValidationStatus::Valid);
        assert_eq!(
            ValidationStatus::BiomeIncompatible(BiomeType::Desert),
            ValidationStatus::BiomeIncompatible(BiomeType::Desert)
        );
        assert_ne!(
            ValidationStatus::BiomeIncompatible(BiomeType::Desert),
            ValidationStatus::BiomeIncompatible(BiomeType::Forest)
        );
    }
}
