// Default implementations for test scaffolding
impl Default for PlayerState {
    fn default() -> Self {
        PlayerState {
            hp: 100,
            pos: IVec2 { x: 0, y: 0 },
            stance: Stance::Stand,
            orders: vec![],
        }
    }
}

impl Default for CompanionState {
    fn default() -> Self {
        CompanionState {
            ammo: 10,
            cooldowns: BTreeMap::new(),
            morale: 1.0,
            pos: IVec2 { x: 0, y: 0 },
        }
    }
}

impl Default for EnemyState {
    fn default() -> Self {
        EnemyState {
            id: 0,
            pos: IVec2 { x: 0, y: 0 },
            hp: 100,
            cover: CoverType::None,
            last_seen: 0.0,
        }
    }
}

impl Default for Poi {
    fn default() -> Self {
        Poi {
            k: "poi".to_string(),
            pos: IVec2 { x: 0, y: 0 },
        }
    }
}

impl Default for WorldSnapshot {
    fn default() -> Self {
        WorldSnapshot {
            t: 0.0,
            player: PlayerState::default(),
            me: CompanionState::default(),
            enemies: vec![],
            pois: vec![],
            obstacles: vec![],
            objective: None,
        }
    }
}
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

pub type Entity = u32;

// ============================================================================
// Zero-cost enums replacing per-snapshot String allocations
// ============================================================================

/// Character stance — replaces `stance: String` to eliminate per-snapshot allocation.
/// Serializes to the same JSON strings for backward compatibility.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stance {
    Stand,
    Crouch,
    Prone,
}

impl Default for Stance {
    fn default() -> Self {
        Self::Stand
    }
}

impl fmt::Display for Stance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Stand => write!(f, "stand"),
            Self::Crouch => write!(f, "crouch"),
            Self::Prone => write!(f, "prone"),
        }
    }
}

impl From<&str> for Stance {
    fn from(s: &str) -> Self {
        match s {
            "crouch" => Self::Crouch,
            "prone" => Self::Prone,
            _ => Self::Stand,
        }
    }
}

impl From<String> for Stance {
    fn from(s: String) -> Self {
        Self::from(s.as_str())
    }
}

/// Cover level for enemies — replaces `cover: String` to eliminate per-snapshot allocation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoverType {
    None,
    Low,
    High,
    Wall,
    Unknown,
    Partial,
}

impl Default for CoverType {
    fn default() -> Self {
        Self::None
    }
}

impl fmt::Display for CoverType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::None => write!(f, "none"),
            Self::Low => write!(f, "low"),
            Self::High => write!(f, "high"),
            Self::Wall => write!(f, "wall"),
            Self::Unknown => write!(f, "unknown"),
            Self::Partial => write!(f, "partial"),
        }
    }
}

impl From<&str> for CoverType {
    fn from(s: &str) -> Self {
        match s {
            "low" => Self::Low,
            "high" => Self::High,
            "wall" => Self::Wall,
            "unknown" => Self::Unknown,
            "partial" => Self::Partial,
            _ => Self::None,
        }
    }
}

impl From<String> for CoverType {
    fn from(s: String) -> Self {
        Self::from(s.as_str())
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
/// A 2D integer vector used for grid-based positions.
///
/// # Examples
///
/// ```
/// use astraweave_core::IVec2;
///
/// let a = IVec2::new(3, 4);
/// let b = IVec2::new(6, 8);
/// assert_eq!(a.manhattan_distance(&b), 7);
/// assert_eq!(a + b, IVec2::new(9, 12));
/// ```
pub struct IVec2 {
    pub x: i32,
    pub y: i32,
}

impl IVec2 {
    /// Convenience constructor matching glam-style APIs.
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// Returns the zero vector.
    pub const fn zero() -> Self {
        Self { x: 0, y: 0 }
    }

    /// Returns `true` if both components are zero.
    pub fn is_zero(&self) -> bool {
        self.x == 0 && self.y == 0
    }

    /// Returns the Manhattan distance to another point.
    pub fn manhattan_distance(&self, other: &Self) -> i32 {
        (self.x - other.x).abs() + (self.y - other.y).abs()
    }

    /// Returns the squared Euclidean distance to another point.
    pub fn distance_squared(&self, other: &Self) -> i32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        dx * dx + dy * dy
    }

    /// Returns the Euclidean distance to another point.
    pub fn distance(&self, other: &Self) -> f32 {
        (self.distance_squared(other) as f32).sqrt()
    }

    /// Returns a new vector offset by the given amounts.
    pub fn offset(&self, dx: i32, dy: i32) -> Self {
        Self {
            x: self.x + dx,
            y: self.y + dy,
        }
    }
}

impl std::fmt::Display for IVec2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

impl std::ops::Add for IVec2 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl std::ops::Sub for IVec2 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
/// A snapshot of the game world at a point in time, used for AI perception.
///
/// `WorldSnapshot` is the primary input to all AI orchestrators. It captures
/// the player, companion, enemies, and points of interest.
///
/// # Examples
///
/// ```
/// use astraweave_core::{WorldSnapshot, PlayerState, CompanionState, IVec2};
/// use std::collections::BTreeMap;
///
/// let snap = WorldSnapshot {
///     t: 1.0,
///     player: PlayerState { hp: 100, pos: IVec2::new(0, 0), stance: "stand".into(), orders: vec![] },
///     me: CompanionState { ammo: 30, cooldowns: BTreeMap::new(), morale: 1.0, pos: IVec2::new(1, 1) },
///     enemies: vec![],
///     pois: vec![],
///     obstacles: vec![],
///     objective: Some("patrol".into()),
/// };
/// assert!(snap.has_no_enemies());
/// assert!(snap.has_objective());
/// ```
pub struct WorldSnapshot {
    pub t: f32,
    pub player: PlayerState,
    pub me: CompanionState,
    pub enemies: Vec<EnemyState>,
    pub pois: Vec<Poi>,
    pub obstacles: Vec<IVec2>,
    pub objective: Option<String>,
}

impl WorldSnapshot {
    /// Returns the number of enemies in the snapshot.
    pub fn enemy_count(&self) -> usize {
        self.enemies.len()
    }

    /// Returns `true` if there are no enemies.
    pub fn has_no_enemies(&self) -> bool {
        self.enemies.is_empty()
    }

    /// Returns the nearest enemy to the companion's position.
    pub fn nearest_enemy(&self) -> Option<&EnemyState> {
        self.enemies
            .iter()
            .min_by_key(|e| self.me.pos.distance_squared(&e.pos))
    }

    /// Returns enemies within the given Manhattan distance from the companion.
    pub fn enemies_within_range(&self, range: i32) -> Vec<&EnemyState> {
        self.enemies
            .iter()
            .filter(|e| self.me.pos.manhattan_distance(&e.pos) <= range)
            .collect()
    }

    /// Returns `true` if the companion has ammo.
    pub fn has_ammo(&self) -> bool {
        self.me.ammo > 0
    }

    /// Returns `true` if there are any points of interest.
    pub fn has_pois(&self) -> bool {
        !self.pois.is_empty()
    }

    /// Returns `true` if there is an objective.
    pub fn has_objective(&self) -> bool {
        self.objective.is_some()
    }

