use crate::schema::{CoverType, Poi, Stance};
use crate::{CompanionState, EnemyState, Entity, IVec2, PlayerState, World, WorldSnapshot};

pub struct PerceptionConfig {
    pub los_max: i32,
}

/// A breach the nav publish reported. `x` and `y` are world cells, not the
/// retired constant cell.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ObservedBreach {
    pub portal_id: u32,
    pub x: i32,
    pub y: i32,
}

/// Cover that the last nav commit removed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ObservedCover {
    pub id: u32,
    pub x: i32,
    pub y: i32,
}

/// Hook for a weaving effect perception will score later.
///
/// `kind` is reserved for the weaving owner. A non-zero kind on a cell seals
/// a breach that shares that cell.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WeaveEffectHook {
    pub effect_id: u32,
    pub x: i32,
    pub y: i32,
    pub kind: u16,
}

/// Hook for a local-sim hazard (fire, gas, heat) perception will score later.
///
/// A hazard on a cell blocks a breach on that cell.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LocalHazardHook {
    pub hazard_id: u32,
    pub x: i32,
    pub y: i32,
    pub kind: u16,
}

/// Fixed buffers the AI tick fills from nav, weaving, and the local sim.
///
/// Empty lengths mean those channels saw nothing. The legacy snapshot builder
/// copies breaches and lost cover into `WorldSnapshot::pois`. Weaving and
/// hazards stay here so `WorldSnapshot` field names do not change.
#[derive(Clone, Copy, Debug)]
pub struct ObservationBuf {
    pub breaches: [ObservedBreach; 16],
    pub breach_len: u8,
    pub lost_cover: [ObservedCover; 16],
    pub lost_cover_len: u8,
    pub weaving: [WeaveEffectHook; 8],
    pub weaving_len: u8,
    pub hazards: [LocalHazardHook; 8],
    pub hazard_len: u8,
}

impl ObservationBuf {
    #[must_use]
    pub const fn none() -> Self {
        Self {
            breaches: [ObservedBreach {
                portal_id: 0,
                x: 0,
                y: 0,
            }; 16],
            breach_len: 0,
            lost_cover: [ObservedCover { id: 0, x: 0, y: 0 }; 16],
            lost_cover_len: 0,
            weaving: [WeaveEffectHook {
                effect_id: 0,
                x: 0,
                y: 0,
                kind: 0,
            }; 8],
            weaving_len: 0,
            hazards: [LocalHazardHook {
                hazard_id: 0,
                x: 0,
                y: 0,
                kind: 0,
            }; 8],
            hazard_len: 0,
        }
    }
}

impl Default for ObservationBuf {
    fn default() -> Self {
        Self::none()
    }
}

/// A snapshot could not be built because a required component is absent.
///
/// The AI tick records this as `AiPlanningFailedEvent` instead of aborting
/// the process. Callers that have just spawned the entities and treat a
/// missing component as a programmer error use [`build_snapshot`], which
/// still panics.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SnapshotError {
    pub entity: Entity,
    pub component: &'static str,
}

impl std::fmt::Display for SnapshotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "entity {} is missing the {} component",
            self.entity, self.component
        )
    }
}

impl std::error::Error for SnapshotError {}

#[allow(clippy::expect_used)] // Programmer-error path. The tick uses `try_build_snapshot`.
pub fn build_snapshot(
    w: &World,
    t_player: Entity,
    t_companion: Entity,
    enemies: &[Entity],
    objective: Option<String>,
    cfg: &PerceptionConfig,
) -> WorldSnapshot {
    try_build_snapshot(w, t_player, t_companion, enemies, objective, cfg)
        .expect("snapshot entities must carry position, health, ammo, and cooldowns")
}

/// Fallible snapshot builder. A missing component is an error, not a panic.
pub fn try_build_snapshot(
    w: &World,
    t_player: Entity,
    t_companion: Entity,
    enemies: &[Entity],
    objective: Option<String>,
    cfg: &PerceptionConfig,
) -> Result<WorldSnapshot, SnapshotError> {
    assemble_snapshot(
        w,
        t_player,
        t_companion,
        enemies,
        objective,
        cfg,
        w.sorted_obstacles(),
        &ObservationBuf::none(),
    )
}

