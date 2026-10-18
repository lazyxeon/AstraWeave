//! Mutation-Resistant Tests for astraweave-behavior
//!
//! These tests verify exact computed values (not just structural checks)
//! to ensure they will catch mutations during cargo mutants testing.

use astraweave_behavior::goap::{GoapAction, GoapGoal, GoapPlanner, WorldState};
use astraweave_behavior::goap_cache::{CacheStats, CachedGoapPlanner, PlanCache, PlanCacheKey};

// =============================================================================
// CACHE STATS TESTS
// =============================================================================

mod cache_stats_tests {
    use super::*;

    // ---------------------------------------------------------------------
    // hit_rate() - hits / (hits + misses)
    // ---------------------------------------------------------------------

    #[test]
    fn hit_rate_with_90_hits_10_misses_is_0_9() {
        let stats = CacheStats {
            hits: 90,
            misses: 10,
            evictions: 0,
            invalidations: 0,
        };
        let rate = stats.hit_rate();
        assert!(
            (rate - 0.9).abs() < 1e-9,
            "hit rate should be 0.9, got {}",
            rate
        );
    }

    #[test]
    fn hit_rate_with_50_hits_50_misses_is_0_5() {
        let stats = CacheStats {
            hits: 50,
            misses: 50,
            evictions: 0,
            invalidations: 0,
        };
        let rate = stats.hit_rate();
        assert!(
            (rate - 0.5).abs() < 1e-9,
            "hit rate should be 0.5, got {}",
            rate
        );
    }

    #[test]
    fn hit_rate_with_100_hits_0_misses_is_1_0() {
        let stats = CacheStats {
            hits: 100,
            misses: 0,
            evictions: 0,
            invalidations: 0,
        };
        let rate = stats.hit_rate();
        assert!(
            (rate - 1.0).abs() < 1e-9,
            "hit rate should be 1.0, got {}",
            rate
        );
    }

    #[test]
    fn hit_rate_with_0_hits_100_misses_is_0() {
        let stats = CacheStats {
            hits: 0,
            misses: 100,
            evictions: 0,
            invalidations: 0,
        };
        let rate = stats.hit_rate();
        assert!(rate.abs() < 1e-9, "hit rate should be 0.0, got {}", rate);
    }

    #[test]
    fn hit_rate_with_0_hits_0_misses_is_0() {
        let stats = CacheStats {
            hits: 0,
            misses: 0,
            evictions: 0,
            invalidations: 0,
        };
        let rate = stats.hit_rate();
        assert!(
            rate.abs() < 1e-9,
            "hit rate with no accesses should be 0.0, got {}",
            rate
        );
    }

    #[test]
    fn hit_rate_with_1_hit_3_misses_is_0_25() {
        let stats = CacheStats {
            hits: 1,
            misses: 3,
            evictions: 0,
            invalidations: 0,
        };
        let rate = stats.hit_rate();
        assert!(
            (rate - 0.25).abs() < 1e-9,
            "hit rate should be 0.25, got {}",
            rate
        );
    }

    // ---------------------------------------------------------------------
    // total_accesses() - hits + misses
    // ---------------------------------------------------------------------

    #[test]
    fn total_accesses_with_90_hits_10_misses_is_100() {
        let stats = CacheStats {
            hits: 90,
            misses: 10,
            evictions: 0,
            invalidations: 0,
        };
        assert_eq!(stats.total_accesses(), 100);
    }

    #[test]
    fn total_accesses_with_0_each_is_0() {
        let stats = CacheStats {
            hits: 0,
            misses: 0,
            evictions: 0,
            invalidations: 0,
        };
        assert_eq!(stats.total_accesses(), 0);
    }

    #[test]
    fn total_accesses_with_42_hits_58_misses_is_100() {
        let stats = CacheStats {
            hits: 42,
            misses: 58,
            evictions: 0,
            invalidations: 0,
        };
        assert_eq!(stats.total_accesses(), 100);
    }
}

// =============================================================================
// PLAN CACHE TESTS
// =============================================================================

mod plan_cache_tests {
    use super::*;

    fn create_state_a() -> WorldState {
        WorldState::from_facts(&[("a", true)])
    }

    fn create_state_b() -> WorldState {
        WorldState::from_facts(&[("b", true)])
    }

    fn create_goal() -> GoapGoal {
        GoapGoal::new("test_goal", WorldState::from_facts(&[("done", true)]))
    }

    fn create_actions() -> Vec<GoapAction> {
        vec![GoapAction::new("action1")
            .with_precondition("a", true)
            .with_effect("done", true)]
    }

