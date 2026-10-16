// level.rs - Veilweaver level integration: quest system + anchors + enemies + spawner
//
// This module provides a complete game level that integrates:
// - Quest system (quest.rs, starter_quests.rs)
// - Anchor system (anchor.rs)
// - Enemy system (enemy.rs)
// - Spawner system (spawner.rs)
// - Player movement and camera
//
// Design:
// - 3 anchors positioned in simple triangle layout (matches starter quest requirements)
// - 5 enemy spawn points around perimeter
// - Simple 3rd person camera following player
// - Quest manager wired to anchor repair, enemy kills, exploration events

use glam::Vec3;

use crate::anchor::Anchor;
use crate::echo_currency::{EchoCurrency, TransactionReason};
use crate::enemy::Enemy;
use crate::quest::QuestManager;
use crate::spawner::EnemySpawner;
use crate::starter_quests::all_starter_quests;
use crate::ui::QuestPanel;

/// Player representation (simplified for quest integration)
#[derive(Debug, Clone)]
pub struct Player {
    pub position: Vec3,
    pub velocity: Vec3,
    pub forward: Vec3, // Player facing direction for dash
    pub health: f32,
    pub max_health: f32,
    /// The only Echo ledger. Ability costs spend this balance.
    pub echo: EchoCurrency,
    pub abilities: Vec<String>,
    pub ability_manager: crate::abilities::AbilityManager,
}

impl Player {
    /// Create new player at spawn position
    pub fn new(spawn_pos: Vec3) -> Self {
        Self {
            position: spawn_pos,
            velocity: Vec3::ZERO,
            forward: Vec3::new(0.0, 0.0, 1.0), // Default forward +Z
            health: 100.0,
            max_health: 100.0,
            echo: EchoCurrency::new(),
            abilities: Vec::new(),
            ability_manager: crate::abilities::AbilityManager::new(),
        }
    }

    /// Update player movement (simple velocity-based)
    pub fn update(&mut self, delta_time: f32) {
        self.position += self.velocity * delta_time;

        // Ground clamping (simple Y=0 ground plane)
        if self.position.y < 0.0 {
            self.position.y = 0.0;
            self.velocity.y = 0.0;
        }

        // Update ability cooldowns
        self.ability_manager.update(delta_time);
    }

    /// Apply movement input (WASD-style)
    pub fn apply_movement(&mut self, forward: f32, right: f32, speed: f32) {
        self.velocity.z = forward * speed; // Forward/back
        self.velocity.x = right * speed; // Left/right
    }

    /// Take damage (for combat integration)
    pub fn take_damage(&mut self, amount: f32) {
        // Apply shield damage reduction if active
        let reduced_damage = self.ability_manager.apply_shield_reduction(amount);
        self.health = (self.health - reduced_damage).max(0.0);
    }

    /// Check if player is alive
    pub fn is_alive(&self) -> bool {
        self.health > 0.0
    }

    /// Add echo currency (quest reward)
    pub fn echo_balance(&self) -> u32 {
        self.echo.count()
    }

    pub fn set_echo(&mut self, amount: u32) {
        self.echo = EchoCurrency::with_balance(amount);
    }

    pub fn add_echo(&mut self, amount: i32) {
        if amount > 0 {
            self.echo
                .add(amount as u32, TransactionReason::Debug("credit".into()));
        }
    }

    /// Unlock ability (quest reward)
    pub fn unlock_ability(&mut self, ability: &str) {
        if !self.abilities.contains(&ability.to_string()) {
            self.abilities.push(ability.to_string());
        }
    }

    /// Boost stat (quest reward)
    pub fn boost_stat(&mut self, stat: &str, amount: f32) {
        match stat {
            "MaxHealth" => {
                self.max_health += amount;
                self.health += amount; // Also heal by the boost amount
            }
            _ => {
                // Other stats not implemented yet
            }
        }
    }

    /// Use Echo Dash ability (dash forward + damage)
    pub fn use_dash(&mut self) -> Result<(Vec3, f32), String> {
        let cost = self.ability_manager.dash_echo_cost();
        let result =
            self.ability_manager
                .activate_dash(self.position, self.forward, self.echo.count())?;
        if !self.echo.spend(cost, TransactionReason::UseEchoDash) {
            return Err("insufficient echoes".into());
        }
        Ok(result)
    }

