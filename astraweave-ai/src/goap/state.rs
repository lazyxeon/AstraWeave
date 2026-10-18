use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};

/// State value with deterministic hashing and extended comparison support
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum StateValue {
    Bool(bool),
    Int(i32),
    Float(OrderedFloat),
    String(String),
    /// Integer range for partial matching
    IntRange(i32, i32),
    /// Float with tolerance for approximate matching
    FloatApprox(f32, f32), // (value, epsilon)
}

impl Eq for StateValue {}

impl Hash for StateValue {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            StateValue::Bool(b) => {
                0u8.hash(state);
                b.hash(state);
            }
            StateValue::Int(i) => {
                1u8.hash(state);
                i.hash(state);
            }
            StateValue::Float(f) => {
                2u8.hash(state);
                f.0.to_bits().hash(state);
            }
            StateValue::String(s) => {
                3u8.hash(state);
                s.hash(state);
            }
            StateValue::IntRange(min, max) => {
                4u8.hash(state);
                min.hash(state);
                max.hash(state);
            }
            StateValue::FloatApprox(val, eps) => {
                5u8.hash(state);
                val.to_bits().hash(state);
                eps.to_bits().hash(state);
            }
        }
    }
}

impl StateValue {
    /// Check if this value satisfies a target condition
    pub fn satisfies(&self, target: &StateValue) -> bool {
        match (self, target) {
            // Exact matches
            (StateValue::Bool(a), StateValue::Bool(b)) => a == b,
            (StateValue::String(a), StateValue::String(b)) => a == b,

            // Numeric exact matches
            (StateValue::Int(a), StateValue::Int(b)) => a == b,
            (StateValue::Float(a), StateValue::Float(b)) => (a.0 - b.0).abs() < 1e-6,

            // Range matching
            (StateValue::Int(val), StateValue::IntRange(min, max)) => val >= min && val <= max,
            (StateValue::Float(val), StateValue::FloatApprox(target_val, epsilon)) => {
                (val.0 - target_val).abs() <= *epsilon
            }

            // Mixed type conversions
            (StateValue::Int(i), StateValue::Float(f)) => (*i as f32 - f.0).abs() < 1e-6,
            (StateValue::Float(f), StateValue::Int(i)) => (f.0 - *i as f32).abs() < 1e-6,

            _ => false,
        }
    }

    /// Calculate numeric difference for heuristic estimation
    pub fn numeric_distance(&self, target: &StateValue) -> f32 {
        match (self, target) {
            (StateValue::Int(a), StateValue::Int(b)) => (b - a).abs() as f32,
            (StateValue::Float(a), StateValue::Float(b)) => (b.0 - a.0).abs(),
            (StateValue::Int(a), StateValue::Float(b)) => (b.0 - *a as f32).abs(),
            (StateValue::Float(a), StateValue::Int(b)) => (a.0 - *b as f32).abs(),
            (StateValue::Int(val), StateValue::IntRange(min, max)) => {
                if val < min {
                    (min - val) as f32
                } else if val > max {
                    (val - max) as f32
                } else {
                    0.0
                }
            }
            _ => {
                if self == target {
                    0.0
                } else {
                    1.0
                }
            }
        }
    }
}

/// Wrapper for f32 to make it hashable and comparable
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct OrderedFloat(pub f32);

impl Eq for OrderedFloat {}

impl Hash for OrderedFloat {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.to_bits().hash(state);
    }
}

/// World state represented as a deterministic key-value store
/// Uses BTreeMap for consistent ordering across platforms
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldState {
    state: BTreeMap<String, StateValue>,
}

impl Hash for WorldState {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // BTreeMap iteration is deterministic (sorted by key)
        for (k, v) in &self.state {
            k.hash(state);
            v.hash(state);
        }
    }
}

impl WorldState {
    pub fn new() -> Self {
        Self {
            state: BTreeMap::new(),
        }
    }

    pub fn set(&mut self, key: impl Into<String>, value: StateValue) {
        self.state.insert(key.into(), value);
    }

    pub fn get(&self, key: &str) -> Option<&StateValue> {
        self.state.get(key)
    }

    pub fn remove(&mut self, key: &str) -> Option<StateValue> {
        self.state.remove(key)
    }

    pub fn contains_key(&self, key: &str) -> bool {
        self.state.contains_key(key)
    }

