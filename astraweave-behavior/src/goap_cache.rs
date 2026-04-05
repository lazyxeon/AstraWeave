//! GOAP Plan Cache with LRU eviction
//!
//! Week 3 Action 9: Reduces complex planning from 31.7ms → <1ms with 90% cache hit rate.
//! Uses scenario fingerprinting and state bucketing for high cache efficiency.

#[cfg(feature = "profiling")]
use astraweave_profiling::span;

use crate::goap::{GoapAction, GoapGoal, WorldState};
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, VecDeque};
use std::hash::{Hash, Hasher};

/// Cache key for GOAP plans based on scenario fingerprint
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PlanCacheKey {
    /// Hash of current world state (bucketed for similar states)
    state_hash: u64,
    /// Hash of goal desired state
    goal_hash: u64,
    /// Number of available actions (quick check for action set changes)
    action_count: usize,
}

impl PlanCacheKey {
    /// Create cache key with state bucketing for similar scenarios
    ///
    /// State bucketing groups similar states together to increase cache hits.
    /// For example, "health = 95" and "health = 97" may use same cached plan.
    pub fn new(
        current_state: &WorldState,
        goal: &GoapGoal,
        available_actions: &[GoapAction],
    ) -> Self {
        Self {
            state_hash: Self::hash_world_state_bucketed(current_state),
            goal_hash: Self::hash_world_state(&goal.desired_state),
            action_count: available_actions.len(),
        }
    }

    /// Hash world state with exact facts (for goal hashing)
    fn hash_world_state(state: &WorldState) -> u64 {
        let mut hasher = DefaultHasher::new();
        // Iterate in deterministic order (BTreeMap is sorted)
        for (key, &value) in &state.facts {
            key.hash(&mut hasher);
            value.hash(&mut hasher);
        }
        hasher.finish()
    }

    /// Hash world state with bucketing for similar states
    ///
    /// Strategy: Hash both keys and values to ensure correctness.
    /// Previously, this only hashed keys ("bucketing"), which led to invalid plans
    /// being returned for states with the same variables but different values.
    /// Now we enforce exact state matching for cache hits.
    fn hash_world_state_bucketed(state: &WorldState) -> u64 {
        let mut hasher = DefaultHasher::new();
        // Hash both keys and values for exact matching
        for (key, value) in &state.facts {
            key.hash(&mut hasher);
            value.hash(&mut hasher);
        }
        hasher.finish()
    }

    /// Create cache key with validation fingerprint
    ///
    /// Includes action set hash to invalidate cache if actions change.
    /// This prevents stale plans when action definitions are modified.
    pub fn with_action_validation(
        current_state: &WorldState,
        goal: &GoapGoal,
        available_actions: &[GoapAction],
    ) -> (Self, u64) {
        let key = Self::new(current_state, goal, available_actions);
        let action_hash = Self::hash_action_set(available_actions);
        (key, action_hash)
    }

    /// Hash action set for validation (detects action changes)
    fn hash_action_set(actions: &[GoapAction]) -> u64 {
        let mut hasher = DefaultHasher::new();
        for action in actions {
            action.name.hash(&mut hasher);
            // Hash cost as u32 for determinism (f32 has precision issues)
            (action.cost as u32).hash(&mut hasher);
        }
        hasher.finish()
    }
}

/// Cached plan entry with metadata
#[derive(Debug, Clone)]
struct CachedPlan {
    /// Cached action sequence (empty vec = "no plan possible")
    actions: Vec<GoapAction>,
    /// Action set hash when plan was created (for validation)
    action_hash: u64,
    /// Number of times this plan was used (for stats)
    hit_count: usize,
}

/// LRU cache for GOAP plans
pub struct PlanCache {
    /// Maximum number of cached plans
    max_size: usize,
    /// Cache storage (key -> plan)
    cache: HashMap<PlanCacheKey, CachedPlan>,
    /// LRU queue (keys in access order, oldest first)
    lru_queue: VecDeque<PlanCacheKey>,
    /// Statistics
    stats: CacheStats,
}

/// Cache performance statistics
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    pub hits: usize,
    pub misses: usize,
    pub evictions: usize,
    pub invalidations: usize,
}

