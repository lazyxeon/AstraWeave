//! Enemy spawner system for Veilweaver.
//!
//! Manages enemy spawn points, wave spawning, and difficulty scaling based on
//! anchor states. Enemies spawn near broken anchors with increasing difficulty.

use glam::Vec3;
use std::collections::HashMap;

use crate::sim_rng::{SimRng, SIM_SYSTEM_SPAWNER};
use crate::{Anchor, AnchorVfxState};

/// Enemy spawner managing waves and spawn points.
#[derive(Debug)]
pub struct EnemySpawner {
    /// Spawn point definitions (ID -> SpawnPoint)
    spawn_points: HashMap<usize, SpawnPoint>,
    /// Next spawn point ID
    next_spawn_id: usize,
    /// Current wave number (starts at 1)
    current_wave: u32,
    /// Time until next wave spawn (seconds)
    wave_timer: f32,
    /// Time between waves (seconds)
    wave_interval: f32,
    /// Base enemies per wave
    base_enemies_per_wave: u32,
    /// Difficulty scaling factor
    difficulty_multiplier: f32,
    /// Active enemy count (for spawn limiting)
    active_enemy_count: usize,
    /// Max concurrent enemies
    max_concurrent_enemies: usize,
    /// World seed mixed into each spawn draw.
    world_seed: u64,
    /// Spawns drawn so far. The next enemy uses this as the `SimRng` tick.
    draw_index: u64,
}

/// Spawn point configuration.
#[derive(Debug, Clone)]
pub struct SpawnPoint {
    /// Unique ID
    pub id: usize,
    /// World position
    pub position: Vec3,
    /// Spawn radius (enemies spawn within this radius)
    pub radius: f32,
    /// Associated anchor ID (enemies patrol near this anchor)
    pub anchor_id: Option<usize>,
    /// Is this spawn point active?
    pub active: bool,
    /// Spawn cooldown timer (seconds)
    pub cooldown: f32,
    /// Minimum time between spawns at this point (seconds)
    pub min_cooldown: f32,
}

/// Spawn request returned by spawner update.
#[derive(Debug, Clone)]
pub struct SpawnRequest {
    /// Position to spawn enemy
    pub position: Vec3,
    /// Patrol radius for enemy
    pub patrol_radius: f32,
    /// Associated anchor ID (if any)
    pub anchor_id: Option<usize>,
    /// Spawn point ID that created this request
    pub spawn_point_id: usize,
    /// Wave number this spawn belongs to
    pub wave: u32,
    /// Enemy archetype to spawn (from enemy_types.rs)
    pub archetype: crate::enemy_types::EnemyArchetype,
}

impl EnemySpawner {
    /// Creates a new enemy spawner with default settings.
    ///
    /// # Example
    /// ```
    /// use astraweave_weaving::spawner::EnemySpawner;
    ///
    /// let spawner = EnemySpawner::new();
    /// assert_eq!(spawner.current_wave(), 1);
    /// ```
    pub fn new() -> Self {
        Self {
            spawn_points: HashMap::new(),
            next_spawn_id: 0,
            current_wave: 1,
            wave_timer: 5.0, // First wave in 5 seconds
            wave_interval: 30.0,
            base_enemies_per_wave: 3,
            difficulty_multiplier: 1.0,
            active_enemy_count: 0,
            max_concurrent_enemies: 20,
            world_seed: 0,
            draw_index: 0,
        }
    }

    /// Sets the world seed used for spawn position and archetype.
    pub fn set_world_seed(&mut self, world_seed: u64) {
        self.world_seed = world_seed;
    }

    /// Creates a spawner with custom settings.
    pub fn with_settings(
        wave_interval: f32,
        base_enemies_per_wave: u32,
        max_concurrent_enemies: usize,
    ) -> Self {
        Self {
            wave_interval,
            base_enemies_per_wave,
            max_concurrent_enemies,
            wave_timer: wave_interval,
            ..Self::new()
        }
    }

