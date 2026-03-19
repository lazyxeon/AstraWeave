use anyhow::{Context, Result};
use astraweave_behavior::BehaviorGraph;
use astraweave_core::{Entity, IVec2, World};
use astraweave_security::path::{safe_under, validate_extension};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tracing::debug;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EntityData {
    pub id: Entity,
    pub name: String,
    pub pos: IVec2,
    pub rotation: f32,
    pub rotation_x: f32,
    pub rotation_z: f32,
    pub scale: f32,
    pub hp: i32,
    pub team_id: u8,
    pub ammo: i32,
    pub cooldowns: HashMap<String, f32>,
    #[serde(default)]
    pub behavior_graph: Option<BehaviorGraph>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SceneData {
    pub version: u32,
    pub time: f32,
    pub next_entity_id: Entity,
    pub entities: Vec<EntityData>,
    pub obstacles: Vec<(i32, i32)>,
}

impl SceneData {
    pub fn from_world(world: &World) -> Self {
        let mut entities = Vec::new();

        for entity_id in world.entities() {
            let pose = world.pose(entity_id).unwrap_or(astraweave_core::Pose {
                pos: IVec2 { x: 0, y: 0 },
                height: 0.0,
                rotation: 0.0,
                rotation_x: 0.0,
                rotation_z: 0.0,
                scale: 1.0,
            });

            let health = world.health(entity_id).map(|h| h.hp).unwrap_or(100);
            let team = world.team(entity_id).map(|t| t.id).unwrap_or(0);
            let ammo_rounds = world.ammo(entity_id).map(|a| a.rounds).unwrap_or(0);
            let name = world.name(entity_id).unwrap_or("Unnamed").to_string();

            let cooldowns = world
                .cooldowns(entity_id)
                .map(|cd| cd.map.clone())
                .unwrap_or_default();

            let behavior_graph = world.behavior_graph(entity_id).cloned();

            entities.push(EntityData {
                id: entity_id,
                name,
                pos: pose.pos,
                rotation: pose.rotation,
                rotation_x: pose.rotation_x,
                rotation_z: pose.rotation_z,
                scale: pose.scale,
                hp: health,
                team_id: team,
                ammo: ammo_rounds,
                cooldowns,
                behavior_graph,
            });
        }

        let obstacles: Vec<(i32, i32)> = world.obstacles.iter().copied().collect();

        SceneData {
            version: 1,
            time: world.t,
            next_entity_id: world.next_id,
            entities,
            obstacles,
        }
    }

    pub fn to_world(&self) -> World {
        let mut world = World::new();
        world.t = self.time;
        world.next_id = self.next_entity_id;

        world.obstacles = self.obstacles.iter().copied().collect();

        for entity_data in &self.entities {
            let id = world.spawn_with_id(
                entity_data.id,
                &entity_data.name,
                entity_data.pos,
                astraweave_core::Team {
                    id: entity_data.team_id,
                },
                entity_data.hp,
                entity_data.ammo,
            );

            if let Some(pose) = world.pose_mut(id) {
                pose.rotation = entity_data.rotation;
                pose.rotation_x = entity_data.rotation_x;
                pose.rotation_z = entity_data.rotation_z;
                pose.scale = entity_data.scale;
            }

            if let Some(cooldowns) = world.cooldowns_mut(id) {
                cooldowns.map = entity_data.cooldowns.clone();
            }

            if let Some(graph) = &entity_data.behavior_graph {
                world.set_behavior_graph(id, graph.clone());
            }
        }

        world
    }

    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        // Security: Validate path is within content directory and has correct extension
        let base = std::env::current_dir().context("Failed to get current directory")?;
        let content_base = base.join("content");

        // Create content directory if it doesn't exist
        fs::create_dir_all(&content_base).context("Failed to create content directory")?;

        // Validate path is within content directory
        let safe_path = safe_under(&content_base, path)
            .map_err(|e| anyhow::anyhow!("Invalid scene path: {}", e))?;

        // Validate extension
        validate_extension(&safe_path, &["ron", "json", "toml"])
            .map_err(|e| anyhow::anyhow!("Invalid scene file extension: {}", e))?;

        let ron_string = ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default())
            .context("Failed to serialize scene to RON")?;

        if let Some(parent) = safe_path.parent() {
            fs::create_dir_all(parent)
                .context(format!("Failed to create directory: {:?}", parent))?;
        }

        fs::write(&safe_path, ron_string)
            .context(format!("Failed to write scene to {:?}", safe_path))?;

        debug!("Saved scene to {:?}", safe_path);
        Ok(())
    }

    pub fn load_from_file(path: &Path) -> Result<Self> {
        // Security: Validate path is within content directory and has correct extension
        let base = std::env::current_dir().context("Failed to get current directory")?;
        let content_base = base.join("content");

        // Validate path is within content directory
        let safe_path = safe_under(&content_base, path)
            .map_err(|e| anyhow::anyhow!("Invalid scene path: {}", e))?;

        // Validate extension
        validate_extension(&safe_path, &["ron", "json", "toml"])
            .map_err(|e| anyhow::anyhow!("Invalid scene file extension: {}", e))?;

        let contents = fs::read_to_string(&safe_path)
            .context(format!("Failed to read scene from {:?}", safe_path))?;

        let scene: SceneData = ron::from_str(&contents)
            .context(format!("Failed to deserialize scene from {:?}", safe_path))?;

        debug!("Loaded scene from {:?}", safe_path);
        Ok(scene)
    }

    /// Validate scene data integrity and consistency.
    ///
    /// Checks for:
    /// - Duplicate entity IDs
    /// - Invalid/missing component data
    /// - Referential integrity
    ///
    /// Returns a list of validation issues (empty = valid).
    pub fn validate(&self) -> Vec<SceneValidationIssue> {
        let mut issues = Vec::new();

        // Check for duplicate entity IDs
        let mut seen_ids = std::collections::HashSet::new();
        for entity in &self.entities {
            if !seen_ids.insert(entity.id) {
                issues.push(SceneValidationIssue::Error(format!(
                    "Duplicate entity ID: {}",
                    entity.id
                )));
            }
        }

        // Validate entity data
        for (i, entity) in self.entities.iter().enumerate() {
            // Name validation
            if entity.name.is_empty() {
                issues.push(SceneValidationIssue::Warning(format!(
                    "Entity {} (ID {}) has empty name",
                    i, entity.id
                )));
            }

            // Scale validation
            if entity.scale <= 0.0 {
                issues.push(SceneValidationIssue::Error(format!(
                    "Entity '{}' (ID {}) has invalid scale: {}",
                    entity.name, entity.id, entity.scale
                )));
            }

            // HP sanity check
            if entity.hp < 0 {
                issues.push(SceneValidationIssue::Warning(format!(
                    "Entity '{}' (ID {}) has negative HP: {}",
                    entity.name, entity.id, entity.hp
                )));
            }

            // Ammo sanity check
            if entity.ammo < 0 {
                issues.push(SceneValidationIssue::Warning(format!(
                    "Entity '{}' (ID {}) has negative ammo: {}",
                    entity.name, entity.id, entity.ammo
                )));
            }
        }

        // Check next_entity_id consistency
        let max_id = self.entities.iter().map(|e| e.id).max().unwrap_or(0);
        if self.next_entity_id <= max_id {
            issues.push(SceneValidationIssue::Error(format!(
                "next_entity_id ({}) is not greater than max existing ID ({})",
                self.next_entity_id, max_id
            )));
        }

        issues
    }

    /// Check if scene data is valid (no errors).
    pub fn is_valid(&self) -> bool {
        self.validate()
            .iter()
            .all(|issue| !matches!(issue, SceneValidationIssue::Error(_)))
    }

    /// Get statistics about the scene.
    pub fn stats(&self) -> SceneStats {
        SceneStats {
            entity_count: self.entities.len(),
            obstacle_count: self.obstacles.len(),
            has_behavior_graphs: self.entities.iter().any(|e| e.behavior_graph.is_some()),
            total_cooldowns: self.entities.iter().map(|e| e.cooldowns.len()).sum(),
        }
    }
}

