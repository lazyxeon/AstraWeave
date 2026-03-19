use astraweave_core::{Ammo, Entity, Health, Pose, Team, World};
use egui::Ui;

#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub enum ComponentEdit {
    Health {
        entity: Entity,
        old_hp: i32,
        new_hp: i32,
    },
    Team {
        entity: Entity,
        old_id: u8,
        new_id: u8,
    },
    Ammo {
        entity: Entity,
        old_rounds: i32,
        new_rounds: i32,
    },
}

impl ComponentEdit {
    /// Get the entity this edit applies to
    pub fn entity(&self) -> Entity {
        match self {
            ComponentEdit::Health { entity, .. } => *entity,
            ComponentEdit::Team { entity, .. } => *entity,
            ComponentEdit::Ammo { entity, .. } => *entity,
        }
    }

    /// Get the component type this edit applies to
    pub fn component_type(&self) -> ComponentType {
        match self {
            ComponentEdit::Health { .. } => ComponentType::Health,
            ComponentEdit::Team { .. } => ComponentType::Team,
            ComponentEdit::Ammo { .. } => ComponentType::Ammo,
        }
    }

    /// Get a human-readable name for this edit
    pub fn name(&self) -> &'static str {
        match self {
            ComponentEdit::Health { .. } => "Health Edit",
            ComponentEdit::Team { .. } => "Team Edit",
            ComponentEdit::Ammo { .. } => "Ammo Edit",
        }
    }

    /// Get description of the change
    pub fn description(&self) -> String {
        match self {
            ComponentEdit::Health { old_hp, new_hp, .. } => {
                format!("HP: {} -> {}", old_hp, new_hp)
            }
            ComponentEdit::Team { old_id, new_id, .. } => {
                format!("Team: {} -> {}", old_id, new_id)
            }
            ComponentEdit::Ammo {
                old_rounds,
                new_rounds,
                ..
            } => {
                format!("Ammo: {} -> {}", old_rounds, new_rounds)
            }
        }
    }

    /// Check if this is a health edit
    pub fn is_health(&self) -> bool {
        matches!(self, ComponentEdit::Health { .. })
    }

    /// Check if this is a team edit
    pub fn is_team(&self) -> bool {
        matches!(self, ComponentEdit::Team { .. })
    }

    /// Check if this is an ammo edit
    pub fn is_ammo(&self) -> bool {
        matches!(self, ComponentEdit::Ammo { .. })
    }
}

impl std::fmt::Display for ComponentEdit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.name(), self.description())
    }
}

impl std::fmt::Display for ComponentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

pub trait InspectorUI {
    fn ui(&mut self, ui: &mut Ui, label: &str) -> bool;
    fn component_name() -> &'static str
    where
        Self: Sized;
}

impl InspectorUI for Pose {
    fn ui(&mut self, ui: &mut Ui, label: &str) -> bool {
        let mut changed = false;

        ui.collapsing(label, |ui| {
            ui.horizontal(|ui| {
                ui.label("Position:");
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut self.pos.x)
                            .prefix("X: ")
                            .speed(0.1),
                    )
                    .changed();
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut self.height)
                            .prefix("H: ")
                            .speed(0.05),
                    )
                    .changed();
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut self.pos.y)
                            .prefix("Z: ")
                            .speed(0.1),
                    )
                    .changed();
            });

            ui.horizontal(|ui| {
                ui.label("Rotation:");
                let mut rotation_deg = self.rotation.to_degrees();
                if ui
                    .add(
                        egui::DragValue::new(&mut rotation_deg)
                            .suffix("°")
                            .speed(1.0),
                    )
                    .changed()
                {
                    self.rotation = rotation_deg.to_radians();
                    changed = true;
                }
            });

            ui.horizontal(|ui| {
                ui.label("Pitch:");
                let mut pitch_deg = self.rotation_x.to_degrees();
                if ui
                    .add(egui::DragValue::new(&mut pitch_deg).suffix("°").speed(1.0))
                    .changed()
                {
                    self.rotation_x = pitch_deg.to_radians();
                    changed = true;
                }
            });

            ui.horizontal(|ui| {
                ui.label("Roll:");
                let mut roll_deg = self.rotation_z.to_degrees();
                if ui
                    .add(egui::DragValue::new(&mut roll_deg).suffix("°").speed(1.0))
                    .changed()
                {
                    self.rotation_z = roll_deg.to_radians();
                    changed = true;
                }
            });

            ui.horizontal(|ui| {
                ui.label("Scale:");
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut self.scale)
                            .speed(0.01)
                            .range(0.01..=100.0),
                    )
                    .changed();
            });
        });

        changed
    }

    fn component_name() -> &'static str {
        "Pose"
    }
}

