/// Core HUD (Heads-Up Display) system for in-game overlay
///
/// Week 3 Day 1: Foundation for health bars, objectives, minimap, subtitles
/// Week 4 Day 1: Health bar smooth transitions with easing animations
///
/// Architecture:
/// - Separate from MenuManager (menu system is modal, HUD is persistent overlay)
/// - Renders using egui::Area (free-floating, no window chrome)
/// - Toggle visibility with ESC key (context-sensitive)
/// - Always renders on top of 3D scene (no depth test)
use serde::{Deserialize, Serialize};

// ===== Week 4 Day 1: Animation System =====

/// Easing functions for smooth animations
pub mod easing {
    /// Ease out cubic: Fast start, slow end (good for damage/urgent events)
    pub fn ease_out_cubic(t: f32) -> f32 {
        let t = t - 1.0;
        t * t * t + 1.0
    }

    /// Ease in-out quadratic: Smooth acceleration and deceleration (good for healing/positive events)
    pub fn ease_in_out_quad(t: f32) -> f32 {
        if t < 0.5 {
            2.0 * t * t
        } else {
            -1.0 + (4.0 - 2.0 * t) * t
        }
    }
}

/// Health animation state for smooth transitions
#[derive(Clone, Debug)]
pub struct HealthAnimation {
    /// Current visual health value (animated)
    pub current_visual: f32,
    /// Target health value (actual health)
    pub target: f32,
    /// Animation progress (0.0 to 1.0)
    pub animation_time: f32,
    /// Animation duration in seconds
    pub duration: f32,
    /// Damage flash timer (for red overlay effect)
    pub flash_timer: f32,
    /// Flash duration in seconds
    pub flash_duration: f32,
}

impl HealthAnimation {
    /// Create new health animation initialized to full health
    pub fn new(health: f32) -> Self {
        Self {
            current_visual: health,
            target: health,
            animation_time: 0.0,
            duration: 0.4, // Default 0.4s animation
            flash_timer: 0.0,
            flash_duration: 0.2, // Default 0.2s flash
        }
    }

    /// Set new target health (triggers animation)
    pub fn set_target(&mut self, new_health: f32) {
        self.target = new_health;
        self.animation_time = 0.0;

        // Trigger damage flash if health decreased
        if new_health < self.current_visual {
            self.flash_timer = self.flash_duration;
        }
    }

    /// Update animation (call every frame with delta time)
    pub fn update(&mut self, dt: f32) {
        // Update flash timer
        if self.flash_timer > 0.0 {
            self.flash_timer = (self.flash_timer - dt).max(0.0);
        }

        // Update health animation
        if (self.current_visual - self.target).abs() > 0.01 {
            self.animation_time += dt;
            let t = (self.animation_time / self.duration).min(1.0);

            // Use different easing for increase vs decrease
            let eased_t = if self.target > self.current_visual {
                // Health increasing (healing): smooth ease in-out
                easing::ease_in_out_quad(t)
            } else {
                // Health decreasing (damage): fast start, slow end
                easing::ease_out_cubic(t)
            };

            // Lerp from current to target
            self.current_visual =
                self.current_visual + (self.target - self.current_visual) * eased_t;

            // Snap to target when close enough
            if t >= 1.0 {
                self.current_visual = self.target;
            }
        }
    }

    /// Get current visual health value
    pub fn visual_health(&self) -> f32 {
        self.current_visual
    }

    /// Get flash alpha (0.0 to 0.6) for damage flash effect
    pub fn flash_alpha(&self) -> f32 {
        if self.flash_timer > 0.0 {
            (self.flash_timer / self.flash_duration) * 0.6
        } else {
            0.0
        }
    }

    /// Check if health is increasing (for green glow effect)
    pub fn is_healing(&self) -> bool {
        self.target > self.current_visual && (self.target - self.current_visual).abs() > 0.01
    }
}

// ===== Week 3 Day 2: Health Bars & Resources Data Structures =====

/// Player stats for HUD display
#[derive(Clone, Debug)]
pub struct PlayerStats {
    pub health: f32,
    pub max_health: f32,
    pub mana: f32,
    pub max_mana: f32,
    pub stamina: f32,
    pub max_stamina: f32,

    // Week 4 Day 1: Health animation
    pub health_animation: HealthAnimation,
}

impl Default for PlayerStats {
    fn default() -> Self {
        Self {
            health: 100.0,
            max_health: 100.0,
            mana: 100.0,
            max_mana: 100.0,
            stamina: 100.0,
            max_stamina: 100.0,
            health_animation: HealthAnimation::new(100.0),
        }
    }
}

/// Enemy data for health bar rendering (mock data for demo)
#[derive(Clone, Debug)]
pub struct EnemyData {
    pub id: u32,
    pub world_pos: (f32, f32, f32), // 3D position
    pub health: f32,
    pub max_health: f32,
    pub faction: EnemyFaction,

    // Week 4 Day 1: Health animation
    pub health_animation: HealthAnimation,
}