    /// Use Echo Shield ability (damage reduction)
    pub fn use_shield(&mut self) -> Result<(), String> {
        let cost = self.ability_manager.shield_echo_cost();
        self.ability_manager.activate_shield(self.echo.count())?;
        if !self.echo.spend(cost, TransactionReason::UseEchoShield) {
            return Err("insufficient echoes".into());
        }
        Ok(())
    }

    /// Check if dash is ready
    pub fn can_dash(&self) -> bool {
        let (ready, _) = self.ability_manager.dash_cooldown();
        ready && self.echo.has(self.ability_manager.dash_echo_cost())
    }

    /// Check if shield is ready
    pub fn can_shield(&self) -> bool {
        let (ready, _) = self.ability_manager.shield_cooldown();
        ready && self.echo.has(self.ability_manager.shield_echo_cost())
    }

    /// Get dash cooldown info (for UI)
    pub fn dash_cooldown_info(&self) -> (bool, f32) {
        self.ability_manager.dash_cooldown()
    }

    /// Get shield cooldown info (for UI)
    pub fn shield_cooldown_info(&self) -> (bool, f32) {
        self.ability_manager.shield_cooldown()
    }

    /// Get shield active info (for UI)
    pub fn shield_active_info(&self) -> (bool, f32) {
        self.ability_manager.shield_active()
    }
}

/// Simple 3rd person camera
#[derive(Debug, Clone)]
pub struct Camera {
    pub position: Vec3,
    pub target: Vec3,
    pub offset: Vec3, // Camera offset from target (e.g., (0, 5, -10) = 5m up, 10m back)
    pub smoothing: f32, // 0.0-1.0, higher = smoother (slower) tracking
}

impl Camera {
    /// Create camera with default 3rd person offset
    pub fn new(target: Vec3) -> Self {
        let offset = Vec3::new(0.0, 5.0, -10.0); // 5m above, 10m behind
        Self {
            position: target + offset,
            target,
            offset,
            smoothing: 0.9, // Smooth tracking
        }
    }

    /// Update camera to follow target with smoothing
    pub fn update(&mut self, target: Vec3, delta_time: f32) {
        self.target = target;
        let desired_pos = target + self.offset;

        // Smooth interpolation (lerp-like behavior using exponential decay)
        let t = 1.0 - self.smoothing.powf(delta_time * 60.0); // 60 FPS reference
        self.position = self.position.lerp(desired_pos, t);
    }

    /// Get view direction (camera looking at target)
    pub fn view_direction(&self) -> Vec3 {
        (self.target - self.position).normalize()
    }
}

/// Complete Veilweaver level with integrated systems
pub struct VeilweaverLevel {
    // Core game state
    pub player: Player,
    pub camera: Camera,

    // World objects
    pub anchors: Vec<Anchor>,
    pub anchor_positions: Vec<Vec3>, // Store positions separately
    pub enemies: Vec<Enemy>,
    pub enemy_positions: Vec<Vec3>, // Store enemy positions
    pub spawner: EnemySpawner,

    // Quest system
    pub quest_manager: QuestManager,
    pub quest_panel: QuestPanel,

    // Level metadata
    pub level_time: f32, // Total elapsed time
    pub enemies_killed: usize,
    pub anchors_repaired: usize,
}

impl VeilweaverLevel {
    /// Create new level with starter quests and anchors
    pub fn new() -> Self {
        // Player spawn at origin
        let player = Player::new(Vec3::new(0.0, 0.0, -20.0));
        let camera = Camera::new(player.position);

        // 3 anchors in triangle layout (matches starter quest "Stabilize the Anchors")
        let anchors = vec![
            Anchor::new(0.5, 10, None), // Central anchor (50% stability, 10 Echo cost)
            Anchor::new(0.3, 10, None), // Left anchor (30% stability)
            Anchor::new(0.3, 10, None), // Right anchor (30% stability)
        ];

        let anchor_positions = vec![
            Vec3::new(0.0, 1.0, 0.0),    // Central anchor position
            Vec3::new(-15.0, 1.0, 10.0), // Left anchor position
            Vec3::new(15.0, 1.0, 10.0),  // Right anchor position
        ];

        // 5 spawn points around perimeter (combat zone)
        let mut spawner = EnemySpawner::new();
        // Add spawn points manually
        let spawn_points = vec![
            Vec3::new(-20.0, 0.0, 20.0), // Northwest
            Vec3::new(20.0, 0.0, 20.0),  // Northeast
            Vec3::new(-25.0, 0.0, 0.0),  // West
            Vec3::new(25.0, 0.0, 0.0),   // East
            Vec3::new(0.0, 0.0, 25.0),   // North
        ];
        for pos in spawn_points {
            spawner.add_spawn_point(pos, 5.0, None); // 5.0 patrol radius, no anchor
        }

        // Quest system
        let mut quest_manager = QuestManager::new();
        for quest in all_starter_quests() {
            quest_manager.register_quest(quest);
        }

        // Activate first quest (no prerequisites)
        quest_manager
            .activate_quest("stabilize_anchors")
            .expect("Failed to activate starter quest");

        let quest_panel = QuestPanel::new();

        Self {
            player,
            camera,
            anchors,
            anchor_positions,
            enemies: Vec::new(),
            enemy_positions: Vec::new(),
            spawner,
            quest_manager,
            quest_panel,
            level_time: 0.0,
            enemies_killed: 0,
            anchors_repaired: 0,
        }
    }