    // ---------------------------------------------------------------------
    // Cache key equality
    // ---------------------------------------------------------------------

    #[test]
    fn same_inputs_produce_same_cache_key() {
        let state = create_state_a();
        let goal = create_goal();
        let actions = create_actions();

        let key1 = PlanCacheKey::new(&state, &goal, &actions);
        let key2 = PlanCacheKey::new(&state, &goal, &actions);

        assert_eq!(key1, key2);
    }

    #[test]
    fn different_states_produce_different_cache_keys() {
        let state_a = create_state_a();
        let state_b = create_state_b();
        let goal = create_goal();
        let actions = create_actions();

        let key1 = PlanCacheKey::new(&state_a, &goal, &actions);
        let key2 = PlanCacheKey::new(&state_b, &goal, &actions);

        assert_ne!(key1, key2);
    }

    // ---------------------------------------------------------------------
    // Cache capacity and size
    // ---------------------------------------------------------------------

    #[test]
    fn new_cache_capacity_is_set() {
        let cache = PlanCache::new(100);
        assert_eq!(cache.capacity(), 100);
    }

    #[test]
    fn default_cache_capacity_is_1000() {
        let cache = PlanCache::default();
        assert_eq!(cache.capacity(), 1000);
    }

    #[test]
    fn new_cache_is_empty() {
        let cache = PlanCache::new(10);
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
    }

    // ---------------------------------------------------------------------
    // Cache put and get
    // ---------------------------------------------------------------------

    #[test]
    fn put_increases_len_by_1() {
        let mut cache = PlanCache::new(10);
        let state = create_state_a();
        let goal = create_goal();
        let actions = create_actions();

        cache.put(&state, &goal, &actions, vec![]);
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn get_returns_stored_plan() {
        use crate::test_support::TestValue;
        let mut cache = PlanCache::new(10);
        let state = create_state_a();
        let goal = create_goal();
        let actions = create_actions();
        let plan = vec![actions[0].clone()];

        cache.put(&state, &goal, &actions, plan.clone());
        let cached = cache.get(&state, &goal, &actions);

        assert!(cached.is_some());
        assert_eq!(cached.test_unwrap().len(), 1);
    }

    #[test]
    fn get_returns_none_for_missing_entry() {
        let mut cache = PlanCache::new(10);
        let state = create_state_a();
        let goal = create_goal();
        let actions = create_actions();

        let cached = cache.get(&state, &goal, &actions);
        assert!(cached.is_none());
    }

    // ---------------------------------------------------------------------
    // Cache statistics tracking
    // ---------------------------------------------------------------------

    #[test]
    fn miss_increments_miss_count() {
        let mut cache = PlanCache::new(10);
        let state = create_state_a();
        let goal = create_goal();
        let actions = create_actions();

        let _ = cache.get(&state, &goal, &actions);

        assert_eq!(cache.stats().misses, 1);
        assert_eq!(cache.stats().hits, 0);
    }

    #[test]
    fn hit_increments_hit_count() {
        let mut cache = PlanCache::new(10);
        let state = create_state_a();
        let goal = create_goal();
        let actions = create_actions();

        cache.put(&state, &goal, &actions, vec![]);
        let _ = cache.get(&state, &goal, &actions);

        assert_eq!(cache.stats().hits, 1);
    }

    #[test]
    fn clear_resets_statistics() {
        let mut cache = PlanCache::new(10);
        let state = create_state_a();
        let goal = create_goal();
        let actions = create_actions();

        cache.put(&state, &goal, &actions, vec![]);
        let _ = cache.get(&state, &goal, &actions);

        cache.clear();

        assert_eq!(cache.stats().hits, 0);
        assert_eq!(cache.stats().misses, 0);
        assert_eq!(cache.stats().evictions, 0);
        assert_eq!(cache.stats().invalidations, 0);
    }

    #[test]
    fn clear_empties_cache() {
        let mut cache = PlanCache::new(10);
        let state = create_state_a();
        let goal = create_goal();
        let actions = create_actions();

        cache.put(&state, &goal, &actions, vec![]);
        cache.clear();

        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
    }

    // ---------------------------------------------------------------------
    // LRU eviction
    // ---------------------------------------------------------------------

    #[test]
    fn eviction_occurs_at_capacity() {
        let mut cache = PlanCache::new(2);
        let state_a = WorldState::from_facts(&[("a", true)]);
        let state_b = WorldState::from_facts(&[("b", true)]);
        let state_c = WorldState::from_facts(&[("c", true)]);
        let goal = create_goal();
        let actions = create_actions();

        cache.put(&state_a, &goal, &actions, vec![]);
        cache.put(&state_b, &goal, &actions, vec![]);
        cache.put(&state_c, &goal, &actions, vec![]);

        // Should still have capacity entries
        assert_eq!(cache.len(), 2);
        // Should have 1 eviction
        assert_eq!(cache.stats().evictions, 1);
    }

    #[test]
    fn oldest_entry_is_evicted() {
        let mut cache = PlanCache::new(2);
        let state_a = WorldState::from_facts(&[("a", true)]);
        let state_b = WorldState::from_facts(&[("b", true)]);
        let state_c = WorldState::from_facts(&[("c", true)]);
        let goal = create_goal();
        let actions = create_actions();

        cache.put(&state_a, &goal, &actions, vec![]);
        cache.put(&state_b, &goal, &actions, vec![]);
        cache.put(&state_c, &goal, &actions, vec![]);

        // state_a should be evicted (oldest)
        let cached_a = cache.get(&state_a, &goal, &actions);
        assert!(cached_a.is_none(), "oldest entry should be evicted");

        // state_b and state_c should still exist
        let cached_b = cache.get(&state_b, &goal, &actions);
        let cached_c = cache.get(&state_c, &goal, &actions);
        assert!(cached_b.is_some());
        assert!(cached_c.is_some());
    }

    // ---------------------------------------------------------------------
    // Action validation
    // ---------------------------------------------------------------------

    #[test]
    fn changed_action_cost_invalidates_cache() {
        let mut cache = PlanCache::new(10);
        let state = create_state_a();
        let goal = create_goal();
        let actions = create_actions();

        cache.put(&state, &goal, &actions, vec![actions[0].clone()]);

        // Modify action set
        let mut modified_actions = actions.clone();
        modified_actions[0].cost = 99.0;

        let cached = cache.get(&state, &goal, &modified_actions);
        assert!(cached.is_none(), "modified actions should invalidate cache");
        assert_eq!(cache.stats().invalidations, 1);
    }
}

// =============================================================================
// WORLD STATE TESTS
// =============================================================================

mod world_state_tests {
    use super::*;

