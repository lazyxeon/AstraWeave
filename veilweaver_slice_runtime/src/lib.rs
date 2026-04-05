//! # Veilweaver Vertical Slice Runtime
//!
//! Headless game runtime for the AstraWeave **Veilweaver** vertical slice —
//! a 30-minute playable demo spanning 6 zones (Z0–Z4 + Z2a), a 3-phase
//! boss encounter, companion AI, narrative branching, and full HUD/VFX/audio
//! data pipelines.
//!
//! ## Architecture
//!
//! The crate is organized around [`walkthrough::SliceOrchestrator`], which
//! composes 10+ subsystems into a deterministic `tick(dt) → TickResult` loop:
//!
//! - **Core**: [`game_loop`], [`player_state`], [`combat`], [`storm_choice`]
//! - **HUD**: [`hud_state`], [`boss_hud`], [`companion_hud`], [`decision_ui`], [`recap_panel`]
//! - **Presentation**: [`vfx_specs`], [`audio_specs`], [`vfx_dispatch`], [`palette`]
//! - **Validation**: [`determinism`], [`perf_budget`], [`checkpoint`], [`telemetry`]
//! - **AI** (feature-gated): [`boss_encounter`], [`companion_ai`]
//!
//! ## Design Principles
//!
//! - `#![forbid(unsafe_code)]` — memory safety at compile time
//! - **Headless-safe** — zero rendering/audio/windowing dependencies
//! - **Pure data models** — runtime produces data, presentation reads each frame
//! - **Deterministic** — identical inputs produce identical state
//! - **NaN-hardened** — all numeric paths guard against non-finite values

#![forbid(unsafe_code)]
use anyhow::Result;
use astraweave_core::ecs_adapter;
use astraweave_core::ecs_bridge::EntityBridge;
use astraweave_core::ecs_events::Events;
use astraweave_core::{Entity as LegacyEntity, World as LegacyWorld};
use astraweave_ecs::{App, World};
use astraweave_gameplay::veilweaver_tutorial::{
    tutorial_anchor_events, tutorial_anchor_sync, tutorial_trigger_system, AnchorStabilizedEvent,
    TriggerVolumeEvent, WeaveTutorialState,
};
use astraweave_gameplay::{TriggerZoneSpec, VeilweaverSliceMetadata, WeaveAnchorSpec};
use astraweave_scene::world_partition::{Cell, GridCoord, WorldPartition};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use tracing::info;

pub mod cinematic_player;
pub mod game_loop;
pub mod storm_choice;
pub mod zone_transitions;

// Phase 2: Core game loop integration (headless-safe)
pub mod combat;
pub mod player_state;
pub mod walkthrough;

// Phase 3: UI & HUD data models (headless-safe, no rendering deps)
pub mod boss_hud;
pub mod companion_hud;
pub mod decision_ui;
pub mod hud_state;
pub mod recap_panel;
pub mod telemetry;

// Phase 4: VFX, audio & polish descriptors (headless-safe)
pub mod audio_specs;
pub mod palette;
pub mod vfx_dispatch;
pub mod vfx_specs;

// Phase 5: Validation & ship (headless-safe)
pub mod checkpoint;
pub mod determinism;
pub mod perf_budget;

/// Boss encounter system — Oathbound Warden fight lifecycle.
///
/// Requires the `boss-director` feature (pulls in `astraweave-director`).
#[cfg(feature = "boss-director")]
pub mod boss_encounter;

/// Companion AI integration — GOAP-based companion planner.
///
/// Requires the `ai-companion` feature (pulls in `astraweave-ai`).
#[cfg(feature = "ai-companion")]
pub mod companion_ai;

/// Configuration loaded from a TOML/RON slice descriptor.
///
/// Controls the fixed timestep (`dt`), initial cell coordinate, and
/// optional camera spawn position for the Veilweaver vertical slice.
#[derive(Debug, Clone, Deserialize)]
pub struct VeilweaverSliceConfig {
    pub dt: f32,
    pub initial_cell: Option<[i32; 3]>,
    #[serde(default)]
    pub camera_start: Option<[f32; 3]>,
}

#[derive(Debug, Clone, Default)]
struct TutorialEventContext {
    trigger_occupancy: HashMap<String, HashSet<LegacyEntity>>,
    anchor_stabilized: HashMap<String, bool>,
    anchor_trigger_map: HashMap<String, Vec<String>>,
}

