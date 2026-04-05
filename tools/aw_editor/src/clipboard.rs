use anyhow::{Context, Result};
use astraweave_core::{Entity, IVec2, World};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Current clipboard data schema version.
/// Increment when making breaking changes to ClipboardEntityData.
pub const CLIPBOARD_SCHEMA_VERSION: u32 = 2;

/// Validation result for clipboard data integrity checks.
#[derive(Debug, Clone)]
pub struct ClipboardValidation {
    /// Whether the data passed all validation checks
    pub is_valid: bool,
    /// List of validation errors (if any)
    pub errors: Vec<String>,
    /// List of validation warnings (non-fatal issues)
    pub warnings: Vec<String>,
}

impl ClipboardValidation {
    /// Create a passing validation result
    pub fn valid() -> Self {
        Self {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Get total issue count (errors + warnings)
    pub fn issue_count(&self) -> usize {
        self.errors.len() + self.warnings.len()
    }

    /// Check if there are any warnings
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    /// Check if there are any errors
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Create a failed validation result with an error
    pub fn with_error(error: impl Into<String>) -> Self {
        Self {
            is_valid: false,
            errors: vec![error.into()],
            warnings: Vec::new(),
        }
    }

    /// Add a warning without failing validation
    pub fn add_warning(&mut self, warning: impl Into<String>) {
        self.warnings.push(warning.into());
    }

    /// Add an error and mark as invalid
    pub fn add_error(&mut self, error: impl Into<String>) {
        self.errors.push(error.into());
        self.is_valid = false;
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClipboardEntityData {
    pub name: String,
    pub pos: IVec2,
    /// Vertical position (Y axis in 3D).
    #[serde(default)]
    pub height: f32,
    pub rotation: f32,
    pub rotation_x: f32,
    pub rotation_z: f32,
    pub scale: f32,
    pub hp: i32,
    pub team_id: u8,
    pub ammo: i32,
    pub cooldowns: BTreeMap<String, f32>,
    pub behavior_graph: Option<astraweave_behavior::BehaviorGraph>,
    /// Parent entity ID for hierarchy preservation.
    #[serde(default)]
    pub parent: Option<Entity>,
}

impl ClipboardEntityData {
    /// Validate this entity data for consistency
    pub fn validate(&self) -> ClipboardValidation {
        let mut result = ClipboardValidation::valid();

        // Name validation
        if self.name.is_empty() {
            result.add_warning("Entity has empty name");
        }
        if self.name.len() > 256 {
            result.add_error("Entity name exceeds maximum length (256 characters)");
        }

        // Scale validation
        if self.scale <= 0.0 {
            result.add_error(format!("Invalid scale value: {} (must be > 0)", self.scale));
        }
        if self.scale > 1000.0 {
            result.add_warning(format!("Unusually large scale value: {}", self.scale));
        }

        // Health validation
        if self.hp < 0 {
            result.add_warning(format!("Negative health value: {}", self.hp));
        }

        // Ammo validation
        if self.ammo < 0 {
            result.add_warning(format!("Negative ammo value: {}", self.ammo));
        }

        result
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClipboardData {
    /// Schema version for forward/backward compatibility
    #[serde(default = "default_schema_version")]
    pub version: u32,
    /// The copied entities
    pub entities: Vec<ClipboardEntityData>,
}

fn default_schema_version() -> u32 {
    1 // Legacy data without version field
}

/// Statistics about clipboard contents.
#[derive(Debug, Clone, Copy)]
pub struct ClipboardStats {
    /// Number of entities in clipboard
    pub entity_count: usize,
    /// Entities with behavior graphs
    pub with_behavior_graph: usize,
    /// Entities with cooldowns
    pub with_cooldowns: usize,
    /// Total cooldown entries across all entities
    pub total_cooldowns: usize,
    /// Number of unique teams
    pub unique_teams: usize,
    /// Schema version
    pub version: u32,
}

impl ClipboardStats {
    /// Check if clipboard has any entities with AI behavior
    pub fn has_ai_entities(&self) -> bool {
        self.with_behavior_graph > 0
    }

    /// Check if clipboard has any entities with cooldowns
    pub fn has_cooldown_entities(&self) -> bool {
        self.with_cooldowns > 0
    }

    /// Check if clipboard has multiple teams
    pub fn is_multi_team(&self) -> bool {
        self.unique_teams > 1
    }

    /// Check if clipboard is empty
    pub fn is_empty(&self) -> bool {
        self.entity_count == 0
    }

    /// Get summary string for clipboard contents
    pub fn summary(&self) -> String {
        format!(
            "{} entities, {} with AI, {} teams",
            self.entity_count, self.with_behavior_graph, self.unique_teams
        )
    }
}

impl std::fmt::Display for ClipboardStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Clipboard: {} entities (v{})",
            self.entity_count, self.version
        )
    }
}

impl ClipboardEntityData {
    /// Check if entity has a behavior graph (AI)
    pub fn has_ai(&self) -> bool {
        self.behavior_graph.is_some()
    }

    /// Check if entity has any cooldowns
    pub fn has_cooldowns(&self) -> bool {
        !self.cooldowns.is_empty()
    }

    /// Get cooldown count
    pub fn cooldown_count(&self) -> usize {
        self.cooldowns.len()
    }

    /// Check if entity is scaled (non-uniform scale)
    pub fn is_scaled(&self) -> bool {
        (self.scale - 1.0).abs() > 0.001
    }

    /// Check if entity is rotated
    pub fn is_rotated(&self) -> bool {
        self.rotation.abs() > 0.001
            || self.rotation_x.abs() > 0.001
            || self.rotation_z.abs() > 0.001
    }

    /// Get position as tuple
    pub fn position(&self) -> (i32, i32) {
        (self.pos.x, self.pos.y)
    }

    /// Get summary string
    pub fn summary(&self) -> String {
        format!(
            "{} @ ({}, {}) HP:{} Team:{}",
            self.name, self.pos.x, self.pos.y, self.hp, self.team_id
        )
    }
}

impl std::fmt::Display for ClipboardEntityData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} @ ({}, {})", self.name, self.pos.x, self.pos.y)
    }
}

