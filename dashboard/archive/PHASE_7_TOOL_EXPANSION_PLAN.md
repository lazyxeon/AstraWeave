# Phase 7: LLM Prompt Engineering & Tool Vocabulary Expansion

**Status**: 📋 PLANNED (Reference Document)  
**Prerequisites**: Phase 6 Complete ✅  
**Estimated Effort**: 4-6 hours  
**Expected Improvement**: LLM plan success rate 0% → 85%+

---

## Mission

Fix Phi-3 prompt engineering so it generates valid, creative plans using an expanded tool vocabulary. The LLM currently hallucinates tools (e.g., "MoveTo") and fails JSON parsing - we need robust prompts that guide the model to success.

**Current Problem**:
- ✅ Phi-3 connects successfully via Ollama
- ❌ Returns 0-step plans (parse failures)
- ❌ Hallucinates disallowed tools
- ❌ Returns non-JSON text
- ❌ Only 3 tools available (too restrictive)

**Phase 7 Solution**:
- ✅ Expand to 37 tools across 6 categories
- ✅ Robust prompt templates with JSON schema enforcement
- ✅ Few-shot learning (5+ example scenarios)
- ✅ Multi-tier fallback system
- ✅ Prompt caching (50× speedup)
- ✅ JSON validation to catch hallucinations

---

## Phase 1: Expand Tool Vocabulary

### 1.1 Current Tool Limitations

**Current tools (only 3)**:
```rust
enum ToolAction {
    ThrowSmoke,
    CoverFire,
    Attack,
}
```

**Problem**: Too restrictive for creative AI planning. LLM hallucinates "MoveTo" because it needs movement tools.

---

### 1.2 Expanded Tool Vocabulary (37 Tools)

**Design comprehensive tool set organized by category**:

