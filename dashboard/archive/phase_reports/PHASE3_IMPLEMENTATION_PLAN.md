# Phase 3 Implementation Plan: AI & Gameplay Systems

**Date**: October 1, 2025  
**Status**: 🚧 **IN PROGRESS**  
**Target**: Complete AI planners, gameplay systems, weaving, and PCG

---

## Executive Summary

Phase 3 builds on the validated AI loop (Perception → Reasoning → Planning → Action) from Phase 1, adding:
- **Dynamic Planners**: Behavior Trees (BT) and GOAP for intelligent agent decision-making
- **Gameplay Systems**: Combat, crafting, dialogue on ECS with deterministic execution
- **Weaving System**: Emergent behavior layer that detects patterns and proposes intents
- **Procedural Generation**: Seed-reproducible PCG for encounters and layouts

**Guardrails**:
- Determinism via fixed seeds (no unseeded RNG)
- Feature flags for opt-in complexity
- No breaking API changes (adapters + deprecation warnings)
- CI green: format, lint, all tests passing
- Data-driven configs (TOML/JSON)
- LLM guardrails via existing sandbox/validator

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                   Phase 3 Architecture                       │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  ┌────────────────┐         ┌──────────────────┐            │
│  │ astraweave-ai  │────────▶│ astraweave-      │            │
│  │ (Core Loop)    │         │ behavior         │            │
│  │                │         │ • BT Planner     │            │
│  │ Perception     │         │ • GOAP Planner   │            │
│  │ Reasoning      │         │ • Blackboard     │            │
│  │ Planning ───────┼────────▶│ • Data Loaders  │            │
│  │ Action         │         └──────────────────┘            │
│  └────────────────┘                                          │
│         │                                                     │
│         │                   ┌──────────────────┐            │
│         └──────────────────▶│ astraweave-      │            │
│                             │ gameplay         │            │
│                             │ • Combat Plugin  │            │
│                             │ • Crafting Plugin│            │
│                             │ • Dialogue Plugin│            │
│                             └──────────────────┘            │
│                                                              │
│  ┌────────────────┐         ┌──────────────────┐           │
│  │ astraweave-    │────────▶│ astraweave-pcg   │           │
│  │ weaving        │         │ • SeedRng        │           │
│  │ • Patterns     │         │ • Encounters     │           │
│  │ • Intents      │         │ • Layouts        │           │
│  │ • Adjudicator  │         │ • ECS Integration│           │
│  └────────────────┘         └──────────────────┘           │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---

## Implementation Phases

### Phase A: Dynamic Planners (Days 1-4)

#### A.1: Behavior Trees (astraweave-behavior)

**Goal**: Implement composable BT nodes with deterministic execution

**Nodes**:
```rust
pub enum BehaviorNode {
    // Composites
    Sequence(Vec<BehaviorNode>),      // AND logic
    Selector(Vec<BehaviorNode>),      // OR logic
    Parallel { children: Vec<BehaviorNode>, min_success: usize },
    
    // Decorators
    Invert(Box<BehaviorNode>),        // Success ↔ Failure
    Succeeder(Box<BehaviorNode>),     // Always Success
    Failer(Box<BehaviorNode>),        // Always Failure
    Repeat { child: Box<BehaviorNode>, count: usize },
    UntilFail(Box<BehaviorNode>),
    
    // Leaves
    Action(ActionNode),               // Execute action
    Condition(ConditionNode),         // Check predicate
}

pub enum NodeStatus {
    Success,
    Failure,
    Running,
}
```

**Blackboard**:
```rust
pub struct Blackboard {
    data: BTreeMap<String, BlackboardValue>,
}

pub enum BlackboardValue {
    Bool(bool),
    I32(i32),
    F32(f32),
    String(String),
    EntityId(EntityId),
}
```

**ECS Integration**:
```rust
// Component
pub struct CBehaviorTree {
    pub tree_id: String,
    pub state: BTreeMap<usize, NodeStatus>,  // Node index → status
    pub blackboard: Blackboard,
}

// Plugin
pub struct BehaviorTreePlugin;

impl Plugin for BehaviorTreePlugin {
    fn build(&self, app: &mut App) {
        app.add_system(Stage::AiPlanning, tick_behavior_trees);
    }
}

fn tick_behavior_trees(
    world: &mut World,
    trees: &BTreeMap<String, BehaviorTree>,
    dt: f32,
) {
    for (entity, tree_component) in world.query::<&mut CBehaviorTree>() {
        let tree = &trees[&tree_component.tree_id];
        let status = tree.tick(&mut tree_component.blackboard, dt);
        // Update state, emit intents if needed
    }
}
```

**Data Format** (TOML):
```toml
[tree.patrol_and_attack]
root = { type = "Selector", children = ["attack_nearby", "patrol"] }

[tree.patrol_and_attack.nodes.attack_nearby]
type = "Sequence"
children = ["has_target", "in_range", "attack"]

[tree.patrol_and_attack.nodes.patrol]
type = "Action"
action = "move_to_waypoint"
```

**Tests**:
- Unit: Node semantics (sequence stops on failure, selector on success)
- Unit: Decorator correctness (invert flips, repeat counts)
- Unit: Loader validation (acyclic, valid refs)
- Integration: BT agent solves patrol→chase→attack deterministically

**Estimated Effort**: 2 days

---