    /// Returns the distance from the companion to the player.
    pub fn distance_to_player(&self) -> f32 {
        self.me.pos.distance(&self.player.pos)
    }
}

impl std::fmt::Display for WorldSnapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "WorldSnapshot(t={:.1}s, {} enemies, {} POIs)",
            self.t,
            self.enemies.len(),
            self.pois.len()
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayerState {
    pub hp: i32,
    pub pos: IVec2,
    pub stance: Stance,
    pub orders: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompanionState {
    pub ammo: i32,
    pub cooldowns: BTreeMap<String, f32>,
    pub morale: f32,
    pub pos: IVec2,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EnemyState {
    pub id: Entity,
    pub pos: IVec2,
    pub hp: i32,
    pub cover: CoverType,
    pub last_seen: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Poi {
    pub k: String,
    pub pos: IVec2,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PhysicsContext {
    pub blocking_objects: Vec<BlockingObject>,
    pub interactable_objects: Vec<InteractableObject>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockingObject {
    pub id: Entity,
    pub pos: IVec2,
    pub object_type: String,
    pub is_locked: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InteractableObject {
    pub id: Entity,
    pub pos: IVec2,
    pub object_type: String,
    pub requires_item: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq)]
/// An AI plan consisting of ordered action steps.
///
/// Created by orchestrators and consumed by the tool sandbox for validation
/// and execution. Use the builder pattern with [`PlanIntent::with_step`].
///
/// # Examples
///
/// ```
/// use astraweave_core::{PlanIntent, ActionStep};
///
/// let plan = PlanIntent::new("assault-plan")
///     .with_step(ActionStep::MoveTo { x: 5, y: 3, speed: None })
///     .with_step(ActionStep::Attack { target_id: 42 });
///
/// assert_eq!(plan.step_count(), 2);
/// assert!(!plan.is_empty());
/// ```
pub struct PlanIntent {
    pub plan_id: String,
    pub steps: Vec<ActionStep>,
}

impl PlanIntent {
    /// Creates an empty plan with the given ID.
    pub fn new(plan_id: impl Into<String>) -> Self {
        Self {
            plan_id: plan_id.into(),
            steps: Vec::new(),
        }
    }

    /// Creates an empty plan with no ID.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Returns `true` if the plan has no steps.
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// Returns the number of steps in the plan.
    pub fn step_count(&self) -> usize {
        self.steps.len()
    }

    /// Adds a step to the plan and returns self.
    pub fn with_step(mut self, step: ActionStep) -> Self {
        self.steps.push(step);
        self
    }

    /// Returns the first step, if any.
    pub fn first_step(&self) -> Option<&ActionStep> {
        self.steps.first()
    }

    /// Returns `true` if the plan contains any movement actions.
    pub fn has_movement(&self) -> bool {
        self.steps.iter().any(|s| s.is_movement())
    }

    /// Returns `true` if the plan contains any offensive actions.
    pub fn has_offensive(&self) -> bool {
        self.steps.iter().any(|s| s.is_offensive())
    }
}

impl std::fmt::Display for PlanIntent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.plan_id.is_empty() {
            write!(f, "PlanIntent({} steps)", self.steps.len())
        } else {
            write!(
                f,
                "PlanIntent('{}', {} steps)",
                self.plan_id,
                self.steps.len()
            )
        }
    }
}

// ============================================================================
// MOVEMENT SPEED & DIRECTION ENUMS
// ============================================================================

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum MovementSpeed {
    Walk,
    Run,
    Sprint,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum StrafeDirection {
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum AttackType {
    Light,
    Heavy,
}

// ============================================================================
// TERRAIN GENERATION DSL - AI-Orchestrated Dynamic Terrain (Phase 10)
// ============================================================================

/// Terrain feature types for LLM-driven generation
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
#[non_exhaustive]
pub enum TerrainFeatureType {
    /// Underground cave system with specified depth
    Cave { depth: u32 },
    /// Impact crater with specified radius
    Crater { radius: u32 },
    /// Vertical cliff face with specified height
    Cliff { height: u32 },
    /// Body of water with specified depth
    Lake { depth: u32 },
    /// Dense vegetation area with specified density (0.0-1.0)
    Forest { density: f32 },
    /// Ancient ruins with specified complexity level (1-5)
    Ruins { complexity: u32 },
    /// Custom feature type with arbitrary parameters
    Custom {
        feature_id: String,
        params: std::collections::HashMap<String, f32>,
    },
}

impl Default for TerrainFeatureType {
    fn default() -> Self {
        TerrainFeatureType::Crater { radius: 10 }
    }
}

/// Cardinal direction for spatial references
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum CardinalDirection {
    North,
    South,
    East,
    West,
    NorthEast,
    NorthWest,
    SouthEast,
    SouthWest,
}

impl CardinalDirection {
    /// Convert to a unit vector (x, z) on the horizontal plane
    pub fn to_unit_vector(self) -> (f32, f32) {
        match self {
            CardinalDirection::North => (0.0, -1.0),
            CardinalDirection::South => (0.0, 1.0),
            CardinalDirection::East => (1.0, 0.0),
            CardinalDirection::West => (-1.0, 0.0),
            CardinalDirection::NorthEast => (0.707, -0.707),
            CardinalDirection::NorthWest => (-0.707, -0.707),
            CardinalDirection::SouthEast => (0.707, 0.707),
            CardinalDirection::SouthWest => (-0.707, 0.707),
        }
    }
}

/// Distance categories to prevent LLM spatial hallucination
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum DistanceCategory {
    /// 10-50 units from reference point
    Near,
    /// 50-150 units from reference point
    Medium,
    /// 150-500 units from reference point
    Far,
}

impl DistanceCategory {
    /// Get the actual distance range (min, max) in world units
    pub fn to_range(self) -> (f32, f32) {
        match self {
            DistanceCategory::Near => (10.0, 50.0),
            DistanceCategory::Medium => (50.0, 150.0),
            DistanceCategory::Far => (150.0, 500.0),
        }
    }

    /// Get the midpoint distance for this category
    pub fn midpoint(self) -> f32 {
        let (min, max) = self.to_range();
        (min + max) / 2.0
    }
}

/// Relative location for LLM-friendly spatial references
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "method")]
#[non_exhaustive]
pub enum RelativeLocation {
    /// Place at the point the camera/player is looking at
    LineOfSight {
        /// Maximum raycast distance
        look_distance: f32,
    },
    /// Place in a direction from the current position
    DirectionFrom {
        /// Cardinal direction
        cardinal: CardinalDirection,
        /// Distance category
        distance: DistanceCategory,
    },
    /// Explicit world coordinates (fallback)
    Coordinates { x: f32, y: f32, z: f32 },
}

impl Default for RelativeLocation {
    fn default() -> Self {
        RelativeLocation::LineOfSight {
            look_distance: 50.0,
        }
    }
}

/// Persistence mode for terrain modifications
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum PersistenceMode {
    /// Terrain changes are lost when the session ends
    #[default]
    SessionOnly,
    /// Terrain changes are saved to disk and persist across sessions
    Persistent,
}

/// LLM terrain generation request (DSL for Hermes 2 Pro)
///
/// This struct represents a complete terrain modification request that can be
/// generated by an LLM and validated by the TerrainSolver before execution.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TerrainGenerationRequest {
    /// Unique identifier for this request (UUID v4)
    pub request_id: String,
    /// Type of terrain feature to generate
    pub feature_type: TerrainFeatureType,
    /// Where to place the feature relative to the player/camera
    pub relative_location: RelativeLocation,
    /// Intensity of the modification (0.0 = subtle, 1.0 = dramatic)
    pub intensity: f32,
    /// Narrative justification for the terrain change (max 100 chars)
    pub narrative_reason: String,
    /// Whether this change should persist across sessions
    pub persistence_mode: PersistenceMode,
    /// Biome constraints - if non-empty, feature can only spawn in these biomes
    pub biome_constraints: Vec<String>,
    /// Optional seed for deterministic generation and replay validation
    pub seed: Option<u64>,
}

impl Default for TerrainGenerationRequest {
    fn default() -> Self {
        TerrainGenerationRequest {
            request_id: String::new(),
            feature_type: TerrainFeatureType::default(),
            relative_location: RelativeLocation::default(),
            intensity: 0.5,
            narrative_reason: String::new(),
            persistence_mode: PersistenceMode::default(),
            biome_constraints: Vec::new(),
            seed: None,
        }
    }
}

impl TerrainGenerationRequest {
    /// Validate the request before processing
    pub fn validate(&self) -> Result<(), String> {
        // Validate intensity bounds
        if self.intensity < 0.0 || self.intensity > 1.0 {
            return Err("Intensity must be between 0.0 and 1.0".to_string());
        }

        // Validate narrative reason length
        if self.narrative_reason.len() > 100 {
            return Err("Narrative reason exceeds 100 characters".to_string());
        }

        // Validate request_id is not empty
        if self.request_id.is_empty() {
            return Err("Request ID cannot be empty".to_string());
        }

        Ok(())
    }
}

// ============================================================================
// ACTION STEP ENUM - 38 Tools Across 7 Categories (includes ModifyTerrain)
// ============================================================================

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "act")]
#[non_exhaustive]
/// A single action in an AI plan.
///
/// `ActionStep` represents the 37-tool vocabulary available to AI agents.
/// Each variant maps to a validated, executable engine action.
///
/// # Examples
///
/// ```
/// use astraweave_core::ActionStep;
///
/// let step = ActionStep::MoveTo { x: 10, y: 20, speed: None };
/// assert!(matches!(step, ActionStep::MoveTo { .. }));
///
/// let attack = ActionStep::Attack { target_id: 7 };
/// assert!(matches!(attack, ActionStep::Attack { target_id: 7 }));
/// ```
pub enum ActionStep {
    // ═══════════════════════════════════════
    // MOVEMENT (6 tools)
    // ═══════════════════════════════════════
    /// Move to a specific position
    MoveTo {
        x: i32,
        y: i32,
        #[serde(default)]
        speed: Option<MovementSpeed>,
    },

    /// Move toward target entity while maintaining distance
    Approach {
        target_id: Entity,
        /// Desired distance (e.g., melee=2, ranged=15)
        distance: f32,
    },

    /// Move away from target entity
    Retreat {
        target_id: Entity,
        /// Safe distance to reach
        distance: f32,
    },

    /// Take cover behind nearest obstacle
    TakeCover {
        /// Optional: specific cover position
        position: Option<IVec2>,
    },

    /// Strafe around target (circle)
    Strafe {
        target_id: Entity,
        direction: StrafeDirection,
    },

    /// Patrol between waypoints
    Patrol { waypoints: Vec<IVec2> },