```rust
// In astraweave-core/src/tool_action.rs or new crate

/// Complete tool vocabulary for LLM planning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolAction {
    // ═══════════════════════════════════════
    // MOVEMENT (6 tools)
    // ═══════════════════════════════════════
    
    /// Move to a specific position
    MoveTo { 
        x: i32, 
        y: i32,
        /// Speed: Walk, Run, Sprint
        speed: MovementSpeed,
    },
    
    /// Move toward target entity while maintaining distance
    Approach { 
        target_id: u64,
        /// Desired distance (e.g., melee=2, ranged=15)
        distance: f32,
    },
    
    /// Move away from target entity
    Retreat { 
        target_id: u64,
        /// Safe distance to reach
        distance: f32,
    },
    
    /// Take cover behind nearest obstacle
    TakeCover {
        /// Optional: specific cover position
        position: Option<(i32, i32)>,
    },
    
    /// Strafe around target (circle)
    Strafe {
        target_id: u64,
        /// Left or Right
        direction: StrafeDirection,
    },
    
    /// Patrol between waypoints
    Patrol {
        waypoints: Vec<(i32, i32)>,
    },
    
    // ═══════════════════════════════════════
    // OFFENSIVE ACTIONS (8 tools)
    // ═══════════════════════════════════════
    
    /// Direct attack on target
    Attack {
        target_id: u64,
        /// Optional: specify weapon/attack type
        attack_type: Option<AttackType>,
    },
    
    /// Aimed shot (requires time, more accurate)
    AimedShot {
        target_id: u64,
    },
    
    /// Quick attack (less damage, faster)
    QuickAttack {
        target_id: u64,
    },
    
    /// Heavy attack (more damage, slower, consumes stamina)
    HeavyAttack {
        target_id: u64,
    },
    
    /// Area-of-effect attack
    AoEAttack {
        center: (i32, i32),
        radius: f32,
    },
    
    /// Throw grenade/explosive
    ThrowExplosive {
        target: (i32, i32),
        /// Type: Frag, Flashbang, Smoke
        explosive_type: ExplosiveType,
    },
    
    /// Suppressive fire (reduces enemy accuracy)
    CoverFire {
        target_id: u64,
        /// Duration in seconds
        duration: f32,
    },
    
    /// Charge attack (rush + melee)
    Charge {
        target_id: u64,
    },
    
    // ═══════════════════════════════════════
    // DEFENSIVE ACTIONS (6 tools)
    // ═══════════════════════════════════════
    
    /// Raise shield/block
    Block {
        /// Optional: direction to block from
        direction: Option<Direction>,
    },
    
    /// Dodge/roll (consumes stamina)
    Dodge {
        direction: Direction,
    },
    
    /// Parry incoming attack
    Parry {
        /// Anticipated attacker
        attacker_id: u64,
    },
    
    /// Deploy smoke screen
    ThrowSmoke {
        position: (i32, i32),
    },
    
    /// Heal self or ally
    Heal {
        target_id: u64,
        /// Healing item/ability
        heal_type: HealType,
    },
    
    /// Use defensive ability (shield, armor buff)
    UseDefensiveAbility {
        ability: String,
    },
    
    // ═══════════════════════════════════════
    // EQUIPMENT & INVENTORY (5 tools)
    // ═══════════════════════════════════════
    
    /// Equip weapon from inventory
    EquipWeapon {
        weapon_id: String,
    },
    
    /// Switch to different weapon
    SwitchWeapon {
        weapon_slot: u32, // 1=primary, 2=secondary, 3=melee
    },
    
    /// Reload current weapon
    Reload,
    
    /// Use consumable item
    UseItem {
        item_id: String,
    },
    
    /// Drop item or weapon
    DropItem {
        item_id: String,
    },
    
    // ═══════════════════════════════════════
    // TACTICAL & COORDINATION (7 tools)
    // ═══════════════════════════════════════
    
    /// Call for reinforcements
    CallReinforcements {
        position: (i32, i32),
    },
    
    /// Mark target for allies
    MarkTarget {
        target_id: u64,
        /// Priority: High, Medium, Low
        priority: Priority,
    },
    
    /// Request covering fire from ally
    RequestCover {
        ally_id: u64,
        target_id: u64,
    },
    
    /// Coordinate pincer attack
    CoordinateAttack {
        allies: Vec<u64>,
        target_id: u64,
        strategy: AttackStrategy,
    },
    
    /// Set up ambush position
    SetAmbush {
        position: (i32, i32),
    },
    
    /// Distract enemy (noise, decoy)
    Distract {
        target_id: u64,
        distraction_type: DistractionType,
    },
    
    /// Retreat and regroup
    Regroup {
        rally_point: (i32, i32),
    },
    
    // ═══════════════════════════════════════
    // UTILITY & SPECIAL (5 tools)
    // ═══════════════════════════════════════
    
    /// Scan/survey area
    Scan {
        center: (i32, i32),
        radius: f32,
    },
    
    /// Wait/observe (defensive stance)
    Wait {
        duration: f32,
    },
    
    /// Interact with object (door, lever, terminal)
    Interact {
        object_id: u64,
    },
    
    /// Use special ability
    UseAbility {
        ability_name: String,
        target: Option<u64>,
    },
    
    /// Taunt enemy (draw aggro)
    Taunt {
        target_id: u64,
    },
}

// Total: 37 tools across 6 categories
```

---

### 1.3 Tool Metadata for LLM

**Create tool descriptions that LLM can understand**:

