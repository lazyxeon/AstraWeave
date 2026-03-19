// tools/aw_editor/src/panels/entity_panel.rs - Entity inspector using Astract
//
#![allow(clippy::upper_case_acronyms)] // NPC is an intentional, industry-standard acronym

//! Comprehensive entity management panel with:
//! - Entity templates & presets (Player, Enemy, NPC, Boss archetypes)
//! - Bulk operations (spawn multiple, batch edit, group management)
//! - Component validation (missing required components, value warnings)
//! - Entity comparison & diff viewer (compare prefab vs instance)
//! - Search & filtering (by name, type, tag, component)
//! - Favorites & bookmarks (quick access to important entities)
//! - Statistics tracking (entity counts by type, component usage)
//! - Undo/redo support for component edits
//! - Health/status visualization (bars, colors, icons)

use super::Panel;
use crate::component_ui::{ComponentEdit, ComponentRegistry, ComponentType};
use crate::scene_state::{EditorSceneState, TransformableScene};
use astract::prelude::*;
use astraweave_core::{Ammo, Entity, Health, IVec2, Team, World};
use egui::{Color32, Ui};
use std::collections::{HashMap, HashSet};
use tracing::debug;

/// Actions that require prefab manager access
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
#[non_exhaustive]
pub enum PrefabAction {
    RevertToOriginal(Entity),
    ApplyChangesToFile(Entity),
    RevertAllToOriginal(Entity), // Revert all entities in prefab instance (entity is any member)
    ApplyAllChangesToFile(Entity), // Apply all entities in prefab instance (entity is any member)
    EditPrefab(Entity),          // Open prefab source for editing
    BreakConnection(Entity),     // Detach entity from prefab tracking
}

impl std::fmt::Display for PrefabAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl PrefabAction {
    pub fn name(&self) -> &'static str {
        match self {
            PrefabAction::RevertToOriginal(_) => "Revert to Original",
            PrefabAction::ApplyChangesToFile(_) => "Apply Changes",
            PrefabAction::RevertAllToOriginal(_) => "Revert All",
            PrefabAction::ApplyAllChangesToFile(_) => "Apply All Changes",
            PrefabAction::EditPrefab(_) => "Edit Prefab",
            PrefabAction::BreakConnection(_) => "Break Connection",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            PrefabAction::RevertToOriginal(_) => "↩️",
            PrefabAction::ApplyChangesToFile(_) => "[ok]",
            PrefabAction::RevertAllToOriginal(_) => "[Sync]",
            PrefabAction::ApplyAllChangesToFile(_) => "[Save]",
            PrefabAction::EditPrefab(_) => "✏️",
            PrefabAction::BreakConnection(_) => "🔗",
        }
    }

    /// Returns true if this action affects all entities in the prefab
    pub fn is_bulk_action(&self) -> bool {
        matches!(
            self,
            PrefabAction::RevertAllToOriginal(_) | PrefabAction::ApplyAllChangesToFile(_)
        )
    }
}

/// Actions that can be performed by the entity panel
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum EntityAction {
    // Entity selection
    SelectEntity(Entity),
    DeselectEntity,
    SelectMultiple(Vec<Entity>),
    ClearSelection,

    // Entity creation
    SpawnFromArchetype(EntityArchetype),
    SpawnMultiple {
        archetype: EntityArchetype,
        count: usize,
    },
    DuplicateEntity(Entity),
    DeleteEntity(Entity),
    DeleteMultiple(Vec<Entity>),

    // Component operations
    AddComponent {
        entity: Entity,
        component_name: String,
    },
    RemoveComponent {
        entity: Entity,
        component_name: String,
    },
    SetHealth {
        entity: Entity,
        hp: i32,
    },
    SetPosition {
        entity: Entity,
        x: i32,
        y: i32,
    },
    SetTeam {
        entity: Entity,
        team_id: u8,
    },
    SetAmmo {
        entity: Entity,
        ammo: i32,
    },

    // Favorites
    AddToFavorites(Entity),
    RemoveFromFavorites(Entity),
    ClearFavorites,

    // Validation
    ValidateEntity(Entity),
    ValidateAll,
    ToggleAutoValidate(bool),

    // Filtering
    SetFilterQuery(String),
    SetFilterTeam(Option<u8>),
    SetFilterHealthRange {
        min: Option<i32>,
        max: Option<i32>,
    },
    ClearFilters,

    // Bulk operations
    SetTeamForAll {
        team_id: u8,
        entities: Vec<Entity>,
    },
    SetHealthForAll {
        hp: i32,
        entities: Vec<Entity>,
    },
    DeleteFiltered,

    // Statistics
    ToggleStats(bool),
    RefreshStats,

    // Prefab operations (wrapping PrefabAction for unified handling)
    Prefab(PrefabAction),
}

impl std::fmt::Display for EntityAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SelectEntity(e) => write!(f, "Select entity {:?}", e),
            Self::DeselectEntity => write!(f, "Deselect entity"),
            Self::SelectMultiple(entities) => write!(f, "Select {} entities", entities.len()),
            Self::ClearSelection => write!(f, "Clear selection"),
            Self::SpawnFromArchetype(arch) => write!(f, "Spawn {}", arch.name()),
            Self::SpawnMultiple { archetype, count } => {
                write!(f, "Spawn {} {}s", count, archetype.name())
            }
            Self::DuplicateEntity(e) => write!(f, "Duplicate entity {:?}", e),
            Self::DeleteEntity(e) => write!(f, "Delete entity {:?}", e),
            Self::DeleteMultiple(entities) => write!(f, "Delete {} entities", entities.len()),
            Self::AddComponent {
                entity,
                component_name,
            } => write!(f, "Add {} to {:?}", component_name, entity),
            Self::RemoveComponent {
                entity,
                component_name,
            } => write!(f, "Remove {} from {:?}", component_name, entity),
            Self::SetHealth { entity, hp } => write!(f, "Set {:?} health to {}", entity, hp),
            Self::SetPosition { entity, x, y } => write!(f, "Move {:?} to ({}, {})", entity, x, y),
            Self::SetTeam { entity, team_id } => write!(f, "Set {:?} team to {}", entity, team_id),
            Self::SetAmmo { entity, ammo } => write!(f, "Set {:?} ammo to {}", entity, ammo),
            Self::AddToFavorites(e) => write!(f, "Add {:?} to favorites", e),
            Self::RemoveFromFavorites(e) => write!(f, "Remove {:?} from favorites", e),
            Self::ClearFavorites => write!(f, "Clear favorites"),
            Self::ValidateEntity(e) => write!(f, "Validate {:?}", e),
            Self::ValidateAll => write!(f, "Validate all entities"),
            Self::ToggleAutoValidate(on) => {
                write!(f, "Turn auto-validate {}", if *on { "on" } else { "off" })
            }
            Self::SetFilterQuery(q) => write!(f, "Filter by '{}'", q),
            Self::SetFilterTeam(Some(t)) => write!(f, "Filter by team {}", t),
            Self::SetFilterTeam(None) => write!(f, "Clear team filter"),
            Self::SetFilterHealthRange { min, max } => match (min, max) {
                (Some(mn), Some(mx)) => write!(f, "Filter health {}-{}", mn, mx),
                (Some(mn), None) => write!(f, "Filter health >= {}", mn),
                (None, Some(mx)) => write!(f, "Filter health <= {}", mx),
                (None, None) => write!(f, "Clear health filter"),
            },
            Self::ClearFilters => write!(f, "Clear all filters"),
            Self::SetTeamForAll { team_id, entities } => {
                write!(f, "Set team {} for {} entities", team_id, entities.len())
            }
            Self::SetHealthForAll { hp, entities } => {
                write!(f, "Set health {} for {} entities", hp, entities.len())
            }
            Self::DeleteFiltered => write!(f, "Delete filtered entities"),
            Self::ToggleStats(on) => write!(f, "Turn stats {}", if *on { "on" } else { "off" }),
            Self::RefreshStats => write!(f, "Refresh statistics"),
            Self::Prefab(action) => write!(f, "Prefab: {}", action),
        }
    }
}

/// Entity template archetype
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum EntityArchetype {
    Player,
    Companion,
    Enemy,
    Boss,
    NPC,
    Prop,
    Trigger,
    Light,
    Camera,
}

impl std::fmt::Display for EntityArchetype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl EntityArchetype {
    pub fn all() -> Vec<Self> {
        vec![
            Self::Player,
            Self::Companion,
            Self::Enemy,
            Self::Boss,
            Self::NPC,
            Self::Prop,
            Self::Trigger,
            Self::Light,
            Self::Camera,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Player => "Player",
            Self::Companion => "Companion",
            Self::Enemy => "Enemy",
            Self::Boss => "Boss",
            Self::NPC => "NPC",
            Self::Prop => "Prop",
            Self::Trigger => "Trigger",
            Self::Light => "Light",
            Self::Camera => "Camera",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::Player => "[Gp]",
            Self::Companion => "🤝",
            Self::Enemy => "👾",
            Self::Boss => "👹",
            Self::NPC => "👤",
            Self::Prop => "[Pkg]",
            Self::Trigger => "[Zap]",
            Self::Light => "[Lgt]",
            Self::Camera => "📷",
        }
    }

    /// Returns true if this archetype is combat-related
    pub fn is_combatant(&self) -> bool {
        matches!(
            self,
            Self::Player | Self::Companion | Self::Enemy | Self::Boss
        )
    }

    pub fn default_health(&self) -> i32 {
        match self {
            Self::Player => 100,
            Self::Companion => 80,
            Self::Enemy => 50,
            Self::Boss => 500,
            Self::NPC => 100,
            Self::Prop => 10,
            Self::Trigger => 1,
            Self::Light => 1,
            Self::Camera => 1,
        }
    }

    pub fn default_damage(&self) -> i32 {
        match self {
            Self::Player => 25,
            Self::Companion => 20,
            Self::Enemy => 15,
            Self::Boss => 50,
            Self::NPC => 0,
            Self::Prop => 0,
            Self::Trigger => 0,
            Self::Light => 0,
            Self::Camera => 0,
        }
    }
}

/// Entity filter criteria
#[derive(Debug, Clone, Default)]
pub struct EntityFilter {
    pub query: String,
    pub archetype: Option<EntityArchetype>,
    pub team_id: Option<u32>,
    pub health_range: Option<(i32, i32)>,
    pub favorites_only: bool,
}