impl TutorialEventContext {
    fn from_metadata(metadata: &VeilweaverSliceMetadata) -> Self {
        let mut trigger_occupancy = HashMap::new();
        for trigger in &metadata.trigger_zones {
            trigger_occupancy.insert(trigger.trigger_id.clone(), HashSet::new());
        }

        let mut anchor_stabilized = HashMap::new();
        let mut anchor_trigger_map = HashMap::new();

        for anchor in &metadata.anchors {
            let stabilized = anchor
                .stability
                .as_deref()
                .map(|value| {
                    matches!(
                        value.to_ascii_lowercase().as_str(),
                        "stabilized" | "stable" | "reinforced"
                    )
                })
                .unwrap_or(false);
            anchor_stabilized.insert(anchor.anchor_id.clone(), stabilized);

            let mut nearby_triggers = Vec::new();
            for trigger in &metadata.trigger_zones {
                if anchor.cell != trigger.cell {
                    continue;
                }
                if positions_close(&anchor.position, &trigger.position, 0.75) {
                    nearby_triggers.push(trigger.trigger_id.clone());
                }
            }

            if !nearby_triggers.is_empty() {
                anchor_trigger_map.insert(anchor.anchor_id.clone(), nearby_triggers);
            }
        }

        Self {
            trigger_occupancy,
            anchor_stabilized,
            anchor_trigger_map,
        }
    }
}

struct TriggerEventIntermediate {
    trigger_id: String,
    entering: bool,
    legacy_entity: Option<LegacyEntity>,
}

fn tutorial_event_emitters(world: &mut World) {
    let metadata = match world.get_resource::<VeilweaverSliceMetadata>() {
        Some(meta) => meta.clone(),
        None => return,
    };

    if world.get_resource::<TutorialEventContext>().is_none() {
        world.insert_resource(TutorialEventContext::from_metadata(&metadata));
    }

    let entity_snapshots: Vec<(LegacyEntity, [f32; 3])> = {
        let Some(legacy_world) = world.get_resource::<LegacyWorld>() else {
            return;
        };

        legacy_world
            .entities()
            .into_iter()
            .filter_map(|legacy_id| {
                let pose = legacy_world.pose(legacy_id)?;
                let team = legacy_world.team(legacy_id).map(|t| t.id).unwrap_or(0);
                if team > 1 {
                    return None;
                }
                Some((
                    legacy_id,
                    [pose.pos.x as f32, pose.height, pose.pos.y as f32],
                ))
            })
            .collect()
    };

    let (trigger_events_data, anchor_events) = {
        let context = match world.get_resource_mut::<TutorialEventContext>() {
            Some(ctx) => ctx,
            None => return,
        };

        let mut occupancy_updates: HashMap<String, HashSet<LegacyEntity>> = metadata
            .trigger_zones
            .iter()
            .map(|trigger| (trigger.trigger_id.clone(), HashSet::new()))
            .collect();

        for (legacy_id, position) in &entity_snapshots {
            for trigger in &metadata.trigger_zones {
                if trigger_contains(trigger, *position) {
                    if let Some(set) = occupancy_updates.get_mut(&trigger.trigger_id) {
                        set.insert(*legacy_id);
                    }
                }
            }
        }

        let mut trigger_events = Vec::new();
        for trigger in &metadata.trigger_zones {
            let trigger_id = &trigger.trigger_id;
            let new_set = occupancy_updates.remove(trigger_id).unwrap_or_default();
            let prev_set = context
                .trigger_occupancy
                .entry(trigger_id.clone())
                .or_insert_with(HashSet::new);

            for &legacy_id in new_set.iter() {
                if !prev_set.contains(&legacy_id) {
                    trigger_events.push(TriggerEventIntermediate {
                        trigger_id: trigger_id.clone(),
                        entering: true,
                        legacy_entity: Some(legacy_id),
                    });
                }
            }

            for &legacy_id in prev_set.iter() {
                if !new_set.contains(&legacy_id) {
                    trigger_events.push(TriggerEventIntermediate {
                        trigger_id: trigger_id.clone(),
                        entering: false,
                        legacy_entity: Some(legacy_id),
                    });
                }
            }

            prev_set.clear();
            prev_set.extend(new_set.iter().copied());
        }

        let mut anchor_events = Vec::new();
        for anchor in &metadata.anchors {
            if context
                .anchor_stabilized
                .get(&anchor.anchor_id)
                .copied()
                .unwrap_or(false)
            {
                continue;
            }

            let is_stabilized = context
                .anchor_trigger_map
                .get(&anchor.anchor_id)
                .map(|trigger_ids| {
                    trigger_ids.iter().any(|trigger_id| {
                        context
                            .trigger_occupancy
                            .get(trigger_id)
                            .map(|set| !set.is_empty())
                            .unwrap_or(false)
                    })
                })
                .unwrap_or(false);

            if is_stabilized {
                context
                    .anchor_stabilized
                    .insert(anchor.anchor_id.clone(), true);
                anchor_events.push(AnchorStabilizedEvent {
                    anchor_id: anchor.anchor_id.clone(),
                });
            }
        }

        (trigger_events, anchor_events)
    };

    if trigger_events_data.is_empty() && anchor_events.is_empty() {
        return;
    }

    let trigger_events: Vec<TriggerVolumeEvent> = {
        let bridge = world.get_resource::<EntityBridge>();
        trigger_events_data
            .into_iter()
            .map(|event| {
                let ecs_entity = match (bridge, event.legacy_entity) {
                    (Some(bridge), Some(legacy)) => bridge.get(&legacy),
                    _ => None,
                };

                TriggerVolumeEvent {
                    trigger_id: event.trigger_id,
                    entering: event.entering,
                    entity: ecs_entity,
                }
            })
            .collect()
    };

    if !trigger_events.is_empty() {
        if let Some(events) = world.get_resource_mut::<Events<TriggerVolumeEvent>>() {
            let mut writer = events.writer();
            for event in trigger_events {
                writer.send(event);
            }
        }
    }

    if !anchor_events.is_empty() {
        if let Some(events) = world.get_resource_mut::<Events<AnchorStabilizedEvent>>() {
            let mut writer = events.writer();
            for event in anchor_events {
                writer.send(event);
            }
        }
    }
}