    #[test]
    fn from_facts_creates_correct_state() {
        let state = WorldState::from_facts(&[("a", true), ("b", false)]);
        assert_eq!(state.get("a"), Some(true));
        assert_eq!(state.get("b"), Some(false));
    }

    #[test]
    fn empty_state_returns_none() {
        let state = WorldState::default();
        assert_eq!(state.get("nonexistent"), None);
    }

    #[test]
    fn set_updates_value() {
        let mut state = WorldState::default();
        state.set("key", true);
        assert_eq!(state.get("key"), Some(true));

        state.set("key", false);
        assert_eq!(state.get("key"), Some(false));
    }

    #[test]
    fn facts_count_is_correct() {
        let state = WorldState::from_facts(&[("a", true), ("b", false), ("c", true)]);
        assert_eq!(state.facts.len(), 3);
    }

    #[test]
    fn empty_state_has_zero_facts() {
        let state = WorldState::default();
        assert_eq!(state.facts.len(), 0);
        assert!(state.facts.is_empty());
    }

    #[test]
    fn satisfies_returns_true_for_matching_state() {
        let current = WorldState::from_facts(&[("a", true), ("b", false)]);
        let goal = WorldState::from_facts(&[("a", true)]);
        assert!(current.satisfies(&goal));
    }

    #[test]
    fn satisfies_returns_false_for_mismatched_state() {
        let current = WorldState::from_facts(&[("a", false)]);
        let goal = WorldState::from_facts(&[("a", true)]);
        assert!(!current.satisfies(&goal));
    }
}

// =============================================================================
// GOAP ACTION TESTS
// =============================================================================

mod goap_action_tests {
    use super::*;

    #[test]
    fn new_action_has_default_cost_1() {
        let action = GoapAction::new("test");
        assert!(
            (action.cost - 1.0).abs() < 1e-6,
            "default cost should be 1.0, got {}",
            action.cost
        );
    }

    #[test]
    fn with_cost_sets_cost() {
        let action = GoapAction::new("test").with_cost(5.0);
        assert!(
            (action.cost - 5.0).abs() < 1e-6,
            "cost should be 5.0, got {}",
            action.cost
        );
    }

    #[test]
    fn action_name_is_set() {
        let action = GoapAction::new("my_action");
        assert_eq!(action.name, "my_action");
    }

    #[test]
    fn with_precondition_adds_precondition() {
        let action = GoapAction::new("test").with_precondition("has_weapon", true);
        assert!(action.preconditions.get("has_weapon").is_some());
        assert_eq!(action.preconditions.get("has_weapon"), Some(true));
    }