impl ClipboardValidation {
    /// Get summary of validation result
    pub fn summary(&self) -> String {
        if self.is_valid && self.warnings.is_empty() {
            "Valid".to_string()
        } else if self.is_valid {
            format!("Valid with {} warnings", self.warnings.len())
        } else {
            format!(
                "{} errors, {} warnings",
                self.errors.len(),
                self.warnings.len()
            )
        }
    }
}

impl std::fmt::Display for ClipboardValidation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_valid {
            write!(f, "✓ Valid")
        } else {
            write!(f, "✗ Invalid ({} errors)", self.errors.len())
        }
    }
}

impl ClipboardStats {
    /// Get percentage of entities with behavior graphs
    pub fn ai_percentage(&self) -> f32 {
        if self.entity_count == 0 {
            0.0
        } else {
            (self.with_behavior_graph as f32 / self.entity_count as f32) * 100.0
        }
    }

    /// Average cooldowns per entity
    pub fn avg_cooldowns_per_entity(&self) -> f32 {
        if self.entity_count == 0 {
            0.0
        } else {
            self.total_cooldowns as f32 / self.entity_count as f32
        }
    }
}

impl ClipboardData {
    pub fn from_entities(world: &World, entity_ids: &[Entity]) -> Self {
        let mut entities = Vec::new();

        for &entity_id in entity_ids {
            let pose = world.pose(entity_id).unwrap_or(astraweave_core::Pose {
                pos: IVec2 { x: 0, y: 0 },
                height: 0.0,
                rotation: 0.0,
                rotation_x: 0.0,
                rotation_z: 0.0,
                scale: 1.0,
                scale_y: 1.0,
                scale_z: 1.0,
                float_x: 0.0,
                float_z: 0.0,
                use_float_pos: false,
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

            let parent = world.parent_of(entity_id);
            entities.push(ClipboardEntityData {
                name,
                pos: pose.pos,
                height: pose.height,
                rotation: pose.rotation,
                rotation_x: pose.rotation_x,
                rotation_z: pose.rotation_z,
                scale: pose.scale,
                hp: health,
                team_id: team,
                ammo: ammo_rounds,
                cooldowns,
                behavior_graph,
                parent,
            });
        }

        ClipboardData {
            version: CLIPBOARD_SCHEMA_VERSION,
            entities,
        }
    }

    /// Validate the clipboard data for consistency and version compatibility.
    pub fn validate(&self) -> ClipboardValidation {
        let mut result = ClipboardValidation::valid();

        // Version check
        if self.version > CLIPBOARD_SCHEMA_VERSION {
            result.add_warning(format!(
                "Clipboard data from newer version ({}) - some features may not work",
                self.version
            ));
        }

        // Empty check
        if self.entities.is_empty() {
            result.add_warning("Clipboard contains no entities");
        }

        // Validate each entity
        for (i, entity) in self.entities.iter().enumerate() {
            let entity_validation = entity.validate();
            for error in entity_validation.errors {
                result.add_error(format!("Entity {}: {}", i, error));
            }
            for warning in entity_validation.warnings {
                result.add_warning(format!("Entity {}: {}", i, warning));
            }
        }

        result
    }

    /// Check if data is compatible with current schema version.
    pub fn is_compatible(&self) -> bool {
        self.version <= CLIPBOARD_SCHEMA_VERSION
    }

    /// Get the number of entities in clipboard.
    pub fn len(&self) -> usize {
        self.entities.len()
    }

    /// Check if clipboard is empty.
    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    /// Get statistics about clipboard contents
    pub fn stats(&self) -> ClipboardStats {
        let total_entities = self.entities.len();
        let with_behavior_graph = self
            .entities
            .iter()
            .filter(|e| e.behavior_graph.is_some())
            .count();
        let with_cooldowns = self
            .entities
            .iter()
            .filter(|e| !e.cooldowns.is_empty())
            .count();
        let total_cooldowns: usize = self.entities.iter().map(|e| e.cooldowns.len()).sum();
        let teams: std::collections::HashSet<u8> =
            self.entities.iter().map(|e| e.team_id).collect();

        ClipboardStats {
            entity_count: total_entities,
            with_behavior_graph,
            with_cooldowns,
            total_cooldowns,
            unique_teams: teams.len(),
            version: self.version,
        }
    }

    /// Get entity names in clipboard
    pub fn entity_names(&self) -> Vec<&str> {
        self.entities.iter().map(|e| e.name.as_str()).collect()
    }

    /// Find entities by name pattern (case-insensitive)
    pub fn find_by_name(&self, pattern: &str) -> Vec<usize> {
        let pattern_lower = pattern.to_lowercase();
        self.entities
            .iter()
            .enumerate()
            .filter(|(_, e)| e.name.to_lowercase().contains(&pattern_lower))
            .map(|(i, _)| i)
            .collect()
    }

    /// Get entities belonging to a specific team
    pub fn filter_by_team(&self, team_id: u8) -> Vec<&ClipboardEntityData> {
        self.entities
            .iter()
            .filter(|e| e.team_id == team_id)
            .collect()
    }

    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string(self).context("Failed to serialize clipboard data")
    }

    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json).context("Failed to deserialize clipboard data")
    }

    pub fn spawn_entities(&self, world: &mut World, offset: IVec2) -> Result<Vec<Entity>> {
        let mut spawned = Vec::new();

        for entity_data in &self.entities {
            let new_pos = IVec2 {
                x: entity_data.pos.x + offset.x,
                y: entity_data.pos.y + offset.y,
            };

            let id = world.spawn(
                &entity_data.name,
                new_pos,
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

            if let Some(bg) = &entity_data.behavior_graph {
                world.set_behavior_graph(id, bg.clone());
            }

            spawned.push(id);
        }

        Ok(spawned)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clipboard_from_entities() {
        let mut world = World::new();
        let e1 = world.spawn(
            "Entity1",
            IVec2 { x: 5, y: 10 },
            astraweave_core::Team { id: 0 },
            100,
            30,
        );
        let e2 = world.spawn(
            "Entity2",
            IVec2 { x: 15, y: 20 },
            astraweave_core::Team { id: 1 },
            80,
            20,
        );

        let clipboard = ClipboardData::from_entities(&world, &[e1, e2]);

        assert_eq!(clipboard.entities.len(), 2);
        assert_eq!(clipboard.entities[0].name, "Entity1");
        assert_eq!(clipboard.entities[1].name, "Entity2");
    }

    #[test]
    fn test_clipboard_json_serialization() {
        let mut world = World::new();
        let e1 = world.spawn(
            "TestEntity",
            IVec2 { x: 0, y: 0 },
            astraweave_core::Team { id: 0 },
            100,
            30,
        );

        let clipboard = ClipboardData::from_entities(&world, &[e1]);
        let json = clipboard.to_json().unwrap();

        assert!(json.contains("TestEntity"));

        let restored = ClipboardData::from_json(&json).unwrap();
        assert_eq!(restored.entities.len(), 1);
        assert_eq!(restored.entities[0].name, "TestEntity");
    }

    #[test]
    fn test_spawn_entities_with_offset() {
        let mut world = World::new();
        let e1 = world.spawn(
            "Original",
            IVec2 { x: 10, y: 10 },
            astraweave_core::Team { id: 0 },
            100,
            30,
        );

        let clipboard = ClipboardData::from_entities(&world, &[e1]);

        let spawned = clipboard
            .spawn_entities(&mut world, IVec2 { x: 5, y: 5 })
            .unwrap();

        assert_eq!(spawned.len(), 1);

        let new_pos = world.pose(spawned[0]).unwrap().pos;
        assert_eq!(new_pos.x, 15);
        assert_eq!(new_pos.y, 15);
    }

    #[test]
    fn test_multiple_entities_spawn() {
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
            IVec2 { x: 5, y: 5 },
            astraweave_core::Team { id: 1 },
            50,
            15,
        );

        let clipboard = ClipboardData::from_entities(&world, &[e1, e2]);
        let spawned = clipboard
            .spawn_entities(&mut world, IVec2 { x: 10, y: 10 })
            .unwrap();

        assert_eq!(spawned.len(), 2);
        assert_eq!(world.entities().len(), 4);
    }

    #[test]
    fn test_preserve_all_properties() {
        let mut world = World::new();
        let entity = world.spawn(
            "CompleteEntity",
            IVec2 { x: 5, y: 10 },
            astraweave_core::Team { id: 2 },
            75,
            25,
        );

        if let Some(pose) = world.pose_mut(entity) {
            pose.rotation = 1.57;
            pose.rotation_x = 0.78;
            pose.rotation_z = 0.39;
            pose.scale = 2.5;
        }

        let clipboard = ClipboardData::from_entities(&world, &[entity]);
        let entity_data = &clipboard.entities[0];

        assert_eq!(entity_data.name, "CompleteEntity");
        assert_eq!(entity_data.hp, 75);
        assert_eq!(entity_data.team_id, 2);
        assert_eq!(entity_data.ammo, 25);
        assert!((entity_data.rotation - 1.57).abs() < 0.01);
        assert!((entity_data.scale - 2.5).abs() < 0.01);
    }

    #[test]
    fn test_empty_clipboard() {
        let world = World::new();
        let clipboard = ClipboardData::from_entities(&world, &[]);

        assert_eq!(clipboard.entities.len(), 0);

        let json = clipboard.to_json().unwrap();
        let restored = ClipboardData::from_json(&json).unwrap();
        assert_eq!(restored.entities.len(), 0);
    }

    #[test]
    fn test_behavior_graph_preservation() {
        use astraweave_behavior::{BehaviorGraph, BehaviorNode};

        let mut world = World::new();
        let entity = world.spawn(
            "AIEntity",
            IVec2 { x: 0, y: 0 },
            astraweave_core::Team { id: 0 },
            100,
            30,
        );

        // Create a simple behavior graph
        let root = BehaviorNode::Sequence(vec![
            BehaviorNode::Action("patrol".into()),
            BehaviorNode::Action("attack".into()),
        ]);
        let graph = BehaviorGraph::new(root);
        world.set_behavior_graph(entity, graph);

        // Copy to clipboard
        let clipboard = ClipboardData::from_entities(&world, &[entity]);
        assert!(clipboard.entities[0].behavior_graph.is_some());

        // Spawn from clipboard
        let spawned = clipboard
            .spawn_entities(&mut world, IVec2 { x: 10, y: 10 })
            .unwrap();

        // Verify BehaviorGraph was restored
        let restored_graph = world.behavior_graph(spawned[0]);
        assert!(
            restored_graph.is_some(),
            "BehaviorGraph should be restored after paste"
        );
    }

    // ====================================================================
    // ClipboardValidation Tests
    // ====================================================================

    #[test]
    fn test_clipboard_validation_valid() {
        let validation = ClipboardValidation::valid();
        assert!(validation.is_valid);
        assert!(validation.errors.is_empty());
        assert!(validation.warnings.is_empty());
    }

    #[test]
    fn test_clipboard_validation_with_error() {
        let validation = ClipboardValidation::with_error("Test error");
        assert!(!validation.is_valid);
        assert_eq!(validation.errors.len(), 1);
        assert!(validation.errors[0].contains("Test error"));
    }

    #[test]
    fn test_clipboard_validation_add_warning() {
        let mut validation = ClipboardValidation::valid();
        validation.add_warning("Minor issue");

        assert!(validation.is_valid); // Warnings don't invalidate
        assert!(validation.warnings.len() == 1);
        assert!(validation.warnings[0].contains("Minor issue"));
    }

    #[test]
    fn test_clipboard_validation_add_error() {
        let mut validation = ClipboardValidation::valid();
        validation.add_error("Critical issue");

        assert!(!validation.is_valid);
        assert_eq!(validation.errors.len(), 1);
    }

    // ====================================================================
    // ClipboardEntityData Validation Tests
    // ====================================================================

    #[test]
    fn test_entity_data_validate_valid() {
        let entity = ClipboardEntityData {
            name: "ValidEntity".to_string(),
            pos: IVec2 { x: 0, y: 0 },
            rotation: 0.0,
            rotation_x: 0.0,
            rotation_z: 0.0,
            scale: 1.0,
            hp: 100,
            team_id: 0,
            ammo: 30,
            cooldowns: BTreeMap::new(),
            behavior_graph: None,
            height: 0.0,
            parent: None,
        };

        let validation = entity.validate();
        assert!(validation.is_valid);
        assert!(validation.errors.is_empty());
        assert!(validation.warnings.is_empty());
    }

    #[test]
    fn test_entity_data_validate_empty_name() {
        let entity = ClipboardEntityData {
            name: "".to_string(),
            pos: IVec2 { x: 0, y: 0 },
            rotation: 0.0,
            rotation_x: 0.0,
            rotation_z: 0.0,
            scale: 1.0,
            hp: 100,
            team_id: 0,
            ammo: 30,
            cooldowns: BTreeMap::new(),
            behavior_graph: None,
            height: 0.0,
            parent: None,
        };

        let validation = entity.validate();
        assert!(validation.is_valid); // Empty name is a warning, not error
        assert_eq!(validation.warnings.len(), 1);
        assert!(validation.warnings[0].contains("empty name"));
    }

    #[test]
    fn test_entity_data_validate_invalid_scale() {
        let entity = ClipboardEntityData {
            name: "BadScale".to_string(),
            pos: IVec2 { x: 0, y: 0 },
            rotation: 0.0,
            rotation_x: 0.0,
            rotation_z: 0.0,
            scale: 0.0, // Invalid!
            hp: 100,
            team_id: 0,
            ammo: 30,
            cooldowns: BTreeMap::new(),
            behavior_graph: None,
            height: 0.0,
            parent: None,
        };

        let validation = entity.validate();
        assert!(!validation.is_valid);
        assert!(!validation.errors.is_empty());
        assert!(validation.errors[0].contains("scale"));
    }

    #[test]
    fn test_entity_data_validate_negative_values() {
        let entity = ClipboardEntityData {
            name: "NegativeValues".to_string(),
            pos: IVec2 { x: 0, y: 0 },
            rotation: 0.0,
            rotation_x: 0.0,
            rotation_z: 0.0,
            scale: 1.0,
            hp: -50, // Warning
            team_id: 0,
            ammo: -10, // Warning
            cooldowns: BTreeMap::new(),
            behavior_graph: None,
            height: 0.0,
            parent: None,
        };

        let validation = entity.validate();
        assert!(validation.is_valid); // Negative values are warnings
        assert_eq!(validation.warnings.len(), 2);
    }

    // ====================================================================
    // ClipboardData Validation Tests
    // ====================================================================

    #[test]
    fn test_clipboard_data_version() {
        let mut world = World::new();
        let entity = world.spawn(
            "E1",
            IVec2 { x: 0, y: 0 },
            astraweave_core::Team { id: 0 },
            100,
            30,
        );

        let clipboard = ClipboardData::from_entities(&world, &[entity]);
        assert_eq!(clipboard.version, CLIPBOARD_SCHEMA_VERSION);
        assert!(clipboard.is_compatible());
    }

    #[test]
    fn test_clipboard_data_len_empty() {
        let clipboard = ClipboardData {
            version: CLIPBOARD_SCHEMA_VERSION,
            entities: vec![],
        };

        assert_eq!(clipboard.len(), 0);
        assert!(clipboard.is_empty());
    }

    #[test]
    fn test_clipboard_data_validate_empty() {
        let clipboard = ClipboardData {
            version: CLIPBOARD_SCHEMA_VERSION,
            entities: vec![],
        };

        let validation = clipboard.validate();
        assert!(validation.is_valid); // Empty is valid, just with warning
        assert_eq!(validation.warnings.len(), 1);
        assert!(validation.warnings[0].contains("no entities"));
    }

    #[test]
    fn test_clipboard_data_validate_future_version() {
        let clipboard = ClipboardData {
            version: CLIPBOARD_SCHEMA_VERSION + 1,
            entities: vec![ClipboardEntityData {
                name: "Future".to_string(),
                pos: IVec2 { x: 0, y: 0 },
                rotation: 0.0,
                rotation_x: 0.0,
                rotation_z: 0.0,
                scale: 1.0,
                hp: 100,
                team_id: 0,
                ammo: 30,
                cooldowns: BTreeMap::new(),
                behavior_graph: None,
                height: 0.0,
                parent: None,
            }],
        };

        assert!(!clipboard.is_compatible());
        let validation = clipboard.validate();
        assert!(validation.is_valid); // Still valid, just warning
        assert!(validation
            .warnings
            .iter()
            .any(|w| w.contains("newer version")));
    }

    #[test]
    fn test_clipboard_data_json_preserves_version() {
        let clipboard = ClipboardData {
            version: CLIPBOARD_SCHEMA_VERSION,
            entities: vec![ClipboardEntityData {
                name: "Test".to_string(),
                pos: IVec2 { x: 0, y: 0 },
                rotation: 0.0,
                rotation_x: 0.0,
                rotation_z: 0.0,
                scale: 1.0,
                hp: 100,
                team_id: 0,
                ammo: 30,
                cooldowns: BTreeMap::new(),
                behavior_graph: None,
                height: 0.0,
                parent: None,
            }],
        };

        let json = clipboard.to_json().unwrap();
        let restored = ClipboardData::from_json(&json).unwrap();

        assert_eq!(restored.version, CLIPBOARD_SCHEMA_VERSION);
    }

    #[test]
    fn test_clipboard_data_legacy_json_defaults_version() {
        // Simulate legacy data without version field
        let legacy_json = r#"{"entities":[{"name":"Old","pos":{"x":0,"y":0},"rotation":0.0,"rotation_x":0.0,"rotation_z":0.0,"scale":1.0,"hp":100,"team_id":0,"ammo":30,"cooldowns":{},"behavior_graph":null}]}"#;

        let clipboard = ClipboardData::from_json(legacy_json).unwrap();
        assert_eq!(clipboard.version, 1); // default_schema_version()
        assert!(clipboard.is_compatible());
    }

    // ====================================================================
    // ClipboardValidation New Methods Tests
    // ====================================================================

    #[test]
    fn test_clipboard_validation_issue_count() {
        let mut validation = ClipboardValidation::valid();
        assert_eq!(validation.issue_count(), 0);

        validation.add_warning("warn1");
        validation.add_error("err1");
        assert_eq!(validation.issue_count(), 2);
    }

    #[test]
    fn test_clipboard_validation_has_warnings_errors() {
        let mut validation = ClipboardValidation::valid();
        assert!(!validation.has_warnings());
        assert!(!validation.has_errors());

        validation.add_warning("warn");
        assert!(validation.has_warnings());
        assert!(!validation.has_errors());

        validation.add_error("err");
        assert!(validation.has_errors());
    }

    // ====================================================================
    // ClipboardStats Tests
    // ====================================================================

    #[test]
    fn test_clipboard_stats_empty() {
        let clipboard = ClipboardData {
            version: CLIPBOARD_SCHEMA_VERSION,
            entities: vec![],
        };
        let stats = clipboard.stats();

        assert_eq!(stats.entity_count, 0);
        assert_eq!(stats.with_behavior_graph, 0);
        assert!(!stats.has_ai_entities());
    }

    #[test]
    fn test_clipboard_stats_with_entities() {
        let mut world = World::new();
        let e1 = world.spawn(
            "E1",
            IVec2 { x: 0, y: 0 },
            astraweave_core::Team { id: 0 },
            100,
            30,
        );
        let _e2 = world.spawn(
            "E2",
            IVec2 { x: 1, y: 1 },
            astraweave_core::Team { id: 1 },
            50,
            20,
        );

        let clipboard = ClipboardData::from_entities(&world, &[e1]);
        let stats = clipboard.stats();

        assert_eq!(stats.entity_count, 1);
        assert_eq!(stats.version, CLIPBOARD_SCHEMA_VERSION);
    }

    #[test]
    fn test_clipboard_stats_ai_percentage() {
        let stats = ClipboardStats {
            entity_count: 10,
            with_behavior_graph: 3,
            with_cooldowns: 0,
            total_cooldowns: 0,
            unique_teams: 1,
            version: CLIPBOARD_SCHEMA_VERSION,
        };

        assert!((stats.ai_percentage() - 30.0).abs() < 0.1);
    }

    #[test]
    fn test_clipboard_stats_avg_cooldowns() {
        let stats = ClipboardStats {
            entity_count: 5,
            with_behavior_graph: 0,
            with_cooldowns: 3,
            total_cooldowns: 15,
            unique_teams: 1,
            version: CLIPBOARD_SCHEMA_VERSION,
        };

        assert!((stats.avg_cooldowns_per_entity() - 3.0).abs() < 0.1);
    }

    #[test]
    fn test_clipboard_stats_zero_division() {
        let stats = ClipboardStats {
            entity_count: 0,
            with_behavior_graph: 0,
            with_cooldowns: 0,
            total_cooldowns: 0,
            unique_teams: 0,
            version: CLIPBOARD_SCHEMA_VERSION,
        };

        assert_eq!(stats.ai_percentage(), 0.0);
        assert_eq!(stats.avg_cooldowns_per_entity(), 0.0);
    }

    // ====================================================================
    // ClipboardData Query Methods Tests
    // ====================================================================

    #[test]
    fn test_clipboard_entity_names() {
        let mut world = World::new();
        let e1 = world.spawn(
            "Alpha",
            IVec2 { x: 0, y: 0 },
            astraweave_core::Team { id: 0 },
            100,
            30,
        );
        let e2 = world.spawn(
            "Beta",
            IVec2 { x: 1, y: 1 },
            astraweave_core::Team { id: 0 },
            100,
            30,
        );

        let clipboard = ClipboardData::from_entities(&world, &[e1, e2]);
        let names = clipboard.entity_names();

        assert_eq!(names.len(), 2);
        assert!(names.contains(&"Alpha"));
        assert!(names.contains(&"Beta"));
    }

    #[test]
    fn test_clipboard_find_by_name() {
        let clipboard = ClipboardData {
            version: CLIPBOARD_SCHEMA_VERSION,
            entities: vec![
                ClipboardEntityData {
                    name: "PlayerUnit".to_string(),
                    pos: IVec2 { x: 0, y: 0 },
                    rotation: 0.0,
                    rotation_x: 0.0,
                    rotation_z: 0.0,
                    scale: 1.0,
                    hp: 100,
                    team_id: 0,
                    ammo: 30,
                    cooldowns: BTreeMap::new(),
                    behavior_graph: None,
                    height: 0.0,
                    parent: None,
                },
                ClipboardEntityData {
                    name: "EnemyUnit".to_string(),
                    pos: IVec2 { x: 1, y: 1 },
                    rotation: 0.0,
                    rotation_x: 0.0,
                    rotation_z: 0.0,
                    scale: 1.0,
                    hp: 100,
                    team_id: 1,
                    ammo: 30,
                    cooldowns: BTreeMap::new(),
                    behavior_graph: None,
                    height: 0.0,
                    parent: None,
                },
            ],
        };

        let results = clipboard.find_by_name("unit");
        assert_eq!(results.len(), 2); // Both match "unit"

        let results = clipboard.find_by_name("player");
        assert_eq!(results.len(), 1);
        assert!(results.contains(&0));
    }

    #[test]
    fn test_clipboard_filter_by_team() {
        let clipboard = ClipboardData {
            version: CLIPBOARD_SCHEMA_VERSION,
            entities: vec![
                ClipboardEntityData {
                    name: "Ally1".to_string(),
                    pos: IVec2 { x: 0, y: 0 },
                    rotation: 0.0,
                    rotation_x: 0.0,
                    rotation_z: 0.0,
                    scale: 1.0,
                    hp: 100,
                    team_id: 0,
                    ammo: 30,
                    cooldowns: BTreeMap::new(),
                    behavior_graph: None,
                    height: 0.0,
                    parent: None,
                },
                ClipboardEntityData {
                    name: "Enemy1".to_string(),
                    pos: IVec2 { x: 1, y: 1 },
                    rotation: 0.0,
                    rotation_x: 0.0,
                    rotation_z: 0.0,
                    scale: 1.0,
                    hp: 100,
                    team_id: 1,
                    ammo: 30,
                    cooldowns: BTreeMap::new(),
                    behavior_graph: None,
                    height: 0.0,
                    parent: None,
                },
                ClipboardEntityData {
                    name: "Ally2".to_string(),
                    pos: IVec2 { x: 2, y: 2 },
                    rotation: 0.0,
                    rotation_x: 0.0,
                    rotation_z: 0.0,
                    scale: 1.0,
                    hp: 100,
                    team_id: 0,
                    ammo: 30,
                    cooldowns: BTreeMap::new(),
                    behavior_graph: None,
                    height: 0.0,
                    parent: None,
                },
            ],
        };

        let team0 = clipboard.filter_by_team(0);
        assert_eq!(team0.len(), 2);

        let team1 = clipboard.filter_by_team(1);
        assert_eq!(team1.len(), 1);
        assert_eq!(team1[0].name, "Enemy1");
    }

    // ====================================================================
    // ClipboardStats New Methods Tests
    // ====================================================================

    #[test]
    fn test_clipboard_stats_has_cooldown_entities() {
        let stats = ClipboardStats {
            entity_count: 5,
            with_behavior_graph: 0,
            with_cooldowns: 2,
            total_cooldowns: 5,
            unique_teams: 1,
            version: CLIPBOARD_SCHEMA_VERSION,
        };
        assert!(stats.has_cooldown_entities());
    }

    #[test]
    fn test_clipboard_stats_is_multi_team() {
        let single = ClipboardStats {
            entity_count: 5,
            with_behavior_graph: 0,
            with_cooldowns: 0,
            total_cooldowns: 0,
            unique_teams: 1,
            version: CLIPBOARD_SCHEMA_VERSION,
        };
        assert!(!single.is_multi_team());

        let multi = ClipboardStats {
            entity_count: 5,
            with_behavior_graph: 0,
            with_cooldowns: 0,
            total_cooldowns: 0,
            unique_teams: 3,
            version: CLIPBOARD_SCHEMA_VERSION,
        };
        assert!(multi.is_multi_team());
    }

    #[test]
    fn test_clipboard_stats_is_empty() {
        let empty = ClipboardStats {
            entity_count: 0,
            with_behavior_graph: 0,
            with_cooldowns: 0,
            total_cooldowns: 0,
            unique_teams: 0,
            version: CLIPBOARD_SCHEMA_VERSION,
        };
        assert!(empty.is_empty());
    }

    #[test]
    fn test_clipboard_stats_summary() {
        let stats = ClipboardStats {
            entity_count: 10,
            with_behavior_graph: 5,
            with_cooldowns: 3,
            total_cooldowns: 8,
            unique_teams: 2,
            version: CLIPBOARD_SCHEMA_VERSION,
        };
        let summary = stats.summary();
        assert!(summary.contains("10"));
        assert!(summary.contains("5"));
    }

    #[test]
    fn test_clipboard_stats_display() {
        let stats = ClipboardStats {
            entity_count: 5,
            with_behavior_graph: 0,
            with_cooldowns: 0,
            total_cooldowns: 0,
            unique_teams: 1,
            version: CLIPBOARD_SCHEMA_VERSION,
        };
        let display = format!("{}", stats);
        assert!(display.contains("5"));
        assert!(display.contains("Clipboard"));
    }

    // ====================================================================
    // ClipboardEntityData New Methods Tests
    // ====================================================================

    #[test]
    fn test_clipboard_entity_data_has_ai() {
        let without_ai = ClipboardEntityData {
            name: "Test".to_string(),
            pos: IVec2 { x: 0, y: 0 },
            rotation: 0.0,
            rotation_x: 0.0,
            rotation_z: 0.0,
            scale: 1.0,
            hp: 100,
            team_id: 0,
            ammo: 30,
            cooldowns: BTreeMap::new(),
            behavior_graph: None,
            height: 0.0,
            parent: None,
        };
        assert!(!without_ai.has_ai());
    }

    #[test]
    fn test_clipboard_entity_data_has_cooldowns() {
        let mut data = ClipboardEntityData {
            name: "Test".to_string(),
            pos: IVec2 { x: 0, y: 0 },
            rotation: 0.0,
            rotation_x: 0.0,
            rotation_z: 0.0,
            scale: 1.0,
            hp: 100,
            team_id: 0,
            ammo: 30,
            cooldowns: BTreeMap::new(),
            behavior_graph: None,
            height: 0.0,
            parent: None,
        };
        assert!(!data.has_cooldowns());
        data.cooldowns.insert("attack".to_string(), 1.5);
        assert!(data.has_cooldowns());
        assert_eq!(data.cooldown_count(), 1);
    }

    #[test]
    fn test_clipboard_entity_data_is_scaled() {
        let normal = ClipboardEntityData {
            name: "Test".to_string(),
            pos: IVec2 { x: 0, y: 0 },
            rotation: 0.0,
            rotation_x: 0.0,
            rotation_z: 0.0,
            scale: 1.0,
            hp: 100,
            team_id: 0,
            ammo: 30,
            cooldowns: BTreeMap::new(),
            behavior_graph: None,
            height: 0.0,
            parent: None,
        };
        assert!(!normal.is_scaled());

        let scaled = ClipboardEntityData {
            name: "Test".to_string(),
            pos: IVec2 { x: 0, y: 0 },
            rotation: 0.0,
            rotation_x: 0.0,
            rotation_z: 0.0,
            scale: 2.5,
            hp: 100,
            team_id: 0,
            ammo: 30,
            cooldowns: BTreeMap::new(),
            behavior_graph: None,
            height: 0.0,
            parent: None,
        };
        assert!(scaled.is_scaled());
    }

    #[test]
    fn test_clipboard_entity_data_is_rotated() {
        let not_rotated = ClipboardEntityData {
            name: "Test".to_string(),
            pos: IVec2 { x: 0, y: 0 },
            rotation: 0.0,
            rotation_x: 0.0,
            rotation_z: 0.0,
            scale: 1.0,
            hp: 100,
            team_id: 0,
            ammo: 30,
            cooldowns: BTreeMap::new(),
            behavior_graph: None,
            height: 0.0,
            parent: None,
        };
        assert!(!not_rotated.is_rotated());

        let rotated = ClipboardEntityData {
            name: "Test".to_string(),
            pos: IVec2 { x: 0, y: 0 },
            rotation: 1.57,
            rotation_x: 0.0,
            rotation_z: 0.0,
            scale: 1.0,
            hp: 100,
            team_id: 0,
            ammo: 30,
            cooldowns: BTreeMap::new(),
            behavior_graph: None,
            height: 0.0,
            parent: None,
        };
        assert!(rotated.is_rotated());
    }

    #[test]
    fn test_clipboard_entity_data_position() {
        let data = ClipboardEntityData {
            name: "Test".to_string(),
            pos: IVec2 { x: 5, y: 10 },
            rotation: 0.0,
            rotation_x: 0.0,
            rotation_z: 0.0,
            scale: 1.0,
            hp: 100,
            team_id: 0,
            ammo: 30,
            cooldowns: BTreeMap::new(),
            behavior_graph: None,
            height: 0.0,
            parent: None,
        };
        assert_eq!(data.position(), (5, 10));
    }

    #[test]
    fn test_clipboard_entity_data_summary() {
        let data = ClipboardEntityData {
            name: "Soldier".to_string(),
            pos: IVec2 { x: 5, y: 10 },
            rotation: 0.0,
            rotation_x: 0.0,
            rotation_z: 0.0,
            scale: 1.0,
            hp: 75,
            team_id: 1,
            ammo: 30,
            cooldowns: BTreeMap::new(),
            behavior_graph: None,
            height: 0.0,
            parent: None,
        };
        let summary = data.summary();
        assert!(summary.contains("Soldier"));
        assert!(summary.contains("75"));
    }

    #[test]
    fn test_clipboard_entity_data_display() {
        let data = ClipboardEntityData {
            name: "Unit".to_string(),
            pos: IVec2 { x: 3, y: 7 },
            rotation: 0.0,
            rotation_x: 0.0,
            rotation_z: 0.0,
            scale: 1.0,
            hp: 100,
            team_id: 0,
            ammo: 30,
            cooldowns: BTreeMap::new(),
            behavior_graph: None,
            height: 0.0,
            parent: None,
        };
        let display = format!("{}", data);
        assert!(display.contains("Unit"));
        assert!(display.contains("3"));
        assert!(display.contains("7"));
    }

    // ====================================================================
    // ClipboardValidation New Methods Tests
    // ====================================================================

    #[test]
    fn test_clipboard_validation_summary_valid() {
        let valid = ClipboardValidation::valid();
        assert_eq!(valid.summary(), "Valid");
    }

    #[test]
    fn test_clipboard_validation_summary_warnings() {
        let mut v = ClipboardValidation::valid();
        v.add_warning("test warning");
        let summary = v.summary();
        assert!(summary.contains("warning"));
    }

    #[test]
    fn test_clipboard_validation_summary_errors() {
        let mut v = ClipboardValidation::valid();
        v.add_error("test error");
        let summary = v.summary();
        assert!(summary.contains("error"));
    }

    #[test]
    fn test_clipboard_validation_display() {
        let valid = ClipboardValidation::valid();
        let display = format!("{}", valid);
        assert!(display.contains("Valid"));

        let invalid = ClipboardValidation::with_error("bad");
        let display = format!("{}", invalid);
        assert!(display.contains("Invalid"));
    }
}