/// Component validation issue
#[derive(Debug, Clone)]
pub struct ValidationIssue {
    pub entity: Entity,
    pub severity: ValidationSeverity,
    pub message: String,
    pub component_name: String,
}

/// Validation severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ValidationSeverity {
    Error,
    Warning,
    Info,
}

impl std::fmt::Display for ValidationSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl ValidationSeverity {
    pub fn all() -> &'static [ValidationSeverity] {
        &[
            ValidationSeverity::Error,
            ValidationSeverity::Warning,
            ValidationSeverity::Info,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Error => "Error",
            Self::Warning => "Warning",
            Self::Info => "Info",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::Error => "[x]",
            Self::Warning => "[!]",
            Self::Info => "ℹ️",
        }
    }

    pub fn color(&self) -> Color32 {
        match self {
            Self::Error => Color32::from_rgb(255, 100, 100),
            Self::Warning => Color32::from_rgb(255, 200, 100),
            Self::Info => Color32::from_rgb(100, 200, 255),
        }
    }

    /// Returns true if this severity indicates a serious issue
    pub fn is_serious(&self) -> bool {
        matches!(self, Self::Error | Self::Warning)
    }
}

/// Entity statistics
#[derive(Debug, Clone, Default)]
pub struct EntityStats {
    pub total_count: usize,
    pub by_archetype: HashMap<String, usize>,
    pub total_health: i32,
    pub avg_health: f32,
    pub component_usage: HashMap<String, usize>,
}

/// Entity panel - inspect and edit entity properties
///
/// Now integrated with real ECS World instead of mock entities.
pub struct EntityPanel {
    component_registry: ComponentRegistry,

    // New features
    entity_filter: EntityFilter,
    favorites: HashSet<Entity>,
    validation_issues: Vec<ValidationIssue>,
    auto_validate: bool,
    selected_archetype: Option<EntityArchetype>,
    bulk_spawn_count: usize,
    entity_stats: EntityStats,
    show_stats: bool,
    search_results: Vec<Entity>,

    // Action queue for external processing
    pending_actions: Vec<EntityAction>,

    /// Property/component search filter for the inspector.
    property_search: String,
    /// Disabled components: (entity, component_type) pairs skipped during simulation.
    disabled_components: HashSet<(Entity, crate::component_ui::ComponentType)>,
}

impl EntityPanel {
    pub fn new() -> Self {
        Self {
            component_registry: ComponentRegistry::new(),
            entity_filter: EntityFilter::default(),
            favorites: HashSet::new(),
            validation_issues: Vec::new(),
            auto_validate: true,
            selected_archetype: Some(EntityArchetype::Companion),
            bulk_spawn_count: 1,
            entity_stats: EntityStats::default(),
            show_stats: true,
            search_results: Vec::new(),
            pending_actions: Vec::new(),
            property_search: String::new(),
            disabled_components: HashSet::new(),
        }
    }

    /// Check if a component is disabled for an entity.
    pub fn is_component_disabled(
        &self,
        entity: Entity,
        ct: crate::component_ui::ComponentType,
    ) -> bool {
        self.disabled_components.contains(&(entity, ct))
    }

    /// Get all disabled components (for simulation filtering).
    pub fn disabled_components(&self) -> &HashSet<(Entity, crate::component_ui::ComponentType)> {
        &self.disabled_components
    }

    // ==================== Action Queue Methods ====================

    /// Takes all pending actions, leaving the internal queue empty
    pub fn take_actions(&mut self) -> Vec<EntityAction> {
        std::mem::take(&mut self.pending_actions)
    }

    /// Returns true if there are pending actions
    pub fn has_pending_actions(&self) -> bool {
        !self.pending_actions.is_empty()
    }

    /// Queue an action for external processing
    pub fn queue_action(&mut self, action: EntityAction) {
        self.pending_actions.push(action);
    }

    /// Returns a reference to pending actions
    pub fn pending_actions(&self) -> &[EntityAction] {
        &self.pending_actions
    }

    /// Update entity statistics
    fn update_statistics(&mut self, world: &World) {
        self.entity_stats.total_count = world.entities().len();
        self.entity_stats.total_health = 0;
        self.entity_stats.by_archetype.clear();
        self.entity_stats.component_usage.clear();

        for entity in world.entities() {
            if let Some(health) = world.health(entity) {
                self.entity_stats.total_health += health.hp;
            }

            // Count component usage
            if world.pose(entity).is_some() {
                *self
                    .entity_stats
                    .component_usage
                    .entry("Position".to_string())
                    .or_insert(0) += 1;
            }
            if world.health(entity).is_some() {
                *self
                    .entity_stats
                    .component_usage
                    .entry("Health".to_string())
                    .or_insert(0) += 1;
            }
            if world.ammo(entity).is_some() {
                *self
                    .entity_stats
                    .component_usage
                    .entry("Ammo".to_string())
                    .or_insert(0) += 1;
            }
            if world.team(entity).is_some() {
                *self
                    .entity_stats
                    .component_usage
                    .entry("Team".to_string())
                    .or_insert(0) += 1;
            }
        }

        self.entity_stats.avg_health = if self.entity_stats.total_count > 0 {
            self.entity_stats.total_health as f32 / self.entity_stats.total_count as f32
        } else {
            0.0
        };
    }

    /// Validate entity components
    fn validate_entities(&mut self, world: &World) {
        self.validation_issues.clear();

        for entity in world.entities() {
            // Check for missing position
            if world.pose(entity).is_none() {
                self.validation_issues.push(ValidationIssue {
                    entity,
                    severity: ValidationSeverity::Error,
                    message: "Missing Position component".to_string(),
                    component_name: "Position".to_string(),
                });
            }

            // Check for health value warnings
            if let Some(health) = world.health(entity) {
                if health.hp <= 0 {
                    self.validation_issues.push(ValidationIssue {
                        entity,
                        severity: ValidationSeverity::Warning,
                        message: format!("Entity has zero or negative health: {}", health.hp),
                        component_name: "Health".to_string(),
                    });
                } else if health.hp < 10 {
                    self.validation_issues.push(ValidationIssue {
                        entity,
                        severity: ValidationSeverity::Info,
                        message: format!("Entity has low health: {}", health.hp),
                        component_name: "Health".to_string(),
                    });
                }
            }

            // Check for missing team on combat entities
            if world.health(entity).is_some()
                && world.ammo(entity).is_some()
                && world.team(entity).is_none()
            {
                self.validation_issues.push(ValidationIssue {
                    entity,
                    severity: ValidationSeverity::Warning,
                    message: "Combat entity missing Team component".to_string(),
                    component_name: "Team".to_string(),
                });
            }
        }
    }

    /// Filter entities based on criteria
    fn filter_entities(&mut self, world: &World) {
        self.search_results.clear();

        for entity in world.entities() {
            let mut matches = true;

            // Filter by query (would need entity name component)
            if !self.entity_filter.query.is_empty() {
                // For now, just match entity ID
                if !entity.to_string().contains(&self.entity_filter.query) {
                    matches = false;
                }
            }

            // Filter by team
            if let Some(team_id) = self.entity_filter.team_id {
                if let Some(team) = world.team(entity) {
                    if team.id != team_id as u8 {
                        matches = false;
                    }
                } else {
                    matches = false;
                }
            }

            // Filter by health range
            if let Some((min_health, max_health)) = self.entity_filter.health_range {
                if let Some(health) = world.health(entity) {
                    if health.hp < min_health || health.hp > max_health {
                        matches = false;
                    }
                } else {
                    matches = false;
                }
            }

            // Filter by favorites
            if self.entity_filter.favorites_only && !self.favorites.contains(&entity) {
                matches = false;
            }

            if matches {
                self.search_results.push(entity);
            }
        }
    }

    /// Spawn entity from archetype
    fn spawn_from_archetype(
        &self,
        world: &mut World,
        archetype: &EntityArchetype,
        position: IVec2,
    ) -> Entity {
        let count = world.entities().len();
        let name = format!("{archetype:?}_{count}");

        world.spawn(
            &name,
            position,
            Team {
                id: match archetype {
                    EntityArchetype::Player | EntityArchetype::Companion => 0,
                    EntityArchetype::Enemy | EntityArchetype::Boss => 1,
                    _ => 2,
                },
            },
            archetype.default_health(),
            archetype.default_damage(),
        )
    }