    /// Check if this state satisfies all target conditions
    pub fn satisfies(&self, conditions: &BTreeMap<String, StateValue>) -> bool {
        conditions.iter().all(|(key, target_value)| {
            self.state
                .get(key)
                .map(|current_value| current_value.satisfies(target_value))
                .unwrap_or(false)
        })
    }

    /// Apply effects from an action to this state
    pub fn apply_effects(&mut self, effects: &BTreeMap<String, StateValue>) {
        for (key, value) in effects {
            self.state.insert(key.clone(), value.clone());
        }
    }

    /// Calculate heuristic distance to goal state
    pub fn distance_to(&self, goal: &BTreeMap<String, StateValue>) -> f32 {
        let mut unmet_conditions = 0;
        let mut total_numeric_distance = 0.0;

        for (key, target_value) in goal {
            match self.state.get(key) {
                Some(current_value) => {
                    if !current_value.satisfies(target_value) {
                        unmet_conditions += 1;
                        total_numeric_distance += current_value.numeric_distance(target_value);
                    }
                }
                None => {
                    unmet_conditions += 1;
                    // Missing state is like being at distance 1.0
                    total_numeric_distance += 1.0;
                }
            }
        }

        // Base heuristic: unmet conditions + numeric differences
        unmet_conditions as f32 * 2.0 + total_numeric_distance
    }

    /// Get iterator over state entries (deterministic order)
    pub fn iter(&self) -> impl Iterator<Item = (&String, &StateValue)> {
        self.state.iter()
    }

    /// Number of state variables
    pub fn len(&self) -> usize {
        self.state.len()
    }

    pub fn is_empty(&self) -> bool {
        self.state.is_empty()
    }
}