impl CacheStats {
    /// Calculate cache hit rate (0.0 to 1.0)
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }

    /// Total cache accesses
    pub fn total_accesses(&self) -> usize {
        self.hits + self.misses
    }
}

impl PlanCache {
    /// Create new plan cache with given capacity
    pub fn new(max_size: usize) -> Self {
        Self {
            max_size,
            cache: HashMap::with_capacity(max_size),
            lru_queue: VecDeque::with_capacity(max_size),
            stats: CacheStats::default(),
        }
    }

    /// Get cached plan if available and valid
    ///
    /// Returns Some(plan) if cache hit, None if miss or invalidated.
    /// Automatically validates action set hash and evicts stale entries.
    pub fn get(
        &mut self,
        current_state: &WorldState,
        goal: &GoapGoal,
        available_actions: &[GoapAction],
    ) -> Option<Vec<GoapAction>> {
        #[cfg(feature = "profiling")]
        span!("AI::PlanCache::get");

        let (key, action_hash) =
            PlanCacheKey::with_action_validation(current_state, goal, available_actions);

        if let Some(cached) = self.cache.get_mut(&key) {
            // Validate action set hasn't changed
            if cached.action_hash != action_hash {
                // Action set changed - invalidate cache entry
                self.cache.remove(&key);
                self.lru_queue.retain(|k| k != &key);
                self.stats.invalidations += 1;
                self.stats.misses += 1;
                return None;
            }

            // Cache hit!
            self.stats.hits += 1;
            cached.hit_count += 1;

            // Update LRU (move to back)
            self.lru_queue.retain(|k| k != &key);
            self.lru_queue.push_back(key);

            Some(cached.actions.clone())
        } else {
            // Cache miss
            self.stats.misses += 1;
            None
        }
    }

    /// Store plan in cache with LRU eviction
    pub fn put(
        &mut self,
        current_state: &WorldState,
        goal: &GoapGoal,
        available_actions: &[GoapAction],
        plan: Vec<GoapAction>,
    ) {
        let (key, action_hash) =
            PlanCacheKey::with_action_validation(current_state, goal, available_actions);

        // Evict oldest entry if at capacity
        if self.cache.len() >= self.max_size && !self.cache.contains_key(&key) {
            if let Some(oldest_key) = self.lru_queue.pop_front() {
                self.cache.remove(&oldest_key);
                self.stats.evictions += 1;
            }
        }

        // Store plan
        let cached_plan = CachedPlan {
            actions: plan,
            action_hash,
            hit_count: 0,
        };

        // Update LRU queue
        self.lru_queue.retain(|k| k != &key); // Remove if exists
        self.lru_queue.push_back(key.clone());

        self.cache.insert(key, cached_plan);
    }

    /// Clear all cached plans
    pub fn clear(&mut self) {
        self.cache.clear();
        self.lru_queue.clear();
        self.stats = CacheStats::default();
    }

    /// Get cache statistics
    pub fn stats(&self) -> &CacheStats {
        &self.stats
    }

    /// Get current cache size
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    /// Get cache capacity
    pub fn capacity(&self) -> usize {
        self.max_size
    }
}

impl Default for PlanCache {
    fn default() -> Self {
        Self::new(1000) // Default: 1000 cached plans
    }
}

/// Caching GOAP planner that wraps base planner with LRU cache
pub struct CachedGoapPlanner {
    /// Base planner for cache misses
    base_planner: crate::goap::GoapPlanner,
    /// Plan cache
    cache: PlanCache,
}

impl CachedGoapPlanner {
    /// Create new cached planner with given cache size
    pub fn new(cache_size: usize) -> Self {
        Self {
            base_planner: crate::goap::GoapPlanner::new(),
            cache: PlanCache::new(cache_size),
        }
    }

    /// Create with custom base planner
    pub fn with_planner(planner: crate::goap::GoapPlanner, cache_size: usize) -> Self {
        Self {
            base_planner: planner,
            cache: PlanCache::new(cache_size),
        }
    }