    // ═══════════════════════════════════════
    // OFFENSIVE (8 tools)
    // ═══════════════════════════════════════
    /// Basic attack targeting entity
    Attack { target_id: Entity },

    /// Aimed shot with higher accuracy
    AimedShot { target_id: Entity },

    /// Quick attack with lower damage
    QuickAttack { target_id: Entity },

    /// Heavy attack with higher damage
    HeavyAttack { target_id: Entity },

    /// Area-of-effect attack
    AoEAttack { x: i32, y: i32, radius: f32 },

    /// Throw explosive (grenade, etc.)
    ThrowExplosive { x: i32, y: i32 },

    /// Suppressive covering fire
    CoverFire { target_id: Entity, duration: f32 },

    /// Charge at target
    Charge { target_id: Entity },

    // ═══════════════════════════════════════
    // DEFENSIVE (6 tools)
    // ═══════════════════════════════════════
    /// Block incoming attack
    Block,

    /// Dodge attack
    Dodge { direction: Option<StrafeDirection> },

    /// Parry incoming attack
    Parry,

    /// Throw smoke grenade
    ThrowSmoke { x: i32, y: i32 },

    /// Heal self or ally
    Heal { target_id: Option<Entity> },

    /// Use defensive ability
    UseDefensiveAbility { ability_name: String },

    // ═══════════════════════════════════════
    // EQUIPMENT (5 tools)
    // ═══════════════════════════════════════
    /// Equip weapon
    EquipWeapon { weapon_name: String },

    /// Switch to different weapon
    SwitchWeapon { slot: u32 },

    /// Reload current weapon
    Reload,

    /// Use item from inventory
    UseItem { item_name: String },

    /// Drop item
    DropItem { item_name: String },

    // ═══════════════════════════════════════
    // TACTICAL (7 tools)
    // ═══════════════════════════════════════
    /// Call for reinforcements
    CallReinforcements { count: u32 },

    /// Mark target for allies
    MarkTarget { target_id: Entity },

    /// Request covering fire
    RequestCover { duration: f32 },

    /// Coordinate attack with allies
    CoordinateAttack { target_id: Entity },

    /// Set up ambush
    SetAmbush { position: IVec2 },

    /// Distract enemy
    Distract { target_id: Entity },

    /// Regroup with allies
    Regroup { rally_point: IVec2 },

    // ═══════════════════════════════════════
    // UTILITY (5 tools)
    // ═══════════════════════════════════════
    /// Scan area for threats
    Scan { radius: f32 },

    /// Wait for duration
    Wait { duration: f32 },

    /// Interact with object
    Interact { target_id: Entity },

    /// Use special ability
    UseAbility { ability_name: String },

    /// Taunt enemy
    Taunt { target_id: Entity },

    // ═══════════════════════════════════════
    // LEGACY (kept for backward compatibility)
    // ═══════════════════════════════════════
    /// Generic throw (now use ThrowSmoke or ThrowExplosive)
    Throw { item: String, x: i32, y: i32 },

    /// Revive ally
    Revive { ally_id: Entity },

    // ═══════════════════════════════════════
    // TERRAIN (1 tool) - AI-Orchestrated Dynamic Terrain
    // ═══════════════════════════════════════
    /// Modify terrain at a location (LLM-driven, validated by TerrainSolver)
    ModifyTerrain {
        /// Unique request identifier
        request_id: String,
        /// Full terrain generation request payload
        payload: TerrainGenerationRequest,
    },
}

impl ActionStep {
    /// Returns the category name for this action (Movement, Offensive, etc.).
    pub fn category(&self) -> &'static str {
        match self {
            Self::MoveTo { .. }
            | Self::Approach { .. }
            | Self::Retreat { .. }
            | Self::TakeCover { .. }
            | Self::Strafe { .. }
            | Self::Patrol { .. } => "Movement",

            Self::Attack { .. }
            | Self::AimedShot { .. }
            | Self::QuickAttack { .. }
            | Self::HeavyAttack { .. }
            | Self::AoEAttack { .. }
            | Self::ThrowExplosive { .. }
            | Self::CoverFire { .. }
            | Self::Charge { .. } => "Offensive",

            Self::Block
            | Self::Dodge { .. }
            | Self::Parry
            | Self::ThrowSmoke { .. }
            | Self::Heal { .. }
            | Self::UseDefensiveAbility { .. } => "Defensive",

            Self::EquipWeapon { .. }
            | Self::SwitchWeapon { .. }
            | Self::Reload
            | Self::UseItem { .. }
            | Self::DropItem { .. } => "Equipment",

            Self::CallReinforcements { .. }
            | Self::MarkTarget { .. }
            | Self::RequestCover { .. }
            | Self::CoordinateAttack { .. }
            | Self::SetAmbush { .. }
            | Self::Distract { .. }
            | Self::Regroup { .. } => "Tactical",

            Self::Scan { .. }
            | Self::Wait { .. }
            | Self::Interact { .. }
            | Self::UseAbility { .. }
            | Self::Taunt { .. } => "Utility",

            Self::Throw { .. } | Self::Revive { .. } => "Legacy",

            Self::ModifyTerrain { .. } => "Terrain",
        }
    }

    /// Returns the action name as a string (e.g., "MoveTo", "Attack").
    pub fn action_name(&self) -> &'static str {
        match self {
            Self::MoveTo { .. } => "MoveTo",
            Self::Approach { .. } => "Approach",
            Self::Retreat { .. } => "Retreat",
            Self::TakeCover { .. } => "TakeCover",
            Self::Strafe { .. } => "Strafe",
            Self::Patrol { .. } => "Patrol",
            Self::Attack { .. } => "Attack",
            Self::AimedShot { .. } => "AimedShot",
            Self::QuickAttack { .. } => "QuickAttack",
            Self::HeavyAttack { .. } => "HeavyAttack",
            Self::AoEAttack { .. } => "AoEAttack",
            Self::ThrowExplosive { .. } => "ThrowExplosive",
            Self::CoverFire { .. } => "CoverFire",
            Self::Charge { .. } => "Charge",
            Self::Block => "Block",
            Self::Dodge { .. } => "Dodge",
            Self::Parry => "Parry",
            Self::ThrowSmoke { .. } => "ThrowSmoke",
            Self::Heal { .. } => "Heal",
            Self::UseDefensiveAbility { .. } => "UseDefensiveAbility",
            Self::EquipWeapon { .. } => "EquipWeapon",
            Self::SwitchWeapon { .. } => "SwitchWeapon",
            Self::Reload => "Reload",
            Self::UseItem { .. } => "UseItem",
            Self::DropItem { .. } => "DropItem",
            Self::CallReinforcements { .. } => "CallReinforcements",
            Self::MarkTarget { .. } => "MarkTarget",
            Self::RequestCover { .. } => "RequestCover",
            Self::CoordinateAttack { .. } => "CoordinateAttack",
            Self::SetAmbush { .. } => "SetAmbush",
            Self::Distract { .. } => "Distract",
            Self::Regroup { .. } => "Regroup",
            Self::Scan { .. } => "Scan",
            Self::Wait { .. } => "Wait",
            Self::Interact { .. } => "Interact",
            Self::UseAbility { .. } => "UseAbility",
            Self::Taunt { .. } => "Taunt",
            Self::Throw { .. } => "Throw",
            Self::Revive { .. } => "Revive",
            Self::ModifyTerrain { .. } => "ModifyTerrain",
        }
    }

    /// Returns `true` if this is a movement action.
    pub fn is_movement(&self) -> bool {
        self.category() == "Movement"
    }

    /// Returns `true` if this is an offensive action.
    pub fn is_offensive(&self) -> bool {
        self.category() == "Offensive"
    }

    /// Returns `true` if this is a defensive action.
    pub fn is_defensive(&self) -> bool {
        self.category() == "Defensive"
    }

    /// Returns `true` if this is an equipment action.
    pub fn is_equipment(&self) -> bool {
        self.category() == "Equipment"
    }

    /// Returns `true` if this is a tactical action.
    pub fn is_tactical(&self) -> bool {
        self.category() == "Tactical"
    }

    /// Returns `true` if this is a utility action.
    pub fn is_utility(&self) -> bool {
        self.category() == "Utility"
    }

    /// Returns `true` if this targets a specific entity.
    pub fn targets_entity(&self) -> bool {
        matches!(
            self,
            Self::Approach { .. }
                | Self::Retreat { .. }
                | Self::Strafe { .. }
                | Self::Attack { .. }
                | Self::AimedShot { .. }
                | Self::QuickAttack { .. }
                | Self::HeavyAttack { .. }
                | Self::CoverFire { .. }
                | Self::Charge { .. }
                | Self::MarkTarget { .. }
                | Self::CoordinateAttack { .. }
                | Self::Distract { .. }
                | Self::Interact { .. }
                | Self::Taunt { .. }
                | Self::Revive { .. }
        )
    }

    /// Returns the target entity ID if this action targets one.
    pub fn target_entity(&self) -> Option<Entity> {
        match self {
            Self::Approach { target_id, .. }
            | Self::Retreat { target_id, .. }
            | Self::Strafe { target_id, .. }
            | Self::Attack { target_id }
            | Self::AimedShot { target_id }
            | Self::QuickAttack { target_id }
            | Self::HeavyAttack { target_id }
            | Self::CoverFire { target_id, .. }
            | Self::Charge { target_id }
            | Self::MarkTarget { target_id }
            | Self::CoordinateAttack { target_id }
            | Self::Distract { target_id }
            | Self::Interact { target_id }
            | Self::Taunt { target_id } => Some(*target_id),
            Self::Revive { ally_id } => Some(*ally_id),
            Self::Heal { target_id } => *target_id,
            _ => None,
        }
    }

    /// Returns `true` if this action has a position component.
    pub fn has_position(&self) -> bool {
        matches!(
            self,
            Self::MoveTo { .. }
                | Self::TakeCover {
                    position: Some(_),
                    ..
                }
                | Self::AoEAttack { .. }
                | Self::ThrowExplosive { .. }
                | Self::ThrowSmoke { .. }
                | Self::SetAmbush { .. }
                | Self::Regroup { .. }
                | Self::Throw { .. }
        )
    }
}

