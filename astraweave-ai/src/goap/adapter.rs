// Enhanced WorldSnapshot to GOAP WorldState adapter
// Phase 2: Engine Integration

use super::{OrderedFloat, StateValue, WorldState};
use astraweave_core::WorldSnapshot;

/// Enhanced adapter with richer state extraction
pub struct SnapshotAdapter;

impl SnapshotAdapter {
    /// Convert WorldSnapshot to comprehensive GOAP WorldState
    pub fn to_world_state(snap: &WorldSnapshot) -> WorldState {
        let mut state = WorldState::new();

        // === Player State ===
        state.set("player_hp", StateValue::Int(snap.player.hp));
        state.set("player_x", StateValue::Int(snap.player.pos.x));
        state.set("player_y", StateValue::Int(snap.player.pos.y));
        state.set(
            "player_stance",
            StateValue::String(snap.player.stance.to_string()),
        );

        // Player health categories
        let player_critical = snap.player.hp < 30;
        let player_wounded = snap.player.hp < 60;
        state.set("player_critical", StateValue::Bool(player_critical));
        state.set("player_wounded", StateValue::Bool(player_wounded));

        // === Companion State ===
        state.set("my_ammo", StateValue::Int(snap.me.ammo));
        state.set("my_x", StateValue::Int(snap.me.pos.x));
        state.set("my_y", StateValue::Int(snap.me.pos.y));
        state.set("my_morale", StateValue::Float(OrderedFloat(snap.me.morale)));

        // Ammo state flags
        state.set("has_ammo", StateValue::Bool(snap.me.ammo > 0));
        state.set("ammo_low", StateValue::Bool(snap.me.ammo < 10));
        state.set("ammo_critical", StateValue::Bool(snap.me.ammo < 5));

        // Morale flags
        state.set("morale_high", StateValue::Bool(snap.me.morale > 0.7));
        state.set("morale_low", StateValue::Bool(snap.me.morale < 0.4));

        // === Cooldowns ===
        for (name, value) in &snap.me.cooldowns {
            let cd_key = format!("cd_{}", name);
            state.set(&cd_key, StateValue::Float(OrderedFloat(*value)));

            // Add boolean flag for active cooldowns
            let active_key = format!("{}_on_cooldown", name);
            state.set(&active_key, StateValue::Bool(*value > 0.0));
        }

        // Smoke grenade availability
        let smoke_cd = snap.me.cooldowns.get("throw:smoke").copied().unwrap_or(0.0);
        state.set("smoke_cooldown", StateValue::Float(OrderedFloat(smoke_cd)));
        state.set("smoke_available", StateValue::Bool(smoke_cd <= 0.0));

        // === Enemy State (Aggregate) ===
        let enemy_count = snap.enemies.len() as i32;
        state.set("enemy_count", StateValue::Int(enemy_count));
        state.set("enemy_present", StateValue::Bool(!snap.enemies.is_empty()));

        if let Some(first_enemy) = snap.enemies.first() {
            state.set("enemy_hp", StateValue::Int(first_enemy.hp));
            state.set("enemy_x", StateValue::Int(first_enemy.pos.x));
            state.set("enemy_y", StateValue::Int(first_enemy.pos.y));
            state.set("enemy_cover", StateValue::String(first_enemy.cover.to_string()));

            // Calculate distance to closest enemy
            let dist_x = (snap.me.pos.x - first_enemy.pos.x).abs();
            let dist_y = (snap.me.pos.y - first_enemy.pos.y).abs();
            let manhattan_distance = dist_x + dist_y;

            state.set("enemy_distance", StateValue::Int(manhattan_distance));

            // Range flags
            state.set("in_range", StateValue::Bool(manhattan_distance <= 8));
            state.set("in_melee_range", StateValue::Bool(manhattan_distance <= 2));
            state.set("enemy_far", StateValue::Bool(manhattan_distance > 10));
            state.set("enemy_close", StateValue::Bool(manhattan_distance < 5));

            // Threat assessment
            let enemy_dangerous = first_enemy.hp > 30 && manhattan_distance < 8;
            state.set("enemy_dangerous", StateValue::Bool(enemy_dangerous));

            // Enemy health categories
            state.set("enemy_wounded", StateValue::Bool(first_enemy.hp < 50));
            state.set("enemy_critical", StateValue::Bool(first_enemy.hp < 20));
        } else {
            state.set("enemy_distance", StateValue::Int(999));
            state.set("in_range", StateValue::Bool(false));
            state.set("enemy_far", StateValue::Bool(true));
            state.set("enemy_close", StateValue::Bool(false));
            state.set("enemy_dangerous", StateValue::Bool(false));
        }

        // === Combat State ===
        state.set("in_combat", StateValue::Bool(!snap.enemies.is_empty()));

        // Safe state assessment
        let low_health = snap.me.ammo < 10 || snap.player.hp < 40;
        state.set("low_health", StateValue::Bool(low_health));

        let safe_state = snap.enemies.is_empty() || (snap.me.ammo > 10 && snap.player.hp > 60);
        state.set("safe", StateValue::Bool(safe_state));

        // === Tactical Flags ===

        // Should retreat?
        let should_retreat = (snap.player.hp < 30 && !snap.enemies.is_empty())
            || (snap.me.ammo < 5 && !snap.enemies.is_empty());
        state.set("should_retreat", StateValue::Bool(should_retreat));

        // Should heal?
        let should_heal = snap.player.hp < 50;
        state.set("should_heal", StateValue::Bool(should_heal));

        // Should reload?
        let should_reload = snap.me.ammo < 10 && snap.me.ammo > 0;
        state.set("should_reload", StateValue::Bool(should_reload));

        // === Cover State ===
        // Note: Real implementation would check actual cover positions
        state.set("in_cover", StateValue::Bool(false)); // Placeholder
        state.set("cover_available", StateValue::Bool(true)); // Placeholder

        // === Resource Flags ===
        state.set("has_medkit", StateValue::Bool(true)); // Placeholder - would check inventory

        // === Positional State ===
        let distance_to_player =
            (snap.me.pos.x - snap.player.pos.x).abs() + (snap.me.pos.y - snap.player.pos.y).abs();
        state.set("distance_to_player", StateValue::Int(distance_to_player));
        state.set("near_player", StateValue::Bool(distance_to_player < 5));
        state.set("far_from_player", StateValue::Bool(distance_to_player > 10));

        // === Objective State ===
        if let Some(ref objective) = snap.objective {
            state.set("has_objective", StateValue::Bool(true));
            state.set("objective", StateValue::String(objective.clone()));
        } else {
            state.set("has_objective", StateValue::Bool(false));
        }

        // === POIs (Points of Interest) ===
        state.set("poi_count", StateValue::Int(snap.pois.len() as i32));
        state.set("has_pois", StateValue::Bool(!snap.pois.is_empty()));

        state
    }

