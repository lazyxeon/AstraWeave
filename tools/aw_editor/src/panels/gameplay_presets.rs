//! Gameplay Genre Presets — skeleton component schemas and starter configurations
//! for common game genres.
//!
//! Each preset defines:
//! - Recommended camera mode
//! - Skeleton ECS component schemas (empty structs with field names)
//! - Starter entity list
//! - Recommended world template
//!
//! Presets: OpenWorldRPG, TopDownRTS, SurvivalBuilder, TotalWarSimulation, Custom.

use super::world_wizard::WorldTemplate;

// ============================================================================
// GAMEPLAY PRESET ENUM
// ============================================================================

/// A curated gameplay genre preset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameplayPreset {
    OpenWorldRPG,
    TopDownRTS,
    SurvivalBuilder,
    TotalWarSimulation,
    Custom,
}

impl GameplayPreset {
    pub const ALL: &'static [GameplayPreset] = &[
        GameplayPreset::OpenWorldRPG,
        GameplayPreset::TopDownRTS,
        GameplayPreset::SurvivalBuilder,
        GameplayPreset::TotalWarSimulation,
        GameplayPreset::Custom,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            Self::OpenWorldRPG => "Open-World RPG",
            Self::TopDownRTS => "Top-Down RTS",
            Self::SurvivalBuilder => "Survival Builder",
            Self::TotalWarSimulation => "Total War Simulation",
            Self::Custom => "Custom",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::OpenWorldRPG => "[Sword]",
            Self::TopDownRTS => "[Flag]",
            Self::SurvivalBuilder => "[Hammer]",
            Self::TotalWarSimulation => "[Shield]",
            Self::Custom => "⚙️",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::OpenWorldRPG => "Third-person exploration with inventory, quests, dialogue, and combat stats. Think Skyrim or Witcher.",
            Self::TopDownRTS => "Overhead camera with unit selection, build queues, resource gathering, and fog of war. Think StarCraft or Age of Empires.",
            Self::SurvivalBuilder => "First-person crafting, hunger/thirst, base building, and tool durability. Think Rust or Valheim.",
            Self::TotalWarSimulation => "Grand strategy with army units, territory control, diplomacy, and formations. Think Total War or Crusader Kings.",
            Self::Custom => "Start with no preset components. Add your own systems from scratch.",
        }
    }

    /// The recommended camera mode string.
    pub fn camera_mode(&self) -> &'static str {
        match self {
            Self::OpenWorldRPG => "ThirdPersonOrbit",
            Self::TopDownRTS => "TopDownFixed",
            Self::SurvivalBuilder => "FirstPerson",
            Self::TotalWarSimulation => "StrategicOverhead",
            Self::Custom => "FreeFly",
        }
    }

    /// Recommended world template for quick-start workflows.
    pub fn recommended_template(&self) -> WorldTemplate {
        match self {
            Self::OpenWorldRPG => WorldTemplate::LushForest,
            Self::TopDownRTS => WorldTemplate::DesertWasteland,
            Self::SurvivalBuilder => WorldTemplate::TropicalCoast,
            Self::TotalWarSimulation => WorldTemplate::VolcanicWasteland,
            Self::Custom => WorldTemplate::Custom,
        }
    }

    /// Component schema definitions for this genre.
    /// Each entry is (component_name, &[(field_name, field_type)]).
    pub fn component_schemas(&self) -> &'static [ComponentSchema] {
        match self {
            Self::OpenWorldRPG => &RPG_SCHEMAS,
            Self::TopDownRTS => &RTS_SCHEMAS,
            Self::SurvivalBuilder => &SURVIVAL_SCHEMAS,
            Self::TotalWarSimulation => &TOTAL_WAR_SCHEMAS,
            Self::Custom => &[],
        }
    }

    /// Starter entities to add when this preset is applied.
    pub fn starter_entities(&self) -> &'static [StarterEntity] {
        match self {
            Self::OpenWorldRPG => &RPG_STARTERS,
            Self::TopDownRTS => &RTS_STARTERS,
            Self::SurvivalBuilder => &SURVIVAL_STARTERS,
            Self::TotalWarSimulation => &TOTAL_WAR_STARTERS,
            Self::Custom => &[],
        }
    }
}