fn positions_close(a: &[f32; 3], b: &[f32; 3], threshold: f32) -> bool {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    let dz = a[2] - b[2];
    dx * dx + dy * dy + dz * dz <= threshold * threshold
}

fn trigger_contains(trigger: &TriggerZoneSpec, point: [f32; 3]) -> bool {
    let center = trigger.position;
    let shape = trigger
        .shape
        .as_deref()
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_else(|| {
            if trigger.extents.is_some() {
                "box".to_string()
            } else {
                "sphere".to_string()
            }
        });

    match shape.as_str() {
        "box" => {
            let extents = trigger.extents.unwrap_or([1.0, 1.0, 1.0]);
            let dx = (point[0] - center[0]).abs();
            let dy = (point[1] - center[1]).abs();
            let dz = (point[2] - center[2]).abs();
            dx <= extents[0] && dy <= extents[1] && dz <= extents[2]
        }
        "cylinder" => {
            let fallback_radius = trigger.extents.map(|ext| ext[0]).unwrap_or(2.0);
            let radius = trigger.radius.unwrap_or(fallback_radius);
            let half_height = trigger.extents.map(|ext| ext[1]).unwrap_or(1.0);
            let dx = point[0] - center[0];
            let dz = point[2] - center[2];
            let distance_sq = dx * dx + dz * dz;
            let min_y = center[1] - half_height;
            let max_y = center[1] + half_height;
            distance_sq <= radius * radius && point[1] >= min_y && point[1] <= max_y
        }
        _ => {
            let fallback_radius = trigger.extents.map(|ext| ext[0].max(ext[2])).unwrap_or(2.0);
            let radius = trigger.radius.unwrap_or(fallback_radius);
            let dx = point[0] - center[0];
            let dy = point[1] - center[1];
            let dz = point[2] - center[2];
            dx * dx + dy * dy + dz * dz <= radius * radius
        }
    }
}

/// Top-level ECS runtime for the Veilweaver vertical slice.
///
/// Wraps an [`App`] (ECS world + schedule), a [`WorldPartition`] (zone cells),
/// and aggregated [`VeilweaverSliceMetadata`] (triggers, anchors, dialogue).
/// Tutorial systems are auto-installed on construction.
pub struct VeilweaverRuntime {
    pub app: App,
    pub partition: WorldPartition,
    pub metadata: VeilweaverSliceMetadata,
}