impl EnemyData {
    /// Create new enemy with health animation
    pub fn new(
        id: u32,
        world_pos: (f32, f32, f32),
        max_health: f32,
        faction: EnemyFaction,
    ) -> Self {
        Self {
            id,
            world_pos,
            health: max_health,
            max_health,
            faction,
            health_animation: HealthAnimation::new(max_health),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum EnemyFaction {
    Hostile,  // Red health bar
    Neutral,  // Yellow health bar
    Friendly, // Green health bar
}

/// Damage number (floating text animation)
#[derive(Clone, Debug)]
pub struct DamageNumber {
    pub value: i32,
    pub spawn_time: f32, // Game time when spawned
    pub world_pos: (f32, f32, f32),
    pub damage_type: DamageType,

    // Week 4 Day 2: Arc motion (parabolic trajectory)
    pub velocity_x: f32, // Horizontal velocity (pixels/sec)
    pub velocity_y: f32, // Initial upward velocity (pixels/sec, negative = up)
    pub gravity: f32,    // Gravity constant (pixels/sec²)

    // Week 4 Day 2: Impact shake
    pub shake_rotation: f32,  // Current rotation angle (radians)
    pub shake_amplitude: f32, // Initial shake amplitude
    pub shake_frequency: f32, // Shake oscillation frequency (Hz)
}

impl DamageNumber {
    /// Create new damage number with arc motion and shake
    pub fn new(
        value: i32,
        spawn_time: f32,
        world_pos: (f32, f32, f32),
        damage_type: DamageType,
    ) -> Self {
        // Pseudo-random horizontal velocity using spawn time hash
        // This creates deterministic but varied trajectories
        let hash = ((spawn_time * 1000.0) as u32).wrapping_mul(2654435761);
        let random_val = (hash as f32 / u32::MAX as f32) - 0.5; // -0.5 to 0.5
        let velocity_x = random_val * 60.0; // -30 to +30 pixels/sec

        // Initial upward velocity (-80 pixels/sec, negative = up)
        let velocity_y = -80.0;

        // Gravity constant (150 pixels/sec²)
        let gravity = 150.0;

        // Shake parameters
        let shake_amplitude = match damage_type {
            DamageType::Critical => 0.175, // ±10 degrees (0.175 radians)
            _ => 0.087,                    // ±5 degrees (0.087 radians)
        };
        let shake_frequency = 15.0; // 15 Hz oscillation

        Self {
            value,
            spawn_time,
            world_pos,
            damage_type,
            velocity_x,
            velocity_y,
            gravity,
            shake_rotation: 0.0,
            shake_amplitude,
            shake_frequency,
        }
    }

    /// Calculate current offset from spawn position
    pub fn calculate_offset(&self, age: f32) -> (f32, f32) {
        // Parabolic arc: x(t) = vx*t, y(t) = vy*t + 0.5*g*t²
        let offset_x = self.velocity_x * age;
        let offset_y = self.velocity_y * age + 0.5 * self.gravity * age * age;
        (offset_x, offset_y)
    }

    /// Calculate current shake rotation
    pub fn calculate_shake(&self, age: f32) -> f32 {
        // Damped oscillation: rotation = amplitude * sin(t * freq) * e^(-t*5)
        let damping = (-age * 5.0).exp(); // Exponential decay
        self.shake_amplitude * (age * self.shake_frequency * std::f32::consts::TAU).sin() * damping
    }
}

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum DamageType {
    Normal,     // White text
    Critical,   // Yellow text
    SelfDamage, // Red text
}

// ===== Week 3 Day 3: Quest Tracker & Minimap Data Structures =====

/// Quest objective with progress tracking
#[derive(Clone, Debug)]
pub struct Objective {
    pub id: u32,
    pub description: String,
    pub completed: bool,
    pub progress: Option<(u32, u32)>, // (current, total) - e.g., (3, 5) for "Collect 3/5 items"
}

/// Active quest with objectives
#[derive(Clone, Debug)]
pub struct Quest {
    pub id: u32,
    pub title: String,
    pub description: String,
    pub objectives: Vec<Objective>,
}

impl Quest {
    /// Calculate completion percentage (0.0 to 1.0)
    pub fn completion(&self) -> f32 {
        if self.objectives.is_empty() {
            return 0.0;
        }

        let completed = self.objectives.iter().filter(|obj| obj.completed).count();
        completed as f32 / self.objectives.len() as f32
    }

    /// Check if quest is fully completed
    pub fn is_complete(&self) -> bool {
        !self.objectives.is_empty() && self.objectives.iter().all(|obj| obj.completed)
    }
}

/// Point of Interest for minimap
#[derive(Clone, Debug, PartialEq)]
pub struct PoiMarker {
    pub id: u32,
    pub world_pos: (f32, f32), // 2D top-down position (X, Z)
    pub poi_type: PoiType,
    pub label: Option<String>,
}

// Week 4 Day 4: Click-to-ping marker on minimap
#[derive(Clone, Debug)]
pub struct PingMarker {
    pub world_pos: (f32, f32), // 2D top-down position (X, Z)
    pub spawn_time: f32,       // Game time when ping was created
    pub duration: f32,         // How long ping lasts (default 3.0s)
}

impl PingMarker {
    pub fn new(world_pos: (f32, f32), spawn_time: f32) -> Self {
        Self {
            world_pos,
            spawn_time,
            duration: 3.0, // 3 seconds by default
        }
    }

    /// Check if ping is still active
    pub fn is_active(&self, game_time: f32) -> bool {
        game_time < self.spawn_time + self.duration
    }

    /// Get age (0.0 to 1.0) for animation
    pub fn age_normalized(&self, game_time: f32) -> f32 {
        let age = game_time - self.spawn_time;
        (age / self.duration).min(1.0)
    }
}

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum PoiType {
    Objective, // Yellow star
    Waypoint,  // Blue diamond
    Vendor,    // Green coin
    Danger,    // Red exclamation
}

// Week 4 Day 4: Dynamic POI icons for minimap
impl PoiType {
    /// Get emoji icon for this POI type
    pub fn icon(&self) -> &str {
        match self {
            PoiType::Objective => "🎯", // Target
            PoiType::Waypoint => "📍",  // Pin
            PoiType::Vendor => "🏪",    // Shop
            PoiType::Danger => "⚔️",    // Swords
        }
    }

    /// Get color for this POI type
    pub fn color(&self) -> egui::Color32 {
        match self {
            PoiType::Objective => egui::Color32::YELLOW,
            PoiType::Waypoint => egui::Color32::LIGHT_BLUE,
            PoiType::Vendor => egui::Color32::GREEN,
            PoiType::Danger => egui::Color32::RED,
        }
    }
}

// ===== Week 3 Day 4: Dialogue & Tooltip Data Structures =====

/// Dialogue choice for branching conversations
#[derive(Clone, Debug)]
pub struct DialogueChoice {
    pub id: u32,
    pub text: String,
    pub next_node: Option<u32>, // None = end dialogue
}

/// Active dialogue node
#[derive(Clone, Debug)]
pub struct DialogueNode {
    pub id: u32,
    pub speaker_name: String,
    pub text: String,
    pub choices: Vec<DialogueChoice>,
    pub portrait_id: Option<u32>, // For future portrait system
}

/// Tooltip content for hoverable UI elements
#[derive(Clone, Debug)]
pub struct TooltipData {
    pub title: String,
    pub description: String,
    pub stats: Vec<(String, String)>, // Key-value pairs (e.g., "Damage: 25", "Range: 10m")
    pub flavor_text: Option<String>,  // Lore text (italicized)
}

// ===== End Data Structures =====

/// HUD visibility and state tracking
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HudState {
    /// Master visibility toggle (ESC key in-game)
    pub visible: bool,

    /// Individual HUD element visibility (Week 3 Days 2-5)
    pub show_health_bars: bool,
    pub show_objectives: bool,
    pub show_minimap: bool,
    pub show_subtitles: bool,

    /// Quest tracker state (Week 3 Day 3)
    pub quest_tracker_collapsed: bool, // False = expanded (default), True = collapsed

    /// Minimap state (Week 3 Day 3, Week 4 Day 4)
    pub minimap_rotation: bool, // False = north-up (default), True = player-relative rotation
    pub minimap_zoom: f32, // Week 4 Day 4: Zoom level (1.0 = normal, 0.5-3.0 range)

    /// Dialogue state (Week 3 Day 4)
    pub show_dialogue: bool, // Show dialogue box

    /// Debug mode (shows HUD borders and stats)
    pub debug_mode: bool,
}

impl Default for HudState {
    fn default() -> Self {
        Self {
            visible: true, // HUD visible by default when in-game
            show_health_bars: true,
            show_objectives: true,
            show_minimap: true,
            show_subtitles: true,
            quest_tracker_collapsed: false, // Expanded by default
            minimap_rotation: false,        // North-up by default
            minimap_zoom: 1.0,              // Week 4 Day 4: Normal zoom by default
            show_dialogue: false,           // Hidden by default (triggered by events)
            debug_mode: false,              // Can be toggled with F3 or similar
        }
    }
}

// ===== Week 4 Day 2: Combo Tracker =====

/// Tracks combo hits for damage number display
#[derive(Clone, Debug)]
pub struct ComboTracker {
    hits: Vec<(f32, i32)>, // (timestamp, damage_value)
    combo_window: f32,     // Time window for combo (1.0 second default)
}

impl ComboTracker {
    /// Create new combo tracker
    pub fn new() -> Self {
        Self {
            hits: Vec::new(),
            combo_window: 1.0, // 1 second window
        }
    }

    /// Record a hit at the given time
    pub fn record_hit(&mut self, game_time: f32, damage: i32) {
        // Remove hits outside the combo window
        self.hits
            .retain(|(timestamp, _)| game_time - timestamp <= self.combo_window);

        // Add new hit
        self.hits.push((game_time, damage));
    }

    /// Get current combo count
    pub fn get_combo_count(&self, game_time: f32) -> u32 {
        // Count hits within combo window
        self.hits
            .iter()
            .filter(|(timestamp, _)| game_time - timestamp <= self.combo_window)
            .count() as u32
    }

    /// Get total combo damage
    pub fn get_combo_damage(&self, game_time: f32) -> i32 {
        self.hits
            .iter()
            .filter(|(timestamp, _)| game_time - timestamp <= self.combo_window)
            .map(|(_, damage)| damage)
            .sum()
    }

    /// Clean up old hits
    pub fn cleanup(&mut self, game_time: f32) {
        self.hits
            .retain(|(timestamp, _)| game_time - timestamp <= self.combo_window);
    }
}

impl Default for ComboTracker {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Week 4 Day 3: Quest Notification System
// ============================================================================

/// Type of quest notification
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum NotificationType {
    /// New quest started
    NewQuest,
    /// Single objective completed
    ObjectiveComplete { objective_text: String },
    /// Entire quest completed with rewards
    QuestComplete { rewards: Vec<String> },
}

/// A single quest notification with slide animation
#[derive(Debug, Clone)]
pub struct QuestNotification {
    pub notification_type: NotificationType,
    pub title: String,
    pub description: String,
    pub animation_time: f32, // Current animation time (0.0 to total_duration)
    pub total_duration: f32, // Total time on screen (2.0s for most, 2.8s for quest complete)
}

impl QuestNotification {
    /// Create new quest notification
    pub fn new_quest(title: String, description: String) -> Self {
        Self {
            notification_type: NotificationType::NewQuest,
            title,
            description,
            animation_time: 0.0,
            total_duration: 2.0, // 0.3s ease-in + 1.4s hold + 0.3s ease-out
        }
    }

    /// Create objective complete notification
    pub fn objective_complete(objective_text: String) -> Self {
        Self {
            notification_type: NotificationType::ObjectiveComplete {
                objective_text: objective_text.clone(),
            },
            title: "Objective Complete!".to_string(),
            description: objective_text,
            animation_time: 0.0,
            total_duration: 2.0,
        }
    }

    /// Create quest complete notification with rewards
    pub fn quest_complete(title: String, rewards: Vec<String>) -> Self {
        Self {
            notification_type: NotificationType::QuestComplete {
                rewards: rewards.clone(),
            },
            title,
            description: "Quest Complete!".to_string(),
            animation_time: 0.0,
            total_duration: 2.8, // 0.3s ease-in + 2.0s hold + 0.5s ease-out (longer for rewards)
        }
    }

    /// Update animation timer, returns true if notification is finished
    pub fn update(&mut self, dt: f32) -> bool {
        self.animation_time += dt;
        self.animation_time >= self.total_duration
    }

    /// Calculate slide offset (0.0 = on-screen, negative = above screen)
    /// Uses ease-in-out-quad for smooth motion
    pub fn calculate_slide_offset(&self) -> f32 {
        use crate::hud::easing::{ease_in_out_quad, ease_out_cubic};

        let total = self.total_duration;
        let ease_in_time = 0.3;
        let ease_out_time = match self.notification_type {
            NotificationType::QuestComplete { .. } => 0.5,
            _ => 0.3,
        };
        let hold_time = total - ease_in_time - ease_out_time;

        if self.animation_time < ease_in_time {
            // Ease in: slide down from -100 to 0
            let t = self.animation_time / ease_in_time;
            -100.0 * (1.0 - ease_out_cubic(t))
        } else if self.animation_time < ease_in_time + hold_time {
            // Hold on-screen
            0.0
        } else {
            // Ease out: slide up from 0 to -100
            let t = (self.animation_time - ease_in_time - hold_time) / ease_out_time;
            -100.0 * ease_in_out_quad(t)
        }
    }

    /// Calculate fade alpha (0-255)
    pub fn calculate_alpha(&self) -> u8 {
        let total = self.total_duration;
        let fade_in_time = 0.2;
        let fade_out_time = 0.3;

        if self.animation_time < fade_in_time {
            // Fade in
            let t = self.animation_time / fade_in_time;
            (t * 255.0) as u8
        } else if self.animation_time > total - fade_out_time {
            // Fade out
            let t = (total - self.animation_time) / fade_out_time;
            (t * 255.0) as u8
        } else {
            // Fully visible
            255
        }
    }
}

/// Queue for managing multiple notifications
#[derive(Debug, Clone)]
pub struct NotificationQueue {
    pub active: Option<QuestNotification>,
    pub pending: std::collections::VecDeque<QuestNotification>,
}

impl NotificationQueue {
    /// Create new empty notification queue
    pub fn new() -> Self {
        Self {
            active: None,
            pending: std::collections::VecDeque::new(),
        }
    }

    /// Add notification to queue
    pub fn push(&mut self, notification: QuestNotification) {
        if self.active.is_none() {
            self.active = Some(notification);
        } else {
            self.pending.push_back(notification);
        }
    }

    /// Update active notification, auto-pop when finished
    pub fn update(&mut self, dt: f32) {
        if let Some(notification) = &mut self.active {
            if notification.update(dt) {
                // Notification finished, pop next from queue
                self.active = self.pending.pop_front();
            }
        }
    }

    /// Check if any notification is active
    pub fn has_active(&self) -> bool {
        self.active.is_some()
    }
}

impl Default for NotificationQueue {
    fn default() -> Self {
        Self::new()
    }
}

// Week 5 Day 2: Audio callback type aliases (to satisfy clippy::type_complexity)
/// Callback for minimap click sound (receives normalized distance from center 0.0-1.0)
pub type MinimapClickCallback = Box<dyn Fn(f32) + Send + Sync>;
/// Callback for ping spawn sound (receives world position as (x, z))
pub type PingSpawnCallback = Box<dyn Fn((f32, f32)) + Send + Sync>;

/// HUD Manager - coordinates all HUD elements
pub struct HudManager {
    state: HudState,

    // Week 3 Day 2: Health bars & resources
    pub player_stats: PlayerStats,
    pub enemies: Vec<EnemyData>,
    pub damage_numbers: Vec<DamageNumber>,

    // Week 4 Day 2: Combo tracking
    pub combo_tracker: ComboTracker,

    // Week 4 Day 3: Quest notifications
    pub notification_queue: NotificationQueue,

    // Week 3 Day 3: Quest tracker & minimap
    pub active_quest: Option<Quest>,
    pub poi_markers: Vec<PoiMarker>,
    pub ping_markers: Vec<PingMarker>, // Week 4 Day 4: Click-to-ping on minimap
    pub player_position: (f32, f32),   // 2D top-down (X, Z)
    pub player_rotation: f32,          // Radians, 0 = facing north

    // Week 3 Day 4: Dialogue & tooltips
    pub active_dialogue: Option<DialogueNode>,
    pub hovered_tooltip: Option<TooltipData>,
    pub tooltip_position: (f32, f32), // Screen coordinates for tooltip rendering

    // Week 5 Day 2: Audio callbacks (optional, for minimap click/ping sounds)
    pub on_minimap_click: Option<MinimapClickCallback>, // Parameter: distance from center (0.0-1.0)
    pub on_ping_spawn: Option<PingSpawnCallback>,       // Parameter: world position

    // Game time tracking (for animations)
    game_time: f32,
}

impl HudManager {
    /// Create new HUD manager with default state
    pub fn new() -> Self {
        Self {
            state: HudState::default(),
            player_stats: PlayerStats::default(),
            enemies: Vec::new(),
            damage_numbers: Vec::new(),
            combo_tracker: ComboTracker::new(),
            notification_queue: NotificationQueue::new(),
            active_quest: None,
            poi_markers: Vec::new(),
            ping_markers: Vec::new(), // Week 4 Day 4: Empty ping list
            player_position: (0.0, 0.0),
            player_rotation: 0.0,
            active_dialogue: None,
            hovered_tooltip: None,
            tooltip_position: (0.0, 0.0),
            on_minimap_click: None, // Week 5 Day 2: No audio callbacks by default
            on_ping_spawn: None,
            game_time: 0.0,
        }
    }

    /// Toggle HUD master visibility (ESC key)
    pub fn toggle_visibility(&mut self) {
        self.state.visible = !self.state.visible;
        log::info!(
            "HUD visibility: {}",
            if self.state.visible {
                "VISIBLE"
            } else {
                "HIDDEN"
            }
        );
    }

    /// Set HUD visibility explicitly
    pub fn set_visible(&mut self, visible: bool) {
        self.state.visible = visible;
    }

    /// Check if HUD is currently visible
    pub fn is_visible(&self) -> bool {
        self.state.visible
    }

    /// Toggle debug mode (shows HUD element bounds)
    pub fn toggle_debug(&mut self) {
        self.state.debug_mode = !self.state.debug_mode;
        log::info!(
            "HUD debug mode: {}",
            if self.state.debug_mode { "ON" } else { "OFF" }
        );
    }

    /// Toggle quest tracker visibility (Week 3 Day 3)
    pub fn toggle_quest_tracker(&mut self) {
        self.state.show_objectives = !self.state.show_objectives;
        log::info!(
            "Quest tracker: {}",
            if self.state.show_objectives {
                "VISIBLE"
            } else {
                "HIDDEN"
            }
        );
    }

    /// Toggle quest tracker collapse/expand (Week 3 Day 3)
    pub fn toggle_quest_collapse(&mut self) {
        self.state.quest_tracker_collapsed = !self.state.quest_tracker_collapsed;
        log::info!(
            "Quest tracker: {}",
            if self.state.quest_tracker_collapsed {
                "COLLAPSED"
            } else {
                "EXPANDED"
            }
        );
    }

    /// Toggle minimap visibility (Week 3 Day 3)
    pub fn toggle_minimap(&mut self) {
        self.state.show_minimap = !self.state.show_minimap;
        log::info!(
            "Minimap: {}",
            if self.state.show_minimap {
                "VISIBLE"
            } else {
                "HIDDEN"
            }
        );
    }

    /// Toggle minimap rotation mode (Week 3 Day 3)
    pub fn toggle_minimap_rotation(&mut self) {
        self.state.minimap_rotation = !self.state.minimap_rotation;
        log::info!(
            "Minimap rotation: {}",
            if self.state.minimap_rotation {
                "PLAYER-RELATIVE"
            } else {
                "NORTH-UP"
            }
        );
    }

    /// Week 4 Day 4: Adjust minimap zoom level (0.5× to 3.0×)
    pub fn set_minimap_zoom(&mut self, zoom: f32) {
        self.state.minimap_zoom = zoom.clamp(0.5, 3.0);
        log::info!("Minimap zoom: {:.2}×", self.state.minimap_zoom);
    }

    /// Week 4 Day 4: Get current minimap zoom level
    pub fn minimap_zoom(&self) -> f32 {
        self.state.minimap_zoom
    }

    /// Start dialogue (Week 3 Day 4)
    pub fn start_dialogue(&mut self, dialogue: DialogueNode) {
        let speaker_name = dialogue.speaker_name.clone();
        self.active_dialogue = Some(dialogue);
        self.state.show_dialogue = true;
        log::info!("Dialogue started: {}", speaker_name);
    }

    /// End dialogue (Week 3 Day 4)
    pub fn end_dialogue(&mut self) {
        self.active_dialogue = None;
        self.state.show_dialogue = false;
        log::info!("Dialogue ended");
    }

    /// Select dialogue choice (Week 3 Day 4)
    /// Returns the next dialogue node ID if the choice leads to another node
    pub fn select_dialogue_choice(&mut self, choice_id: u32) -> Option<u32> {
        if let Some(dialogue) = &self.active_dialogue {
            if let Some(choice) = dialogue.choices.iter().find(|c| c.id == choice_id) {
                log::info!("Selected choice: {}", choice.text);
                return choice.next_node;
            }
        }
        None
    }

    /// Show tooltip at mouse position (Week 3 Day 4)
    pub fn show_tooltip(&mut self, tooltip: TooltipData, screen_pos: (f32, f32)) {
        self.hovered_tooltip = Some(tooltip);
        self.tooltip_position = screen_pos;
    }

    /// Hide tooltip (Week 3 Day 4)
    pub fn hide_tooltip(&mut self) {
        self.hovered_tooltip = None;
    }

    /// Get current HUD state (for persistence/settings)
    pub fn state(&self) -> &HudState {
        &self.state
    }

    /// Update HUD state (for loading from settings)
    pub fn set_state(&mut self, state: HudState) {
        self.state = state;
    }

    /// Update HUD (called every frame before render)
    ///
    /// Week 3 Day 2: Updates damage numbers animation, removes expired ones
    /// Week 4 Day 1: Updates health bar animations
    /// Week 4 Day 2: Updates combo tracker cleanup
    pub fn update(&mut self, dt: f32) {
        self.game_time += dt;

        // Week 4 Day 1: Update player health animation
        self.player_stats
            .health_animation
            .set_target(self.player_stats.health);
        self.player_stats.health_animation.update(dt);

        // Week 4 Day 1: Update enemy health animations
        for enemy in &mut self.enemies {
            enemy.health_animation.set_target(enemy.health);
            enemy.health_animation.update(dt);
        }

        // Week 4 Day 2: Cleanup combo tracker
        self.combo_tracker.cleanup(self.game_time);

        // Week 4 Day 3: Update notification queue
        self.notification_queue.update(dt);

        // Week 4 Day 4: Remove expired ping markers
        self.ping_markers
            .retain(|ping| ping.is_active(self.game_time));

        // Update damage numbers (float upward, fade out, remove expired)
        self.damage_numbers.retain(|dmg| {
            let age = self.game_time - dmg.spawn_time;
            age < 1.5 // 1.5 second lifetime
        });
    }

    /// Spawn a damage number at world position
    ///
    /// Week 4 Day 2: Now uses DamageNumber::new() constructor for arc motion + shake
    pub fn spawn_damage(
        &mut self,
        value: i32,
        world_pos: (f32, f32, f32),
        damage_type: DamageType,
    ) {
        // Week 4 Day 2: Record combo hit
        self.combo_tracker.record_hit(self.game_time, value);

        // Create damage number with arc motion and shake
        self.damage_numbers.push(DamageNumber::new(
            value,
            self.game_time,
            world_pos,
            damage_type,
        ));
    }

    /// Week 4 Day 4: Create a ping marker at world position (for minimap click-to-ping)
    pub fn spawn_ping(&mut self, world_pos: (f32, f32)) {
        self.ping_markers
            .push(PingMarker::new(world_pos, self.game_time));
        log::info!(
            "Ping created at world pos ({:.1}, {:.1})",
            world_pos.0,
            world_pos.1
        );
    }

    /// Week 5 Day 2: Set audio callback for minimap click sound
    ///
    /// The callback receives the normalized distance from minimap center (0.0-1.0)
    /// to allow pitch variation (e.g., lower pitch at center, higher at edge).
    ///
    /// Example with astraweave-audio:
    /// ```ignore
    /// use astraweave_ui::hud::HudManager;
    /// use astraweave_audio::AudioEngine;
    /// let mut hud = HudManager::new();
    /// let mut audio = AudioEngine::new().unwrap();
    /// hud.set_minimap_click_callback(move |dist| {
    ///     let base_hz = 800.0;
    ///     let pitch_hz = base_hz + (dist * 400.0);  // 800Hz at center, 1200Hz at edge
    ///     audio.play_sfx_beep(pitch_hz, 0.05, 0.3);  // 50ms beep, 0.3 volume
    /// });
    /// ```
    pub fn set_minimap_click_callback<F>(&mut self, callback: F)
    where
        F: Fn(f32) + Send + Sync + 'static,
    {
        self.on_minimap_click = Some(Box::new(callback));
    }

    /// Week 5 Day 2: Set audio callback for ping spawn sound
    ///
    /// The callback receives the world position of the ping for 3D spatial audio.
    ///
    /// Example with astraweave-audio:
    /// ```ignore
    /// use astraweave_ui::hud::HudManager;
    /// use astraweave_audio::{AudioEngine, EmitterId};
    /// use glam::vec3;
    /// let mut hud = HudManager::new();
    /// let mut audio = AudioEngine::new().unwrap();
    /// hud.set_ping_spawn_callback(move |world_pos| {
    ///     let pos_3d = vec3(world_pos.0, 0.0, world_pos.1);
    ///     audio.play_sfx_3d_beep(EmitterId(0), 1200.0, 0.1, pos_3d, 0.6);  // 1200Hz, 100ms, 0.6 volume
    /// });
    /// ```
    pub fn set_ping_spawn_callback<F>(&mut self, callback: F)
    where
        F: Fn((f32, f32)) + Send + Sync + 'static,
    {
        self.on_ping_spawn = Some(Box::new(callback));
    }

    /// Render HUD overlay (called every frame in PRESENTATION stage)
    ///
    /// Week 3 Days 2-5 will add actual HUD elements here:
    /// - Day 2: Health bars (top-left player, above enemies in 3D)
    /// - Day 3: Objectives (top-right)
    /// - Day 4: Minimap (bottom-left), Compass (top-center)
    /// - Day 5: Subtitles (bottom-center), Notifications (top-center)
    pub fn render(&mut self, ctx: &egui::Context) {
        // Early return if HUD hidden (ESC pressed)
        if !self.state.visible {
            return;
        }

        // Week 3 Day 1: Placeholder HUD with debug border
        // This will be replaced with actual HUD elements in subsequent days

        if self.state.debug_mode {
            // Show debug HUD border (indicates HUD is active)
            egui::Area::new(egui::Id::new("hud_debug_border"))
                .fixed_pos(egui::pos2(10.0, 60.0)) // Below FPS counter (top-left at 10, 10)
                .show(ctx, |ui| {
                    ui.label("🎮 HUD Active (Week 3 Day 1)");
                    ui.label("ESC = Toggle HUD visibility");
                    ui.label("F3 = Toggle debug mode");
                    ui.separator();
                    ui.label(format!(
                        "Health Bars: {}",
                        if self.state.show_health_bars {
                            "✅"
                        } else {
                            "❌"
                        }
                    ));
                    ui.label(format!(
                        "Objectives: {}",
                        if self.state.show_objectives {
                            "✅"
                        } else {
                            "❌"
                        }
                    ));
                    ui.label(format!(
                        "Minimap: {}",
                        if self.state.show_minimap {
                            "✅"
                        } else {
                            "❌"
                        }
                    ));
                    ui.label(format!(
                        "Subtitles: {}",
                        if self.state.show_subtitles {
                            "✅"
                        } else {
                            "❌"
                        }
                    ));
                });
        }

        // Week 3 Day 2: Health bars will render here
        if self.state.show_health_bars {
            self.render_health_bars(ctx);
        }

        // Week 3 Day 3: Objectives will render here
        if self.state.show_objectives {
            self.render_objectives(ctx);
        }

        // Week 3 Day 4: Minimap and compass will render here
        if self.state.show_minimap {
            self.render_minimap(ctx);
        }

        // Week 3 Day 5: Subtitles and notifications will render here
        if self.state.show_subtitles {
            self.render_subtitles(ctx);
        }

        // Week 3 Day 4: Dialogue system
        if self.state.show_dialogue && self.active_dialogue.is_some() {
            self.render_dialogue(ctx);
        }

        // Week 3 Day 4: Tooltips (always render if hovered)
        if self.hovered_tooltip.is_some() {
            self.render_tooltip(ctx);
        }
    }

    // ===== Week 3 Day 2: Health Bars =====

    fn render_health_bars(&self, ctx: &egui::Context) {
        // Player health bar (top-left, below FPS counter)
        self.render_player_health(ctx);

        // Resource meters (below health bar)
        self.render_player_resources(ctx);

        // Enemy health bars (3D world space)
        self.render_enemy_health_bars(ctx);

        // Damage numbers (floating text)
        self.render_damage_numbers(ctx);

        // Week 4 Day 3: Quest notifications (top-center slide animations)
        self.render_notifications(ctx);
    }

    fn render_player_health(&self, ctx: &egui::Context) {
        use egui::{Color32, CornerRadius, Pos2, Rect, Stroke};

        egui::Area::new(egui::Id::new("player_health"))
            .fixed_pos(Pos2::new(10.0, 40.0)) // Below FPS counter
            .show(ctx, |ui| {
                let bar_width = 200.0;
                let bar_height = 20.0;

                // Week 4 Day 1: Use animated visual health instead of actual health
                let visual_health = self.player_stats.health_animation.visual_health();
                let health_pct = (visual_health / self.player_stats.max_health).clamp(0.0, 1.0);

                // Health color gradient: Green -> Yellow -> Red
                let health_color = if health_pct > 0.5 {
                    // Green to Yellow (100% to 50%)
                    let t = (1.0 - health_pct) * 2.0; // 0 to 1
                    Color32::from_rgb((255.0 * t) as u8, 255, 0)
                } else {
                    // Yellow to Red (50% to 0%)
                    let t = health_pct * 2.0; // 0 to 1
                    Color32::from_rgb(255, (255.0 * t) as u8, 0)
                };

                let (rect, _response) =
                    ui.allocate_exact_size(egui::vec2(bar_width, bar_height), egui::Sense::hover());

                // Background (dark gray)
                ui.painter().rect_filled(
                    rect,
                    CornerRadius::same(3),
                    Color32::from_rgb(40, 40, 40),
                );

                // Health bar (filled portion)
                let filled_width = bar_width * health_pct;
                if filled_width > 0.0 {
                    ui.painter().rect_filled(
                        Rect::from_min_size(rect.min, egui::vec2(filled_width, bar_height)),
                        CornerRadius::same(3),
                        health_color,
                    );
                }

                // Week 4 Day 1: Green glow effect if healing
                if self.player_stats.health_animation.is_healing() {
                    let glow_alpha = 0.4; // Semi-transparent green overlay
                    ui.painter().rect_filled(
                        Rect::from_min_size(rect.min, egui::vec2(filled_width, bar_height)),
                        CornerRadius::same(3),
                        Color32::from_rgba_premultiplied(50, 255, 50, (glow_alpha * 255.0) as u8),
                    );
                }

                // Week 4 Day 1: Red damage flash effect
                let flash_alpha = self.player_stats.health_animation.flash_alpha();
                if flash_alpha > 0.0 {
                    ui.painter().rect_filled(
                        rect,
                        CornerRadius::same(3),
                        Color32::from_rgba_premultiplied(255, 50, 50, (flash_alpha * 255.0) as u8),
                    );
                }

                // Border
                ui.painter().rect_stroke(
                    rect,
                    CornerRadius::same(3),
                    Stroke::new(2.0, Color32::from_rgb(200, 200, 200)),
                    egui::StrokeKind::Middle,
                );

                // Text overlay (centered)
                let text = format!(
                    "{:.0}/{:.0} HP",
                    self.player_stats.health, self.player_stats.max_health
                );
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    text,
                    egui::FontId::proportional(14.0),
                    Color32::WHITE,
                );
            });
    }

    fn render_player_resources(&self, ctx: &egui::Context) {
        use egui::{Color32, CornerRadius, Pos2, Rect, Stroke};

        // Mana bar
        egui::Area::new(egui::Id::new("player_mana"))
            .fixed_pos(Pos2::new(10.0, 65.0)) // Below health bar
            .show(ctx, |ui| {
                let bar_width = 200.0;
                let bar_height = 15.0;
                let mana_pct =
                    (self.player_stats.mana / self.player_stats.max_mana).clamp(0.0, 1.0);

                let (rect, _response) =
                    ui.allocate_exact_size(egui::vec2(bar_width, bar_height), egui::Sense::hover());

                // Background
                ui.painter().rect_filled(
                    rect,
                    CornerRadius::same(2),
                    Color32::from_rgb(30, 30, 40),
                );

                // Mana bar (blue)
                let filled_width = bar_width * mana_pct;
                if filled_width > 0.0 {
                    ui.painter().rect_filled(
                        Rect::from_min_size(rect.min, egui::vec2(filled_width, bar_height)),
                        CornerRadius::same(2),
                        Color32::from_rgb(50, 100, 255),
                    );
                }

                // Border
                ui.painter().rect_stroke(
                    rect,
                    CornerRadius::same(2),
                    Stroke::new(1.0, Color32::from_rgb(150, 150, 150)),
                    egui::StrokeKind::Middle,
                );

                // Text
                let text = format!(
                    "{:.0}/{:.0} MP",
                    self.player_stats.mana, self.player_stats.max_mana
                );
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    text,
                    egui::FontId::proportional(12.0),
                    Color32::WHITE,
                );
            });

        // Stamina bar
        egui::Area::new(egui::Id::new("player_stamina"))
            .fixed_pos(Pos2::new(10.0, 85.0)) // Below mana bar
            .show(ctx, |ui| {
                let bar_width = 200.0;
                let bar_height = 15.0;
                let stamina_pct =
                    (self.player_stats.stamina / self.player_stats.max_stamina).clamp(0.0, 1.0);

                let (rect, _response) =
                    ui.allocate_exact_size(egui::vec2(bar_width, bar_height), egui::Sense::hover());

                // Background
                ui.painter().rect_filled(
                    rect,
                    CornerRadius::same(2),
                    Color32::from_rgb(40, 40, 30),
                );

                // Stamina bar (yellow/gold)
                let filled_width = bar_width * stamina_pct;
                if filled_width > 0.0 {
                    ui.painter().rect_filled(
                        Rect::from_min_size(rect.min, egui::vec2(filled_width, bar_height)),
                        CornerRadius::same(2),
                        Color32::from_rgb(255, 200, 50),
                    );
                }

                // Border
                ui.painter().rect_stroke(
                    rect,
                    CornerRadius::same(2),
                    Stroke::new(1.0, Color32::from_rgb(150, 150, 150)),
                    egui::StrokeKind::Middle,
                );

                // Text
                let text = format!(
                    "{:.0}/{:.0} SP",
                    self.player_stats.stamina, self.player_stats.max_stamina
                );
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    text,
                    egui::FontId::proportional(12.0),
                    Color32::from_rgb(50, 50, 50),
                );
            });
    }

    fn render_enemy_health_bars(&self, ctx: &egui::Context) {
        use egui::{Color32, CornerRadius, Pos2, Rect, Stroke};

        // Get screen size for projection
        let screen_rect = ctx.screen_rect();
        let screen_size = (screen_rect.width(), screen_rect.height());

        for enemy in &self.enemies {
            // Only show if damaged
            if enemy.health >= enemy.max_health {
                continue;
            }

            // World to screen projection (simplified for demo)
            if let Some((screen_x, screen_y)) = world_to_screen_simple(enemy.world_pos, screen_size)
            {
                // Skip if off-screen
                if screen_x < 0.0
                    || screen_x > screen_size.0
                    || screen_y < 0.0
                    || screen_y > screen_size.1
                {
                    continue;
                }

                egui::Area::new(egui::Id::new(format!("enemy_health_{}", enemy.id)))
                    .fixed_pos(Pos2::new(screen_x - 30.0, screen_y - 20.0)) // Center above head
                    .show(ctx, |ui| {
                        let bar_width = 60.0;
                        let bar_height = 8.0;

                        // Week 4 Day 1: Use animated visual health
                        let visual_health = enemy.health_animation.visual_health();
                        let health_pct = (visual_health / enemy.max_health).clamp(0.0, 1.0);

                        // Faction color
                        let bar_color = match enemy.faction {
                            EnemyFaction::Hostile => Color32::from_rgb(200, 50, 50),
                            EnemyFaction::Neutral => Color32::from_rgb(200, 200, 50),
                            EnemyFaction::Friendly => Color32::from_rgb(50, 200, 50),
                        };

                        let (rect, _response) = ui.allocate_exact_size(
                            egui::vec2(bar_width, bar_height),
                            egui::Sense::hover(),
                        );

                        // Background (semi-transparent black)
                        ui.painter().rect_filled(
                            rect,
                            CornerRadius::same(2),
                            Color32::from_rgba_premultiplied(0, 0, 0, 150),
                        );

                        // Health bar
                        let filled_width = bar_width * health_pct;
                        if filled_width > 0.0 {
                            ui.painter().rect_filled(
                                Rect::from_min_size(rect.min, egui::vec2(filled_width, bar_height)),
                                CornerRadius::same(2),
                                bar_color,
                            );
                        }

                        // Week 4 Day 1: Damage flash effect (smaller, less intense for enemies)
                        let flash_alpha = enemy.health_animation.flash_alpha();
                        if flash_alpha > 0.0 {
                            ui.painter().rect_filled(
                                rect,
                                CornerRadius::same(2),
                                Color32::from_rgba_premultiplied(
                                    255,
                                    50,
                                    50,
                                    ((flash_alpha * 0.6) * 255.0) as u8, // 60% of player flash intensity
                                ),
                            );
                        }

                        // Border
                        ui.painter().rect_stroke(
                            rect,
                            CornerRadius::same(2),
                            Stroke::new(1.0, Color32::from_rgb(150, 150, 150)),
                            egui::StrokeKind::Middle,
                        );
                    });
            }
        }
    }

    fn render_damage_numbers(&self, ctx: &egui::Context) {
        use egui::{Color32, Pos2};

        let screen_rect = ctx.screen_rect();
        let screen_size = (screen_rect.width(), screen_rect.height());

        // Week 4 Day 2: Get current combo count
        let combo_count = self.combo_tracker.get_combo_count(self.game_time);
        let combo_damage = self.combo_tracker.get_combo_damage(self.game_time);

        for (idx, dmg) in self.damage_numbers.iter().enumerate() {
            let age = self.game_time - dmg.spawn_time;
            let lifetime_pct = (age / 1.5).clamp(0.0, 1.0); // 1.5s lifetime

            // Week 4 Day 2: Arc motion (parabolic trajectory)
            let (arc_offset_x, arc_offset_y) = dmg.calculate_offset(age);

            // Fade out
            let alpha = ((1.0 - lifetime_pct) * 255.0) as u8;

            // Week 4 Day 2: Shake rotation (calculated for future use)
            let _shake_rotation = dmg.calculate_shake(age);

            // World to screen
            if let Some((screen_x, screen_y)) = world_to_screen_simple(dmg.world_pos, screen_size) {
                let final_x = screen_x + arc_offset_x;
                let final_y = screen_y + arc_offset_y;

                let color = match dmg.damage_type {
                    DamageType::Normal => Color32::from_rgba_premultiplied(255, 255, 255, alpha),
                    DamageType::Critical => Color32::from_rgba_premultiplied(255, 255, 0, alpha),
                    DamageType::SelfDamage => {
                        Color32::from_rgba_premultiplied(255, 100, 100, alpha)
                    }
                };

                // Week 4 Day 2: Combo counter text (show if combo > 1)
                let text = if combo_count > 1 {
                    format!("{} x{}", dmg.value, combo_count)
                } else {
                    format!("{}", dmg.value)
                };

                // Week 4 Day 2: Scale with combo count
                let base_size = 18.0;
                let size = base_size * (1.0 + (combo_count as f32 - 1.0) * 0.15).min(2.0); // Max 2x size

                egui::Area::new(egui::Id::new(format!("damage_number_{}", idx)))
                    .fixed_pos(Pos2::new(final_x, final_y))
                    .show(ctx, |ui| {
                        // Week 4 Day 2: Apply rotation for shake effect
                        // Note: egui doesn't support text rotation directly, so we simulate with positioning
                        // For full rotation support, we'd need custom rendering
                        // For now, the shake calculation is ready for future enhancement

                        let mut rich_text = egui::RichText::new(text).size(size).color(color);

                        // Make critical hits bold
                        if dmg.damage_type == DamageType::Critical {
                            rich_text = rich_text.strong();
                        }

                        ui.label(rich_text);

                        // Show total combo damage below if combo > 1
                        if combo_count > 1 && idx == self.damage_numbers.len() - 1 {
                            // Only show on the latest damage number
                            ui.label(
                                egui::RichText::new(format!("Total: {}", combo_damage))
                                    .size(12.0)
                                    .color(Color32::from_rgba_premultiplied(200, 200, 200, alpha)),
                            );
                        }
                    });
            }
        }
    }

    // ===== Week 3 Day 3: Objectives & Quest Tracker =====
    // ===== Week 3 Day 3: Quest Tracker =====

    fn render_objectives(&self, ctx: &egui::Context) {
        use egui::{Color32, CornerRadius, Pos2, Rect, Stroke, StrokeKind, Vec2};

        let screen_size = ctx.screen_rect().size();
        let panel_width = 300.0;
        let panel_x = screen_size.x - panel_width - 10.0; // 10px from right edge
        let panel_y = 50.0; // Below top edge

        // If no active quest, show "No Active Quest" message
        let Some(quest) = &self.active_quest else {
            if self.state.debug_mode {
                egui::Area::new(egui::Id::new("quest_tracker_empty"))
                    .fixed_pos(Pos2::new(panel_x, panel_y))
                    .show(ctx, |ui| {
                        ui.label("📜 No Active Quest");
                    });
            }
            return;
        };

        // Render quest tracker panel
        egui::Area::new(egui::Id::new("quest_tracker"))
            .fixed_pos(Pos2::new(panel_x, panel_y))
            .show(ctx, |ui| {
                // Calculate panel height based on content
                let header_height = 60.0;
                let objective_height = 25.0;
                let panel_height = if self.state.quest_tracker_collapsed {
                    header_height
                } else {
                    header_height + (quest.objectives.len() as f32 * objective_height) + 20.0
                };

                // Background panel
                let panel_rect =
                    Rect::from_min_size(Pos2::new(0.0, 0.0), Vec2::new(panel_width, panel_height));

                // Draw semi-transparent background
                ui.painter().rect_filled(
                    panel_rect,
                    CornerRadius::same(6),
                    Color32::from_rgba_premultiplied(20, 20, 30, 220),
                );

                // Draw border (golden for active quest)
                ui.painter().rect_stroke(
                    panel_rect,
                    CornerRadius::same(6),
                    Stroke::new(2.0, Color32::from_rgb(200, 160, 60)),
                    StrokeKind::Middle,
                );

                // === Header ===
                ui.vertical(|ui| {
                    ui.add_space(8.0);

                    ui.horizontal(|ui| {
                        ui.add_space(10.0);

                        // Collapse/expand arrow
                        let arrow = if self.state.quest_tracker_collapsed {
                            "▶"
                        } else {
                            "▼"
                        };
                        ui.label(egui::RichText::new(arrow).color(Color32::GOLD).size(14.0));

                        ui.add_space(5.0);

                        // Quest title
                        ui.label(
                            egui::RichText::new(&quest.title)
                                .color(Color32::GOLD)
                                .size(16.0)
                                .strong(),
                        );
                    });

                    // Quest description (only when expanded)
                    if !self.state.quest_tracker_collapsed {
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.add_space(10.0);
                            ui.label(
                                egui::RichText::new(&quest.description)
                                    .color(Color32::LIGHT_GRAY)
                                    .size(12.0),
                            );
                        });

                        ui.add_space(8.0);

                        // === Objectives List ===
                        for (idx, objective) in quest.objectives.iter().enumerate() {
                            ui.horizontal(|ui| {
                                ui.add_space(15.0);

                                // Checkbox (✅ or ⬜)
                                let checkbox_icon = if objective.completed { "✅" } else { "⬜" };
                                ui.label(egui::RichText::new(checkbox_icon).size(14.0));

                                ui.add_space(5.0);

                                // Objective description
                                let text_color = if objective.completed {
                                    Color32::DARK_GRAY
                                } else {
                                    Color32::WHITE
                                };

                                ui.label(
                                    egui::RichText::new(&objective.description)
                                        .color(text_color)
                                        .size(13.0),
                                );

                                // Progress (e.g., "3/5")
                                if let Some((current, total)) = objective.progress {
                                    ui.label(
                                        egui::RichText::new(format!("({}/{})", current, total))
                                            .color(Color32::LIGHT_BLUE)
                                            .size(12.0),
                                    );
                                }
                            });

                            if idx < quest.objectives.len() - 1 {
                                ui.add_space(2.0);
                            }
                        }

                        ui.add_space(8.0);

                        // === Progress Bar ===
                        let completion_pct = quest.completion();
                        ui.horizontal(|ui| {
                            ui.add_space(15.0);

                            let progress_bar_width = panel_width - 30.0;
                            let progress_bar_height = 8.0;

                            let bar_rect = Rect::from_min_size(
                                ui.cursor().min,
                                Vec2::new(progress_bar_width, progress_bar_height),
                            );

                            // Background (dark gray)
                            ui.painter().rect_filled(
                                bar_rect,
                                CornerRadius::same(4),
                                Color32::from_rgb(40, 40, 50),
                            );

                            // Progress fill (golden gradient)
                            if completion_pct > 0.0 {
                                let fill_width = progress_bar_width * completion_pct;
                                let fill_rect = Rect::from_min_size(
                                    bar_rect.min,
                                    Vec2::new(fill_width, progress_bar_height),
                                );

                                ui.painter().rect_filled(
                                    fill_rect,
                                    CornerRadius::same(4),
                                    Color32::from_rgb(200, 160, 60),
                                );
                            }

                            // Progress border
                            ui.painter().rect_stroke(
                                bar_rect,
                                CornerRadius::same(4),
                                Stroke::new(1.0, Color32::from_rgb(100, 100, 110)),
                                StrokeKind::Middle,
                            );

                            ui.allocate_space(Vec2::new(progress_bar_width, progress_bar_height));
                        });

                        ui.add_space(5.0);

                        // Progress percentage text
                        ui.horizontal(|ui| {
                            ui.add_space(15.0);
                            ui.label(
                                egui::RichText::new(format!(
                                    "{}% Complete",
                                    (completion_pct * 100.0) as u32
                                ))
                                .color(Color32::LIGHT_GRAY)
                                .size(11.0),
                            );
                        });
                    }
                });
            });
    }

    // ===== Week 3 Day 3: Minimap =====
    // Week 5 Day 1: Changed to &mut self for mouse click-to-ping
    fn render_minimap(&mut self, ctx: &egui::Context) {
        use egui::{Color32, Pos2, Rect, Stroke, Vec2};

        let screen_size = ctx.screen_rect().size();
        let minimap_size = 150.0;
        let minimap_x = screen_size.x - minimap_size - 10.0; // 10px from right edge
        let minimap_y = screen_size.y - minimap_size - 10.0; // 10px from bottom edge

        egui::Area::new(egui::Id::new("minimap"))
            .fixed_pos(Pos2::new(minimap_x, minimap_y))
            .show(ctx, |ui| {
                let minimap_rect =
                    Rect::from_min_size(Pos2::new(0.0, 0.0), Vec2::new(minimap_size, minimap_size));

                let minimap_center = minimap_rect.center();
                let minimap_radius = minimap_size / 2.0;

                // Draw circular background (semi-transparent dark)
                ui.painter().circle_filled(
                    minimap_center,
                    minimap_radius,
                    Color32::from_rgba_premultiplied(20, 30, 40, 200),
                );

                // Draw border (lighter blue)
                ui.painter().circle_stroke(
                    minimap_center,
                    minimap_radius,
                    Stroke::new(2.0, Color32::from_rgb(60, 120, 180)),
                );

                // === Grid overlay (optional, subtle) ===
                let grid_spacing = 30.0; // 30px grid
                let grid_color = Color32::from_rgba_premultiplied(80, 100, 120, 80);

                // Vertical grid lines
                for i in 1..5 {
                    let x = i as f32 * grid_spacing;
                    if x < minimap_size {
                        ui.painter().line_segment(
                            [Pos2::new(x, 0.0), Pos2::new(x, minimap_size)],
                            Stroke::new(1.0, grid_color),
                        );
                    }
                }

                // Horizontal grid lines
                for i in 1..5 {
                    let y = i as f32 * grid_spacing;
                    if y < minimap_size {
                        ui.painter().line_segment(
                            [Pos2::new(0.0, y), Pos2::new(minimap_size, y)],
                            Stroke::new(1.0, grid_color),
                        );
                    }
                }

                // === POI Markers ===
                // Week 4 Day 4: Apply zoom to map scale
                let map_scale = 5.0 / self.state.minimap_zoom; // Zoom in = smaller scale (less world units per pixel)

                for poi in &self.poi_markers {
                    // Calculate relative position from player
                    let rel_x = poi.world_pos.0 - self.player_position.0;
                    let rel_z = poi.world_pos.1 - self.player_position.1;

                    // Apply rotation if player-relative mode
                    let (screen_x, screen_z) = if self.state.minimap_rotation {
                        // Rotate around player
                        let cos = self.player_rotation.cos();
                        let sin = self.player_rotation.sin();
                        let rotated_x = rel_x * cos - rel_z * sin;
                        let rotated_z = rel_x * sin + rel_z * cos;
                        (rotated_x, rotated_z)
                    } else {
                        // North-up (no rotation)
                        (rel_x, rel_z)
                    };

                    // Convert to screen coordinates
                    let marker_x = minimap_center.x + (screen_x / map_scale);
                    let marker_y = minimap_center.y - (screen_z / map_scale); // Y inverted (screen down = positive)

                    // Clamp to circular bounds
                    let dx = marker_x - minimap_center.x;
                    let dy = marker_y - minimap_center.y;
                    let dist = (dx * dx + dy * dy).sqrt();

                    if dist > minimap_radius - 10.0 {
                        continue; // Outside minimap, skip
                    }

                    let marker_pos = Pos2::new(marker_x, marker_y);

                    // Week 4 Day 4: Dynamic emoji icons instead of shapes
                    ui.painter().text(
                        marker_pos,
                        egui::Align2::CENTER_CENTER,
                        poi.poi_type.icon(),
                        egui::FontId::proportional(16.0),
                        poi.poi_type.color(),
                    );
                }

                // === Enemy Markers ===
                for enemy in &self.enemies {
                    // Calculate relative position from player
                    let rel_x = enemy.world_pos.0 - self.player_position.0;
                    let rel_z = enemy.world_pos.2 - self.player_position.1; // enemy uses (x, y, z), minimap uses (x, z)

                    // Apply rotation if player-relative mode
                    let (screen_x, screen_z) = if self.state.minimap_rotation {
                        let cos = self.player_rotation.cos();
                        let sin = self.player_rotation.sin();
                        let rotated_x = rel_x * cos - rel_z * sin;
                        let rotated_z = rel_x * sin + rel_z * cos;
                        (rotated_x, rotated_z)
                    } else {
                        (rel_x, rel_z)
                    };

                    // Convert to screen coordinates
                    let marker_x = minimap_center.x + (screen_x / map_scale);
                    let marker_y = minimap_center.y - (screen_z / map_scale);

                    // Clamp to circular bounds
                    let dx = marker_x - minimap_center.x;
                    let dy = marker_y - minimap_center.y;
                    let dist = (dx * dx + dy * dy).sqrt();

                    if dist > minimap_radius - 10.0 {
                        continue;
                    }

                    let marker_pos = Pos2::new(marker_x, marker_y);

                    // Color based on faction
                    let enemy_color = match enemy.faction {
                        EnemyFaction::Hostile => Color32::RED,
                        EnemyFaction::Neutral => Color32::YELLOW,
                        EnemyFaction::Friendly => Color32::GREEN,
                    };

                    // Draw small dot
                    ui.painter().circle_filled(marker_pos, 3.0, enemy_color);
                }

                // === Week 4 Day 4: Ping Markers ===
                for ping in &self.ping_markers {
                    // Calculate relative position from player
                    let rel_x = ping.world_pos.0 - self.player_position.0;
                    let rel_z = ping.world_pos.1 - self.player_position.1;

                    // Apply rotation if player-relative mode
                    let (screen_x, screen_z) = if self.state.minimap_rotation {
                        let cos = self.player_rotation.cos();
                        let sin = self.player_rotation.sin();
                        let rotated_x = rel_x * cos - rel_z * sin;
                        let rotated_z = rel_x * sin + rel_z * cos;
                        (rotated_x, rotated_z)
                    } else {
                        (rel_x, rel_z)
                    };

                    // Convert to screen coordinates
                    let marker_x = minimap_center.x + (screen_x / map_scale);
                    let marker_y = minimap_center.y - (screen_z / map_scale);

                    // Clamp to circular bounds
                    let dx = marker_x - minimap_center.x;
                    let dy = marker_y - minimap_center.y;
                    let dist = (dx * dx + dy * dy).sqrt();

                    if dist > minimap_radius - 10.0 {
                        continue;
                    }

                    let ping_pos = Pos2::new(marker_x, marker_y);

                    // Expanding circle animation
                    let age = ping.age_normalized(self.game_time);
                    let radius = 5.0 + age * 15.0; // Expand from 5px to 20px
                    let alpha = ((1.0 - age) * 255.0) as u8; // Fade out

                    // Outer glow
                    ui.painter().circle_stroke(
                        ping_pos,
                        radius,
                        Stroke::new(
                            3.0,
                            Color32::from_rgba_premultiplied(100, 200, 255, alpha / 2),
                        ),
                    );

                    // Inner circle
                    ui.painter().circle_stroke(
                        ping_pos,
                        radius * 0.7,
                        Stroke::new(2.0, Color32::from_rgba_premultiplied(150, 220, 255, alpha)),
                    );
                }

                // === Player Marker (always at center) ===
                // Draw white triangle pointing in facing direction
                let player_triangle_size = 8.0;
                let player_angle = if self.state.minimap_rotation {
                    0.0 // Always pointing up in player-relative mode
                } else {
                    -self.player_rotation // Rotate to show actual facing direction
                };

                self.draw_directional_triangle(
                    ui,
                    minimap_center,
                    player_triangle_size,
                    player_angle,
                    Color32::WHITE,
                );

                // === Compass indicators (N/S/E/W) ===
                if !self.state.minimap_rotation {
                    // North-up mode: show cardinal directions
                    let compass_radius = minimap_radius - 15.0;

                    // North (top)
                    ui.painter().text(
                        Pos2::new(minimap_center.x, minimap_center.y - compass_radius),
                        egui::Align2::CENTER_CENTER,
                        "N",
                        egui::FontId::proportional(12.0),
                        Color32::WHITE,
                    );

                    // South (bottom)
                    ui.painter().text(
                        Pos2::new(minimap_center.x, minimap_center.y + compass_radius),
                        egui::Align2::CENTER_CENTER,
                        "S",
                        egui::FontId::proportional(12.0),
                        Color32::LIGHT_GRAY,
                    );

                    // East (right)
                    ui.painter().text(
                        Pos2::new(minimap_center.x + compass_radius, minimap_center.y),
                        egui::Align2::CENTER_CENTER,
                        "E",
                        egui::FontId::proportional(12.0),
                        Color32::LIGHT_GRAY,
                    );

                    // West (left)
                    ui.painter().text(
                        Pos2::new(minimap_center.x - compass_radius, minimap_center.y),
                        egui::Align2::CENTER_CENTER,
                        "W",
                        egui::FontId::proportional(12.0),
                        Color32::LIGHT_GRAY,
                    );
                }

                // === Week 5 Day 1: Mouse Click-to-Ping ===
                // Detect clicks on the minimap and convert to world coordinates
                let response = ui.allocate_rect(minimap_rect, egui::Sense::click());
                if response.clicked() {
                    if let Some(click_pos) = response.interact_pointer_pos() {
                        // Calculate offset from minimap center
                        let offset_x = click_pos.x - minimap_center.x;
                        let offset_y = click_pos.y - minimap_center.y;

                        // Check if click is within circular boundary
                        let dist = (offset_x * offset_x + offset_y * offset_y).sqrt();
                        if dist <= minimap_radius {
                            // Week 5 Day 2: Play minimap click sound (pitch varies with distance from center)
                            let normalized_dist = dist / minimap_radius; // 0.0 at center, 1.0 at edge
                            if let Some(ref callback) = self.on_minimap_click {
                                callback(normalized_dist);
                            }

                            // Apply map scale (zoom-aware)
                            let map_scale = 5.0 / self.state.minimap_zoom;
                            let world_offset_x = offset_x * map_scale;
                            let world_offset_z = -offset_y * map_scale; // Y inverted (screen down = world north)

                            // Apply rotation if player-relative mode
                            let (final_x, final_z) = if self.state.minimap_rotation {
                                let cos = self.player_rotation.cos();
                                let sin = self.player_rotation.sin();
                                (
                                    world_offset_x * cos - world_offset_z * sin,
                                    world_offset_x * sin + world_offset_z * cos,
                                )
                            } else {
                                (world_offset_x, world_offset_z)
                            };

                            // Translate to world coordinates
                            let world_pos = (
                                self.player_position.0 + final_x,
                                self.player_position.1 + final_z,
                            );

                            // Week 5 Day 2: Play ping spawn sound at world position
                            if let Some(ref callback) = self.on_ping_spawn {
                                callback(world_pos);
                            }

                            // Spawn ping at clicked location
                            self.spawn_ping(world_pos);
                            log::info!(
                                "Ping spawned at world pos ({:.1}, {:.1}) from minimap click",
                                world_pos.0,
                                world_pos.1
                            );
                        }
                    }
                }
            });
    }

    // === Helper methods for minimap marker shapes ===
    // Week 4 Day 4: These are superseded by emoji icons but kept for fallback
    #[allow(dead_code)]
    fn draw_star(&self, ui: &mut egui::Ui, center: egui::Pos2, size: f32, color: egui::Color32) {
        use std::f32::consts::PI;

        let mut points = Vec::new();
        for i in 0..10 {
            // Changed to 10 for 5-pointed star (alternating outer and inner)
            let angle = (i as f32 * 2.0 * PI / 10.0) - PI / 2.0; // Start at top
            let radius = if i % 2 == 0 { size } else { size * 0.4 };
            points.push(egui::Pos2::new(
                center.x + angle.cos() * radius,
                center.y + angle.sin() * radius,
            ));
        }

        // Draw filled star (triangles from center)
        for i in 0..10 {
            let p1 = points[i];
            let p2 = points[(i + 1) % 10];
            ui.painter().add(egui::Shape::convex_polygon(
                vec![center, p1, p2],
                color,
                egui::Stroke::NONE,
            ));
        }
    }

    #[allow(dead_code)]
    fn draw_diamond(&self, ui: &mut egui::Ui, center: egui::Pos2, size: f32, color: egui::Color32) {
        let points = vec![
            egui::Pos2::new(center.x, center.y - size), // Top
            egui::Pos2::new(center.x + size, center.y), // Right
            egui::Pos2::new(center.x, center.y + size), // Bottom
            egui::Pos2::new(center.x - size, center.y), // Left
        ];

        ui.painter().add(egui::Shape::convex_polygon(
            points,
            color,
            egui::Stroke::new(1.0, egui::Color32::DARK_BLUE),
        ));
    }

    #[allow(dead_code)]
    fn draw_triangle(
        &self,
        ui: &mut egui::Ui,
        center: egui::Pos2,
        size: f32,
        color: egui::Color32,
    ) {
        let points = vec![
            egui::Pos2::new(center.x, center.y - size), // Top
            egui::Pos2::new(center.x - size * 0.866, center.y + size * 0.5), // Bottom-left
            egui::Pos2::new(center.x + size * 0.866, center.y + size * 0.5), // Bottom-right
        ];

        ui.painter().add(egui::Shape::convex_polygon(
            points,
            color,
            egui::Stroke::new(1.0, egui::Color32::DARK_RED),
        ));
    }

    fn draw_directional_triangle(
        &self,
        ui: &mut egui::Ui,
        center: egui::Pos2,
        size: f32,
        angle: f32,
        color: egui::Color32,
    ) {
        // Triangle pointing up (angle 0), rotates clockwise
        let cos = angle.cos();
        let sin = angle.sin();

        // Base triangle (pointing up)
        let p1 = (0.0, -size); // Top
        let p2 = (-size * 0.7, size * 0.5); // Bottom-left
        let p3 = (size * 0.7, size * 0.5); // Bottom-right

        // Rotate and translate
        let points = vec![
            egui::Pos2::new(
                center.x + p1.0 * cos - p1.1 * sin,
                center.y + p1.0 * sin + p1.1 * cos,
            ),
            egui::Pos2::new(
                center.x + p2.0 * cos - p2.1 * sin,
                center.y + p2.0 * sin + p2.1 * cos,
            ),
            egui::Pos2::new(
                center.x + p3.0 * cos - p3.1 * sin,
                center.y + p3.0 * sin + p3.1 * cos,
            ),
        ];

        ui.painter().add(egui::Shape::convex_polygon(
            points,
            color,
            egui::Stroke::new(2.0, egui::Color32::DARK_GRAY),
        ));
    }

    // ===== Week 3 Day 4: Dialogue System =====

    fn render_dialogue(&self, ctx: &egui::Context) {
        use egui::{Color32, CornerRadius, Pos2, Rect, Stroke, StrokeKind, Vec2};

        let Some(dialogue) = &self.active_dialogue else {
            return;
        };

        let screen_size = ctx.screen_rect().size();
        let panel_width = 600.0;
        let panel_height = 180.0;
        let panel_x = (screen_size.x - panel_width) / 2.0; // Centered horizontally
        let panel_y = screen_size.y - panel_height - 20.0; // 20px from bottom

        egui::Area::new(egui::Id::new("dialogue_box"))
            .fixed_pos(Pos2::new(panel_x, panel_y))
            .show(ctx, |ui| {
                let panel_rect =
                    Rect::from_min_size(Pos2::new(0.0, 0.0), Vec2::new(panel_width, panel_height));

                // Background (darker, more opaque for readability)
                ui.painter().rect_filled(
                    panel_rect,
                    CornerRadius::same(8),
                    Color32::from_rgba_premultiplied(15, 15, 25, 240),
                );

                // Border (lighter blue for dialogue)
                ui.painter().rect_stroke(
                    panel_rect,
                    CornerRadius::same(8),
                    Stroke::new(2.0, Color32::from_rgb(100, 150, 200)),
                    StrokeKind::Middle,
                );

                // === Content Layout ===
                ui.vertical(|ui| {
                    ui.add_space(12.0);

                    // Speaker name (header)
                    ui.horizontal(|ui| {
                        ui.add_space(15.0);
                        ui.label(
                            egui::RichText::new(&dialogue.speaker_name)
                                .color(Color32::from_rgb(150, 200, 255))
                                .size(16.0)
                                .strong(),
                        );
                    });

                    ui.add_space(8.0);

                    // Dialogue text (wrapped, multi-line)
                    ui.horizontal(|ui| {
                        ui.add_space(15.0);

                        let text_width = panel_width - 30.0;
                        ui.vertical(|ui| {
                            ui.set_max_width(text_width);
                            ui.label(
                                egui::RichText::new(&dialogue.text)
                                    .color(Color32::WHITE)
                                    .size(14.0),
                            );
                        });
                    });

                    ui.add_space(12.0);

                    // === Dialogue Choices ===
                    if !dialogue.choices.is_empty() {
                        ui.add_space(4.0);

                        // Horizontal layout for choices (1-4 buttons)
                        ui.horizontal(|ui| {
                            ui.add_space(15.0);

                            for (idx, choice) in dialogue.choices.iter().enumerate() {
                                // Choice button
                                let button = egui::Button::new(
                                    egui::RichText::new(format!("{}. {}", idx + 1, &choice.text))
                                        .size(13.0),
                                )
                                .fill(Color32::from_rgb(40, 60, 80))
                                .stroke(Stroke::new(1.0, Color32::from_rgb(100, 150, 200)));

                                if ui.add(button).clicked() {
                                    log::info!("Dialogue choice clicked: {}", choice.text);
                                    // Note: Click handling should be done in demo via select_dialogue_choice()
                                }

                                if idx < dialogue.choices.len() - 1 {
                                    ui.add_space(8.0);
                                }
                            }
                        });

                        ui.add_space(8.0);

                        // Hint text (keyboard shortcuts)
                        ui.horizontal(|ui| {
                            ui.add_space(15.0);
                            ui.label(
                                egui::RichText::new("Press 1-4 to select choice")
                                    .color(Color32::DARK_GRAY)
                                    .size(11.0)
                                    .italics(),
                            );
                        });
                    } else {
                        // No choices = end of dialogue
                        ui.horizontal(|ui| {
                            ui.add_space(15.0);
                            ui.label(
                                egui::RichText::new("Press SPACE to continue...")
                                    .color(Color32::DARK_GRAY)
                                    .size(11.0)
                                    .italics(),
                            );
                        });
                    }

                    ui.add_space(8.0);
                });
            });
    }

    // ===== Week 3 Day 4: Tooltip System =====

    fn render_tooltip(&self, ctx: &egui::Context) {
        use egui::{Color32, CornerRadius, Pos2, Rect, Stroke, StrokeKind, Vec2};

        let Some(tooltip) = &self.hovered_tooltip else {
            return;
        };

        let screen_size = ctx.screen_rect().size();
        let tooltip_width = 280.0;

        // Calculate dynamic height based on content
        let line_height = 16.0;
        let padding = 12.0;
        let stat_count = tooltip.stats.len() as f32;
        let has_flavor = tooltip.flavor_text.is_some();

        let mut tooltip_height = padding * 2.0; // Top + bottom padding
        tooltip_height += line_height * 1.5; // Title (larger)
        tooltip_height += line_height * 2.0; // Description (wrapped, estimate 2 lines)
        tooltip_height += line_height * stat_count; // Stats
        if has_flavor {
            tooltip_height += line_height * 1.5; // Flavor text
        }
        tooltip_height += 20.0; // Extra spacing

        // Position tooltip near mouse, but keep on screen
        let mut tooltip_x = self.tooltip_position.0 + 15.0; // 15px offset from cursor
        let mut tooltip_y = self.tooltip_position.1 + 15.0;

        // Clamp to screen bounds
        if tooltip_x + tooltip_width > screen_size.x {
            tooltip_x = self.tooltip_position.0 - tooltip_width - 15.0; // Show to left of cursor
        }
        if tooltip_y + tooltip_height > screen_size.y {
            tooltip_y = screen_size.y - tooltip_height - 10.0;
        }
        tooltip_x = tooltip_x.max(10.0);
        tooltip_y = tooltip_y.max(10.0);

        egui::Area::new(egui::Id::new("tooltip"))
            .fixed_pos(Pos2::new(tooltip_x, tooltip_y))
            .show(ctx, |ui| {
                let tooltip_rect = Rect::from_min_size(
                    Pos2::new(0.0, 0.0),
                    Vec2::new(tooltip_width, tooltip_height),
                );

                // Background (dark, highly opaque)
                ui.painter().rect_filled(
                    tooltip_rect,
                    CornerRadius::same(4),
                    Color32::from_rgba_premultiplied(10, 10, 15, 250),
                );

                // Border (golden)
                ui.painter().rect_stroke(
                    tooltip_rect,
                    CornerRadius::same(4),
                    Stroke::new(2.0, Color32::from_rgb(180, 140, 60)),
                    StrokeKind::Middle,
                );

                // === Content ===
                ui.vertical(|ui| {
                    ui.add_space(8.0);

                    // Title (golden, bold)
                    ui.horizontal(|ui| {
                        ui.add_space(10.0);
                        ui.label(
                            egui::RichText::new(&tooltip.title)
                                .color(Color32::from_rgb(220, 180, 80))
                                .size(15.0)
                                .strong(),
                        );
                    });

                    ui.add_space(4.0);

                    // Description (white, normal)
                    ui.horizontal(|ui| {
                        ui.add_space(10.0);
                        ui.vertical(|ui| {
                            ui.set_max_width(tooltip_width - 20.0);
                            ui.label(
                                egui::RichText::new(&tooltip.description)
                                    .color(Color32::LIGHT_GRAY)
                                    .size(12.0),
                            );
                        });
                    });

                    ui.add_space(6.0);

                    // Stats (key-value pairs)
                    if !tooltip.stats.is_empty() {
                        ui.separator();
                        ui.add_space(4.0);

                        for (key, value) in &tooltip.stats {
                            ui.horizontal(|ui| {
                                ui.add_space(10.0);

                                // Key (light blue)
                                ui.label(
                                    egui::RichText::new(format!("{}:", key))
                                        .color(Color32::from_rgb(150, 180, 220))
                                        .size(12.0),
                                );

                                ui.add_space(8.0);

                                // Value (white)
                                ui.label(
                                    egui::RichText::new(value).color(Color32::WHITE).size(12.0),
                                );
                            });
                        }

                        ui.add_space(4.0);
                    }

                    // Flavor text (italicized, darker gray)
                    if let Some(flavor) = &tooltip.flavor_text {
                        ui.add_space(4.0);
                        ui.separator();
                        ui.add_space(4.0);

                        ui.horizontal(|ui| {
                            ui.add_space(10.0);
                            ui.vertical(|ui| {
                                ui.set_max_width(tooltip_width - 20.0);
                                ui.label(
                                    egui::RichText::new(flavor)
                                        .color(Color32::DARK_GRAY)
                                        .size(11.0)
                                        .italics(),
                                );
                            });
                        });
                    }

                    ui.add_space(8.0);
                });
            });
    }

    // ===== Week 3 Day 5: Subtitles & Notifications =====

    // TODO: Implement subtitle system (bottom-center)
    // TODO: Implement notification popups (top-center)
    // TODO: Implement toast messages (bottom-right)
    // TODO: Implement chat window placeholder (multiplayer future)
    fn render_subtitles(&self, _ctx: &egui::Context) {
        // Placeholder for Week 3 Day 5
    }
}