// ============================================================================
// COMPONENT SCHEMA
// ============================================================================

/// A skeleton ECS component definition (name + fields).
pub struct ComponentSchema {
    pub name: &'static str,
    pub fields: &'static [(&'static str, &'static str)],
}

// ── RPG Components ──

static RPG_SCHEMAS: [ComponentSchema; 5] = [
    ComponentSchema {
        name: "Inventory",
        fields: &[
            ("slots", "Vec<ItemSlot>"),
            ("capacity", "u32"),
            ("gold", "i64"),
        ],
    },
    ComponentSchema {
        name: "QuestLog",
        fields: &[
            ("active_quests", "Vec<QuestId>"),
            ("completed_quests", "Vec<QuestId>"),
            ("tracked_quest", "Option<QuestId>"),
        ],
    },
    ComponentSchema {
        name: "DialogueState",
        fields: &[
            ("current_node", "Option<NodeId>"),
            ("known_topics", "HashSet<String>"),
            ("reputation", "f32"),
        ],
    },
    ComponentSchema {
        name: "CombatStats",
        fields: &[
            ("health", "f32"),
            ("max_health", "f32"),
            ("attack_power", "f32"),
            ("defense", "f32"),
            ("critical_chance", "f32"),
        ],
    },
    ComponentSchema {
        name: "Experience",
        fields: &[
            ("level", "u32"),
            ("current_xp", "u64"),
            ("next_level_xp", "u64"),
            ("skill_points", "u32"),
        ],
    },
];

// ── RTS Components ──

static RTS_SCHEMAS: [ComponentSchema; 4] = [
    ComponentSchema {
        name: "SelectableUnit",
        fields: &[
            ("is_selected", "bool"),
            ("team_id", "u8"),
            ("unit_type", "UnitType"),
            ("move_speed", "f32"),
        ],
    },
    ComponentSchema {
        name: "BuildQueue",
        fields: &[
            ("queue", "Vec<BuildOrder>"),
            ("progress", "f32"),
            ("max_queue_size", "u32"),
        ],
    },
    ComponentSchema {
        name: "ResourceGatherer",
        fields: &[
            ("resource_type", "ResourceKind"),
            ("gather_rate", "f32"),
            ("carry_capacity", "u32"),
            ("current_load", "u32"),
        ],
    },
    ComponentSchema {
        name: "FogOfWar",
        fields: &[
            ("vision_range", "f32"),
            ("reveal_radius", "f32"),
            ("is_revealed", "bool"),
        ],
    },
];

// ── Survival Components ──

static SURVIVAL_SCHEMAS: [ComponentSchema; 4] = [
    ComponentSchema {
        name: "CraftingGrid",
        fields: &[
            ("grid_size", "(u32, u32)"),
            ("slots", "Vec<CraftSlot>"),
            ("known_recipes", "Vec<RecipeId>"),
        ],
    },
    ComponentSchema {
        name: "HungerThirst",
        fields: &[
            ("hunger", "f32"),
            ("thirst", "f32"),
            ("max_hunger", "f32"),
            ("max_thirst", "f32"),
            ("decay_rate", "f32"),
        ],
    },
    ComponentSchema {
        name: "BuildingPlacer",
        fields: &[
            ("blueprint", "Option<BlueprintId>"),
            ("snap_to_grid", "bool"),
            ("rotation_step", "f32"),
            ("is_valid_placement", "bool"),
        ],
    },
    ComponentSchema {
        name: "ToolDurability",
        fields: &[
            ("durability", "f32"),
            ("max_durability", "f32"),
            ("repair_cost", "u32"),
            ("tool_tier", "u8"),
        ],
    },
];

// ── Total War Components ──