/// Scene validation issue types.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum SceneValidationIssue {
    /// Critical issue that must be fixed
    Error(String),
    /// Non-critical issue that should be reviewed
    Warning(String),
}

impl std::fmt::Display for SceneValidationIssue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SceneValidationIssue::Error(msg) => write!(f, "Error: {}", msg),
            SceneValidationIssue::Warning(msg) => write!(f, "Warning: {}", msg),
        }
    }
}

impl SceneValidationIssue {
    /// Returns a list of all variant names.
    pub fn all_variants() -> &'static [&'static str] {
        &["Error", "Warning"]
    }

    /// Returns the display name of this issue type.
    pub fn name(&self) -> &'static str {
        match self {
            SceneValidationIssue::Error(_) => "Error",
            SceneValidationIssue::Warning(_) => "Warning",
        }
    }

    /// Returns an icon for this issue type.
    pub fn icon(&self) -> &'static str {
        match self {
            SceneValidationIssue::Error(_) => "[x]",
            SceneValidationIssue::Warning(_) => "[!]",
        }
    }

    /// Check if this is an error (critical issue).
    pub fn is_error(&self) -> bool {
        matches!(self, SceneValidationIssue::Error(_))
    }

    /// Check if this is a warning.
    pub fn is_warning(&self) -> bool {
        matches!(self, SceneValidationIssue::Warning(_))
    }

    /// Get the message text.
    pub fn message(&self) -> &str {
        match self {
            SceneValidationIssue::Error(msg) | SceneValidationIssue::Warning(msg) => msg,
        }
    }

    /// Create an error issue.
    pub fn error(msg: impl Into<String>) -> Self {
        SceneValidationIssue::Error(msg.into())
    }

    /// Create a warning issue.
    pub fn warning(msg: impl Into<String>) -> Self {
        SceneValidationIssue::Warning(msg.into())
    }
}