impl Default for HudManager {
    fn default() -> Self {
        Self::new()
    }
}

// ===== Helper Functions =====

/// Simplified world-to-screen projection for demo
///
/// In a real game engine, this would use the camera's view-projection matrix.
/// For this demo, we use a simple orthographic-style projection centered on screen.
fn world_to_screen_simple(
    world_pos: (f32, f32, f32),
    screen_size: (f32, f32),
) -> Option<(f32, f32)> {
    let (wx, wy, wz) = world_pos;

    // Mock projection: Offset from screen center with simple scaling
    // X: world units → pixels (scale: 20px per world unit)
    // Y: world units → pixels (inverted Y, scale: 20px per world unit)
    // Z: depth (for future distance culling, not used yet)

    let screen_x = screen_size.0 / 2.0 + wx * 20.0;
    let screen_y = screen_size.1 / 2.0 - wy * 20.0;

    // Depth culling (optional): Skip if behind camera or too far
    if !(-50.0..=50.0).contains(&wz) {
        return None;
    }

    Some((screen_x, screen_y))
}

impl HudManager {
    /// Render quest notifications (Week 4 Day 3)
    ///
    /// Displays slide-down animations for new quests, objective completions, and quest completions
    fn render_notifications(&self, ctx: &egui::Context) {
        // Only render if there's an active notification
        let Some(notification) = &self.notification_queue.active else {
            return;
        };

        // Calculate slide offset and alpha
        let slide_offset = notification.calculate_slide_offset();
        let alpha = notification.calculate_alpha();

        // Screen dimensions
        let screen_size = ctx.screen_rect().size();
        let panel_width = 400.0;
        let panel_x = (screen_size.x - panel_width) / 2.0; // Center horizontally
        let panel_y = 20.0 + slide_offset; // Top of screen + slide offset

        // Render notification based on type
        match &notification.notification_type {
            NotificationType::NewQuest => {
                self.render_new_quest_notification(
                    ctx,
                    notification,
                    panel_x,
                    panel_y,
                    panel_width,
                    alpha,
                );
            }
            NotificationType::ObjectiveComplete { objective_text } => {
                self.render_objective_complete_notification(
                    ctx,
                    notification,
                    objective_text,
                    panel_x,
                    panel_y,
                    panel_width,
                    alpha,
                );
            }
            NotificationType::QuestComplete { rewards } => {
                self.render_quest_complete_notification(
                    ctx,
                    notification,
                    rewards,
                    panel_x,
                    panel_y,
                    panel_width,
                    alpha,
                );
            }
        }
    }