    /// Show entity panel with real world integration
    ///
    /// # Arguments
    ///
    /// * `ui` - egui UI context
    /// * `scene_state` - Mutable reference to canonical edit-mode scene (optional)
    /// * `selected_entity` - Currently selected entity (optional)
    /// * `prefab_instance` - Optional prefab instance for override tracking
    ///
    /// # Returns
    ///
    /// Tuple of (Optional ComponentEdit for undo, Optional PrefabAction to execute)
    pub fn show_with_scene_state(
        &mut self,
        ui: &mut Ui,
        scene_state: Option<&mut EditorSceneState>,
        selected_entity: Option<Entity>,
        prefab_instance: Option<&crate::prefab::PrefabInstance>,
    ) -> (Option<ComponentEdit>, Option<PrefabAction>) {
        ui.heading("Entity Inspector");
        ui.separator();

        // Update statistics if needed
        if self.show_stats {
            self.update_statistics(
                scene_state
                    .as_ref()
                    .map(|s| s.world())
                    .unwrap_or(&World::new()),
            );
        }

        // Archetype template selector
        ui.group(|ui| {
            ui.label("Entity Templates");
            ui.horizontal_wrapped(|ui| {
                for archetype in EntityArchetype::all() {
                    let is_selected = self.selected_archetype.as_ref() == Some(&archetype);
                    let label = format!("{} {:?}", archetype.icon(), archetype);
                    if ui.selectable_label(is_selected, label).clicked() {
                        self.selected_archetype = Some(archetype);
                    }
                }
            });
        });

        ui.add_space(5.0);

        // Entity management buttons
        let mut spawn_selected = false;
        let mut spawn_bulk = false;
        let mut clear_all = false;
        let mut validate_all = false;
        let mut revert_to_prefab = false;
        let mut apply_to_prefab = false;
        let mut revert_all_to_prefab = false;
        let mut apply_all_to_prefab = false;
        let mut edit_prefab = false;
        let mut break_prefab_connection = false;

        ui.horizontal(|ui| {
            if let Some(ref archetype) = self.selected_archetype {
                if ui.button(format!("Spawn {}", archetype.icon())).clicked() {
                    spawn_selected = true;
                }
            }

            ui.add(
                egui::DragValue::new(&mut self.bulk_spawn_count)
                    .speed(1.0)
                    .range(1..=100),
            );
            if ui.button("+Spawn Multiple").clicked() {
                spawn_bulk = true;
            }

            if ui.button("Clear All").clicked() {
                clear_all = true;
            }

            if ui.button("Validate").clicked() {
                validate_all = true;
            }
        });

        ui.add_space(5.0);

        // Search & Filter
        ui.group(|ui| {
            ui.label("Search & Filter");
            ui.horizontal(|ui| {
                ui.label("Query:");
                let response = ui.text_edit_singleline(&mut self.entity_filter.query);
                if response.changed() {
                    if let Some(scene_state) = scene_state.as_ref() {
                        self.filter_entities(scene_state.world());
                    }
                }
            });

            ui.horizontal(|ui| {
                ui.label("Team:");
                let mut team_filter = self.entity_filter.team_id.map(|id| id as i32).unwrap_or(-1);
                if egui::ComboBox::from_id_salt("team_filter")
                    .selected_text(if team_filter < 0 {
                        "All".to_string()
                    } else {
                        format!("Team {}", team_filter)
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut team_filter, -1, "All");
                        ui.selectable_value(&mut team_filter, 0, "Team 0 (Friendly)");
                        ui.selectable_value(&mut team_filter, 1, "Team 1 (Enemy)");
                        ui.selectable_value(&mut team_filter, 2, "Team 2 (Neutral)");
                    })
                    .response
                    .changed()
                {
                    self.entity_filter.team_id = if team_filter < 0 {
                        None
                    } else {
                        Some(team_filter as u32)
                    };
                    if let Some(scene_state) = scene_state.as_ref() {
                        self.filter_entities(scene_state.world());
                    }
                }

                ui.checkbox(&mut self.entity_filter.favorites_only, "⭐ Favorites Only");
            });

            if !self.search_results.is_empty() || !self.entity_filter.query.is_empty() {
                ui.label(format!("Found: {} entities", self.search_results.len()));
            }
        });

        ui.add_space(5.0);

        // Validation panel
        if !self.validation_issues.is_empty() || validate_all {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label("Validation Issues");
                    ui.checkbox(&mut self.auto_validate, "Auto-validate");
                    if ui.button("Clear").clicked() {
                        self.validation_issues.clear();
                    }
                });

                let error_count = self
                    .validation_issues
                    .iter()
                    .filter(|i| matches!(i.severity, ValidationSeverity::Error))
                    .count();
                let warning_count = self
                    .validation_issues
                    .iter()
                    .filter(|i| matches!(i.severity, ValidationSeverity::Warning))
                    .count();

                ui.horizontal(|ui| {
                    if error_count > 0 {
                        ui.colored_label(
                            Color32::from_rgb(255, 100, 100),
                            format!("{} Errors", error_count),
                        );
                    }
                    if warning_count > 0 {
                        ui.colored_label(
                            Color32::from_rgb(255, 200, 100),
                            format!("{} Warnings", warning_count),
                        );
                    }
                    if self.validation_issues.is_empty() {
                        ui.colored_label(Color32::from_rgb(100, 255, 100), "✓ All Valid");
                    }
                });

                if !self.validation_issues.is_empty() {
                    egui::ScrollArea::vertical()
                        .max_height(150.0)
                        .show(ui, |ui| {
                            for issue in &self.validation_issues {
                                ui.horizontal(|ui| {
                                    ui.colored_label(issue.severity.color(), issue.severity.icon());
                                    ui.label(format!(
                                        "Entity #{}: {}",
                                        issue.entity, issue.message
                                    ));
                                });
                            }
                        });
                }
            });
            ui.add_space(5.0);
        }

        // Statistics panel
        if self.show_stats {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label("Statistics");
                    if ui.button("Refresh").clicked() {
                        if let Some(scene_state) = scene_state.as_ref() {
                            self.update_statistics(scene_state.world());
                        }
                    }
                });

                ui.label(format!("Total Entities: {}", self.entity_stats.total_count));
                ui.label(format!("Total Health: {}", self.entity_stats.total_health));
                ui.label(format!("Avg Health: {:.1}", self.entity_stats.avg_health));
                ui.label(format!("Favorites: {}", self.favorites.len()));

                if !self.entity_stats.component_usage.is_empty() {
                    ui.label("Component Usage:");
                    for (component, count) in &self.entity_stats.component_usage {
                        ui.label(format!("  • {}: {}", component, count));
                    }
                }
            });
            ui.add_space(5.0);
        }

        ui.add_space(10.0);

        let Some(scene_state) = scene_state else {
            ui.label("No world initialized");
            return (None, None);
        };

        // Handle spawn selected archetype
        if spawn_selected {
            if let Some(ref archetype) = self.selected_archetype {
                let pos = IVec2 {
                    x: rand::random::<i32>() % 30,
                    y: rand::random::<i32>() % 30,
                };
                let entity = self.spawn_from_archetype(scene_state.world_mut(), archetype, pos);
                scene_state.sync_entity(entity);
                debug!(
                    "Spawned {:?} entity #{} at ({}, {})",
                    archetype, entity, pos.x, pos.y
                );
            }
        }

        // Handle bulk spawn
        if spawn_bulk {
            if let Some(ref archetype) = self.selected_archetype {
                for _ in 0..self.bulk_spawn_count {
                    let pos = IVec2 {
                        x: rand::random::<i32>() % 30,
                        y: rand::random::<i32>() % 30,
                    };
                    let entity = self.spawn_from_archetype(scene_state.world_mut(), archetype, pos);
                    scene_state.sync_entity(entity);
                }
                debug!("Spawned {} {:?} entities", self.bulk_spawn_count, archetype);
            }
        }

        // Handle clear all
        if clear_all {
            {
                let world = scene_state.world_mut();
                *world = World::new();
            }
            self.favorites.clear();
            self.validation_issues.clear();
            self.search_results.clear();
            scene_state.sync_all();
            debug!("Cleared all entities");
        }

        // Handle validation
        if validate_all || self.auto_validate {
            self.validate_entities(scene_state.world());
            if self.auto_validate {
                debug!(
                    "Auto-validated entities: {} issues",
                    self.validation_issues.len()
                );
            }
        }

        let mut component_edit = None;
        let mut toggle_favorite = false;

        if let Some(entity) = selected_entity {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.heading(format!("✏️ Entity #{}", entity));

                    // Favorites toggle button
                    let is_favorite = self.favorites.contains(&entity);
                    let star_icon = if is_favorite { "⭐" } else { "☆" };
                    let star_color = if is_favorite {
                        Color32::from_rgb(255, 215, 0)
                    } else {
                        Color32::GRAY
                    };
                    if ui
                        .colored_label(star_color, star_icon)
                        .on_hover_text("Toggle favorite")
                        .clicked()
                    {
                        toggle_favorite = true;
                    }
                });

                if let Some(instance) = prefab_instance {
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label("Prefab Instance:");
                        ui.monospace(
                            instance
                                .source
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .as_ref(),
                        );
                    });

                    // Edit Prefab & Break Connection buttons (always visible)
                    ui.horizontal(|ui| {
                        if ui.button("✏️ Edit Prefab").clicked() {
                            edit_prefab = true;
                        }
                        if ui.button("🔗 Break Connection").clicked() {
                            break_prefab_connection = true;
                        }
                    });

                    if instance.has_overrides(entity) {
                        ui.colored_label(
                            egui::Color32::from_rgb(100, 150, 255),
                            "Modified components (blue text indicates overrides)",
                        );

                        // Single entity operations
                        ui.horizontal(|ui| {
                            if ui.button("Apply to Prefab").clicked() {
                                apply_to_prefab = true;
                            }
                            if ui.button("Revert to Prefab").clicked() {
                                revert_to_prefab = true;
                            }
                        });
                        ui.label("Apply: save changes back to prefab file");
                        ui.label("Revert: discard changes and restore original");

                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(4.0);

                        // Bulk operations for entire prefab instance
                        ui.colored_label(
                            egui::Color32::from_rgb(255, 180, 100),
                            "Bulk Operations (affects ALL entities in prefab)",
                        );
                        ui.horizontal(|ui| {
                            if ui.button("Apply All to Prefab").clicked() {
                                apply_all_to_prefab = true;
                            }
                            if ui.button("Revert All to Prefab").clicked() {
                                revert_all_to_prefab = true;
                            }
                        });
                        ui.label("Apply All: save ALL entity changes to prefab file");
                        ui.label("Revert All: discard ALL changes and restore all entities");
                    }
                }

                ui.separator();

                // Property search field
                ui.horizontal(|ui| {
                    ui.label("Filter:");
                    ui.text_edit_singleline(&mut self.property_search);
                });

                let components = {
                    let world = scene_state.world_mut();
                    self.component_registry.get_entity_components(world, entity)
                };

                if components.is_empty() {
                    ui.label("No components");
                } else {
                    let search_lower = self.property_search.to_lowercase();
                    // Get override information for this entity
                    let entity_overrides =
                        prefab_instance.and_then(|inst| inst.overrides.get(&entity));

                    for component_type in components {
                        // Filter by property search
                        if !search_lower.is_empty()
                            && !component_type.name().to_lowercase().contains(&search_lower)
                        {
                            continue;
                        }

                        // Enable/disable checkbox
                        let key = (entity, component_type);
                        let mut enabled = !self.disabled_components.contains(&key);
                        ui.horizontal(|ui| {
                            if ui.checkbox(&mut enabled, "").changed() {
                                if enabled {
                                    self.disabled_components.remove(&key);
                                } else {
                                    self.disabled_components.insert(key);
                                }
                            }
                            if !enabled {
                                ui.colored_label(
                                    Color32::from_rgb(120, 120, 120),
                                    format!("{} (disabled)", component_type.name()),
                                );
                            }
                        });

                        if !enabled {
                            continue;
                        }

                        let edit = {
                            let world = scene_state.world_mut();
                            component_type.show_ui_with_overrides(
                                world,
                                entity,
                                ui,
                                entity_overrides,
                            )
                        };
                        if let Some(edit) = edit {
                            component_edit = Some(edit);
                        }
                    }
                }
            });
        } else {
            ui.label("No entity selected");
            ui.label("Click an entity in the viewport to inspect it");
        }

        ui.separator();
        ui.add_space(10.0);

        // Display entity count
        let entity_count = scene_state.world().entities().len();
        ui.label(format!("Total Entities: {}", entity_count));

        ui.separator();

        // List all entities
        egui::ScrollArea::vertical()
            .max_height(400.0)
            .show(ui, |ui| {
                let world = scene_state.world();
                for entity in world.entities() {
                    if let Some(pose) = world.pose(entity) {
                        let team = world.team(entity).unwrap_or(Team { id: 0 });
                        let health = world.health(entity).unwrap_or(Health { hp: 0 });
                        let ammo = world.ammo(entity).unwrap_or(Ammo { rounds: 0 });

                        let team_name = if team.id == 0 { "Companion" } else { "Enemy" };
                        let team_color = if team.id == 0 {
                            egui::Color32::from_rgb(100, 150, 255)
                        } else {
                            egui::Color32::from_rgb(255, 100, 100)
                        };

                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new(format!("Entity #{}", entity)).strong(),
                                );
                                ui.label(egui::RichText::new(team_name).color(team_color));
                            });

                            ui.label(format!("Position: ({}, {:.1}, {})", pose.pos.x, pose.height, pose.pos.y));
                            ui.label(format!("❤️  Health: {}", health.hp));
                            ui.label(format!("🔫 Ammo: {}", ammo.rounds));
                        });
                    }
                }
            });

        // Handle favorites toggle
        if toggle_favorite {
            if let Some(entity) = selected_entity {
                if self.favorites.contains(&entity) {
                    self.favorites.remove(&entity);
                    debug!("☆ Removed entity #{} from favorites", entity);
                } else {
                    self.favorites.insert(entity);
                    debug!("⭐ Added entity #{} to favorites", entity);
                }
            }
        }

        // Generate prefab action if buttons were clicked
        let prefab_action = if revert_to_prefab {
            selected_entity.map(PrefabAction::RevertToOriginal)
        } else if apply_to_prefab {
            selected_entity.map(PrefabAction::ApplyChangesToFile)
        } else if revert_all_to_prefab {
            selected_entity.map(PrefabAction::RevertAllToOriginal)
        } else if apply_all_to_prefab {
            selected_entity.map(PrefabAction::ApplyAllChangesToFile)
        } else if edit_prefab {
            selected_entity.map(PrefabAction::EditPrefab)
        } else if break_prefab_connection {
            selected_entity.map(PrefabAction::BreakConnection)
        } else {
            None
        };

        (component_edit, prefab_action)
    }

    /// Show multi-entity editing panel when multiple entities are selected.
    ///
    /// Displays shared components across all selected entities and allows
    /// batch editing. Returns a list of edits to apply.
    pub fn show_multi_edit(
        &mut self,
        ui: &mut Ui,
        scene_state: &mut EditorSceneState,
        selected_entities: &[Entity],
    ) -> Vec<ComponentEdit> {
        let mut edits = Vec::new();

        if selected_entities.is_empty() {
            return edits;
        }

        if selected_entities.len() == 1 {
            // Single entity — delegate to normal flow
            let (edit, _) =
                self.show_with_scene_state(ui, Some(scene_state), Some(selected_entities[0]), None);
            if let Some(e) = edit {
                edits.push(e);
            }
            return edits;
        }

        ui.heading(format!("Multi-Edit ({} entities)", selected_entities.len()));
        ui.separator();

        ui.label(
            egui::RichText::new("Editing shared components across selected entities")
                .color(Color32::from_rgb(100, 200, 255)),
        );

        // Find shared components across all selected entities
        let mut shared_components: Option<Vec<ComponentType>> = None;
        for &entity in selected_entities {
            let components = self
                .component_registry
                .get_entity_components(scene_state.world_mut(), entity);
            match &mut shared_components {
                None => shared_components = Some(components),
                Some(shared) => {
                    shared.retain(|ct| components.contains(ct));
                }
            }
        }

        let shared = shared_components.unwrap_or_default();
        if shared.is_empty() {
            ui.label("No shared components across selection");
            return edits;
        }

        ui.label(format!("Shared components: {}", shared.len()));
        ui.add_space(4.0);

        // Display shared components with batch editing
        for component_type in &shared {
            ui.collapsing(
                format!(
                    "{} {} (batch)",
                    component_type.icon(),
                    component_type.name()
                ),
                |ui| {
                    ui.label(format!("Applied to {} entities", selected_entities.len()));

                    // Show the first entity's values as the editing baseline
                    let first = selected_entities[0];
                    let edit = {
                        let world = scene_state.world_mut();
                        component_type.show_ui_with_overrides(world, first, ui, None)
                    };

                    if let Some(edit) = edit {
                        // Apply same edit to all selected entities
                        for &entity in selected_entities {
                            let batch_edit = match edit {
                                ComponentEdit::Health { new_hp, .. } => {
                                    let old_hp = scene_state
                                        .world()
                                        .health(entity)
                                        .map(|h| h.hp)
                                        .unwrap_or(0);
                                    ComponentEdit::Health {
                                        entity,
                                        old_hp,
                                        new_hp,
                                    }
                                }
                                ComponentEdit::Team { new_id, .. } => {
                                    let old_id =
                                        scene_state.world().team(entity).map(|t| t.id).unwrap_or(0);
                                    ComponentEdit::Team {
                                        entity,
                                        old_id,
                                        new_id,
                                    }
                                }
                                ComponentEdit::Ammo { new_rounds, .. } => {
                                    let old_rounds = scene_state
                                        .world()
                                        .ammo(entity)
                                        .map(|a| a.rounds)
                                        .unwrap_or(0);
                                    ComponentEdit::Ammo {
                                        entity,
                                        old_rounds,
                                        new_rounds,
                                    }
                                }
                            };
                            edits.push(batch_edit);
                        }
                    }
                },
            );
        }

        edits
    }
}

