//! Deterministic RNG with explicit seeds per layer

use rand::distr::uniform::{SampleRange, SampleUniform};
use rand::rngs::StdRng;
use rand::{Rng, RngCore, SeedableRng};

/// Deterministic random number generator with layer tracking
pub struct SeedRng {
    inner: StdRng,
    layer: String,
}

impl SeedRng {
    /// Create a new seeded RNG for a specific layer
    pub fn new(seed: u64, layer: &str) -> Self {
        Self {
            inner: StdRng::seed_from_u64(seed),
            layer: layer.to_string(),
        }
    }

    /// Fork this RNG into a child with a new sublayer
    /// The child has a deterministic seed derived from this RNG
    pub fn fork(&mut self, sublayer: &str) -> Self {
        let subseed = self.inner.random::<u64>();
        Self::new(subseed, &format!("{}::{}", self.layer, sublayer))
    }

    /// Get the layer name (useful for debugging)
    pub fn layer(&self) -> &str {
        &self.layer
    }

    /// Generate a random value in the given range
    pub fn gen_range<T, R>(&mut self, range: R) -> T
    where
        T: SampleUniform,
        R: SampleRange<T>,
    {
        self.inner.random_range(range)
    }

    /// Generate a random boolean
    pub fn gen_bool(&mut self) -> bool {
        self.inner.random()
    }

    /// Generate a random boolean with given probability
    pub fn gen_bool_with_prob(&mut self, probability: f64) -> bool {
        self.inner.random_bool(probability)
    }

    /// Choose a random element from a slice
    pub fn choose<'a, T>(&mut self, slice: &'a [T]) -> Option<&'a T> {
        if slice.is_empty() {
            None
        } else {
            Some(&slice[self.gen_range(0..slice.len())])
        }
    }

    /// Choose a random element from a slice and return a mutable reference
    pub fn choose_mut<'a, T>(&mut self, slice: &'a mut [T]) -> Option<&'a mut T> {
        if slice.is_empty() {
            None
        } else {
            let idx = self.gen_range(0..slice.len());
            Some(&mut slice[idx])
        }
    }

    /// Shuffle a slice in place
    pub fn shuffle<T>(&mut self, slice: &mut [T]) {
        use rand::seq::SliceRandom;
        slice.shuffle(&mut self.inner);
    }

    /// Generate a random f32 in [0, 1)
    pub fn gen_f32(&mut self) -> f32 {
        self.inner.random()
    }

    /// Generate a random f64 in [0, 1)
    pub fn gen_f64(&mut self) -> f64 {
        self.inner.random()
    }
}

impl RngCore for SeedRng {
    fn next_u32(&mut self) -> u32 {
        self.inner.next_u32()
    }