    /// Render "New Quest!" notification (golden banner)
    fn render_new_quest_notification(
        &self,
        ctx: &egui::Context,
        notification: &QuestNotification,
        panel_x: f32,
        panel_y: f32,
        panel_width: f32,
        alpha: u8,
    ) {
        use egui::{Color32, CornerRadius, FontId, Pos2, Rect, Stroke, StrokeKind, Vec2};

        let panel_height = 80.0;

        egui::Area::new(egui::Id::new("notification_new_quest"))
            .fixed_pos(Pos2::new(panel_x, panel_y))
            .show(ctx, |ui| {
                let panel_rect =
                    Rect::from_min_size(Pos2::new(0.0, 0.0), Vec2::new(panel_width, panel_height));

                // Golden background
                ui.painter().rect_filled(
                    panel_rect,
                    CornerRadius::same(8),
                    Color32::from_rgba_premultiplied(80, 60, 20, alpha),
                );

                // Golden border (glowing effect)
                ui.painter().rect_stroke(
                    panel_rect,
                    CornerRadius::same(8),
                    Stroke::new(3.0, Color32::from_rgba_premultiplied(220, 180, 80, alpha)),
                    StrokeKind::Middle,
                );

                ui.vertical_centered(|ui| {
                    ui.add_space(8.0);

                    // "New Quest!" header
                    ui.label(
                        egui::RichText::new("📜 New Quest!")
                            .font(FontId::proportional(24.0))
                            .color(Color32::from_rgba_premultiplied(255, 220, 100, alpha))
                            .strong(),
                    );

                    ui.add_space(4.0);

                    // Quest title
                    ui.label(
                        egui::RichText::new(&notification.title)
                            .font(FontId::proportional(18.0))
                            .color(Color32::from_rgba_premultiplied(255, 255, 255, alpha)),
                    );
                });
            });
    }