    /// Adds a spawn point.
    ///
    /// # Example
    /// ```
    /// use astraweave_weaving::spawner::EnemySpawner;
    /// use glam::Vec3;
    ///
    /// let mut spawner = EnemySpawner::new();
    /// let spawn_id = spawner.add_spawn_point(Vec3::new(10.0, 0.0, 5.0), 5.0, Some(0));
    /// assert_eq!(spawner.spawn_point_count(), 1);
    /// ```
    pub fn add_spawn_point(
        &mut self,
        position: Vec3,
        radius: f32,
        anchor_id: Option<usize>,
    ) -> usize {
        let id = self.next_spawn_id;
        self.next_spawn_id += 1;

        let spawn_point = SpawnPoint {
            id,
            position,
            radius,
            anchor_id,
            active: true,
            cooldown: 0.0,
            min_cooldown: 5.0,
        };

        self.spawn_points.insert(id, spawn_point);
        id
    }

    /// Removes a spawn point.
    pub fn remove_spawn_point(&mut self, id: usize) -> bool {
        self.spawn_points.remove(&id).is_some()
    }

    /// Returns number of spawn points.
    pub fn spawn_point_count(&self) -> usize {
        self.spawn_points.len()
    }

    /// Returns current wave number.
    pub fn current_wave(&self) -> u32 {
        self.current_wave
    }

    /// Returns time until next wave.
    pub fn time_until_wave(&self) -> f32 {
        self.wave_timer
    }

    /// Sets active enemy count (call when enemies spawn/die).
    pub fn set_active_enemy_count(&mut self, count: usize) {
        self.active_enemy_count = count;
    }

    /// Updates spawner and returns spawn requests.
    ///
    /// # Arguments
    /// * `delta_time` - Time since last update (seconds)
    /// * `anchors` - List of (anchor_id, anchor) tuples for difficulty scaling
    ///
    /// # Returns
    /// Vector of spawn requests (empty if no spawns this frame)
    ///
    /// # Example
    /// ```
    /// use astraweave_weaving::spawner::EnemySpawner;
    /// use astraweave_weaving::anchor::Anchor;
    /// use glam::Vec3;
    ///
    /// let mut spawner = EnemySpawner::new();
    /// spawner.add_spawn_point(Vec3::ZERO, 5.0, Some(0));
    ///
    /// let mut anchor = Anchor::new(0.3, 50, None);
    /// let anchors = vec![(0, &anchor)];
    ///
    /// // First update (5 seconds elapsed)
    /// let requests = spawner.update(5.1, &anchors);
    /// assert!(requests.len() > 0); // Wave spawned
    /// ```
    pub fn update(&mut self, delta_time: f32, anchors: &[(usize, &Anchor)]) -> Vec<SpawnRequest> {
        // Update spawn point cooldowns
        for spawn_point in self.spawn_points.values_mut() {
            if spawn_point.cooldown > 0.0 {
                spawn_point.cooldown = (spawn_point.cooldown - delta_time).max(0.0);
            }
        }

        // Update wave timer
        self.wave_timer -= delta_time;

        // Check if wave should spawn
        if self.wave_timer <= 0.0 {
            self.wave_timer = self.wave_interval;
            return self.spawn_wave(anchors);
        }

        Vec::new()
    }

