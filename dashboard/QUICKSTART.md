# Advanced GOAP tutorial

This file is not the engine install guide. Clone, toolchain, and run commands are in the repository [README](../README.md) and [build quick reference](guides/BUILD_QUICK_REFERENCE.md). Rust is pinned at **1.89.0**.

`AdvancedGOAP` and `GoalDefinition` exist behind the `planner_advanced` feature on `astraweave-ai`. That feature is not what a default `hello_companion` run enables. The types below are the feature-gated API, not the tick path that `nav_reroute` uses.

---

# Advanced GOAP - tutorial body

---

## Table of Contents
1. [Installation](#installation)
2. [Your First Goal](#your-first-goal)
3. [Testing the Goal](#testing-the-goal)
4. [Hierarchical Goals](#hierarchical-goals)
5. [Learning & Persistence](#learning--persistence)
6. [Validation & Debugging](#validation--debugging)
7. [Next Steps](#next-steps)

---

## Installation

### Prerequisites
- Rust 1.89.0 (the pin in `rust-toolchain.toml`)
- AstraWeave engine cloned

### Enable Advanced GOAP
Add to your `Cargo.toml`:

```toml
[dependencies]
astraweave-ai = { path = "../astraweave-ai", features = ["planner_advanced"] }
```

### Build
```bash
cargo build --features planner_advanced
```

---

## Your First Goal

### Step 1: Create a Simple Goal (TOML)

Create `goals/my_first_goal.toml`:

```toml
name = "defeat_enemy"
priority = 8.0
deadline_seconds = 60.0

[desired_state]
enemy_defeated = true
my_ammo = { min = 1, max = 100 }
```

### Step 2: Load and Use in Code

```rust
use astraweave_ai::goap::*;

// Load goal from file
let goal_def = GoalDefinition::load("goals/my_first_goal.toml")?;
let goal = goal_def.to_goal();

// Create planner
let mut planner = AdvancedGOAP::new();
register_all_actions(&mut planner); // Adds standard actions

// Create starting state
let mut world = WorldState::new();
world.set("my_ammo", StateValue::Int(30));
world.set("enemy_hp", StateValue::Int(100));
world.set("in_range", StateValue::Bool(true));

// Plan!
let plan = planner.plan(&world, &goal);

match plan {
    Some(actions) => {
        println!("Found plan with {} actions:", actions.len());
        for (i, action) in actions.iter().enumerate() {
            println!("  {}. {}", i + 1, action);
        }
    }
    None => println!("No plan found!"),
}
```

### Output:
```
Found plan with 2 actions:
  1. approach_enemy
  2. attack
```

---

## Testing the Goal

### Validate Your Goal

```rust
use astraweave_ai::goap::*;

let validator = GoalValidator::new();
let goal_def = GoalDefinition::load("goals/my_first_goal.toml")?;
let result = validator.validate(&goal_def);

if result.is_valid() {
    println!("✓ Goal is valid!");
} else {
    println!("✗ Goal has errors:");
    for error in result.errors {
        println!("  - {}", error.message);
    }
}
```

### Visualize the Plan

```rust
use astraweave_ai::goap::*;

let visualizer = PlanVisualizer::new(VisualizationFormat::AsciiTree);
let output = visualizer.visualize_plan(&plan, &actions, &history, &world);
println!("{}", output);
```

**Output:**
```
Plan (2 actions, cost: 3.0, risk: 0.15)
├─ approach_enemy (cost: 1.0, risk: 0.05)
└─ attack (cost: 2.0, risk: 0.1)
```

---

## Hierarchical Goals

### Create a Hierarchical Goal

`goals/assault_position.toml`:

```toml
name = "assault_position"
priority = 9.0
decomposition = "sequential"

[desired_state]
position_captured = true

[[sub_goals]]
name = "scout"
priority = 8.0

[sub_goals.desired_state]
area_scanned = true
enemies_located = true

[[sub_goals]]
name = "prepare"
priority = 8.5
decomposition = "parallel"

[sub_goals.desired_state]
ammo_loaded = true
team_coordinated = true

[[sub_goals.sub_goals]]
name = "reload"
[sub_goals.sub_goals.desired_state]
my_ammo = { min = 25, max = 100 }

[[sub_goals.sub_goals]]
name = "coordinate"
[sub_goals.sub_goals.desired_state]
team_ready = true

[[sub_goals]]
name = "attack"
priority = 10.0

[sub_goals.desired_state]
position_captured = true
```

### Use It:

```rust
let goal_def = GoalDefinition::load("goals/assault_position.toml")?;
let goal = goal_def.to_goal();

// Planner automatically handles hierarchy!
let plan = planner.plan(&world, &goal)?;

// Plan will include actions from all sub-goals in order:
// 1. scan
// 2. reload (parallel)
// 3. coordinate (parallel)
// 4. advance
// 5. capture
```

---

## Learning & Persistence

### Enable Learning

Create `config/goap_learning.toml`:

```toml
[learning]
enabled = true
min_success_rate = 0.1
max_success_rate = 0.9

[learning.smoothing]
method = "ewma"
ewma_alpha = 0.2

[cost_tuning]
risk_weight = 5.0
```

### Use Learning in Code:

```rust
use astraweave_ai::goap::*;

// Load config
let config = GOAPConfig::load("config/goap_learning.toml")?;
let manager = LearningManager::new(config);

// Load persistent history
let mut history = HistoryPersistence::load_or_default(
    "saves/goap_history.json",
    PersistenceFormat::Json
);

// After action execution:
if action_succeeded {
    history.record_success("attack", duration_seconds);
} else {
    history.record_failure("attack");
}

// Get learned probabilities
let success_prob = manager.get_probability("attack", &history);
println!("Attack success rate: {:.1}%", success_prob * 100.0);

// Save periodically
HistoryPersistence::save(&history, "saves/goap_history.json", PersistenceFormat::Json)?;
```

**Result:** Actions get smarter over time! Success rates improve from defaults to actual observed performance.

---

## Validation & Debugging

### Validate Goals

```rust
let validator = GoalValidator::new();
let result = validator.validate(&goal_def);

println!("Errors: {}", result.errors.len());
println!("Warnings: {}", result.warnings.len());
println!("Valid: {}", result.is_valid());

for error in result.errors {
    println!("ERROR: {}", error.message);
    if let Some(suggestion) = error.suggestion {
        println!("  Suggestion: {}", suggestion);
    }
}
```

### Debug Plans Step-by-Step

```rust
use astraweave_ai::goap::*;

let debugger = PlanDebugger::new(plan, start_state, actions);

// Step through plan
while !debugger.at_end() {
    println!("Step {}: {}", debugger.current_step(), debugger.current_action().unwrap());
    
    // Show state changes
    if let Some(diff) = debugger.get_state_diff() {
        println!("  Changed: {} variables", diff.changed.len());
    }
    
    debugger.step_forward()?;
}

// Check goal progress
let progress = debugger.check_goal_progress(&goal);
println!("Goal progress: {:.1}%", progress.progress * 100.0);
```

### Analyze Plan Quality

```rust
use astraweave_ai::goap::*;

let metrics = PlanAnalyzer::analyze(&plan, &actions, &history, &start);

println!("=== Plan Analysis ===");
println!("Total Cost: {:.2}", metrics.total_cost);
println!("Total Risk: {:.2}", metrics.total_risk);
println!("Success Probability: {:.1}%", metrics.success_probability * 100.0);
println!("Estimated Duration: {:.1}s", metrics.estimated_duration);

// Get optimization suggestions
let suggestions = PlanAnalyzer::suggest_optimizations(&metrics);
for suggestion in suggestions.iter().take(3) {
    println!("- [{:?}] {}", suggestion.priority, suggestion.message);
}
```

---

## Multi-Goal Scheduling

### Managing Multiple Goals

```rust
use astraweave_ai::goap::*;

let mut scheduler = GoalScheduler::new();

// Add multiple concurrent goals
let escort_goal = GoalDefinition::load("goals/escort.toml")?.to_goal();
let defend_goal = GoalDefinition::load("goals/defend.toml")?.to_goal();
let resupply_goal = GoalDefinition::load("goals/resupply.toml")?.to_goal();

scheduler.add_goal(escort_goal);   // Priority 10
scheduler.add_goal(defend_goal);   // Priority 8
scheduler.add_goal(resupply_goal); // Priority 3

// Each frame/update:
let plan = scheduler.update(current_time, &world, &planner);

if let Some(actions) = plan {
    println!("Executing: {:?}", actions);
}

println!("Active goals: {}", scheduler.goal_count());
```

**Scheduler automatically**:
- Picks most urgent goal
- Removes satisfied goals
- Removes expired goals
- Preempts for urgent deadlines

---

## Common Patterns

### Pattern 1: Combat Goal

```toml
name = "engage_enemy"
priority = 8.0
decomposition = "sequential"

[desired_state]
enemy_defeated = true

[[sub_goals]]
name = "get_in_range"
[sub_goals.desired_state]
enemy_distance = { min = 0, max = 8 }

[[sub_goals]]
name = "attack_enemy"
[sub_goals.desired_state]
enemy_defeated = true
```

### Pattern 2: Survival Goal

```toml
name = "stay_alive"
priority = 10.0
decomposition = "any_of"  # Any method works

[desired_state]
my_hp = { min = 50, max = 100 }

[[sub_goals]]
name = "heal"
[sub_goals.desired_state]
my_hp = { min = 70, max = 100 }

[[sub_goals]]
name = "take_cover"
[sub_goals.desired_state]
in_cover = true

[[sub_goals]]
name = "retreat"
[sub_goals.desired_state]
distance_from_danger = { min = 20, max = 100 }
```

### Pattern 3: Support Goal

```toml
name = "support_player"
priority = 7.0
decomposition = "parallel"  # Do simultaneously

[desired_state]
player_supported = true

[[sub_goals]]
name = "cover_fire"
[sub_goals.desired_state]
providing_cover = true

[[sub_goals]]
name = "watch_flank"
[sub_goals.desired_state]
flank_secure = true
```

---

## Tips & Tricks

### 1. Start Simple
Begin with flat goals (no sub-goals). Add hierarchy as needed.

### 2. Validate Early
Always validate goals before runtime:
```rust
let result = validator.validate(&goal_def);
assert!(result.is_valid());
```

### 3. Use Ranges for Flexibility
```toml
# Too rigid:
health = 100

# Better:
health = { min = 70, max = 100 }
```

### 4. Choose Right Decomposition
- **Sequential**: Order matters (scout → prepare → attack)
- **Parallel**: Independent tasks (reload + scan + coordinate)
- **AnyOf**: Alternatives (smoke OR suppress OR eliminate)
- **AllOf**: All required (clear + secure + fortify)

### 5. Monitor Learning
```rust
let stats = history.get_action_stats("attack");
if let Some(stats) = stats {
    println!("Attack: {}% success over {} attempts",
             stats.success_rate() * 100.0, stats.executions);
}
```

### 6. Visualize for Debugging
When plans don't work as expected:
```rust
// Try different formats
let visualizer = PlanVisualizer::new(VisualizationFormat::AsciiTimeline);
let viz = visualizer.visualize_plan(&plan, &actions, &history, &world);
println!("{}", viz);
```

---

## Troubleshooting

### Problem: "No plan found"

**Causes**:
1. Goal is impossible from current state
2. No actions available that achieve goal
3. Preconditions never satisfied

**Solutions**:
```rust
// Check if goal is already satisfied
if goal.is_satisfied(&world) {
    println!("Goal already satisfied!");
}

// Check what's missing
let unmet = goal.unmet_conditions(&world);
for (key, value) in unmet {
    println!("Missing: {} = {:?}", key, value);
}

// Verify actions are registered
println!("Actions available: {}", planner.action_count());
```

### Problem: "Validation errors"

```rust
let result = validator.validate(&goal_def);
for error in result.errors {
    println!("ERROR in {}: {}", 
             error.field.unwrap_or("unknown".to_string()), 
             error.message);
    if let Some(fix) = error.suggestion {
        println!("  Fix: {}", fix);
    }
}
```

### Problem: "Plans are too long"

```rust
// Analyze and optimize
let metrics = PlanAnalyzer::analyze(&plan, &actions, &history, &world);
if metrics.action_count > 10 {
    let suggestions = PlanAnalyzer::suggest_optimizations(&metrics);
    for suggestion in suggestions {
        println!("{}", suggestion.message);
    }
}
```

---

## Next Steps

### Learn More
- **Designer Guide**: `docs/hierarchical_goals_designer_guide.md` (50+ pages)
- **Phase 3 Complete**: `docs/PHASE3_COMPLETE.md` (Learning & Persistence)
- **Phase 4 Complete**: `docs/PHASE4_COMPLETE.md` (Hierarchical Goals)
- **Examples**: `examples/goal_templates/` (6 complete templates)

### Explore Examples
```bash
# Look at real goal templates
ls examples/goal_templates/
- escort_mission.toml
- defend_position.toml
- assault_position.toml
- revive_and_protect.toml
- patrol_area.toml
- goal_library_example.toml
```

### Join the Community
- Report issues on GitHub
- Share your goals and improvements
- Contribute new action templates

---

## Complete Example

Here's a full working example:

```rust
use astraweave_ai::goap::*;
use anyhow::Result;

fn main() -> Result<()> {
    // 1. Create planner
    let mut planner = AdvancedGOAP::new();
    register_all_actions(&mut planner);

    // 2. Load goal
    let goal_def = GoalDefinition::load("goals/escort.toml")?;
    let goal = goal_def.to_goal();

    // 3. Validate goal
    let validator = GoalValidator::new();
    let validation = validator.validate(&goal_def);
    if !validation.is_valid() {
        eprintln!("Goal validation failed!");
        return Ok(());
    }

    // 4. Create world state
    let mut world = WorldState::new();
    world.set("my_ammo", StateValue::Int(30));
    world.set("player_x", StateValue::Int(10));
    world.set("player_y", StateValue::Int(10));

    // 5. Plan
    let plan = planner.plan(&world, &goal)
        .ok_or_else(|| anyhow::anyhow!("No plan found"))?;

    // 6. Visualize
    let visualizer = PlanVisualizer::new(VisualizationFormat::AsciiTree);
    let output = visualizer.visualize_plan(
        &plan, 
        &planner.actions, 
        planner.get_history(), 
        &world
    );
    println!("{}", output);

    // 7. Analyze
    let metrics = PlanAnalyzer::analyze(
        &plan, 
        &planner.actions, 
        planner.get_history(), 
        &world
    );
    println!("\nPlan Quality:");
    println!("  Cost: {:.2}", metrics.total_cost);
    println!("  Risk: {:.2}", metrics.total_risk);
    println!("  Success: {:.1}%", metrics.success_probability * 100.0);

    Ok(())
}
```

---

## Summary

**You now know how to**:
✅ Create goals in TOML  
✅ Load and plan with hierarchical goals  
✅ Enable learning and persistence  
✅ Validate goals before runtime  
✅ Debug plans step-by-step  
✅ Analyze plan quality  
✅ Use multi-goal scheduling  

**Total time**: ~10 minutes to get started! 🚀

---

## Quick Reference Card

```rust
// Load goal
let goal = GoalDefinition::load("path.toml")?.to_goal();

// Validate
let result = GoalValidator::new().validate(&goal_def);

// Plan
let plan = planner.plan(&world, &goal)?;

// Visualize
let viz = PlanVisualizer::new(VisualizationFormat::AsciiTree);
println!("{}", viz.visualize_plan(&plan, &actions, &history, &world));

// Analyze
let metrics = PlanAnalyzer::analyze(&plan, &actions, &history, &world);

// Debug
let mut debugger = PlanDebugger::new(plan, world, actions);
debugger.step_forward()?;

// Learn
history.record_success("attack", duration);
HistoryPersistence::save(&history, "history.json", PersistenceFormat::Json)?;
```

---

**Happy Planning!** 🎯

*For issues or questions, see `docs/hierarchical_goals_designer_guide.md` or the README.*