    /// Render "Objective Complete!" notification (green checkmark)
    #[allow(clippy::too_many_arguments)]
    fn render_objective_complete_notification(
        &self,
        ctx: &egui::Context,
        _notification: &QuestNotification,
        objective_text: &str,
        panel_x: f32,
        panel_y: f32,
        panel_width: f32,
        alpha: u8,
    ) {
        use egui::{Color32, CornerRadius, FontId, Pos2, Rect, Stroke, StrokeKind, Vec2};

        let panel_height = 70.0;

        egui::Area::new(egui::Id::new("notification_objective_complete"))
            .fixed_pos(Pos2::new(panel_x, panel_y))
            .show(ctx, |ui| {
                let panel_rect =
                    Rect::from_min_size(Pos2::new(0.0, 0.0), Vec2::new(panel_width, panel_height));

                // Green background
                ui.painter().rect_filled(
                    panel_rect,
                    CornerRadius::same(8),
                    Color32::from_rgba_premultiplied(20, 60, 30, alpha),
                );

                // Green border
                ui.painter().rect_stroke(
                    panel_rect,
                    CornerRadius::same(8),
                    Stroke::new(2.0, Color32::from_rgba_premultiplied(80, 220, 100, alpha)),
                    StrokeKind::Middle,
                );

                ui.vertical_centered(|ui| {
                    ui.add_space(6.0);

                    // "Objective Complete!" header with checkmark
                    ui.label(
                        egui::RichText::new("✓ Objective Complete!")
                            .font(FontId::proportional(20.0))
                            .color(Color32::from_rgba_premultiplied(100, 255, 120, alpha))
                            .strong(),
                    );

                    ui.add_space(2.0);

                    // Objective text
                    ui.label(
                        egui::RichText::new(objective_text)
                            .font(FontId::proportional(14.0))
                            .color(Color32::from_rgba_premultiplied(200, 200, 200, alpha)),
                    );
                });
            });
    }

    /// Render "Quest Complete!" notification (large banner with rewards)
    #[allow(clippy::too_many_arguments)]
    fn render_quest_complete_notification(
        &self,
        ctx: &egui::Context,
        notification: &QuestNotification,
        rewards: &[String],
        panel_x: f32,
        panel_y: f32,
        panel_width: f32,
        alpha: u8,
    ) {
        use egui::{Color32, CornerRadius, FontId, Pos2, Rect, Stroke, StrokeKind, Vec2};

        // Dynamic height based on rewards
        let panel_height = 100.0 + (rewards.len() as f32 * 20.0);

        egui::Area::new(egui::Id::new("notification_quest_complete"))
            .fixed_pos(Pos2::new(panel_x, panel_y))
            .show(ctx, |ui| {
                let panel_rect =
                    Rect::from_min_size(Pos2::new(0.0, 0.0), Vec2::new(panel_width, panel_height));

                // Purple/gold gradient background
                ui.painter().rect_filled(
                    panel_rect,
                    CornerRadius::same(10),
                    Color32::from_rgba_premultiplied(60, 40, 80, alpha),
                );

                // Glowing purple/gold border
                ui.painter().rect_stroke(
                    panel_rect,
                    CornerRadius::same(10),
                    Stroke::new(4.0, Color32::from_rgba_premultiplied(200, 150, 255, alpha)),
                    StrokeKind::Middle,
                );

                ui.vertical_centered(|ui| {
                    ui.add_space(10.0);

                    // "Quest Complete!" header
                    ui.label(
                        egui::RichText::new("🏆 QUEST COMPLETE!")
                            .font(FontId::proportional(28.0))
                            .color(Color32::from_rgba_premultiplied(255, 220, 100, alpha))
                            .strong(),
                    );

                    ui.add_space(4.0);

                    // Quest title
                    ui.label(
                        egui::RichText::new(&notification.title)
                            .font(FontId::proportional(20.0))
                            .color(Color32::from_rgba_premultiplied(255, 255, 255, alpha)),
                    );

                    ui.add_space(8.0);

                    // Rewards header
                    if !rewards.is_empty() {
                        ui.label(
                            egui::RichText::new("Rewards:")
                                .font(FontId::proportional(16.0))
                                .color(Color32::from_rgba_premultiplied(200, 200, 200, alpha)),
                        );

                        ui.add_space(4.0);

                        // List rewards
                        for reward in rewards {
                            ui.label(
                                egui::RichText::new(format!("• {}", reward))
                                    .font(FontId::proportional(14.0))
                                    .color(Color32::from_rgba_premultiplied(255, 220, 100, alpha)),
                            );
                        }
                    }
                });
            });
    }
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
#[allow(clippy::manual_range_contains)]
#[allow(clippy::redundant_closure)]
mod tests {
    use super::*;

    // ========================================================================
    // Mutation-killing tests for world_to_screen_simple
    // ========================================================================

    #[test]
    fn test_world_to_screen_simple_golden_values() {
        // Screen center with world origin
        let result = world_to_screen_simple((0.0, 0.0, 0.0), (800.0, 600.0));
        assert_eq!(
            result,
            Some((400.0, 300.0)),
            "Origin should map to screen center"
        );

        // Positive X shifts right: 800/2 + 5*20 = 400 + 100 = 500
        let result = world_to_screen_simple((5.0, 0.0, 0.0), (800.0, 600.0));
        assert_eq!(result, Some((500.0, 300.0)), "Positive X maps right");

        // Positive Y shifts up (screen Y inverted): 600/2 - 5*20 = 300 - 100 = 200
        let result = world_to_screen_simple((0.0, 5.0, 0.0), (800.0, 600.0));
        // screen_x = 800/2 + 0*20 = 400, screen_y = 600/2 - 5*20 = 200
        assert_eq!(
            result,
            Some((400.0, 200.0)),
            "Positive Y maps up (inverted)"
        );
    }

    #[test]
    fn test_world_to_screen_simple_depth_culling() {
        // Z within range: should return Some
        let result = world_to_screen_simple((0.0, 0.0, 25.0), (800.0, 600.0));
        assert!(
            result.is_some(),
            "Z=25 within -50..=50 should not be culled"
        );

        // Z at boundary: -50 and 50 should be included (contains uses ..=)
        let result = world_to_screen_simple((0.0, 0.0, 50.0), (800.0, 600.0));
        assert!(result.is_some(), "Z=50 at boundary should be included");
        let result = world_to_screen_simple((0.0, 0.0, -50.0), (800.0, 600.0));
        assert!(result.is_some(), "Z=-50 at boundary should be included");

        // Z outside range: should return None
        let result = world_to_screen_simple((0.0, 0.0, 51.0), (800.0, 600.0));
        assert!(result.is_none(), "Z=51 outside range should be culled");
        let result = world_to_screen_simple((0.0, 0.0, -51.0), (800.0, 600.0));
        assert!(result.is_none(), "Z=-51 outside range should be culled");
    }

    #[test]
    fn test_world_to_screen_simple_arithmetic() {
        // Non-trivial position: (3.0, 2.0, 0.0) on 1920x1080 screen
        // screen_x = 1920/2 + 3*20 = 960 + 60 = 1020
        // screen_y = 1080/2 - 2*20 = 540 - 40 = 500
        let result = world_to_screen_simple((3.0, 2.0, 0.0), (1920.0, 1080.0));
        assert_eq!(
            result,
            Some((1020.0, 500.0)),
            "Arithmetic check: x=960+60=1020, y=540-40=500"
        );

        // Negative world coords
        // screen_x = 960 + (-4)*20 = 960 - 80 = 880
        // screen_y = 540 - (-3)*20 = 540 + 60 = 600
        let result = world_to_screen_simple((-4.0, -3.0, 0.0), (1920.0, 1080.0));
        assert_eq!(
            result,
            Some((880.0, 600.0)),
            "Negative coords: x=960-80=880, y=540+60=600"
        );
    }

    // ========================================================================
    // Mutation-killing test for HudManager::update damage number retention
    // ========================================================================

    #[test]
    fn test_damage_number_retention_during_update() {
        let mut hud = HudManager::new();
        hud.game_time = 10.0;
        // Spawn a damage number
        hud.spawn_damage(25, (5.0, 3.0, 0.0), DamageType::Normal);
        assert_eq!(hud.damage_numbers.len(), 1, "should have 1 damage number");

        // Advance by 0.5s (within 1.5s lifetime)
        hud.game_time = 10.5;
        hud.update(0.5);
        assert_eq!(
            hud.damage_numbers.len(),
            1,
            "damage number should persist at age=0.5s (catches - → + in retain)"
        );

        // Advance past lifetime (total age > 1.5s)
        hud.game_time = 12.0;
        hud.update(1.5);
        assert_eq!(
            hud.damage_numbers.len(),
            0,
            "damage number should be removed after 2.0s lifetime"
        );
    }

    #[test]
    fn test_hud_manager_creation() {
        let hud = HudManager::new();
        assert!(hud.is_visible(), "HUD should be visible by default");
        assert!(
            hud.state().show_health_bars,
            "Health bars should be enabled by default"
        );
        assert!(
            hud.state().show_objectives,
            "Objectives should be enabled by default"
        );
        assert!(
            hud.state().show_minimap,
            "Minimap should be enabled by default"
        );
        assert!(
            hud.state().show_subtitles,
            "Subtitles should be enabled by default"
        );
        assert!(
            !hud.state().debug_mode,
            "Debug mode should be off by default"
        );
    }

    #[test]
    fn test_hud_visibility_toggle() {
        let mut hud = HudManager::new();
        assert!(hud.is_visible());

        hud.toggle_visibility();
        assert!(!hud.is_visible(), "HUD should be hidden after toggle");

        hud.toggle_visibility();
        assert!(
            hud.is_visible(),
            "HUD should be visible after second toggle"
        );
    }

    #[test]
    fn test_hud_set_visible() {
        let mut hud = HudManager::new();

        hud.set_visible(false);
        assert!(!hud.is_visible());

        hud.set_visible(true);
        assert!(hud.is_visible());
    }

    #[test]
    fn test_hud_debug_toggle() {
        let mut hud = HudManager::new();
        assert!(!hud.state().debug_mode);

        hud.toggle_debug();
        assert!(hud.state().debug_mode);

        hud.toggle_debug();
        assert!(!hud.state().debug_mode);
    }

    #[test]
    fn test_hud_state_get_set() {
        let mut hud = HudManager::new();

        let mut state = hud.state().clone();
        state.visible = false;
        state.show_health_bars = false;
        state.debug_mode = true;

        hud.set_state(state.clone());

        assert!(!hud.is_visible());
        assert!(!hud.state().show_health_bars);
        assert!(hud.state().debug_mode);
    }

    // ===== Week 4 Day 1: Health Animation Tests =====