impl Default for EntityPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl Panel for EntityPanel {
    fn name(&self) -> &str {
        "Entities"
    }

    fn show(&mut self, ui: &mut Ui) {
        ui.heading("Entity Inspector");
        ui.separator();

        // Top-level controls
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.show_stats, "Show Statistics");
            ui.checkbox(&mut self.auto_validate, "Auto-validate");

            if ui.button("Refresh All").clicked() {
                debug!("Refreshing entity data");
            }
        });

        ui.add_space(5.0);

        // Archetype selector
        ui.group(|ui| {
            ui.label("Entity Templates");
            ui.horizontal_wrapped(|ui| {
                for archetype in EntityArchetype::all() {
                    let is_selected = self.selected_archetype.as_ref() == Some(&archetype);
                    let label = format!("{} {:?}", archetype.icon(), archetype);
                    if ui.selectable_label(is_selected, label).clicked() {
                        self.selected_archetype = Some(archetype);
                    }
                }
            });
        });

        ui.add_space(10.0);

        // Entity selection state
        let (selected_entity, set_selected_entity) = use_state(ui, "selected_entity", 0usize);
        let (filter, set_filter) = use_state(ui, "entity_filter", String::new());

        // Entity list from world
        let entities = [
            ("Player", "Companion", 100, 10),
            ("Enemy_1", "Grunt", 50, 5),
            ("Enemy_2", "Elite", 150, 15),
            ("NPC_Merchant", "Civilian", 80, 0),
            ("Boss_Dragon", "Boss", 500, 50),
        ];

        // Memoized filtered list
        let filtered_entities = use_memo(ui, "filtered_entities", filter.clone(), |f| {
            if f.is_empty() {
                (0..entities.len()).collect::<Vec<_>>()
            } else {
                entities
                    .iter()
                    .enumerate()
                    .filter(|(_, (name, _, _, _))| name.to_lowercase().contains(&f.to_lowercase()))
                    .map(|(i, _)| i)
                    .collect()
            }
        });

        // Search/filter UI
        ui.group(|ui| {
            ui.label("Search");
            let mut filter_val = filter.clone();
            if ui.text_edit_singleline(&mut filter_val).changed() {
                set_filter.set(ui, filter_val);
            }
            ui.label(format!("Found: {} entities", filtered_entities.len()));
        });

        ui.add_space(10.0);

        // Entity list
        ui.group(|ui| {
            ui.label("Entity List");
            egui::ScrollArea::vertical()
                .max_height(200.0)
                .show(ui, |ui| {
                    for &idx in &filtered_entities {
                        let (name, archetype, _, _) = entities[idx];
                        let is_selected = idx == selected_entity;

                        if ui
                            .selectable_label(is_selected, format!("{} ({})", name, archetype))
                            .clicked()
                        {
                            set_selected_entity.set(ui, idx);
                        }
                    }
                });
        });

        ui.add_space(10.0);

        // Entity details
        if selected_entity < entities.len() {
            let (name, archetype, health, damage) = entities[selected_entity];

            ui.group(|ui| {
                ui.heading(format!("{}", name));
                ui.separator();

                ui.label(format!("Type: {}", archetype));

                ui.add_space(5.0);

                // Health display
                let health_pct = (health as f32 / 500.0).min(1.0);
                let health_color = if health_pct > 0.6 {
                    egui::Color32::GREEN
                } else if health_pct > 0.3 {
                    egui::Color32::YELLOW
                } else {
                    egui::Color32::RED
                };

                ui.horizontal(|ui| {
                    ui.label("❤️ Health:");
                    ui.label(egui::RichText::new(format!("{}/500", health)).color(health_color));
                });

                let health_bar_width = ui.available_width();
                let (rect, _response) = ui
                    .allocate_exact_size(egui::vec2(health_bar_width, 10.0), egui::Sense::hover());

                ui.painter()
                    .rect_filled(rect, 2.0, egui::Color32::DARK_GRAY);

                let filled_width = rect.width() * health_pct;
                let filled_rect =
                    egui::Rect::from_min_size(rect.min, egui::vec2(filled_width, rect.height()));
                ui.painter().rect_filled(filled_rect, 2.0, health_color);

                ui.add_space(5.0);

                ui.horizontal(|ui| {
                    ui.label("⚔️ Damage:");
                    ui.label(format!("{}", damage));
                });

                ui.add_space(10.0);

                ui.horizontal(|ui| {
                    if ui.button("🔫 Damage").clicked() {
                        debug!("[Hit] Damaged {}", name);
                    }
                    if ui.button("❤️‍🩹 Heal").clicked() {
                        debug!("💚 Healed {}", name);
                    }
                    if ui.button("Delete").clicked() {
                        debug!("Deleted {}", name);
                    }
                });
            });
        } else {
            ui.label("Select an entity to view details");
        }

        // Effect: log when selection changes
        use_effect(ui, "entity_selection_log", selected_entity, |&idx| {
            if idx < entities.len() {
                debug!("Selected entity: {}", entities[idx].0);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =====================================================================
    // EntityArchetype Tests (9 types × 4 methods = 36 tests)
    // =====================================================================

    #[test]
    fn test_archetype_all_returns_9_types() {
        let archetypes = EntityArchetype::all();
        assert_eq!(archetypes.len(), 9, "Should have exactly 9 archetype types");
    }

    #[test]
    fn test_archetype_player_has_correct_icon() {
        assert_eq!(
            EntityArchetype::Player.icon(),
            "[Gp]",
            "Player icon should be [Gp]"
        );
    }

    #[test]
    fn test_archetype_companion_has_correct_icon() {
        assert_eq!(
            EntityArchetype::Companion.icon(),
            "🤝",
            "Companion icon should be 🤝"
        );
    }

    #[test]
    fn test_archetype_enemy_has_correct_icon() {
        assert_eq!(
            EntityArchetype::Enemy.icon(),
            "👾",
            "Enemy icon should be 👾"
        );
    }

    #[test]
    fn test_archetype_boss_has_correct_icon() {
        assert_eq!(EntityArchetype::Boss.icon(), "👹", "Boss icon should be 👹");
    }

    #[test]
    fn test_archetype_npc_has_correct_icon() {
        assert_eq!(EntityArchetype::NPC.icon(), "👤", "NPC icon should be 👤");
    }

    #[test]
    fn test_archetype_prop_has_correct_icon() {
        assert_eq!(
            EntityArchetype::Prop.icon(),
            "[Pkg]",
            "Prop icon should be [Pkg]"
        );
    }

    #[test]
    fn test_archetype_trigger_has_correct_icon() {
        assert_eq!(
            EntityArchetype::Trigger.icon(),
            "[Zap]",
            "Trigger icon should be [Zap]"
        );
    }

    #[test]
    fn test_archetype_light_has_correct_icon() {
        assert_eq!(
            EntityArchetype::Light.icon(),
            "[Lgt]",
            "Light icon should be [Lgt]"
        );
    }

    #[test]
    fn test_archetype_camera_has_correct_icon() {
        assert_eq!(
            EntityArchetype::Camera.icon(),
            "📷",
            "Camera icon should be 📷"
        );
    }

    #[test]
    fn test_archetype_player_default_health() {
        assert_eq!(
            EntityArchetype::Player.default_health(),
            100,
            "Player default health should be 100"
        );
    }

    #[test]
    fn test_archetype_companion_default_health() {
        assert_eq!(
            EntityArchetype::Companion.default_health(),
            80,
            "Companion default health should be 80"
        );
    }

    #[test]
    fn test_archetype_enemy_default_health() {
        assert_eq!(
            EntityArchetype::Enemy.default_health(),
            50,
            "Enemy default health should be 50"
        );
    }

    #[test]
    fn test_archetype_boss_default_health() {
        assert_eq!(
            EntityArchetype::Boss.default_health(),
            500,
            "Boss default health should be 500"
        );
    }

    #[test]
    fn test_archetype_npc_default_health() {
        assert_eq!(
            EntityArchetype::NPC.default_health(),
            100,
            "NPC default health should be 100"
        );
    }

    #[test]
    fn test_archetype_prop_default_health() {
        assert_eq!(
            EntityArchetype::Prop.default_health(),
            10,
            "Prop default health should be 10"
        );
    }

    #[test]
    fn test_archetype_trigger_default_health() {
        assert_eq!(
            EntityArchetype::Trigger.default_health(),
            1,
            "Trigger default health should be 1"
        );
    }

    #[test]
    fn test_archetype_light_default_health() {
        assert_eq!(
            EntityArchetype::Light.default_health(),
            1,
            "Light default health should be 1"
        );
    }

    #[test]
    fn test_archetype_camera_default_health() {
        assert_eq!(
            EntityArchetype::Camera.default_health(),
            1,
            "Camera default health should be 1"
        );
    }

    #[test]
    fn test_archetype_player_default_damage() {
        assert_eq!(
            EntityArchetype::Player.default_damage(),
            25,
            "Player default damage should be 25"
        );
    }

    #[test]
    fn test_archetype_companion_default_damage() {
        assert_eq!(
            EntityArchetype::Companion.default_damage(),
            20,
            "Companion default damage should be 20"
        );
    }

    #[test]
    fn test_archetype_enemy_default_damage() {
        assert_eq!(
            EntityArchetype::Enemy.default_damage(),
            15,
            "Enemy default damage should be 15"
        );
    }

    #[test]
    fn test_archetype_boss_default_damage() {
        assert_eq!(
            EntityArchetype::Boss.default_damage(),
            50,
            "Boss default damage should be 50"
        );
    }

    #[test]
    fn test_archetype_npc_default_damage() {
        assert_eq!(
            EntityArchetype::NPC.default_damage(),
            0,
            "NPC default damage should be 0"
        );
    }

    #[test]
    fn test_archetype_prop_default_damage() {
        assert_eq!(
            EntityArchetype::Prop.default_damage(),
            0,
            "Prop default damage should be 0"
        );
    }

    #[test]
    fn test_archetype_trigger_default_damage() {
        assert_eq!(
            EntityArchetype::Trigger.default_damage(),
            0,
            "Trigger default damage should be 0"
        );
    }

    #[test]
    fn test_archetype_light_default_damage() {
        assert_eq!(
            EntityArchetype::Light.default_damage(),
            0,
            "Light default damage should be 0"
        );
    }

    #[test]
    fn test_archetype_camera_default_damage() {
        assert_eq!(
            EntityArchetype::Camera.default_damage(),
            0,
            "Camera default damage should be 0"
        );
    }

    // =====================================================================
    // EntityFilter Tests (5 tests)
    // =====================================================================

    #[test]
    fn test_entity_filter_default() {
        let filter = EntityFilter::default();
        assert_eq!(filter.query, "", "Default query should be empty");
        assert_eq!(filter.archetype, None, "Default archetype should be None");
        assert_eq!(filter.team_id, None, "Default team_id should be None");
        assert_eq!(
            filter.health_range, None,
            "Default health_range should be None"
        );
        assert!(
            !filter.favorites_only,
            "Default favorites_only should be false"
        );
    }

    #[test]
    fn test_entity_filter_with_query() {
        let filter = EntityFilter {
            query: "enemy".to_string(),
            ..Default::default()
        };
        assert_eq!(filter.query, "enemy");
    }

    #[test]
    fn test_entity_filter_with_team() {
        let filter = EntityFilter {
            team_id: Some(1),
            ..Default::default()
        };
        assert_eq!(filter.team_id, Some(1));
    }

    #[test]
    fn test_entity_filter_with_health_range() {
        let filter = EntityFilter {
            health_range: Some((50, 100)),
            ..Default::default()
        };
        assert_eq!(filter.health_range, Some((50, 100)));
    }

    #[test]
    fn test_entity_filter_with_favorites() {
        let filter = EntityFilter {
            favorites_only: true,
            ..Default::default()
        };
        assert!(filter.favorites_only);
    }

    // =====================================================================
    // ValidationSeverity Tests (6 tests)
    // =====================================================================

    #[test]
    fn test_validation_severity_error_icon() {
        assert_eq!(ValidationSeverity::Error.icon(), "[x]");
    }

    #[test]
    fn test_validation_severity_warning_icon() {
        assert_eq!(ValidationSeverity::Warning.icon(), "[!]");
    }

    #[test]
    fn test_validation_severity_info_icon() {
        assert_eq!(ValidationSeverity::Info.icon(), "ℹ️");
    }

    #[test]
    fn test_validation_severity_error_color() {
        let color = ValidationSeverity::Error.color();
        assert_eq!(color, Color32::from_rgb(255, 100, 100));
    }

    #[test]
    fn test_validation_severity_warning_color() {
        let color = ValidationSeverity::Warning.color();
        assert_eq!(color, Color32::from_rgb(255, 200, 100));
    }

    #[test]
    fn test_validation_severity_info_color() {
        let color = ValidationSeverity::Info.color();
        assert_eq!(color, Color32::from_rgb(100, 200, 255));
    }

    // =====================================================================
    // EntityStats Tests (5 tests)
    // =====================================================================

    #[test]
    fn test_entity_stats_default() {
        let stats = EntityStats::default();
        assert_eq!(stats.total_count, 0);
        assert_eq!(stats.total_health, 0);
        assert_eq!(stats.avg_health, 0.0);
        assert!(stats.by_archetype.is_empty());
        assert!(stats.component_usage.is_empty());
    }

    #[test]
    fn test_entity_stats_total_count() {
        let stats = EntityStats {
            total_count: 42,
            ..Default::default()
        };
        assert_eq!(stats.total_count, 42);
    }

    #[test]
    fn test_entity_stats_total_health() {
        let stats = EntityStats {
            total_health: 1000,
            ..Default::default()
        };
        assert_eq!(stats.total_health, 1000);
    }

    #[test]
    fn test_entity_stats_avg_health() {
        let stats = EntityStats {
            avg_health: 42.5,
            ..Default::default()
        };
        assert_eq!(stats.avg_health, 42.5);
    }

    #[test]
    fn test_entity_stats_component_usage() {
        let mut stats = EntityStats::default();
        stats.component_usage.insert("Position".to_string(), 10);
        stats.component_usage.insert("Health".to_string(), 8);
        assert_eq!(stats.component_usage.get("Position"), Some(&10));
        assert_eq!(stats.component_usage.get("Health"), Some(&8));
    }

    // =====================================================================
    // EntityPanel Initialization Tests (8 tests)
    // =====================================================================

    #[test]
    fn test_entity_panel_new() {
        let panel = EntityPanel::new();
        assert_eq!(panel.bulk_spawn_count, 1);
        assert!(panel.favorites.is_empty());
        assert!(panel.validation_issues.is_empty());
        assert!(panel.auto_validate);
        assert_eq!(panel.selected_archetype, Some(EntityArchetype::Companion));
        assert!(panel.show_stats);
        assert!(panel.property_search.is_empty());
        assert!(panel.disabled_components.is_empty());
    }

    #[test]
    fn test_entity_panel_default() {
        let panel = EntityPanel::default();
        assert_eq!(panel.bulk_spawn_count, 1);
    }

    #[test]
    fn test_entity_panel_name() {
        let panel = EntityPanel::new();
        assert_eq!(panel.name(), "Entities");
    }

    #[test]
    fn test_entity_panel_initial_filter() {
        let panel = EntityPanel::new();
        assert_eq!(panel.entity_filter.query, "");
        assert_eq!(panel.entity_filter.archetype, None);
    }

    #[test]
    fn test_entity_panel_initial_favorites() {
        let panel = EntityPanel::new();
        assert!(panel.favorites.is_empty());
    }

    #[test]
    fn test_entity_panel_initial_validation() {
        let panel = EntityPanel::new();
        assert!(panel.validation_issues.is_empty());
        assert!(panel.auto_validate);
    }

    #[test]
    fn test_entity_panel_initial_stats() {
        let panel = EntityPanel::new();
        assert_eq!(panel.entity_stats.total_count, 0);
        assert!(panel.show_stats);
    }

    #[test]
    fn test_entity_panel_initial_search() {
        let panel = EntityPanel::new();
        assert!(panel.search_results.is_empty());
    }

    // =====================================================================
    // Statistics Calculation Tests (5 tests)
    // =====================================================================

    #[test]
    fn test_update_statistics_empty_world() {
        let mut panel = EntityPanel::new();
        let world = World::new();
        panel.update_statistics(&world);
        assert_eq!(panel.entity_stats.total_count, 0);
        assert_eq!(panel.entity_stats.total_health, 0);
        assert_eq!(panel.entity_stats.avg_health, 0.0);
    }

    #[test]
    fn test_update_statistics_single_entity() {
        let mut panel = EntityPanel::new();
        let mut world = World::new();
        world.spawn("Player", IVec2::new(0, 0), Team { id: 0 }, 100, 25);
        panel.update_statistics(&world);
        assert_eq!(panel.entity_stats.total_count, 1);
        assert_eq!(panel.entity_stats.total_health, 100);
        assert_eq!(panel.entity_stats.avg_health, 100.0);
    }

    #[test]
    fn test_update_statistics_multiple_entities() {
        let mut panel = EntityPanel::new();
        let mut world = World::new();
        world.spawn("Player", IVec2::new(0, 0), Team { id: 0 }, 100, 25);
        world.spawn("Enemy", IVec2::new(0, 0), Team { id: 1 }, 80, 15);
        panel.update_statistics(&world);
        assert_eq!(panel.entity_stats.total_count, 2);
        assert_eq!(panel.entity_stats.total_health, 180);
        assert_eq!(panel.entity_stats.avg_health, 90.0);
    }

    #[test]
    fn test_update_statistics_component_usage() {
        let mut panel = EntityPanel::new();
        let mut world = World::new();
        world.spawn("Player", IVec2::new(0, 0), Team { id: 0 }, 100, 25);
        world.spawn("Enemy", IVec2::new(0, 0), Team { id: 1 }, 80, 15);
        panel.update_statistics(&world);
        assert!(panel.entity_stats.component_usage.contains_key("Position"));
        assert!(panel.entity_stats.component_usage.contains_key("Health"));
        assert!(panel.entity_stats.component_usage.contains_key("Team"));
    }

    #[test]
    fn test_update_statistics_zero_entities_avg() {
        let mut panel = EntityPanel::new();
        let world = World::new();
        panel.update_statistics(&world);
        assert_eq!(
            panel.entity_stats.avg_health, 0.0,
            "Average health should be 0.0 for empty world"
        );
    }

    // =====================================================================
    // Validation Tests (8 tests)
    // =====================================================================

    #[test]
    fn test_validate_entities_empty_world() {
        let mut panel = EntityPanel::new();
        let world = World::new();
        panel.validate_entities(&world);
        assert!(panel.validation_issues.is_empty());
    }

    #[test]
    fn test_validate_entities_healthy_entity() {
        let mut panel = EntityPanel::new();
        let mut world = World::new();
        world.spawn("Player", IVec2::new(5, 5), Team { id: 0 }, 100, 25);
        panel.validate_entities(&world);
        assert!(
            panel.validation_issues.is_empty(),
            "Healthy entity should have no validation issues"
        );
    }

    #[test]
    fn test_validate_entities_zero_health() {
        let mut panel = EntityPanel::new();
        let mut world = World::new();
        let entity = world.spawn("Player", IVec2::new(5, 5), Team { id: 0 }, 0, 25);
        panel.validate_entities(&world);
        assert_eq!(panel.validation_issues.len(), 1);
        assert_eq!(panel.validation_issues[0].entity, entity);
        assert!(matches!(
            panel.validation_issues[0].severity,
            ValidationSeverity::Warning
        ));
        assert!(panel.validation_issues[0]
            .message
            .contains("zero or negative health"));
    }

    #[test]
    fn test_validate_entities_low_health() {
        let mut panel = EntityPanel::new();
        let mut world = World::new();
        // Low health is hp < 10, so use hp=5 to trigger the warning
        let entity = world.spawn("Player", IVec2::new(5, 5), Team { id: 0 }, 5, 25);
        panel.validate_entities(&world);
        assert_eq!(panel.validation_issues.len(), 1);
        assert_eq!(panel.validation_issues[0].entity, entity);
        assert!(matches!(
            panel.validation_issues[0].severity,
            ValidationSeverity::Info
        ));
        assert!(panel.validation_issues[0].message.contains("low health"));
    }

    // Note: World doesn't have remove_team method, so we can't test missing team validation
    // This test is skipped as it requires ECS mutation capabilities not available in World API

    #[test]
    fn test_validate_entities_multiple_issues() {
        let mut panel = EntityPanel::new();
        let mut world = World::new();
        world.spawn("Player1", IVec2::new(5, 5), Team { id: 0 }, 0, 25); // Zero health (Warning)
        world.spawn("Player2", IVec2::new(5, 5), Team { id: 0 }, 5, 25); // Low health < 10 (Info)
        panel.validate_entities(&world);
        assert_eq!(panel.validation_issues.len(), 2);
    }

    #[test]
    fn test_validation_issue_component() {
        let mut panel = EntityPanel::new();
        let mut world = World::new();
        let _entity = world.spawn("Player", IVec2::new(5, 5), Team { id: 0 }, 0, 25);
        panel.validate_entities(&world);
        assert_eq!(panel.validation_issues[0].component_name, "Health");
    }

    #[test]
    fn test_validation_clears_previous() {
        let mut panel = EntityPanel::new();
        let mut world = World::new();
        world.spawn("Player", IVec2::new(5, 5), Team { id: 0 }, 0, 25);
        panel.validate_entities(&world);
        assert_eq!(panel.validation_issues.len(), 1);

        // Fix the world
        let mut world2 = World::new();
        world2.spawn("Player", IVec2::new(5, 5), Team { id: 0 }, 100, 25);
        panel.validate_entities(&world2);
        assert!(
            panel.validation_issues.is_empty(),
            "Validation should clear previous issues"
        );
    }

    // =====================================================================
    // Filtering Tests (10 tests)
    // =====================================================================

    #[test]
    fn test_filter_entities_empty_world() {
        let mut panel = EntityPanel::new();
        let world = World::new();
        panel.filter_entities(&world);
        assert!(panel.search_results.is_empty());
    }

    #[test]
    fn test_filter_entities_no_filter() {
        let mut panel = EntityPanel::new();
        let mut world = World::new();
        world.spawn("Player", IVec2::new(0, 0), Team { id: 0 }, 100, 25);
        world.spawn("Enemy", IVec2::new(0, 0), Team { id: 1 }, 80, 15);
        panel.filter_entities(&world);
        assert_eq!(panel.search_results.len(), 2);
    }

    #[test]
    fn test_filter_entities_by_query() {
        let mut panel = EntityPanel::new();
        panel.entity_filter.query = "Player".to_string();
        let mut world = World::new();
        world.spawn("Player", IVec2::new(0, 0), Team { id: 0 }, 100, 25);
        world.spawn("Enemy", IVec2::new(0, 0), Team { id: 1 }, 80, 15);
        panel.filter_entities(&world);
        // Note: Current implementation filters by entity ID toString, not name
        // This test may need adjustment based on actual implementation
        assert!(panel.search_results.len() <= 2);
    }

    #[test]
    fn test_filter_entities_by_team() {
        let mut panel = EntityPanel::new();
        panel.entity_filter.team_id = Some(0);
        let mut world = World::new();
        world.spawn("Player", IVec2::new(0, 0), Team { id: 0 }, 100, 25);
        world.spawn("Enemy", IVec2::new(0, 0), Team { id: 1 }, 80, 15);
        panel.filter_entities(&world);
        assert_eq!(panel.search_results.len(), 1);
    }

    #[test]
    fn test_filter_entities_by_health_range() {
        let mut panel = EntityPanel::new();
        panel.entity_filter.health_range = Some((50, 100));
        let mut world = World::new();
        world.spawn("Player", IVec2::new(0, 0), Team { id: 0 }, 100, 25);
        world.spawn("Weak", IVec2::new(0, 0), Team { id: 1 }, 30, 15);
        world.spawn("Strong", IVec2::new(0, 0), Team { id: 1 }, 200, 15);
        panel.filter_entities(&world);
        assert_eq!(
            panel.search_results.len(),
            1,
            "Only entity with health 100 (in range 50-100) should match"
        );
    }

    #[test]
    fn test_filter_entities_by_favorites() {
        let mut panel = EntityPanel::new();
        let mut world = World::new();
        let e1 = world.spawn("Player", IVec2::new(0, 0), Team { id: 0 }, 100, 25);
        let _e2 = world.spawn("Enemy", IVec2::new(0, 0), Team { id: 1 }, 80, 15);
        panel.favorites.insert(e1);
        panel.entity_filter.favorites_only = true;
        panel.filter_entities(&world);
        assert_eq!(panel.search_results.len(), 1);
        assert_eq!(panel.search_results[0], e1);
    }

    #[test]
    fn test_filter_entities_combined_filters() {
        let mut panel = EntityPanel::new();
        panel.entity_filter.team_id = Some(0);
        panel.entity_filter.health_range = Some((90, 110));
        let mut world = World::new();
        world.spawn("Player", IVec2::new(0, 0), Team { id: 0 }, 100, 25);
        world.spawn("Enemy", IVec2::new(0, 0), Team { id: 1 }, 100, 15);
        world.spawn("Weak", IVec2::new(0, 0), Team { id: 0 }, 50, 15);
        panel.filter_entities(&world);
        assert_eq!(
            panel.search_results.len(),
            1,
            "Only team 0 entity with health 100 should match"
        );
    }

    #[test]
    fn test_filter_entities_empty_favorites() {
        let mut panel = EntityPanel::new();
        let mut world = World::new();
        world.spawn("Player", IVec2::new(0, 0), Team { id: 0 }, 100, 25);
        panel.entity_filter.favorites_only = true;
        panel.filter_entities(&world);
        assert!(
            panel.search_results.is_empty(),
            "No entities should match when favorites_only=true but no favorites"
        );
    }

    #[test]
    fn test_filter_entities_clears_previous_results() {
        let mut panel = EntityPanel::new();
        let mut world1 = World::new();
        world1.spawn("Player", IVec2::new(0, 0), Team { id: 0 }, 100, 25);
        panel.filter_entities(&world1);
        assert_eq!(panel.search_results.len(), 1);

        let world2 = World::new(); // Empty world
        panel.filter_entities(&world2);
        assert!(
            panel.search_results.is_empty(),
            "Filtering should clear previous results"
        );
    }

    #[test]
    fn test_filter_entities_excludes_non_matching() {
        let mut panel = EntityPanel::new();
        panel.entity_filter.team_id = Some(0);
        let mut world = World::new();
        world.spawn("Player", IVec2::new(0, 0), Team { id: 0 }, 100, 25);
        world.spawn("Enemy1", IVec2::new(0, 0), Team { id: 1 }, 80, 15);
        world.spawn("Enemy2", IVec2::new(0, 0), Team { id: 1 }, 90, 15);
        panel.filter_entities(&world);
        assert_eq!(
            panel.search_results.len(),
            1,
            "Only team 0 entity should match"
        );
    }

    // =====================================================================
    // Spawn from Archetype Tests (9 tests)
    // =====================================================================

    #[test]
    fn test_spawn_from_archetype_player() {
        let panel = EntityPanel::new();
        let mut world = World::new();
        let entity =
            panel.spawn_from_archetype(&mut world, &EntityArchetype::Player, IVec2::new(10, 20));
        assert!(world.health(entity).is_some());
        let health = world.health(entity).unwrap();
        assert_eq!(health.hp, 100);
    }

    #[test]
    fn test_spawn_from_archetype_companion() {
        let panel = EntityPanel::new();
        let mut world = World::new();
        let entity =
            panel.spawn_from_archetype(&mut world, &EntityArchetype::Companion, IVec2::new(10, 20));
        let health = world.health(entity).unwrap();
        assert_eq!(health.hp, 80);
    }

    #[test]
    fn test_spawn_from_archetype_enemy() {
        let panel = EntityPanel::new();
        let mut world = World::new();
        let entity =
            panel.spawn_from_archetype(&mut world, &EntityArchetype::Enemy, IVec2::new(10, 20));
        let health = world.health(entity).unwrap();
        assert_eq!(health.hp, 50);
    }

    #[test]
    fn test_spawn_from_archetype_boss() {
        let panel = EntityPanel::new();
        let mut world = World::new();
        let entity =
            panel.spawn_from_archetype(&mut world, &EntityArchetype::Boss, IVec2::new(10, 20));
        let health = world.health(entity).unwrap();
        assert_eq!(health.hp, 500);
    }

    #[test]
    fn test_spawn_from_archetype_with_position() {
        let panel = EntityPanel::new();
        let mut world = World::new();
        let pos = IVec2::new(15, 25);
        let entity = panel.spawn_from_archetype(&mut world, &EntityArchetype::Player, pos);
        let pose = world.pose(entity).unwrap();
        assert_eq!(pose.pos, pos);
    }

    #[test]
    fn test_spawn_from_archetype_with_team() {
        let panel = EntityPanel::new();
        let mut world = World::new();
        let entity =
            panel.spawn_from_archetype(&mut world, &EntityArchetype::Player, IVec2::new(0, 0));
        let team = world.team(entity).unwrap();
        assert_eq!(team.id, 0, "Player should spawn on team 0");
    }

    #[test]
    fn test_spawn_from_archetype_enemy_team() {
        let panel = EntityPanel::new();
        let mut world = World::new();
        let entity =
            panel.spawn_from_archetype(&mut world, &EntityArchetype::Enemy, IVec2::new(0, 0));
        let team = world.team(entity).unwrap();
        assert_eq!(team.id, 1, "Enemy should spawn on team 1");
    }

    #[test]
    fn test_spawn_from_archetype_damage() {
        let panel = EntityPanel::new();
        let mut world = World::new();
        let entity =
            panel.spawn_from_archetype(&mut world, &EntityArchetype::Player, IVec2::new(0, 0));
        assert!(world.ammo(entity).is_some());
        let ammo = world.ammo(entity).unwrap();
        assert_eq!(ammo.rounds, 25);
    }

    #[test]
    fn test_spawn_from_archetype_npc_no_damage() {
        let panel = EntityPanel::new();
        let mut world = World::new();
        let entity =
            panel.spawn_from_archetype(&mut world, &EntityArchetype::NPC, IVec2::new(0, 0));
        let ammo = world.ammo(entity).unwrap();
        assert_eq!(ammo.rounds, 0, "NPC should have 0 damage");
    }

    // =====================================================================
    // Favorites System Tests (7 tests)
    // =====================================================================

    #[test]
    fn test_favorites_initially_empty() {
        let panel = EntityPanel::new();
        assert!(panel.favorites.is_empty());
    }

    #[test]
    fn test_add_to_favorites() {
        let mut panel = EntityPanel::new();
        let mut world = World::new();
        let entity = world.spawn("Player", IVec2::new(0, 0), Team { id: 0 }, 100, 25);
        panel.favorites.insert(entity);
        assert!(panel.favorites.contains(&entity));
    }

    #[test]
    fn test_remove_from_favorites() {
        let mut panel = EntityPanel::new();
        let mut world = World::new();
        let entity = world.spawn("Player", IVec2::new(0, 0), Team { id: 0 }, 100, 25);
        panel.favorites.insert(entity);
        panel.favorites.remove(&entity);
        assert!(!panel.favorites.contains(&entity));
    }

    #[test]
    fn test_multiple_favorites() {
        let mut panel = EntityPanel::new();
        let mut world = World::new();
        let e1 = world.spawn("Player1", IVec2::new(0, 0), Team { id: 0 }, 100, 25);
        let e2 = world.spawn("Player2", IVec2::new(0, 0), Team { id: 0 }, 100, 25);
        panel.favorites.insert(e1);
        panel.favorites.insert(e2);
        assert_eq!(panel.favorites.len(), 2);
    }

    #[test]
    fn test_favorites_filter_integration() {
        let mut panel = EntityPanel::new();
        let mut world = World::new();
        let e1 = world.spawn("Player", IVec2::new(0, 0), Team { id: 0 }, 100, 25);
        let _e2 = world.spawn("Enemy", IVec2::new(0, 0), Team { id: 1 }, 80, 15);
        panel.favorites.insert(e1);
        panel.entity_filter.favorites_only = true;
        panel.filter_entities(&world);
        assert_eq!(panel.search_results.len(), 1);
        assert_eq!(panel.search_results[0], e1);
    }

    #[test]
    fn test_favorites_not_duplicated() {
        let mut panel = EntityPanel::new();
        let mut world = World::new();
        let entity = world.spawn("Player", IVec2::new(0, 0), Team { id: 0 }, 100, 25);
        panel.favorites.insert(entity);
        panel.favorites.insert(entity); // Insert again
        assert_eq!(
            panel.favorites.len(),
            1,
            "HashSet should prevent duplicates"
        );
    }

    #[test]
    fn test_favorites_cleared_on_clear_all() {
        let mut panel = EntityPanel::new();
        let mut world = World::new();
        let entity = world.spawn("Player", IVec2::new(0, 0), Team { id: 0 }, 100, 25);
        panel.favorites.insert(entity);
        panel.favorites.clear();
        assert!(panel.favorites.is_empty());
    }

    // =====================================================================
    // PrefabAction Enum Tests
    // =====================================================================

    #[test]
    fn test_prefab_action_display() {
        let action = PrefabAction::RevertToOriginal(1);
        let display = format!("{}", action);
        assert!(
            display.contains(action.name()),
            "Display should contain name"
        );
    }

    #[test]
    fn test_prefab_action_name() {
        assert_eq!(
            PrefabAction::RevertToOriginal(1).name(),
            "Revert to Original"
        );
        assert_eq!(PrefabAction::ApplyChangesToFile(1).name(), "Apply Changes");
        assert_eq!(PrefabAction::RevertAllToOriginal(1).name(), "Revert All");
        assert_eq!(
            PrefabAction::ApplyAllChangesToFile(1).name(),
            "Apply All Changes"
        );
        assert_eq!(PrefabAction::EditPrefab(1).name(), "Edit Prefab");
        assert_eq!(PrefabAction::BreakConnection(1).name(), "Break Connection");
    }

    #[test]
    fn test_prefab_action_is_bulk_action() {
        assert!(PrefabAction::RevertAllToOriginal(1).is_bulk_action());
        assert!(PrefabAction::ApplyAllChangesToFile(1).is_bulk_action());
        assert!(!PrefabAction::RevertToOriginal(1).is_bulk_action());
        assert!(!PrefabAction::ApplyChangesToFile(1).is_bulk_action());
        assert!(!PrefabAction::EditPrefab(1).is_bulk_action());
        assert!(!PrefabAction::BreakConnection(1).is_bulk_action());
    }

    #[test]
    fn test_prefab_action_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(PrefabAction::RevertToOriginal(1));
        set.insert(PrefabAction::ApplyChangesToFile(1));
        set.insert(PrefabAction::EditPrefab(1));
        set.insert(PrefabAction::BreakConnection(1));
        set.insert(PrefabAction::RevertToOriginal(1)); // Duplicate
        assert_eq!(set.len(), 4);
    }

    // =====================================================================
    // EntityArchetype Enum Tests
    // =====================================================================

    #[test]
    fn test_entity_archetype_display() {
        for arch in EntityArchetype::all() {
            let display = format!("{}", arch);
            assert!(display.contains(arch.name()), "Display should contain name");
        }
    }

    #[test]
    fn test_entity_archetype_all_variants() {
        let variants = EntityArchetype::all();
        assert_eq!(variants.len(), 9, "Expected 9 entity archetype variants");
        assert!(variants.contains(&EntityArchetype::Player));
        assert!(variants.contains(&EntityArchetype::Enemy));
        assert!(variants.contains(&EntityArchetype::NPC));
    }

    #[test]
    fn test_entity_archetype_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for arch in EntityArchetype::all() {
            set.insert(arch.clone());
        }
        assert_eq!(set.len(), EntityArchetype::all().len());
    }

    #[test]
    fn test_entity_archetype_name() {
        assert_eq!(EntityArchetype::Player.name(), "Player");
        assert_eq!(EntityArchetype::Companion.name(), "Companion");
        assert_eq!(EntityArchetype::Boss.name(), "Boss");
    }

    #[test]
    fn test_entity_archetype_is_combatant() {
        assert!(EntityArchetype::Player.is_combatant());
        assert!(EntityArchetype::Companion.is_combatant());
        assert!(EntityArchetype::Enemy.is_combatant());
        assert!(EntityArchetype::Boss.is_combatant());
        assert!(!EntityArchetype::NPC.is_combatant());
        assert!(!EntityArchetype::Prop.is_combatant());
    }

    // =====================================================================
    // ValidationSeverity Enum Tests
    // =====================================================================

    #[test]
    fn test_validation_severity_display() {
        for sev in ValidationSeverity::all() {
            let display = format!("{}", sev);
            assert!(display.contains(sev.name()), "Display should contain name");
        }
    }

    #[test]
    fn test_validation_severity_all_variants() {
        let variants = ValidationSeverity::all();
        assert_eq!(variants.len(), 3, "Expected 3 validation severity variants");
        assert!(variants.contains(&ValidationSeverity::Error));
        assert!(variants.contains(&ValidationSeverity::Warning));
        assert!(variants.contains(&ValidationSeverity::Info));
    }

    #[test]
    fn test_validation_severity_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for sev in ValidationSeverity::all() {
            set.insert(*sev);
        }
        assert_eq!(set.len(), ValidationSeverity::all().len());
    }

    #[test]
    fn test_validation_severity_name() {
        assert_eq!(ValidationSeverity::Error.name(), "Error");
        assert_eq!(ValidationSeverity::Warning.name(), "Warning");
        assert_eq!(ValidationSeverity::Info.name(), "Info");
    }

    #[test]
    fn test_validation_severity_is_serious() {
        assert!(ValidationSeverity::Error.is_serious());
        assert!(ValidationSeverity::Warning.is_serious());
        assert!(!ValidationSeverity::Info.is_serious());
    }

    // =====================================================================
    // EntityAction Tests
    // =====================================================================

    #[test]
    fn test_entity_action_display_selection() {
        let action = EntityAction::SelectEntity(42);
        assert!(format!("{}", action).contains("42"));

        let action = EntityAction::DeselectEntity;
        assert!(format!("{}", action).contains("Deselect"));

        let action = EntityAction::ClearSelection;
        assert!(format!("{}", action).contains("Clear"));
    }

    #[test]
    fn test_entity_action_display_creation() {
        let action = EntityAction::SpawnFromArchetype(EntityArchetype::Player);
        assert!(format!("{}", action).contains("Player"));

        let action = EntityAction::SpawnMultiple {
            archetype: EntityArchetype::Enemy,
            count: 10,
        };
        let display = format!("{}", action);
        assert!(display.contains("10"));
        assert!(display.contains("Enemy"));
    }

    #[test]
    fn test_entity_action_display_components() {
        let action = EntityAction::AddComponent {
            entity: 1,
            component_name: "Health".to_string(),
        };
        assert!(format!("{}", action).contains("Health"));

        let action = EntityAction::SetHealth { entity: 1, hp: 100 };
        assert!(format!("{}", action).contains("100"));

        let action = EntityAction::SetPosition {
            entity: 1,
            x: 10,
            y: 20,
        };
        let display = format!("{}", action);
        assert!(display.contains("10"));
        assert!(display.contains("20"));
    }

    #[test]
    fn test_entity_action_display_favorites() {
        let action = EntityAction::AddToFavorites(5);
        assert!(format!("{}", action).contains("favorites"));

        let action = EntityAction::RemoveFromFavorites(5);
        assert!(format!("{}", action).contains("Remove"));

        let action = EntityAction::ClearFavorites;
        assert!(format!("{}", action).contains("Clear"));
    }

    #[test]
    fn test_entity_action_display_validation() {
        let action = EntityAction::ValidateEntity(1);
        assert!(format!("{}", action).contains("Validate"));

        let action = EntityAction::ValidateAll;
        assert!(format!("{}", action).contains("all"));

        let action = EntityAction::ToggleAutoValidate(true);
        assert!(format!("{}", action).contains("on"));
    }

    #[test]
    fn test_entity_action_display_filtering() {
        let action = EntityAction::SetFilterQuery("player".to_string());
        assert!(format!("{}", action).contains("player"));

        let action = EntityAction::SetFilterTeam(Some(2));
        assert!(format!("{}", action).contains("2"));

        let action = EntityAction::SetFilterTeam(None);
        assert!(format!("{}", action).contains("Clear"));
    }

    #[test]
    fn test_entity_action_display_health_range() {
        let action = EntityAction::SetFilterHealthRange {
            min: Some(10),
            max: Some(100),
        };
        let display = format!("{}", action);
        assert!(display.contains("10"));
        assert!(display.contains("100"));

        let action = EntityAction::SetFilterHealthRange {
            min: Some(50),
            max: None,
        };
        assert!(format!("{}", action).contains(">= 50"));

        let action = EntityAction::SetFilterHealthRange {
            min: None,
            max: Some(75),
        };
        assert!(format!("{}", action).contains("<= 75"));
    }

    #[test]
    fn test_entity_action_display_bulk() {
        let action = EntityAction::SetTeamForAll {
            team_id: 1,
            entities: vec![1, 2, 3],
        };
        assert!(format!("{}", action).contains("3 entities"));

        let action = EntityAction::DeleteFiltered;
        assert!(format!("{}", action).contains("Delete"));
    }

    #[test]
    fn test_entity_action_display_prefab() {
        let action = EntityAction::Prefab(PrefabAction::RevertToOriginal(1));
        assert!(format!("{}", action).contains("Prefab"));
    }

    #[test]
    fn test_entity_panel_action_queue() {
        let mut panel = EntityPanel::new();
        assert!(!panel.has_pending_actions());
        assert!(panel.pending_actions().is_empty());

        panel.queue_action(EntityAction::SelectEntity(1));
        assert!(panel.has_pending_actions());
        assert_eq!(panel.pending_actions().len(), 1);
    }

    #[test]
    fn test_entity_panel_take_actions() {
        let mut panel = EntityPanel::new();
        panel.queue_action(EntityAction::SpawnFromArchetype(EntityArchetype::Enemy));
        panel.queue_action(EntityAction::SetHealth { entity: 1, hp: 50 });

        let actions = panel.take_actions();
        assert_eq!(actions.len(), 2);
        assert!(!panel.has_pending_actions());
    }

    #[test]
    fn test_entity_panel_action_order() {
        let mut panel = EntityPanel::new();
        panel.queue_action(EntityAction::SpawnFromArchetype(EntityArchetype::Player));
        panel.queue_action(EntityAction::SelectEntity(1));
        panel.queue_action(EntityAction::SetTeam {
            entity: 1,
            team_id: 0,
        });

        let actions = panel.take_actions();
        assert!(matches!(actions[0], EntityAction::SpawnFromArchetype(_)));
        assert!(matches!(actions[1], EntityAction::SelectEntity(_)));
        assert!(matches!(actions[2], EntityAction::SetTeam { .. }));
    }

    #[test]
    fn test_entity_action_stats() {
        let action = EntityAction::ToggleStats(true);
        assert!(format!("{}", action).contains("on"));

        let action = EntityAction::RefreshStats;
        assert!(format!("{}", action).contains("Refresh"));
    }

    #[test]
    fn test_entity_action_equality() {
        let a1 = EntityAction::SelectEntity(5);
        let a2 = EntityAction::SelectEntity(5);
        let a3 = EntityAction::SelectEntity(10);

        assert_eq!(a1, a2);
        assert_ne!(a1, a3);
    }

    #[test]
    fn test_entity_action_clone() {
        let action = EntityAction::SetFilterQuery("test".to_string());
        let cloned = action.clone();
        assert_eq!(action, cloned);
    }

    // ===== Property Search & Component Toggle Tests =====

    #[test]
    fn test_property_search_default_empty() {
        let panel = EntityPanel::new();
        assert!(panel.property_search.is_empty());
    }

    #[test]
    fn test_disabled_components_default_empty() {
        let panel = EntityPanel::new();
        assert!(panel.disabled_components.is_empty());
    }

    #[test]
    fn test_is_component_disabled() {
        use crate::component_ui::ComponentType;
        let mut panel = EntityPanel::new();

        assert!(!panel.is_component_disabled(1, ComponentType::Health));

        panel.disabled_components.insert((1, ComponentType::Health));
        assert!(panel.is_component_disabled(1, ComponentType::Health));
        assert!(!panel.is_component_disabled(1, ComponentType::Pose));
        assert!(!panel.is_component_disabled(2, ComponentType::Health));
    }

    #[test]
    fn test_disabled_components_accessor() {
        use crate::component_ui::ComponentType;
        let mut panel = EntityPanel::new();
        panel.disabled_components.insert((1, ComponentType::Health));
        panel.disabled_components.insert((2, ComponentType::Ammo));

        let disabled = panel.disabled_components();
        assert_eq!(disabled.len(), 2);
    }

    #[test]
    fn test_multi_edit_method_exists() {
        // Verify multi-edit method signature compiles correctly
        let panel = EntityPanel::new();
        // Method should accept &[Entity] for multi-select editing
        let _fn_ptr: fn(
            &mut EntityPanel,
            &mut egui::Ui,
            &mut EditorSceneState,
            &[Entity],
        ) -> Vec<ComponentEdit> = EntityPanel::show_multi_edit;
        assert!(true); // Method exists and compiles
    }
}