```rust
// In astraweave-llm/src/tool_vocabulary.rs

use serde::{Serialize, Deserialize};

/// Tool description for LLM prompt
#[derive(Debug, Clone, Serialize)]
pub struct ToolDefinition {
    pub name: String,
    pub category: String,
    pub description: String,
    pub parameters: Vec<ParameterDef>,
    pub cooldown_seconds: f32,
    pub resource_cost: ResourceCost,
    pub preconditions: Vec<String>,
    pub examples: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ParameterDef {
    pub name: String,
    pub type_: String, // "u64", "f32", "string", etc.
    pub description: String,
    pub required: bool,
    pub default: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResourceCost {
    pub stamina: f32,
    pub mana: f32,
    pub ammo: u32,
}

/// Generate complete tool vocabulary for LLM
pub fn get_tool_vocabulary() -> Vec<ToolDefinition> {
    vec![
        // MOVEMENT
        ToolDefinition {
            name: "MoveTo".to_string(),
            category: "Movement".to_string(),
            description: "Move to a specific position on the map".to_string(),
            parameters: vec![
                ParameterDef {
                    name: "x".to_string(),
                    type_: "i32".to_string(),
                    description: "X coordinate".to_string(),
                    required: true,
                    default: None,
                },
                ParameterDef {
                    name: "y".to_string(),
                    type_: "i32".to_string(),
                    description: "Y coordinate".to_string(),
                    required: true,
                    default: None,
                },
                ParameterDef {
                    name: "speed".to_string(),
                    type_: "string".to_string(),
                    description: "Movement speed: Walk, Run, or Sprint".to_string(),
                    required: false,
                    default: Some("Run".to_string()),
                },
            ],
            cooldown_seconds: 0.0,
            resource_cost: ResourceCost { stamina: 0.0, mana: 0.0, ammo: 0 },
            preconditions: vec!["Path must be walkable".to_string()],
            examples: vec![
                r#"{"tool": "MoveTo", "x": 10, "y": 5, "speed": "Run"}"#.to_string(),
            ],
        },
        
        // Add remaining 36 tools with similar detail...
        // (See full implementation in detailed plan)
    ]
}
```

---

## Phase 2: Robust Prompt Engineering

### 2.1 Production-Grade Prompt Template

**Create structured prompt with clear constraints**:

```rust
// In astraweave-llm/src/prompt_template.rs

pub struct PromptBuilder {
    handlebars: Handlebars<'static>,
}

impl PromptBuilder {
    pub fn build_prompt(
        &self,
        context: &PerceptionSnapshot,
        goal: &Goal,
        agent_profile: &AgentProfile,
    ) -> Result<String> {
        let tools = get_tool_vocabulary();
        
        // Build tool documentation
        let tool_docs = tools.iter()
            .map(|t| format_tool_doc(t))
            .collect::<Vec<_>>()
            .join("\n\n");
        
        // Render prompt with all context
        // ... (see full implementation)
    }
}

// Prompt template with strict JSON schema
const TACTICAL_PLANNING_TEMPLATE: &str = r#"
You are {{agent_name}}, a {{agent_role}} in a tactical combat scenario.

═══════════════════════════════════════════════════════════════════
CURRENT SITUATION
═══════════════════════════════════════════════════════════════════

Your Status:
- Health: {{agent_health_percent}}%
- Position: {{agent_position}}
- Stamina: {{agent_stamina}}

Environment:
{{nearby_enemies}}
{{nearby_allies}}

Mission Objective: {{goal}}

═══════════════════════════════════════════════════════════════════
AVAILABLE TOOLS ({{tool_count}} total)
═══════════════════════════════════════════════════════════════════

You MUST use ONLY these tools. Using any tool not listed here will result in plan rejection.

{{tools}}

═══════════════════════════════════════════════════════════════════
OUTPUT FORMAT (STRICT JSON)
═══════════════════════════════════════════════════════════════════

You MUST return ONLY this JSON structure, with no additional text:

{
  "reasoning": "Brief tactical analysis (1-2 sentences)",
  "plan": [
    {
      "step": 1,
      "tool": "ToolName",
      "parameters": {"param1": value1},
      "reason": "Why this step"
    }
  ],
  "expected_outcome": "What you expect to achieve"
}

BEGIN YOUR RESPONSE (JSON only, starting with '{'):
"#;
```

---

## Phase 3: JSON Schema Enforcement

### 3.1 Strict Validation

**Add schema validation to catch hallucinations**:

```rust
// In astraweave-llm/src/plan_parser.rs

use jsonschema::{Draft, JSONSchema};

const PLAN_SCHEMA: &str = r#"{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "required": ["reasoning", "plan", "expected_outcome"],
  "properties": {
    "reasoning": {"type": "string", "minLength": 10},
    "plan": {
      "type": "array",
      "minItems": 1,
      "maxItems": 5,
      "items": {
        "type": "object",
        "required": ["step", "tool", "parameters", "reason"],
        "properties": {
          "tool": {
            "type": "string",
            "enum": ["MoveTo", "Approach", "Attack", /* ... all 37 tools */]
          }
        }
      }
    }
  }
}"#;

pub struct PlanParser {
    schema: JSONSchema,
}

impl PlanParser {
    pub fn parse(&self, llm_response: &str) -> Result<Plan> {
        // 1. Extract JSON from response
        let json_text = self.extract_json(llm_response)?;
        
        // 2. Parse as JSON
        let json: Value = serde_json::from_str(&json_text)?;
        
        // 3. Validate against schema
        if let Err(errors) = self.schema.validate(&json) {
            return Err(anyhow!("Schema validation failed: {:?}", errors));
        }
        
        // 4. Validate tool names
        for step in json["plan"].as_array() {
            let tool_name = step["tool"].as_str().unwrap();
            if !self.is_valid_tool(tool_name) {
                return Err(anyhow!("Invalid tool: {}", tool_name));
            }
        }
        
        // 5. Convert to Plan struct
        Ok(self.json_to_plan(&json)?)
    }
}
```

---

## Phase 4: Few-Shot Learning Examples

### 4.1 Add Example Plans to Prompt

**Teach LLM by showing good examples**:

```rust
const FEW_SHOT_EXAMPLES: &str = r#"
═══════════════════════════════════════════════════════════════════
EXAMPLE SCENARIOS (Learn from these)
═══════════════════════════════════════════════════════════════════

SCENARIO 1: Enemy at Long Range, Open Terrain
{
  "reasoning": "Enemy too far for melee, need cover to close distance",
  "plan": [
    {"step": 1, "tool": "ThrowSmoke", "parameters": {"x": 15, "y": 10}, "reason": "Block line of sight"},
    {"step": 2, "tool": "Approach", "parameters": {"target_id": 42, "distance": 10.0}, "reason": "Close gap"},
    {"step": 3, "tool": "AimedShot", "parameters": {"target_id": 42}, "reason": "Strike from medium range"}
  ],
  "expected_outcome": "Engage safely at medium range"
}

SCENARIO 2: Low Health, Multiple Enemies
{
  "reasoning": "Critically wounded, cannot win direct engagement",
  "plan": [
    {"step": 1, "tool": "ThrowSmoke", "parameters": {"x": 5, "y": 5}, "reason": "Cover retreat"},
    {"step": 2, "tool": "Retreat", "parameters": {"target_id": 42, "distance": 30.0}, "reason": "Escape"},
    {"step": 3, "tool": "Heal", "parameters": {"target_id": 0}, "reason": "Restore health"}
  ],
  "expected_outcome": "Survive by tactical withdrawal"
}

/* ... 3 more scenarios ... */

NOW IT'S YOUR TURN - Generate a similar plan for the current situation.
"#;
```

---

## Phase 5: Multi-Tier Fallback System

### 5.1 Graceful Degradation

**4-tier fallback when LLM fails**:

```rust
// In astraweave-llm/src/plan_from_llm.rs

pub fn plan_from_llm_with_fallback(
    agent: Entity,
    snapshot: &PerceptionSnapshot,
    goal: Goal,
) -> Result<Plan> {
    // Tier 1: Try full LLM planning (37 tools)
    match try_llm_planning(agent, snapshot, goal.clone()) {
        Ok(plan) => {
            log::info!("✅ LLM planning succeeded");
            return Ok(plan);
        }
        Err(e) => log::warn!("LLM failed: {}. Trying simplified...", e),
    }
    
    // Tier 2: Try simplified LLM prompt (5 core tools only)
    match try_simplified_llm(agent, snapshot, goal.clone()) {
        Ok(plan) => {
            log::info!("✅ Simplified LLM succeeded");
            return Ok(plan);
        }
        Err(e) => log::warn!("Simplified failed: {}. Trying heuristic...", e),
    }
    
    // Tier 3: Heuristic fallback (deterministic logic)
    match generate_heuristic_plan(agent, snapshot, goal.clone()) {
        Ok(plan) => {
            log::info!("✅ Heuristic fallback succeeded");
            return Ok(plan);
        }
        Err(e) => log::error!("Heuristic failed: {}. Emergency plan.", e),
    }
    
    // Tier 4: Emergency safe plan (always succeeds)
    Ok(generate_emergency_plan(agent, snapshot))
}
```