impl std::fmt::Display for ActionStep {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}::{}", self.category(), self.action_name())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolSpec {
    pub name: String,
    pub args: BTreeMap<String, String>, // k: name, v: type ("i32","f32","enum[...]")
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolRegistry {
    pub tools: Vec<ToolSpec>,
    pub constraints: Constraints,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Constraints {
    pub enforce_cooldowns: bool,
    pub enforce_los: bool,
    pub enforce_stamina: bool,
}

#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
#[must_use]
pub enum EngineError {
    #[error("invalid action: {0}")]
    InvalidAction(String),
    #[error("cooldown blocked: {0}")]
    Cooldown(String),
    #[error("line of sight blocked")]
    LosBlocked,
    #[error("path not found")]
    NoPath,
    #[error("resource missing: {0}")]
    Resource(String),
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Rect {
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "op")]
#[non_exhaustive]
pub enum DirectorOp {
    Fortify {
        rect: Rect,
    }, // add obstacles
    SpawnWave {
        archetype: String,
        count: u32,
        origin: IVec2,
    },
    Collapse {
        a: IVec2,
        b: IVec2,
    }, // line of obstacles ("bridge down")
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DirectorBudget {
    pub traps: i32,
    pub terrain_edits: i32,
    pub spawns: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DirectorPlan {
    pub ops: Vec<DirectorOp>,
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;

    #[test]
    fn test_ivec2_default() {
        let v = IVec2::default();
        assert_eq!(v.x, 0);
        assert_eq!(v.y, 0);
    }

    #[test]
    fn test_ivec2_equality() {
        let v1 = IVec2 { x: 5, y: 10 };
        let v2 = IVec2 { x: 5, y: 10 };
        let v3 = IVec2 { x: 3, y: 10 };
        assert_eq!(v1, v2);
        assert_ne!(v1, v3);
    }

    #[test]
    fn test_player_state_default() {
        let player = PlayerState::default();
        assert_eq!(player.hp, 100);
        assert_eq!(player.pos, IVec2 { x: 0, y: 0 });
        assert_eq!(player.stance, Stance::Stand);
        assert!(player.orders.is_empty());
    }

    #[test]
    fn test_companion_state_default() {
        let companion = CompanionState::default();
        assert_eq!(companion.ammo, 10);
        assert_eq!(companion.morale, 1.0);
        assert_eq!(companion.pos, IVec2 { x: 0, y: 0 });
        assert!(companion.cooldowns.is_empty());
    }

    #[test]
    fn test_enemy_state_default() {
        let enemy = EnemyState::default();
        assert_eq!(enemy.id, 0);
        assert_eq!(enemy.hp, 100);
        assert_eq!(enemy.cover, CoverType::None);
        assert_eq!(enemy.last_seen, 0.0);
    }

    #[test]
    fn test_poi_default() {
        let poi = Poi::default();
        assert_eq!(poi.k, "poi");
        assert_eq!(poi.pos, IVec2 { x: 0, y: 0 });
    }

    #[test]
    fn test_world_snapshot_default() {
        let snapshot = WorldSnapshot::default();
        assert_eq!(snapshot.t, 0.0);
        assert!(snapshot.enemies.is_empty());
        assert!(snapshot.pois.is_empty());
        assert!(snapshot.obstacles.is_empty());
        assert!(snapshot.objective.is_none());
        assert!(snapshot.objective.is_none());
    }

    #[test]
    fn test_plan_intent_default() {
        let intent = PlanIntent::default();
        assert!(intent.plan_id.is_empty());
        assert!(intent.steps.is_empty());
    }

    #[test]
    fn test_movement_speed_serde() {
        let walk = MovementSpeed::Walk;
        let json = serde_json::to_string(&walk).unwrap();
        assert_eq!(json, "\"walk\"");

        let deserialized: MovementSpeed = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, MovementSpeed::Walk);
    }

    #[test]
    fn test_strafe_direction_serde() {
        let left = StrafeDirection::Left;
        let json = serde_json::to_string(&left).unwrap();
        assert_eq!(json, "\"left\"");

        let deserialized: StrafeDirection = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, StrafeDirection::Left);
    }

    #[test]
    fn test_attack_type_serde() {
        let heavy = AttackType::Heavy;
        let json = serde_json::to_string(&heavy).unwrap();
        assert_eq!(json, "\"heavy\"");
    }

    #[test]
    fn test_action_step_move_to() {
        let action = ActionStep::MoveTo {
            x: 10,
            y: 20,
            speed: Some(MovementSpeed::Run),
        };
        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("\"act\":\"MoveTo\""));
        assert!(json.contains("\"x\":10"));
        assert!(json.contains("\"y\":20"));
    }

    #[test]
    fn test_action_step_attack() {
        let action = ActionStep::Attack { target_id: 42 };
        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("\"act\":\"Attack\""));
        assert!(json.contains("\"target_id\":42"));
    }

    #[test]
    fn test_action_step_take_cover() {
        let action = ActionStep::TakeCover { position: None };
        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("\"act\":\"TakeCover\""));
    }

    #[test]
    fn test_action_step_reload() {
        let action = ActionStep::Reload;
        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("\"act\":\"Reload\""));
    }

    #[test]
    fn test_rect_structure() {
        let rect = Rect {
            x0: 0,
            y0: 0,
            x1: 10,
            y1: 10,
        };
        assert_eq!(rect.x0, 0);
        assert_eq!(rect.x1, 10);
    }

    #[test]
    fn test_director_op_spawn_wave() {
        let op = DirectorOp::SpawnWave {
            archetype: "zombie".to_string(),
            count: 5,
            origin: IVec2 { x: 100, y: 200 },
        };
        let json = serde_json::to_string(&op).unwrap();
        assert!(json.contains("\"op\":\"SpawnWave\""));
        assert!(json.contains("\"count\":5"));
    }

    #[test]
    fn test_director_budget() {
        let budget = DirectorBudget {
            traps: 3,
            terrain_edits: 5,
            spawns: 10,
        };
        assert_eq!(budget.traps, 3);
        assert_eq!(budget.spawns, 10);
    }

    #[test]
    fn test_tool_registry() {
        let registry = ToolRegistry {
            tools: vec![],
            constraints: Constraints {
                enforce_cooldowns: true,
                enforce_los: false,
                enforce_stamina: true,
            },
        };
        assert!(registry.constraints.enforce_cooldowns);
        assert!(!registry.constraints.enforce_los);
    }

    #[test]
    fn test_engine_error_display() {
        let err = EngineError::InvalidAction("test".to_string());
        assert_eq!(err.to_string(), "invalid action: test");

        let err2 = EngineError::LosBlocked;
        assert_eq!(err2.to_string(), "line of sight blocked");
    }

    #[test]
    fn test_world_snapshot_with_data() {
        let snapshot = WorldSnapshot {
            t: 10.5,
            player: PlayerState::default(),
            me: CompanionState::default(),
            enemies: vec![EnemyState::default()],
            pois: vec![Poi::default()],
            obstacles: vec![IVec2 { x: 5, y: 5 }],
            objective: Some("Survive".to_string()),
        };
        assert_eq!(snapshot.t, 10.5);
        assert_eq!(snapshot.enemies.len(), 1);
        assert_eq!(snapshot.pois.len(), 1);
        assert_eq!(snapshot.obstacles.len(), 1);
        assert_eq!(snapshot.objective, Some("Survive".to_string()));
    }

    #[test]
    fn test_action_step_deserialization() {
        let json = r#"{"act":"MoveTo","x":5,"y":10,"speed":"run"}"#;
        let action: ActionStep = serde_json::from_str(json).unwrap();

        match action {
            ActionStep::MoveTo { x, y, speed } => {
                assert_eq!(x, 5);
                assert_eq!(y, 10);
                assert_eq!(speed, Some(MovementSpeed::Run));
            }
            _ => panic!("Expected MoveTo action"),
        }
    }

    #[test]
    fn test_companion_state_cooldowns() {
        let mut companion = CompanionState::default();
        companion.cooldowns.insert("attack".to_string(), 2.5);
        companion.cooldowns.insert("reload".to_string(), 1.0);

        assert_eq!(companion.cooldowns.get("attack"), Some(&2.5));
        assert_eq!(companion.cooldowns.len(), 2);
    }

    #[test]
    fn test_plan_intent_with_steps() {
        let intent = PlanIntent {
            plan_id: "plan_123".to_string(),
            steps: vec![
                ActionStep::MoveTo {
                    x: 10,
                    y: 20,
                    speed: None,
                },
                ActionStep::Attack { target_id: 5 },
                ActionStep::Reload,
            ],
        };
        assert_eq!(intent.plan_id, "plan_123");
        assert_eq!(intent.steps.len(), 3);
    }

    #[test]
    fn test_all_action_steps_compile() {
        // Ensure all ActionStep variants compile
        let _ = ActionStep::MoveTo {
            x: 0,
            y: 0,
            speed: None,
        };
        let _ = ActionStep::Approach {
            target_id: 1,
            distance: 5.0,
        };
        let _ = ActionStep::Retreat {
            target_id: 1,
            distance: 10.0,
        };
        let _ = ActionStep::TakeCover { position: None };
        let _ = ActionStep::Strafe {
            target_id: 1,
            direction: StrafeDirection::Left,
        };
        let _ = ActionStep::Patrol { waypoints: vec![] };
        let _ = ActionStep::Attack { target_id: 1 };
        let _ = ActionStep::AimedShot { target_id: 1 };
        let _ = ActionStep::QuickAttack { target_id: 1 };
        let _ = ActionStep::HeavyAttack { target_id: 1 };
        let _ = ActionStep::AoEAttack {
            x: 0,
            y: 0,
            radius: 5.0,
        };
        let _ = ActionStep::ThrowExplosive { x: 0, y: 0 };
        let _ = ActionStep::CoverFire {
            target_id: 1,
            duration: 3.0,
        };
        let _ = ActionStep::Charge { target_id: 1 };
        let _ = ActionStep::Block;
        let _ = ActionStep::Dodge { direction: None };
        let _ = ActionStep::Parry;
        let _ = ActionStep::ThrowSmoke { x: 0, y: 0 };
        let _ = ActionStep::Heal { target_id: None };
        let _ = ActionStep::UseDefensiveAbility {
            ability_name: "shield".to_string(),
        };
        let _ = ActionStep::EquipWeapon {
            weapon_name: "sword".to_string(),
        };
        let _ = ActionStep::SwitchWeapon { slot: 1 };
        let _ = ActionStep::Reload;
        let _ = ActionStep::UseItem {
            item_name: "potion".to_string(),
        };
        let _ = ActionStep::DropItem {
            item_name: "trash".to_string(),
        };
        let _ = ActionStep::CallReinforcements { count: 3 };
        let _ = ActionStep::MarkTarget { target_id: 1 };
        let _ = ActionStep::RequestCover { duration: 5.0 };
        let _ = ActionStep::CoordinateAttack { target_id: 1 };
        let _ = ActionStep::SetAmbush {
            position: IVec2 { x: 0, y: 0 },
        };
        let _ = ActionStep::Distract { target_id: 1 };
        let _ = ActionStep::Regroup {
            rally_point: IVec2 { x: 0, y: 0 },
        };
        let _ = ActionStep::Scan { radius: 10.0 };
        let _ = ActionStep::Wait { duration: 2.0 };
        let _ = ActionStep::Interact { target_id: 1 };
        let _ = ActionStep::UseAbility {
            ability_name: "fireball".to_string(),
        };
        let _ = ActionStep::Taunt { target_id: 1 };
        let _ = ActionStep::Throw {
            item: "grenade".to_string(),
            x: 0,
            y: 0,
        };
        let _ = ActionStep::Revive { ally_id: 1 };
        // New terrain action
        let _ = ActionStep::ModifyTerrain {
            request_id: "test-req-1".to_string(),
            payload: TerrainGenerationRequest::default(),
        };
    }

    // =========================================================================
    // TERRAIN DSL TESTS
    // =========================================================================

    #[test]
    fn test_terrain_feature_type_default() {
        let feature = TerrainFeatureType::default();
        if let TerrainFeatureType::Crater { radius } = feature {
            assert_eq!(radius, 10);
        } else {
            panic!("Default should be Crater");
        }
    }

    #[test]
    fn test_terrain_feature_type_serde() {
        let cave = TerrainFeatureType::Cave { depth: 50 };
        let json = serde_json::to_string(&cave).unwrap();
        assert!(json.contains("\"type\":\"Cave\""));
        assert!(json.contains("\"depth\":50"));

        let forest = TerrainFeatureType::Forest { density: 0.8 };
        let json = serde_json::to_string(&forest).unwrap();
        assert!(json.contains("\"type\":\"Forest\""));
    }

    #[test]
    fn test_cardinal_direction_to_unit_vector() {
        let (x, z) = CardinalDirection::North.to_unit_vector();
        assert!((x - 0.0).abs() < 0.001);
        assert!((z - (-1.0)).abs() < 0.001);

        let (x, z) = CardinalDirection::East.to_unit_vector();
        assert!((x - 1.0).abs() < 0.001);
        assert!((z - 0.0).abs() < 0.001);

        let (x, z) = CardinalDirection::NorthEast.to_unit_vector();
        assert!((x - 0.707).abs() < 0.001);
        assert!((z - (-0.707)).abs() < 0.001);
    }

    #[test]
    fn test_distance_category_to_range() {
        let (min, max) = DistanceCategory::Near.to_range();
        assert_eq!(min, 10.0);
        assert_eq!(max, 50.0);

        let (min, max) = DistanceCategory::Medium.to_range();
        assert_eq!(min, 50.0);
        assert_eq!(max, 150.0);

        let (min, max) = DistanceCategory::Far.to_range();
        assert_eq!(min, 150.0);
        assert_eq!(max, 500.0);
    }

    #[test]
    fn test_distance_category_midpoint() {
        assert_eq!(DistanceCategory::Near.midpoint(), 30.0);
        assert_eq!(DistanceCategory::Medium.midpoint(), 100.0);
        assert_eq!(DistanceCategory::Far.midpoint(), 325.0);
    }

    #[test]
    fn test_relative_location_serde() {
        let los = RelativeLocation::LineOfSight {
            look_distance: 100.0,
        };
        let json = serde_json::to_string(&los).unwrap();
        assert!(json.contains("\"method\":\"LineOfSight\""));
        assert!(json.contains("\"look_distance\":100"));

        let dir = RelativeLocation::DirectionFrom {
            cardinal: CardinalDirection::North,
            distance: DistanceCategory::Medium,
        };
        let json = serde_json::to_string(&dir).unwrap();
        assert!(json.contains("\"method\":\"DirectionFrom\""));
        assert!(json.contains("\"cardinal\":\"north\""));
        assert!(json.contains("\"distance\":\"medium\""));
    }

    #[test]
    fn test_persistence_mode_default() {
        let mode = PersistenceMode::default();
        assert_eq!(mode, PersistenceMode::SessionOnly);
    }

    #[test]
    fn test_terrain_generation_request_default() {
        let req = TerrainGenerationRequest::default();
        assert!(req.request_id.is_empty());
        assert_eq!(req.intensity, 0.5);
        assert!(req.biome_constraints.is_empty());
        assert_eq!(req.persistence_mode, PersistenceMode::SessionOnly);
    }

    #[test]
    fn test_terrain_generation_request_validate_success() {
        let req = TerrainGenerationRequest {
            request_id: "test-123".to_string(),
            feature_type: TerrainFeatureType::Crater { radius: 20 },
            relative_location: RelativeLocation::LineOfSight {
                look_distance: 50.0,
            },
            intensity: 0.7,
            narrative_reason: "Creating a crater for quest objective".to_string(),
            persistence_mode: PersistenceMode::Persistent,
            biome_constraints: vec!["grassland".to_string()],
            seed: Some(12345),
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_terrain_generation_request_validate_intensity_too_high() {
        let req = TerrainGenerationRequest {
            request_id: "test-123".to_string(),
            intensity: 1.5, // Invalid
            ..TerrainGenerationRequest::default()
        };
        let result = req.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Intensity"));
    }

    #[test]
    fn test_terrain_generation_request_validate_intensity_negative() {
        let req = TerrainGenerationRequest {
            request_id: "test-123".to_string(),
            intensity: -0.1, // Invalid
            ..TerrainGenerationRequest::default()
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_terrain_generation_request_validate_narrative_too_long() {
        let req = TerrainGenerationRequest {
            request_id: "test-123".to_string(),
            narrative_reason: "x".repeat(101), // Too long
            ..TerrainGenerationRequest::default()
        };
        let result = req.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("100 characters"));
    }

    #[test]
    fn test_terrain_generation_request_validate_empty_request_id() {
        let req = TerrainGenerationRequest::default(); // Empty request_id
        let result = req.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Request ID"));
    }

    #[test]
    fn test_action_step_modify_terrain_serde() {
        let action = ActionStep::ModifyTerrain {
            request_id: "terrain-001".to_string(),
            payload: TerrainGenerationRequest {
                request_id: "terrain-001".to_string(),
                feature_type: TerrainFeatureType::Cave { depth: 30 },
                relative_location: RelativeLocation::DirectionFrom {
                    cardinal: CardinalDirection::North,
                    distance: DistanceCategory::Near,
                },
                intensity: 0.6,
                narrative_reason: "Secret cave for quest".to_string(),
                persistence_mode: PersistenceMode::Persistent,
                biome_constraints: vec!["mountain".to_string(), "forest".to_string()],
                seed: Some(42),
            },
        };
        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("\"act\":\"ModifyTerrain\""));
        assert!(json.contains("\"request_id\":\"terrain-001\""));
        assert!(json.contains("\"type\":\"Cave\""));
        assert!(json.contains("\"depth\":30"));
    }

    // ========== IVec2 Helper Tests ==========

    #[test]
    fn test_ivec2_zero() {
        let v = IVec2::zero();
        assert_eq!(v.x, 0);
        assert_eq!(v.y, 0);
        assert!(v.is_zero());
    }

    #[test]
    fn test_ivec2_is_zero() {
        assert!(IVec2::zero().is_zero());
        assert!(IVec2 { x: 0, y: 0 }.is_zero());
        assert!(!IVec2 { x: 1, y: 0 }.is_zero());
        assert!(!IVec2 { x: 0, y: 1 }.is_zero());
        assert!(!IVec2 { x: -1, y: -1 }.is_zero());
    }

    #[test]
    fn test_ivec2_manhattan_distance() {
        let a = IVec2 { x: 0, y: 0 };
        let b = IVec2 { x: 3, y: 4 };
        assert_eq!(a.manhattan_distance(&b), 7);
        assert_eq!(b.manhattan_distance(&a), 7);

        let c = IVec2 { x: -2, y: 3 };
        let d = IVec2 { x: 1, y: -1 };
        assert_eq!(c.manhattan_distance(&d), 7); // |3| + |4| = 7
    }

    #[test]
    fn test_ivec2_distance_squared() {
        let a = IVec2 { x: 0, y: 0 };
        let b = IVec2 { x: 3, y: 4 };
        assert_eq!(a.distance_squared(&b), 25); // 3*3 + 4*4 = 25
        assert_eq!(b.distance_squared(&a), 25);

        let c = IVec2 { x: 1, y: 1 };
        let d = IVec2 { x: 4, y: 5 };
        assert_eq!(c.distance_squared(&d), 25); // 3*3 + 4*4 = 25
    }

    #[test]
    fn test_ivec2_distance() {
        let a = IVec2 { x: 0, y: 0 };
        let b = IVec2 { x: 3, y: 4 };
        assert!((a.distance(&b) - 5.0).abs() < 0.001);

        let c = IVec2 { x: 0, y: 0 };
        let d = IVec2 { x: 0, y: 10 };
        assert!((c.distance(&d) - 10.0).abs() < 0.001);
    }

    #[test]
    fn test_ivec2_offset() {
        let v = IVec2 { x: 5, y: 10 };
        let result = v.offset(3, -2);
        assert_eq!(result, IVec2 { x: 8, y: 8 });

        let zero = IVec2::zero();
        let result = zero.offset(-5, 5);
        assert_eq!(result, IVec2 { x: -5, y: 5 });
    }

    #[test]
    fn test_ivec2_add() {
        let a = IVec2 { x: 1, y: 2 };
        let b = IVec2 { x: 3, y: 4 };
        let sum = a + b;
        assert_eq!(sum, IVec2 { x: 4, y: 6 });
    }

    #[test]
    fn test_ivec2_sub() {
        let a = IVec2 { x: 5, y: 10 };
        let b = IVec2 { x: 2, y: 3 };
        let diff = a - b;
        assert_eq!(diff, IVec2 { x: 3, y: 7 });
    }

    #[test]
    fn test_ivec2_display() {
        let v = IVec2 { x: 42, y: -17 };
        assert_eq!(format!("{}", v), "(42, -17)");
    }

    // ========== WorldSnapshot Helper Tests ==========

    #[test]
    fn test_world_snapshot_enemy_count() {
        let mut snap = WorldSnapshot::default();
        assert_eq!(snap.enemy_count(), 0);

        snap.enemies.push(EnemyState::default());
        snap.enemies.push(EnemyState::default());
        assert_eq!(snap.enemy_count(), 2);
    }

    #[test]
    fn test_world_snapshot_has_no_enemies() {
        let mut snap = WorldSnapshot::default();
        assert!(snap.has_no_enemies());

        snap.enemies.push(EnemyState::default());
        assert!(!snap.has_no_enemies());
    }

    #[test]
    fn test_world_snapshot_nearest_enemy() {
        let mut snap = WorldSnapshot::default();
        snap.me.pos = IVec2::zero();

        // No enemies
        assert!(snap.nearest_enemy().is_none());

        // Add enemies at various distances
        snap.enemies.push(EnemyState {
            pos: IVec2 { x: 10, y: 0 },
            ..Default::default()
        });
        snap.enemies.push(EnemyState {
            pos: IVec2 { x: 5, y: 0 },
            ..Default::default()
        });
        snap.enemies.push(EnemyState {
            pos: IVec2 { x: 20, y: 0 },
            ..Default::default()
        });

        let nearest = snap.nearest_enemy().unwrap();
        assert_eq!(nearest.pos, IVec2 { x: 5, y: 0 });
    }

    #[test]
    fn test_world_snapshot_enemies_within_range() {
        let mut snap = WorldSnapshot::default();
        snap.me.pos = IVec2::zero();

        snap.enemies.push(EnemyState {
            id: 1,
            pos: IVec2 { x: 5, y: 0 }, // manhattan = 5
            ..Default::default()
        });
        snap.enemies.push(EnemyState {
            id: 2,
            pos: IVec2 { x: 15, y: 0 }, // manhattan = 15
            ..Default::default()
        });
        snap.enemies.push(EnemyState {
            id: 3,
            pos: IVec2 { x: 3, y: 4 }, // manhattan = 7
            ..Default::default()
        });

        let in_range = snap.enemies_within_range(10);
        assert_eq!(in_range.len(), 2);
        assert!(in_range.iter().any(|e| e.id == 1));
        assert!(in_range.iter().any(|e| e.id == 3));
    }

    #[test]
    fn test_world_snapshot_has_ammo() {
        let mut snap = WorldSnapshot::default();
        snap.me.ammo = 10;
        assert!(snap.has_ammo());

        snap.me.ammo = 0;
        assert!(!snap.has_ammo());

        snap.me.ammo = -1;
        assert!(!snap.has_ammo());
    }

    #[test]
    fn test_world_snapshot_has_pois() {
        let mut snap = WorldSnapshot::default();
        assert!(!snap.has_pois());

        snap.pois.push(Poi::default());
        assert!(snap.has_pois());
    }

    #[test]
    fn test_world_snapshot_has_objective() {
        let mut snap = WorldSnapshot::default();
        assert!(!snap.has_objective());

        snap.objective = Some("Capture the point".to_string());
        assert!(snap.has_objective());
    }

    #[test]
    fn test_world_snapshot_distance_to_player() {
        let mut snap = WorldSnapshot::default();
        snap.me.pos = IVec2::zero();
        snap.player.pos = IVec2 { x: 3, y: 4 };

        assert!((snap.distance_to_player() - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_world_snapshot_display() {
        let mut snap = WorldSnapshot::default();
        snap.t = 10.5;
        snap.enemies.push(EnemyState::default());
        snap.pois.push(Poi::default());

        let display = format!("{}", snap);
        assert!(display.contains("10.5"));
        assert!(display.contains("1 enemies"));
        assert!(display.contains("1 POIs"));
    }

    // ========== PlanIntent Helper Tests ==========

    #[test]
    fn test_plan_intent_new() {
        let plan = PlanIntent::new("test-plan");
        assert_eq!(plan.plan_id, "test-plan");
        assert!(plan.steps.is_empty());
    }

    #[test]
    fn test_plan_intent_empty() {
        let plan = PlanIntent::empty();
        assert!(plan.plan_id.is_empty());
        assert!(plan.steps.is_empty());
    }

    #[test]
    fn test_plan_intent_is_empty() {
        let empty_plan = PlanIntent::empty();
        assert!(empty_plan.is_empty());

        let non_empty = PlanIntent {
            plan_id: "test".to_string(),
            steps: vec![ActionStep::Wait { duration: 1.0 }],
        };
        assert!(!non_empty.is_empty());
    }

    #[test]
    fn test_plan_intent_step_count() {
        let mut plan = PlanIntent::new("test");
        assert_eq!(plan.step_count(), 0);

        plan.steps.push(ActionStep::Reload);
        plan.steps.push(ActionStep::Wait { duration: 1.0 });
        assert_eq!(plan.step_count(), 2);
    }

    #[test]
    fn test_plan_intent_with_step() {
        let plan = PlanIntent::new("test")
            .with_step(ActionStep::Reload)
            .with_step(ActionStep::Wait { duration: 1.0 });

        assert_eq!(plan.step_count(), 2);
        assert_eq!(plan.steps[0], ActionStep::Reload);
    }

    #[test]
    fn test_plan_intent_first_step() {
        let empty = PlanIntent::empty();
        assert!(empty.first_step().is_none());

        let plan = PlanIntent::new("test").with_step(ActionStep::Reload);
        assert_eq!(plan.first_step(), Some(&ActionStep::Reload));
    }

    #[test]
    fn test_plan_intent_has_movement() {
        let empty = PlanIntent::empty();
        assert!(!empty.has_movement());

        let with_move = PlanIntent::new("test").with_step(ActionStep::MoveTo {
            x: 5,
            y: 5,
            speed: None,
        });
        assert!(with_move.has_movement());

        let without_move = PlanIntent::new("test").with_step(ActionStep::Reload);
        assert!(!without_move.has_movement());
    }

    #[test]
    fn test_plan_intent_has_offensive() {
        let empty = PlanIntent::empty();
        assert!(!empty.has_offensive());

        let with_attack = PlanIntent::new("test").with_step(ActionStep::Attack { target_id: 1 });
        assert!(with_attack.has_offensive());

        let without_attack = PlanIntent::new("test").with_step(ActionStep::Reload);
        assert!(!without_attack.has_offensive());
    }

    #[test]
    fn test_plan_intent_display() {
        let plan = PlanIntent::new("combat-plan")
            .with_step(ActionStep::Reload)
            .with_step(ActionStep::Attack { target_id: 1 });

        let display = format!("{}", plan);
        assert!(display.contains("combat-plan"));
        assert!(display.contains("2 steps"));
    }

    // ========== ActionStep Helper Tests ==========

    #[test]
    fn test_action_step_category_movement() {
        assert_eq!(
            ActionStep::MoveTo {
                x: 0,
                y: 0,
                speed: None
            }
            .category(),
            "Movement"
        );
        assert_eq!(
            ActionStep::Approach {
                target_id: 1,
                distance: 5.0
            }
            .category(),
            "Movement"
        );
        assert_eq!(
            ActionStep::Retreat {
                target_id: 1,
                distance: 5.0
            }
            .category(),
            "Movement"
        );
        assert_eq!(
            ActionStep::TakeCover { position: None }.category(),
            "Movement"
        );
        assert_eq!(
            ActionStep::Strafe {
                target_id: 1,
                direction: StrafeDirection::Left
            }
            .category(),
            "Movement"
        );
        assert_eq!(
            ActionStep::Patrol { waypoints: vec![] }.category(),
            "Movement"
        );
    }

    #[test]
    fn test_action_step_category_offensive() {
        assert_eq!(ActionStep::Attack { target_id: 1 }.category(), "Offensive");
        assert_eq!(
            ActionStep::AimedShot { target_id: 1 }.category(),
            "Offensive"
        );
        assert_eq!(
            ActionStep::QuickAttack { target_id: 1 }.category(),
            "Offensive"
        );
        assert_eq!(
            ActionStep::HeavyAttack { target_id: 1 }.category(),
            "Offensive"
        );
        assert_eq!(
            ActionStep::AoEAttack {
                x: 0,
                y: 0,
                radius: 5.0
            }
            .category(),
            "Offensive"
        );
        assert_eq!(
            ActionStep::ThrowExplosive { x: 0, y: 0 }.category(),
            "Offensive"
        );
        assert_eq!(
            ActionStep::CoverFire {
                target_id: 1,
                duration: 2.0
            }
            .category(),
            "Offensive"
        );
        assert_eq!(ActionStep::Charge { target_id: 1 }.category(), "Offensive");
    }

    #[test]
    fn test_action_step_category_defensive() {
        assert_eq!(ActionStep::Block.category(), "Defensive");
        assert_eq!(
            ActionStep::Dodge { direction: None }.category(),
            "Defensive"
        );
        assert_eq!(ActionStep::Parry.category(), "Defensive");
        assert_eq!(
            ActionStep::ThrowSmoke { x: 0, y: 0 }.category(),
            "Defensive"
        );
        assert_eq!(ActionStep::Heal { target_id: None }.category(), "Defensive");
        assert_eq!(
            ActionStep::UseDefensiveAbility {
                ability_name: "shield".to_string()
            }
            .category(),
            "Defensive"
        );
    }

    #[test]
    fn test_action_step_category_equipment() {
        assert_eq!(ActionStep::Reload.category(), "Equipment");
        assert_eq!(
            ActionStep::EquipWeapon {
                weapon_name: "rifle".to_string()
            }
            .category(),
            "Equipment"
        );
        assert_eq!(ActionStep::SwitchWeapon { slot: 1 }.category(), "Equipment");
        assert_eq!(
            ActionStep::UseItem {
                item_name: "medkit".to_string()
            }
            .category(),
            "Equipment"
        );
        assert_eq!(
            ActionStep::DropItem {
                item_name: "grenade".to_string()
            }
            .category(),
            "Equipment"
        );
    }

    #[test]
    fn test_action_step_category_tactical() {
        assert_eq!(
            ActionStep::CallReinforcements { count: 2 }.category(),
            "Tactical"
        );
        assert_eq!(
            ActionStep::MarkTarget { target_id: 1 }.category(),
            "Tactical"
        );
        assert_eq!(
            ActionStep::RequestCover { duration: 3.0 }.category(),
            "Tactical"
        );
        assert_eq!(
            ActionStep::CoordinateAttack { target_id: 1 }.category(),
            "Tactical"
        );
        assert_eq!(
            ActionStep::SetAmbush {
                position: IVec2::zero()
            }
            .category(),
            "Tactical"
        );
        assert_eq!(ActionStep::Distract { target_id: 1 }.category(), "Tactical");
        assert_eq!(
            ActionStep::Regroup {
                rally_point: IVec2::zero()
            }
            .category(),
            "Tactical"
        );
    }

    #[test]
    fn test_action_step_category_utility() {
        assert_eq!(ActionStep::Scan { radius: 10.0 }.category(), "Utility");
        assert_eq!(ActionStep::Wait { duration: 1.0 }.category(), "Utility");
        assert_eq!(ActionStep::Interact { target_id: 1 }.category(), "Utility");
        assert_eq!(
            ActionStep::UseAbility {
                ability_name: "dash".to_string()
            }
            .category(),
            "Utility"
        );
        assert_eq!(ActionStep::Taunt { target_id: 1 }.category(), "Utility");
    }

    #[test]
    fn test_action_step_category_legacy() {
        assert_eq!(
            ActionStep::Throw {
                item: "grenade".to_string(),
                x: 0,
                y: 0
            }
            .category(),
            "Legacy"
        );
        assert_eq!(ActionStep::Revive { ally_id: 1 }.category(), "Legacy");
    }

    #[test]
    fn test_action_step_is_movement() {
        assert!(ActionStep::MoveTo {
            x: 0,
            y: 0,
            speed: None
        }
        .is_movement());
        assert!(ActionStep::Approach {
            target_id: 1,
            distance: 5.0
        }
        .is_movement());
        assert!(ActionStep::TakeCover { position: None }.is_movement());
        assert!(!ActionStep::Reload.is_movement());
        assert!(!ActionStep::Attack { target_id: 1 }.is_movement());
    }

    #[test]
    fn test_action_step_is_offensive() {
        assert!(ActionStep::Attack { target_id: 1 }.is_offensive());
        assert!(ActionStep::AimedShot { target_id: 1 }.is_offensive());
        assert!(ActionStep::ThrowExplosive { x: 0, y: 0 }.is_offensive());
        assert!(!ActionStep::Reload.is_offensive());
        assert!(!ActionStep::MoveTo {
            x: 0,
            y: 0,
            speed: None
        }
        .is_offensive());
    }

    #[test]
    fn test_action_step_is_defensive() {
        assert!(ActionStep::Block.is_defensive());
        assert!(ActionStep::Dodge { direction: None }.is_defensive());
        assert!(ActionStep::Parry.is_defensive());
        assert!(ActionStep::ThrowSmoke { x: 0, y: 0 }.is_defensive());
        assert!(!ActionStep::Reload.is_defensive());
        assert!(!ActionStep::Attack { target_id: 1 }.is_defensive());
    }

    #[test]
    fn test_action_step_is_equipment() {
        assert!(ActionStep::Reload.is_equipment());
        assert!(ActionStep::SwitchWeapon { slot: 1 }.is_equipment());
        assert!(ActionStep::EquipWeapon {
            weapon_name: "rifle".to_string()
        }
        .is_equipment());
        assert!(!ActionStep::Attack { target_id: 1 }.is_equipment());
    }

    #[test]
    fn test_action_step_is_tactical() {
        assert!(ActionStep::MarkTarget { target_id: 1 }.is_tactical());
        assert!(ActionStep::CallReinforcements { count: 2 }.is_tactical());
        assert!(ActionStep::Distract { target_id: 1 }.is_tactical());
        assert!(!ActionStep::Reload.is_tactical());
    }

    #[test]
    fn test_action_step_is_utility() {
        assert!(ActionStep::Wait { duration: 1.0 }.is_utility());
        assert!(ActionStep::Scan { radius: 10.0 }.is_utility());
        assert!(ActionStep::Interact { target_id: 1 }.is_utility());
        assert!(!ActionStep::Reload.is_utility());
    }

    #[test]
    fn test_action_step_action_name() {
        assert_eq!(
            ActionStep::MoveTo {
                x: 0,
                y: 0,
                speed: None
            }
            .action_name(),
            "MoveTo"
        );
        assert_eq!(ActionStep::Attack { target_id: 1 }.action_name(), "Attack");
        assert_eq!(ActionStep::Reload.action_name(), "Reload");
        assert_eq!(
            ActionStep::TakeCover { position: None }.action_name(),
            "TakeCover"
        );
        assert_eq!(ActionStep::Wait { duration: 1.0 }.action_name(), "Wait");
        assert_eq!(ActionStep::Block.action_name(), "Block");
        assert_eq!(ActionStep::Parry.action_name(), "Parry");
    }

    #[test]
    fn test_action_step_targets_entity() {
        assert!(ActionStep::Attack { target_id: 1 }.targets_entity());
        assert!(ActionStep::AimedShot { target_id: 1 }.targets_entity());
        assert!(ActionStep::Charge { target_id: 1 }.targets_entity());
        assert!(ActionStep::MarkTarget { target_id: 1 }.targets_entity());
        assert!(ActionStep::Interact { target_id: 1 }.targets_entity());

        assert!(!ActionStep::Reload.targets_entity());
        assert!(!ActionStep::MoveTo {
            x: 0,
            y: 0,
            speed: None
        }
        .targets_entity());
        assert!(!ActionStep::Wait { duration: 1.0 }.targets_entity());
        assert!(!ActionStep::Block.targets_entity());
    }

    #[test]
    fn test_action_step_target_entity() {
        assert_eq!(
            ActionStep::Attack { target_id: 42 }.target_entity(),
            Some(42)
        );
        assert_eq!(
            ActionStep::AimedShot { target_id: 5 }.target_entity(),
            Some(5)
        );
        assert_eq!(
            ActionStep::MarkTarget { target_id: 10 }.target_entity(),
            Some(10)
        );
        assert_eq!(ActionStep::Charge { target_id: 3 }.target_entity(), Some(3));
        assert_eq!(
            ActionStep::Interact { target_id: 99 }.target_entity(),
            Some(99)
        );
        assert_eq!(
            ActionStep::Approach {
                target_id: 7,
                distance: 5.0
            }
            .target_entity(),
            Some(7)
        );
        assert_eq!(
            ActionStep::Retreat {
                target_id: 8,
                distance: 10.0
            }
            .target_entity(),
            Some(8)
        );

        assert_eq!(ActionStep::Reload.target_entity(), None);
        assert_eq!(ActionStep::Wait { duration: 1.0 }.target_entity(), None);
        assert_eq!(
            ActionStep::MoveTo {
                x: 0,
                y: 0,
                speed: None
            }
            .target_entity(),
            None
        );
        assert_eq!(ActionStep::Block.target_entity(), None);

        // Remediation: kill delete match arm Self::Revive{ally_id}
        assert_eq!(
            ActionStep::Revive { ally_id: 77 }.target_entity(),
            Some(77),
            "Revive should return ally_id as target"
        );
        // Remediation: kill delete match arm Self::Heal{target_id}
        assert_eq!(
            ActionStep::Heal {
                target_id: Some(88)
            }
            .target_entity(),
            Some(88),
            "Heal with target should return target_id"
        );
        assert_eq!(
            ActionStep::Heal { target_id: None }.target_entity(),
            None,
            "Heal with no target should return None"
        );
    }

    #[test]
    fn test_action_step_has_position() {
        assert!(ActionStep::MoveTo {
            x: 5,
            y: 10,
            speed: None
        }
        .has_position());
        assert!(ActionStep::TakeCover {
            position: Some(IVec2 { x: 1, y: 2 })
        }
        .has_position());
        assert!(ActionStep::ThrowExplosive { x: 3, y: 4 }.has_position());
        assert!(ActionStep::ThrowSmoke { x: 5, y: 6 }.has_position());
        assert!(ActionStep::AoEAttack {
            x: 0,
            y: 0,
            radius: 5.0
        }
        .has_position());
        assert!(ActionStep::SetAmbush {
            position: IVec2::zero()
        }
        .has_position());

        assert!(!ActionStep::TakeCover { position: None }.has_position());
        assert!(!ActionStep::Reload.has_position());
        assert!(!ActionStep::Attack { target_id: 1 }.has_position());
    }

    #[test]
    fn test_action_step_display() {
        // Display format is "Category::ActionName"
        assert_eq!(
            format!(
                "{}",
                ActionStep::MoveTo {
                    x: 5,
                    y: 10,
                    speed: None
                }
            ),
            "Movement::MoveTo"
        );
        assert_eq!(
            format!("{}", ActionStep::Attack { target_id: 42 }),
            "Offensive::Attack"
        );
        assert_eq!(format!("{}", ActionStep::Reload), "Equipment::Reload");
        assert_eq!(
            format!(
                "{}",
                ActionStep::TakeCover {
                    position: Some(IVec2 { x: 3, y: 4 })
                }
            ),
            "Movement::TakeCover"
        );
        assert_eq!(
            format!("{}", ActionStep::TakeCover { position: None }),
            "Movement::TakeCover"
        );
        assert_eq!(
            format!("{}", ActionStep::Wait { duration: 2.5 }),
            "Utility::Wait"
        );
        assert_eq!(format!("{}", ActionStep::Block), "Defensive::Block");
        assert_eq!(format!("{}", ActionStep::Parry), "Defensive::Parry");
    }

    #[test]
    fn test_action_step_equality() {
        assert_eq!(ActionStep::Reload, ActionStep::Reload);
        assert_eq!(ActionStep::Block, ActionStep::Block);
        assert_eq!(
            ActionStep::Attack { target_id: 1 },
            ActionStep::Attack { target_id: 1 }
        );
        assert_ne!(
            ActionStep::Attack { target_id: 1 },
            ActionStep::Attack { target_id: 2 }
        );
        assert_ne!(ActionStep::Reload, ActionStep::Block);
    }

    #[test]
    fn test_plan_intent_equality() {
        let plan1 = PlanIntent::new("test").with_step(ActionStep::Reload);
        let plan2 = PlanIntent::new("test").with_step(ActionStep::Reload);
        let plan3 = PlanIntent::new("other").with_step(ActionStep::Reload);

        assert_eq!(plan1, plan2);
        assert_ne!(plan1, plan3);
    }

    // ===== Mutation-resistant CardinalDirection tests =====
    // Catches: `delete - in to_unit_vector` (West's -1.0 → 1.0, North's -1.0 → 1.0, etc.)

    #[test]
    fn test_cardinal_west_is_negative_x() {
        let (x, z) = CardinalDirection::West.to_unit_vector();
        assert!(x < 0.0, "West x must be negative, got {}", x);
        assert!((x - (-1.0)).abs() < 0.001);
        assert!((z - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_cardinal_south_is_positive_z() {
        let (x, z) = CardinalDirection::South.to_unit_vector();
        assert!((x - 0.0).abs() < 0.001);
        assert!((z - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_cardinal_northwest_both_components() {
        let (x, z) = CardinalDirection::NorthWest.to_unit_vector();
        assert!(x < 0.0, "NW x must be negative");
        assert!(z < 0.0, "NW z must be negative");
        assert!((x - (-0.707)).abs() < 0.001);
        assert!((z - (-0.707)).abs() < 0.001);
    }

    #[test]
    fn test_cardinal_southwest_sign_check() {
        let (x, z) = CardinalDirection::SouthWest.to_unit_vector();
        assert!(x < 0.0, "SW x must be negative");
        assert!(z > 0.0, "SW z must be positive");
        assert!((x - (-0.707)).abs() < 0.001);
        assert!((z - 0.707).abs() < 0.001);
    }

    #[test]
    fn test_cardinal_southeast_sign_check() {
        let (x, z) = CardinalDirection::SouthEast.to_unit_vector();
        assert!(x > 0.0, "SE x must be positive");
        assert!(z > 0.0, "SE z must be positive");
        assert!((x - 0.707).abs() < 0.001);
        assert!((z - 0.707).abs() < 0.001);
    }
}