static TOTAL_WAR_SCHEMAS: [ComponentSchema; 5] = [
    ComponentSchema {
        name: "ArmyUnit",
        fields: &[
            ("unit_count", "u32"),
            ("morale", "f32"),
            ("fatigue", "f32"),
            ("formation", "FormationType"),
            ("commander", "Option<EntityId>"),
        ],
    },
    ComponentSchema {
        name: "TerritoryControl",
        fields: &[
            ("owner_faction", "FactionId"),
            ("control_points", "f32"),
            ("is_contested", "bool"),
            ("garrison_strength", "u32"),
        ],
    },
    ComponentSchema {
        name: "Diplomacy",
        fields: &[
            ("relations", "HashMap<FactionId, f32>"),
            ("treaties", "Vec<TreatyId>"),
            ("war_score", "f32"),
        ],
    },
    ComponentSchema {
        name: "FormationSlot",
        fields: &[
            ("slot_index", "u32"),
            ("offset", "(f32, f32)"),
            ("role", "FormationRole"),
        ],
    },
    ComponentSchema {
        name: "Morale",
        fields: &[
            ("value", "f32"),
            ("modifiers", "Vec<MoraleModifier>"),
            ("is_routing", "bool"),
            ("rally_threshold", "f32"),
        ],
    },
];

// ============================================================================
// STARTER ENTITIES
// ============================================================================

/// An entity that gets spawned when a gameplay preset is applied.
pub struct StarterEntity {
    pub name: &'static str,
    pub archetype: &'static str,
    pub position: [f32; 3],
    pub description: &'static str,
}

static RPG_STARTERS: [StarterEntity; 4] = [
    StarterEntity {
        name: "Player",
        archetype: "PlayerCharacter",
        position: [0.0, 0.0, 0.0],
        description: "The player character with inventory, combat stats, and quest log.",
    },
    StarterEntity {
        name: "Village Elder",
        archetype: "NPC",
        position: [10.0, 0.0, 5.0],
        description: "Quest-giving NPC with dialogue tree.",
    },
    StarterEntity {
        name: "Treasure Chest",
        archetype: "Interactable",
        position: [20.0, 0.0, -10.0],
        description: "Lootable container with randomized rewards.",
    },
    StarterEntity {
        name: "Goblin Scout",
        archetype: "Enemy",
        position: [30.0, 0.0, 15.0],
        description: "Basic enemy with patrol AI and combat stats.",
    },
];

static RTS_STARTERS: [StarterEntity; 4] = [
    StarterEntity {
        name: "Command Center",
        archetype: "Building",
        position: [0.0, 0.0, 0.0],
        description: "Main base building with build queue.",
    },
    StarterEntity {
        name: "Worker",
        archetype: "Unit",
        position: [5.0, 0.0, 0.0],
        description: "Resource-gathering unit.",
    },
    StarterEntity {
        name: "Gold Mine",
        archetype: "Resource",
        position: [30.0, 0.0, 20.0],
        description: "Harvestable gold resource node.",
    },
    StarterEntity {
        name: "Scout",
        archetype: "Unit",
        position: [5.0, 0.0, 5.0],
        description: "Fast unit for map exploration.",
    },
];

static SURVIVAL_STARTERS: [StarterEntity; 4] = [
    StarterEntity {
        name: "Survivor",
        archetype: "PlayerCharacter",
        position: [0.0, 0.0, 0.0],
        description: "Player with hunger, thirst, and crafting grid.",
    },
    StarterEntity {
        name: "Campfire",
        archetype: "Placeable",
        position: [3.0, 0.0, 0.0],
        description: "Warmth source and basic cooking station.",
    },
    StarterEntity {
        name: "Berry Bush",
        archetype: "Harvestable",
        position: [8.0, 0.0, 4.0],
        description: "Renewable food source.",
    },
    StarterEntity {
        name: "Rock Deposit",
        archetype: "Harvestable",
        position: [12.0, 0.0, -6.0],
        description: "Mineable stone resource.",
    },
];

static TOTAL_WAR_STARTERS: [StarterEntity; 4] = [
    StarterEntity {
        name: "General",
        archetype: "Commander",
        position: [0.0, 0.0, 0.0],
        description: "Army commander with morale and formation buffs.",
    },
    StarterEntity {
        name: "Infantry Regiment",
        archetype: "ArmyUnit",
        position: [5.0, 0.0, 0.0],
        description: "Front-line melee unit with formation slots.",
    },
    StarterEntity {
        name: "Archer Company",
        archetype: "ArmyUnit",
        position: [-5.0, 0.0, -10.0],
        description: "Ranged unit with volley fire capability.",
    },
    StarterEntity {
        name: "Supply Wagon",
        archetype: "Support",
        position: [0.0, 0.0, -15.0],
        description: "Logistics unit that reduces fatigue.",
    },
];