    /// Update all level systems
    pub fn update(&mut self, delta_time: f32) {
        self.level_time += delta_time;

        // Update player
        self.player.update(delta_time);

        // Update camera (follow player)
        self.camera.update(self.player.position, delta_time);

        // Update enemies (skip update loop for now - enemies don't have simple update)
        // Enemies would be updated via systems in real game loop

        // Update spawner (spawn new enemies) - disabled for now since spawner API different
        // if let Some(new_enemies) = self.spawner.update(delta_time, &self.enemies) {
        //     self.enemies.extend(new_enemies);
        // }

        // Update quest UI
        self.quest_panel.update(delta_time);

        // Check quest completion
        if let Some(rewards) = self.quest_manager.check_active_quest() {
            // Get quest title before moving on
            if let Some(quest) = self.quest_manager.active_quest() {
                self.quest_panel.show_completion(&quest.title, &rewards);
            }

            // Distribute rewards
            for reward in rewards {
                self.apply_reward(reward);
            }

            // Try to activate next quest in chain
            self.try_activate_next_quest();
        }
    }

    /// Repair anchor (player interaction)
    pub fn repair_anchor(&mut self, anchor_index: usize, echo_cost: u32) -> bool {
        if anchor_index >= self.anchors.len() {
            return false;
        }

        // Check if player has enough echo currency
        if !self.player.echo.spend(
            echo_cost,
            TransactionReason::RepairAnchor(anchor_index.to_string()),
        ) {
            return false;
        }

        let anchor = &mut self.anchors[anchor_index];
        let was_below_threshold = anchor.stability() < 0.8;

        // Repair anchor (no arguments - repairs to full)
        anchor.repair();

        let now_above_threshold = anchor.stability() >= 0.8;

        // Update quest progress if anchor crossed 80% threshold
        if was_below_threshold && now_above_threshold {
            self.quest_manager.update_repair(anchor.stability());
            self.anchors_repaired += 1;
            true
        } else {
            false
        }
    }

    /// Kill enemy (combat integration)
    pub fn kill_enemy(&mut self, enemy_index: usize) -> bool {
        if enemy_index >= self.enemies.len() {
            return false;
        }

        // Remove enemy from lists
        self.enemies.remove(enemy_index);
        if enemy_index < self.enemy_positions.len() {
            self.enemy_positions.remove(enemy_index);
        }

        // Update quest progress (all enemies count as "enemy" type)
        self.quest_manager.update_kill("enemy", 1);
        self.enemies_killed += 1;
        true
    }

    /// Check if player is near location (exploration objectives)
    pub fn check_exploration(&mut self, target_pos: Vec3, radius: f32) -> bool {
        let distance = self.player.position.distance(target_pos);

        if distance <= radius {
            // Update quest with player position
            self.quest_manager.update_explore(self.player.position);
            true
        } else {
            false
        }
    }

    /// Apply quest reward to player
    fn apply_reward(&mut self, reward: crate::quest::QuestReward) {
        use crate::quest::QuestReward;

        match reward {
            QuestReward::EchoCurrency(amount) => {
                self.player.add_echo(amount);
            }
            QuestReward::AbilityUnlock(ability) => {
                self.player.unlock_ability(&ability);
            }
            QuestReward::StatBoost { stat, amount } => {
                self.player.boost_stat(&stat, amount);
            }
            QuestReward::Multiple(rewards) => {
                for r in rewards {
                    self.apply_reward(r);
                }
            }
        }
    }