---

## Phase 6: Prompt Caching

### 6.1 Cache Strategy

**50× speedup for similar situations**:

```rust
// In astraweave-llm/src/prompt_cache.rs

pub struct PromptCache {
    /// Exact match cache (instant lookup)
    cache: Arc<RwLock<HashMap<u64, CachedResponse>>>,
    /// Semantic similarity cache (near matches)
    semantic_cache: Option<SemanticCache>,
    stats: Arc<RwLock<CacheStats>>,
}

impl PromptCache {
    pub fn get(&self, prompt: &str) -> Option<Plan> {
        // Try exact match first (fastest)
        let hash = hash_prompt(prompt);
        if let Some(cached) = self.cache.read().unwrap().get(&hash) {
            self.stats.write().unwrap().hits += 1;
            return Some(cached.plan.clone());
        }
        
        // Try semantic similarity (slower, but catches near-matches)
        if let Some(semantic) = &self.semantic_cache {
            if let Some(plan) = semantic.find_similar(prompt, 0.85) {
                self.stats.write().unwrap().semantic_hits += 1;
                return Some(plan);
            }
        }
        
        None
    }
    
    pub fn insert(&mut self, prompt: String, plan: Plan) {
        // Store in both caches
        // Implement LRU eviction if cache > 10k entries
    }
}
```

---

## Phase 7: Testing & Validation

### 7.1 Comprehensive Test Suite

```rust
// In astraweave-llm/tests/prompt_tests.rs

#[test]
fn test_prompt_contains_all_tools() {
    let prompt = PromptBuilder::new().build_prompt(...);
    
    // Verify all 37 tools documented
    assert!(prompt.contains("MoveTo"));
    assert!(prompt.contains("Approach"));
    // ... verify all 37
}

#[test]
fn test_parse_valid_llm_response() {
    let response = r#"{
      "reasoning": "Enemy at medium range, need to close safely",
      "plan": [
        {"step": 1, "tool": "ThrowSmoke", "parameters": {"x": 10, "y": 5}, "reason": "Block LOS"},
        {"step": 2, "tool": "Approach", "parameters": {"target_id": 42, "distance": 10.0}, "reason": "Close gap"}
      ],
      "expected_outcome": "Reach and engage safely"
    }"#;
    
    let parser = PlanParser::new().unwrap();
    let plan = parser.parse(response).unwrap();
    
    assert_eq!(plan.actions.len(), 2);
    println!("✅ Valid response parsed");
}

#[test]
fn test_reject_hallucinated_tool() {
    let response = r#"{
      "plan": [{"step": 1, "tool": "FlyToMoon", "parameters": {}}]
    }"#;
    
    let parser = PlanParser::new().unwrap();
    let result = parser.parse(response);
    
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid tool"));
}

#[test]
fn test_fallback_chain() {
    // Mock LLM failure
    let result = plan_from_llm_with_fallback(...);
    
    // Should succeed via fallback
    assert!(result.is_ok());
}
```

---

## Phase 8: Integration & Documentation

### 8.1 Update hello_companion Demo

```rust
// In examples/hello_companion/main.rs

fn demo_tool_vocabulary() {
    println!("\n╔════════════════════════════════════════╗");
    println!("║  Available AI Tools (37 total)        ║");
    println!("╚════════════════════════════════════════╝\n");
    
    let tools = get_tool_vocabulary();
    
    // Group by category
    for (category, tools) in categories {
        println!("📦 {}:", category);
        for tool in tools {
            println!("   • {} - {}", tool.name, tool.description);
        }
    }
}
```

