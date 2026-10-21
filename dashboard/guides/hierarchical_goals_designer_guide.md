# Hierarchical GOAP Goals - Designer Guide

## Overview

The Advanced GOAP (Goal-Oriented Action Planning) system in AstraWeave enables you to create complex, intelligent AI behavior through hierarchical goal definitions. This guide will show you how to author goals in TOML format without touching code.

## Table of Contents

1. [Basic Concepts](#basic-concepts)
2. [Goal Structure](#goal-structure)
3. [Decomposition Strategies](#decomposition-strategies)
4. [State Values](#state-values)
5. [Authoring Workflow](#authoring-workflow)
6. [Example Scenarios](#example-scenarios)
7. [Best Practices](#best-practices)
8. [Troubleshooting](#troubleshooting)

---

## Basic Concepts

### What is a Goal?

A **goal** represents a desired state that the AI wants to achieve. For example:
- "Enemy is defeated"
- "Player is at extraction point"
- "Position is secured"

### Hierarchical Goals

**Hierarchical goals** break down complex objectives into smaller, manageable sub-goals. This mimics human planning:

```
Goal: "Assault Enemy Position"
├─ Sub-Goal: "Reconnaissance"
├─ Sub-Goal: "Prepare Assault"
│  ├─ Sub-Sub-Goal: "Reload Weapons"
│  └─ Sub-Sub-Goal: "Check Equipment"
└─ Sub-Goal: "Execute Assault"
```

### How Planning Works

1. AI evaluates the current world state
2. Identifies which goals to pursue based on priority and urgency
3. Decomposes hierarchical goals into sub-goals
4. Plans a sequence of actions to achieve each goal
5. Executes the plan, adapting as conditions change

---

## Goal Structure

### Basic Goal Definition (TOML)

```toml
name = "survive_combat"
priority = 9.0           # Higher = more important (0-10 scale)
deadline_seconds = 30.0  # Optional time limit
decomposition = "sequential"  # How to handle sub-goals
max_depth = 5            # Max levels of hierarchy (safety limit)

[desired_state]
# Conditions that must be true for goal to be satisfied
health = { min = 50, max = 100 }
in_cover = true
enemies_nearby = { min = 0, max = 2 }
```

### Goal with Sub-Goals

```toml
name = "defend_position"
priority = 8.0
decomposition = "sequential"

[desired_state]
position_held = true

[[sub_goals]]
name = "take_cover"
priority = 7.0

[sub_goals.desired_state]
in_cover = true

[[sub_goals]]
name = "suppress_enemies"
priority = 8.0

[sub_goals.desired_state]
enemies_suppressed = true
```

---

## Decomposition Strategies

Decomposition strategies determine how the planner handles sub-goals:

### 1. Sequential
**Use when:** Sub-goals must be completed in order.

```toml
decomposition = "sequential"
```

**Example:** "Assault Position"
1. Scout the area first
2. Then prepare equipment
3. Finally execute assault

**Planning Behavior:**
- Sub-goals are attempted in the order defined
- If one fails, the entire goal fails
- State from previous sub-goals carries forward

### 2. Parallel (All-Of)
**Use when:** All sub-goals must be achieved, but order doesn't matter.

```toml
decomposition = "parallel"
```

**Example:** "Prepare for Combat"
- Load weapon
- Check armor
- Coordinate with team

**Planning Behavior:**
- Planner orders sub-goals by priority
- All must succeed for parent goal to succeed
- More efficient than sequential when independence exists

### 3. Any-Of
**Use when:** Achieving any one sub-goal satisfies the parent.

```toml
decomposition = "any_of"
```

**Example:** "Create Safe Zone"
- Option A: Throw smoke grenade
- Option B: Suppress enemies
- Option C: Eliminate nearby threats

**Planning Behavior:**
- Tries each sub-goal in priority order
- First successful plan is used
- Great for alternative approaches

### 4. All-Of
**Use when:** All sub-goals must succeed (similar to parallel, but enforces completion).

```toml
decomposition = "all_of"
```

**Example:** "Secure Area"
- Clear all enemies
- Establish perimeter
- Set up defenses

---

## State Values

State values describe conditions in the game world. The GOAP planner matches these against actual game state.

### Boolean

```toml
in_cover = true
has_weapon = false
```

### Integer

```toml
ammo_count = 30
enemy_count = 5
```

### Integer Range (Flexible Matching)

```toml
# Satisfied if actual value is between 50 and 100
health = { min = 50, max = 100 }

# "At least 5 ammo"
ammo = { min = 5, max = 999 }

# "No more than 2 enemies"
enemies_nearby = { min = 0, max = 2 }
```

### Float

```toml
distance_to_player = 10.5
time_remaining = 30.0
```

### Float Approximate (With Tolerance)

```toml
# Satisfied if actual value is 100.0 ± 5.0
target_health = { value = 100.0, tolerance = 5.0 }
```

### String

```toml
stance = "crouched"
weapon_type = "rifle"
```

---

## Authoring Workflow

### Step 1: Define the High-Level Goal

Start with what you want the AI to ultimately achieve:

```toml
name = "escort_player"
priority = 10.0
deadline_seconds = 300.0

[desired_state]
player_at_extraction = true
player_alive = true
```

### Step 2: Break Down into Sub-Goals

Identify the steps needed:

```toml
[[sub_goals]]
name = "clear_path"
# ...

[[sub_goals]]
name = "stay_close"
# ...

[[sub_goals]]
name = "reach_extraction"
# ...
```

### Step 3: Define State Conditions

For each goal, specify what "done" looks like:

```toml
[sub_goals.desired_state]
path_clear = true
threats_neutralized = true
```

### Step 4: Choose Decomposition Strategy

Think about the relationship between sub-goals:
- Must they happen in order? → `sequential`
- Are they independent? → `parallel`
- Are they alternatives? → `any_of`

### Step 5: Set Priorities and Deadlines

```toml
priority = 9.0  # How important (0-10)
deadline_seconds = 60.0  # How urgent (optional)
```

**Priority Rules:**
- 10 = Critical (must do immediately)
- 7-9 = High (important objectives)
- 4-6 = Medium (tactical goals)
- 1-3 = Low (opportunistic actions)

**Deadline Effects:**
- As deadline approaches, goal urgency increases dramatically
- Urgent goals can preempt higher-priority goals with distant deadlines
- Goals past deadline are auto-removed

### Step 6: Test and Iterate

1. Load your goal in-game
2. Observe AI behavior
3. Adjust priorities, deadlines, or decomposition strategies
4. Verify state conditions match actual game values

---

## Example Scenarios

### Example 1: Escort Mission

**File:** `escort_mission.toml`

```toml
name = "escort_player_to_extraction"
priority = 10.0
deadline_seconds = 300.0
decomposition = "sequential"

[desired_state]
player_at_extraction = true
player_alive = true

[[sub_goals]]
name = "clear_path_to_extraction"
priority = 9.0
decomposition = "sequential"

[sub_goals.desired_state]
path_clear = true

[[sub_goals.sub_goals]]
name = "scout_route"
priority = 8.0

[sub_goals.sub_goals.desired_state]
route_scanned = true

[[sub_goals.sub_goals]]
name = "eliminate_threats"
priority = 9.0

[sub_goals.sub_goals.desired_state]
threats_in_path = 0

[[sub_goals]]
name = "stay_close_to_player"
priority = 8.0

[sub_goals.desired_state]
distance_to_player = { min = 0, max = 10 }

[[sub_goals]]
name = "reach_extraction"
priority = 10.0

[sub_goals.desired_state]
at_extraction = true
```

**Behavior:**
1. AI scouts the route to extraction
2. Eliminates any threats along the path
3. Stays within 10 units of player
4. Guides player to extraction point

### Example 2: Revive Under Fire

**File:** `revive_and_protect.toml`

```toml
name = "revive_and_protect"
priority = 10.0
deadline_seconds = 60.0
decomposition = "sequential"

[desired_state]
ally_revived = true
ally_safe = true

[[sub_goals]]
name = "create_safe_zone"
priority = 9.5
decomposition = "any_of"  # Multiple ways to create safety

[sub_goals.desired_state]
safe_zone_established = true

[[sub_goals.sub_goals]]
name = "throw_smoke"
priority = 10.0

[sub_goals.sub_goals.desired_state]
smoke_deployed = true

[[sub_goals.sub_goals]]
name = "suppress_threats"
priority = 9.0

[sub_goals.sub_goals.desired_state]
enemies_suppressed = true

[[sub_goals.sub_goals]]
name = "eliminate_threats"
priority = 8.0

[sub_goals.sub_goals.desired_state]
nearby_enemies = 0

[[sub_goals]]
name = "reach_ally"
priority = 10.0

[sub_goals.desired_state]
distance_to_ally = { min = 0, max = 2 }

[[sub_goals]]
name = "perform_revive"
priority = 10.0

[sub_goals.desired_state]
ally_health = { min = 1, max = 100 }
```

**Behavior:**
1. AI chooses fastest way to create safety (smoke, suppression, or elimination)
2. Moves to downed ally
3. Performs revive action
4. Ensures ally has health > 0

### Example 3: Adaptive Defense

**File:** `defend_position.toml`

```toml
name = "defend_position"
priority = 8.0
deadline_seconds = 180.0
decomposition = "parallel"  # Can do simultaneously

[desired_state]
position_held = true
enemy_count = { min = 0, max = 2 }

[[sub_goals]]
name = "take_cover"
priority = 7.0

[sub_goals.desired_state]
in_cover = true
cover_quality = { min = 70, max = 100 }

[[sub_goals]]
name = "suppress_enemies"
priority = 8.0

[sub_goals.desired_state]
enemies_suppressed = true

[[sub_goals]]
name = "call_reinforcements"
priority = 5.0

[sub_goals.desired_state]
reinforcements_called = true
```

**Behavior:**
- AI finds good cover (quality ≥ 70)
- Suppresses enemies while in cover
- Calls for backup if available
- Holds position until enemies reduced to ≤ 2

---

## Best Practices

### 1. Keep Hierarchies Shallow (≤ 3 Levels)

**Good:**
```
Root Goal
├─ Sub-Goal 1
└─ Sub-Goal 2
```

**Avoid:**
```
Root Goal
├─ Sub-Goal
│  └─ Sub-Sub-Goal
│     └─ Sub-Sub-Sub-Goal
│        └─ Sub-Sub-Sub-Sub-Goal  ← Too deep!
```

**Why:** Deep hierarchies are slow to plan and hard to debug.

### 2. Use Clear, Descriptive Names

**Good:**
```toml
name = "eliminate_nearby_threats"
name = "advance_to_cover"
name = "reload_primary_weapon"
```

**Avoid:**
```toml
name = "do_thing"
name = "step1"
name = "action_x"
```

### 3. Set Realistic Deadlines

**Too Short:**
```toml
deadline_seconds = 2.0  # Unrealistic for complex goal
```

**Too Long:**
```toml
deadline_seconds = 3600.0  # 1 hour - effectively no urgency
```

**Good:**
```toml
deadline_seconds = 60.0  # 1 minute - creates urgency without being impossible
```

### 4. Use Ranges for Flexibility

**Rigid:**
```toml
health = 100  # Must be exactly 100
```

**Flexible:**
```toml
health = { min = 75, max = 100 }  # "Healthy" range
```

### 5. Balance Priorities

Don't make everything priority 10!

**Distribution:**
- Priority 10: Life-or-death situations (1-2 goals max)
- Priority 7-9: Core mission objectives
- Priority 4-6: Tactical advantages
- Priority 1-3: Optional/opportunistic

### 6. Test Incrementally

1. Start with a simple goal (no sub-goals)
2. Verify it works
3. Add one sub-goal at a time
4. Test after each addition

### 7. Use Appropriate Decomposition

**Sequential:** When order matters
```toml
# Must scout before attacking
[[sub_goals]]
name = "scout"

[[sub_goals]]
name = "attack"
```

**Any-Of:** When alternatives exist
```toml
# Can heal OR use cover OR retreat
decomposition = "any_of"
```

**Parallel:** When independent
```toml
# Can reload and scan simultaneously
decomposition = "parallel"
```

---

## Troubleshooting

### Problem: AI Does Nothing

**Possible Causes:**
1. Goal conditions don't match actual game state variable names
2. Initial state doesn't satisfy any action preconditions
3. Goal is already satisfied

**Solutions:**
- Verify state variable names match game code
- Check that at least one action is executable from start state
- Add debug logging to see planned actions

### Problem: AI Keeps Switching Goals

**Possible Causes:**
1. Multiple goals with similar urgency
2. Goals with very close deadlines
3. Replan interval too short

**Solutions:**
- Differentiate priorities more (use full 1-10 range)
- Stagger deadlines
- Increase `replan_interval` in `GoalScheduler`

### Problem: Planner is Slow

**Possible Causes:**
1. Hierarchy too deep
2. Too many sub-goals at one level
3. Action space too large

**Solutions:**
- Reduce `max_depth` (try 3 or less)
- Split large goals into separate top-level goals
- Limit number of sub-goals per level (≤ 5 recommended)

### Problem: Goal Never Completes

**Possible Causes:**
1. Desired state is impossible to reach
2. Circular dependencies in sub-goals
3. State conditions too strict

**Solutions:**
- Verify goal is achievable manually
- Check for conflicting state requirements
- Use ranges instead of exact values

### Problem: Sub-Goals Executed in Wrong Order

**Possible Causes:**
1. Using `parallel` instead of `sequential`
2. Priority conflicts

**Solutions:**
- Change decomposition to `sequential`
- Adjust sub-goal priorities to match desired order

---

## Advanced Tips

### Dynamic Priority with Deadlines

Goals near their deadline become increasingly urgent:

```toml
# Low priority, but urgent deadline
priority = 3.0
deadline_seconds = 10.0
# Effective urgency at t=9s: ~33 (3.0 * 11)
# Will preempt higher-priority goals without deadlines!
```

**Formula:** `urgency = priority * (1 + 10 / (time_remaining + 1))`

### Combining Strategies

Use different strategies at different levels:

```toml
decomposition = "sequential"  # Top level: ordered

[[sub_goals]]
decomposition = "parallel"    # Sub-level: simultaneous

[[sub_goals]]
decomposition = "any_of"      # Sub-level: alternatives
```

### Goal Libraries

Create reusable goal collections:

```toml
# goal_library.toml
[[goals]]
name = "stay_alive"
# ...

[[goals]]
name = "engage_enemy"
# ...

[[goals]]
name = "support_team"
# ...
```

Load multiple goals at once and activate as needed.

### State Monitoring

Goals automatically remove themselves when:
- Satisfied (desired state achieved)
- Deadline passed
- Marked unachievable by planner

---

## Quick Reference

### File Template

```toml
name = "goal_name_here"
priority = 5.0
deadline_seconds = 60.0
decomposition = "sequential"
max_depth = 3

[desired_state]
variable_name = value
another_variable = { min = 0, max = 100 }

[[sub_goals]]
name = "sub_goal_name"
priority = 5.0

[sub_goals.desired_state]
sub_variable = true
```

### State Value Types

| Type | Example | Use Case |
|------|---------|----------|
| Bool | `in_cover = true` | Binary states |
| Int | `ammo = 30` | Exact counts |
| IntRange | `health = { min = 50, max = 100 }` | Flexible ranges |
| Float | `distance = 10.5` | Precise values |
| FloatApprox | `angle = { value = 90.0, tolerance = 5.0 }` | Approximate values |
| String | `stance = "crouched"` | Categorical states |

### Decomposition Quick Guide

| Strategy | Use When | Example |
|----------|----------|---------|
| `sequential` | Order matters | Scout → Prepare → Attack |
| `parallel` | Independent tasks | Reload + Scan + Coordinate |
| `any_of` | Alternatives | Smoke OR Suppress OR Eliminate |
| `all_of` | All required | Clear + Secure + Fortify |

---

## Getting Help

- **Examples:** See `examples/goal_templates/` for working examples
- **Validation:** Save your TOML and load in-game to get validation errors
- **Debugging:** Enable GOAP logging to see planning decisions
- **Documentation:** Refer to `advanced_goap_roadmap.md` for technical details

---

**Happy Goal Authoring!** 🎯