    /// Spawns a wave of enemies.
    fn spawn_wave(&mut self, anchors: &[(usize, &Anchor)]) -> Vec<SpawnRequest> {
        // Calculate difficulty multiplier based on broken anchors
        self.update_difficulty(anchors);

        // Calculate enemies to spawn
        let enemies_this_wave = self.calculate_wave_size();

        // Don't spawn if at max capacity
        if self.active_enemy_count >= self.max_concurrent_enemies {
            return Vec::new();
        }

        // Spawn enemies at available spawn points
        let mut requests = Vec::new();
        let available_spawns = self.get_active_spawn_points(anchors);

        if available_spawns.is_empty() {
            return Vec::new();
        }

        let enemies_to_spawn =
            enemies_this_wave.min((self.max_concurrent_enemies - self.active_enemy_count) as u32);

        for i in 0..enemies_to_spawn {
            // Round-robin spawn points
            let spawn_point_id = available_spawns[i as usize % available_spawns.len()];

            // Get spawn point data first (immutable borrow)
            let (spawn_position, spawn_radius, spawn_anchor_id, min_cooldown) = {
                if let Some(sp) = self.spawn_points.get(&spawn_point_id) {
                    (sp.position, sp.radius, sp.anchor_id, sp.min_cooldown)
                } else {
                    continue; // Skip if spawn point doesn't exist
                }
            };

            let mut rng = self.next_spawn_rng();
            let position = Self::generate_spawn_position(spawn_position, spawn_radius, &mut rng);
            let archetype = self.determine_archetype(&mut rng);

            requests.push(SpawnRequest {
                position,
                patrol_radius: spawn_radius,
                anchor_id: spawn_anchor_id,
                spawn_point_id,
                wave: self.current_wave,
                archetype,
            });

            // Set cooldown (mutable borrow)
            if let Some(spawn_point) = self.spawn_points.get_mut(&spawn_point_id) {
                spawn_point.cooldown = min_cooldown;
            }
        }

        // Increment wave counter
        self.current_wave += 1;

        requests
    }

    /// Updates difficulty multiplier based on anchor states.
    fn update_difficulty(&mut self, anchors: &[(usize, &Anchor)]) {
        if anchors.is_empty() {
            self.difficulty_multiplier = 1.0;
            return;
        }

        let broken_count = anchors
            .iter()
            .filter(|(_, anchor)| anchor.vfx_state() == AnchorVfxState::Broken)
            .count();

        let critical_count = anchors
            .iter()
            .filter(|(_, anchor)| anchor.vfx_state() == AnchorVfxState::Critical)
            .count();

        // Base 1.0, +0.5 per broken anchor, +0.25 per critical anchor
        self.difficulty_multiplier =
            1.0 + (broken_count as f32 * 0.5) + (critical_count as f32 * 0.25);
    }

    /// Calculates wave size based on current wave and difficulty.
    fn calculate_wave_size(&self) -> u32 {
        let base = self.base_enemies_per_wave as f32;
        let wave_scaling = (self.current_wave - 1) as f32 * 0.5; // +0.5 enemy per wave
        let size = (base + wave_scaling) * self.difficulty_multiplier;
        size.round().max(1.0) as u32
    }

    /// Returns IDs of active spawn points (prioritizing broken anchors).
    fn get_active_spawn_points(&self, anchors: &[(usize, &Anchor)]) -> Vec<usize> {
        let broken_anchor_ids: Vec<usize> = anchors
            .iter()
            .filter(|(_, anchor)| {
                anchor.vfx_state() == AnchorVfxState::Broken
                    || anchor.vfx_state() == AnchorVfxState::Critical
            })
            .map(|(id, _)| *id)
            .collect();

        // Get spawn points associated with broken anchors (priority)
        let mut active_spawns: Vec<usize> = self
            .spawn_points
            .values()
            .filter(|sp| sp.active && sp.cooldown <= 0.0)
            .filter(|sp| {
                if let Some(anchor_id) = sp.anchor_id {
                    broken_anchor_ids.contains(&anchor_id)
                } else {
                    false
                }
            })
            .map(|sp| sp.id)
            .collect();
        // HashMap walk order is per-process. Spawn ids are the stable order.
        active_spawns.sort_unstable();

        // If no broken-anchor spawns available, use any active spawn
        if active_spawns.is_empty() {
            active_spawns = self
                .spawn_points
                .values()
                .filter(|sp| sp.active && sp.cooldown <= 0.0)
                .map(|sp| sp.id)
                .collect();
            active_spawns.sort_unstable();
        }

        active_spawns
    }

