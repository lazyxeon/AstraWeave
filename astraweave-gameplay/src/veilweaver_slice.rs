use anyhow::{Context, Result};
use astraweave_scene::world_partition::{Cell, CellComponentView, GridCoord};
use serde::de::DeserializeOwned;
use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct WeaveAnchorSpec {
    pub cell: GridCoord,
    pub position: [f32; 3],
    pub anchor_id: String,
    pub anchor_type: Option<String>,
    pub stability: Option<String>,
    pub echo_cost: Option<f32>,
}

#[derive(Debug, Clone)]
pub struct TriggerZoneSpec {
    pub cell: GridCoord,
    pub position: [f32; 3],
    pub trigger_id: String,
    pub shape: Option<String>,
    pub radius: Option<f32>,
    pub extents: Option<[f32; 3]>,
    /// Data-driven action string (e.g. `"zone.transition:Z4_boss_courtyard"`).
    pub action: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DecisionPromptSpec {
    pub cell: GridCoord,
    pub position: [f32; 3],
    pub trigger_id: String,
    pub options: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct EncounterTriggerSpec {
    pub cell: GridCoord,
    pub position: [f32; 3],
    pub trigger_id: String,
    pub script: Option<String>,
}

#[derive(Debug, Clone)]
pub struct EncounterCompleteSpec {
    pub cell: GridCoord,
    pub position: [f32; 3],
    pub trigger_id: String,
    pub next_cell: Option<GridCoord>,
}

#[derive(Debug, Clone)]
pub struct EffectAnchorSpec {
    pub cell: GridCoord,
    pub position: [f32; 3],
    pub trigger_id: Option<String>,
    pub effect: Option<String>,
    pub intensity: Option<f32>,
}

#[derive(Debug, Clone)]
pub struct SpawnPointSpec {
    pub cell: GridCoord,
    pub position: [f32; 3],
    pub spawn_id: String,
    pub facing: Option<[f32; 3]>,
}

#[derive(Debug, Default, Clone)]
pub struct VeilweaverSliceMetadata {
    pub anchors: Vec<WeaveAnchorSpec>,
    pub trigger_zones: Vec<TriggerZoneSpec>,
    pub decision_prompts: Vec<DecisionPromptSpec>,
    pub encounter_triggers: Vec<EncounterTriggerSpec>,
    pub encounter_completes: Vec<EncounterCompleteSpec>,
    pub effect_anchors: Vec<EffectAnchorSpec>,
    pub spawn_points: Vec<SpawnPointSpec>,
}

impl VeilweaverSliceMetadata {
    pub fn extend_cell(&mut self, cell: &Cell) -> Result<()> {
        self.anchors
            .extend(parse_components(cell, "WeaveAnchor", parse_weave_anchor)?);
        self.trigger_zones
            .extend(parse_components(cell, "TriggerZone", parse_trigger_zone)?);
        self.decision_prompts.extend(parse_components(
            cell,
            "DecisionPrompt",
            parse_decision_prompt,
        )?);
        self.encounter_triggers.extend(parse_components(
            cell,
            "EncounterTrigger",
            parse_encounter_trigger,
        )?);
        self.encounter_completes.extend(parse_components(
            cell,
            "EncounterComplete",
            parse_encounter_complete,
        )?);
        self.effect_anchors
            .extend(parse_components(cell, "EffectAnchor", parse_effect_anchor)?);
        self.spawn_points
            .extend(parse_components(cell, "SpawnPoint", parse_spawn_point)?);
        Ok(())
    }

    pub fn from_cells(cells: &[&Cell]) -> Result<Self> {
        let mut meta = Self::default();
        for cell in cells {
            meta.extend_cell(cell)?;
        }
        Ok(meta)
    }
}

fn parse_components<T, F>(cell: &Cell, component_type: &str, parser: F) -> Result<Vec<T>>
where
    F: Fn(CellComponentView<'_>, GridCoord) -> Result<T>,
{
    cell.components_of_type(component_type)
        .map(|view| parser(view, cell.coord))
        .collect()
}

fn parse_weave_anchor(view: CellComponentView<'_>, coord: GridCoord) -> Result<WeaveAnchorSpec> {
    #[derive(Deserialize)]
    struct Data {
        anchor_id: String,
        #[serde(rename = "type")]
        anchor_type: Option<String>,
        stability: Option<String>,
        echo_cost: Option<f32>,
    }
    let data: Data = deserialize_component(&view)?;
    Ok(WeaveAnchorSpec {
        cell: coord,
        position: view.entity.position,
        anchor_id: data.anchor_id,
        anchor_type: data.anchor_type,
        stability: data.stability,
        echo_cost: data.echo_cost,
    })
}

fn parse_trigger_zone(view: CellComponentView<'_>, coord: GridCoord) -> Result<TriggerZoneSpec> {
    #[derive(Deserialize)]
    struct Data {
        trigger_id: String,
        shape: Option<String>,
        radius: Option<f32>,
        extents: Option<[f32; 3]>,
        action: Option<String>,
    }
    let data: Data = deserialize_component(&view)?;
    Ok(TriggerZoneSpec {
        cell: coord,
        position: view.entity.position,
        trigger_id: data.trigger_id,
        shape: data.shape,
        radius: data.radius,
        extents: data.extents,
        action: data.action,
    })
}

fn parse_decision_prompt(
    view: CellComponentView<'_>,
    coord: GridCoord,
) -> Result<DecisionPromptSpec> {
    #[derive(Deserialize)]
    struct Data {
        trigger_id: String,
        #[serde(default)]
        options: Vec<String>,
    }
    let data: Data = deserialize_component(&view)?;
    Ok(DecisionPromptSpec {
        cell: coord,
        position: view.entity.position,
        trigger_id: data.trigger_id,
        options: data.options,
    })
}

fn parse_encounter_trigger(
    view: CellComponentView<'_>,
    coord: GridCoord,
) -> Result<EncounterTriggerSpec> {
    #[derive(Deserialize)]
    struct Data {
        trigger_id: String,
        #[serde(default)]
        script: Option<String>,
    }
    let data: Data = deserialize_component(&view)?;
    Ok(EncounterTriggerSpec {
        cell: coord,
        position: view.entity.position,
        trigger_id: data.trigger_id,
        script: data.script,
    })
}

fn parse_encounter_complete(
    view: CellComponentView<'_>,
    coord: GridCoord,
) -> Result<EncounterCompleteSpec> {
    #[derive(Deserialize)]
    struct Data {
        trigger_id: String,
        #[serde(default)]
        next_cell: Option<[i32; 3]>,
    }
    let data: Data = deserialize_component(&view)?;
    Ok(EncounterCompleteSpec {
        cell: coord,
        position: view.entity.position,
        trigger_id: data.trigger_id,
        next_cell: data.next_cell.map(|c| GridCoord::new(c[0], c[1], c[2])),
    })
}

fn parse_effect_anchor(view: CellComponentView<'_>, coord: GridCoord) -> Result<EffectAnchorSpec> {
    #[derive(Deserialize)]
    struct Data {
        #[serde(default)]
        trigger_id: Option<String>,
        #[serde(default)]
        effect: Option<String>,
        #[serde(default)]
        intensity: Option<f32>,
    }
    let data: Data = deserialize_component(&view)?;
    Ok(EffectAnchorSpec {
        cell: coord,
        position: view.entity.position,
        trigger_id: data.trigger_id,
        effect: data.effect,
        intensity: data.intensity,
    })
}

fn parse_spawn_point(view: CellComponentView<'_>, coord: GridCoord) -> Result<SpawnPointSpec> {
    #[derive(Deserialize)]
    struct Data {
        id: String,
        #[serde(default)]
        facing: Option<[f32; 3]>,
    }
    let data: Data = deserialize_component(&view)?;
    Ok(SpawnPointSpec {
        cell: coord,
        position: view.entity.position,
        spawn_id: data.id,
        facing: data.facing,
    })
}

fn deserialize_component<T>(view: &CellComponentView<'_>) -> Result<T>
where
    T: DeserializeOwned,
{
    serde_json::from_str(&view.component.data).context(format!(
        "Failed to parse component '{}' JSON",
        view.component.component_type
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    pub use crate::test_support::TestValue;
    use astraweave_asset::cell_loader::ComponentData;
    use astraweave_scene::world_partition::{Cell, CellEntityBlueprint, CellState, AABB};
    use glam::Vec3;

    fn make_cell_with_components(components: Vec<(&str, &str)>) -> Cell {
        let coord = GridCoord::new(0, 0, 0);
        let entity_blueprints = vec![CellEntityBlueprint {
            name: Some("test_entity".to_string()),
            position: [1.0, 2.0, 3.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0, 1.0, 1.0],
            components: components
                .into_iter()
                .map(|(comp_type, data)| ComponentData {
                    component_type: comp_type.to_string(),
                    data: data.to_string(),
                })
                .collect(),
        }];
        Cell {
            coord,
            state: CellState::Loaded,
            entities: Vec::new(),
            assets: Vec::new(),
            bounds: AABB::from_center_half_extents(Vec3::ZERO, Vec3::splat(50.0)),
            entity_blueprints,
            metadata: None,
        }
    }

    // ==================== WeaveAnchorSpec Tests ====================

    #[test]
    fn test_weave_anchor_spec_creation() {
        let anchor = WeaveAnchorSpec {
            cell: GridCoord::new(1, 2, 3),
            position: [10.0, 20.0, 30.0],
            anchor_id: "anchor_1".to_string(),
            anchor_type: Some("stability".to_string()),
            stability: Some("high".to_string()),
            echo_cost: Some(5.0),
        };
        assert_eq!(anchor.anchor_id, "anchor_1");
        assert_eq!(anchor.cell.x, 1);
        assert!((anchor.position[0] - 10.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_weave_anchor_spec_optional_fields() {
        let anchor = WeaveAnchorSpec {
            cell: GridCoord::new(0, 0, 0),
            position: [0.0, 0.0, 0.0],
            anchor_id: "minimal".to_string(),
            anchor_type: None,
            stability: None,
            echo_cost: None,
        };
        assert!(anchor.anchor_type.is_none());
        assert!(anchor.stability.is_none());
        assert!(anchor.echo_cost.is_none());
    }

    // ==================== TriggerZoneSpec Tests ====================

    #[test]
    fn test_trigger_zone_spec_creation() {
        let trigger = TriggerZoneSpec {
            cell: GridCoord::new(5, 5, 5),
            position: [50.0, 50.0, 50.0],
            trigger_id: "zone_a".to_string(),
            shape: Some("sphere".to_string()),
            radius: Some(10.0),
            extents: None,
            action: None,
        };
        assert_eq!(trigger.trigger_id, "zone_a");
        assert_eq!(trigger.shape, Some("sphere".to_string()));
    }

    #[test]
    fn test_trigger_zone_spec_box_shape() {
        let trigger = TriggerZoneSpec {
            cell: GridCoord::new(0, 0, 0),
            position: [0.0, 0.0, 0.0],
            trigger_id: "box_zone".to_string(),
            shape: Some("box".to_string()),
            radius: None,
            extents: Some([5.0, 10.0, 5.0]),
            action: None,
        };
        assert!(trigger.extents.is_some());
        assert!(trigger.radius.is_none());
    }

    // ==================== DecisionPromptSpec Tests ====================

    #[test]
    fn test_decision_prompt_spec_creation() {
        let prompt = DecisionPromptSpec {
            cell: GridCoord::new(0, 0, 0),
            position: [0.0, 0.0, 0.0],
            trigger_id: "decision_1".to_string(),
            options: vec!["Option A".to_string(), "Option B".to_string()],
        };
        assert_eq!(prompt.options.len(), 2);
        assert_eq!(prompt.options[0], "Option A");
    }

    #[test]
    fn test_decision_prompt_spec_empty_options() {
        let prompt = DecisionPromptSpec {
            cell: GridCoord::new(0, 0, 0),
            position: [0.0, 0.0, 0.0],
            trigger_id: "empty_decision".to_string(),
            options: vec![],
        };
        assert!(prompt.options.is_empty());
    }

    // ==================== EncounterTriggerSpec Tests ====================

    #[test]
    fn test_encounter_trigger_spec_creation() {
        let trigger = EncounterTriggerSpec {
            cell: GridCoord::new(0, 0, 0),
            position: [0.0, 0.0, 0.0],
            trigger_id: "encounter_1".to_string(),
            script: Some("combat_encounter.lua".to_string()),
        };
        assert_eq!(trigger.script, Some("combat_encounter.lua".to_string()));
    }

    #[test]
    fn test_encounter_trigger_spec_no_script() {
        let trigger = EncounterTriggerSpec {
            cell: GridCoord::new(0, 0, 0),
            position: [0.0, 0.0, 0.0],
            trigger_id: "simple_encounter".to_string(),
            script: None,
        };
        assert!(trigger.script.is_none());
    }

    // ==================== EncounterCompleteSpec Tests ====================

    #[test]
    fn test_encounter_complete_spec_creation() {
        let complete = EncounterCompleteSpec {
            cell: GridCoord::new(0, 0, 0),
            position: [0.0, 0.0, 0.0],
            trigger_id: "complete_1".to_string(),
            next_cell: Some(GridCoord::new(1, 0, 0)),
        };
        assert!(complete.next_cell.is_some());
        assert_eq!(complete.next_cell.test_unwrap().x, 1);
    }

    #[test]
    fn test_encounter_complete_spec_no_next_cell() {
        let complete = EncounterCompleteSpec {
            cell: GridCoord::new(0, 0, 0),
            position: [0.0, 0.0, 0.0],
            trigger_id: "final_encounter".to_string(),
            next_cell: None,
        };
        assert!(complete.next_cell.is_none());
    }

    // ==================== EffectAnchorSpec Tests ====================

    #[test]
    fn test_effect_anchor_spec_creation() {
        let effect = EffectAnchorSpec {
            cell: GridCoord::new(0, 0, 0),
            position: [0.0, 5.0, 0.0],
            trigger_id: Some("effect_trigger".to_string()),
            effect: Some("particle_fire".to_string()),
            intensity: Some(0.8),
        };
        assert_eq!(effect.effect, Some("particle_fire".to_string()));
        assert!((effect.intensity.test_unwrap() - 0.8).abs() < f32::EPSILON);
    }

    #[test]
    fn test_effect_anchor_spec_minimal() {
        let effect = EffectAnchorSpec {
            cell: GridCoord::new(0, 0, 0),
            position: [0.0, 0.0, 0.0],
            trigger_id: None,
            effect: None,
            intensity: None,
        };
        assert!(effect.trigger_id.is_none());
        assert!(effect.effect.is_none());
    }

    // ==================== SpawnPointSpec Tests ====================

    #[test]
    fn test_spawn_point_spec_creation() {
        let spawn = SpawnPointSpec {
            cell: GridCoord::new(0, 0, 0),
            position: [100.0, 0.0, 100.0],
            spawn_id: "player_start".to_string(),
            facing: Some([0.0, 0.0, 1.0]),
        };
        assert_eq!(spawn.spawn_id, "player_start");
        assert!(spawn.facing.is_some());
    }

    #[test]
    fn test_spawn_point_spec_no_facing() {
        let spawn = SpawnPointSpec {
            cell: GridCoord::new(0, 0, 0),
            position: [0.0, 0.0, 0.0],
            spawn_id: "default_spawn".to_string(),
            facing: None,
        };
        assert!(spawn.facing.is_none());
    }

    // ==================== VeilweaverSliceMetadata Tests ====================

    #[test]
    fn test_veilweaver_slice_metadata_default() {
        let meta = VeilweaverSliceMetadata::default();
        assert!(meta.anchors.is_empty());
        assert!(meta.trigger_zones.is_empty());
        assert!(meta.decision_prompts.is_empty());
        assert!(meta.encounter_triggers.is_empty());
        assert!(meta.encounter_completes.is_empty());
        assert!(meta.effect_anchors.is_empty());
        assert!(meta.spawn_points.is_empty());
    }

    #[test]
    fn test_veilweaver_slice_metadata_clone() {
        let mut meta = VeilweaverSliceMetadata::default();
        meta.anchors.push(WeaveAnchorSpec {
            cell: GridCoord::new(0, 0, 0),
            position: [0.0, 0.0, 0.0],
            anchor_id: "test".to_string(),
            anchor_type: None,
            stability: None,
            echo_cost: None,
        });
        let cloned = meta.clone();
        assert_eq!(cloned.anchors.len(), 1);
    }

    #[test]
    fn test_extend_cell_weave_anchor() {
        let cell = make_cell_with_components(vec![(
            "WeaveAnchor",
            r#"{"anchor_id": "anchor_test", "type": "stability", "stability": "medium", "echo_cost": 3.5}"#,
        )]);
        let mut meta = VeilweaverSliceMetadata::default();
        meta.extend_cell(&cell)
            .test_expect("extend_cell should succeed");
        assert_eq!(meta.anchors.len(), 1);
        assert_eq!(meta.anchors[0].anchor_id, "anchor_test");
        assert_eq!(meta.anchors[0].anchor_type, Some("stability".to_string()));
        assert!((meta.anchors[0].echo_cost.test_unwrap() - 3.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_extend_cell_trigger_zone() {
        let cell = make_cell_with_components(vec![(
            "TriggerZone",
            r#"{"trigger_id": "zone_1", "shape": "sphere", "radius": 5.0}"#,
        )]);
        let mut meta = VeilweaverSliceMetadata::default();
        meta.extend_cell(&cell)
            .test_expect("extend_cell should succeed");
        assert_eq!(meta.trigger_zones.len(), 1);
        assert_eq!(meta.trigger_zones[0].trigger_id, "zone_1");
        assert_eq!(meta.trigger_zones[0].shape, Some("sphere".to_string()));
    }

    #[test]
    fn test_extend_cell_decision_prompt() {
        let cell = make_cell_with_components(vec![(
            "DecisionPrompt",
            r#"{"trigger_id": "prompt_1", "options": ["Yes", "No"]}"#,
        )]);
        let mut meta = VeilweaverSliceMetadata::default();
        meta.extend_cell(&cell)
            .test_expect("extend_cell should succeed");
        assert_eq!(meta.decision_prompts.len(), 1);
        assert_eq!(meta.decision_prompts[0].options.len(), 2);
    }

    #[test]
    fn test_extend_cell_encounter_trigger() {
        let cell = make_cell_with_components(vec![(
            "EncounterTrigger",
            r#"{"trigger_id": "enc_1", "script": "battle.lua"}"#,
        )]);
        let mut meta = VeilweaverSliceMetadata::default();
        meta.extend_cell(&cell)
            .test_expect("extend_cell should succeed");
        assert_eq!(meta.encounter_triggers.len(), 1);
        assert_eq!(
            meta.encounter_triggers[0].script,
            Some("battle.lua".to_string())
        );
    }

    #[test]
    fn test_extend_cell_encounter_complete() {
        let cell = make_cell_with_components(vec![(
            "EncounterComplete",
            r#"{"trigger_id": "complete_1", "next_cell": [1, 0, 0]}"#,
        )]);
        let mut meta = VeilweaverSliceMetadata::default();
        meta.extend_cell(&cell)
            .test_expect("extend_cell should succeed");
        assert_eq!(meta.encounter_completes.len(), 1);
        assert!(meta.encounter_completes[0].next_cell.is_some());
    }

    #[test]
    fn test_extend_cell_effect_anchor() {
        let cell = make_cell_with_components(vec![(
            "EffectAnchor",
            r#"{"trigger_id": "fx_1", "effect": "smoke", "intensity": 0.5}"#,
        )]);
        let mut meta = VeilweaverSliceMetadata::default();
        meta.extend_cell(&cell)
            .test_expect("extend_cell should succeed");
        assert_eq!(meta.effect_anchors.len(), 1);
        assert_eq!(meta.effect_anchors[0].effect, Some("smoke".to_string()));
    }

    #[test]
    fn test_extend_cell_spawn_point() {
        let cell = make_cell_with_components(vec![(
            "SpawnPoint",
            r#"{"id": "spawn_1", "facing": [1.0, 0.0, 0.0]}"#,
        )]);
        let mut meta = VeilweaverSliceMetadata::default();
        meta.extend_cell(&cell)
            .test_expect("extend_cell should succeed");
        assert_eq!(meta.spawn_points.len(), 1);
        assert_eq!(meta.spawn_points[0].spawn_id, "spawn_1");
    }

    #[test]
    fn test_extend_cell_multiple_components() {
        let cell = make_cell_with_components(vec![
            ("WeaveAnchor", r#"{"anchor_id": "anchor_1"}"#),
            ("TriggerZone", r#"{"trigger_id": "trigger_1"}"#),
            ("SpawnPoint", r#"{"id": "spawn_1"}"#),
        ]);
        let mut meta = VeilweaverSliceMetadata::default();
        meta.extend_cell(&cell)
            .test_expect("extend_cell should succeed");
        assert_eq!(meta.anchors.len(), 1);
        assert_eq!(meta.trigger_zones.len(), 1);
        assert_eq!(meta.spawn_points.len(), 1);
    }

    #[test]
    fn test_extend_cell_empty_cell() {
        let cell = make_cell_with_components(vec![]);
        let mut meta = VeilweaverSliceMetadata::default();
        meta.extend_cell(&cell)
            .test_expect("extend_cell should succeed");
        assert!(meta.anchors.is_empty());
        assert!(meta.trigger_zones.is_empty());
    }

    #[test]
    fn test_from_cells_multiple_cells() {
        let cell1 =
            make_cell_with_components(vec![("WeaveAnchor", r#"{"anchor_id": "anchor_1"}"#)]);
        let cell2 = make_cell_with_components(vec![
            ("WeaveAnchor", r#"{"anchor_id": "anchor_2"}"#),
            ("SpawnPoint", r#"{"id": "spawn_1"}"#),
        ]);
        let cells = vec![&cell1, &cell2];
        let meta =
            VeilweaverSliceMetadata::from_cells(&cells).test_expect("from_cells should succeed");
        assert_eq!(meta.anchors.len(), 2);
        assert_eq!(meta.spawn_points.len(), 1);
    }

    #[test]
    fn test_from_cells_empty_slice() {
        let cells: Vec<&Cell> = vec![];
        let meta =
            VeilweaverSliceMetadata::from_cells(&cells).test_expect("from_cells should succeed");
        assert!(meta.anchors.is_empty());
    }

    #[test]
    fn test_extend_cell_invalid_json() {
        let cell = make_cell_with_components(vec![
            ("WeaveAnchor", r#"{"anchor_id": "test"}"#),  // Valid
            ("TriggerZone", r#"{"trigger_id": "zone"}"#), // Valid
        ]);
        let mut meta = VeilweaverSliceMetadata::default();
        // Should succeed
        let result = meta.extend_cell(&cell);
        assert!(result.is_ok());
    }

    #[test]
    fn test_extend_cell_minimal_weave_anchor() {
        let cell = make_cell_with_components(vec![("WeaveAnchor", r#"{"anchor_id": "minimal"}"#)]);
        let mut meta = VeilweaverSliceMetadata::default();
        meta.extend_cell(&cell)
            .test_expect("extend_cell should succeed");
        assert_eq!(meta.anchors[0].anchor_id, "minimal");
        assert!(meta.anchors[0].anchor_type.is_none());
        assert!(meta.anchors[0].stability.is_none());
        assert!(meta.anchors[0].echo_cost.is_none());
    }

    #[test]
    fn test_extend_cell_trigger_zone_with_extents() {
        let cell = make_cell_with_components(vec![(
            "TriggerZone",
            r#"{"trigger_id": "box", "shape": "box", "extents": [10.0, 5.0, 10.0]}"#,
        )]);
        let mut meta = VeilweaverSliceMetadata::default();
        meta.extend_cell(&cell)
            .test_expect("extend_cell should succeed");
        let extents = meta.trigger_zones[0].extents.test_unwrap();
        assert!((extents[0] - 10.0).abs() < f32::EPSILON);
        assert!((extents[1] - 5.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_extend_cell_decision_prompt_no_options() {
        let cell = make_cell_with_components(vec![(
            "DecisionPrompt",
            r#"{"trigger_id": "empty_prompt"}"#,
        )]);
        let mut meta = VeilweaverSliceMetadata::default();
        meta.extend_cell(&cell)
            .test_expect("extend_cell should succeed");
        assert!(meta.decision_prompts[0].options.is_empty());
    }

    #[test]
    fn test_extend_cell_encounter_complete_no_next_cell() {
        let cell =
            make_cell_with_components(vec![("EncounterComplete", r#"{"trigger_id": "final"}"#)]);
        let mut meta = VeilweaverSliceMetadata::default();
        meta.extend_cell(&cell)
            .test_expect("extend_cell should succeed");
        assert!(meta.encounter_completes[0].next_cell.is_none());
    }

    #[test]
    fn test_spec_position_preserved() {
        let cell = make_cell_with_components(vec![("WeaveAnchor", r#"{"anchor_id": "pos_test"}"#)]);
        let mut meta = VeilweaverSliceMetadata::default();
        meta.extend_cell(&cell)
            .test_expect("extend_cell should succeed");
        // Position comes from entity blueprint: [1.0, 2.0, 3.0]
        assert!((meta.anchors[0].position[0] - 1.0).abs() < f32::EPSILON);
        assert!((meta.anchors[0].position[1] - 2.0).abs() < f32::EPSILON);
        assert!((meta.anchors[0].position[2] - 3.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_spec_cell_coord_preserved() {
        let cell =
            make_cell_with_components(vec![("WeaveAnchor", r#"{"anchor_id": "coord_test"}"#)]);
        let mut meta = VeilweaverSliceMetadata::default();
        meta.extend_cell(&cell)
            .test_expect("extend_cell should succeed");
        // Coord is [0, 0, 0] from make_cell_with_components
        assert_eq!(meta.anchors[0].cell.x, 0);
        assert_eq!(meta.anchors[0].cell.y, 0);
        assert_eq!(meta.anchors[0].cell.z, 0);
    }

    #[test]
    fn test_effect_anchor_all_optional() {
        let cell = make_cell_with_components(vec![("EffectAnchor", r#"{}"#)]);
        let mut meta = VeilweaverSliceMetadata::default();
        meta.extend_cell(&cell)
            .test_expect("extend_cell should succeed");
        assert!(meta.effect_anchors[0].trigger_id.is_none());
        assert!(meta.effect_anchors[0].effect.is_none());
        assert!(meta.effect_anchors[0].intensity.is_none());
    }

    #[test]
    fn test_spawn_point_no_facing() {
        let cell = make_cell_with_components(vec![("SpawnPoint", r#"{"id": "no_facing"}"#)]);
        let mut meta = VeilweaverSliceMetadata::default();
        meta.extend_cell(&cell)
            .test_expect("extend_cell should succeed");
        assert!(meta.spawn_points[0].facing.is_none());
    }
}