#### A.2: GOAP Planner (astraweave-behavior)

**Goal**: A* planning over symbolic world states

**Domain**:
```rust
pub struct GoapAction {
    pub name: String,
    pub cost: f32,
    pub preconditions: WorldState,
    pub effects: WorldState,
}

pub struct WorldState {
    data: BTreeMap<String, bool>,  // Deterministic iteration
}

pub struct GoapGoal {
    pub desired_state: WorldState,
    pub priority: f32,
}
```

**Planner**:
```rust
pub struct GoapPlanner;

impl GoapPlanner {
    pub fn plan(
        current_state: &WorldState,
        goal: &GoapGoal,
        available_actions: &[GoapAction],
    ) -> Option<Vec<GoapAction>> {
        // A* with deterministic tie-breaking (by name then cost)
        let mut open_set = BTreeSet::new();  // Stable ordering
        let mut came_from = BTreeMap::new();
        
        // ... A* implementation
    }
}
```

**ECS Integration**:
```rust
pub struct CGoals {
    pub goals: Vec<GoapGoal>,  // Sorted by priority
}

pub struct CActionsKnown {
    pub actions: Vec<GoapAction>,
}

pub struct CPlan {
    pub actions: Vec<GoapAction>,
    pub current_index: usize,
}

fn plan_goap(world: &mut World) {
    for (entity, (goals, actions, mut plan)) in 
        world.query::<(&CGoals, &CActionsKnown, &mut CPlan)>()
    {
        if plan.is_complete() || plan.is_invalid() {
            // Build current state from perception
            let current_state = build_state_from_perception(world, entity);
            
            // Plan for highest priority goal
            if let Some(goal) = goals.goals.first() {
                if let Some(new_plan) = GoapPlanner::plan(
                    &current_state,
                    goal,
                    &actions.actions,
                ) {
                    plan.actions = new_plan;
                    plan.current_index = 0;
                }
            }
        }
    }
}
```

**Tests**:
- Unit: Plan optimality (minimal cost path)
- Unit: Reproducibility (same input → same plan)
- Integration: GOAP agent achieves "has_food" via gather→craft→eat

**Estimated Effort**: 2 days

---

### Phase B: Gameplay Systems (Days 5-8)

#### B.1: Combat System (astraweave-gameplay)

**Components**:
```rust
pub struct CHealth {
    pub current: f32,
    pub max: f32,
}

pub struct CAttack {
    pub damage: f32,
    pub range: f32,
    pub cooldown: f32,
    pub last_attack: f32,
}

pub struct CTarget {
    pub entity: EntityId,
    pub priority: f32,
}

pub struct CAggro {
    pub table: BTreeMap<EntityId, f32>,  // Entity → aggro value
}
```

**Events**:
```rust
pub struct DamageEvent {
    pub source: EntityId,
    pub target: EntityId,
    pub amount: f32,
    pub timestamp: f32,
}

pub struct AttackEvent {
    pub attacker: EntityId,
    pub target: EntityId,
    pub timestamp: f32,
}

pub struct DeathEvent {
    pub entity: EntityId,
    pub timestamp: f32,
}
```

**Systems**:
```rust
// Stage 1: Target selection (ai_planning)
fn select_targets(world: &mut World) {
    for (entity, (position, aggro)) in world.query::<(&CPosition, &mut CAggro)>() {
        // Find nearby enemies, update aggro
        // Select highest aggro as CTarget
    }
}

// Stage 2: Attack execution (simulation)
fn execute_attacks(world: &mut World, events: &mut Events<AttackEvent>) {
    for (entity, (attack, target, position)) in 
        world.query::<(&mut CAttack, &CTarget, &CPosition)>()
    {
        if attack.can_attack(current_time) && in_range(position, target) {
            events.emit(AttackEvent {
                attacker: entity,
                target: target.entity,
                timestamp: current_time,
            });
            attack.last_attack = current_time;
        }
    }
}

// Stage 3: Apply damage (simulation)
fn apply_damage(
    world: &mut World,
    attack_events: &Events<AttackEvent>,
    damage_events: &mut Events<DamageEvent>,
) {
    for attack in attack_events.drain() {
        let damage = world.get::<CAttack>(attack.attacker)?.damage;
        
        damage_events.emit(DamageEvent {
            source: attack.attacker,
            target: attack.target,
            amount: damage,
            timestamp: attack.timestamp,
        });
        
        if let Some(health) = world.get_mut::<CHealth>(attack.target) {
            health.current -= damage;
        }
    }
}

// Stage 4: Death cleanup (simulation)
fn cleanup_deaths(
    world: &mut World,
    damage_events: &Events<DamageEvent>,
    death_events: &mut Events<DeathEvent>,
) {
    for damage in damage_events.iter() {
        if let Some(health) = world.get::<CHealth>(damage.target) {
            if health.current <= 0.0 {
                death_events.emit(DeathEvent {
                    entity: damage.target,
                    timestamp: damage.timestamp,
                });
                world.despawn(damage.target);
            }
        }
    }
}
```

**Data Format** (TOML):
```toml
[weapons.sword]
damage = 10.0
range = 1.5
cooldown = 1.0

[weapons.bow]
damage = 8.0
range = 10.0
cooldown = 2.0
```

**Tests**:
- Unit: Aggro table updates deterministically
- Unit: Attack cooldown enforced
- Integration: Deterministic duel over 100 ticks (same outcome)
- Integration: Event order stable (attack → damage → death)