    fn next_spawn_rng(&mut self) -> SimRng {
        let rng = SimRng::new(self.world_seed, self.draw_index, SIM_SYSTEM_SPAWNER);
        self.draw_index = self.draw_index.wrapping_add(1);
        rng
    }

    /// Spawn offset inside the disk. Seeded; no `sin`/`cos`.
    fn generate_spawn_position(center: Vec3, radius: f32, rng: &mut SimRng) -> Vec3 {
        let (x, z) = rng.disk_xz(radius);
        center + Vec3::new(x, 0.0, z)
    }

    /// Determines enemy archetype based on wave number and difficulty.
    ///
    /// Wave 1-2: Standard enemies only
    /// Wave 3-5: Standard + Riftstalkers (fast flankers)
    /// Wave 6-9: Standard + Riftstalkers + Sentinels (tanky AOE)
    /// Wave 10+: All types including VoidBoss (25% chance on wave 10, 15, 20...)
    fn determine_archetype(&self, rng: &mut SimRng) -> crate::enemy_types::EnemyArchetype {
        // Boss waves (every 5 waves starting at wave 10)
        if self.current_wave >= 10 && self.current_wave % 5 == 0 {
            let boss_chance = rng.unit_f32();
            if boss_chance < 0.25 {
                return crate::enemy_types::EnemyArchetype::VoidBoss;
            }
        }

        // Wave progression
        match self.current_wave {
            1..=2 => crate::enemy_types::EnemyArchetype::Standard,
            3..=5 => {
                // 70% Standard, 30% Riftstalker
                if rng.unit_f32() < 0.3 {
                    crate::enemy_types::EnemyArchetype::Riftstalker
                } else {
                    crate::enemy_types::EnemyArchetype::Standard
                }
            }
            6..=9 => {
                // 50% Standard, 30% Riftstalker, 20% Sentinel
                let roll = rng.unit_f32();
                if roll < 0.2 {
                    crate::enemy_types::EnemyArchetype::Sentinel
                } else if roll < 0.5 {
                    crate::enemy_types::EnemyArchetype::Riftstalker
                } else {
                    crate::enemy_types::EnemyArchetype::Standard
                }
            }
            _ => {
                // Wave 10+: 40% Standard, 35% Riftstalker, 25% Sentinel
                let roll = rng.unit_f32();
                if roll < 0.25 {
                    crate::enemy_types::EnemyArchetype::Sentinel
                } else if roll < 0.60 {
                    crate::enemy_types::EnemyArchetype::Riftstalker
                } else {
                    crate::enemy_types::EnemyArchetype::Standard
                }
            }
        }
    }

    /// Manually triggers a wave spawn (for testing).
    pub fn force_spawn_wave(&mut self, anchors: &[(usize, &Anchor)]) -> Vec<SpawnRequest> {
        self.spawn_wave(anchors)
    }

    /// Resets spawner to wave 1.
    pub fn reset(&mut self) {
        self.current_wave = 1;
        self.wave_timer = self.wave_interval;
        self.difficulty_multiplier = 1.0;
        self.active_enemy_count = 0;
        self.draw_index = 0;

        for spawn_point in self.spawn_points.values_mut() {
            spawn_point.cooldown = 0.0;
        }
    }

    /// Returns current difficulty multiplier.
    pub fn difficulty_multiplier(&self) -> f32 {
        self.difficulty_multiplier
    }

    /// Alias for `difficulty_multiplier()` (shorter name for integration tests).
    pub fn difficulty(&self) -> f32 {
        self.difficulty_multiplier
    }

    /// Sets whether a spawn point is active.
    pub fn set_spawn_point_active(&mut self, id: usize, active: bool) -> bool {
        if let Some(spawn_point) = self.spawn_points.get_mut(&id) {
            spawn_point.active = active;
            true
        } else {
            false
        }
    }
}