---

## Expected Results

### Before Phase 7 (Current State - Phase 6)

```bash
Running: LLM (Phi-3 via Ollama)
   ✅ Ollama + phi3 confirmed
   ⚠️ Phi-3 returned fallback: Parse failed
   ✅ Generated 0 step plan in 3462ms  ← FAILURE
```

**Metrics**:
- Valid Plans: 0%
- Tool Hallucinations: 100%
- JSON Parse Errors: 100%
- Available Tools: 3
- Plan Creativity: Low

---

### After Phase 7 (Target State)

```bash
Running: LLM (Phi-3 via Ollama)
   Tool vocabulary: 37 tools available
   ✅ Ollama + phi3 confirmed
   
   LLM reasoning: "Enemy at medium range, need cover to close safely"
   
   Plan generated:
   1. ThrowSmoke(x:8, y:4) - Block line of sight
   2. Approach(target:42, distance:10.0) - Close gap under cover
   3. Attack(target:42) - Engage from medium range
   
   ✅ Generated 3 step plan in 2847ms  ← SUCCESS
   📊 Cache hit rate: 78%
```

**Metrics**:
- Valid Plans: 85%+ (from 0%)
- Tool Hallucinations: <5% (from 100%)
- JSON Parse Success: 90%+ (from 0%)
- Available Tools: 37 (from 3)
- Plan Creativity: High
- Cache Hit Rate: 70%+

---

## Performance Comparison Table

```
╔════════════════════════════════════════════════════════════╗
║                LLM IMPROVEMENT METRICS                     ║
╠════════════════════════════╦═══════════╦═══════════╦═══════╣
║ Metric                     ║   Before  ║   After   ║ Delta ║
╠════════════════════════════╬═══════════╬═══════════╬═══════╣
║ Valid Plans Generated      ║      0%   ║     85%   ║ +85%  ║
║ Tool Hallucinations        ║    100%   ║      5%   ║ -95%  ║
║ JSON Parse Success         ║      0%   ║     90%   ║ +90%  ║
║ Available Tools            ║      3    ║     37    ║ +34   ║
║ Plan Steps (avg)           ║      0    ║    2.8    ║ +2.8  ║
║ Cache Hit Rate             ║     N/A   ║     78%   ║  NEW  ║
║ Fallback Trigger Rate      ║    100%   ║     15%   ║ -85%  ║
╚════════════════════════════╩═══════════╩═══════════╩═══════╝
```

---

## Implementation Timeline

**Total effort: 4-6 hours**

| Phase | Task | Time | Priority |
|-------|------|------|----------|
| 1 | Expand tool vocabulary (37 tools) | 1h | 🔴 Critical |
| 2 | Create prompt templates | 1h | 🔴 Critical |
| 3 | Implement JSON parsing/validation | 1h | 🔴 Critical |
| 4 | Build fallback system | 30m | 🟡 High |
| 5 | Add prompt caching | 30m | 🟢 Medium |
| 6 | Write comprehensive tests | 1h | 🔴 Critical |
| 7 | Update hello_companion demo | 30m | 🟡 High |
| 8 | Documentation | 30m | 🟢 Medium |

---

## Deliverables Checklist

### Code Files to Create/Modify