impl std::fmt::Debug for VeilweaverRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VeilweaverRuntime")
            .field("partition", &self.partition)
            .field("metadata", &self.metadata)
            .finish_non_exhaustive()
    }
}

impl VeilweaverRuntime {
    /// Creates a new runtime from a config and a pre-loaded world partition.
    ///
    /// Installs tutorial ECS systems and gathers zone metadata.
    ///
    /// # Errors
    ///
    /// Returns an error if the world partition fails to produce valid
    /// metadata (e.g., unresolvable cell references).
    pub fn new(config: VeilweaverSliceConfig, partition: WorldPartition) -> Result<Self> {
        let legacy_world = LegacyWorld::new();
        let dt = config.dt.max(0.0001);
        let app = ecs_adapter::build_app(legacy_world, dt);

        let metadata = Self::gather_metadata(&partition);
        let mut runtime = Self {
            app,
            partition,
            metadata,
        };
        runtime.install_tutorial_systems();
        Ok(runtime)
    }

    fn gather_metadata(partition: &WorldPartition) -> VeilweaverSliceMetadata {
        let cells: Vec<&Cell> = partition
            .loaded_cells()
            .iter()
            .filter_map(|coord| partition.get_cell(*coord))
            .collect();
        VeilweaverSliceMetadata::from_cells(&cells).unwrap_or_default()
    }

    /// Returns all weave anchors that belong to the given cell coordinate.
    pub fn anchors_in_cell(&self, coord: GridCoord) -> Vec<&WeaveAnchorSpec> {
        self.metadata
            .anchors
            .iter()
            .filter(|anchor| anchor.cell == coord)
            .collect()
    }

    /// Registers a system to run once after the initial ECS setup.
    pub fn add_post_setup_system(&mut self, system: fn(&mut World)) {
        self.app
            .schedule
            .add_system("veilweaver_post_setup", system);
    }

    /// Advances the ECS world by one fixed tick.
    pub fn run_tick(&mut self) {
        let mut app = std::mem::take(&mut self.app);
        app = app.run_fixed(1);
        self.app = app;
    }

    /// Re-scans the world partition and updates metadata + tutorial context.
    ///
    /// Call after loading or unloading cells to keep anchors, triggers,
    /// and tutorial state in sync.
    pub fn refresh_metadata(&mut self) {
        self.metadata = Self::gather_metadata(&self.partition);
        info!(
            "Veilweaver metadata refreshed: {} anchors, {} triggers",
            self.metadata.anchors.len(),
            self.metadata.trigger_zones.len()
        );

        if let Some(meta_res) = self.app.world.get_resource_mut::<VeilweaverSliceMetadata>() {
            *meta_res = self.metadata.clone();
        } else {
            self.app.world.insert_resource(self.metadata.clone());
        }

        if let Some(context) = self.app.world.get_resource_mut::<TutorialEventContext>() {
            let mut new_context = TutorialEventContext::from_metadata(&self.metadata);
            for (anchor_id, is_stabilized) in context.anchor_stabilized.iter() {
                if *is_stabilized {
                    new_context
                        .anchor_stabilized
                        .insert(anchor_id.clone(), true);
                }
            }
            *context = new_context;
        } else {
            self.app
                .world
                .insert_resource(TutorialEventContext::from_metadata(&self.metadata));
        }
    }

    fn install_tutorial_systems(&mut self) {
        self.app.world.insert_resource(self.metadata.clone());

        let tutorial_state = WeaveTutorialState::from_metadata(self.metadata.clone());
        self.app.world.insert_resource(tutorial_state);

        self.app
            .world
            .insert_resource(TutorialEventContext::from_metadata(&self.metadata));

        self.app
            .world
            .insert_resource(Events::<AnchorStabilizedEvent>::default());
        self.app
            .world
            .insert_resource(Events::<TriggerVolumeEvent>::default());

        self.add_post_setup_system(tutorial_event_emitters);
        self.add_post_setup_system(tutorial_anchor_sync);
        self.add_post_setup_system(tutorial_anchor_events);
        self.add_post_setup_system(tutorial_trigger_system);
    }
}