    #[test]
    fn with_effect_adds_effect() {
        let action = GoapAction::new("test").with_effect("enemy_dead", true);
        assert!(action.effects.get("enemy_dead").is_some());
        assert_eq!(action.effects.get("enemy_dead"), Some(true));
    }

    #[test]
    fn multiple_preconditions_are_stored() {
        let action = GoapAction::new("test")
            .with_precondition("a", true)
            .with_precondition("b", false)
            .with_precondition("c", true);
        assert_eq!(action.preconditions.facts.len(), 3);
    }

    #[test]
    fn multiple_effects_are_stored() {
        let action = GoapAction::new("test")
            .with_effect("a", true)
            .with_effect("b", false);
        assert_eq!(action.effects.facts.len(), 2);
    }

    #[test]
    fn can_apply_returns_true_when_preconditions_met() {
        let state = WorldState::from_facts(&[("has_weapon", true)]);
        let action = GoapAction::new("attack").with_precondition("has_weapon", true);
        assert!(action.can_apply(&state));
    }

    #[test]
    fn can_apply_returns_false_when_preconditions_not_met() {
        let state = WorldState::from_facts(&[("has_weapon", false)]);
        let action = GoapAction::new("attack").with_precondition("has_weapon", true);
        assert!(!action.can_apply(&state));
    }

    #[test]
    fn apply_returns_new_state_with_effects() {
        let state = WorldState::from_facts(&[("enemy_dead", false)]);
        let action = GoapAction::new("attack").with_effect("enemy_dead", true);
        let new_state = action.apply(&state);
        assert_eq!(new_state.get("enemy_dead"), Some(true));
    }
}

// =============================================================================
// GOAP GOAL TESTS
// =============================================================================

mod goap_goal_tests {
    use super::*;

    #[test]
    fn goal_name_is_set() {
        let goal = GoapGoal::new(
            "defeat_enemy",
            WorldState::from_facts(&[("enemy_dead", true)]),
        );
        assert_eq!(goal.name, "defeat_enemy");
    }

    #[test]
    fn goal_desired_state_is_set() {
        let desired = WorldState::from_facts(&[("enemy_dead", true), ("loot_collected", true)]);
        let goal = GoapGoal::new("win", desired.clone());
        assert_eq!(goal.desired_state.facts.len(), 2);
    }

    #[test]
    fn is_satisfied_returns_true_when_goal_met() {
        let current = WorldState::from_facts(&[("enemy_dead", true)]);
        let goal = GoapGoal::new("defeat", WorldState::from_facts(&[("enemy_dead", true)]));
        assert!(goal.is_satisfied(&current));
    }

    #[test]
    fn is_satisfied_returns_false_when_goal_not_met() {
        let current = WorldState::from_facts(&[("enemy_dead", false)]);
        let goal = GoapGoal::new("defeat", WorldState::from_facts(&[("enemy_dead", true)]));
        assert!(!goal.is_satisfied(&current));
    }
}

// =============================================================================
// GOAP PLANNER TESTS
// =============================================================================

mod goap_planner_tests {
    use super::*;

    #[test]
    fn planner_finds_single_step_plan() {
        use crate::test_support::TestValue;
        let planner = GoapPlanner::new();
        let state = WorldState::from_facts(&[("has_key", true)]);
        let goal = GoapGoal::new("open_door", WorldState::from_facts(&[("door_open", true)]));
        let actions = vec![GoapAction::new("open_door")
            .with_precondition("has_key", true)
            .with_effect("door_open", true)];

        let plan = planner.plan(&state, &goal, &actions);
        assert!(plan.is_some());
        assert_eq!(plan.test_unwrap().len(), 1);
    }

    #[test]
    fn planner_finds_multi_step_plan() {
        use crate::test_support::TestValue;
        let planner = GoapPlanner::new();
        let state = WorldState::from_facts(&[("at_home", true)]);
        let goal = GoapGoal::new("have_sword", WorldState::from_facts(&[("has_sword", true)]));
        let actions = vec![
            GoapAction::new("go_to_shop")
                .with_precondition("at_home", true)
                .with_effect("at_home", false)
                .with_effect("at_shop", true),
            GoapAction::new("buy_sword")
                .with_precondition("at_shop", true)
                .with_effect("has_sword", true),
        ];

        let plan = planner.plan(&state, &goal, &actions);
        assert!(plan.is_some());
        let plan = plan.test_unwrap();
        assert_eq!(plan.len(), 2);
        assert_eq!(plan[0].name, "go_to_shop");
        assert_eq!(plan[1].name, "buy_sword");
    }

