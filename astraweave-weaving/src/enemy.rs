//! Enemy AI component for Veilweaver.
//!
//! Enemies disrupt Reality Anchors and attack the player. They use a state machine
//! to determine behavior: patrol near broken anchors, attack anchors to accelerate
//! decay, or engage the player if within aggro range.

use glam::Vec3;

use crate::sim_rng::{SimRng, SIM_SYSTEM_ENEMY_PATROL};

/// AI-driven enemy that disrupts anchors and attacks the player.
#[derive(Debug, Clone)]
pub struct Enemy {
    /// Current health (0-100 HP)
    pub health: f32,
    /// Maximum health (100 HP)
    pub max_health: f32,
    /// Movement speed (units/sec)
    pub speed: f32,
    /// Current AI state
    pub state: EnemyState,
    /// Which anchor to attack (if any)
    pub target_anchor_id: Option<usize>,
    /// Base damage per attack (HP)
    pub attack_damage: f32,
    /// Time between attacks (seconds)
    pub attack_cooldown: f32,
    /// Current attack cooldown timer (seconds)
    pub attack_timer: f32,
    /// Center point for patrol behavior
    pub patrol_center: Vec3,
    /// Patrol radius (units)
    pub patrol_radius: f32,
    /// Distance at which enemy engages player (units)
    pub aggro_range: f32,
    /// Health threshold to flee (HP)
    pub flee_health: f32,
    /// Current patrol target position
    patrol_target: Vec3,
    /// Time until next patrol target update
    patrol_retarget_timer: f32,
    /// World seed mixed into each patrol retarget.
    world_seed: u64,
    /// How many patrol targets this enemy has drawn. The next draw is
    /// `SimRng::new(world_seed, draw_index, SIM_SYSTEM_ENEMY_PATROL)`.
    draw_index: u64,
}

/// AI state machine for enemy behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum EnemyState {
    /// Walk randomly near patrol center
    Patrol,
    /// Move toward and attack target anchor
    AttackAnchor,
    /// Move toward and attack player
    EngagePlayer,
    /// Run away from player (low health)
    Flee,
    /// Dead (despawn after delay)
    Dead,
}

/// Behavior command returned by enemy AI update.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum EnemyBehavior {
    /// Move toward target position
    MoveTo(Vec3),
    /// Attack target (anchor or player)
    Attack(AttackTarget),
    /// Stay at current position (no action)
    Idle,
}

/// Target for enemy attack.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum AttackTarget {
    /// Attack specific anchor by ID
    Anchor(usize),
    /// Attack player
    Player,
}

impl Enemy {
    /// Creates a new enemy at the given position.
    ///
    /// # Arguments
    /// * `position` - Starting position (becomes patrol center)
    /// * `patrol_radius` - How far from center enemy will patrol
    ///
    /// # Example
    /// ```
    /// use astraweave_weaving::enemy::Enemy;
    /// use glam::Vec3;
    ///
    /// let enemy = Enemy::new(Vec3::new(10.0, 0.0, 5.0), 5.0);
    /// assert_eq!(enemy.health, 100.0);
    /// ```
    pub fn new(position: Vec3, patrol_radius: f32) -> Self {
        Self {
            health: 100.0,
            max_health: 100.0,
            speed: 3.0,
            state: EnemyState::Patrol,
            target_anchor_id: None,
            attack_damage: 10.0,
            attack_cooldown: 1.0,
            attack_timer: 0.0,
            patrol_center: position,
            patrol_radius,
            aggro_range: 10.0,
            flee_health: 20.0,
            patrol_target: position,
            patrol_retarget_timer: 0.0,
            world_seed: 0,
            draw_index: 0,
        }
    }

    /// Sets the world seed used for patrol targets. Draw index is left as-is
    /// so a seed change does not replay earlier targets.
    pub fn set_world_seed(&mut self, world_seed: u64) {
        self.world_seed = world_seed;
    }