impl Default for EnemySpawner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spawner_creation() {
        let spawner = EnemySpawner::new();
        assert_eq!(spawner.current_wave(), 1);
        assert_eq!(spawner.spawn_point_count(), 0);
        assert_eq!(spawner.time_until_wave(), 5.0);
    }

    #[test]
    fn test_spawner_with_settings() {
        let spawner = EnemySpawner::with_settings(60.0, 5, 50);
        assert_eq!(spawner.wave_interval, 60.0);
        assert_eq!(spawner.base_enemies_per_wave, 5);
        assert_eq!(spawner.max_concurrent_enemies, 50);
    }

    #[test]
    fn test_add_spawn_point() {
        let mut spawner = EnemySpawner::new();

        let id1 = spawner.add_spawn_point(Vec3::new(10.0, 0.0, 5.0), 5.0, Some(0));
        assert_eq!(spawner.spawn_point_count(), 1);

        let id2 = spawner.add_spawn_point(Vec3::new(20.0, 0.0, 10.0), 7.0, Some(1));
        assert_eq!(spawner.spawn_point_count(), 2);

        assert_ne!(id1, id2);
    }

    #[test]
    fn test_remove_spawn_point() {
        let mut spawner = EnemySpawner::new();
        let id = spawner.add_spawn_point(Vec3::ZERO, 5.0, None);

        assert_eq!(spawner.spawn_point_count(), 1);
        assert!(spawner.remove_spawn_point(id));
        assert_eq!(spawner.spawn_point_count(), 0);

        // Removing again fails
        assert!(!spawner.remove_spawn_point(id));
    }

    #[test]
    fn test_wave_timer_countdown() {
        let mut spawner = EnemySpawner::new();
        spawner.add_spawn_point(Vec3::ZERO, 5.0, None);

        let anchor = Anchor::new(1.0, 50, None);
        let anchors = vec![(0, &anchor)];

        // Timer starts at 5.0
        assert_eq!(spawner.time_until_wave(), 5.0);

        // Update 2 seconds
        spawner.update(2.0, &anchors);
        assert_eq!(spawner.time_until_wave(), 3.0);

        // Update 3 more seconds (wave spawns)
        let requests = spawner.update(3.1, &anchors);
        assert!(!requests.is_empty());
        assert_eq!(spawner.current_wave(), 2); // Wave incremented
    }

    #[test]
    fn test_wave_spawning() {
        let mut spawner = EnemySpawner::new();
        spawner.add_spawn_point(Vec3::ZERO, 5.0, Some(0));

        let anchor = Anchor::new(0.5, 50, None);
        let anchors = vec![(0, &anchor)];

        // Fast-forward to wave spawn
        let requests = spawner.update(5.1, &anchors);

        assert_eq!(requests.len(), 3); // base_enemies_per_wave = 3
        assert_eq!(spawner.current_wave(), 2);

        for request in &requests {
            assert_eq!(request.wave, 1);
            assert_eq!(request.spawn_point_id, 0);
            assert_eq!(request.anchor_id, Some(0));
        }
    }

    #[test]
    fn test_difficulty_scaling_broken_anchors() {
        let mut spawner = EnemySpawner::new();
        spawner.add_spawn_point(Vec3::ZERO, 5.0, Some(0));

        // Create anchors with low stability (broken state < 0.1)
        let anchor1 = Anchor::new(0.05, 50, None); // Broken (< 0.1)
        let anchor2 = Anchor::new(0.03, 50, None); // Broken (< 0.1)

        let anchors = vec![(0, &anchor1), (1, &anchor2)];

        // Trigger wave spawn to update difficulty
        let _requests = spawner.update(5.1, &anchors);

        // Base 1.0 + 2 broken * 0.5 = 2.0
        assert_eq!(spawner.difficulty_multiplier(), 2.0);
    }

    #[test]
    fn test_difficulty_scaling_critical_anchors() {
        let mut spawner = EnemySpawner::new();
        spawner.add_spawn_point(Vec3::ZERO, 5.0, Some(0));

        // Create anchor with critical stability (0.1 <= stability < 0.4)
        let anchor = Anchor::new(0.25, 50, None); // Critical

        let anchors = vec![(0, &anchor)];

        // Trigger wave spawn to update difficulty
        let _requests = spawner.update(5.1, &anchors);

        // Base 1.0 + 1 critical * 0.25 = 1.25
        assert_eq!(spawner.difficulty_multiplier(), 1.25);
    }

    #[test]
    fn test_wave_size_scaling() {
        let mut spawner = EnemySpawner::new();
        spawner.add_spawn_point(Vec3::ZERO, 5.0, None);

        let anchor = Anchor::new(1.0, 50, None);
        let anchors = vec![(0, &anchor)];

        // Wave 1: 3 enemies
        let requests = spawner.update(5.1, &anchors);
        assert_eq!(requests.len(), 3);

        // Wave 2: 3.5 rounded to 4 enemies
        let requests = spawner.update(30.1, &anchors);
        assert_eq!(requests.len(), 4);

        // Wave 3: 4 enemies
        let requests = spawner.update(30.1, &anchors);
        assert_eq!(requests.len(), 4);
    }

    #[test]
    fn test_max_concurrent_enemies() {
        let mut spawner = EnemySpawner::with_settings(5.0, 10, 5);
        spawner.add_spawn_point(Vec3::ZERO, 5.0, None);

        spawner.set_active_enemy_count(3); // 3 alive

        let anchor = Anchor::new(1.0, 50, None);
        let anchors = vec![(0, &anchor)];

        // Wave wants to spawn 10, but only 2 slots available
        let requests = spawner.update(5.1, &anchors);
        assert_eq!(requests.len(), 2);
    }

    #[test]
    fn test_no_spawn_at_max_capacity() {
        let mut spawner = EnemySpawner::with_settings(5.0, 5, 10);
        spawner.add_spawn_point(Vec3::ZERO, 5.0, None);

        spawner.set_active_enemy_count(10); // At max

        let anchor = Anchor::new(1.0, 50, None);
        let anchors = vec![(0, &anchor)];

        let requests = spawner.update(5.1, &anchors);
        assert_eq!(requests.len(), 0);
    }

    #[test]
    fn test_spawn_point_cooldown() {
        let mut spawner = EnemySpawner::new();
        let _id = spawner.add_spawn_point(Vec3::ZERO, 5.0, None);

        let anchor = Anchor::new(1.0, 50, None);
        let anchors = vec![(0, &anchor)];

        // First spawn works (wave spawns at 5.1s)
        let requests = spawner.update(5.1, &anchors);
        assert_eq!(requests.len(), 3);
        assert_eq!(spawner.current_wave(), 2); // Wave incremented

        // Immediate second spawn blocked by cooldown (force spawn)
        let requests = spawner.force_spawn_wave(&anchors);
        assert_eq!(requests.len(), 0); // Cooldown active

        // Cooldown expires after 5 seconds
        spawner.update(5.1, &anchors);
        let requests = spawner.force_spawn_wave(&anchors);
        assert_eq!(requests.len(), 4); // Wave 3: base(3) + wave_scaling(0.5*2) = 4
    }

    #[test]
    fn test_prioritize_broken_anchor_spawns() {
        let mut spawner = EnemySpawner::new();
        spawner.add_spawn_point(Vec3::new(0.0, 0.0, 0.0), 5.0, Some(0)); // Anchor 0 (perfect)
        spawner.add_spawn_point(Vec3::new(10.0, 0.0, 0.0), 5.0, Some(1)); // Anchor 1 (broken)

        let anchor0 = Anchor::new(1.0, 50, None); // Perfect
        let anchor1 = Anchor::new(0.05, 50, None); // Broken (< 0.1)

        let anchors = vec![(0, &anchor0), (1, &anchor1)];

        let requests = spawner.update(5.1, &anchors);

        // All spawns should be at broken anchor's spawn point
        assert!(!requests.is_empty(), "Expected spawn requests");
        for request in &requests {
            assert_eq!(
                request.anchor_id,
                Some(1),
                "Expected spawn at broken anchor (1), got anchor {:?}",
                request.anchor_id
            );
        }
    }

    #[test]
    fn test_round_robin_spawning() {
        let mut spawner = EnemySpawner::with_settings(5.0, 6, 20);
        spawner.add_spawn_point(Vec3::new(0.0, 0.0, 0.0), 5.0, Some(0));
        spawner.add_spawn_point(Vec3::new(10.0, 0.0, 0.0), 5.0, Some(1));

        let anchor0 = Anchor::new(1.0, 50, None);
        let anchor1 = Anchor::new(1.0, 50, None);
        let anchors = vec![(0, &anchor0), (1, &anchor1)];

        let requests = spawner.update(5.1, &anchors);

        // 6 enemies, 2 spawn points -> 3 per spawn point
        assert_eq!(requests.len(), 6);

        let spawn0_count = requests.iter().filter(|r| r.spawn_point_id == 0).count();
        let spawn1_count = requests.iter().filter(|r| r.spawn_point_id == 1).count();

        assert_eq!(spawn0_count, 3);
        assert_eq!(spawn1_count, 3);
    }

    #[test]
    fn test_reset_spawner() {
        let mut spawner = EnemySpawner::new();
        spawner.add_spawn_point(Vec3::ZERO, 5.0, None);

        let anchor = Anchor::new(1.0, 50, None);
        let anchors = vec![(0, &anchor)];

        // Advance to wave 3
        spawner.update(5.1, &anchors);
        spawner.update(30.1, &anchors);
        assert_eq!(spawner.current_wave(), 3);

        spawner.reset();
        assert_eq!(spawner.current_wave(), 1);
        assert_eq!(spawner.time_until_wave(), 30.0);
    }

    #[test]
    fn test_set_spawn_point_active() {
        let mut spawner = EnemySpawner::new();
        let id = spawner.add_spawn_point(Vec3::ZERO, 5.0, None);

        let anchor = Anchor::new(1.0, 50, None);
        let anchors = vec![(0, &anchor)];

        // Spawn works when active
        let requests = spawner.update(5.1, &anchors);
        assert_eq!(requests.len(), 3);

        // Disable spawn point
        assert!(spawner.set_spawn_point_active(id, false));

        // No spawns when disabled
        let requests = spawner.update(30.1, &anchors);
        assert_eq!(requests.len(), 0);

        // Re-enable
        assert!(spawner.set_spawn_point_active(id, true));
        let requests = spawner.update(30.1, &anchors);
        assert_eq!(requests.len(), 4); // Wave 4
    }

    fn first_wave(seed: u64) -> Vec<SpawnRequest> {
        let mut spawner = EnemySpawner::new();
        spawner.set_world_seed(seed);
        spawner.add_spawn_point(Vec3::new(3.0, 0.0, 4.0), 5.0, None);
        let anchor = Anchor::new(1.0, 50, None);
        spawner.force_spawn_wave(&[(0, &anchor)])
    }

    #[test]
    fn spawn_positions_are_bit_identical_for_the_same_seed() {
        let a = first_wave(0xA11C);
        let b = first_wave(0xA11C);
        assert_eq!(a.len(), b.len());
        assert!(!a.is_empty());
        for (left, right) in a.iter().zip(b.iter()) {
            assert_eq!(left.position.x.to_bits(), right.position.x.to_bits());
            assert_eq!(left.position.y.to_bits(), right.position.y.to_bits());
            assert_eq!(left.position.z.to_bits(), right.position.z.to_bits());
            assert_eq!(left.archetype, right.archetype);
            let d = left.position.distance(Vec3::new(3.0, 0.0, 4.0));
            assert!(d <= 5.0);
        }
        let p = a[0].position;
        assert_eq!(
            (p.x.to_bits(), p.y.to_bits(), p.z.to_bits()),
            (3_205_979_728, 0, 1_082_688_523)
        );
    }
}