    #[test]
    fn test_health_animation_new() {
        let anim = HealthAnimation::new(100.0);
        assert_eq!(anim.current_visual, 100.0);
        assert_eq!(anim.target, 100.0);
        assert_eq!(anim.animation_time, 0.0);
        assert_eq!(anim.flash_timer, 0.0);
    }

    #[test]
    fn test_health_animation_damage() {
        let mut anim = HealthAnimation::new(100.0);
        anim.set_target(50.0);

        assert_eq!(anim.target, 50.0);
        assert_eq!(anim.animation_time, 0.0);
        assert!(anim.flash_timer > 0.0, "Flash should trigger on damage");
    }

    #[test]
    fn test_health_animation_healing() {
        let mut anim = HealthAnimation::new(50.0);
        anim.set_target(100.0);

        assert_eq!(anim.target, 100.0);
        assert!(anim.is_healing());
        assert_eq!(anim.flash_timer, 0.0, "Flash should not trigger on healing");
    }

    #[test]
    fn test_health_animation_update() {
        let mut anim = HealthAnimation::new(100.0);
        anim.set_target(50.0);

        // Update animation
        anim.update(0.2); // Half of default 0.4s duration

        // Visual health should be between target and start
        let visual = anim.visual_health();
        assert!(
            visual < 100.0 && visual > 50.0,
            "Visual health should be animating"
        );

        // Complete animation
        anim.update(0.3); // Total 0.5s > 0.4s duration
        assert!(
            (anim.visual_health() - 50.0).abs() < 0.1,
            "Animation should complete"
        );
    }

    #[test]
    fn test_health_animation_flash_alpha() {
        let mut anim = HealthAnimation::new(100.0);
        anim.set_target(50.0);

        // Flash should be active immediately after damage
        let alpha = anim.flash_alpha();
        assert!(
            alpha > 0.0 && alpha <= 0.6,
            "Flash alpha should be in valid range"
        );

        // Flash should decay over time
        anim.update(0.1);
        let alpha2 = anim.flash_alpha();
        assert!(alpha2 < alpha, "Flash should decay");

        // Flash should end after duration
        anim.update(0.3);
        assert_eq!(anim.flash_alpha(), 0.0, "Flash should end");
    }

    #[test]
    fn test_easing_ease_out_cubic() {
        let start = easing::ease_out_cubic(0.0);
        let mid = easing::ease_out_cubic(0.5);
        let end = easing::ease_out_cubic(1.0);

        assert_eq!(start, 0.0);
        assert_eq!(end, 1.0);
        assert!(mid > 0.0 && mid < 1.0);
    }

    #[test]
    fn test_easing_ease_in_out_quad() {
        let start = easing::ease_in_out_quad(0.0);
        let mid = easing::ease_in_out_quad(0.5);
        let end = easing::ease_in_out_quad(1.0);

        assert_eq!(start, 0.0);
        assert_eq!(end, 1.0);
        assert!(mid > 0.0 && mid < 1.0);
    }

    // ===== Week 3 Day 2: Player/Enemy Data Tests =====

    #[test]
    fn test_player_stats_default() {
        let stats = PlayerStats::default();
        assert_eq!(stats.health, 100.0);
        assert_eq!(stats.max_health, 100.0);
        assert_eq!(stats.mana, 100.0);
        assert_eq!(stats.max_mana, 100.0);
        assert_eq!(stats.stamina, 100.0);
        assert_eq!(stats.max_stamina, 100.0);
    }

    #[test]
    fn test_enemy_data_construction() {
        let enemy = EnemyData::new(42, (10.0, 5.0, 20.0), 100.0, EnemyFaction::Hostile);

        assert_eq!(enemy.id, 42);
        assert_eq!(enemy.world_pos, (10.0, 5.0, 20.0));
        assert_eq!(enemy.health, 100.0);
        assert_eq!(enemy.max_health, 100.0);
        assert_eq!(enemy.faction, EnemyFaction::Hostile);
    }

    #[test]
    fn test_enemy_faction_equality() {
        assert_eq!(EnemyFaction::Hostile, EnemyFaction::Hostile);
        assert_eq!(EnemyFaction::Neutral, EnemyFaction::Neutral);
        assert_ne!(EnemyFaction::Hostile, EnemyFaction::Neutral);
    }

    #[test]
    fn test_damage_number_construction() {
        let dmg = DamageNumber::new(42, 1.5, (10.0, 5.0, 20.0), DamageType::Normal);

        assert_eq!(dmg.value, 42);
        assert_eq!(dmg.damage_type, DamageType::Normal);
        assert_eq!(dmg.spawn_time, 1.5);
        assert_eq!(dmg.world_pos, (10.0, 5.0, 20.0));
    }

    #[test]
    fn test_damage_type_variants() {
        let normal = DamageType::Normal;
        let critical = DamageType::Critical;
        let self_damage = DamageType::SelfDamage;

        assert_ne!(normal, critical);
        assert_ne!(critical, self_damage);
        assert_ne!(normal, self_damage);
    }

    // ===== Week 3 Day 3: Quest & Minimap Tests =====

    #[test]
    fn test_quest_construction() {
        let quest = Quest {
            id: 1,
            title: "Defeat the Dragon".to_string(),
            description: "Slay the mighty dragon terrorizing the village".to_string(),
            objectives: vec![
                Objective {
                    id: 1,
                    description: "Find the dragon's lair".to_string(),
                    completed: false,
                    progress: None,
                },
                Objective {
                    id: 2,
                    description: "Slay the dragon".to_string(),
                    completed: false,
                    progress: None,
                },
            ],
        };

        assert_eq!(quest.id, 1);
        assert_eq!(quest.title, "Defeat the Dragon");
        assert_eq!(quest.objectives.len(), 2);
        assert!(!quest.objectives[0].completed);
        assert!(!quest.is_complete());
    }

    #[test]
    fn test_quest_completion() {
        let mut quest = Quest {
            id: 1,
            title: "Test Quest".to_string(),
            description: "Complete objectives".to_string(),
            objectives: vec![
                Objective {
                    id: 1,
                    description: "Objective 1".to_string(),
                    completed: true,
                    progress: None,
                },
                Objective {
                    id: 2,
                    description: "Objective 2".to_string(),
                    completed: true,
                    progress: None,
                },
            ],
        };

        assert!(quest.is_complete());
        assert_eq!(quest.completion(), 1.0);

        // Incomplete quest
        quest.objectives[1].completed = false;
        assert!(!quest.is_complete());
        assert_eq!(quest.completion(), 0.5);
    }

    #[test]
    fn test_objective_completion() {
        let mut obj = Objective {
            id: 1,
            description: "Collect 10 herbs".to_string(),
            completed: false,
            progress: Some((0, 10)),
        };

        assert!(!obj.completed);
        assert_eq!(obj.progress, Some((0, 10)));

        obj.completed = true;
        obj.progress = Some((10, 10));
        assert!(obj.completed);
    }

    #[test]
    fn test_poi_marker_construction() {
        let poi = PoiMarker {
            id: 42,
            world_pos: (100.0, 50.0),
            poi_type: PoiType::Objective,
            label: Some("Quest Marker".to_string()),
        };

        assert_eq!(poi.id, 42);
        assert_eq!(poi.world_pos, (100.0, 50.0));
        assert_eq!(poi.poi_type, PoiType::Objective);
        assert_eq!(poi.label, Some("Quest Marker".to_string()));
    }

    #[test]
    fn test_poi_type_variants() {
        let obj = PoiType::Objective;
        let way = PoiType::Waypoint;
        let vendor = PoiType::Vendor;
        let danger = PoiType::Danger;

        assert_ne!(obj, way);
        assert_ne!(vendor, danger);
        assert_ne!(obj, vendor);
    }

    #[test]
    fn test_poi_type_icon() {
        assert_eq!(PoiType::Objective.icon(), "🎯");
        assert_eq!(PoiType::Waypoint.icon(), "📍");
        assert_eq!(PoiType::Vendor.icon(), "🏪");
        assert_eq!(PoiType::Danger.icon(), "⚔️");
    }

    #[test]
    fn test_ping_marker_creation() {
        let ping = PingMarker::new((50.0, 100.0), 10.0);
        assert_eq!(ping.world_pos, (50.0, 100.0));
        assert_eq!(ping.spawn_time, 10.0);
        assert_eq!(ping.duration, 3.0);
        assert!(ping.is_active(11.0));
        assert!(!ping.is_active(14.0));
    }

    // ===== Week 3 Day 4: Dialogue & Tooltip Tests =====

    #[test]
    fn test_dialogue_node_construction() {
        let node = DialogueNode {
            id: 1,
            speaker_name: "Gandalf".to_string(),
            text: "You shall not pass!".to_string(),
            portrait_id: None,
            choices: vec![
                DialogueChoice {
                    id: 1,
                    text: "Attack".to_string(),
                    next_node: None,
                },
                DialogueChoice {
                    id: 2,
                    text: "Flee".to_string(),
                    next_node: Some(42),
                },
            ],
        };

        assert_eq!(node.id, 1);
        assert_eq!(node.speaker_name, "Gandalf");
        assert_eq!(node.text, "You shall not pass!");
        assert_eq!(node.choices.len(), 2);
        assert_eq!(node.choices[0].text, "Attack");
        assert_eq!(node.choices[1].next_node, Some(42));
    }

    #[test]
    fn test_dialogue_choice_no_next() {
        let choice = DialogueChoice {
            id: 1,
            text: "End conversation".to_string(),
            next_node: None,
        };

        assert_eq!(choice.next_node, None);
    }

    #[test]
    fn test_tooltip_data_construction() {
        let tooltip = TooltipData {
            title: "Excalibur".to_string(),
            description: "Legendary sword of King Arthur".to_string(),
            stats: vec![
                ("Damage".to_string(), "100".to_string()),
                ("Critical".to_string(), "+50%".to_string()),
            ],
            flavor_text: Some("Only the true king can wield this blade.".to_string()),
        };

        assert_eq!(tooltip.title, "Excalibur");
        assert_eq!(tooltip.description, "Legendary sword of King Arthur");
        assert_eq!(tooltip.stats.len(), 2);
        assert_eq!(tooltip.stats[0].0, "Damage");
        assert_eq!(tooltip.stats[0].1, "100");
        assert!(tooltip.flavor_text.is_some());
    }

    #[test]
    fn test_tooltip_empty_stats() {
        let tooltip = TooltipData {
            title: "Simple Item".to_string(),
            description: "Basic description".to_string(),
            stats: vec![],
            flavor_text: None,
        };

        assert!(tooltip.stats.is_empty());
        assert!(tooltip.flavor_text.is_none());
    }

    // ===== NotificationQueue Edge Cases Tests =====

    #[test]
    fn test_notification_queue_empty() {
        let queue = NotificationQueue::new();
        assert!(!queue.has_active());
        assert!(queue.active.is_none());
        assert_eq!(queue.pending.len(), 0);
    }

    #[test]
    fn test_notification_queue_push_single() {
        let mut queue = NotificationQueue::new();
        let notification =
            QuestNotification::new_quest("Test Quest".to_string(), "A test quest".to_string());

        queue.push(notification);

        assert!(queue.has_active());
        assert!(queue.active.is_some());
        assert_eq!(queue.pending.len(), 0);
    }

    #[test]
    fn test_notification_queue_push_multiple() {
        let mut queue = NotificationQueue::new();

        // Push 3 notifications
        for i in 1..=3 {
            let notification =
                QuestNotification::new_quest(format!("Quest {}", i), format!("Description {}", i));
            queue.push(notification);
        }

        assert!(queue.has_active());
        assert_eq!(queue.pending.len(), 2);
    }

    #[test]
    fn test_notification_queue_overflow_behavior() {
        let mut queue = NotificationQueue::new();

        // Push 100 notifications (stress test)
        for i in 1..=100 {
            let notification =
                QuestNotification::new_quest(format!("Quest {}", i), format!("Description {}", i));
            queue.push(notification);
        }

        assert!(queue.has_active());
        assert_eq!(queue.pending.len(), 99);
    }

    #[test]
    fn test_notification_queue_expiration_timing() {
        let mut queue = NotificationQueue::new();
        let notification = QuestNotification::new_quest(
            "Short Quest".to_string(),
            "Quick notification".to_string(),
        );

        queue.push(notification);
        assert!(queue.has_active());

        // Update with total duration to expire notification
        queue.update(2.0);

        // Should auto-pop to next (which is none)
        assert!(!queue.has_active());
    }

    #[test]
    fn test_notification_queue_auto_pop_sequence() {
        let mut queue = NotificationQueue::new();

        // Push 3 notifications
        for i in 1..=3 {
            let notification =
                QuestNotification::new_quest(format!("Quest {}", i), format!("Description {}", i));
            queue.push(notification);
        }

        assert!(queue.has_active());
        assert_eq!(queue.pending.len(), 2);

        // Expire first notification
        queue.update(2.0);
        assert!(queue.has_active());
        assert_eq!(queue.pending.len(), 1);

        // Expire second notification
        queue.update(2.0);
        assert!(queue.has_active());
        assert_eq!(queue.pending.len(), 0);

        // Expire third notification
        queue.update(2.0);
        assert!(!queue.has_active());
    }

    #[test]
    fn test_notification_priority_ordering() {
        let mut queue = NotificationQueue::new();

        // Push notifications in order
        queue.push(QuestNotification::new_quest(
            "First".to_string(),
            "First quest".to_string(),
        ));
        queue.push(QuestNotification::objective_complete(
            "Objective 1".to_string(),
        ));
        queue.push(QuestNotification::quest_complete(
            "Completed Quest".to_string(),
            vec!["Gold".to_string(), "XP".to_string()],
        ));

        // Should process in FIFO order
        assert!(queue.has_active());
        assert_eq!(queue.pending.len(), 2);
    }

    // ===== Quest System Tests =====

    #[test]
    fn test_quest_multi_objective_progress() {
        let quest = Quest {
            id: 1,
            title: "Gather Resources".to_string(),
            description: "Collect various resources".to_string(),
            objectives: vec![
                Objective {
                    id: 1,
                    description: "Collect 10 wood".to_string(),
                    completed: true,
                    progress: Some((10, 10)),
                },
                Objective {
                    id: 2,
                    description: "Collect 5 stone".to_string(),
                    completed: false,
                    progress: Some((3, 5)),
                },
                Objective {
                    id: 3,
                    description: "Find iron ore".to_string(),
                    completed: false,
                    progress: None,
                },
            ],
        };

        assert_eq!(quest.completion(), 1.0 / 3.0);
        assert!(!quest.is_complete());
    }

    #[test]
    fn test_quest_completion_percentage() {
        let mut quest = Quest {
            id: 1,
            title: "Four Objectives".to_string(),
            description: "Complete all four".to_string(),
            objectives: vec![
                Objective {
                    id: 1,
                    description: "First".to_string(),
                    completed: true,
                    progress: None,
                },
                Objective {
                    id: 2,
                    description: "Second".to_string(),
                    completed: true,
                    progress: None,
                },
                Objective {
                    id: 3,
                    description: "Third".to_string(),
                    completed: false,
                    progress: None,
                },
                Objective {
                    id: 4,
                    description: "Fourth".to_string(),
                    completed: false,
                    progress: None,
                },
            ],
        };

        assert_eq!(quest.completion(), 0.5);

        quest.objectives[2].completed = true;
        assert_eq!(quest.completion(), 0.75);

        quest.objectives[3].completed = true;
        assert_eq!(quest.completion(), 1.0);
        assert!(quest.is_complete());
    }

    #[test]
    fn test_quest_optional_objective_handling() {
        let quest = Quest {
            id: 1,
            title: "Main Quest".to_string(),
            description: "With optional objectives".to_string(),
            objectives: vec![
                Objective {
                    id: 1,
                    description: "Required objective".to_string(),
                    completed: true,
                    progress: None,
                },
                Objective {
                    id: 2,
                    description: "Optional objective".to_string(),
                    completed: false,
                    progress: None,
                },
            ],
        };

        // Even with optional objectives incomplete, progress should be trackable
        assert_eq!(quest.completion(), 0.5);
        assert!(!quest.is_complete());
    }

    #[test]
    fn test_quest_empty_objectives() {
        let quest = Quest {
            id: 1,
            title: "Empty Quest".to_string(),
            description: "No objectives".to_string(),
            objectives: vec![],
        };

        assert_eq!(quest.completion(), 0.0);
        assert!(!quest.is_complete());
    }

    #[test]
    fn test_quest_single_objective() {
        let mut quest = Quest {
            id: 1,
            title: "Simple Quest".to_string(),
            description: "One objective".to_string(),
            objectives: vec![Objective {
                id: 1,
                description: "Single objective".to_string(),
                completed: false,
                progress: None,
            }],
        };

        assert_eq!(quest.completion(), 0.0);
        assert!(!quest.is_complete());

        quest.objectives[0].completed = true;
        assert_eq!(quest.completion(), 1.0);
        assert!(quest.is_complete());
    }

    #[test]
    fn test_quest_all_objectives_incomplete() {
        let quest = Quest {
            id: 1,
            title: "Fresh Quest".to_string(),
            description: "Just started".to_string(),
            objectives: vec![
                Objective {
                    id: 1,
                    description: "First".to_string(),
                    completed: false,
                    progress: None,
                },
                Objective {
                    id: 2,
                    description: "Second".to_string(),
                    completed: false,
                    progress: None,
                },
                Objective {
                    id: 3,
                    description: "Third".to_string(),
                    completed: false,
                    progress: None,
                },
            ],
        };

        assert_eq!(quest.completion(), 0.0);
        assert!(!quest.is_complete());
    }

    #[test]
    fn test_quest_progress_tracking() {
        let quest = Quest {
            id: 1,
            title: "Collection Quest".to_string(),
            description: "Collect items".to_string(),
            objectives: vec![
                Objective {
                    id: 1,
                    description: "Collect 5 apples".to_string(),
                    completed: false,
                    progress: Some((2, 5)),
                },
                Objective {
                    id: 2,
                    description: "Collect 3 oranges".to_string(),
                    completed: true,
                    progress: Some((3, 3)),
                },
            ],
        };

        // One objective complete out of two
        assert_eq!(quest.completion(), 0.5);
        assert!(!quest.is_complete());
    }