    /// Creates a new enemy with custom stats.
    ///
    /// # Example
    /// ```
    /// use astraweave_weaving::enemy::Enemy;
    /// use glam::Vec3;
    ///
    /// let enemy = Enemy::with_stats(
    ///     Vec3::new(0.0, 0.0, 0.0),
    ///     5.0,
    ///     150.0, // More health
    ///     5.0,   // Faster
    ///     15.0,  // More damage
    /// );
    /// assert_eq!(enemy.health, 150.0);
    /// assert_eq!(enemy.speed, 5.0);
    /// ```
    pub fn with_stats(
        position: Vec3,
        patrol_radius: f32,
        max_health: f32,
        speed: f32,
        attack_damage: f32,
    ) -> Self {
        Self {
            health: max_health,
            max_health,
            speed,
            attack_damage,
            ..Self::new(position, patrol_radius)
        }
    }

    /// Updates enemy AI and returns behavior command.
    ///
    /// # Arguments
    /// * `delta_time` - Time since last update (seconds)
    /// * `position` - Current enemy position
    /// * `player_pos` - Current player position
    /// * `broken_anchor_positions` - List of (anchor_id, position) for broken anchors
    ///
    /// # Returns
    /// Behavior command for this frame (MoveTo, Attack, or Idle)
    pub fn update(
        &mut self,
        delta_time: f32,
        position: Vec3,
        player_pos: Vec3,
        broken_anchor_positions: &[(usize, Vec3)],
    ) -> EnemyBehavior {
        // Update attack cooldown
        if self.attack_timer > 0.0 {
            self.attack_timer = (self.attack_timer - delta_time).max(0.0);
        }

        // Update patrol retarget timer
        if self.patrol_retarget_timer > 0.0 {
            self.patrol_retarget_timer = (self.patrol_retarget_timer - delta_time).max(0.0);
        }

        // Determine new state based on current situation
        self.update_state(position, player_pos, broken_anchor_positions);

        // Execute behavior for current state
        match self.state {
            EnemyState::Patrol => self.patrol_behavior(position, delta_time),
            EnemyState::AttackAnchor => {
                self.attack_anchor_behavior(position, broken_anchor_positions)
            }
            EnemyState::EngagePlayer => self.engage_player_behavior(position, player_pos),
            EnemyState::Flee => self.flee_behavior(position, player_pos),
            EnemyState::Dead => EnemyBehavior::Idle,
        }
    }

    /// Updates AI state based on current situation.
    fn update_state(
        &mut self,
        position: Vec3,
        player_pos: Vec3,
        broken_anchor_positions: &[(usize, Vec3)],
    ) {
        // Dead enemies stay dead
        if self.state == EnemyState::Dead {
            return;
        }

        let distance_to_player = position.distance(player_pos);

        // Priority 1: Flee if health low
        if self.health <= self.flee_health {
            self.state = EnemyState::Flee;
            return;
        }

        // Priority 2: Engage player if within aggro range
        if distance_to_player <= self.aggro_range {
            self.state = EnemyState::EngagePlayer;
            return;
        }

        // Priority 3: Attack nearest broken anchor
        if let Some((anchor_id, _)) = self.find_nearest_anchor(position, broken_anchor_positions) {
            self.target_anchor_id = Some(anchor_id);
            self.state = EnemyState::AttackAnchor;
            return;
        }

        // Default: Patrol
        self.target_anchor_id = None;
        self.state = EnemyState::Patrol;
    }