impl InspectorUI for Health {
    fn ui(&mut self, ui: &mut Ui, label: &str) -> bool {
        let mut changed = false;

        ui.collapsing(label, |ui| {
            ui.horizontal(|ui| {
                ui.label("HP:");
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut self.hp)
                            .speed(1.0)
                            .range(0..=1000),
                    )
                    .changed();

                let health_pct = (self.hp as f32 / 100.0).clamp(0.0, 1.0);
                let health_color = if health_pct > 0.6 {
                    egui::Color32::GREEN
                } else if health_pct > 0.3 {
                    egui::Color32::YELLOW
                } else {
                    egui::Color32::RED
                };

                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(100.0, 10.0), egui::Sense::hover());
                ui.painter()
                    .rect_filled(rect, 2.0, egui::Color32::DARK_GRAY);

                let filled_width = rect.width() * health_pct;
                let filled_rect =
                    egui::Rect::from_min_size(rect.min, egui::vec2(filled_width, rect.height()));
                ui.painter().rect_filled(filled_rect, 2.0, health_color);
            });
        });

        changed
    }

    fn component_name() -> &'static str {
        "Health"
    }
}

impl InspectorUI for Team {
    fn ui(&mut self, ui: &mut Ui, label: &str) -> bool {
        let mut changed = false;

        ui.collapsing(label, |ui| {
            ui.horizontal(|ui| {
                ui.label("Team ID:");
                let mut id_i32 = self.id as i32;
                if ui
                    .add(egui::DragValue::new(&mut id_i32).speed(1.0).range(0..=255))
                    .changed()
                {
                    self.id = id_i32 as u8;
                    changed = true;
                }

                let team_name = match self.id {
                    0 => "Player",
                    1 => "Companion",
                    2 => "Enemy",
                    _ => "Unknown",
                };
                ui.label(format!("({})", team_name));
            });
        });

        changed
    }

    fn component_name() -> &'static str {
        "Team"
    }
}

impl InspectorUI for Ammo {
    fn ui(&mut self, ui: &mut Ui, label: &str) -> bool {
        let mut changed = false;

        ui.collapsing(label, |ui| {
            ui.horizontal(|ui| {
                ui.label("Rounds:");
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut self.rounds)
                            .speed(1.0)
                            .range(0..=1000),
                    )
                    .changed();
            });
        });

        changed
    }

    fn component_name() -> &'static str {
        "Ammo"
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ComponentType {
    Pose,
    Health,
    Team,
    Ammo,
}