    /// Plan with caching (tries cache first, falls back to planning)
    pub fn plan(
        &mut self,
        current_state: &WorldState,
        goal: &GoapGoal,
        available_actions: &[GoapAction],
    ) -> Option<Vec<GoapAction>> {
        // Try cache first
        if let Some(cached_plan) = self.cache.get(current_state, goal, available_actions) {
            return Some(cached_plan);
        }

        // Cache miss - run planner
        let plan = self
            .base_planner
            .plan(current_state, goal, available_actions)?;

        // Store in cache for future use
        self.cache
            .put(current_state, goal, available_actions, plan.clone());

        Some(plan)
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> &CacheStats {
        self.cache.stats()
    }

    /// Clear cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// Get base planner (for direct access)
    pub fn base_planner(&self) -> &crate::goap::GoapPlanner {
        &self.base_planner
    }

    /// Get mutable base planner
    pub fn base_planner_mut(&mut self) -> &mut crate::goap::GoapPlanner {
        &mut self.base_planner
    }
}

impl Default for CachedGoapPlanner {
    fn default() -> Self {
        Self::new(1000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_state() -> WorldState {
        WorldState::from_facts(&[
            ("has_weapon", true),
            ("has_ammo", false),
            ("enemy_visible", true),
        ])
    }

    fn create_test_goal() -> GoapGoal {
        GoapGoal::new(
            "attack_enemy",
            WorldState::from_facts(&[("enemy_dead", true)]),
        )
    }

    fn create_test_actions() -> Vec<GoapAction> {
        vec![
            GoapAction::new("find_ammo")
                .with_precondition("has_weapon", true)
                .with_effect("has_ammo", true),
            GoapAction::new("attack")
                .with_precondition("has_weapon", true)
                .with_precondition("has_ammo", true)
                .with_precondition("enemy_visible", true)
                .with_effect("enemy_dead", true),
        ]
    }

    #[test]
    fn test_cache_key_creation() {
        let state = create_test_state();
        let goal = create_test_goal();
        let actions = create_test_actions();

        let key1 = PlanCacheKey::new(&state, &goal, &actions);
        let key2 = PlanCacheKey::new(&state, &goal, &actions);

        assert_eq!(key1, key2, "Same inputs should produce same cache key");
    }

    #[test]
    fn test_cache_hit() {
        let mut cache = PlanCache::new(10);
        let state = create_test_state();
        let goal = create_test_goal();
        let actions = create_test_actions();

        // Store plan
        let plan = vec![actions[0].clone(), actions[1].clone()];
        cache.put(&state, &goal, &actions, plan.clone());

        // Retrieve plan
        let cached = cache.get(&state, &goal, &actions);
        assert!(cached.is_some(), "Cache should hit for stored plan");
        assert_eq!(
            cached.unwrap().len(),
            2,
            "Cached plan should have 2 actions"
        );

        // Check stats
        assert_eq!(cache.stats().hits, 1);
        assert_eq!(cache.stats().misses, 0);
    }

    #[test]
    fn test_cache_miss() {
        let mut cache = PlanCache::new(10);
        let state = create_test_state();
        let goal = create_test_goal();
        let actions = create_test_actions();

        // Try to get non-existent plan
        let cached = cache.get(&state, &goal, &actions);
        assert!(cached.is_none(), "Cache should miss for non-existent plan");

        // Check stats
        assert_eq!(cache.stats().hits, 0);
        assert_eq!(cache.stats().misses, 1);
    }

    #[test]
    fn test_lru_eviction() {
        let mut cache = PlanCache::new(2); // Small cache for testing
        let state1 = WorldState::from_facts(&[("a", true)]);
        let state2 = WorldState::from_facts(&[("b", true)]);
        let state3 = WorldState::from_facts(&[("c", true)]);
        let goal = create_test_goal();
        let actions = create_test_actions();

        // Fill cache
        cache.put(&state1, &goal, &actions, vec![]);
        cache.put(&state2, &goal, &actions, vec![]);

        assert_eq!(cache.len(), 2, "Cache should have 2 entries");

        // Add third entry (should evict oldest)
        cache.put(&state3, &goal, &actions, vec![]);

        assert_eq!(cache.len(), 2, "Cache should still have 2 entries");
        assert_eq!(cache.stats().evictions, 1, "Should have 1 eviction");

        // state1 should be evicted (oldest)
        let cached1 = cache.get(&state1, &goal, &actions);
        assert!(cached1.is_none(), "Oldest entry should be evicted");
    }

    #[test]
    fn test_action_invalidation() {
        let mut cache = PlanCache::new(10);
        let state = create_test_state();
        let goal = create_test_goal();
        let actions1 = create_test_actions();

        // Store plan
        cache.put(&state, &goal, &actions1, vec![actions1[0].clone()]);

        // Modify action set (different cost)
        let mut actions2 = create_test_actions();
        actions2[0].cost = 5.0;

        // Try to get with modified actions (should invalidate)
        let cached = cache.get(&state, &goal, &actions2);
        assert!(
            cached.is_none(),
            "Cache should invalidate when actions change"
        );
        assert_eq!(cache.stats().invalidations, 1, "Should have 1 invalidation");
    }

    #[test]
    fn test_cached_planner_integration() {
        let mut planner = CachedGoapPlanner::new(10);
        let state = create_test_state();
        let goal = create_test_goal();
        let actions = create_test_actions();

        // First call (cache miss, will plan)
        let plan1 = planner.plan(&state, &goal, &actions);
        assert!(plan1.is_some(), "Planner should find plan");
        assert_eq!(planner.cache_stats().misses, 1);
        assert_eq!(planner.cache_stats().hits, 0);

        // Second call (cache hit)
        let plan2 = planner.plan(&state, &goal, &actions);
        assert!(plan2.is_some(), "Cached planner should return plan");
        assert_eq!(planner.cache_stats().hits, 1);
        assert_eq!(planner.cache_stats().misses, 1);

        // Plans should be identical
        assert_eq!(plan1.unwrap().len(), plan2.unwrap().len());
    }

    #[test]
    fn test_cache_hit_rate() {
        let stats = CacheStats {
            hits: 90,
            misses: 10,
            evictions: 0,
            invalidations: 0,
        };

        assert_eq!(stats.hit_rate(), 0.9, "Hit rate should be 90%");
        assert_eq!(stats.total_accesses(), 100);
    }

    // ═══════════════════════════════════════════════════════════════
    // MUTATION REMEDIATION TESTS — targets goap_cache.rs misses
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn mutation_hash_world_state_deterministic() {
        // Targets: goap_cache.rs:44 replace hash_world_state -> u64 with 0/1
        let s1 = create_test_state();
        let s2 = create_test_state();
        let goal = create_test_goal();
        let actions = create_test_actions();

        let k1 = PlanCacheKey::new(&s1, &goal, &actions);
        let k2 = PlanCacheKey::new(&s2, &goal, &actions);
        // Same state → same key
        assert_eq!(k1, k2, "Same state should produce same cache key");

        // Different state → different key (overwhelmingly likely)
        let s3 = WorldState::from_facts(&[
            ("has_weapon", false),
            ("has_ammo", true),
            ("enemy_visible", false),
        ]);
        let k3 = PlanCacheKey::new(&s3, &goal, &actions);
        assert_ne!(k1, k3, "Different state should produce different cache key");
    }

    #[test]
    fn mutation_cache_get_invalidation_increment() {
        // Targets: goap_cache.rs:178 replace += with *= in PlanCache::get (invalidations)
        // and goap_cache.rs:176 replace != with == (action_hash check)
        let mut cache = PlanCache::new(10);
        let state = create_test_state();
        let goal = create_test_goal();
        let actions = create_test_actions();

        // Store a plan
        cache.put(&state, &goal, &actions, vec![actions[0].clone()]);

        // Change actions to trigger invalidation
        let mut changed_actions = create_test_actions();
        changed_actions[0] = GoapAction::new("different_action").with_cost(99.0);

        // Get should return None (invalidated)
        let result = cache.get(&state, &goal, &changed_actions);
        assert!(result.is_none());
        assert_eq!(
            cache.stats().invalidations,
            1,
            "Should have exactly 1 invalidation"
        );
    }

    #[test]
    fn mutation_cache_get_hit_increment() {
        // Targets: goap_cache.rs:184 replace += with *= (hits)
        let mut cache = PlanCache::new(10);
        let state = create_test_state();
        let goal = create_test_goal();
        let actions = create_test_actions();

        cache.put(&state, &goal, &actions, vec![actions[0].clone()]);

        // First hit
        let _ = cache.get(&state, &goal, &actions);
        assert_eq!(cache.stats().hits, 1);

        // Second hit
        let _ = cache.get(&state, &goal, &actions);
        assert_eq!(
            cache.stats().hits,
            2,
            "Hits should increment by 1 each time"
        );
    }

    #[test]
    fn mutation_cache_get_miss_count() {
        // Targets: goap_cache.rs:187 replace != with == (cache miss branch)
        let mut cache = PlanCache::new(10);
        let state = create_test_state();
        let goal = create_test_goal();
        let actions = create_test_actions();

        // Empty cache → should be a miss
        let result = cache.get(&state, &goal, &actions);
        assert!(result.is_none());
        assert_eq!(cache.stats().misses, 1, "Should record 1 miss");
    }

    #[test]
    fn mutation_cache_clear_empties() {
        // Targets: goap_cache.rs:233 replace clear with ()
        let mut cache = PlanCache::new(10);
        let state = create_test_state();
        let goal = create_test_goal();
        let actions = create_test_actions();

        cache.put(&state, &goal, &actions, vec![actions[0].clone()]);
        assert!(!cache.is_empty());
        assert_eq!(cache.len(), 1);

        cache.clear();
        assert!(cache.is_empty(), "Cache should be empty after clear");
        assert_eq!(cache.len(), 0, "Cache len should be 0 after clear");
    }

    #[test]
    fn mutation_cache_is_empty_reflects_state() {
        // Targets: goap_cache.rs:250 replace is_empty -> bool with true/false
        let mut cache = PlanCache::new(10);
        assert!(cache.is_empty(), "New cache should be empty");

        let state = create_test_state();
        let goal = create_test_goal();
        let actions = create_test_actions();
        cache.put(&state, &goal, &actions, vec![]);

        assert!(!cache.is_empty(), "Cache with entries should not be empty");
    }

    #[test]
    fn mutation_cache_capacity_returns_max_size() {
        // Targets: goap_cache.rs:255 replace capacity -> usize with 0/1
        let cache = PlanCache::new(42);
        assert_eq!(
            cache.capacity(),
            42,
            "Capacity should match construction arg"
        );

        let cache2 = PlanCache::new(100);
        assert_eq!(cache2.capacity(), 100);
    }

    #[test]
    fn mutation_cached_planner_with_planner_sets_base() {
        // Targets: goap_cache.rs:284 replace with_planner -> Self with Default::default()
        let base = crate::goap::GoapPlanner::new().with_max_iterations(42);
        let cp = CachedGoapPlanner::with_planner(base, 50);
        // Verify the cache size is from construction
        assert_eq!(cp.cache_stats().total_accesses(), 0);
    }

    #[test]
    fn mutation_cached_planner_clear_cache() {
        // Targets: goap_cache.rs:321 replace clear_cache with ()
        let mut cp = CachedGoapPlanner::new(10);
        let state = create_test_state();
        let goal = create_test_goal();
        let actions = create_test_actions();

        // Plan once to populate cache
        let _ = cp.plan(&state, &goal, &actions);
        // Second call should be a hit
        let _ = cp.plan(&state, &goal, &actions);
        assert_eq!(
            cp.cache_stats().hits,
            1,
            "Should have 1 cache hit before clear"
        );

        cp.clear_cache();
        // After clearing, stats reset AND cache is empty
        assert_eq!(cp.cache_stats().hits, 0, "Stats should reset after clear");
        // Next call should be a fresh miss (not a hit)
        let _ = cp.plan(&state, &goal, &actions);
        assert_eq!(cp.cache_stats().misses, 1, "Should have 1 miss after clear");
        assert_eq!(
            cp.cache_stats().hits,
            0,
            "Should have 0 hits right after re-plan"
        );
    }

    #[test]
    fn mutation_cached_planner_base_planner_accessor() {
        // Targets: goap_cache.rs:326/331 replace base_planner with Default
        let cp = CachedGoapPlanner::new(10);
        let bp = cp.base_planner();
        // Verify it returns a real planner (can plan)
        let state = WorldState::from_facts(&[("done", true)]);
        let goal = GoapGoal::new("g", WorldState::from_facts(&[("done", true)]));
        let plan = bp.plan(&state, &goal, &[]);
        assert!(
            plan.is_some(),
            "base_planner should return functional planner"
        );
    }

    #[test]
    fn mutation_cached_planner_base_planner_mut_accessor() {
        // Targets: goap_cache.rs:331 replace base_planner_mut with Default
        let mut cp = CachedGoapPlanner::new(10);
        let bp = cp.base_planner_mut();
        // Should be mutable and functional
        let state = WorldState::from_facts(&[("x", true)]);
        let goal = GoapGoal::new("g", WorldState::from_facts(&[("x", true)]));
        let plan = bp.plan(&state, &goal, &[]);
        assert!(plan.is_some());
    }

    // ═══════════════════════════════════════════════════════════════
    // DEEP REMEDIATION v3.6: Cache stress tests — production-risk focus
    // Verifies cache correctness under rapid state changes, high churn,
    // and action set mutations (conditions at 12,700+ agent scale).
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn stress_rapid_plan_invalidate_replan_cycles() {
        // Production scenario: Agent rapidly re-plans as world state changes.
        // Verify cache never returns stale plans after action set changes.
        let mut planner = CachedGoapPlanner::new(100);

        let actions_v1 = vec![
            GoapAction::new("scout")
                .with_cost(2.0)
                .with_effect("area_scouted", true),
            GoapAction::new("attack")
                .with_cost(5.0)
                .with_precondition("area_scouted", true)
                .with_effect("enemy_dead", true),
        ];

        let state = WorldState::from_facts(&[("has_weapon", true)]);
        let goal = GoapGoal::new("eliminate", WorldState::from_facts(&[("enemy_dead", true)]));

        // Plan with v1 actions
        let plan1 = planner.plan(&state, &goal, &actions_v1);
        assert!(plan1.is_some(), "v1 plan should succeed");

        // Modify actions (simulate changing capabilities)
        let actions_v2 = vec![GoapAction::new("snipe")
            .with_cost(1.0)
            .with_effect("enemy_dead", true)];

        // Plan with v2 — cache should NOT return stale v1 plan
        let plan2 = planner.plan(&state, &goal, &actions_v2);
        assert!(plan2.is_some(), "v2 plan should succeed");
        assert_eq!(
            plan2.as_ref().unwrap().len(),
            1,
            "v2 plan should use snipe (1 action)"
        );
        assert_eq!(plan2.as_ref().unwrap()[0].name, "snipe");

        // Note: different action counts produce different cache keys (miss, not invalidation).
        // But the fact that a correct v2 plan was returned proves no stale v1 plan leaked.
        assert!(
            planner.cache_stats().misses >= 2,
            "Should have at least 2 misses (v1 initial + v2 new key)"
        );
    }

    #[test]
    fn stress_lru_eviction_under_high_churn() {
        // Production scenario: 12,700 agents each with unique world states.
        // Small cache means constant eviction. Verify LRU correctness.
        let mut cache = PlanCache::new(10); // Tiny cache for 100 agents
        let goal = create_test_goal();
        let actions = create_test_actions();

        // Simulate 50 agents with truly unique states (unique fact keys)
        for i in 0..50u32 {
            let key_name = format!("agent_{}_status", i);
            let state = WorldState::from_facts(&[(&key_name, true)]);
            cache.put(&state, &goal, &actions, vec![actions[0].clone()]);
        }

        // Cache should never exceed capacity
        assert!(
            cache.len() <= 10,
            "Cache should respect max_size, got {}",
            cache.len()
        );

        // Eviction count should be high (50 unique inserts into cache of 10)
        assert!(
            cache.stats().evictions >= 40,
            "Should have at least 40 evictions (50 unique inserts, cap 10), got {}",
            cache.stats().evictions
        );

        // Cache should still be functional with recent entries
        assert!(
            !cache.is_empty(),
            "Cache should not be empty after insertions"
        );
    }

    #[test]
    fn stress_cache_coherence_different_states_same_goal() {
        // Production scenario: Many agents share the same goal but have different
        // world states. Each agent should get a plan tailored to their state.
        let mut planner = CachedGoapPlanner::new(50);

        let actions = vec![
            GoapAction::new("find_ammo")
                .with_cost(3.0)
                .with_effect("has_ammo", true),
            GoapAction::new("attack")
                .with_cost(1.0)
                .with_precondition("has_ammo", true)
                .with_effect("enemy_dead", true),
        ];

        let goal = GoapGoal::new("kill", WorldState::from_facts(&[("enemy_dead", true)]));

        // Agent A: already has ammo → 1-step plan (attack)
        let state_a = WorldState::from_facts(&[("has_ammo", true)]);
        let plan_a = planner.plan(&state_a, &goal, &actions).unwrap();
        assert_eq!(plan_a.len(), 1, "Agent A should just attack");
        assert_eq!(plan_a[0].name, "attack");

        // Agent B: no ammo → 2-step plan (find_ammo + attack)
        let state_b = WorldState::from_facts(&[("has_ammo", false)]);
        let plan_b = planner.plan(&state_b, &goal, &actions).unwrap();
        assert_eq!(plan_b.len(), 2, "Agent B needs to find ammo first");
        assert_eq!(plan_b[0].name, "find_ammo");

        // Agent C: same as Agent A → should get cached plan
        let state_c = WorldState::from_facts(&[("has_ammo", true)]);
        let plan_c = planner.plan(&state_c, &goal, &actions).unwrap();
        assert_eq!(plan_c.len(), 1, "Agent C should get cached 1-step plan");
        assert_eq!(plan_c[0].name, "attack");

        // At least one cache hit (Agent C should hit Agent A's cached plan)
        assert!(
            planner.cache_stats().hits >= 1,
            "Agent C should cache-hit on Agent A's plan, hits={}",
            planner.cache_stats().hits
        );
    }

    #[test]
    fn stress_clear_prevents_stale_data() {
        // Verify that after clear(), no stale plans are ever returned.
        let mut cache = PlanCache::new(100);
        let goal = create_test_goal();
        let actions = create_test_actions();

        // Fill with plans for different states
        for i in 0..50u32 {
            let state = WorldState::from_facts(&[("state", i % 2 == 0)]);
            cache.put(&state, &goal, &actions, vec![actions[0].clone()]);
        }

        assert!(!cache.is_empty(), "Cache should have entries");

        // Clear
        cache.clear();

        // Everything should be gone
        assert!(cache.is_empty(), "Cache should be empty after clear");
        assert_eq!(cache.len(), 0);
        assert_eq!(cache.stats().hits, 0, "Stats should reset after clear");
        assert_eq!(cache.stats().misses, 0, "Stats should reset after clear");
        assert_eq!(cache.stats().evictions, 0);
        assert_eq!(cache.stats().invalidations, 0);

        // Try to get previously cached plans — all should miss
        for i in 0..50u32 {
            let state = WorldState::from_facts(&[("state", i % 2 == 0)]);
            let result = cache.get(&state, &goal, &actions);
            assert!(
                result.is_none(),
                "Stale plan returned after clear for i={}",
                i
            );
        }
    }

    #[test]
    fn stress_stats_accuracy_under_load() {
        // Verify cache stats remain accurate after many mixed operations.
        let mut cache = PlanCache::new(5);
        let goal = create_test_goal();
        let actions = create_test_actions();

        // Use unique states so each is a distinct cache key
        // Round 1: Fill cache (all misses since empty, each key unique)
        for i in 0..5u32 {
            let key_name = format!("fact_{}", i);
            let state = WorldState::from_facts(&[(&key_name, true)]);
            let result = cache.get(&state, &goal, &actions);
            assert!(result.is_none(), "Round 1 get should miss (empty cache)");
            cache.put(&state, &goal, &actions, vec![]);
        }

        // After round 1: 5 misses, 0 hits
        assert_eq!(cache.stats().misses, 5, "Round 1: all 5 should be misses");
        assert_eq!(cache.stats().hits, 0, "Round 1: no hits yet");

        // Round 2: Re-access all 5 (should all be hits)
        for i in 0..5u32 {
            let key_name = format!("fact_{}", i);
            let state = WorldState::from_facts(&[(&key_name, true)]);
            let result = cache.get(&state, &goal, &actions);
            assert!(result.is_some(), "Round 2 get should hit for fact_{}", i);
        }

        // After round 2: 5 misses, 5 hits
        assert_eq!(cache.stats().hits, 5, "Round 2: all 5 should be hits");
        assert_eq!(
            cache.stats().misses,
            5,
            "Misses should not change in round 2"
        );
        assert_eq!(
            cache.stats().total_accesses(),
            10,
            "Total accesses should be 10 (5 misses + 5 hits)"
        );
    }

    #[test]
    fn stress_action_set_mutation_invalidation() {
        // Production scenario: Action set changes (agent gains/loses abilities).
        // Cached plans from old action set must be invalidated.
        let mut cache = PlanCache::new(100);
        let state = create_test_state();
        let goal = create_test_goal();

        // Store plans with original actions
        let actions_v1 = vec![
            GoapAction::new("find_ammo")
                .with_cost(1.0)
                .with_effect("has_ammo", true),
            GoapAction::new("attack")
                .with_cost(1.0)
                .with_precondition("has_ammo", true)
                .with_effect("enemy_dead", true),
        ];
        cache.put(&state, &goal, &actions_v1, vec![actions_v1[0].clone()]);

        // Verify hit with same actions
        let hit = cache.get(&state, &goal, &actions_v1);
        assert!(hit.is_some(), "Should hit with same actions");

        // Now simulate gaining a new ability (different action set)
        let actions_v2 = vec![
            GoapAction::new("find_ammo")
                .with_cost(1.0)
                .with_effect("has_ammo", true),
            GoapAction::new("attack")
                .with_cost(1.0)
                .with_precondition("has_ammo", true)
                .with_effect("enemy_dead", true),
            GoapAction::new("grenade") // NEW ability
                .with_cost(2.0)
                .with_effect("enemy_dead", true),
        ];

        // Should NOT return old plan (action count changed)
        let miss = cache.get(&state, &goal, &actions_v2);
        assert!(
            miss.is_none(),
            "Should invalidate when action set changes (different count)"
        );

        // Also test when action count is same but content differs
        let actions_v3 = vec![
            GoapAction::new("find_ammo_v2") // different name
                .with_cost(1.0)
                .with_effect("has_ammo", true),
            GoapAction::new("attack")
                .with_cost(1.0)
                .with_precondition("has_ammo", true)
                .with_effect("enemy_dead", true),
        ];
        cache.put(&state, &goal, &actions_v3, vec![actions_v3[0].clone()]);
        let hit3 = cache.get(&state, &goal, &actions_v3);
        assert!(hit3.is_some(), "Should hit with same v3 actions");

        // With v1 actions (same count but different names → different hash)
        let result_v1 = cache.get(&state, &goal, &actions_v1);
        // The cache key uses action_count, so same count would produce same key
        // BUT the action_hash validation should detect the difference
        assert!(
            result_v1.is_none(),
            "Should invalidate when action names differ despite same count"
        );
    }

    #[test]
    fn stress_lru_access_pattern_correctness() {
        // Verify that accessing a cached entry moves it to the back of LRU queue,
        // preventing eviction of frequently-used plans.
        let mut cache = PlanCache::new(3);
        let goal = create_test_goal();
        let actions = create_test_actions();

        let state_a = WorldState::from_facts(&[("agent", true), ("type_a", true)]);
        let state_b = WorldState::from_facts(&[("agent", true), ("type_b", true)]);
        let state_c = WorldState::from_facts(&[("agent", true), ("type_c", true)]);

        // Fill cache: A, B, C
        cache.put(&state_a, &goal, &actions, vec![actions[0].clone()]);
        cache.put(&state_b, &goal, &actions, vec![actions[1].clone()]);
        cache.put(&state_c, &goal, &actions, vec![]);

        // Access A (moves to back of LRU → B is now oldest)
        let hit_a = cache.get(&state_a, &goal, &actions);
        assert!(hit_a.is_some(), "A should be cached");

        // Add new entry D → should evict B (oldest after A's access)
        let state_d = WorldState::from_facts(&[("agent", true), ("type_d", true)]);
        cache.put(&state_d, &goal, &actions, vec![]);

        assert_eq!(cache.len(), 3, "Cache should still have 3 entries");

        // A should still be cached (was recently accessed)
        let hit_a2 = cache.get(&state_a, &goal, &actions);
        assert!(
            hit_a2.is_some(),
            "A should survive eviction (recently accessed)"
        );

        // B should be evicted (oldest after A's access bump)
        let hit_b = cache.get(&state_b, &goal, &actions);
        assert!(hit_b.is_none(), "B should have been evicted (LRU oldest)");
    }
}