**Estimated Effort**: 2 days

---

#### B.2: Crafting System (astraweave-gameplay)

**Components**:
```rust
pub struct CInventory {
    pub items: BTreeMap<String, u32>,  // Item ID → count
    pub capacity: u32,
}

pub struct CRecipeBook {
    pub recipes: Vec<RecipeId>,
}

pub struct CCraftingQueue {
    pub jobs: VecDeque<CraftingJob>,
}

pub struct CraftingJob {
    pub recipe_id: RecipeId,
    pub progress: f32,
    pub duration: f32,
}
```

**Recipe Format** (TOML):
```toml
[recipes.health_potion]
inputs = { herb = 2, water = 1 }
outputs = { health_potion = 1 }
duration = 5.0

[recipes.iron_sword]
inputs = { iron_ingot = 3, wood = 1 }
outputs = { iron_sword = 1 }
duration = 10.0
```

**Systems**:
```rust
// Stage 1: Enqueue crafting jobs
fn enqueue_crafting(
    world: &mut World,
    recipes: &BTreeMap<RecipeId, Recipe>,
) {
    for (entity, (inventory, recipe_book, queue)) in
        world.query::<(&CInventory, &CRecipeBook, &mut CCraftingQueue)>()
    {
        // Check if can craft (has inputs, queue not full)
        // Enqueue job if valid
    }
}

// Stage 2: Process crafting jobs
fn process_crafting(
    world: &mut World,
    recipes: &BTreeMap<RecipeId, Recipe>,
    dt: f32,
) {
    for (entity, (inventory, queue)) in
        world.query::<(&mut CInventory, &mut CCraftingQueue)>()
    {
        if let Some(job) = queue.jobs.front_mut() {
            job.progress += dt;
            
            if job.progress >= job.duration {
                // Consume inputs
                let recipe = &recipes[&job.recipe_id];
                for (item, count) in &recipe.inputs {
                    *inventory.items.get_mut(item).unwrap() -= count;
                }
                
                // Produce outputs
                for (item, count) in &recipe.outputs {
                    *inventory.items.entry(item.clone()).or_insert(0) += count;
                }
                
                queue.jobs.pop_front();
            }
        }
    }
}
```

**Tests**:
- Unit: Recipe validation (inputs exist, outputs defined)
- Unit: Inventory delta correctness
- Integration: Craft health potion → inventory changes deterministically
- Integration: Failure paths (insufficient inputs, full inventory)

**Estimated Effort**: 1 day

---

#### B.3: Dialogue System (astraweave-gameplay)

**Graph Structure**:
```rust
pub struct DialogueGraph {
    pub nodes: BTreeMap<String, DialogueNode>,
    pub start_node: String,
}

pub enum DialogueNode {
    Line {
        speaker: String,
        text: String,
        next: String,
    },
    Choice {
        speaker: String,
        text: String,
        choices: Vec<Choice>,
    },
    Condition {
        check: ConditionExpr,
        if_true: String,
        if_false: String,
    },
}

pub struct Choice {
    pub text: String,
    pub conditions: Vec<ConditionExpr>,  // Gate choice
    pub effects: Vec<EffectExpr>,        // Apply on select
    pub next: String,
}

pub enum ConditionExpr {
    HasItem(String, u32),
    StatGreaterThan(String, f32),
    PersonaTrait(String),
}
```

**Components**:
```rust
pub struct CDialogueState {
    pub graph_id: String,
    pub current_node: String,
    pub history: Vec<String>,  // Node IDs visited
}

pub struct CPersona {
    pub traits: BTreeSet<String>,  // e.g., "brave", "cautious"
    pub stats: BTreeMap<String, f32>,
}
```

**Systems**:
```rust
fn advance_dialogue(
    world: &mut World,
    graphs: &BTreeMap<String, DialogueGraph>,
    events: &mut Events<DialogueEvent>,
) {
    for (entity, (state, persona)) in
        world.query::<(&mut CDialogueState, &CPersona)>()
    {
        let graph = &graphs[&state.graph_id];
        let node = &graph.nodes[&state.current_node];
        
        match node {
            DialogueNode::Line { next, .. } => {
                state.history.push(state.current_node.clone());
                state.current_node = next.clone();
            }
            DialogueNode::Condition { check, if_true, if_false } => {
                let result = evaluate_condition(check, world, entity, persona);
                state.current_node = if result {
                    if_true.clone()
                } else {
                    if_false.clone()
                };
            }
            DialogueNode::Choice { choices, .. } => {
                // Wait for player input (external event)
                // For AI: select first valid choice deterministically
            }
        }
    }
}
```

**Data Format** (TOML):
```toml
[dialogue.merchant_greeting.start]
type = "Line"
speaker = "Merchant"
text = "Welcome, traveler! What can I do for you?"
next = "choice_action"

[dialogue.merchant_greeting.choice_action]
type = "Choice"
speaker = "Player"
text = "What would you like to do?"

[[dialogue.merchant_greeting.choice_action.choices]]
text = "I'd like to buy something."
next = "show_shop"

[[dialogue.merchant_greeting.choice_action.choices]]
text = "Just browsing."
conditions = [{ type = "PersonaTrait", trait = "curious" }]
next = "ask_about_rumors"
```