    fn next_u64(&mut self) -> u64 {
        self.inner.next_u64()
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        self.inner.fill_bytes(dest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_same_seed_same_sequence() {
        let mut rng1 = SeedRng::new(42, "test");
        let mut rng2 = SeedRng::new(42, "test");

        for _ in 0..100 {
            assert_eq!(rng1.gen_range(0..100), rng2.gen_range(0..100));
        }
    }

    #[test]
    fn test_different_seed_different_sequence() {
        let mut rng1 = SeedRng::new(42, "test");
        let mut rng2 = SeedRng::new(43, "test");

        let vals1: Vec<_> = (0..100).map(|_| rng1.gen_range(0..100)).collect();
        let vals2: Vec<_> = (0..100).map(|_| rng2.gen_range(0..100)).collect();

        // Extremely unlikely to be all equal
        assert_ne!(vals1, vals2);
    }

    #[test]
    fn test_fork_deterministic() {
        let mut rng1 = SeedRng::new(42, "parent");
        let mut rng2 = SeedRng::new(42, "parent");

        let mut child1 = rng1.fork("child");
        let mut child2 = rng2.fork("child");

        for _ in 0..100 {
            assert_eq!(child1.gen_range(0..100), child2.gen_range(0..100));
        }
    }

    #[test]
    fn test_fork_independent() {
        let mut parent = SeedRng::new(42, "parent");
        let mut child = parent.fork("child");

        // Parent and child should have different sequences
        let parent_vals: Vec<_> = (0..100).map(|_| parent.gen_range(0..100)).collect();
        let child_vals: Vec<_> = (0..100).map(|_| child.gen_range(0..100)).collect();

        assert_ne!(parent_vals, child_vals);
    }

    #[test]
    fn test_choose() {
        let mut rng = SeedRng::new(42, "test");
        let items = vec![1, 2, 3, 4, 5];

        let chosen = rng.choose(&items);
        assert!(chosen.is_some());
        assert!(items.contains(chosen.unwrap()));
    }

    #[test]
    fn test_choose_empty() {
        let mut rng = SeedRng::new(42, "test");
        let items: Vec<i32> = vec![];

        let chosen = rng.choose(&items);
        assert!(chosen.is_none());
    }

    #[test]
    fn test_shuffle_deterministic() {
        let mut rng1 = SeedRng::new(42, "test");
        let mut rng2 = SeedRng::new(42, "test");

        let mut items1 = vec![1, 2, 3, 4, 5];
        let mut items2 = vec![1, 2, 3, 4, 5];

        rng1.shuffle(&mut items1);
        rng2.shuffle(&mut items2);

        assert_eq!(items1, items2);
    }

    #[test]
    fn test_layer_tracking() {
        let parent = SeedRng::new(42, "parent");
        assert_eq!(parent.layer(), "parent");

        let mut parent_mut = parent;
        let child = parent_mut.fork("child");
        assert_eq!(child.layer(), "parent::child");
    }

    // ── Additional SeedRng coverage tests ──

    #[test]
    fn test_gen_bool_deterministic() {
        let mut rng1 = SeedRng::new(42, "test");
        let mut rng2 = SeedRng::new(42, "test");
        for _ in 0..50 {
            assert_eq!(rng1.gen_bool(), rng2.gen_bool());
        }
    }

    #[test]
    fn test_gen_bool_with_prob_always_false() {
        let mut rng = SeedRng::new(42, "test");
        for _ in 0..100 {
            assert!(!rng.gen_bool_with_prob(0.0));
        }
    }

    #[test]
    fn test_gen_bool_with_prob_always_true() {
        let mut rng = SeedRng::new(42, "test");
        for _ in 0..100 {
            assert!(rng.gen_bool_with_prob(1.0));
        }
    }

    #[test]
    fn test_choose_mut_nonempty() {
        let mut rng = SeedRng::new(42, "test");
        let mut items = vec![10, 20, 30];
        let chosen = rng.choose_mut(&mut items);
        assert!(chosen.is_some());
        let val = *chosen.unwrap();
        assert!(val == 10 || val == 20 || val == 30);
    }

    #[test]
    fn test_choose_mut_empty() {
        let mut rng = SeedRng::new(42, "test");
        let mut items: Vec<i32> = vec![];
        assert!(rng.choose_mut(&mut items).is_none());
    }

    #[test]
    fn test_choose_mut_mutates() {
        let mut rng = SeedRng::new(42, "test");
        let mut items = vec![1, 2, 3];
        if let Some(val) = rng.choose_mut(&mut items) {
            *val = 99;
        }
        assert!(items.contains(&99));
    }

    #[test]
    fn test_gen_f32_range() {
        let mut rng = SeedRng::new(42, "test");
        for _ in 0..100 {
            let v = rng.gen_f32();
            assert!((0.0..1.0).contains(&v), "gen_f32 out of [0,1): {}", v);
        }
    }

    #[test]
    fn test_gen_f64_range() {
        let mut rng = SeedRng::new(42, "test");
        for _ in 0..100 {
            let v = rng.gen_f64();
            assert!((0.0..1.0).contains(&v), "gen_f64 out of [0,1): {}", v);
        }
    }

    #[test]
    fn test_shuffle_empty_slice() {
        let mut rng = SeedRng::new(42, "test");
        let mut items: Vec<i32> = vec![];
        rng.shuffle(&mut items); // should not panic
        assert!(items.is_empty());
    }

    #[test]
    fn test_shuffle_single_element() {
        let mut rng = SeedRng::new(42, "test");
        let mut items = vec![42];
        rng.shuffle(&mut items);
        assert_eq!(items, vec![42]);
    }

    #[test]
    fn test_fork_layer_nesting() {
        let mut parent = SeedRng::new(1, "root");
        let mut child = parent.fork("level1");
        let grandchild = child.fork("level2");
        assert_eq!(grandchild.layer(), "root::level1::level2");
    }
}