impl Default for WorldState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::hash_map::DefaultHasher;

    fn hash_state(state: &WorldState) -> u64 {
        let mut hasher = DefaultHasher::new();
        state.hash(&mut hasher);
        hasher.finish()
    }

    #[test]
    fn test_deterministic_hashing() {
        let mut state1 = WorldState::new();
        state1.set("health", StateValue::Int(100));
        state1.set("ammo", StateValue::Int(30));
        state1.set("in_combat", StateValue::Bool(true));

        let mut state2 = WorldState::new();
        // Insert in different order
        state2.set("in_combat", StateValue::Bool(true));
        state2.set("ammo", StateValue::Int(30));
        state2.set("health", StateValue::Int(100));

        // Should produce same hash due to BTreeMap ordering
        assert_eq!(hash_state(&state1), hash_state(&state2));
    }

    #[test]
    fn test_state_value_exact_match() {
        let current = StateValue::Int(50);
        let target = StateValue::Int(50);
        assert!(current.satisfies(&target));

        let current = StateValue::Int(50);
        let target = StateValue::Int(60);
        assert!(!current.satisfies(&target));
    }

    #[test]
    fn test_state_value_range_match() {
        let current = StateValue::Int(75);
        let target = StateValue::IntRange(50, 100);
        assert!(current.satisfies(&target));

        let current = StateValue::Int(30);
        let target = StateValue::IntRange(50, 100);
        assert!(!current.satisfies(&target));
    }

    #[test]
    fn test_state_value_float_approx() {
        let current = StateValue::Float(OrderedFloat(1.501));
        let target = StateValue::FloatApprox(1.5, 0.01);
        assert!(current.satisfies(&target));

        let current = StateValue::Float(OrderedFloat(1.52));
        let target = StateValue::FloatApprox(1.5, 0.01);
        assert!(!current.satisfies(&target));
    }

    #[test]
    fn test_numeric_distance() {
        let current = StateValue::Int(30);
        let target = StateValue::Int(100);
        assert_eq!(current.numeric_distance(&target), 70.0);

        let current = StateValue::Int(75);
        let target = StateValue::IntRange(50, 100);
        assert_eq!(current.numeric_distance(&target), 0.0);

        let current = StateValue::Int(30);
        let target = StateValue::IntRange(50, 100);
        assert_eq!(current.numeric_distance(&target), 20.0);
    }

    #[test]
    fn test_world_state_satisfies() {
        let mut state = WorldState::new();
        state.set("health", StateValue::Int(80));
        state.set("ammo", StateValue::Int(20));
        state.set("weapon_equipped", StateValue::Bool(true));

        let mut conditions = BTreeMap::new();
        conditions.insert("health".to_string(), StateValue::IntRange(50, 100));
        conditions.insert("weapon_equipped".to_string(), StateValue::Bool(true));

        assert!(state.satisfies(&conditions));

        conditions.insert("ammo".to_string(), StateValue::Int(30));
        assert!(!state.satisfies(&conditions));
    }

    #[test]
    fn test_world_state_distance() {
        let mut current = WorldState::new();
        current.set("health", StateValue::Int(30));
        current.set("ammo", StateValue::Int(5));

        let mut goal = BTreeMap::new();
        goal.insert("health".to_string(), StateValue::Int(100));
        goal.insert("ammo".to_string(), StateValue::Int(30));

        let distance = current.distance_to(&goal);
        // 2 unmet conditions (4.0) + numeric distances (70 + 25 = 95)
        assert_eq!(distance, 99.0);
    }

    #[test]
    fn test_apply_effects() {
        let mut state = WorldState::new();
        state.set("health", StateValue::Int(50));
        state.set("in_combat", StateValue::Bool(true));

        let mut effects = BTreeMap::new();
        effects.insert("health".to_string(), StateValue::Int(100));
        effects.insert("in_combat".to_string(), StateValue::Bool(false));
        effects.insert("healed".to_string(), StateValue::Bool(true));

        state.apply_effects(&effects);

        assert_eq!(state.get("health"), Some(&StateValue::Int(100)));
        assert_eq!(state.get("in_combat"), Some(&StateValue::Bool(false)));
        assert_eq!(state.get("healed"), Some(&StateValue::Bool(true)));
    }

    #[test]
    fn test_multiple_hash_calls_consistent() {
        let mut state = WorldState::new();
        state.set("x", StateValue::Int(10));
        state.set("y", StateValue::Int(20));

        let hash1 = hash_state(&state);
        let hash2 = hash_state(&state);
        let hash3 = hash_state(&state);

        assert_eq!(hash1, hash2);
        assert_eq!(hash2, hash3);
    }

    // ── Mutation-killing tests ──

    fn hash_value(v: &StateValue) -> u64 {
        let mut hasher = DefaultHasher::new();
        v.hash(&mut hasher);
        hasher.finish()
    }

    #[test]
    fn test_hash_discriminant_tags_differ() {
        // Each variant must have a unique discriminant tag so that
        // e.g. Bool(true) vs Int(1) produce different hashes
        let bool_val = StateValue::Bool(true);
        let int_val = StateValue::Int(1);
        let float_val = StateValue::Float(OrderedFloat(1.0));
        let string_val = StateValue::String("1".to_string());
        let range_val = StateValue::IntRange(1, 1);
        let approx_val = StateValue::FloatApprox(1.0, 0.0);

        // All pairs must differ — kills tag-swap mutations (0u8↔1u8, etc.)
        let hashes = [
            hash_value(&bool_val),
            hash_value(&int_val),
            hash_value(&float_val),
            hash_value(&string_val),
            hash_value(&range_val),
            hash_value(&approx_val),
        ];
        for i in 0..hashes.len() {
            for j in (i + 1)..hashes.len() {
                assert_ne!(
                    hashes[i], hashes[j],
                    "Hash collision between variant {} and {}",
                    i, j
                );
            }
        }
    }

    #[test]
    fn test_satisfies_bool_exact() {
        assert!(StateValue::Bool(true).satisfies(&StateValue::Bool(true)));
        assert!(StateValue::Bool(false).satisfies(&StateValue::Bool(false)));
        assert!(!StateValue::Bool(true).satisfies(&StateValue::Bool(false)));
        assert!(!StateValue::Bool(false).satisfies(&StateValue::Bool(true)));
    }

    #[test]
    fn test_satisfies_string_exact() {
        let a = StateValue::String("hello".into());
        let b = StateValue::String("hello".into());
        let c = StateValue::String("world".into());
        assert!(a.satisfies(&b));
        assert!(!a.satisfies(&c));
    }

    #[test]
    fn test_satisfies_int_range_boundaries() {
        // Kills >= vs >, <= vs < boundary mutations on IntRange
        let range = StateValue::IntRange(50, 100);
        assert!(!StateValue::Int(49).satisfies(&range)); // below min
        assert!(StateValue::Int(50).satisfies(&range)); // at min
        assert!(StateValue::Int(75).satisfies(&range)); // mid
        assert!(StateValue::Int(100).satisfies(&range)); // at max
        assert!(!StateValue::Int(101).satisfies(&range)); // above max
    }

    #[test]
    fn test_satisfies_float_tolerance() {
        // Kills < vs <= and tolerance value mutations on Float↔Float
        let a = StateValue::Float(OrderedFloat(1.0));
        let b = StateValue::Float(OrderedFloat(1.0));
        assert!(a.satisfies(&b));

        // Within 1e-6 tolerance
        let close = StateValue::Float(OrderedFloat(1.0 + 5e-7));
        assert!(close.satisfies(&b));

        // Outside 1e-6 tolerance
        let far = StateValue::Float(OrderedFloat(1.0 + 2e-6));
        assert!(!far.satisfies(&b));
    }

    #[test]
    fn test_satisfies_float_approx_epsilon() {
        // FloatApprox uses <= epsilon (not < )
        let val = StateValue::Float(OrderedFloat(1.5));
        let approx = StateValue::FloatApprox(1.5, 0.01);
        assert!(val.satisfies(&approx));

        // Exactly at epsilon boundary — should pass (<=)
        let at_boundary = StateValue::Float(OrderedFloat(1.51));
        assert!(at_boundary.satisfies(&approx));

        // Just beyond epsilon
        let beyond = StateValue::Float(OrderedFloat(1.52));
        assert!(!beyond.satisfies(&approx));
    }

    #[test]
    fn test_satisfies_cross_type_int_float() {
        // Int(5) vs Float(5.0) → true
        assert!(StateValue::Int(5).satisfies(&StateValue::Float(OrderedFloat(5.0))));
        // Float(5.0) vs Int(5) → true
        assert!(StateValue::Float(OrderedFloat(5.0)).satisfies(&StateValue::Int(5)));
        // Int(5) vs Float(5.001) → false (outside 1e-6)
        assert!(!StateValue::Int(5).satisfies(&StateValue::Float(OrderedFloat(5.001))));
        // Float(5.001) vs Int(5) → false
        assert!(!StateValue::Float(OrderedFloat(5.001)).satisfies(&StateValue::Int(5)));
    }

    #[test]
    fn test_satisfies_mismatched_types_return_false() {
        // Kills the fallback `_ => false` → `true` mutation
        assert!(!StateValue::Bool(true).satisfies(&StateValue::Int(1)));
        assert!(!StateValue::String("5".into()).satisfies(&StateValue::Int(5)));
        assert!(!StateValue::Int(5).satisfies(&StateValue::String("5".into())));
    }

    #[test]
    fn test_numeric_distance_float_float() {
        let a = StateValue::Float(OrderedFloat(3.0));
        let b = StateValue::Float(OrderedFloat(7.0));
        assert_eq!(a.numeric_distance(&b), 4.0);
        assert_eq!(b.numeric_distance(&a), 4.0); // abs
    }

    #[test]
    fn test_numeric_distance_cross_type() {
        // Int vs Float
        let i = StateValue::Int(3);
        let f = StateValue::Float(OrderedFloat(7.0));
        assert_eq!(i.numeric_distance(&f), 4.0);
        // Float vs Int
        assert_eq!(f.numeric_distance(&i), 4.0);
    }

    #[test]
    fn test_numeric_distance_int_range_clamping() {
        // Kills arithmetic mutations in IntRange arm: min-val vs val-min, etc.
        let range = StateValue::IntRange(50, 100);

        // Below min: distance = min - val = 50 - 30 = 20
        assert_eq!(StateValue::Int(30).numeric_distance(&range), 20.0);
        // Above max: distance = val - max = 120 - 100 = 20
        assert_eq!(StateValue::Int(120).numeric_distance(&range), 20.0);
        // In range: distance = 0
        assert_eq!(StateValue::Int(75).numeric_distance(&range), 0.0);
        // At boundaries
        assert_eq!(StateValue::Int(50).numeric_distance(&range), 0.0);
        assert_eq!(StateValue::Int(100).numeric_distance(&range), 0.0);
        // Just outside boundaries
        assert_eq!(StateValue::Int(49).numeric_distance(&range), 1.0);
        assert_eq!(StateValue::Int(101).numeric_distance(&range), 1.0);
    }

    #[test]
    fn test_numeric_distance_fallback() {
        // Same type (non-numeric): equal → 0, not equal → 1
        let a = StateValue::Bool(true);
        let b = StateValue::Bool(true);
        assert_eq!(a.numeric_distance(&b), 0.0);

        let c = StateValue::Bool(false);
        assert_eq!(a.numeric_distance(&c), 1.0);
    }

    #[test]
    fn test_distance_to_multiplier_and_missing_key() {
        // Kills: `* 2.0` → `* 0.0`, `+ 0`, missing key `+= 1.0` → other
        let mut current = WorldState::new();
        // Set health to something that does NOT satisfy the goal
        current.set("health", StateValue::Int(50));

        let mut goal = BTreeMap::new();
        goal.insert("health".to_string(), StateValue::Int(100));

        // 1 unmet condition = 2.0 base + 50.0 numeric distance = 52.0
        let d = current.distance_to(&goal);
        assert_eq!(d, 52.0);

        // Already satisfied → distance 0
        current.set("health", StateValue::Int(100));
        assert_eq!(current.distance_to(&goal), 0.0);

        // Missing key → unmet + 1.0 distance
        let empty = WorldState::new();
        // 1 unmet (2.0) + 1.0 missing = 3.0
        assert_eq!(empty.distance_to(&goal), 3.0);
    }

    #[test]
    fn test_remove_contains_key_len_is_empty() {
        let mut ws = WorldState::new();
        assert!(ws.is_empty());
        assert_eq!(ws.len(), 0);
        assert!(!ws.contains_key("x"));

        ws.set("x", StateValue::Int(42));
        assert!(!ws.is_empty());
        assert_eq!(ws.len(), 1);
        assert!(ws.contains_key("x"));

        let removed = ws.remove("x");
        assert_eq!(removed, Some(StateValue::Int(42)));
        assert!(ws.is_empty());
        assert_eq!(ws.len(), 0);
        assert!(!ws.contains_key("x"));
        assert!(ws.get("x").is_none());
    }

    #[test]
    fn test_world_state_satisfies_unwrap_or_false() {
        // Missing key should NOT satisfy (unwrap_or(false))
        let ws = WorldState::new();
        let mut conditions = BTreeMap::new();
        conditions.insert("missing".to_string(), StateValue::Bool(true));
        assert!(!ws.satisfies(&conditions));
    }

    // ── Round 2 mutation-killing tests ──

    #[test]
    fn test_ordered_float_hash_distinguishes_values() {
        // Kills: <impl Hash for OrderedFloat>::hash → ()
        // If hash does nothing, distinct floats hash the same.
        let f1 = StateValue::Float(OrderedFloat(1.0));
        let f2 = StateValue::Float(OrderedFloat(2.0));
        assert_ne!(hash_value(&f1), hash_value(&f2));

        let f3 = StateValue::Float(OrderedFloat(0.0));
        let f4 = StateValue::Float(OrderedFloat(-1.0));
        assert_ne!(hash_value(&f3), hash_value(&f4));
    }

    #[test]
    fn test_satisfies_float_strict_less_than_tolerance() {
        // Kills: < 1e-6 → <= 1e-6 in Float-Float satisfies
        // At exactly 1e-6 distance, < should return false, <= would return true
        let a = StateValue::Float(OrderedFloat(0.0));
        let b = StateValue::Float(OrderedFloat(1e-6));
        // (0.0 - 1e-6).abs() = 1e-6; 1e-6 < 1e-6 is false
        assert!(
            !a.satisfies(&b),
            "Exactly 1e-6 apart should NOT satisfy with strict <"
        );
    }

    #[test]
    fn test_satisfies_int_float_strict_less_than() {
        // Kills: < 1e-6 → <= 1e-6 in Int-Float satisfies (line 73)
        let i = StateValue::Int(0);
        let f = StateValue::Float(OrderedFloat(1e-6));
        // (0 as f32 - 1e-6).abs() = 1e-6; 1e-6 < 1e-6 is false
        assert!(
            !i.satisfies(&f),
            "Int(0) vs Float(1e-6) at boundary should NOT satisfy"
        );
    }

    #[test]
    fn test_satisfies_float_int_strict_less_than() {
        // Kills: < 1e-6 → <= 1e-6 in Float-Int satisfies (line 74)
        let f = StateValue::Float(OrderedFloat(1e-6));
        let i = StateValue::Int(0);
        // (1e-6 - 0.0).abs() = 1e-6; 1e-6 < 1e-6 is false
        assert!(
            !f.satisfies(&i),
            "Float(1e-6) vs Int(0) at boundary should NOT satisfy"
        );
    }
}