**Tests**:
- Unit: Graph validation (all nodes reachable, no dangling refs)
- Unit: Condition evaluation deterministic
- Integration: Branching dialogue produces expected transcript
- Integration: Persona traits gate choices correctly

**Estimated Effort**: 1 day

---

### Phase C: Weaving System (Days 9-11)

#### C.1: Pattern Detection (astraweave-weaving)

**Goal**: Scan ECS state for patterns/tensions → propose emergent intents

**Components**:
```rust
pub struct CWeaveAgent {
    pub patterns_detected: BTreeMap<String, f32>,  // Pattern ID → strength
    pub last_scan: f32,
}

pub struct CWeaveSignal {
    pub kind: String,
    pub strength: f32,
    pub seed: u64,  // Deterministic RNG seed
    pub metadata: BTreeMap<String, String>,
}
```

**Pattern Detector**:
```rust
pub trait PatternDetector {
    fn detect(&self, world: &World) -> Vec<(String, f32)>;
}

// Example: Low health cluster
pub struct LowHealthClusterDetector {
    pub threshold: f32,
    pub min_cluster_size: usize,
}

impl PatternDetector for LowHealthClusterDetector {
    fn detect(&self, world: &World) -> Vec<(String, f32)> {
        let low_health_entities: Vec<_> = world
            .query::<(&CHealth, &CPosition)>()
            .filter(|(h, _)| h.current / h.max < self.threshold)
            .collect();
        
        // Cluster by proximity
        let clusters = cluster_by_proximity(&low_health_entities, 10.0);
        
        clusters.into_iter()
            .filter(|c| c.len() >= self.min_cluster_size)
            .map(|c| {
                let strength = c.len() as f32 / 10.0;  // Normalize
                ("low_health_cluster".into(), strength)
            })
            .collect()
    }
}
```

**System**:
```rust
fn detect_patterns(
    world: &mut World,
    detectors: &[Box<dyn PatternDetector>],
    current_time: f32,
) {
    for (entity, weave_agent) in world.query::<&mut CWeaveAgent>() {
        if current_time - weave_agent.last_scan < 5.0 {
            continue;  // Scan every 5 seconds
        }
        
        weave_agent.patterns_detected.clear();
        
        for detector in detectors {
            for (pattern_id, strength) in detector.detect(world) {
                weave_agent.patterns_detected
                    .entry(pattern_id)
                    .or_insert(0.0)
                    .max(strength);  // Combine strengths
            }
        }
        
        weave_agent.last_scan = current_time;
    }
}
```

**Estimated Effort**: 1 day

---

#### C.2: Intent Proposal & Adjudication (astraweave-weaving)

**Intent**:
```rust
pub struct WeaveIntent {
    pub kind: String,  // e.g., "spawn_scavenger_event"
    pub priority: f32,
    pub cost: u32,     // Budget cost
    pub cooldown_key: String,
    pub payload: BTreeMap<String, String>,  // Event-specific data
}
```

**Proposer**:
```rust
pub trait IntentProposer {
    fn propose(&self, patterns: &BTreeMap<String, f32>, seed: u64) -> Vec<WeaveIntent>;
}

// Example: Spawn aid event for low health cluster
pub struct AidEventProposer;

impl IntentProposer for AidEventProposer {
    fn propose(&self, patterns: &BTreeMap<String, f32>, seed: u64) -> Vec<WeaveIntent> {
        if let Some(&strength) = patterns.get("low_health_cluster") {
            if strength > 0.5 {
                return vec![WeaveIntent {
                    kind: "spawn_aid_event".into(),
                    priority: strength,
                    cost: 10,
                    cooldown_key: "aid_event".into(),
                    payload: BTreeMap::from([
                        ("event_type".into(), "wandering_healer".into()),
                    ]),
                }];
            }
        }
        vec[]
    }
}
```

**Adjudicator**:
```rust
pub struct WeaveAdjudicator {
    pub budget: u32,
    pub cooldowns: BTreeMap<String, f32>,  // Cooldown key → expiry time
}

impl WeaveAdjudicator {
    pub fn adjudicate(
        &mut self,
        intents: Vec<WeaveIntent>,
        current_time: f32,
    ) -> Vec<WeaveIntent> {
        // Sort by priority (deterministic)
        let mut sorted = intents;
        sorted.sort_by(|a, b| {
            b.priority.partial_cmp(&a.priority)
                .unwrap_or(Ordering::Equal)
                .then_with(|| a.kind.cmp(&b.kind))  // Tie-break by name
        });
        
        let mut accepted = Vec::new();
        let mut budget_left = self.budget;
        
        for intent in sorted {
            // Check cooldown
            if let Some(&expiry) = self.cooldowns.get(&intent.cooldown_key) {
                if current_time < expiry {
                    continue;  // On cooldown
                }
            }
            
            // Check budget
            if intent.cost > budget_left {
                continue;  // Can't afford
            }
            
            // Accept
            accepted.push(intent.clone());
            budget_left -= intent.cost;
            self.cooldowns.insert(
                intent.cooldown_key.clone(),
                current_time + 60.0,  // 60s cooldown
            );
        }
        
        accepted
    }
}
```