/// Scene statistics for quick overview.
#[derive(Debug, Clone)]
pub struct SceneStats {
    /// Number of entities in the scene
    pub entity_count: usize,
    /// Number of obstacles
    pub obstacle_count: usize,
    /// Whether any entity has behavior graphs
    pub has_behavior_graphs: bool,
    /// Total number of cooldown entries across all entities
    pub total_cooldowns: usize,
}

pub fn save_scene(world: &World, path: &Path) -> Result<()> {
    let scene = SceneData::from_world(world);
    scene.save_to_file(path)
}

pub fn load_scene(path: &Path) -> Result<World> {
    let scene = SceneData::load_from_file(path)?;
    Ok(scene.to_world())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scene_roundtrip() {
        let mut world = World::new();
        let e1 = world.spawn(
            "Player",
            IVec2 { x: 5, y: 10 },
            astraweave_core::Team { id: 0 },
            100,
            30,
        );
        let _enemy = world.spawn(
            "Enemy",
            IVec2 { x: 20, y: 15 },
            astraweave_core::Team { id: 2 },
            50,
            15,
        );

        world.obstacles.insert((10, 10));
        world.obstacles.insert((11, 10));

        if let Some(pose) = world.pose_mut(e1) {
            pose.rotation = 1.57;
            pose.scale = 2.0;
        }

        let scene = SceneData::from_world(&world);
        let restored_world = scene.to_world();

        assert_eq!(restored_world.entities().len(), 2);
        assert_eq!(restored_world.obstacles.len(), 2);
        assert!(restored_world.obstacle(IVec2 { x: 10, y: 10 }));
        assert_eq!(restored_world.name(1), Some("Player"));
        assert_eq!(restored_world.name(2), Some("Enemy"));

        let pose = restored_world.pose(1).unwrap();
        assert_eq!(pose.pos, IVec2 { x: 5, y: 10 });
        assert!((pose.rotation - 1.57).abs() < 0.01);
        assert!((pose.scale - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_scene_serialization() {
        let mut world = World::new();
        world.spawn(
            "TestEntity",
            IVec2 { x: 0, y: 0 },
            astraweave_core::Team { id: 0 },
            100,
            30,
        );

        let scene = SceneData::from_world(&world);
        let ron_string =
            ron::ser::to_string_pretty(&scene, ron::ser::PrettyConfig::default()).unwrap();

        assert!(ron_string.contains("version"));
        assert!(ron_string.contains("TestEntity"));
        assert!(ron_string.contains("entities"));

        let deserialized: SceneData = ron::from_str(&ron_string).unwrap();
        assert_eq!(deserialized.version, 1);
        assert_eq!(deserialized.entities.len(), 1);
        assert_eq!(deserialized.entities[0].name, "TestEntity");
    }

    #[test]
    fn test_empty_scene() {
        let world = World::new();
        let scene = SceneData::from_world(&world);

        assert_eq!(scene.entities.len(), 0);
        assert_eq!(scene.obstacles.len(), 0);
        assert_eq!(scene.version, 1);

        let restored = scene.to_world();
        assert_eq!(restored.entities().len(), 0);
    }

    #[test]
    fn test_scene_with_multiple_entities() {
        let mut world = World::new();

        for i in 0..10 {
            world.spawn(
                &format!("Entity{}", i),
                IVec2 { x: i, y: i * 2 },
                astraweave_core::Team { id: (i % 3) as u8 },
                100 - i,
                30 + i,
            );
        }

        let scene = SceneData::from_world(&world);
        assert_eq!(scene.entities.len(), 10);

        let restored = scene.to_world();
        let restored_entities = restored.entities();
        assert_eq!(restored_entities.len(), 10);

        for i in 0..10 {
            let entity_name = format!("Entity{}", i);
            let found = restored_entities
                .iter()
                .copied()
                .find(|&id| restored.name(id) == Some(entity_name.as_str()));
            assert!(found.is_some(), "Entity {} should exist", i);
        }
    }

    #[test]
    fn test_scene_with_all_components() {
        use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, PI};
        let mut world = World::new();
        let entity = world.spawn(
            "CompleteEntity",
            IVec2 { x: 5, y: 10 },
            astraweave_core::Team { id: 1 },
            75,
            25,
        );

        if let Some(pose) = world.pose_mut(entity) {
            pose.rotation = PI;
            pose.rotation_x = FRAC_PI_2;
            pose.rotation_z = FRAC_PI_4;
            pose.scale = 1.5;
        }

        let scene = SceneData::from_world(&world);
        let restored = scene.to_world();

        let entity_data = &scene.entities[0];
        assert_eq!(entity_data.name, "CompleteEntity");
        assert_eq!(entity_data.pos.x, 5);
        assert_eq!(entity_data.pos.y, 10);
        assert_eq!(entity_data.team_id, 1);
        assert_eq!(entity_data.hp, 75);
        assert_eq!(entity_data.ammo, 25);
        assert!((entity_data.rotation - PI).abs() < 0.01);
        assert!((entity_data.rotation_x - FRAC_PI_2).abs() < 0.01);
        assert!((entity_data.rotation_z - FRAC_PI_4).abs() < 0.01);
        assert!((entity_data.scale - 1.5).abs() < 0.01);

        let restored_pose = restored.pose(entity).unwrap();
        assert!((restored_pose.rotation - PI).abs() < 0.01);
        assert!((restored_pose.scale - 1.5).abs() < 0.01);
    }

    #[test]
    fn test_scene_with_obstacles() {
        let mut world = World::new();

        for x in 0..5 {
            for y in 0..5 {
                world.obstacles.insert((x, y));
            }
        }

        let scene = SceneData::from_world(&world);
        assert_eq!(scene.obstacles.len(), 25);

        let restored = scene.to_world();
        assert_eq!(restored.obstacles.len(), 25);

        for x in 0..5 {
            for y in 0..5 {
                assert!(restored.obstacle(IVec2 { x, y }));
            }
        }
    }

    #[test]
    fn test_scene_file_io() {
        use std::env;
        let mut world = World::new();
        world.spawn(
            "Player",
            IVec2 { x: 10, y: 20 },
            astraweave_core::Team { id: 0 },
            100,
            30,
        );
        world.obstacles.insert((5, 5));

        // Use relative path within content directory for security validation
        let test_path = Path::new("test_scenes/aw_editor_test_scene.ron");

        let scene = SceneData::from_world(&world);
        scene.save_to_file(test_path).unwrap();

        // Construct full path for cleanup
        let base = env::current_dir().unwrap();
        let full_path = base.join("content").join(test_path);
        assert!(full_path.exists());

        let loaded_scene = SceneData::load_from_file(test_path).unwrap();
        assert_eq!(loaded_scene.entities.len(), 1);
        assert_eq!(loaded_scene.obstacles.len(), 1);
        assert_eq!(loaded_scene.entities[0].name, "Player");

        fs::remove_file(&full_path).unwrap();
    }

    #[test]
    fn test_save_and_load_scene() {
        use std::env;
        let mut world = World::new();
        world.spawn(
            "Entity1",
            IVec2 { x: 1, y: 2 },
            astraweave_core::Team { id: 0 },
            100,
            30,
        );
        world.spawn(
            "Entity2",
            IVec2 { x: 3, y: 4 },
            astraweave_core::Team { id: 1 },
            50,
            15,
        );

        // Use relative path within content directory for security validation
        let test_path = Path::new("test_scenes/aw_editor_test_save_load.ron");

        save_scene(&world, test_path).unwrap();

        // Construct full path for assertions
        let base = env::current_dir().unwrap();
        let full_path = base.join("content").join(test_path);
        assert!(full_path.exists());

        let loaded_world = load_scene(test_path).unwrap();
        assert_eq!(loaded_world.entities().len(), 2);

        fs::remove_file(&full_path).unwrap();
    }

    #[test]
    fn test_scene_preserves_world_time() {
        let mut world = World::new();
        world.t = 123.45;

        let scene = SceneData::from_world(&world);
        assert!((scene.time - 123.45).abs() < 0.01);

        let restored = scene.to_world();
        assert!((restored.t - 123.45).abs() < 0.01);
    }

    #[test]
    fn test_scene_preserves_entity_ids() {
        let mut world = World::new();
        let e1 = world.spawn(
            "E1",
            IVec2 { x: 0, y: 0 },
            astraweave_core::Team { id: 0 },
            100,
            30,
        );
        let e2 = world.spawn(
            "E2",
            IVec2 { x: 1, y: 1 },
            astraweave_core::Team { id: 1 },
            50,
            15,
        );

        let scene = SceneData::from_world(&world);
        let restored = scene.to_world();

        assert!(restored.pose(e1).is_some());
        assert!(restored.pose(e2).is_some());
        assert_eq!(restored.name(e1), Some("E1"));
        assert_eq!(restored.name(e2), Some("E2"));
    }

    #[test]
    fn test_scene_preserves_behavior_graphs() {
        use astraweave_behavior::{BehaviorGraph, BehaviorNode};

        let mut world = World::new();
        let entity = world.spawn(
            "Agent",
            IVec2 { x: 0, y: 0 },
            astraweave_core::Team { id: 0 },
            100,
            30,
        );
        let graph = BehaviorGraph::new(BehaviorNode::Sequence(vec![
            BehaviorNode::Condition("has_target".into()),
            BehaviorNode::Action("attack".into()),
        ]));
        world.set_behavior_graph(entity, graph.clone());

        let scene = SceneData::from_world(&world);
        assert_eq!(scene.entities.len(), 1);
        assert!(scene.entities[0].behavior_graph.is_some());

        let restored = scene.to_world();
        let restored_graph = restored.behavior_graph(entity).expect("graph restored");
        match &restored_graph.root {
            BehaviorNode::Sequence(children) => {
                assert_eq!(children.len(), 2);
            }
            other => panic!("unexpected node: {:?}", other),
        }
    }

    #[test]
    fn test_scene_serialization_cooldowns() {
        let mut world = World::new();
        let entity = world.spawn(
            "CooldownEntity",
            IVec2 { x: 0, y: 0 },
            astraweave_core::Team { id: 0 },
            100,
            30,
        );

        if let Some(cd) = world.cooldowns_mut(entity) {
            cd.map.insert("fire".to_string(), 1.5);
            cd.map.insert("jump".to_string(), 0.5);
        }

        let scene = SceneData::from_world(&world);
        let restored = scene.to_world();

        // Access restricted in world, but we can verify SceneData
        assert_eq!(scene.entities[0].cooldowns.len(), 2);
        assert!((scene.entities[0].cooldowns["fire"] - 1.5).abs() < 0.001);

        // If we want to check restored world, we need public accessors or helper
        // Assuming implementation is correct if SceneData is correct, as to_world copies it back
        let restored_cd = restored.cooldowns(entity).unwrap();
        assert!((restored_cd.map["fire"] - 1.5).abs() < 0.001);
    }

    #[test]
    fn test_scene_serialization_large_coordinates() {
        let mut world = World::new();
        let entity = world.spawn(
            "FarEntity",
            IVec2 {
                x: 1000000,
                y: -1000000,
            },
            astraweave_core::Team { id: 0 },
            100,
            30,
        );

        let scene = SceneData::from_world(&world);
        let restored = scene.to_world();

        let pose = restored.pose(entity).unwrap();
        assert_eq!(pose.pos.x, 1000000);
        assert_eq!(pose.pos.y, -1000000);
    }

    #[test]
    fn test_scene_serialization_json_format() {
        let mut world = World::new();
        world.spawn(
            "Entity",
            IVec2 { x: 0, y: 0 },
            astraweave_core::Team { id: 0 },
            100,
            30,
        );

        let scene = SceneData::from_world(&world);
        let json = serde_json::to_string(&scene).unwrap();

        let deserialized: SceneData = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.entities.len(), 1);
        assert_eq!(deserialized.entities[0].name, "Entity");
    }

    #[test]
    fn test_scene_security_path_traversal() {
        use std::path::Path;
        let world = World::new();
        let scene = SceneData::from_world(&world);

        // Try to save outside content directory
        // We can't easily mock env::current_dir(), but we can try a relative path that goes up
        // This relies on content directory existing which save_to_file ensures
        let result = scene.save_to_file(Path::new("../outside.ron"));
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(err.to_string().contains("Invalid scene path"));
    }

    // ====================================================================
    // SceneValidationIssue Tests
    // ====================================================================

    #[test]
    fn test_scene_validation_issue_error() {
        let issue = SceneValidationIssue::Error("Test error".to_string());
        assert!(issue.is_error());
        assert_eq!(issue.message(), "Test error");
    }

    #[test]
    fn test_scene_validation_issue_warning() {
        let issue = SceneValidationIssue::Warning("Test warning".to_string());
        assert!(!issue.is_error());
        assert_eq!(issue.message(), "Test warning");
    }

    // ====================================================================
    // SceneData Validation Tests
    // ====================================================================

    #[test]
    fn test_scene_validate_empty_valid() {
        let scene = SceneData {
            version: 1,
            time: 0.0,
            next_entity_id: 1,
            entities: vec![],
            obstacles: vec![],
        };

        let issues = scene.validate();
        assert!(issues.is_empty());
        assert!(scene.is_valid());
    }

    #[test]
    fn test_scene_validate_duplicate_ids() {
        let scene = SceneData {
            version: 1,
            time: 0.0,
            next_entity_id: 5,
            entities: vec![
                EntityData {
                    id: 1,
                    name: "E1".to_string(),
                    pos: IVec2 { x: 0, y: 0 },
                    rotation: 0.0,
                    rotation_x: 0.0,
                    rotation_z: 0.0,
                    scale: 1.0,
                    hp: 100,
                    team_id: 0,
                    ammo: 30,
                    cooldowns: std::collections::HashMap::new(),
                    behavior_graph: None,
                },
                EntityData {
                    id: 1, // Duplicate!
                    name: "E2".to_string(),
                    pos: IVec2 { x: 5, y: 5 },
                    rotation: 0.0,
                    rotation_x: 0.0,
                    rotation_z: 0.0,
                    scale: 1.0,
                    hp: 50,
                    team_id: 1,
                    ammo: 15,
                    cooldowns: std::collections::HashMap::new(),
                    behavior_graph: None,
                },
            ],
            obstacles: vec![],
        };

        let issues = scene.validate();
        assert!(!issues.is_empty());
        assert!(issues
            .iter()
            .any(|i| i.is_error() && i.message().contains("Duplicate")));
        assert!(!scene.is_valid());
    }

    #[test]
    fn test_scene_validate_invalid_scale() {
        let scene = SceneData {
            version: 1,
            time: 0.0,
            next_entity_id: 5,
            entities: vec![EntityData {
                id: 1,
                name: "BadScale".to_string(),
                pos: IVec2 { x: 0, y: 0 },
                rotation: 0.0,
                rotation_x: 0.0,
                rotation_z: 0.0,
                scale: 0.0, // Invalid!
                hp: 100,
                team_id: 0,
                ammo: 30,
                cooldowns: std::collections::HashMap::new(),
                behavior_graph: None,
            }],
            obstacles: vec![],
        };

        let issues = scene.validate();
        assert!(issues
            .iter()
            .any(|i| i.is_error() && i.message().contains("scale")));
        assert!(!scene.is_valid());
    }

    #[test]
    fn test_scene_validate_negative_hp_warning() {
        let scene = SceneData {
            version: 1,
            time: 0.0,
            next_entity_id: 5,
            entities: vec![EntityData {
                id: 1,
                name: "NegativeHP".to_string(),
                pos: IVec2 { x: 0, y: 0 },
                rotation: 0.0,
                rotation_x: 0.0,
                rotation_z: 0.0,
                scale: 1.0,
                hp: -50, // Warning
                team_id: 0,
                ammo: 30,
                cooldowns: std::collections::HashMap::new(),
                behavior_graph: None,
            }],
            obstacles: vec![],
        };

        let issues = scene.validate();
        assert!(issues
            .iter()
            .any(|i| !i.is_error() && i.message().contains("HP")));
        assert!(scene.is_valid()); // Warnings don't invalidate
    }

    #[test]
    fn test_scene_validate_next_entity_id_consistency() {
        let scene = SceneData {
            version: 1,
            time: 0.0,
            next_entity_id: 5, // Less than max ID!
            entities: vec![EntityData {
                id: 10, // Max ID is 10
                name: "Entity".to_string(),
                pos: IVec2 { x: 0, y: 0 },
                rotation: 0.0,
                rotation_x: 0.0,
                rotation_z: 0.0,
                scale: 1.0,
                hp: 100,
                team_id: 0,
                ammo: 30,
                cooldowns: std::collections::HashMap::new(),
                behavior_graph: None,
            }],
            obstacles: vec![],
        };

        let issues = scene.validate();
        assert!(issues
            .iter()
            .any(|i| i.is_error() && i.message().contains("next_entity_id")));
        assert!(!scene.is_valid());
    }

    // ====================================================================
    // SceneStats Tests
    // ====================================================================

    #[test]
    fn test_scene_stats_empty() {
        let scene = SceneData {
            version: 1,
            time: 0.0,
            next_entity_id: 1,
            entities: vec![],
            obstacles: vec![],
        };

        let stats = scene.stats();
        assert_eq!(stats.entity_count, 0);
        assert_eq!(stats.obstacle_count, 0);
        assert!(!stats.has_behavior_graphs);
        assert_eq!(stats.total_cooldowns, 0);
    }

    #[test]
    fn test_scene_stats_populated() {
        use astraweave_behavior::{BehaviorGraph, BehaviorNode};

        let mut cooldowns = std::collections::HashMap::new();
        cooldowns.insert("fire".to_string(), 1.5);
        cooldowns.insert("jump".to_string(), 0.5);

        let scene = SceneData {
            version: 1,
            time: 100.0,
            next_entity_id: 5,
            entities: vec![
                EntityData {
                    id: 1,
                    name: "E1".to_string(),
                    pos: IVec2 { x: 0, y: 0 },
                    rotation: 0.0,
                    rotation_x: 0.0,
                    rotation_z: 0.0,
                    scale: 1.0,
                    hp: 100,
                    team_id: 0,
                    ammo: 30,
                    cooldowns: cooldowns.clone(),
                    behavior_graph: Some(BehaviorGraph::new(BehaviorNode::Action("test".into()))),
                },
                EntityData {
                    id: 2,
                    name: "E2".to_string(),
                    pos: IVec2 { x: 5, y: 5 },
                    rotation: 0.0,
                    rotation_x: 0.0,
                    rotation_z: 0.0,
                    scale: 1.0,
                    hp: 50,
                    team_id: 1,
                    ammo: 15,
                    cooldowns: std::collections::HashMap::new(),
                    behavior_graph: None,
                },
            ],
            obstacles: vec![(3, 3)],
        };

        let stats = scene.stats();
        assert_eq!(stats.entity_count, 2);
        assert_eq!(stats.obstacle_count, 1);
        assert!(stats.has_behavior_graphs);
        assert_eq!(stats.total_cooldowns, 2);
    }

    // === SceneValidationIssue enum tests ===

    #[test]
    fn test_scene_validation_issue_display() {
        let error = SceneValidationIssue::Error("Something went wrong".to_string());
        let warning = SceneValidationIssue::Warning("Consider this".to_string());

        assert!(format!("{}", error).contains("Error"));
        assert!(format!("{}", error).contains("Something went wrong"));
        assert!(format!("{}", warning).contains("Warning"));
        assert!(format!("{}", warning).contains("Consider this"));
    }

    #[test]
    fn test_scene_validation_issue_all_variants() {
        let variants = SceneValidationIssue::all_variants();
        assert_eq!(variants.len(), 2);
        assert!(variants.contains(&"Error"));
        assert!(variants.contains(&"Warning"));
    }

    #[test]
    fn test_scene_validation_issue_helpers() {
        let error = SceneValidationIssue::error("Test error");
        let warning = SceneValidationIssue::warning("Test warning");

        assert!(error.is_error());
        assert!(!error.is_warning());
        assert_eq!(error.name(), "Error");
        assert_eq!(error.icon(), "[x]");

        assert!(!warning.is_error());
        assert!(warning.is_warning());
        assert_eq!(warning.name(), "Warning");
        assert_eq!(warning.icon(), "[!]");
    }

    #[test]
    fn test_scene_validation_issue_message() {
        let error = SceneValidationIssue::Error("Error message".to_string());
        let warning = SceneValidationIssue::Warning("Warning message".to_string());

        assert_eq!(error.message(), "Error message");
        assert_eq!(warning.message(), "Warning message");
    }
}