impl ComponentType {
    pub fn all() -> &'static [ComponentType] {
        &[
            ComponentType::Pose,
            ComponentType::Health,
            ComponentType::Team,
            ComponentType::Ammo,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            ComponentType::Pose => "Pose",
            ComponentType::Health => "Health",
            ComponentType::Team => "Team",
            ComponentType::Ammo => "Ammo",
        }
    }

    /// Get icon for this component type
    pub fn icon(&self) -> &'static str {
        match self {
            ComponentType::Pose => "📐",
            ComponentType::Health => "❤️",
            ComponentType::Team => "🏴",
            ComponentType::Ammo => "🔫",
        }
    }

    /// Get description for this component type
    pub fn description(&self) -> &'static str {
        match self {
            ComponentType::Pose => "Position, rotation, and scale transform",
            ComponentType::Health => "Entity health points",
            ComponentType::Team => "Team affiliation (player, companion, enemy)",
            ComponentType::Ammo => "Ammunition count",
        }
    }

    /// Check if this is a transform component
    pub fn is_transform(&self) -> bool {
        matches!(self, ComponentType::Pose)
    }

    /// Check if this is a gameplay component
    pub fn is_gameplay(&self) -> bool {
        matches!(
            self,
            ComponentType::Health | ComponentType::Team | ComponentType::Ammo
        )
    }

    pub fn has_component(&self, world: &World, entity: Entity) -> bool {
        match self {
            ComponentType::Pose => world.pose(entity).is_some(),
            ComponentType::Health => world.health(entity).is_some(),
            ComponentType::Team => world.team(entity).is_some(),
            ComponentType::Ammo => world.ammo(entity).is_some(),
        }
    }

    pub fn show_ui(&self, world: &mut World, entity: Entity, ui: &mut Ui) -> Option<ComponentEdit> {
        self.show_ui_with_overrides(world, entity, ui, None)
    }

    /// Show component UI with optional override indicators
    ///
    /// If overrides are provided, displays visual indicators (icon + colored label) for modified components
    pub fn show_ui_with_overrides(
        &self,
        world: &mut World,
        entity: Entity,
        ui: &mut Ui,
        overrides: Option<&crate::prefab::EntityOverrides>,
    ) -> Option<ComponentEdit> {
        match self {
            ComponentType::Pose => {
                if let Some(pose) = world.pose_mut(entity) {
                    let is_overridden = overrides.is_some_and(|o| o.has_pose_override());
                    let label = if is_overridden { "Pose *" } else { "Pose" };

                    if is_overridden {
                        ui.push_id("pose_override", |ui| {
                            ui.visuals_mut().override_text_color =
                                Some(egui::Color32::from_rgb(100, 150, 255));
                            pose.ui(ui, label);
                        });
                    } else {
                        pose.ui(ui, label);
                    }
                }
                None
            }
            ComponentType::Health => {
                if let Some(health) = world.health_mut(entity) {
                    let old_hp = health.hp;
                    let is_overridden = overrides.is_some_and(|o| o.has_health_override());
                    let label = if is_overridden { "Health *" } else { "Health" };

                    let changed = if is_overridden {
                        ui.push_id("health_override", |ui| {
                            ui.visuals_mut().override_text_color =
                                Some(egui::Color32::from_rgb(100, 150, 255));
                            health.ui(ui, label)
                        })
                        .inner
                    } else {
                        health.ui(ui, label)
                    };

                    if changed {
                        return Some(ComponentEdit::Health {
                            entity,
                            old_hp,
                            new_hp: health.hp,
                        });
                    }
                }
                None
            }
            ComponentType::Team => {
                if let Some(team) = world.team_mut(entity) {
                    let old_id = team.id;
                    let changed = team.ui(ui, "Team");
                    if changed {
                        return Some(ComponentEdit::Team {
                            entity,
                            old_id,
                            new_id: team.id,
                        });
                    }
                }
                None
            }
            ComponentType::Ammo => {
                if let Some(ammo) = world.ammo_mut(entity) {
                    let old_rounds = ammo.rounds;
                    let changed = ammo.ui(ui, "Ammo");
                    if changed {
                        return Some(ComponentEdit::Ammo {
                            entity,
                            old_rounds,
                            new_rounds: ammo.rounds,
                        });
                    }
                }
                None
            }
        }
    }
}

pub struct ComponentRegistry {
    types: Vec<ComponentType>,
}

impl ComponentRegistry {
    pub fn new() -> Self {
        Self {
            types: ComponentType::all().to_vec(),
        }
    }

    pub fn get_entity_components(&self, world: &World, entity: Entity) -> Vec<ComponentType> {
        self.types
            .iter()
            .filter(|ct| ct.has_component(world, entity))
            .copied()
            .collect()
    }
}