**System**:
```rust
fn propose_and_adjudicate_intents(
    world: &mut World,
    proposers: &[Box<dyn IntentProposer>],
    adjudicator: &mut WeaveAdjudicator,
    events: &mut Events<WeaveIntentEvent>,
    current_time: f32,
) {
    for (entity, weave_agent) in world.query::<&CWeaveAgent>() {
        let seed = entity.id() as u64 ^ (current_time as u64);  // Deterministic seed
        
        let mut all_intents = Vec::new();
        for proposer in proposers {
            all_intents.extend(proposer.propose(&weave_agent.patterns_detected, seed));
        }
        
        let accepted = adjudicator.adjudicate(all_intents, current_time);
        
        for intent in accepted {
            events.emit(WeaveIntentEvent { intent });
        }
    }
}
```

**Config** (TOML):
```toml
[weaving.budget]
initial = 100
regen_per_second = 1.0

[weaving.patterns.low_health_cluster]
threshold = 0.3
min_cluster_size = 3

[weaving.cooldowns]
aid_event = 60.0
scavenger_event = 120.0
```

**Tests**:
- Unit: Same patterns → same intents (deterministic seed)
- Unit: Budget enforcement (can't exceed limit)
- Unit: Cooldown enforcement (can't trigger twice in window)
- Integration: Pattern → intent → ECS event flow validated

**Estimated Effort**: 2 days

---

### Phase D: Procedural Content Generation (Days 12-14)

#### D.1: Seed RNG Wrapper (astraweave-pcg)

**Goal**: Deterministic RNG with explicit seeds per layer

```rust
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

pub struct SeedRng {
    inner: ChaCha8Rng,
    layer: String,  // For debugging
}

impl SeedRng {
    pub fn new(seed: u64, layer: &str) -> Self {
        Self {
            inner: ChaCha8Rng::seed_from_u64(seed),
            layer: layer.to_string(),
        }
    }
    
    pub fn fork(&mut self, sublayer: &str) -> Self {
        let subseed = self.inner.next_u64();
        Self::new(subseed, &format!("{}::{}", self.layer, sublayer))
    }
    
    pub fn gen_range<T>(&mut self, range: std::ops::Range<T>) -> T
    where
        T: rand::distributions::uniform::SampleUniform,
    {
        self.inner.gen_range(range)
    }
    
    pub fn choose<'a, T>(&mut self, slice: &'a [T]) -> Option<&'a T> {
        if slice.is_empty() {
            None
        } else {
            Some(&slice[self.gen_range(0..slice.len())])
        }
    }
}
```

**Tests**:
- Unit: Same seed → same sequence
- Unit: Fork creates independent but deterministic child

**Estimated Effort**: 0.5 days

---

#### D.2: Encounter Generation (astraweave-pcg)

**Goal**: Place deterministic encounter nodes from seeds + constraints

```rust
pub struct Encounter {
    pub kind: EncounterKind,
    pub position: IVec2,
    pub difficulty: f32,
    pub metadata: BTreeMap<String, String>,
}

pub enum EncounterKind {
    Combat { enemy_types: Vec<String>, count: u32 },
    Loot { items: Vec<String> },
    Ambient { event_id: String },
}

pub struct EncounterGenerator {
    pub constraints: EncounterConstraints,
}

pub struct EncounterConstraints {
    pub bounds: (IVec2, IVec2),  // Min/max position
    pub min_spacing: f32,
    pub difficulty_range: (f32, f32),
}

impl EncounterGenerator {
    pub fn generate(
        &self,
        rng: &mut SeedRng,
        count: u32,
    ) -> Vec<Encounter> {
        let mut encounters = Vec::new();
        let mut positions = BTreeSet::new();  // Track used positions
        
        for _ in 0..count {
            // Generate position with spacing constraint
            let pos = loop {
                let x = rng.gen_range(self.constraints.bounds.0.x..=self.constraints.bounds.1.x);
                let y = rng.gen_range(self.constraints.bounds.0.y..=self.constraints.bounds.1.y);
                let candidate = IVec2::new(x, y);
                
                if positions.iter().all(|&p: &IVec2| {
                    (p - candidate).length() >= self.constraints.min_spacing
                }) {
                    break candidate;
                }
            };
            positions.insert(pos);
            
            // Generate encounter type
            let kind = match rng.gen_range(0..3) {
                0 => EncounterKind::Combat {
                    enemy_types: vec!["goblin".into()],
                    count: rng.gen_range(1..=3),
                },
                1 => EncounterKind::Loot {
                    items: vec!["health_potion".into()],
                },
                _ => EncounterKind::Ambient {
                    event_id: "ambient_npc".into(),
                },
            };
            
            encounters.push(Encounter {
                kind,
                position: pos,
                difficulty: rng.gen_range(self.constraints.difficulty_range.0..=self.constraints.difficulty_range.1),
                metadata: BTreeMap::new(),
            });
        }
        
        encounters
    }
}
```

**Tests**:
- Unit: Same seed → same encounters (positions, types)
- Unit: Spacing constraint satisfied
- Integration: Snapshot of generated set equals baseline

**Estimated Effort**: 1 day

---

#### D.3: Layout Generation (astraweave-pcg)

**Goal**: Grid/graph layouts with constraints

```rust
pub struct LayoutGenerator {
    pub grid_size: IVec2,
    pub room_min_size: IVec2,
    pub room_max_size: IVec2,
}

pub struct Room {
    pub bounds: (IVec2, IVec2),
    pub connections: Vec<usize>,  // Indices of connected rooms
}

impl LayoutGenerator {
    pub fn generate_rooms(&self, rng: &mut SeedRng, count: u32) -> Vec<Room> {
        let mut rooms = Vec::new();
        
        for _ in 0..count {
            let width = rng.gen_range(self.room_min_size.x..=self.room_max_size.x);
            let height = rng.gen_range(self.room_min_size.y..=self.room_max_size.y);
            
            let x = rng.gen_range(0..=(self.grid_size.x - width));
            let y = rng.gen_range(0..=(self.grid_size.y - height));
            
            rooms.push(Room {
                bounds: (IVec2::new(x, y), IVec2::new(x + width, y + height)),
                connections: Vec::new(),
            });
        }
        
        // Connect rooms (minimum spanning tree or similar)
        for i in 0..rooms.len() - 1 {
            rooms[i].connections.push(i + 1);
            rooms[i + 1].connections.push(i);
        }
        
        rooms
    }
}
```

**Tests**:
- Unit: Same seed → same layout
- Unit: Rooms fit within grid
- Integration: Graph connectivity (all reachable)

**Estimated Effort**: 1 day

---

#### D.4: ECS Integration (astraweave-pcg)

**Plugin**:
```rust
pub struct PCGPlugin {
    pub seed: u64,
}

impl Plugin for PCGPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(PCGSeed(self.seed));
        app.add_system(Stage::Startup, generate_initial_content);
    }
}

fn generate_initial_content(world: &mut World, seed: &PCGSeed) {
    let mut rng = SeedRng::new(seed.0, "initial_gen");
    
    // Generate encounters
    let encounter_gen = EncounterGenerator {
        constraints: EncounterConstraints {
            bounds: (IVec2::new(0, 0), IVec2::new(100, 100)),
            min_spacing: 10.0,
            difficulty_range: (1.0, 5.0),
        },
    };
    
    let encounters = encounter_gen.generate(&mut rng, 10);
    
    // Spawn ECS entities
    for encounter in encounters {
        let entity = world.spawn();
        world.insert(entity, CPosition(encounter.position));
        
        match encounter.kind {
            EncounterKind::Combat { enemy_types, count } => {
                // Spawn enemies
            }
            EncounterKind::Loot { items } => {
                // Spawn loot
            }
            EncounterKind::Ambient { event_id } => {
                // Spawn ambient event
            }
        }
    }
}
```

**Tests**:
- Integration: Seed reproducibility (same seed → same entity spawns)
- Integration: Constraint satisfaction (all encounters in bounds)

**Estimated Effort**: 0.5 days

---

### Phase E: AI Core Loop Integration (Ongoing)

**Goal**: Wire BT/GOAP into Perception → Reasoning → Planning → Action

**Controller Selection**:
```rust
pub enum AiController {
    Rule,         // Simple rule-based (existing)
    BehaviorTree, // BT planner
    Goap,         // GOAP planner
    Llm,          // LLM with sandbox
}

pub struct CAiController {
    pub controller: AiController,
}

fn plan_with_controller(
    world: &mut World,
    clips: &[AnimationClip],
    trees: &BTreeMap<String, BehaviorTree>,
    recipes: &BTreeMap<RecipeId, Recipe>,
) {
    for (entity, controller) in world.query::<&CAiController>() {
        match controller.controller {
            AiController::Rule => {
                // Existing rule-based planning
            }
            AiController::BehaviorTree => {
                // Tick BT, emit intents
                if let Some(tree_component) = world.get_mut::<CBehaviorTree>(entity) {
                    // ... tick logic
                }
            }
            AiController::Goap => {
                // Plan with GOAP, emit intents
                if let Some(plan) = world.get_mut::<CPlan>(entity) {
                    // ... plan logic
                }
            }
            AiController::Llm => {
                // LLM planning with sandbox
            }
        }
    }
}
```

**Action → Gameplay Event Flow**:
```rust
// AI action emits intent
let intent = PlanIntent {
    entity,
    action_steps: vec![
        ActionStep::Attack { target: enemy_id },
    ],
};

// Validate through sandbox
if sandbox.validate_intent(&intent, world).is_ok() {
    // Convert to gameplay event
    events.emit(AttackEvent {
        attacker: entity,
        target: enemy_id,
        timestamp: current_time,
    });
}

// Gameplay system processes event
// (combat system applies damage, etc.)
```

**Tests**:
- Integration: Loop triggers correct systems (no circular deps)
- Integration: BT agent → attack intent → damage event
- Integration: GOAP agent → craft intent → inventory change

**Estimated Effort**: Ongoing throughout phases A-D

---

### Phase F: Examples & Demos (Days 15-16)

#### F.1: core_loop_bt_demo

**Goal**: Agent using BT to patrol, chase, attack

```rust
// examples/core_loop_bt_demo/main.rs

fn main() -> Result<()> {
    // Load BT from TOML
    let tree = BehaviorTree::from_toml("assets/bt/patrol_attack.toml")?;
    
    // Create ECS world
    let mut world = World::new();
    
    // Spawn agent with BT
    let agent = world.spawn();
    world.insert(agent, CPosition(IVec2::new(0, 0)));
    world.insert(agent, CHealth { current: 100.0, max: 100.0 });
    world.insert(agent, CBehaviorTree {
        tree_id: "patrol_attack".into(),
        state: BTreeMap::new(),
        blackboard: Blackboard::new(),
    });
    
    // Spawn enemy
    let enemy = world.spawn();
    world.insert(enemy, CPosition(IVec2::new(10, 10)));
    world.insert(enemy, CHealth { current: 50.0, max: 50.0 });
    
    // Run simulation
    for tick in 0..100 {
        let dt = 1.0 / 60.0;
        
        // Tick BT
        tick_behavior_trees(&mut world, &trees, dt);
        
        // Execute actions
        execute_attacks(&mut world, &mut events);
        apply_damage(&mut world, &events.drain(), &mut damage_events);
        
        println!("[{}] Agent: {:?}, Enemy health: {:.1}",
            tick,
            world.get::<CPosition>(agent),
            world.get::<CHealth>(enemy).map(|h| h.current).unwrap_or(0.0)
        );
    }
    
    Ok(())
}
```

**Expected Output**:
```
[0] Agent: (0, 0), Enemy health: 50.0
[1] Agent: (1, 1), Enemy health: 50.0  // Patrol
[2] Agent: (2, 2), Enemy health: 50.0
...
[10] Agent: (10, 10), Enemy health: 50.0  // In range, attacking
[11] Agent: (10, 10), Enemy health: 40.0
...
[50] Agent: (10, 10), Enemy health: 0.0  // Enemy dead
```

**Estimated Effort**: 1 day

---

#### F.2: core_loop_goap_demo

**Goal**: Agent achieves "has_food" via gather→craft→eat

```rust
// examples/core_loop_goap_demo/main.rs

fn main() -> Result<()> {
    // Create ECS world
    let mut world = World::new();
    
    // Spawn agent with GOAP
    let agent = world.spawn();
    world.insert(agent, CPosition(IVec2::new(0, 0)));
    world.insert(agent, CInventory {
        items: BTreeMap::new(),
        capacity: 10,
    });
    world.insert(agent, CGoals {
        goals: vec![GoapGoal {
            desired_state: WorldState::from([("has_food", true)]),
            priority: 1.0,
        }],
    });
    world.insert(agent, CActionsKnown {
        actions: vec![
            GoapAction {
                name: "gather_herbs".into(),
                cost: 5.0,
                preconditions: WorldState::new(),
                effects: WorldState::from([("has_herbs", true)]),
            },
            GoapAction {
                name: "craft_food".into(),
                cost: 3.0,
                preconditions: WorldState::from([("has_herbs", true)]),
                effects: WorldState::from([("has_food", true)]),
            },
        ],
    });
    world.insert(agent, CPlan {
        actions: Vec::new(),
        current_index: 0,
    });
    
    // Run simulation
    for tick in 0..100 {
        // Plan GOAP
        plan_goap(&mut world);
        
        // Execute plan
        execute_goap_actions(&mut world);
        
        let plan = world.get::<CPlan>(agent).unwrap();
        let inventory = world.get::<CInventory>(agent).unwrap();
        
        println!("[{}] Plan: {:?}, Inventory: {:?}",
            tick,
            plan.actions.iter().map(|a| &a.name).collect::<Vec<_>>(),
            inventory.items
        );
    }
    
    Ok(())
}
```

**Expected Output**:
```
[0] Plan: ["gather_herbs", "craft_food"], Inventory: {}
[1] Plan: ["gather_herbs", "craft_food"], Inventory: {}
[5] Plan: ["craft_food"], Inventory: {"herbs": 1}  // Gathered
[8] Plan: [], Inventory: {"herbs": 0, "food": 1}   // Crafted, goal achieved
```

**Estimated Effort**: 1 day

---

#### F.3: weaving_pcg_demo

**Goal**: Seed, generate encounters + weave emergent aid event

```rust
// examples/weaving_pcg_demo/main.rs

fn main() -> Result<()> {
    let seed = 42;  // Fixed seed for reproducibility
    
    // Generate initial encounters
    let mut rng = SeedRng::new(seed, "initial_gen");
    let encounter_gen = EncounterGenerator {
        constraints: EncounterConstraints {
            bounds: (IVec2::new(0, 0), IVec2::new(50, 50)),
            min_spacing: 10.0,
            difficulty_range: (1.0, 5.0),
        },
    };
    let encounters = encounter_gen.generate(&mut rng, 5);
    
    println!("Generated {} encounters with seed {}", encounters.len(), seed);
    for (i, enc) in encounters.iter().enumerate() {
        println!("  [{}] {:?} at {:?} (difficulty {:.1})",
            i, enc.kind, enc.position, enc.difficulty);
    }
    
    // Create ECS world
    let mut world = World::new();
    
    // Spawn encounters
    for encounter in encounters {
        let entity = world.spawn();
        world.insert(entity, CPosition(encounter.position));
        // ... spawn based on kind
    }
    
    // Add weave agent
    let weave_agent = world.spawn();
    world.insert(weave_agent, CWeaveAgent {
        patterns_detected: BTreeMap::new(),
        last_scan: 0.0,
    });
    
    // Simulate for a bit
    for tick in 0..100 {
        let current_time = tick as f32 * 0.1;
        
        // Detect patterns
        detect_patterns(&mut world, &detectors, current_time);
        
        // Propose intents
        propose_and_adjudicate_intents(
            &mut world,
            &proposers,
            &mut adjudicator,
            &mut events,
            current_time,
        );
        
        if tick % 10 == 0 {
            let agent = world.get::<CWeaveAgent>(weave_agent).unwrap();
            println!("[{:.1}s] Patterns: {:?}", current_time, agent.patterns_detected);
        }
    }
    
    Ok(())
}
```

**Expected Output**:
```
Generated 5 encounters with seed 42
  [0] Combat at (5, 12) (difficulty 2.3)
  [1] Loot at (23, 8) (difficulty 1.5)
  [2] Ambient at (41, 35) (difficulty 3.8)
  [3] Combat at (15, 47) (difficulty 4.2)
  [4] Loot at (38, 19) (difficulty 2.7)

[0.0s] Patterns: {}
[1.0s] Patterns: {}
[2.0s] Patterns: {"low_health_cluster": 0.6}  // Pattern detected
[3.0s] Patterns: {"low_health_cluster": 0.6}
  Weave Intent: spawn_aid_event (wandering_healer) accepted
```

**Estimated Effort**: 1 day

---

### Phase G: Documentation & Diagnostics (Ongoing)

**Documents to Create/Update**:
1. `docs/PHASE3_IMPLEMENTATION_PLAN.md` (this document)
2. `docs/PHASE3_STATUS_REPORT.md` (track task completion)
3. `docs/PHASE3_PROGRESS_REPORT.md` (commands, feature flags, seeds)
4. `astraweave-behavior/README.md`
5. `astraweave-gameplay/README.md`
6. `astraweave-weaving/README.md`
7. `astraweave-pcg/README.md`

**Diagnostics/Counters**:
```rust
pub struct DiagnosticCounters {
    pub bt_ticks: u64,
    pub goap_expansions: u64,
    pub weave_intents_proposed: u64,
    pub weave_intents_accepted: u64,
    pub pcg_entities_spawned: u64,
}

// Log every N frames
if frame % 60 == 0 {
    println!("Diagnostics: BT ticks={}, GOAP expansions={}, Weave intents={}/{}",
        counters.bt_ticks,
        counters.goap_expansions,
        counters.weave_intents_accepted,
        counters.weave_intents_proposed
    );
}
```

**Estimated Effort**: Ongoing, 0.5 days at end

---

## Timeline Summary

| Phase | Description | Days | Dependencies |
|-------|-------------|------|--------------|
| **A** | BT + GOAP planners | 4 | None |
| **B** | Gameplay systems (combat, crafting, dialogue) | 4 | None |
| **C** | Weaving system | 3 | None |
| **D** | PCG (seed RNG, encounters, layouts) | 3 | None |
| **E** | AI core loop integration | Ongoing | A, B |
| **F** | Examples & demos | 3 | A, B, C, D |
| **G** | Documentation & diagnostics | 1 | All |
| **Total** | | **18 days** | |

**Critical Path**: A → E → F → G  
**Parallel Work**: B, C, D can proceed independently

---

## Feature Flags

```toml
[features]
default = []

# AI planners
ai-bt = []
ai-goap = []

# Gameplay modules
gameplay-combat = []
gameplay-crafting = []
gameplay-dialogue = []

# Weaving & PCG
weaving = []
pcg = []

# Convenience bundles
all-gameplay = ["gameplay-combat", "gameplay-crafting", "gameplay-dialogue"]
all-ai = ["ai-bt", "ai-goap"]
all-phase3 = ["all-ai", "all-gameplay", "weaving", "pcg"]
```

---

## Acceptance Criteria

- [ ] **Behavior Trees**: Nodes, loader, ECS plugin, unit + integration tests pass
- [ ] **GOAP**: Domain, planner, ECS plugin, optimality & reproducibility tests pass
- [ ] **Gameplay ECS**: Combat, crafting, dialogue plugins implemented, deterministic tests pass
- [ ] **Weaving**: Pattern detection → intents → adjudication integrated, reproducible tests pass
- [ ] **PCG**: Seed-reproducible generation with constraints; integration tests pass
- [ ] **Core Loop**: BT/GOAP wired into Perception→Planning pipeline; action→gameplay events validated
- [ ] **Determinism**: Fixed-seed tests stable; CI defaults to CPU/seeded paths
- [ ] **CI Green**: `cargo fmt`, `clippy -D warnings`, unit/integration/golden where applicable
- [ ] **Docs Updated**: Plan, status, progress; roadmap Phase 3 items flipped to ✅ with links to tests/demos

---

## Commands Reference

```powershell
# Format & lint
cargo fmt --check
cargo clippy --workspace -- -D warnings

# Core tests (deterministic, no features)
cargo test -p astraweave-behavior
cargo test -p astraweave-gameplay
cargo test -p astraweave-weaving
cargo test -p astraweave-pcg

# Tests with features
cargo test --features ai-bt,ai-goap,gameplay-combat,gameplay-crafting,gameplay-dialogue,weaving,pcg

# Run demos
cargo run -p core_loop_bt_demo --features ai-bt,gameplay-combat
cargo run -p core_loop_goap_demo --features ai-goap,gameplay-crafting
cargo run -p weaving_pcg_demo --features weaving,pcg

# Full workspace check
cargo check --all-targets --all-features
```

---

**Plan Created**: October 1, 2025  
**Status**: 🚧 Ready to implement  
**Next Step**: Begin Phase A (Behavior Trees + GOAP)