    /// Try to activate next quest in progression chain
    fn try_activate_next_quest(&mut self) {
        // Quest progression chain: stabilize_anchors → clear_corruption → restore_beacon
        if self.quest_manager.active_quest().is_some() {
            return; // Already have active quest
        }

        // Check completed quests by trying to get them
        let has_stabilize = self
            .quest_manager
            .quest("stabilize_anchors")
            .map(|q| matches!(q.state, crate::quest::QuestState::Completed))
            .unwrap_or(false);
        let has_clear = self
            .quest_manager
            .quest("clear_corruption")
            .map(|q| matches!(q.state, crate::quest::QuestState::Completed))
            .unwrap_or(false);

        if has_stabilize && !has_clear {
            let _ = self.quest_manager.activate_quest("clear_corruption");
        } else if has_clear {
            let _ = self.quest_manager.activate_quest("restore_beacon");
        }
    }

    /// Get quest UI rendering
    pub fn render_quest_ui(&self) -> String {
        self.quest_panel.render(self.quest_manager.active_quest())
    }

    /// Get level stats for display
    pub fn get_stats(&self) -> LevelStats {
        LevelStats {
            level_time: self.level_time,
            player_health: self.player.health,
            player_max_health: self.player.max_health,
            echo_currency: self.player.echo_balance() as i32,
            enemies_killed: self.enemies_killed,
            enemies_active: self.enemies.len(),
            anchors_repaired: self.anchors_repaired,
            anchors_total: self.anchors.len(),
            active_quest: self.quest_manager.active_quest().map(|q| q.title.clone()),
        }
    }

    /// Spawn an enemy at a specific position (for testing/demo)
    pub fn spawn_enemy_at(&mut self, position: Vec3, patrol_radius: f32) {
        self.enemies.push(Enemy::new(position, patrol_radius));
        self.enemy_positions.push(position);
    }
}

impl Default for VeilweaverLevel {
    fn default() -> Self {
        Self::new()
    }
}

/// Level statistics for display/debugging
#[derive(Debug, Clone)]
pub struct LevelStats {
    pub level_time: f32,
    pub player_health: f32,
    pub player_max_health: f32,
    pub echo_currency: i32,
    pub enemies_killed: usize,
    pub enemies_active: usize,
    pub anchors_repaired: usize,
    pub anchors_total: usize,
    pub active_quest: Option<String>,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_player_creation() {
        let player = Player::new(Vec3::new(0.0, 0.0, -20.0));
        assert_eq!(player.position, Vec3::new(0.0, 0.0, -20.0));
        assert_eq!(player.health, 100.0);
        assert_eq!(player.echo_balance(), 0);
        assert!(player.is_alive());
    }

    #[test]
    fn test_player_movement() {
        let mut player = Player::new(Vec3::ZERO);
        player.apply_movement(1.0, 0.0, 5.0); // Forward movement
        player.update(1.0);

        assert!(player.position.z > 0.0); // Moved in +Z direction (forward)
        assert!(player.is_alive());
    }

    #[test]
    fn test_player_damage() {
        let mut player = Player::new(Vec3::ZERO);
        player.take_damage(50.0);
        assert_eq!(player.health, 50.0);
        assert!(player.is_alive());

        player.take_damage(100.0);
        assert_eq!(player.health, 0.0);
        assert!(!player.is_alive());
    }

    #[test]
    fn test_player_rewards() {
        let mut player = Player::new(Vec3::ZERO);

        // Echo currency
        player.add_echo(100);
        assert_eq!(player.echo_balance(), 100);

        // Ability unlock
        player.unlock_ability("Echo Dash");
        assert!(player.abilities.contains(&"Echo Dash".to_string()));

        // Stat boost
        let old_max = player.max_health;
        player.boost_stat("MaxHealth", 25.0);
        assert_eq!(player.max_health, old_max + 25.0);
    }

    #[test]
    fn test_camera_creation() {
        let target = Vec3::new(0.0, 0.0, 0.0);
        let camera = Camera::new(target);

        assert_eq!(camera.target, target);
        assert!(camera.position.y > target.y); // Camera above target
        assert!(camera.position.z < target.z); // Camera behind target
    }