    /// Extract key tactical situation for logging
    pub fn tactical_summary(snap: &WorldSnapshot) -> String {
        let enemy_count = snap.enemies.len();
        let ammo = snap.me.ammo;
        let hp = snap.player.hp;

        let enemy_dist = snap
            .enemies
            .first()
            .map(|e| (snap.me.pos.x - e.pos.x).abs() + (snap.me.pos.y - e.pos.y).abs())
            .unwrap_or(999);

        format!(
            "HP:{} Ammo:{} Enemies:{} Dist:{} Morale:{:.1}",
            hp, ammo, enemy_count, enemy_dist, snap.me.morale
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use astraweave_core::{CompanionState, CoverType, EnemyState, IVec2, PlayerState, Stance};
    use std::collections::BTreeMap;

    fn make_test_snapshot() -> WorldSnapshot {
        WorldSnapshot {
            t: 1.0,
            player: PlayerState {
                hp: 100,
                pos: IVec2 { x: 0, y: 0 },
                stance: Stance::Stand,
                orders: vec![],
            },
            me: CompanionState {
                ammo: 20,
                cooldowns: BTreeMap::new(),
                morale: 1.0,
                pos: IVec2 { x: 5, y: 5 },
            },
            enemies: vec![EnemyState {
                id: 1,
                pos: IVec2 { x: 10, y: 10 },
                hp: 50,
                cover: CoverType::None,
                last_seen: 1.0,
            }],
            pois: vec![],
            obstacles: vec![],
            objective: Some("Defeat enemies".to_string()),
        }
    }

    #[test]
    fn test_enhanced_adapter_basic_state() {
        let snap = make_test_snapshot();
        let state = SnapshotAdapter::to_world_state(&snap);

        assert_eq!(state.get("player_hp"), Some(&StateValue::Int(100)));
        assert_eq!(state.get("my_ammo"), Some(&StateValue::Int(20)));
        assert_eq!(state.get("enemy_count"), Some(&StateValue::Int(1)));
    }

    #[test]
    fn test_enhanced_adapter_tactical_flags() {
        let snap = make_test_snapshot();
        let state = SnapshotAdapter::to_world_state(&snap);

        assert_eq!(state.get("enemy_present"), Some(&StateValue::Bool(true)));
        assert_eq!(state.get("has_ammo"), Some(&StateValue::Bool(true)));
        assert_eq!(state.get("in_combat"), Some(&StateValue::Bool(true)));
    }

    #[test]
    fn test_enhanced_adapter_distance_calculation() {
        let snap = make_test_snapshot();
        let state = SnapshotAdapter::to_world_state(&snap);

        // Enemy at (10,10), companion at (5,5) -> distance = 10
        assert_eq!(state.get("enemy_distance"), Some(&StateValue::Int(10)));
        assert_eq!(state.get("in_range"), Some(&StateValue::Bool(false))); // >8
    }

    #[test]
    fn test_enhanced_adapter_health_categories() {
        let mut snap = make_test_snapshot();
        snap.player.hp = 25;

        let state = SnapshotAdapter::to_world_state(&snap);

        assert_eq!(state.get("player_critical"), Some(&StateValue::Bool(true)));
        assert_eq!(state.get("should_retreat"), Some(&StateValue::Bool(true)));
    }

    #[test]
    fn test_enhanced_adapter_ammo_flags() {
        let mut snap = make_test_snapshot();
        snap.me.ammo = 3;

        let state = SnapshotAdapter::to_world_state(&snap);

        assert_eq!(state.get("ammo_critical"), Some(&StateValue::Bool(true)));
        assert_eq!(state.get("has_ammo"), Some(&StateValue::Bool(true)));
    }

    #[test]
    fn test_tactical_summary() {
        let snap = make_test_snapshot();
        let summary = SnapshotAdapter::tactical_summary(&snap);

        assert!(summary.contains("HP:100"));
        assert!(summary.contains("Ammo:20"));
        assert!(summary.contains("Enemies:1"));
    }

    // ========== Mutation-killing tests: boundary conditions ==========

    #[test]
    fn test_player_critical_boundary() {
        let mut snap = make_test_snapshot();

        // hp=30 → critical is < 30, so false
        snap.player.hp = 30;
        let s = SnapshotAdapter::to_world_state(&snap);
        assert_eq!(s.get("player_critical"), Some(&StateValue::Bool(false)));

        // hp=29 → true
        snap.player.hp = 29;
        let s = SnapshotAdapter::to_world_state(&snap);
        assert_eq!(s.get("player_critical"), Some(&StateValue::Bool(true)));
    }

    #[test]
    fn test_player_wounded_boundary() {
        let mut snap = make_test_snapshot();

        // hp=60 → wounded is < 60, so false
        snap.player.hp = 60;
        let s = SnapshotAdapter::to_world_state(&snap);
        assert_eq!(s.get("player_wounded"), Some(&StateValue::Bool(false)));

        // hp=59 → true
        snap.player.hp = 59;
        let s = SnapshotAdapter::to_world_state(&snap);
        assert_eq!(s.get("player_wounded"), Some(&StateValue::Bool(true)));
    }

    #[test]
    fn test_morale_flags() {
        let mut snap = make_test_snapshot();

        // High morale: > 0.7
        snap.me.morale = 0.71;
        let s = SnapshotAdapter::to_world_state(&snap);
        assert_eq!(s.get("morale_high"), Some(&StateValue::Bool(true)));
        assert_eq!(s.get("morale_low"), Some(&StateValue::Bool(false)));

        // At 0.7 boundary — NOT high (>0.7 required)
        snap.me.morale = 0.7;
        let s = SnapshotAdapter::to_world_state(&snap);
        assert_eq!(s.get("morale_high"), Some(&StateValue::Bool(false)));

        // Low morale: < 0.4
        snap.me.morale = 0.39;
        let s = SnapshotAdapter::to_world_state(&snap);
        assert_eq!(s.get("morale_low"), Some(&StateValue::Bool(true)));

        // At 0.4 boundary — NOT low (<0.4 required)
        snap.me.morale = 0.4;
        let s = SnapshotAdapter::to_world_state(&snap);
        assert_eq!(s.get("morale_low"), Some(&StateValue::Bool(false)));
    }

    #[test]
    fn test_cooldowns_extraction() {
        let mut snap = make_test_snapshot();
        snap.me.cooldowns.insert("attack".to_string(), 2.5);
        snap.me.cooldowns.insert("dodge".to_string(), 0.0);

        let s = SnapshotAdapter::to_world_state(&snap);

        // CD values
        assert_eq!(
            s.get("cd_attack"),
            Some(&StateValue::Float(OrderedFloat(2.5)))
        );
        assert_eq!(
            s.get("cd_dodge"),
            Some(&StateValue::Float(OrderedFloat(0.0)))
        );

        // Active cooldown flags
        assert_eq!(
            s.get("attack_on_cooldown"),
            Some(&StateValue::Bool(true))
        );
        assert_eq!(
            s.get("dodge_on_cooldown"),
            Some(&StateValue::Bool(false))
        );
    }

    #[test]
    fn test_smoke_cooldown() {
        let mut snap = make_test_snapshot();

        // No throw:smoke key → default 0.0 → available
        let s = SnapshotAdapter::to_world_state(&snap);
        assert_eq!(
            s.get("smoke_cooldown"),
            Some(&StateValue::Float(OrderedFloat(0.0)))
        );
        assert_eq!(s.get("smoke_available"), Some(&StateValue::Bool(true)));

        // Active cooldown → not available
        snap.me.cooldowns.insert("throw:smoke".to_string(), 3.0);
        let s = SnapshotAdapter::to_world_state(&snap);
        assert_eq!(
            s.get("smoke_cooldown"),
            Some(&StateValue::Float(OrderedFloat(3.0)))
        );
        assert_eq!(s.get("smoke_available"), Some(&StateValue::Bool(false)));
    }

    #[test]
    fn test_no_enemies_branch() {
        let mut snap = make_test_snapshot();
        snap.enemies.clear();

        let s = SnapshotAdapter::to_world_state(&snap);

        assert_eq!(s.get("enemy_count"), Some(&StateValue::Int(0)));
        assert_eq!(s.get("enemy_present"), Some(&StateValue::Bool(false)));
        assert_eq!(s.get("in_combat"), Some(&StateValue::Bool(false)));
        assert_eq!(s.get("enemy_distance"), Some(&StateValue::Int(999)));
        assert_eq!(s.get("in_range"), Some(&StateValue::Bool(false)));
        assert_eq!(s.get("enemy_far"), Some(&StateValue::Bool(true)));
        assert_eq!(s.get("enemy_close"), Some(&StateValue::Bool(false)));
        assert_eq!(s.get("enemy_dangerous"), Some(&StateValue::Bool(false)));
        // No enemy_hp, enemy_x, enemy_y set
        assert!(s.get("enemy_hp").is_none());
    }

    #[test]
    fn test_enemy_health_boundaries() {
        let mut snap = make_test_snapshot();

        // enemy_wounded: hp < 50; enemy_critical: hp < 20
        snap.enemies[0].hp = 50;
        let s = SnapshotAdapter::to_world_state(&snap);
        assert_eq!(s.get("enemy_wounded"), Some(&StateValue::Bool(false)));
        assert_eq!(s.get("enemy_critical"), Some(&StateValue::Bool(false)));

        snap.enemies[0].hp = 49;
        let s = SnapshotAdapter::to_world_state(&snap);
        assert_eq!(s.get("enemy_wounded"), Some(&StateValue::Bool(true)));

        snap.enemies[0].hp = 20;
        let s = SnapshotAdapter::to_world_state(&snap);
        assert_eq!(s.get("enemy_critical"), Some(&StateValue::Bool(false)));

        snap.enemies[0].hp = 19;
        let s = SnapshotAdapter::to_world_state(&snap);
        assert_eq!(s.get("enemy_critical"), Some(&StateValue::Bool(true)));
    }

    #[test]
    fn test_range_flag_boundaries() {
        let mut snap = make_test_snapshot();

        // in_range: manhattan <= 8, in_melee_range: <= 2, enemy_far: > 10, enemy_close: < 5
        // Companion at (5,5), move enemy to get specific distances

        // Distance = 8 (at boundary: in_range=true, enemy_far=false)
        snap.enemies[0].pos = IVec2 { x: 10, y: 8 };
        let s = SnapshotAdapter::to_world_state(&snap);
        let dist = 5 + 3; // |5-10| + |5-8| = 5 + 3 = 8
        assert_eq!(s.get("enemy_distance"), Some(&StateValue::Int(dist)));
        assert_eq!(s.get("in_range"), Some(&StateValue::Bool(true)));

        // Distance = 9 (just outside in_range)
        snap.enemies[0].pos = IVec2 { x: 10, y: 9 };
        let s = SnapshotAdapter::to_world_state(&snap);
        assert_eq!(s.get("in_range"), Some(&StateValue::Bool(false)));

        // Distance = 2 (melee boundary)
        snap.enemies[0].pos = IVec2 { x: 6, y: 6 };
        let s = SnapshotAdapter::to_world_state(&snap);
        assert_eq!(s.get("in_melee_range"), Some(&StateValue::Bool(true)));

        // Distance = 3 (outside melee)
        snap.enemies[0].pos = IVec2 { x: 7, y: 6 };
        let s = SnapshotAdapter::to_world_state(&snap);
        assert_eq!(s.get("in_melee_range"), Some(&StateValue::Bool(false)));

        // Distance = 10 (enemy_far boundary: > 10 required)
        snap.enemies[0].pos = IVec2 { x: 10, y: 10 };
        let s = SnapshotAdapter::to_world_state(&snap);
        assert_eq!(s.get("enemy_far"), Some(&StateValue::Bool(false)));

        // Distance = 11
        snap.enemies[0].pos = IVec2 { x: 11, y: 10 };
        let s = SnapshotAdapter::to_world_state(&snap);
        assert_eq!(s.get("enemy_far"), Some(&StateValue::Bool(true)));

        // Distance = 5 (enemy_close boundary: < 5 required)
        snap.enemies[0].pos = IVec2 { x: 8, y: 7 };
        let s = SnapshotAdapter::to_world_state(&snap);
        assert_eq!(s.get("enemy_close"), Some(&StateValue::Bool(false)));

        // Distance = 4
        snap.enemies[0].pos = IVec2 { x: 7, y: 7 };
        let s = SnapshotAdapter::to_world_state(&snap);
        assert_eq!(s.get("enemy_close"), Some(&StateValue::Bool(true)));
    }

    #[test]
    fn test_enemy_dangerous_flag() {
        let mut snap = make_test_snapshot();

        // enemy_dangerous: hp > 30 AND distance < 8
        // Companion at (5,5)

        // hp=31, distance=7 → dangerous
        snap.enemies[0].hp = 31;
        snap.enemies[0].pos = IVec2 { x: 9, y: 8 }; // dist = 4+3 = 7
        let s = SnapshotAdapter::to_world_state(&snap);
        assert_eq!(s.get("enemy_dangerous"), Some(&StateValue::Bool(true)));

        // hp=30, distance=7 → NOT dangerous (hp must be > 30)
        snap.enemies[0].hp = 30;
        let s = SnapshotAdapter::to_world_state(&snap);
        assert_eq!(s.get("enemy_dangerous"), Some(&StateValue::Bool(false)));

        // hp=50, distance=8 → NOT dangerous (distance must be < 8)
        snap.enemies[0].hp = 50;
        snap.enemies[0].pos = IVec2 { x: 10, y: 8 }; // dist = 5+3 = 8
        let s = SnapshotAdapter::to_world_state(&snap);
        assert_eq!(s.get("enemy_dangerous"), Some(&StateValue::Bool(false)));
    }

    #[test]
    fn test_low_health_flag() {
        let mut snap = make_test_snapshot();

        // low_health: ammo < 10 || player.hp < 40
        snap.me.ammo = 20;
        snap.player.hp = 100;
        let s = SnapshotAdapter::to_world_state(&snap);
        assert_eq!(s.get("low_health"), Some(&StateValue::Bool(false)));

        // ammo=9 triggers low_health
        snap.me.ammo = 9;
        let s = SnapshotAdapter::to_world_state(&snap);
        assert_eq!(s.get("low_health"), Some(&StateValue::Bool(true)));

        // ammo=10 doesn't trigger
        snap.me.ammo = 10;
        let s = SnapshotAdapter::to_world_state(&snap);
        assert_eq!(s.get("low_health"), Some(&StateValue::Bool(false)));

        // player.hp=39 triggers
        snap.player.hp = 39;
        let s = SnapshotAdapter::to_world_state(&snap);
        assert_eq!(s.get("low_health"), Some(&StateValue::Bool(true)));

        // player.hp=40 doesn't trigger
        snap.me.ammo = 20;
        snap.player.hp = 40;
        let s = SnapshotAdapter::to_world_state(&snap);
        assert_eq!(s.get("low_health"), Some(&StateValue::Bool(false)));
    }

    #[test]
    fn test_safe_flag() {
        let mut snap = make_test_snapshot();

        // safe: no enemies || (ammo > 10 && player.hp > 60)
        // Has enemies, ammo=20, hp=100 → true
        let s = SnapshotAdapter::to_world_state(&snap);
        assert_eq!(s.get("safe"), Some(&StateValue::Bool(true)));

        // ammo=10 (not > 10) → false
        snap.me.ammo = 10;
        let s = SnapshotAdapter::to_world_state(&snap);
        assert_eq!(s.get("safe"), Some(&StateValue::Bool(false)));

        // ammo=11, hp=60 (not > 60) → false
        snap.me.ammo = 11;
        snap.player.hp = 60;
        let s = SnapshotAdapter::to_world_state(&snap);
        assert_eq!(s.get("safe"), Some(&StateValue::Bool(false)));

        // no enemies → always safe
        snap.enemies.clear();
        snap.me.ammo = 0;
        snap.player.hp = 1;
        let s = SnapshotAdapter::to_world_state(&snap);
        assert_eq!(s.get("safe"), Some(&StateValue::Bool(true)));
    }

    #[test]
    fn test_should_retreat_both_conditions() {
        let mut snap = make_test_snapshot();

        // should_retreat: (player.hp < 30 && enemies) || (ammo < 5 && enemies)

        // Neither condition → false
        snap.player.hp = 80;
        snap.me.ammo = 20;
        let s = SnapshotAdapter::to_world_state(&snap);
        assert_eq!(s.get("should_retreat"), Some(&StateValue::Bool(false)));

        // hp < 30 with enemies → true
        snap.player.hp = 29;
        let s = SnapshotAdapter::to_world_state(&snap);
        assert_eq!(s.get("should_retreat"), Some(&StateValue::Bool(true)));

        // hp=30 → false (needs < 30)
        snap.player.hp = 30;
        let s = SnapshotAdapter::to_world_state(&snap);
        assert_eq!(s.get("should_retreat"), Some(&StateValue::Bool(false)));

        // ammo < 5 with enemies → true
        snap.player.hp = 100;
        snap.me.ammo = 4;
        let s = SnapshotAdapter::to_world_state(&snap);
        assert_eq!(s.get("should_retreat"), Some(&StateValue::Bool(true)));

        // ammo=5 → false (needs < 5)
        snap.me.ammo = 5;
        let s = SnapshotAdapter::to_world_state(&snap);
        assert_eq!(s.get("should_retreat"), Some(&StateValue::Bool(false)));

        // No enemies → never retreat even with low stats
        snap.enemies.clear();
        snap.player.hp = 1;
        snap.me.ammo = 0;
        let s = SnapshotAdapter::to_world_state(&snap);
        assert_eq!(s.get("should_retreat"), Some(&StateValue::Bool(false)));
    }

    #[test]
    fn test_should_heal_boundary() {
        let mut snap = make_test_snapshot();

        // should_heal: player.hp < 50
        snap.player.hp = 50;
        let s = SnapshotAdapter::to_world_state(&snap);
        assert_eq!(s.get("should_heal"), Some(&StateValue::Bool(false)));

        snap.player.hp = 49;
        let s = SnapshotAdapter::to_world_state(&snap);
        assert_eq!(s.get("should_heal"), Some(&StateValue::Bool(true)));
    }

    #[test]
    fn test_should_reload_boundary() {
        let mut snap = make_test_snapshot();

        // should_reload: ammo < 10 && ammo > 0
        snap.me.ammo = 10;
        let s = SnapshotAdapter::to_world_state(&snap);
        assert_eq!(s.get("should_reload"), Some(&StateValue::Bool(false)));

        snap.me.ammo = 9;
        let s = SnapshotAdapter::to_world_state(&snap);
        assert_eq!(s.get("should_reload"), Some(&StateValue::Bool(true)));

        snap.me.ammo = 1;
        let s = SnapshotAdapter::to_world_state(&snap);
        assert_eq!(s.get("should_reload"), Some(&StateValue::Bool(true)));

        snap.me.ammo = 0;
        let s = SnapshotAdapter::to_world_state(&snap);
        assert_eq!(s.get("should_reload"), Some(&StateValue::Bool(false)));
    }

    #[test]
    fn test_distance_to_player_flags() {
        let mut snap = make_test_snapshot();

        // Companion at (5,5), Player at (0,0) → distance = 10
        let s = SnapshotAdapter::to_world_state(&snap);
        assert_eq!(s.get("distance_to_player"), Some(&StateValue::Int(10)));
        assert_eq!(s.get("near_player"), Some(&StateValue::Bool(false)));
        assert_eq!(s.get("far_from_player"), Some(&StateValue::Bool(false)));

        // near_player: < 5
        snap.me.pos = IVec2 { x: 2, y: 2 };
        let s = SnapshotAdapter::to_world_state(&snap);
        let dist = 2 + 2; // = 4
        assert_eq!(s.get("distance_to_player"), Some(&StateValue::Int(dist)));
        assert_eq!(s.get("near_player"), Some(&StateValue::Bool(true)));

        // boundary: distance = 5 → NOT near (< 5 required)
        snap.me.pos = IVec2 { x: 3, y: 2 };
        let s = SnapshotAdapter::to_world_state(&snap);
        assert_eq!(s.get("near_player"), Some(&StateValue::Bool(false)));

        // far_from_player: > 10
        snap.me.pos = IVec2 { x: 6, y: 6 };
        let s = SnapshotAdapter::to_world_state(&snap);
        let dist = 6 + 6; // = 12
        assert_eq!(s.get("distance_to_player"), Some(&StateValue::Int(dist)));
        assert_eq!(s.get("far_from_player"), Some(&StateValue::Bool(true)));

        // boundary: distance = 10 → NOT far (> 10 required)
        snap.me.pos = IVec2 { x: 5, y: 5 };
        let s = SnapshotAdapter::to_world_state(&snap);
        assert_eq!(s.get("far_from_player"), Some(&StateValue::Bool(false)));
    }

    #[test]
    fn test_no_objective() {
        let mut snap = make_test_snapshot();
        snap.objective = None;

        let s = SnapshotAdapter::to_world_state(&snap);
        assert_eq!(s.get("has_objective"), Some(&StateValue::Bool(false)));
        assert!(s.get("objective").is_none());
    }

    #[test]
    fn test_has_objective() {
        let snap = make_test_snapshot();
        let s = SnapshotAdapter::to_world_state(&snap);
        assert_eq!(s.get("has_objective"), Some(&StateValue::Bool(true)));
        assert_eq!(
            s.get("objective"),
            Some(&StateValue::String("Defeat enemies".to_string()))
        );
    }

    #[test]
    fn test_pois() {
        let mut snap = make_test_snapshot();

        // No POIs
        let s = SnapshotAdapter::to_world_state(&snap);
        assert_eq!(s.get("poi_count"), Some(&StateValue::Int(0)));
        assert_eq!(s.get("has_pois"), Some(&StateValue::Bool(false)));

        // Add POIs
        snap.pois.push(astraweave_core::Poi {
            k: "ammo_crate".to_string(),
            pos: IVec2 { x: 3, y: 3 },
        });
        snap.pois.push(astraweave_core::Poi {
            k: "health_pack".to_string(),
            pos: IVec2 { x: 7, y: 7 },
        });
        let s = SnapshotAdapter::to_world_state(&snap);
        assert_eq!(s.get("poi_count"), Some(&StateValue::Int(2)));
        assert_eq!(s.get("has_pois"), Some(&StateValue::Bool(true)));
    }

    #[test]
    fn test_ammo_boundary_flags() {
        let mut snap = make_test_snapshot();

        // ammo_low: < 10, ammo_critical: < 5, has_ammo: > 0
        snap.me.ammo = 10;
        let s = SnapshotAdapter::to_world_state(&snap);
        assert_eq!(s.get("ammo_low"), Some(&StateValue::Bool(false)));

        snap.me.ammo = 9;
        let s = SnapshotAdapter::to_world_state(&snap);
        assert_eq!(s.get("ammo_low"), Some(&StateValue::Bool(true)));

        snap.me.ammo = 5;
        let s = SnapshotAdapter::to_world_state(&snap);
        assert_eq!(s.get("ammo_critical"), Some(&StateValue::Bool(false)));

        snap.me.ammo = 4;
        let s = SnapshotAdapter::to_world_state(&snap);
        assert_eq!(s.get("ammo_critical"), Some(&StateValue::Bool(true)));

        snap.me.ammo = 0;
        let s = SnapshotAdapter::to_world_state(&snap);
        assert_eq!(s.get("has_ammo"), Some(&StateValue::Bool(false)));
    }

    #[test]
    fn test_player_stance_and_positions() {
        let mut snap = make_test_snapshot();
        snap.player.stance = "crouch".to_string();
        snap.player.pos = IVec2 { x: 7, y: 3 };

        let s = SnapshotAdapter::to_world_state(&snap);
        assert_eq!(
            s.get("player_stance"),
            Some(&StateValue::String("crouch".to_string()))
        );
        assert_eq!(s.get("player_x"), Some(&StateValue::Int(7)));
        assert_eq!(s.get("player_y"), Some(&StateValue::Int(3)));
    }

    #[test]
    fn test_companion_position_extraction() {
        let mut snap = make_test_snapshot();
        snap.me.pos = IVec2 { x: 12, y: 8 };

        let s = SnapshotAdapter::to_world_state(&snap);
        assert_eq!(s.get("my_x"), Some(&StateValue::Int(12)));
        assert_eq!(s.get("my_y"), Some(&StateValue::Int(8)));
    }

    #[test]
    fn test_enemy_position_and_cover() {
        let snap = make_test_snapshot();
        let s = SnapshotAdapter::to_world_state(&snap);

        assert_eq!(s.get("enemy_x"), Some(&StateValue::Int(10)));
        assert_eq!(s.get("enemy_y"), Some(&StateValue::Int(10)));
        assert_eq!(
            s.get("enemy_cover"),
            Some(&StateValue::String("none".to_string()))
        );
    }

    #[test]
    fn test_placeholder_flags() {
        let snap = make_test_snapshot();
        let s = SnapshotAdapter::to_world_state(&snap);

        assert_eq!(s.get("in_cover"), Some(&StateValue::Bool(false)));
        assert_eq!(s.get("cover_available"), Some(&StateValue::Bool(true)));
        assert_eq!(s.get("has_medkit"), Some(&StateValue::Bool(true)));
    }

    #[test]
    fn test_tactical_summary_no_enemies() {
        let mut snap = make_test_snapshot();
        snap.enemies.clear();
        snap.me.morale = 0.8;

        let summary = SnapshotAdapter::tactical_summary(&snap);
        assert!(summary.contains("HP:100"));
        assert!(summary.contains("Ammo:20"));
        assert!(summary.contains("Enemies:0"));
        assert!(summary.contains("Dist:999"));
        assert!(summary.contains("Morale:0.8"));
    }

    #[test]
    fn test_tactical_summary_distance() {
        let mut snap = make_test_snapshot();
        // Enemy at (10,10), companion at (5,5) → dist = 10
        let summary = SnapshotAdapter::tactical_summary(&snap);
        assert!(summary.contains("Dist:10"));
    }

    #[test]
    fn test_my_morale_float_extraction() {
        let mut snap = make_test_snapshot();
        snap.me.morale = 0.65;

        let s = SnapshotAdapter::to_world_state(&snap);
        assert_eq!(
            s.get("my_morale"),
            Some(&StateValue::Float(OrderedFloat(0.65)))
        );
    }
}