/// Build a snapshot with pre-computed obstacles to avoid redundant conversion
/// when building multiple snapshots per frame (e.g., one per AI agent).
///
/// The obstacle list is sorted by `(x, y)` before it is stored.
#[allow(clippy::expect_used)]
pub fn build_snapshot_shared_obstacles(
    w: &World,
    t_player: Entity,
    t_companion: Entity,
    enemies: &[Entity],
    objective: Option<String>,
    cfg: &PerceptionConfig,
    obstacles: Vec<IVec2>,
) -> WorldSnapshot {
    try_build_snapshot_shared_obstacles(
        w,
        t_player,
        t_companion,
        enemies,
        objective,
        cfg,
        obstacles,
    )
    .expect("snapshot entities must carry position, health, ammo, and cooldowns")
}

/// Fallible form of [`build_snapshot_shared_obstacles`].
pub fn try_build_snapshot_shared_obstacles(
    w: &World,
    t_player: Entity,
    t_companion: Entity,
    enemies: &[Entity],
    objective: Option<String>,
    cfg: &PerceptionConfig,
    obstacles: Vec<IVec2>,
) -> Result<WorldSnapshot, SnapshotError> {
    assemble_snapshot(
        w,
        t_player,
        t_companion,
        enemies,
        objective,
        cfg,
        sort_obstacles(obstacles),
        &ObservationBuf::none(),
    )
}

/// Snapshot builder that copies real breaches and lost cover into `pois`.
pub fn try_build_snapshot_observed(
    w: &World,
    t_player: Entity,
    t_companion: Entity,
    enemies: &[Entity],
    objective: Option<String>,
    cfg: &PerceptionConfig,
    observation: &ObservationBuf,
) -> Result<WorldSnapshot, SnapshotError> {
    assemble_snapshot(
        w,
        t_player,
        t_companion,
        enemies,
        objective,
        cfg,
        w.sorted_obstacles(),
        observation,
    )
}

#[allow(clippy::too_many_arguments)]
fn assemble_snapshot(
    w: &World,
    t_player: Entity,
    t_companion: Entity,
    enemies: &[Entity],
    objective: Option<String>,
    cfg: &PerceptionConfig,
    obstacles: Vec<IVec2>,
    observation: &ObservationBuf,
) -> Result<WorldSnapshot, SnapshotError> {
    let ppos = w.pos_of(t_player).ok_or(SnapshotError {
        entity: t_player,
        component: "Position",
    })?;
    let cpos = w.pos_of(t_companion).ok_or(SnapshotError {
        entity: t_companion,
        component: "Position",
    })?;
    let player = PlayerState {
        hp: w
            .health(t_player)
            .ok_or(SnapshotError {
                entity: t_player,
                component: "Health",
            })?
            .hp,
        pos: ppos,
        stance: Stance::Crouch,
        orders: vec!["hold_east".into()],
    };
    let me = CompanionState {
        ammo: w
            .ammo(t_companion)
            .ok_or(SnapshotError {
                entity: t_companion,
                component: "Ammo",
            })?
            .rounds,
        cooldowns: w
            .cooldowns(t_companion)
            .ok_or(SnapshotError {
                entity: t_companion,
                component: "Cooldowns",
            })?
            .map
            .clone(),
        morale: 0.8,
        pos: cpos,
    };
    let mut seen = Vec::with_capacity(enemies.len());
    for &e in enemies {
        let Some(pos) = w.pos_of(e) else {
            continue;
        };
        let Some(health) = w.health(e) else {
            continue;
        };
        let cover = if (pos.x - ppos.x).abs() + (pos.y - ppos.y).abs() > cfg.los_max {
            CoverType::Unknown
        } else {
            CoverType::Low
        };
        seen.push(EnemyState {
            id: e,
            pos,
            hp: health.hp,
            cover,
            last_seen: w.t,
        });
    }

    Ok(WorldSnapshot {
        t: w.t,
        player,
        me,
        enemies: seen,
        pois: pois_from_observation(observation),
        obstacles,
        objective,
    })
}