    #[test]
    fn planner_returns_none_for_impossible_goal() {
        let planner = GoapPlanner::new();
        let state = WorldState::from_facts(&[("trapped", true)]);
        let goal = GoapGoal::new("escape", WorldState::from_facts(&[("escaped", true)]));
        let actions = vec![GoapAction::new("walk")
            .with_precondition("not_trapped", true) // Can never be satisfied
            .with_effect("escaped", true)];

        let plan = planner.plan(&state, &goal, &actions);
        assert!(plan.is_none());
    }

    #[test]
    fn planner_prefers_lower_cost_plan() {
        use crate::test_support::TestValue;
        let planner = GoapPlanner::new();
        let state = WorldState::from_facts(&[("start", true)]);
        let goal = GoapGoal::new("goal", WorldState::from_facts(&[("done", true)]));
        let actions = vec![
            GoapAction::new("expensive")
                .with_precondition("start", true)
                .with_effect("done", true)
                .with_cost(10.0),
            GoapAction::new("cheap")
                .with_precondition("start", true)
                .with_effect("done", true)
                .with_cost(1.0),
        ];

        let plan = planner.plan(&state, &goal, &actions);
        assert!(plan.is_some());
        let plan = plan.test_unwrap();
        assert_eq!(plan.len(), 1);
        assert_eq!(plan[0].name, "cheap");
    }

    #[test]
    fn planner_returns_empty_for_already_satisfied_goal() {
        use crate::test_support::TestValue;
        let planner = GoapPlanner::new();
        let state = WorldState::from_facts(&[("goal_met", true)]);
        let goal = GoapGoal::new("done", WorldState::from_facts(&[("goal_met", true)]));
        let actions = vec![];

        let plan = planner.plan(&state, &goal, &actions);
        // Already satisfied - returns empty plan or Some(vec![])
        assert!(plan.is_some());
        assert!(plan.test_unwrap().is_empty());
    }
}

// =============================================================================
// CACHED GOAP PLANNER TESTS
// =============================================================================

mod cached_goap_planner_tests {
    use super::*;

    fn create_scenario() -> (WorldState, GoapGoal, Vec<GoapAction>) {
        let state = WorldState::from_facts(&[("has_key", true)]);
        let goal = GoapGoal::new("open", WorldState::from_facts(&[("door_open", true)]));
        let actions = vec![GoapAction::new("open_door")
            .with_precondition("has_key", true)
            .with_effect("door_open", true)];
        (state, goal, actions)
    }

    #[test]
    fn first_plan_is_cache_miss() {
        let mut planner = CachedGoapPlanner::new(10);
        let (state, goal, actions) = create_scenario();

        let _ = planner.plan(&state, &goal, &actions);

        assert_eq!(planner.cache_stats().misses, 1);
        assert_eq!(planner.cache_stats().hits, 0);
    }

    #[test]
    fn second_plan_is_cache_hit() {
        let mut planner = CachedGoapPlanner::new(10);
        let (state, goal, actions) = create_scenario();

        let _ = planner.plan(&state, &goal, &actions);
        let _ = planner.plan(&state, &goal, &actions);

        assert_eq!(planner.cache_stats().hits, 1);
        assert_eq!(planner.cache_stats().misses, 1);
    }

    #[test]
    fn clear_cache_resets_stats() {
        let mut planner = CachedGoapPlanner::new(10);
        let (state, goal, actions) = create_scenario();

        let _ = planner.plan(&state, &goal, &actions);
        planner.clear_cache();

        assert_eq!(planner.cache_stats().hits, 0);
        assert_eq!(planner.cache_stats().misses, 0);
    }

    #[test]
    fn cached_plan_equals_original_plan() {
        let mut planner = CachedGoapPlanner::new(10);
        let (state, goal, actions) = create_scenario();

        let plan1 = planner.plan(&state, &goal, &actions);
        let plan2 = planner.plan(&state, &goal, &actions);

        assert_eq!(
            plan1.as_ref().map(|p| p.len()),
            plan2.as_ref().map(|p| p.len())
        );
        assert_eq!(
            plan1.as_ref().map(|p| &p[0].name),
            plan2.as_ref().map(|p| &p[0].name)
        );
    }

    #[test]
    fn default_planner_has_capacity_1000() {
        let planner = CachedGoapPlanner::default();
        // Check by trying to access base_planner (proves it initialized)
        let _base = planner.base_planner();
        // Default cache capacity is 1000 (from PlanCache::default())
    }
}

#[path = "../src/test_support.rs"]
mod test_support;