// ============================================================================
// UI HELPER — render preset selector
// ============================================================================

/// Render a gameplay preset selector panel. Returns the newly selected preset
/// if the user clicked one.
pub fn show_preset_selector(ui: &mut egui::Ui, current: GameplayPreset) -> Option<GameplayPreset> {
    let mut result = None;

    ui.label(egui::RichText::new("Gameplay Genre").size(14.0).strong());
    ui.add_space(4.0);

    for preset in GameplayPreset::ALL {
        let is_selected = *preset == current;
        let header = format!("{} {}", preset.icon(), preset.name());
        let rt = if is_selected {
            egui::RichText::new(header)
                .strong()
                .color(egui::Color32::from_rgb(80, 200, 120))
        } else {
            egui::RichText::new(header).color(egui::Color32::from_rgb(190, 190, 200))
        };

        if ui.selectable_label(is_selected, rt).clicked() {
            result = Some(*preset);
        }
    }

    // Show details of current preset
    if current != GameplayPreset::Custom {
        ui.add_space(6.0);
        ui.group(|ui| {
            ui.label(
                egui::RichText::new(format!("{} {}", current.icon(), current.name()))
                    .strong()
                    .size(13.0),
            );
            ui.label(current.description());
            ui.add_space(4.0);
            ui.label(format!("Camera: {}", current.camera_mode()));
            ui.label(format!(
                "Components: {}",
                current
                    .component_schemas()
                    .iter()
                    .map(|s| s.name)
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
            ui.label(format!(
                "Starter entities: {}",
                current
                    .starter_entities()
                    .iter()
                    .map(|e| e.name)
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        });
    }

    result
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_presets_have_names_and_descriptions() {
        for p in GameplayPreset::ALL {
            assert!(!p.name().is_empty());
            assert!(!p.description().is_empty());
            assert!(!p.camera_mode().is_empty());
        }
    }

    #[test]
    fn custom_preset_has_no_schemas_or_starters() {
        assert!(GameplayPreset::Custom.component_schemas().is_empty());
        assert!(GameplayPreset::Custom.starter_entities().is_empty());
    }

    #[test]
    fn rpg_has_expected_components() {
        let schemas = GameplayPreset::OpenWorldRPG.component_schemas();
        let names: Vec<&str> = schemas.iter().map(|s| s.name).collect();
        assert!(names.contains(&"Inventory"));
        assert!(names.contains(&"CombatStats"));
        assert!(names.contains(&"Experience"));
    }

    #[test]
    fn rts_has_expected_components() {
        let schemas = GameplayPreset::TopDownRTS.component_schemas();
        let names: Vec<&str> = schemas.iter().map(|s| s.name).collect();
        assert!(names.contains(&"SelectableUnit"));
        assert!(names.contains(&"FogOfWar"));
    }

    #[test]
    fn survival_has_expected_components() {
        let schemas = GameplayPreset::SurvivalBuilder.component_schemas();
        let names: Vec<&str> = schemas.iter().map(|s| s.name).collect();
        assert!(names.contains(&"CraftingGrid"));
        assert!(names.contains(&"HungerThirst"));
    }

    #[test]
    fn total_war_has_expected_components() {
        let schemas = GameplayPreset::TotalWarSimulation.component_schemas();
        let names: Vec<&str> = schemas.iter().map(|s| s.name).collect();
        assert!(names.contains(&"ArmyUnit"));
        assert!(names.contains(&"Diplomacy"));
        assert!(names.contains(&"Morale"));
    }

    #[test]
    fn every_preset_has_recommended_template() {
        for p in GameplayPreset::ALL {
            let _ = p.recommended_template();
        }
    }

    #[test]
    fn starter_entities_have_valid_positions() {
        for p in GameplayPreset::ALL {
            for e in p.starter_entities() {
                assert!(!e.name.is_empty());
                assert!(!e.archetype.is_empty());
                // Positions should be finite
                for &c in &e.position {
                    assert!(c.is_finite());
                }
            }
        }
    }
}