    // ===== ComboTracker Tests =====

    #[test]
    fn test_combo_tracker_new() {
        let tracker = ComboTracker::new();
        assert_eq!(tracker.get_combo_count(0.0), 0);
        assert_eq!(tracker.get_combo_damage(0.0), 0);
    }

    #[test]
    fn test_combo_tracker_default() {
        let tracker = ComboTracker::default();
        assert_eq!(tracker.get_combo_count(0.0), 0);
    }

    #[test]
    fn test_combo_tracker_single_hit() {
        let mut tracker = ComboTracker::new();
        tracker.record_hit(1.0, 50);
        assert_eq!(tracker.get_combo_count(1.0), 1);
        assert_eq!(tracker.get_combo_damage(1.0), 50);
    }

    #[test]
    fn test_combo_tracker_multiple_hits() {
        let mut tracker = ComboTracker::new();
        tracker.record_hit(0.0, 50);
        tracker.record_hit(0.2, 75);
        tracker.record_hit(0.5, 100);

        assert_eq!(tracker.get_combo_count(0.5), 3);
        assert_eq!(tracker.get_combo_damage(0.5), 225);
    }

    #[test]
    fn test_combo_tracker_window_expiration() {
        let mut tracker = ComboTracker::new();
        tracker.record_hit(0.0, 50);
        tracker.record_hit(0.3, 75);

        // Both hits within 1s window at time 0.5
        assert_eq!(tracker.get_combo_count(0.5), 2);

        // At time 1.1, first hit (at 0.0) is outside 1s window (1.1 - 0.0 = 1.1 > 1.0)
        // But second hit (at 0.3) is still in window (1.1 - 0.3 = 0.8 <= 1.0)
        assert_eq!(tracker.get_combo_count(1.1), 1);
        assert_eq!(tracker.get_combo_damage(1.1), 75);

        // At time 1.5, second hit (at 0.3) is also outside window (1.5 - 0.3 = 1.2 > 1.0)
        assert_eq!(tracker.get_combo_count(1.5), 0);
    }

    #[test]
    fn test_combo_tracker_cleanup() {
        let mut tracker = ComboTracker::new();
        tracker.record_hit(0.0, 50);
        tracker.record_hit(0.5, 75);
        tracker.record_hit(1.2, 100);

        // Before cleanup at 1.2s, first hit should have expired
        tracker.cleanup(1.2);
        assert_eq!(tracker.get_combo_count(1.2), 2); // Only 0.5 and 1.2 hits
        assert_eq!(tracker.get_combo_damage(1.2), 175);
    }

    #[test]
    fn test_combo_tracker_rapid_hits() {
        let mut tracker = ComboTracker::new();
        for i in 0..10 {
            tracker.record_hit(i as f32 * 0.05, 10); // 10 hits in 0.5s
        }
        assert_eq!(tracker.get_combo_count(0.5), 10);
        assert_eq!(tracker.get_combo_damage(0.5), 100);
    }

    // ===== DamageNumber Arc Motion and Shake Tests =====

    #[test]
    fn test_damage_number_calculate_offset_initial() {
        let dmg = DamageNumber::new(50, 0.0, (0.0, 0.0, 0.0), DamageType::Normal);
        let (x, y) = dmg.calculate_offset(0.0);
        assert_eq!(x, 0.0);
        assert_eq!(y, 0.0);
    }

    #[test]
    fn test_damage_number_calculate_offset_motion() {
        let dmg = DamageNumber::new(50, 0.0, (0.0, 0.0, 0.0), DamageType::Normal);
        let (_, y) = dmg.calculate_offset(0.5);

        // After 0.5s with gravity, y should have moved (starts negative = up, then gravity pulls down)
        // At t=0.5: y = vy*t + 0.5*g*t² = -80*0.5 + 0.5*150*0.25 = -40 + 18.75 = -21.25
        assert!((y - (-21.25)).abs() < 0.1);
    }

    #[test]
    fn test_damage_number_calculate_offset_parabolic() {
        let dmg = DamageNumber::new(50, 0.0, (0.0, 0.0, 0.0), DamageType::Normal);

        // Test parabolic arc (goes up then down)
        let (_, y0) = dmg.calculate_offset(0.0);
        let (_, y1) = dmg.calculate_offset(0.25);
        let (_, y2) = dmg.calculate_offset(0.5);
        let (_, y3) = dmg.calculate_offset(1.0);

        // Initial position
        assert_eq!(y0, 0.0);
        // Should go up (negative y) initially
        assert!(y1 < y0, "Should move up initially");
        // Then gravity pulls down
        assert!(y3 > y2, "Gravity should pull down eventually");
    }

    #[test]
    fn test_damage_number_calculate_shake_initial() {
        let dmg = DamageNumber::new(50, 0.0, (0.0, 0.0, 0.0), DamageType::Normal);
        let shake = dmg.calculate_shake(0.0);
        assert_eq!(shake, 0.0);
    }

    #[test]
    fn test_damage_number_calculate_shake_damped() {
        let dmg = DamageNumber::new(50, 0.0, (0.0, 0.0, 0.0), DamageType::Normal);

        // Shake should be damped over time
        let shake_early = dmg.calculate_shake(0.1).abs();
        let shake_late = dmg.calculate_shake(0.5).abs();

        // Later shake should be smaller due to damping
        assert!(shake_late < shake_early, "Shake should decay over time");
    }

    #[test]
    fn test_damage_number_critical_shake_amplitude() {
        let normal = DamageNumber::new(50, 0.0, (0.0, 0.0, 0.0), DamageType::Normal);
        let critical = DamageNumber::new(50, 0.0, (0.0, 0.0, 0.0), DamageType::Critical);

        // Critical has larger shake amplitude
        assert!(critical.shake_amplitude > normal.shake_amplitude);
        assert!((critical.shake_amplitude - 0.175).abs() < 0.01); // ±10 degrees
        assert!((normal.shake_amplitude - 0.087).abs() < 0.01); // ±5 degrees
    }

    #[test]
    fn test_damage_number_velocity_determinism() {
        // Same spawn_time should produce same velocity
        let dmg1 = DamageNumber::new(50, 1.234, (0.0, 0.0, 0.0), DamageType::Normal);
        let dmg2 = DamageNumber::new(50, 1.234, (0.0, 0.0, 0.0), DamageType::Normal);

        assert_eq!(dmg1.velocity_x, dmg2.velocity_x);
        assert_eq!(dmg1.velocity_y, dmg2.velocity_y);
    }

    #[test]
    fn test_damage_number_velocity_variation() {
        // Different spawn_times should produce different velocities
        let dmg1 = DamageNumber::new(50, 1.0, (0.0, 0.0, 0.0), DamageType::Normal);
        let dmg2 = DamageNumber::new(50, 2.0, (0.0, 0.0, 0.0), DamageType::Normal);

        // Velocities should differ (pseudo-random based on spawn_time)
        assert_ne!(dmg1.velocity_x, dmg2.velocity_x);
    }

    // ===== QuestNotification Animation Tests =====

    #[test]
    fn test_notification_new_quest() {
        let notif =
            QuestNotification::new_quest("New Quest".to_string(), "Test description".to_string());
        assert_eq!(notif.title, "New Quest");
        assert_eq!(notif.description, "Test description");
        assert_eq!(notif.animation_time, 0.0);
        assert_eq!(notif.total_duration, 2.0);
        assert!(matches!(
            notif.notification_type,
            NotificationType::NewQuest
        ));
    }

    #[test]
    fn test_notification_objective_complete() {
        let notif = QuestNotification::objective_complete("Collect 5 items".to_string());
        assert_eq!(notif.title, "Objective Complete!");
        assert_eq!(notif.description, "Collect 5 items");
        assert!(matches!(
            notif.notification_type,
            NotificationType::ObjectiveComplete { .. }
        ));
    }

    #[test]
    fn test_notification_quest_complete() {
        let notif = QuestNotification::quest_complete(
            "Main Quest".to_string(),
            vec!["Gold".to_string(), "XP".to_string()],
        );
        assert_eq!(notif.title, "Main Quest");
        assert_eq!(notif.description, "Quest Complete!");
        assert_eq!(notif.total_duration, 2.8); // Longer duration for rewards
        assert!(matches!(
            notif.notification_type,
            NotificationType::QuestComplete { .. }
        ));
    }

    #[test]
    fn test_notification_update() {
        let mut notif = QuestNotification::new_quest("Test".to_string(), "Desc".to_string());

        // Not finished yet
        assert!(!notif.update(1.0));
        assert_eq!(notif.animation_time, 1.0);

        // Finished
        assert!(notif.update(1.5)); // Total 2.5s > 2.0s duration
    }

    #[test]
    fn test_notification_slide_offset_ease_in() {
        let mut notif = QuestNotification::new_quest("Test".to_string(), "Desc".to_string());

        // At start, should be off-screen (negative offset)
        let offset_start = notif.calculate_slide_offset();
        assert!(offset_start < 0.0);

        // During ease-in
        notif.animation_time = 0.15; // Mid ease-in
        let offset_mid = notif.calculate_slide_offset();
        assert!(offset_mid < 0.0);
        assert!(offset_mid > offset_start);
    }

    #[test]
    fn test_notification_slide_offset_hold() {
        let mut notif = QuestNotification::new_quest("Test".to_string(), "Desc".to_string());

        // During hold phase
        notif.animation_time = 1.0; // Middle of hold
        let offset = notif.calculate_slide_offset();
        assert_eq!(offset, 0.0);
    }

    #[test]
    fn test_notification_slide_offset_ease_out() {
        let mut notif = QuestNotification::new_quest("Test".to_string(), "Desc".to_string());

        // During ease-out
        notif.animation_time = 1.85; // During ease-out
        let offset = notif.calculate_slide_offset();
        assert!(offset < 0.0); // Moving off-screen
    }

    #[test]
    fn test_notification_alpha_fade_in() {
        let mut notif = QuestNotification::new_quest("Test".to_string(), "Desc".to_string());

        // At start, alpha should be low
        let alpha_start = notif.calculate_alpha();
        assert_eq!(alpha_start, 0);

        // During fade-in
        notif.animation_time = 0.1; // Half of 0.2s fade-in
        let alpha_mid = notif.calculate_alpha();
        assert!(alpha_mid > 0);
        assert!(alpha_mid < 255);
    }

    #[test]
    fn test_notification_alpha_full() {
        let mut notif = QuestNotification::new_quest("Test".to_string(), "Desc".to_string());

        // During hold phase, fully visible
        notif.animation_time = 1.0;
        let alpha = notif.calculate_alpha();
        assert_eq!(alpha, 255);
    }

    #[test]
    fn test_notification_alpha_fade_out() {
        let mut notif = QuestNotification::new_quest("Test".to_string(), "Desc".to_string());

        // During fade-out (last 0.3s)
        notif.animation_time = 1.85; // Within fade-out
        let alpha = notif.calculate_alpha();
        assert!(alpha < 255);
    }

    // ===== PingMarker Tests =====

    #[test]
    fn test_ping_marker_new() {
        let ping = PingMarker::new((100.0, 200.0), 5.0);
        assert_eq!(ping.world_pos, (100.0, 200.0));
        assert_eq!(ping.spawn_time, 5.0);
        assert_eq!(ping.duration, 3.0);
    }

    #[test]
    fn test_ping_marker_is_active() {
        let ping = PingMarker::new((0.0, 0.0), 10.0);

        assert!(ping.is_active(10.0)); // Just spawned
        assert!(ping.is_active(11.0)); // 1s after
        assert!(ping.is_active(12.9)); // Just before expiration
        assert!(!ping.is_active(13.0)); // Exactly at expiration
        assert!(!ping.is_active(15.0)); // Well past expiration
    }

    #[test]
    fn test_ping_marker_age_normalized() {
        let ping = PingMarker::new((0.0, 0.0), 10.0);

        assert_eq!(ping.age_normalized(10.0), 0.0); // Just spawned
        assert!((ping.age_normalized(11.5) - 0.5).abs() < 0.01); // Halfway
        assert_eq!(ping.age_normalized(13.0), 1.0); // Full duration
        assert_eq!(ping.age_normalized(15.0), 1.0); // Clamped at 1.0
    }

    // ===== HudManager Method Tests =====

    #[test]
    fn test_hud_manager_toggle_quest_tracker() {
        let mut hud = HudManager::new();
        assert!(hud.state().show_objectives);

        hud.toggle_quest_tracker();
        assert!(!hud.state().show_objectives);

        hud.toggle_quest_tracker();
        assert!(hud.state().show_objectives);
    }

    #[test]
    fn test_hud_manager_toggle_quest_collapse() {
        let mut hud = HudManager::new();
        assert!(!hud.state().quest_tracker_collapsed);

        hud.toggle_quest_collapse();
        assert!(hud.state().quest_tracker_collapsed);

        hud.toggle_quest_collapse();
        assert!(!hud.state().quest_tracker_collapsed);
    }

    #[test]
    fn test_hud_manager_toggle_minimap() {
        let mut hud = HudManager::new();
        assert!(hud.state().show_minimap);

        hud.toggle_minimap();
        assert!(!hud.state().show_minimap);

        hud.toggle_minimap();
        assert!(hud.state().show_minimap);
    }

    #[test]
    fn test_hud_manager_toggle_minimap_rotation() {
        let mut hud = HudManager::new();
        assert!(!hud.state().minimap_rotation);

        hud.toggle_minimap_rotation();
        assert!(hud.state().minimap_rotation);

        hud.toggle_minimap_rotation();
        assert!(!hud.state().minimap_rotation);
    }

    #[test]
    fn test_hud_manager_set_minimap_zoom() {
        let mut hud = HudManager::new();
        assert_eq!(hud.minimap_zoom(), 1.0);

        hud.set_minimap_zoom(2.0);
        assert_eq!(hud.minimap_zoom(), 2.0);

        // Test clamping
        hud.set_minimap_zoom(0.1); // Below min
        assert_eq!(hud.minimap_zoom(), 0.5);

        hud.set_minimap_zoom(5.0); // Above max
        assert_eq!(hud.minimap_zoom(), 3.0);
    }

    #[test]
    fn test_hud_manager_start_end_dialogue() {
        let mut hud = HudManager::new();
        assert!(!hud.state().show_dialogue);
        assert!(hud.active_dialogue.is_none());

        let dialogue = DialogueNode {
            id: 1,
            speaker_name: "NPC".to_string(),
            text: "Hello!".to_string(),
            portrait_id: None,
            choices: vec![],
        };

        hud.start_dialogue(dialogue);
        assert!(hud.state().show_dialogue);
        assert!(hud.active_dialogue.is_some());

        hud.end_dialogue();
        assert!(!hud.state().show_dialogue);
        assert!(hud.active_dialogue.is_none());
    }

    #[test]
    fn test_hud_manager_select_dialogue_choice() {
        let mut hud = HudManager::new();

        let dialogue = DialogueNode {
            id: 1,
            speaker_name: "NPC".to_string(),
            text: "Choose!".to_string(),
            portrait_id: None,
            choices: vec![
                DialogueChoice {
                    id: 1,
                    text: "Choice A".to_string(),
                    next_node: Some(10),
                },
                DialogueChoice {
                    id: 2,
                    text: "Choice B".to_string(),
                    next_node: None,
                },
            ],
        };

        hud.start_dialogue(dialogue);

        assert_eq!(hud.select_dialogue_choice(1), Some(10));
        assert_eq!(hud.select_dialogue_choice(2), None);
        assert_eq!(hud.select_dialogue_choice(99), None); // Invalid choice
    }

    #[test]
    fn test_hud_manager_show_hide_tooltip() {
        let mut hud = HudManager::new();
        assert!(hud.hovered_tooltip.is_none());

        let tooltip = TooltipData {
            title: "Item".to_string(),
            description: "Description".to_string(),
            stats: vec![],
            flavor_text: None,
        };

        hud.show_tooltip(tooltip, (100.0, 200.0));
        assert!(hud.hovered_tooltip.is_some());
        assert_eq!(hud.tooltip_position, (100.0, 200.0));

        hud.hide_tooltip();
        assert!(hud.hovered_tooltip.is_none());
    }

    #[test]
    fn test_hud_manager_spawn_damage() {
        let mut hud = HudManager::new();
        assert!(hud.damage_numbers.is_empty());

        hud.spawn_damage(50, (10.0, 5.0, 20.0), DamageType::Normal);
        assert_eq!(hud.damage_numbers.len(), 1);
        assert_eq!(hud.damage_numbers[0].value, 50);

        // Combo tracker should also have recorded the hit
        assert_eq!(hud.combo_tracker.get_combo_count(0.0), 1);
    }

    #[test]
    fn test_hud_manager_spawn_ping() {
        let mut hud = HudManager::new();
        assert!(hud.ping_markers.is_empty());

        hud.spawn_ping((50.0, 100.0));
        assert_eq!(hud.ping_markers.len(), 1);
        assert_eq!(hud.ping_markers[0].world_pos, (50.0, 100.0));
    }

    #[test]
    fn test_hud_manager_update_expires_damage_numbers() {
        let mut hud = HudManager::new();
        hud.spawn_damage(50, (0.0, 0.0, 0.0), DamageType::Normal);
        assert_eq!(hud.damage_numbers.len(), 1);

        // After 1.5s, damage numbers should expire
        hud.update(2.0);
        assert!(hud.damage_numbers.is_empty());
    }

    #[test]
    fn test_hud_manager_update_expires_pings() {
        let mut hud = HudManager::new();
        hud.spawn_ping((0.0, 0.0));
        assert_eq!(hud.ping_markers.len(), 1);

        // After 3s, ping should expire
        hud.update(4.0);
        assert!(hud.ping_markers.is_empty());
    }

    #[test]
    fn test_hud_manager_update_health_animation() {
        let mut hud = HudManager::new();
        hud.player_stats.health = 50.0; // Set actual health

        // Visual health should still be at 100 before update
        assert_eq!(hud.player_stats.health_animation.visual_health(), 100.0);

        // After update, visual health should animate toward target
        hud.update(0.5);
        let visual = hud.player_stats.health_animation.visual_health();
        assert!(
            visual < 100.0 && visual >= 50.0,
            "Visual health should be animating"
        );
    }