impl Default for ComponentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use astraweave_core::{IVec2, Team};

    #[test]
    fn test_component_type_all() {
        let all_types = ComponentType::all();
        assert_eq!(all_types.len(), 4);
        assert!(all_types.contains(&ComponentType::Pose));
        assert!(all_types.contains(&ComponentType::Health));
        assert!(all_types.contains(&ComponentType::Team));
        assert!(all_types.contains(&ComponentType::Ammo));
    }

    #[test]
    fn test_component_type_names() {
        assert_eq!(ComponentType::Pose.name(), "Pose");
        assert_eq!(ComponentType::Health.name(), "Health");
        assert_eq!(ComponentType::Team.name(), "Team");
        assert_eq!(ComponentType::Ammo.name(), "Ammo");
    }

    #[test]
    fn test_component_type_has_component() {
        let mut world = World::new();
        let entity = world.spawn("TestEntity", IVec2::new(0, 0), Team { id: 0 }, 100, 30);

        assert!(ComponentType::Pose.has_component(&world, entity));
        assert!(ComponentType::Health.has_component(&world, entity));
        assert!(ComponentType::Team.has_component(&world, entity));
        assert!(ComponentType::Ammo.has_component(&world, entity));
    }

    #[test]
    fn test_component_type_has_component_false_for_invalid_entity() {
        let world = World::new();
        let invalid_entity = 9999;

        assert!(!ComponentType::Pose.has_component(&world, invalid_entity));
        assert!(!ComponentType::Health.has_component(&world, invalid_entity));
        assert!(!ComponentType::Team.has_component(&world, invalid_entity));
        assert!(!ComponentType::Ammo.has_component(&world, invalid_entity));
    }

    #[test]
    fn test_component_registry_new() {
        let registry = ComponentRegistry::new();
        assert_eq!(registry.types.len(), 4);
    }

    #[test]
    fn test_component_registry_default() {
        let registry = ComponentRegistry::default();
        assert_eq!(registry.types.len(), 4);
    }

    #[test]
    fn test_component_registry_get_entity_components() {
        let mut world = World::new();
        let entity = world.spawn("TestEntity", IVec2::new(0, 0), Team { id: 0 }, 100, 30);

        let registry = ComponentRegistry::new();
        let components = registry.get_entity_components(&world, entity);

        assert_eq!(components.len(), 4);
        assert!(components.contains(&ComponentType::Pose));
        assert!(components.contains(&ComponentType::Health));
        assert!(components.contains(&ComponentType::Team));
        assert!(components.contains(&ComponentType::Ammo));
    }

    #[test]
    fn test_component_registry_get_entity_components_empty_for_invalid_entity() {
        let world = World::new();
        let invalid_entity = 9999;

        let registry = ComponentRegistry::new();
        let components = registry.get_entity_components(&world, invalid_entity);

        assert_eq!(components.len(), 0);
    }

    #[test]
    fn test_component_edit_health_values() {
        let entity = 1;
        let edit = ComponentEdit::Health {
            entity,
            old_hp: 100,
            new_hp: 50,
        };

        if let ComponentEdit::Health {
            entity: e,
            old_hp,
            new_hp,
        } = edit
        {
            assert_eq!(e, entity);
            assert_eq!(old_hp, 100);
            assert_eq!(new_hp, 50);
        } else {
            panic!("Expected Health variant");
        }
    }

    #[test]
    fn test_component_edit_team_values() {
        let entity = 2;
        let edit = ComponentEdit::Team {
            entity,
            old_id: 0,
            new_id: 2,
        };

        if let ComponentEdit::Team {
            entity: e,
            old_id,
            new_id,
        } = edit
        {
            assert_eq!(e, entity);
            assert_eq!(old_id, 0);
            assert_eq!(new_id, 2);
        } else {
            panic!("Expected Team variant");
        }
    }

    #[test]
    fn test_component_edit_ammo_values() {
        let entity = 3;
        let edit = ComponentEdit::Ammo {
            entity,
            old_rounds: 30,
            new_rounds: 15,
        };

        if let ComponentEdit::Ammo {
            entity: e,
            old_rounds,
            new_rounds,
        } = edit
        {
            assert_eq!(e, entity);
            assert_eq!(old_rounds, 30);
            assert_eq!(new_rounds, 15);
        } else {
            panic!("Expected Ammo variant");
        }
    }

    // ====================================================================
    // ComponentType New Methods Tests
    // ====================================================================

    #[test]
    fn test_component_type_icon_not_empty() {
        for ct in ComponentType::all() {
            assert!(!ct.icon().is_empty());
        }
    }

    #[test]
    fn test_component_type_description_not_empty() {
        for ct in ComponentType::all() {
            assert!(!ct.description().is_empty());
        }
    }

    #[test]
    fn test_component_type_is_transform() {
        assert!(ComponentType::Pose.is_transform());
        assert!(!ComponentType::Health.is_transform());
        assert!(!ComponentType::Team.is_transform());
        assert!(!ComponentType::Ammo.is_transform());
    }

    #[test]
    fn test_component_type_is_gameplay() {
        assert!(!ComponentType::Pose.is_gameplay());
        assert!(ComponentType::Health.is_gameplay());
        assert!(ComponentType::Team.is_gameplay());
        assert!(ComponentType::Ammo.is_gameplay());
    }

    #[test]
    fn test_component_type_display() {
        assert_eq!(format!("{}", ComponentType::Pose), "Pose");
        assert_eq!(format!("{}", ComponentType::Health), "Health");
    }

    // ====================================================================
    // ComponentEdit New Methods Tests
    // ====================================================================

    #[test]
    fn test_component_edit_entity() {
        let edit = ComponentEdit::Health {
            entity: 42,
            old_hp: 100,
            new_hp: 50,
        };
        assert_eq!(edit.entity(), 42);
    }

    #[test]
    fn test_component_edit_component_type() {
        let health = ComponentEdit::Health {
            entity: 1,
            old_hp: 100,
            new_hp: 50,
        };
        let team = ComponentEdit::Team {
            entity: 1,
            old_id: 0,
            new_id: 1,
        };
        let ammo = ComponentEdit::Ammo {
            entity: 1,
            old_rounds: 30,
            new_rounds: 15,
        };

        assert!(matches!(health.component_type(), ComponentType::Health));
        assert!(matches!(team.component_type(), ComponentType::Team));
        assert!(matches!(ammo.component_type(), ComponentType::Ammo));
    }

    #[test]
    fn test_component_edit_name() {
        let edit = ComponentEdit::Health {
            entity: 1,
            old_hp: 100,
            new_hp: 50,
        };
        assert_eq!(edit.name(), "Health Edit");
    }

    #[test]
    fn test_component_edit_description() {
        let edit = ComponentEdit::Health {
            entity: 1,
            old_hp: 100,
            new_hp: 50,
        };
        let desc = edit.description();
        assert!(desc.contains("100"));
        assert!(desc.contains("50"));
    }

    #[test]
    fn test_component_edit_is_health() {
        let edit = ComponentEdit::Health {
            entity: 1,
            old_hp: 100,
            new_hp: 50,
        };
        assert!(edit.is_health());
        assert!(!edit.is_team());
        assert!(!edit.is_ammo());
    }

    #[test]
    fn test_component_edit_is_team() {
        let edit = ComponentEdit::Team {
            entity: 1,
            old_id: 0,
            new_id: 1,
        };
        assert!(edit.is_team());
        assert!(!edit.is_health());
        assert!(!edit.is_ammo());
    }

    #[test]
    fn test_component_edit_is_ammo() {
        let edit = ComponentEdit::Ammo {
            entity: 1,
            old_rounds: 30,
            new_rounds: 15,
        };
        assert!(edit.is_ammo());
        assert!(!edit.is_health());
        assert!(!edit.is_team());
    }

    #[test]
    fn test_component_edit_display() {
        let edit = ComponentEdit::Health {
            entity: 1,
            old_hp: 100,
            new_hp: 50,
        };
        let display = format!("{}", edit);
        assert!(display.contains("Health Edit"));
        assert!(display.contains("100"));
    }
}