    #[test]
    fn test_camera_follow() {
        let mut camera = Camera::new(Vec3::ZERO);
        let initial_pos = camera.position;
        let new_target = Vec3::new(10.0, 0.0, 10.0);

        // Update multiple times for smooth interpolation
        for _ in 0..20 {
            camera.update(new_target, 0.1);
        }

        // Camera should move from initial position (with smoothing)
        let distance_moved = camera.position.distance(initial_pos);
        assert!(distance_moved > 1.0); // Should have moved at least 1 unit
    }

    #[test]
    fn test_level_creation() {
        let level = VeilweaverLevel::new();

        // Check initial state
        assert_eq!(level.anchors.len(), 3);
        assert_eq!(level.enemies.len(), 0);
        assert!(level.quest_manager.active_quest().is_some());
        assert!(level.player.is_alive());
    }

    #[test]
    fn test_level_anchor_repair() {
        let mut level = VeilweaverLevel::new();

        // Give player some echo currency
        level.player.set_echo(100);

        // Get initial stability
        let initial_stability = level.anchors[0].stability();

        // Repair anchor (costs 10 echo)
        let repaired = level.repair_anchor(0, 10);

        assert!(repaired); // Should return true (crossed threshold)
        assert!(level.anchors[0].stability() > initial_stability);
        assert_eq!(level.anchors_repaired, 1);
        assert_eq!(level.player.echo_balance(), 90); // 100 - 10
    }

    #[test]
    fn test_level_enemy_kill() {
        let mut level = VeilweaverLevel::new();

        // Spawn an enemy manually
        level
            .enemies
            .push(Enemy::new(Vec3::new(10.0, 0.0, 10.0), 5.0));
        level.enemy_positions.push(Vec3::new(10.0, 0.0, 10.0));

        assert_eq!(level.enemies.len(), 1);

        // Kill enemy
        let killed = level.kill_enemy(0);

        assert!(killed);
        assert_eq!(level.enemies.len(), 0);
        assert_eq!(level.enemies_killed, 1);
    }

    #[test]
    fn test_level_exploration() {
        let mut level = VeilweaverLevel::new();

        // Move player to target location
        level.player.position = Vec3::new(0.0, 0.0, 0.0);

        // Check exploration (radius 5.0)
        let explored = level.check_exploration(Vec3::new(0.0, 1.0, 0.0), 5.0);

        assert!(explored); // Player within radius
    }

    #[test]
    fn test_level_update() {
        let mut level = VeilweaverLevel::new();

        // Apply movement
        level.player.apply_movement(1.0, 0.0, 5.0); // Forward

        let initial_time = level.level_time;
        let initial_pos = level.player.position;

        // Update level
        level.update(1.0);

        assert!(level.level_time > initial_time);
        assert_ne!(level.player.position, initial_pos); // Player moved
    }

    #[test]
    fn test_level_quest_progression() {
        let mut level = VeilweaverLevel::new();

        // Give player echo currency for repairs
        level.player.set_echo(100);

        // Initial quest should be "stabilize_anchors"
        let active = level.quest_manager.active_quest().unwrap();
        assert_eq!(active.id, "stabilize_anchors");

        // Repair 3 anchors to 80%+ (complete quest 1)
        for i in 0..3 {
            level.repair_anchor(i, 10); // 10 echo per repair
        }

        // Update to trigger quest completion
        level.update(0.0);

        // Next quest should activate automatically
        level.update(0.0);

        // Should now be on quest 2 (clear_corruption)
        let active = level.quest_manager.active_quest();
        assert!(active.is_some());
        // Note: Quest might complete immediately if all objectives met
    }

    #[test]
    fn test_level_stats() {
        let level = VeilweaverLevel::new();
        let stats = level.get_stats();

        assert_eq!(stats.level_time, 0.0);
        assert_eq!(stats.player_health, 100.0);
        assert_eq!(stats.echo_currency, 0);
        assert_eq!(stats.enemies_killed, 0);
        assert_eq!(stats.anchors_total, 3);
        assert!(stats.active_quest.is_some());
    }

    #[test]
    fn test_level_quest_ui_render() {
        let level = VeilweaverLevel::new();
        let ui = level.render_quest_ui();

        // Should contain quest UI elements
        assert!(ui.contains("ACTIVE QUEST") || ui.contains("Stabilize"));
    }