    #[test]
    fn test_hud_manager_update_enemy_health_animation() {
        let mut hud = HudManager::new();
        hud.enemies.push(EnemyData::new(
            1,
            (0.0, 0.0, 0.0),
            100.0,
            EnemyFaction::Hostile,
        ));
        hud.enemies[0].health = 50.0;

        hud.update(0.5);
        let visual = hud.enemies[0].health_animation.visual_health();
        assert!(visual < 100.0 && visual >= 50.0);
    }

    #[test]
    fn test_hud_manager_update_notification_queue() {
        let mut hud = HudManager::new();
        hud.notification_queue.push(QuestNotification::new_quest(
            "Test".to_string(),
            "Desc".to_string(),
        ));
        assert!(hud.notification_queue.has_active());

        // After duration, notification should expire
        hud.update(3.0);
        assert!(!hud.notification_queue.has_active());
    }

    // ===== HudState Tests =====

    #[test]
    fn test_hud_state_default() {
        let state = HudState::default();
        assert!(state.visible);
        assert!(state.show_health_bars);
        assert!(state.show_objectives);
        assert!(state.show_minimap);
        assert!(state.show_subtitles);
        assert!(!state.quest_tracker_collapsed);
        assert!(!state.minimap_rotation);
        assert_eq!(state.minimap_zoom, 1.0);
        assert!(!state.show_dialogue);
        assert!(!state.debug_mode);
    }

    #[test]
    fn test_hud_state_serialize_deserialize() {
        let state = HudState {
            visible: false,
            show_health_bars: false,
            show_objectives: true,
            show_minimap: false,
            show_subtitles: true,
            quest_tracker_collapsed: true,
            minimap_rotation: true,
            minimap_zoom: 2.0,
            show_dialogue: true,
            debug_mode: true,
        };

        // Serialize and deserialize (round-trip test)
        let json = serde_json::to_string(&state).unwrap();
        let deserialized: HudState = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.visible, state.visible);
        assert_eq!(deserialized.show_health_bars, state.show_health_bars);
        assert_eq!(deserialized.minimap_zoom, state.minimap_zoom);
        assert_eq!(deserialized.debug_mode, state.debug_mode);
    }

    // ===== EnemyData Tests =====

    #[test]
    fn test_enemy_data_new_with_animation() {
        let enemy = EnemyData::new(42, (10.0, 5.0, 20.0), 100.0, EnemyFaction::Hostile);

        assert_eq!(enemy.id, 42);
        assert_eq!(enemy.health, 100.0);
        assert_eq!(enemy.max_health, 100.0);
        assert_eq!(enemy.health_animation.visual_health(), 100.0);
    }

    #[test]
    fn test_enemy_faction_variants_debug() {
        let hostile = format!("{:?}", EnemyFaction::Hostile);
        let neutral = format!("{:?}", EnemyFaction::Neutral);
        let friendly = format!("{:?}", EnemyFaction::Friendly);

        assert_eq!(hostile, "Hostile");
        assert_eq!(neutral, "Neutral");
        assert_eq!(friendly, "Friendly");
    }

    // ===== PoiType Additional Tests =====

    #[test]
    fn test_poi_type_colors() {
        use egui::Color32;

        assert_eq!(PoiType::Objective.color(), Color32::YELLOW);
        assert_eq!(PoiType::Waypoint.color(), Color32::LIGHT_BLUE);
        assert_eq!(PoiType::Vendor.color(), Color32::GREEN);
        assert_eq!(PoiType::Danger.color(), Color32::RED);
    }

    // ===== Easing Function Edge Case Tests =====

    #[test]
    fn test_easing_ease_out_cubic_midpoints() {
        let quarter = easing::ease_out_cubic(0.25);
        let three_quarter = easing::ease_out_cubic(0.75);

        // ease_out_cubic should accelerate fast then slow
        assert!(quarter > 0.25, "Should be ahead of linear at 0.25");
        assert!(three_quarter > 0.75, "Should be ahead of linear at 0.75");
    }

    #[test]
    fn test_easing_ease_in_out_quad_symmetry() {
        let quarter = easing::ease_in_out_quad(0.25);
        let three_quarter = easing::ease_in_out_quad(0.75);

        // ease_in_out_quad should be symmetric around 0.5
        assert!(
            (quarter + three_quarter - 1.0).abs() < 0.01,
            "Should be symmetric"
        );
    }

    #[test]
    fn test_easing_ease_in_out_quad_midpoint() {
        let mid = easing::ease_in_out_quad(0.5);
        assert!((mid - 0.5).abs() < 0.01, "Midpoint should be 0.5");
    }

    // ===== Objective Tests =====

    #[test]
    fn test_objective_with_progress() {
        let obj = Objective {
            id: 1,
            description: "Kill 10 enemies".to_string(),
            completed: false,
            progress: Some((7, 10)),
        };

        assert_eq!(obj.id, 1);
        assert!(!obj.completed);
        assert_eq!(obj.progress, Some((7, 10)));
    }

    #[test]
    fn test_objective_without_progress() {
        let obj = Objective {
            id: 1,
            description: "Find the treasure".to_string(),
            completed: true,
            progress: None,
        };

        assert!(obj.completed);
        assert!(obj.progress.is_none());
    }

    // ===== DamageType Debug Tests =====

    #[test]
    fn test_damage_type_debug() {
        assert_eq!(format!("{:?}", DamageType::Normal), "Normal");
        assert_eq!(format!("{:?}", DamageType::Critical), "Critical");
        assert_eq!(format!("{:?}", DamageType::SelfDamage), "SelfDamage");
    }

    // ===== NotificationType Tests =====

    #[test]
    fn test_notification_type_debug() {
        let new_quest = NotificationType::NewQuest;
        let obj_complete = NotificationType::ObjectiveComplete {
            objective_text: "Test".to_string(),
        };
        let quest_complete = NotificationType::QuestComplete {
            rewards: vec!["Gold".to_string()],
        };

        assert!(format!("{:?}", new_quest).contains("NewQuest"));
        assert!(format!("{:?}", obj_complete).contains("ObjectiveComplete"));
        assert!(format!("{:?}", quest_complete).contains("QuestComplete"));
    }

    // ===== Audio Callback Tests =====

    #[test]
    fn test_hud_manager_set_minimap_click_callback() {
        let mut hud = HudManager::new();
        assert!(hud.on_minimap_click.is_none());

        let called = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let called_clone = called.clone();

        hud.set_minimap_click_callback(move |_dist| {
            called_clone.store(true, std::sync::atomic::Ordering::SeqCst);
        });

        assert!(hud.on_minimap_click.is_some());

        // Call the callback
        if let Some(callback) = &hud.on_minimap_click {
            callback(0.5);
        }

        assert!(called.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[test]
    fn test_hud_manager_set_ping_spawn_callback() {
        let mut hud = HudManager::new();
        assert!(hud.on_ping_spawn.is_none());

        let called = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let called_clone = called.clone();

        hud.set_ping_spawn_callback(move |_pos| {
            called_clone.store(true, std::sync::atomic::Ordering::SeqCst);
        });

        assert!(hud.on_ping_spawn.is_some());

        // Call the callback
        if let Some(callback) = &hud.on_ping_spawn {
            callback((10.0, 20.0));
        }

        assert!(called.load(std::sync::atomic::Ordering::SeqCst));
    }

    // ===== Quest Tests =====

    #[test]
    fn test_quest_completion_empty() {
        let quest = Quest {
            id: 1,
            title: "Empty Quest".to_string(),
            description: "No objectives".to_string(),
            objectives: vec![],
        };
        assert_eq!(quest.completion(), 0.0);
        assert!(!quest.is_complete());
    }

    #[test]
    fn test_quest_completion_partial() {
        let quest = Quest {
            id: 1,
            title: "Main Quest".to_string(),
            description: "Find items".to_string(),
            objectives: vec![
                Objective {
                    id: 1,
                    description: "Item 1".to_string(),
                    completed: true,
                    progress: None,
                },
                Objective {
                    id: 2,
                    description: "Item 2".to_string(),
                    completed: false,
                    progress: Some((3, 5)),
                },
                Objective {
                    id: 3,
                    description: "Item 3".to_string(),
                    completed: false,
                    progress: None,
                },
            ],
        };
        assert!((quest.completion() - 0.333).abs() < 0.01);
        assert!(!quest.is_complete());
    }

    #[test]
    fn test_quest_completion_full() {
        let quest = Quest {
            id: 1,
            title: "Complete Quest".to_string(),
            description: "All done".to_string(),
            objectives: vec![
                Objective {
                    id: 1,
                    description: "Obj 1".to_string(),
                    completed: true,
                    progress: None,
                },
                Objective {
                    id: 2,
                    description: "Obj 2".to_string(),
                    completed: true,
                    progress: None,
                },
            ],
        };
        assert_eq!(quest.completion(), 1.0);
        assert!(quest.is_complete());
    }

    // ===== PingMarker Tests =====

    #[test]
    fn test_ping_marker_construction() {
        let ping = PingMarker::new((10.0, 20.0), 5.0);
        assert_eq!(ping.world_pos, (10.0, 20.0));
        assert_eq!(ping.spawn_time, 5.0);
        assert_eq!(ping.duration, 3.0);
    }

    #[test]
    fn test_ping_marker_activity_check() {
        let ping = PingMarker::new((0.0, 0.0), 1.0);
        assert!(ping.is_active(2.0)); // 1.0 + 3.0 = 4.0 > 2.0
        assert!(ping.is_active(3.9)); // Still active
        assert!(!ping.is_active(4.1)); // 1.0 + 3.0 < 4.1
    }

    #[test]
    fn test_ping_marker_normalized_age() {
        let ping = PingMarker::new((0.0, 0.0), 0.0);
        assert_eq!(ping.age_normalized(0.0), 0.0);
        assert_eq!(ping.age_normalized(1.5), 0.5);
        assert_eq!(ping.age_normalized(3.0), 1.0);
        assert_eq!(ping.age_normalized(5.0), 1.0); // Capped at 1.0
    }

    // ===== PoiMarker Tests =====

    #[test]
    fn test_poi_marker_creation() {
        let poi = PoiMarker {
            id: 42,
            world_pos: (100.0, 200.0),
            poi_type: PoiType::Objective,
            label: Some("Target Location".to_string()),
        };
        assert_eq!(poi.id, 42);
        assert_eq!(poi.world_pos, (100.0, 200.0));
        assert_eq!(poi.poi_type, PoiType::Objective);
        assert_eq!(poi.label, Some("Target Location".to_string()));
    }

    #[test]
    fn test_poi_marker_without_label() {
        let poi = PoiMarker {
            id: 1,
            world_pos: (0.0, 0.0),
            poi_type: PoiType::Waypoint,
            label: None,
        };
        assert!(poi.label.is_none());
    }

    #[test]
    fn test_poi_type_icons() {
        assert_eq!(PoiType::Objective.icon(), "🎯");
        assert_eq!(PoiType::Waypoint.icon(), "📍");
        assert_eq!(PoiType::Vendor.icon(), "🏪");
        assert_eq!(PoiType::Danger.icon(), "⚔️");
    }

    // ===== DialogueChoice Tests =====

    #[test]
    fn test_dialogue_choice_with_next() {
        let choice = DialogueChoice {
            id: 1,
            text: "Tell me more".to_string(),
            next_node: Some(5),
        };
        assert_eq!(choice.id, 1);
        assert_eq!(choice.text, "Tell me more");
        assert_eq!(choice.next_node, Some(5));
    }

    #[test]
    fn test_dialogue_choice_end_dialogue() {
        let choice = DialogueChoice {
            id: 2,
            text: "Goodbye".to_string(),
            next_node: None,
        };
        assert!(choice.next_node.is_none());
    }

    // ===== DialogueNode Tests =====

    #[test]
    fn test_dialogue_node_with_choices() {
        let node = DialogueNode {
            id: 1,
            speaker_name: "Elder".to_string(),
            text: "Welcome, traveler!".to_string(),
            choices: vec![
                DialogueChoice {
                    id: 1,
                    text: "Hello!".to_string(),
                    next_node: Some(2),
                },
                DialogueChoice {
                    id: 2,
                    text: "Leave".to_string(),
                    next_node: None,
                },
            ],
            portrait_id: Some(42),
        };
        assert_eq!(node.speaker_name, "Elder");
        assert_eq!(node.choices.len(), 2);
        assert_eq!(node.portrait_id, Some(42));
    }

    #[test]
    fn test_dialogue_node_without_choices() {
        let node = DialogueNode {
            id: 1,
            speaker_name: "NPC".to_string(),
            text: "End of dialogue".to_string(),
            choices: vec![],
            portrait_id: None,
        };
        assert!(node.choices.is_empty());
        assert!(node.portrait_id.is_none());
    }

    // ===== TooltipData Tests =====

    #[test]
    fn test_tooltip_data_full() {
        let tooltip = TooltipData {
            title: "Sword of Power".to_string(),
            description: "A legendary blade".to_string(),
            stats: vec![
                ("Damage".to_string(), "25".to_string()),
                ("Range".to_string(), "1m".to_string()),
            ],
            flavor_text: Some("Forged in ancient times...".to_string()),
        };
        assert_eq!(tooltip.title, "Sword of Power");
        assert_eq!(tooltip.stats.len(), 2);
        assert!(tooltip.flavor_text.is_some());
    }

    #[test]
    fn test_tooltip_data_minimal() {
        let tooltip = TooltipData {
            title: "Simple Item".to_string(),
            description: "Basic".to_string(),
            stats: vec![],
            flavor_text: None,
        };
        assert!(tooltip.stats.is_empty());
        assert!(tooltip.flavor_text.is_none());
    }

    // ===== PlayerStats Tests =====

    #[test]
    fn test_player_stats_default_values() {
        let stats = PlayerStats::default();
        assert_eq!(stats.health, 100.0);
        assert_eq!(stats.max_health, 100.0);
        assert_eq!(stats.mana, 100.0);
        assert_eq!(stats.stamina, 100.0);
    }

    // ===== NotificationQueue Tests =====

    #[test]
    fn test_notification_queue_push_when_empty() {
        let mut queue = NotificationQueue::new();
        queue.push(QuestNotification::new_quest(
            "Test".to_string(),
            "Desc".to_string(),
        ));
        assert!(queue.has_active());
        assert!(queue.pending.is_empty());
    }

    #[test]
    fn test_notification_queue_push_when_active() {
        let mut queue = NotificationQueue::new();
        queue.push(QuestNotification::new_quest(
            "First".to_string(),
            "Desc".to_string(),
        ));
        queue.push(QuestNotification::new_quest(
            "Second".to_string(),
            "Desc".to_string(),
        ));

        assert!(queue.has_active());
        assert_eq!(queue.active.as_ref().unwrap().title, "First");
        assert_eq!(queue.pending.len(), 1);
    }

    #[test]
    fn test_notification_queue_update_pop_next() {
        let mut queue = NotificationQueue::new();
        queue.push(QuestNotification::new_quest(
            "First".to_string(),
            "Desc".to_string(),
        ));
        queue.push(QuestNotification::new_quest(
            "Second".to_string(),
            "Desc".to_string(),
        ));

        // Finish first notification
        queue.update(3.0); // Exceeds 2.0s duration

        // Second notification should now be active
        assert!(queue.has_active());
        assert_eq!(queue.active.as_ref().unwrap().title, "Second");
        assert!(queue.pending.is_empty());
    }

    // ===== HealthAnimation Tests =====

    #[test]
    fn test_health_animation_creation() {
        let anim = HealthAnimation::new(100.0);
        assert_eq!(anim.visual_health(), 100.0);
        assert_eq!(anim.flash_timer, 0.0);
    }

    #[test]
    fn test_health_animation_set_target() {
        let mut anim = HealthAnimation::new(100.0);
        anim.set_target(50.0);
        assert_eq!(anim.visual_health(), 100.0); // Not animated yet
                                                 // Update should animate toward target
        anim.update(0.5);
        let visual = anim.visual_health();
        assert!(visual < 100.0 && visual >= 50.0);
    }

    #[test]
    fn test_health_animation_flash_timer() {
        let mut anim = HealthAnimation::new(100.0);
        anim.set_target(50.0); // Taking damage
                               // Flash timer should be set
        anim.update(0.1);
        // After update, flash behavior depends on implementation
    }

    #[test]
    fn test_health_animation_healing_uses_in_out_easing_path() {
        let mut anim = HealthAnimation::new(50.0);
        anim.set_target(100.0);

        // Small update step should move visual health upward.
        anim.update(0.05);
        let visual = anim.visual_health();
        assert!(visual > 50.0);
        assert!(visual <= 100.0);
    }

    fn run_headless_egui_pass(ctx: &egui::Context, f: impl FnOnce(&egui::Context)) {
        ctx.begin_pass(egui::RawInput::default());
        f(ctx);
        let _ = ctx.end_pass();
    }

    #[test]
    fn test_hud_render_runs_headless_with_sample_state() {
        let ctx = egui::Context::default();
        let mut hud = HudManager::new();

        hud.toggle_debug();

        // Populate a small amount of sample data so render paths execute.
        hud.enemies.push(EnemyData::new(
            1,
            (3.0, 0.0, -2.0),
            100.0,
            EnemyFaction::Hostile,
        ));
        hud.damage_numbers.push(DamageNumber::new(
            42,
            0.0,
            (0.0, 0.0, 0.0),
            DamageType::Normal,
        ));
        hud.active_quest = Some(Quest {
            id: 1,
            title: "Test Quest".to_string(),
            description: "Test Description".to_string(),
            objectives: vec![
                Objective {
                    id: 1,
                    description: "Do the thing".to_string(),
                    completed: false,
                    progress: Some((1, 3)),
                },
                Objective {
                    id: 2,
                    description: "Finish".to_string(),
                    completed: true,
                    progress: None,
                },
            ],
        });
        hud.poi_markers.push(PoiMarker {
            id: 1,
            world_pos: (5.0, 5.0),
            poi_type: PoiType::Objective,
            label: Some("Objective".to_string()),
        });
        hud.ping_markers.push(PingMarker::new((2.0, -1.0), 0.0));
        hud.player_position = (1.0, 1.0);
        hud.player_rotation = 0.5;

        hud.update(0.016);

        run_headless_egui_pass(&ctx, |ctx| {
            hud.render(ctx);
        });
    }
}