```
astraweave-core/src/tool_action.rs
  ├─ Expand ToolAction enum (37 tools)
  └─ Add parameter structs (MovementSpeed, Priority, etc.)

astraweave-llm/src/tool_vocabulary.rs  [NEW]
  ├─ ToolDefinition struct
  ├─ get_tool_vocabulary() function
  └─ Tool metadata (descriptions, parameters, examples)

astraweave-llm/src/prompt_template.rs  [NEW]
  ├─ PromptBuilder struct
  ├─ TACTICAL_PLANNING_TEMPLATE
  ├─ FEW_SHOT_EXAMPLES
  └─ format_tool_doc() helper

astraweave-llm/src/plan_parser.rs  [NEW]
  ├─ PlanParser struct
  ├─ PLAN_SCHEMA (JSON schema)
  ├─ parse() with validation
  └─ json_to_tool_action() converter

astraweave-llm/src/plan_from_llm.rs  [MODIFY]
  ├─ plan_from_llm_with_fallback()
  ├─ try_simplified_llm()
  ├─ generate_heuristic_plan()
  └─ generate_emergency_plan()

astraweave-llm/src/prompt_cache.rs  [NEW]
  ├─ PromptCache struct
  ├─ Exact match cache
  ├─ Semantic cache (optional)
  └─ LRU eviction

astraweave-llm/tests/prompt_tests.rs  [NEW]
  ├─ test_prompt_contains_all_tools()
  ├─ test_parse_valid_llm_response()
  ├─ test_reject_hallucinated_tool()
  └─ test_fallback_chain()

examples/hello_companion/src/main.rs  [MODIFY]
  └─ demo_tool_vocabulary() showcase
```

---

## Success Criteria

**Phase 7 is COMPLETE when**:

1. ✅ **Compilation Success**
   - All new files compile without errors
   - hello_companion builds with expanded tools

2. ✅ **Functional Tests Pass**
   - 15+ unit tests covering prompt/parse/validate
   - 5+ integration tests covering full pipeline
   - All tests passing

3. ✅ **LLM Demo Works**
   - hello_companion LLM mode generates valid plans
   - 3+ step plans with tactical reasoning
   - Tools from expanded vocabulary (37 options)
   - <10% hallucination rate
   - >70% cache hit rate after warmup

4. ✅ **Metrics Improvement**
   - Valid plans: 0% → 85%+
   - Tool hallucinations: 100% → <5%
   - JSON parse success: 0% → 90%+
   - Available tools: 3 → 37

5. ✅ **Documentation Complete**
   - Tool vocabulary fully documented
   - Prompt engineering guide written
   - Usage examples provided

---

## Quick Start Instructions (For Implementation)

```bash
# Step 1: Expand tool vocabulary
# Edit: astraweave-core/src/tool_action.rs
# Add 34 new ToolAction variants

# Step 2: Create tool definitions
# Create: astraweave-llm/src/tool_vocabulary.rs
# Implement get_tool_vocabulary() with metadata

# Step 3: Build prompt template
# Create: astraweave-llm/src/prompt_template.rs
# Implement PromptBuilder with comprehensive template

# Step 4: Implement parser
# Create: astraweave-llm/src/plan_parser.rs
# Add JSON schema validation

# Step 5: Test incrementally
cargo test -p astraweave-llm --lib -- --nocapture

# Step 6: Run demo
cargo run -p hello_companion --release --features llm,ollama -- --demo-all
```

---

## Critical Reminders

**While implementing**:

1. ✅ **Don't break existing code** - hello_companion must still work without LLM
2. ✅ **Test incrementally** - Validate each phase before moving to next
3. ✅ **Use real Phi-3 output** - Don't assume LLM behavior, test with actual model
4. ✅ **Document everything** - Future developers need to understand the system
5. ✅ **Measure improvements** - Track metrics before/after to prove success

---

## Dependencies to Add

```toml
# In astraweave-llm/Cargo.toml
[dependencies]
handlebars = "4.3"           # Prompt templating
jsonschema = "0.17"          # JSON schema validation
serde_json = "1.0"           # JSON parsing
anyhow = "1.0"               # Error handling
```

---

## Final Notes

**This prompt engineering system will**:

✅ Transform LLM from 0% success to 85%+ success  
✅ Expand tool vocabulary from 3 to 37 options  
✅ Enable creative tactical planning  
✅ Provide robust fallback system  
✅ Achieve 70%+ cache hit rate  
✅ Make AstraWeave's AI-native claims real  

**This is the missing piece that makes LLM integration production-ready!** 🚀

---

**Status**: 📋 Reference document - ready for implementation when Phase 7 begins  
**Next Action**: Review Phase 6 completion summary, then decide when to start Phase 7

---

*This plan was extracted from user input and saved as reference documentation for future implementation.*