    /// Finds the nearest broken anchor to this position.
    fn find_nearest_anchor(
        &self,
        position: Vec3,
        broken_anchor_positions: &[(usize, Vec3)],
    ) -> Option<(usize, Vec3)> {
        broken_anchor_positions
            .iter()
            .min_by(|(_, a), (_, b)| {
                let dist_a = position.distance(*a);
                let dist_b = position.distance(*b);
                dist_a
                    .partial_cmp(&dist_b)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .copied()
    }

    /// Patrol behavior: walk randomly near patrol center.
    fn patrol_behavior(&mut self, position: Vec3, _delta_time: f32) -> EnemyBehavior {
        // Check if we need a new patrol target
        if self.patrol_retarget_timer <= 0.0 || position.distance(self.patrol_target) < 0.5 {
            self.patrol_target = self.generate_patrol_target();
            self.patrol_retarget_timer = 3.0; // Retarget every 3 seconds
        }

        EnemyBehavior::MoveTo(self.patrol_target)
    }

    fn next_patrol_rng(&mut self) -> SimRng {
        let rng = SimRng::new(self.world_seed, self.draw_index, SIM_SYSTEM_ENEMY_PATROL);
        self.draw_index = self.draw_index.wrapping_add(1);
        rng
    }

    /// Patrol target inside the patrol disk. Seeded; no `sin`/`cos`.
    fn generate_patrol_target(&mut self) -> Vec3 {
        let mut rng = self.next_patrol_rng();
        let (x, z) = rng.disk_xz(self.patrol_radius);
        self.patrol_center + Vec3::new(x, 0.0, z)
    }

    /// Attack anchor behavior: move toward target anchor and attack when in range.
    fn attack_anchor_behavior(
        &self,
        position: Vec3,
        broken_anchor_positions: &[(usize, Vec3)],
    ) -> EnemyBehavior {
        if let Some(target_id) = self.target_anchor_id {
            // Find target anchor position
            if let Some((_, anchor_pos)) = broken_anchor_positions
                .iter()
                .find(|(id, _)| *id == target_id)
            {
                let distance = position.distance(*anchor_pos);

                // Attack if in range and cooldown ready
                if distance <= 2.0 && self.can_attack() {
                    return EnemyBehavior::Attack(AttackTarget::Anchor(target_id));
                }

                // Otherwise, move toward anchor
                return EnemyBehavior::MoveTo(*anchor_pos);
            }
        }

        // No valid target, idle
        EnemyBehavior::Idle
    }

    /// Engage player behavior: move toward player and attack when in range.
    fn engage_player_behavior(&self, position: Vec3, player_pos: Vec3) -> EnemyBehavior {
        let distance = position.distance(player_pos);

        // Attack if in range and cooldown ready
        if distance <= 2.0 && self.can_attack() {
            return EnemyBehavior::Attack(AttackTarget::Player);
        }

        // Otherwise, move toward player
        EnemyBehavior::MoveTo(player_pos)
    }

    /// Flee behavior: run away from player.
    fn flee_behavior(&self, position: Vec3, player_pos: Vec3) -> EnemyBehavior {
        let direction = (position - player_pos).normalize_or_zero();
        let flee_target = position + direction * 10.0;
        EnemyBehavior::MoveTo(flee_target)
    }

    /// Checks if enemy can attack (cooldown ready).
    pub fn can_attack(&self) -> bool {
        self.attack_timer <= 0.0
    }

    /// Executes an attack, returning damage dealt.
    ///
    /// Resets attack cooldown timer.
    ///
    /// # Example
    /// ```
    /// use astraweave_weaving::enemy::Enemy;
    /// use glam::Vec3;
    ///
    /// let mut enemy = Enemy::new(Vec3::ZERO, 5.0);
    /// let damage = enemy.attack();
    /// assert_eq!(damage, 10.0); // Default damage
    /// assert!(!enemy.can_attack()); // Cooldown active
    /// ```
    pub fn attack(&mut self) -> f32 {
        self.attack_timer = self.attack_cooldown;
        self.attack_damage
    }

    /// Applies damage to this enemy.
    ///
    /// # Returns
    /// `true` if enemy was killed, `false` otherwise
    ///
    /// # Example
    /// ```
    /// use astraweave_weaving::enemy::Enemy;
    /// use glam::Vec3;
    ///
    /// let mut enemy = Enemy::new(Vec3::ZERO, 5.0);
    /// let killed = enemy.take_damage(50.0);
    /// assert!(!killed);
    /// assert_eq!(enemy.health, 50.0);
    ///
    /// let killed = enemy.take_damage(60.0);
    /// assert!(killed);
    /// assert_eq!(enemy.health, 0.0);
    /// ```
    pub fn take_damage(&mut self, amount: f32) -> bool {
        self.health = (self.health - amount).max(0.0);

        if self.health <= 0.0 {
            self.state = EnemyState::Dead;
            true
        } else {
            false
        }
    }

    /// Returns true if enemy is dead.
    pub fn is_dead(&self) -> bool {
        self.state == EnemyState::Dead
    }

    /// Returns health as percentage (0.0 to 1.0).
    pub fn health_percentage(&self) -> f32 {
        self.health / self.max_health
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enemy_creation() {
        let enemy = Enemy::new(Vec3::new(5.0, 0.0, 10.0), 5.0);
        assert_eq!(enemy.health, 100.0);
        assert_eq!(enemy.max_health, 100.0);
        assert_eq!(enemy.speed, 3.0);
        assert_eq!(enemy.state, EnemyState::Patrol);
        assert_eq!(enemy.patrol_center, Vec3::new(5.0, 0.0, 10.0));
        assert_eq!(enemy.patrol_radius, 5.0);
    }

    #[test]
    fn test_enemy_with_stats() {
        let enemy = Enemy::with_stats(Vec3::ZERO, 10.0, 150.0, 5.0, 20.0);
        assert_eq!(enemy.health, 150.0);
        assert_eq!(enemy.max_health, 150.0);
        assert_eq!(enemy.speed, 5.0);
        assert_eq!(enemy.attack_damage, 20.0);
    }

    #[test]
    fn test_take_damage() {
        let mut enemy = Enemy::new(Vec3::ZERO, 5.0);

        // Non-lethal damage
        let killed = enemy.take_damage(30.0);
        assert!(!killed);
        assert_eq!(enemy.health, 70.0);
        assert_eq!(enemy.state, EnemyState::Patrol);

        // Lethal damage
        let killed = enemy.take_damage(80.0);
        assert!(killed);
        assert_eq!(enemy.health, 0.0);
        assert_eq!(enemy.state, EnemyState::Dead);
        assert!(enemy.is_dead());
    }

    #[test]
    fn test_health_percentage() {
        let mut enemy = Enemy::new(Vec3::ZERO, 5.0);
        assert_eq!(enemy.health_percentage(), 1.0);

        enemy.take_damage(50.0);
        assert_eq!(enemy.health_percentage(), 0.5);

        enemy.take_damage(50.0);
        assert_eq!(enemy.health_percentage(), 0.0);
    }

    #[test]
    fn test_attack_cooldown() {
        let mut enemy = Enemy::new(Vec3::ZERO, 5.0);

        // Can attack initially
        assert!(enemy.can_attack());

        // Attack sets cooldown
        let damage = enemy.attack();
        assert_eq!(damage, 10.0);
        assert!(!enemy.can_attack());
        assert_eq!(enemy.attack_timer, 1.0);

        // Cooldown decreases over time
        enemy.update(0.5, Vec3::ZERO, Vec3::new(100.0, 0.0, 0.0), &[]);
        assert!(!enemy.can_attack());
        assert_eq!(enemy.attack_timer, 0.5);

        // Cooldown completes
        enemy.update(0.6, Vec3::ZERO, Vec3::new(100.0, 0.0, 0.0), &[]);
        assert!(enemy.can_attack());
        assert_eq!(enemy.attack_timer, 0.0);
    }

    #[test]
    fn test_state_transition_flee() {
        let mut enemy = Enemy::new(Vec3::ZERO, 5.0);
        enemy.take_damage(85.0); // Health = 15 HP (below flee threshold of 20)

        let behavior = enemy.update(0.1, Vec3::ZERO, Vec3::new(5.0, 0.0, 0.0), &[]);
        assert_eq!(enemy.state, EnemyState::Flee);

        if let EnemyBehavior::MoveTo(target) = behavior {
            // Should move away from player
            let direction = target.normalize();
            let player_direction = Vec3::new(5.0, 0.0, 0.0).normalize();
            let dot = direction.dot(player_direction);
            assert!(dot < 0.0, "Enemy should flee away from player");
        } else {
            panic!("Expected MoveTo behavior");
        }
    }

    #[test]
    fn test_state_transition_engage_player() {
        let mut enemy = Enemy::new(Vec3::ZERO, 5.0);
        let player_pos = Vec3::new(8.0, 0.0, 0.0); // Within aggro range (10.0)

        let behavior = enemy.update(0.1, Vec3::ZERO, player_pos, &[]);
        assert_eq!(enemy.state, EnemyState::EngagePlayer);

        if let EnemyBehavior::MoveTo(target) = behavior {
            assert_eq!(target, player_pos);
        } else {
            panic!("Expected MoveTo behavior");
        }
    }

    #[test]
    fn test_state_transition_attack_anchor() {
        let mut enemy = Enemy::new(Vec3::ZERO, 5.0);
        let player_pos = Vec3::new(50.0, 0.0, 0.0); // Far away
        let broken_anchors = vec![
            (0, Vec3::new(10.0, 0.0, 0.0)),
            (1, Vec3::new(20.0, 0.0, 0.0)),
        ];

        enemy.update(0.1, Vec3::ZERO, player_pos, &broken_anchors);
        assert_eq!(enemy.state, EnemyState::AttackAnchor);
        assert_eq!(enemy.target_anchor_id, Some(0)); // Nearest anchor
    }

    #[test]
    fn test_state_transition_patrol() {
        let mut enemy = Enemy::new(Vec3::ZERO, 5.0);
        let player_pos = Vec3::new(50.0, 0.0, 0.0); // Far away
        let broken_anchors = vec![]; // No broken anchors

        enemy.update(0.1, Vec3::ZERO, player_pos, &broken_anchors);
        assert_eq!(enemy.state, EnemyState::Patrol);
        assert_eq!(enemy.target_anchor_id, None);
    }

    #[test]
    fn test_patrol_behavior() {
        let mut enemy = Enemy::new(Vec3::new(10.0, 0.0, 5.0), 5.0);
        let player_pos = Vec3::new(100.0, 0.0, 0.0);

        let behavior = enemy.update(0.1, enemy.patrol_center, player_pos, &[]);

        if let EnemyBehavior::MoveTo(target) = behavior {
            // Target should be within patrol radius
            let distance = target.distance(enemy.patrol_center);
            assert!(
                distance <= enemy.patrol_radius + 0.1,
                "Patrol target too far from center"
            );
        } else {
            panic!("Expected MoveTo behavior");
        }
    }

    #[test]
    fn test_attack_anchor_behavior() {
        let mut enemy = Enemy::new(Vec3::ZERO, 5.0);
        let anchor_pos = Vec3::new(1.5, 0.0, 0.0); // Within attack range (2.0)
        let broken_anchors = vec![(0, anchor_pos)];
        let player_pos = Vec3::new(50.0, 0.0, 0.0);

        // First update: transition to AttackAnchor state
        enemy.update(0.1, Vec3::ZERO, player_pos, &broken_anchors);
        assert_eq!(enemy.state, EnemyState::AttackAnchor);

        // Second update: should attack
        let behavior = enemy.update(0.1, Vec3::ZERO, player_pos, &broken_anchors);
        assert!(matches!(
            behavior,
            EnemyBehavior::Attack(AttackTarget::Anchor(0))
        ));
    }

    #[test]
    fn test_engage_player_behavior() {
        let mut enemy = Enemy::new(Vec3::ZERO, 5.0);
        let player_pos = Vec3::new(1.5, 0.0, 0.0); // Within attack range (2.0)

        // First update: transition to EngagePlayer state
        enemy.update(0.1, Vec3::ZERO, player_pos, &[]);
        assert_eq!(enemy.state, EnemyState::EngagePlayer);

        // Second update: should attack
        let behavior = enemy.update(0.1, Vec3::ZERO, player_pos, &[]);
        assert!(matches!(
            behavior,
            EnemyBehavior::Attack(AttackTarget::Player)
        ));
    }

    #[test]
    fn test_find_nearest_anchor() {
        let enemy = Enemy::new(Vec3::ZERO, 5.0);
        let anchors = vec![
            (0, Vec3::new(10.0, 0.0, 0.0)),
            (1, Vec3::new(5.0, 0.0, 0.0)), // Nearest
            (2, Vec3::new(20.0, 0.0, 0.0)),
        ];

        let nearest = enemy.find_nearest_anchor(Vec3::ZERO, &anchors);
        assert_eq!(nearest, Some((1, Vec3::new(5.0, 0.0, 0.0))));
    }

    #[test]
    fn test_dead_enemy_stays_dead() {
        let mut enemy = Enemy::new(Vec3::ZERO, 5.0);
        enemy.take_damage(100.0);
        assert_eq!(enemy.state, EnemyState::Dead);

        // Updates should not change state
        enemy.update(0.1, Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0), &[]);
        assert_eq!(enemy.state, EnemyState::Dead);

        let behavior = enemy.update(0.1, Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0), &[]);
        assert!(matches!(behavior, EnemyBehavior::Idle));
    }

    #[test]
    fn test_priority_flee_over_engage() {
        let mut enemy = Enemy::new(Vec3::ZERO, 5.0);
        enemy.take_damage(85.0); // Health = 15 HP (flee threshold)

        let player_pos = Vec3::new(5.0, 0.0, 0.0); // Within aggro range
        enemy.update(0.1, Vec3::ZERO, player_pos, &[]);

        // Should flee despite player being in aggro range
        assert_eq!(enemy.state, EnemyState::Flee);
    }

    #[test]
    fn test_priority_engage_over_attack_anchor() {
        let mut enemy = Enemy::new(Vec3::ZERO, 5.0);
        let player_pos = Vec3::new(8.0, 0.0, 0.0); // Within aggro range
        let broken_anchors = vec![(0, Vec3::new(10.0, 0.0, 0.0))];

        enemy.update(0.1, Vec3::ZERO, player_pos, &broken_anchors);

        // Should engage player despite broken anchor nearby
        assert_eq!(enemy.state, EnemyState::EngagePlayer);
    }

    fn first_patrol_target(seed: u64) -> Vec3 {
        let mut enemy = Enemy::new(Vec3::new(10.0, 0.0, 5.0), 5.0);
        enemy.set_world_seed(seed);
        let behavior = enemy.update(0.1, enemy.patrol_center, Vec3::new(100.0, 0.0, 0.0), &[]);
        match behavior {
            EnemyBehavior::MoveTo(target) => target,
            other => panic!("expected patrol MoveTo, got {other:?}"),
        }
    }

    #[test]
    fn patrol_target_is_bit_identical_for_the_same_seed() {
        let a = first_patrol_target(0xA11C);
        let b = first_patrol_target(0xA11C);
        assert_eq!(a.x.to_bits(), b.x.to_bits());
        assert_eq!(a.y.to_bits(), b.y.to_bits());
        assert_eq!(a.z.to_bits(), b.z.to_bits());
        assert!(a.distance(Vec3::new(10.0, 0.0, 5.0)) <= 5.0);
        // Locked after the first seeded run. A libm sin/cos port would miss these bits.
        assert_eq!(
            (a.x.to_bits(), a.y.to_bits(), a.z.to_bits()),
            (1_094_187_435, 0, 1_080_845_884)
        );
    }
}