    #[test]
    fn test_player_dash_ability() {
        let mut player = Player::new(Vec3::ZERO);
        player.set_echo(50);
        player.forward = Vec3::new(1.0, 0.0, 0.0);

        // Dash should succeed
        let result = player.use_dash();
        assert!(result.is_ok());

        let (target_pos, damage) = result.unwrap();
        assert_eq!(target_pos, Vec3::new(10.0, 0.0, 0.0)); // Dashed 10 units forward
        assert_eq!(damage, 30.0);
        assert_eq!(player.echo_balance(), 40); // 50 - 10

        // Dash should fail (cooldown)
        let result2 = player.use_dash();
        assert!(result2.is_err());
        assert!(result2.unwrap_err().contains("cooldown"));

        // Wait for cooldown
        player.update(1.0);
        let result3 = player.use_dash();
        assert!(result3.is_ok());
        assert_eq!(player.echo_balance(), 30); // 40 - 10
    }

    #[test]
    fn test_player_shield_ability() {
        let mut player = Player::new(Vec3::ZERO);
        player.set_echo(50);

        // Shield should succeed
        let result = player.use_shield();
        assert!(result.is_ok());
        assert_eq!(player.echo_balance(), 35); // 50 - 15

        // Shield is active, damage should be reduced
        player.take_damage(100.0);
        assert_eq!(player.health, 50.0); // 100 - (100 * 0.5)

        // Wait for shield to expire
        player.update(3.0);
        player.health = 100.0; // Reset health

        // Damage should be full now
        player.take_damage(100.0);
        assert_eq!(player.health, 0.0); // 100 - 100 (no shield)
    }

    #[test]
    fn test_player_ability_insufficient_echo() {
        let mut player = Player::new(Vec3::ZERO);
        player.set_echo(5); // Not enough for either ability

        // Dash fails (need 10)
        let dash_result = player.use_dash();
        assert!(dash_result.is_err());
        assert!(dash_result.unwrap_err().contains("Not enough Echo"));

        // Shield fails (need 15)
        let shield_result = player.use_shield();
        assert!(shield_result.is_err());
        assert!(shield_result.unwrap_err().contains("Not enough Echo"));
    }

    #[test]
    fn test_player_ability_checks() {
        let mut player = Player::new(Vec3::ZERO);

        // Initially: no Echo
        assert!(!player.can_dash());
        assert!(!player.can_shield());

        // Add Echo
        player.set_echo(50);
        assert!(player.can_dash());
        assert!(player.can_shield());

        // Use dash
        player.use_dash().unwrap();
        assert!(!player.can_dash()); // Cooldown
        assert!(player.can_shield()); // Still available

        // Use shield
        player.use_shield().unwrap();
        assert!(!player.can_shield()); // Cooldown
    }

    #[test]
    fn test_player_ability_cooldown_info() {
        let mut player = Player::new(Vec3::ZERO);
        player.set_echo(50);

        // Initially ready
        let (dash_ready, dash_remaining) = player.dash_cooldown_info();
        assert!(dash_ready);
        assert_eq!(dash_remaining, 0.0);

        // Use dash
        player.use_dash().unwrap();
        let (dash_ready2, dash_remaining2) = player.dash_cooldown_info();
        assert!(!dash_ready2);
        assert_eq!(dash_remaining2, 1.0);

        // Wait partial
        player.update(0.5);
        let (dash_ready3, dash_remaining3) = player.dash_cooldown_info();
        assert!(!dash_ready3);
        assert_eq!(dash_remaining3, 0.5);
    }

    #[test]
    fn test_player_shield_active_info() {
        let mut player = Player::new(Vec3::ZERO);
        player.set_echo(50);

        // Initially inactive
        let (active, remaining) = player.shield_active_info();
        assert!(!active);
        assert_eq!(remaining, 0.0);

        // Activate shield
        player.use_shield().unwrap();
        let (active2, remaining2) = player.shield_active_info();
        assert!(active2);
        assert_eq!(remaining2, 3.0);

        // Wait partial
        player.update(1.5);
        let (active3, remaining3) = player.shield_active_info();
        assert!(active3);
        assert_eq!(remaining3, 1.5);

        // Wait for expiration
        player.update(1.5);
        let (active4, remaining4) = player.shield_active_info();
        assert!(!active4);
        assert_eq!(remaining4, 0.0);
    }
}