fn pois_from_observation(observation: &ObservationBuf) -> Vec<Poi> {
    let mut pois =
        Vec::with_capacity(observation.breach_len as usize + observation.lost_cover_len as usize);
    for breach in observation
        .breaches
        .iter()
        .take(observation.breach_len as usize)
    {
        pois.push(Poi {
            k: "breach".into(),
            pos: IVec2 {
                x: breach.x,
                y: breach.y,
            },
        });
    }
    for cover in observation
        .lost_cover
        .iter()
        .take(observation.lost_cover_len as usize)
    {
        pois.push(Poi {
            k: "cover_lost".into(),
            pos: IVec2 {
                x: cover.x,
                y: cover.y,
            },
        });
    }
    pois
}

fn sort_obstacles(mut obstacles: Vec<IVec2>) -> Vec<IVec2> {
    obstacles.sort_unstable_by(|a, b| a.x.cmp(&b.x).then(a.y.cmp(&b.y)));
    obstacles
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Team, World};

    fn iv2(x: i32, y: i32) -> IVec2 {
        IVec2 { x, y }
    }

    // ===== PerceptionConfig Tests =====
    #[test]
    fn test_perception_config_creation() {
        let cfg = PerceptionConfig { los_max: 10 };
        assert_eq!(cfg.los_max, 10);
    }

    #[test]
    fn test_perception_config_large_los() {
        let cfg = PerceptionConfig { los_max: 100 };
        assert_eq!(cfg.los_max, 100);
    }

    #[test]
    fn test_perception_config_zero_los() {
        let cfg = PerceptionConfig { los_max: 0 };
        assert_eq!(cfg.los_max, 0);
    }

    // ===== build_snapshot Tests =====
    #[test]
    fn test_build_snapshot_basic() {
        let mut w = World::new();
        w.t = 5.0;

        let player = w.spawn("player", iv2(0, 0), Team { id: 1 }, 100, 0);
        let companion = w.spawn("companion", iv2(1, 1), Team { id: 1 }, 100, 10);
        let enemy = w.spawn("enemy", iv2(5, 5), Team { id: 2 }, 50, 0);

        let cfg = PerceptionConfig { los_max: 20 };
        let snap = build_snapshot(&w, player, companion, &[enemy], None, &cfg);

        assert_eq!(snap.t, 5.0);
        assert_eq!(snap.player.hp, 100);
        assert_eq!(snap.player.pos, iv2(0, 0));
        assert_eq!(snap.me.pos, iv2(1, 1));
        assert_eq!(snap.me.ammo, 10);
        assert_eq!(snap.enemies.len(), 1);
        assert_eq!(snap.enemies[0].hp, 50);
        assert_eq!(snap.enemies[0].pos, iv2(5, 5));
    }

    #[test]
    fn test_build_snapshot_time_tracking() {
        let mut w = World::new();
        w.t = 15.5;

        let player = w.spawn("player", iv2(0, 0), Team { id: 1 }, 100, 0);
        let companion = w.spawn("companion", iv2(1, 1), Team { id: 1 }, 80, 5);

        let cfg = PerceptionConfig { los_max: 10 };
        let snap = build_snapshot(&w, player, companion, &[], None, &cfg);

        assert_eq!(snap.t, 15.5);
    }

    #[test]
    fn test_build_snapshot_multiple_enemies() {
        let mut w = World::new();
        w.t = 0.0;

        let player = w.spawn("player", iv2(0, 0), Team { id: 1 }, 100, 0);
        let companion = w.spawn("companion", iv2(1, 1), Team { id: 1 }, 100, 10);
        let enemy1 = w.spawn("enemy1", iv2(5, 5), Team { id: 2 }, 50, 0);
        let enemy2 = w.spawn("enemy2", iv2(6, 6), Team { id: 2 }, 60, 0);
        let enemy3 = w.spawn("enemy3", iv2(7, 7), Team { id: 2 }, 70, 0);

        let cfg = PerceptionConfig { los_max: 20 };
        let snap = build_snapshot(&w, player, companion, &[enemy1, enemy2, enemy3], None, &cfg);

        assert_eq!(snap.enemies.len(), 3);
        assert_eq!(snap.enemies[0].hp, 50);
        assert_eq!(snap.enemies[1].hp, 60);
        assert_eq!(snap.enemies[2].hp, 70);
    }

    #[test]
    fn test_build_snapshot_no_enemies() {
        let mut w = World::new();
        w.t = 0.0;

        let player = w.spawn("player", iv2(0, 0), Team { id: 1 }, 100, 0);
        let companion = w.spawn("companion", iv2(1, 1), Team { id: 1 }, 100, 10);

        let cfg = PerceptionConfig { los_max: 20 };
        let snap = build_snapshot(&w, player, companion, &[], None, &cfg);

        assert_eq!(snap.enemies.len(), 0);
    }

    #[test]
    fn test_build_snapshot_player_state() {
        let mut w = World::new();
        w.t = 0.0;

        let player = w.spawn("player", iv2(10, 20), Team { id: 1 }, 75, 0);
        let companion = w.spawn("companion", iv2(1, 1), Team { id: 1 }, 100, 10);

        let cfg = PerceptionConfig { los_max: 20 };
        let snap = build_snapshot(&w, player, companion, &[], None, &cfg);

        assert_eq!(snap.player.hp, 75);
        assert_eq!(snap.player.pos, iv2(10, 20));
        assert_eq!(snap.player.stance, Stance::Crouch);
        assert_eq!(snap.player.orders.len(), 1);
        assert_eq!(snap.player.orders[0], "hold_east");
    }

    #[test]
    fn test_build_snapshot_companion_state() {
        let mut w = World::new();
        w.t = 0.0;

        let player = w.spawn("player", iv2(0, 0), Team { id: 1 }, 100, 0);
        let companion = w.spawn("companion", iv2(15, 25), Team { id: 1 }, 90, 7);

        let cfg = PerceptionConfig { los_max: 20 };
        let snap = build_snapshot(&w, player, companion, &[], None, &cfg);

        assert_eq!(snap.me.pos, iv2(15, 25));
        assert_eq!(snap.me.ammo, 7);
        assert_eq!(snap.me.morale, 0.8);
    }

    #[test]
    fn test_build_snapshot_cooldowns() {
        let mut w = World::new();
        w.t = 0.0;

        let player = w.spawn("player", iv2(0, 0), Team { id: 1 }, 100, 0);
        let companion = w.spawn("companion", iv2(1, 1), Team { id: 1 }, 100, 10);

        // Get cooldowns component and modify
        if let Some(cds) = w.cooldowns_mut(companion) {
            cds.map.insert("throw".to_string(), 5.0);
            cds.map.insert("heal".to_string(), 2.5);
        }

        let cfg = PerceptionConfig { los_max: 20 };
        let snap = build_snapshot(&w, player, companion, &[], None, &cfg);

        assert_eq!(snap.me.cooldowns.len(), 2);
        assert_eq!(snap.me.cooldowns.get("throw"), Some(&5.0));
        assert_eq!(snap.me.cooldowns.get("heal"), Some(&2.5));
    }

    #[test]
    fn test_build_snapshot_with_objective() {
        let mut w = World::new();
        w.t = 0.0;

        let player = w.spawn("player", iv2(0, 0), Team { id: 1 }, 100, 0);
        let companion = w.spawn("companion", iv2(1, 1), Team { id: 1 }, 100, 10);

        let cfg = PerceptionConfig { los_max: 20 };
        let snap = build_snapshot(
            &w,
            player,
            companion,
            &[],
            Some("Secure the breach point".to_string()),
            &cfg,
        );

        assert!(snap.objective.is_some());
        assert_eq!(snap.objective.unwrap(), "Secure the breach point");
    }

    #[test]
    fn test_build_snapshot_no_objective() {
        let mut w = World::new();
        w.t = 0.0;

        let player = w.spawn("player", iv2(0, 0), Team { id: 1 }, 100, 0);
        let companion = w.spawn("companion", iv2(1, 1), Team { id: 1 }, 100, 10);

        let cfg = PerceptionConfig { los_max: 20 };
        let snap = build_snapshot(&w, player, companion, &[], None, &cfg);

        assert!(snap.objective.is_none());
    }

    #[test]
    fn test_build_snapshot_pois_generated() {
        let mut w = World::new();
        w.t = 0.0;

        let player = w.spawn("player", iv2(0, 0), Team { id: 1 }, 100, 0);
        let companion = w.spawn("companion", iv2(1, 1), Team { id: 1 }, 100, 10);

        let cfg = PerceptionConfig { los_max: 20 };
        let snap = build_snapshot(&w, player, companion, &[], None, &cfg);

        assert!(
            snap.pois.is_empty(),
            "no nav observation means no invented breach"
        );
    }

    #[test]
    fn test_observed_breach_and_lost_cover_become_pois() {
        let mut w = World::new();
        let player = w.spawn("player", iv2(0, 0), Team { id: 1 }, 100, 0);
        let companion = w.spawn("companion", iv2(1, 1), Team { id: 1 }, 100, 10);
        let mut observation = ObservationBuf::none();
        observation.breaches[0] = ObservedBreach {
            portal_id: 7,
            x: 4,
            y: 9,
        };
        observation.breach_len = 1;
        observation.lost_cover[0] = ObservedCover { id: 3, x: 2, y: 2 };
        observation.lost_cover_len = 1;
        observation.weaving[0] = WeaveEffectHook {
            effect_id: 1,
            x: 4,
            y: 9,
            kind: 1,
        };
        observation.weaving_len = 1;
        observation.hazards[0] = LocalHazardHook {
            hazard_id: 2,
            x: 1,
            y: 1,
            kind: 1,
        };
        observation.hazard_len = 1;
        let snap = try_build_snapshot_observed(
            &w,
            player,
            companion,
            &[],
            None,
            &PerceptionConfig { los_max: 10 },
            &observation,
        )
        .expect("snapshot");
        assert_eq!(snap.pois.len(), 2);
        assert_eq!(snap.pois[0].k, "breach");
        assert_eq!(snap.pois[0].pos, iv2(4, 9));
        assert_eq!(snap.pois[1].k, "cover_lost");
        assert_ne!(snap.pois[0].pos, iv2(15, 8));
        assert_eq!(observation.weaving_len, 1);
        assert_eq!(observation.hazard_len, 1);
    }

    #[test]
    fn test_build_snapshot_obstacles() {
        let mut w = World::new();
        w.t = 0.0;
        w.obstacles.insert((5, 5));
        w.obstacles.insert((6, 6));
        w.obstacles.insert((7, 7));

        let player = w.spawn("player", iv2(0, 0), Team { id: 1 }, 100, 0);
        let companion = w.spawn("companion", iv2(1, 1), Team { id: 1 }, 100, 10);

        let cfg = PerceptionConfig { los_max: 20 };
        let snap = build_snapshot(&w, player, companion, &[], None, &cfg);

        assert_eq!(snap.obstacles.len(), 3);
        assert!(snap.obstacles.contains(&iv2(5, 5)));
        assert!(snap.obstacles.contains(&iv2(6, 6)));
        assert!(snap.obstacles.contains(&iv2(7, 7)));
    }

    #[test]
    fn test_build_snapshot_enemy_los_close() {
        let mut w = World::new();
        w.t = 0.0;

        let player = w.spawn("player", iv2(0, 0), Team { id: 1 }, 100, 0);
        let companion = w.spawn("companion", iv2(1, 1), Team { id: 1 }, 100, 10);
        let enemy = w.spawn("enemy", iv2(2, 2), Team { id: 2 }, 50, 0);

        let cfg = PerceptionConfig { los_max: 10 };
        let snap = build_snapshot(&w, player, companion, &[enemy], None, &cfg);

        assert_eq!(snap.enemies.len(), 1);
        // Enemy within los_max should have cover "low"
        assert_eq!(snap.enemies[0].cover, CoverType::Low);
    }

    #[test]
    fn test_build_snapshot_enemy_los_far() {
        let mut w = World::new();
        w.t = 0.0;

        let player = w.spawn("player", iv2(0, 0), Team { id: 1 }, 100, 0);
        let companion = w.spawn("companion", iv2(1, 1), Team { id: 1 }, 100, 10);
        let enemy = w.spawn("enemy", iv2(50, 50), Team { id: 2 }, 50, 0);

        let cfg = PerceptionConfig { los_max: 10 };
        let snap = build_snapshot(&w, player, companion, &[enemy], None, &cfg);

        assert_eq!(snap.enemies.len(), 1);
        // Enemy beyond los_max should have cover "unknown"
        assert_eq!(snap.enemies[0].cover, CoverType::Unknown);
    }

    #[test]
    fn test_build_snapshot_enemy_last_seen() {
        let mut w = World::new();
        w.t = 12.5;

        let player = w.spawn("player", iv2(0, 0), Team { id: 1 }, 100, 0);
        let companion = w.spawn("companion", iv2(1, 1), Team { id: 1 }, 100, 10);
        let enemy = w.spawn("enemy", iv2(5, 5), Team { id: 2 }, 50, 0);

        let cfg = PerceptionConfig { los_max: 20 };
        let snap = build_snapshot(&w, player, companion, &[enemy], None, &cfg);

        assert_eq!(snap.enemies.len(), 1);
        assert_eq!(snap.enemies[0].last_seen, 12.5);
    }

    #[test]
    fn test_build_snapshot_enemy_id_tracking() {
        let mut w = World::new();
        w.t = 0.0;

        let player = w.spawn("player", iv2(0, 0), Team { id: 1 }, 100, 0);
        let companion = w.spawn("companion", iv2(1, 1), Team { id: 1 }, 100, 10);
        let enemy1 = w.spawn("enemy1", iv2(5, 5), Team { id: 2 }, 50, 0);
        let enemy2 = w.spawn("enemy2", iv2(6, 6), Team { id: 2 }, 60, 0);

        let cfg = PerceptionConfig { los_max: 20 };
        let snap = build_snapshot(&w, player, companion, &[enemy1, enemy2], None, &cfg);

        assert_eq!(snap.enemies.len(), 2);
        assert_eq!(snap.enemies[0].id, enemy1);
        assert_eq!(snap.enemies[1].id, enemy2);
    }

    #[test]
    fn test_build_snapshot_comprehensive() {
        let mut w = World::new();
        w.t = 10.0;
        w.obstacles.insert((3, 3));
        w.obstacles.insert((4, 4));

        let player = w.spawn("player", iv2(0, 0), Team { id: 1 }, 85, 0);
        let companion = w.spawn("companion", iv2(2, 2), Team { id: 1 }, 95, 8);
        let enemy1 = w.spawn("enemy1", iv2(5, 5), Team { id: 2 }, 40, 0);
        let enemy2 = w.spawn("enemy2", iv2(100, 100), Team { id: 2 }, 30, 0);

        if let Some(cds) = w.cooldowns_mut(companion) {
            cds.map.insert("grenade".to_string(), 3.0);
        }

        let cfg = PerceptionConfig { los_max: 15 };
        let snap = build_snapshot(
            &w,
            player,
            companion,
            &[enemy1, enemy2],
            Some("Defend position".to_string()),
            &cfg,
        );

        // Verify all components
        assert_eq!(snap.t, 10.0);
        assert_eq!(snap.player.hp, 85);
        assert_eq!(snap.me.ammo, 8);
        assert_eq!(snap.me.cooldowns.len(), 1);
        assert_eq!(snap.enemies.len(), 2);
        assert_eq!(snap.enemies[0].cover, CoverType::Low); // Within LOS
        assert_eq!(snap.enemies[1].cover, CoverType::Unknown); // Beyond LOS
        assert_eq!(snap.obstacles.len(), 2);
        assert!(
            snap.pois.is_empty(),
            "no nav observation means no invented breach"
        );
        assert_eq!(snap.objective, Some("Defend position".to_string()));
    }

    // ===== Mutation-resistant LOS boundary tests =====
    // These catch: `> -> >=`, `- -> +` in the Manhattan distance check

    #[test]
    fn test_los_exact_boundary_equals_max() {
        // Manhattan dist = |5-0| + |5-0| = 10 == los_max → should be "low" (not >)
        let mut w = World::new();
        w.t = 0.0;
        let player = w.spawn("player", iv2(0, 0), Team { id: 1 }, 100, 0);
        let companion = w.spawn("companion", iv2(1, 1), Team { id: 1 }, 100, 10);
        let enemy = w.spawn("enemy", iv2(5, 5), Team { id: 2 }, 50, 0);

        let cfg = PerceptionConfig { los_max: 10 };
        let snap = build_snapshot(&w, player, companion, &[enemy], None, &cfg);
        // dist == los_max → NOT > los_max → cover = "low"
        assert_eq!(
            snap.enemies[0].cover,
            CoverType::Low,
            "dist == los_max should be low, not unknown"
        );
    }

    #[test]
    fn test_los_one_above_boundary() {
        // Manhattan dist = 11 > 10 → "unknown"
        let mut w = World::new();
        w.t = 0.0;
        let player = w.spawn("player", iv2(0, 0), Team { id: 1 }, 100, 0);
        let companion = w.spawn("companion", iv2(1, 1), Team { id: 1 }, 100, 10);
        let enemy = w.spawn("enemy", iv2(6, 5), Team { id: 2 }, 50, 0);

        let cfg = PerceptionConfig { los_max: 10 };
        let snap = build_snapshot(&w, player, companion, &[enemy], None, &cfg);
        assert_eq!(
            snap.enemies[0].cover,
            CoverType::Unknown,
            "dist > los_max should be unknown"
        );
    }

    #[test]
    fn test_los_subtraction_with_nonzero_player_pos() {
        // Player at (10,10), enemy at (12,13) → Manhattan dist = |12-10| + |13-10| = 2+3 = 5
        // If mutated to +, dist = |12+10| + |13+10| = 22+23 = 45 → would be "unknown"
        let mut w = World::new();
        w.t = 0.0;
        let player = w.spawn("player", iv2(10, 10), Team { id: 1 }, 100, 0);
        let companion = w.spawn("companion", iv2(11, 11), Team { id: 1 }, 100, 10);
        let enemy = w.spawn("enemy", iv2(12, 13), Team { id: 2 }, 50, 0);

        let cfg = PerceptionConfig { los_max: 10 };
        let snap = build_snapshot(&w, player, companion, &[enemy], None, &cfg);
        // Correct distance: 5, so enemy is within LOS
        assert_eq!(
            snap.enemies[0].cover,
            CoverType::Low,
            "Enemy near player (non-origin) should have cover=low, not unknown"
        );
    }

    #[test]
    fn test_los_subtraction_negative_direction() {
        // Player at (10,10), enemy at (5,3) → dist = |5-10| + |3-10| = 5+7 = 12
        // If mutated to +, dist = |5+10| + |3+10| = 15+13 = 28 → would be "unknown"
        let mut w = World::new();
        w.t = 0.0;
        let player = w.spawn("player", iv2(10, 10), Team { id: 1 }, 100, 0);
        let companion = w.spawn("companion", iv2(9, 9), Team { id: 1 }, 100, 10);
        let enemy = w.spawn("enemy", iv2(5, 3), Team { id: 2 }, 50, 0);

        let cfg = PerceptionConfig { los_max: 15 };
        let snap = build_snapshot(&w, player, companion, &[enemy], None, &cfg);
        assert_eq!(
            snap.enemies[0].cover,
            CoverType::Low,
            "Negative dir subtraction must work"
        );
    }

    #[test]
    fn test_build_snapshot_obstacles_sorted_by_coordinate() {
        let mut w = World::new();
        w.obstacles.insert((4, 1));
        w.obstacles.insert((1, 8));
        w.obstacles.insert((1, 2));
        let player = w.spawn("player", iv2(0, 0), Team { id: 1 }, 100, 0);
        let companion = w.spawn("companion", iv2(1, 1), Team { id: 1 }, 100, 10);
        let cfg = PerceptionConfig { los_max: 20 };
        let snap = build_snapshot(&w, player, companion, &[], None, &cfg);
        let coords: Vec<(i32, i32)> = snap.obstacles.iter().map(|p| (p.x, p.y)).collect();
        assert_eq!(coords, vec![(1, 2), (1, 8), (4, 1)]);
    }

    #[test]
    fn test_try_build_snapshot_missing_companion_is_error() {
        let mut w = World::new();
        let player = w.spawn("player", iv2(0, 0), Team { id: 0 }, 100, 0);
        let err = try_build_snapshot(&w, player, 99, &[], None, &PerceptionConfig { los_max: 10 })
            .expect_err("missing companion");
        assert_eq!(err.entity, 99);
        assert_eq!(err.component, "Position");
    }
}
