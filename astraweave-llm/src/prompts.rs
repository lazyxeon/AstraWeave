//! Prompt Engineering for Game AI
//!
//! This module provides production-ready prompt templates and builders for
//! converting AstraWeave's `WorldSnapshot` into effective LLM prompts.
//!
//! # Design Principles
//! 1. **Concise** - Minimize tokens to fit in context window
//! 2. **Structured** - Use JSON for easy parsing
//! 3. **Actionable** - Guide LLM to generate valid ActionSteps
//! 4. **Deterministic** - Include constraints and rules
//!
//! # Usage
//! ```no_run
//! use astraweave_llm::prompts::PromptBuilder;
//! use astraweave_core::WorldSnapshot;
//!
//! # fn example(snapshot: WorldSnapshot) -> String {
//! let prompt = PromptBuilder::new()
//!     .system_role("tactical_ai")
//!     .add_snapshot(&snapshot)
//!     .add_goal("Eliminate enemies while staying in cover")
//!     .build();
//! # prompt
//! # }
//! ```

use astraweave_core::{ActionStep, WorldSnapshot};
use serde_json::json;

/// System prompt templates for different AI roles
pub mod templates {
    /// Tactical combat AI (aggressive, strategic)
    pub const TACTICAL_AI: &str = r#"You are a tactical AI agent in a real-time combat scenario.
Your goal is to eliminate threats while minimizing risk to yourself and allies.

Available Actions:
- MoveTo {x, y}: Move to grid position (x, y)
- Throw {item, x, y}: Throw item (smoke/grenade/flashbang) to position
- CoverFire {target_id, duration}: Suppress enemy with sustained fire
- Revive {ally_id}: Revive downed ally

Rules:
1. ALWAYS move to cover before engaging
2. Use smoke grenades to obscure enemy line of sight
3. Prioritize reviving allies if no immediate threat
4. Never engage multiple enemies without cover
5. Conserve grenades (max 3 uses per mission)

Output Format (JSON):
{
  "plan_id": "unique_id",
  "reasoning": "brief explanation",
  "steps": [
    {"act": "MoveTo", "x": 10, "y": 5},
    {"act": "CoverFire", "target_id": 99, "duration": 2.0}
  ]
}
"#;

    /// Stealth/infiltration AI (cautious, methodical)
    pub const STEALTH_AI: &str = r#"You are a stealth infiltration AI. Your objective is to reach the target without detection.

Available Actions:
- MoveTo {x, y}: Move to grid position (silent)
- Throw {item, x, y}: Distract guards with thrown items
- Wait {duration}: Pause for guard patrol timing

Rules:
1. NEVER use CoverFire (alerts all enemies)
2. Wait for guard patrols to pass before moving
3. Use thrown items to create distractions
4. Take cover if detection risk >30%
5. Prioritize indirect routes over direct paths

Output Format: Same JSON as tactical AI
"#;

    /// Support/medic AI (defensive, team-focused)
    pub const SUPPORT_AI: &str = r#"You are a support AI focused on keeping allies alive and providing tactical advantages.

Available Actions:
- MoveTo {x, y}: Reposition to support allies
- Revive {ally_id}: Heal downed ally
- Throw {item, x, y}: Deploy smoke for cover
- CoverFire {target_id, duration}: Suppress threats to allies

Rules:
1. ALWAYS prioritize ally survival over kills
2. Revive downed allies immediately if safe
3. Use smoke to create escape routes for allies
4. Stay behind front-line fighters
5. Suppressing fire only when allies are in danger

Output Format: Same JSON as tactical AI
"#;

    /// Exploration/reconnaissance AI (curious, thorough)
    pub const EXPLORATION_AI: &str = r#"You are an exploration AI tasked with mapping unknown territory and locating objectives.

Available Actions:
- MoveTo {x, y}: Navigate to unexplored areas
- Interact {object_id}: Examine points of interest
- Wait {duration}: Observe area for threats

Rules:
1. Visit all unexplored grid cells
2. Investigate points of interest (items, structures)
3. Avoid combat unless necessary for progress
4. Mark threats on map for tactical team
5. Return to start after full exploration

Output Format: Same JSON as tactical AI
"#;
}

/// Builder for constructing LLM prompts from game state
pub struct PromptBuilder {
    system_prompt: String,
    world_state: Option<String>,
    history: Vec<String>,
    goal: Option<String>,
    constraints: Vec<String>,
}

impl PromptBuilder {
    /// Create a new prompt builder with default tactical AI role
    pub fn new() -> Self {
        Self {
            system_prompt: templates::TACTICAL_AI.to_string(),
            world_state: None,
            history: Vec::new(),
            goal: None,
            constraints: Vec::new(),
        }
    }

    /// Set the AI role/system prompt
    pub fn system_role(mut self, role: &str) -> Self {
        self.system_prompt = match role {
            "tactical" | "tactical_ai" => templates::TACTICAL_AI,
            "stealth" | "stealth_ai" => templates::STEALTH_AI,
            "support" | "support_ai" | "medic" => templates::SUPPORT_AI,
            "exploration" | "scout" => templates::EXPLORATION_AI,
            custom => custom, // Allow custom prompts
        }
        .to_string();
        self
    }

    /// Add world snapshot as JSON
    pub fn add_snapshot(mut self, snapshot: &WorldSnapshot) -> Self {
        self.world_state = Some(Self::snapshot_to_json(snapshot));
        self
    }

    /// Add action history (previous steps)
    pub fn add_history(mut self, steps: &[ActionStep]) -> Self {
        for step in steps {
            self.history.push(Self::action_to_json(step));
        }
        self
    }

    /// Set the current goal/objective
    pub fn add_goal(mut self, goal: &str) -> Self {
        self.goal = Some(goal.to_string());
        self
    }

    /// Add a constraint/rule
    pub fn add_constraint(mut self, constraint: &str) -> Self {
        self.constraints.push(constraint.to_string());
        self
    }

    /// Build final prompt string
    pub fn build(self) -> String {
        let mut parts = vec![self.system_prompt];

        if let Some(goal) = self.goal {
            parts.push(format!("\nCurrent Objective: {}", goal));
        }

        if !self.constraints.is_empty() {
            parts.push("\nAdditional Constraints:".to_string());
            for (i, constraint) in self.constraints.iter().enumerate() {
                parts.push(format!("{}. {}", i + 1, constraint));
            }
        }

        if let Some(world) = self.world_state {
            parts.push(format!("\n--- WORLD STATE ---\n{}", world));
        }

        if !self.history.is_empty() {
            parts.push("\n--- ACTION HISTORY ---".to_string());
            for (i, action) in self.history.iter().enumerate() {
                parts.push(format!("Step {}: {}", i + 1, action));
            }
        }

        parts.push("\n--- YOUR PLAN ---".to_string());
        parts.push("Generate a JSON plan following the format above:".to_string());

        parts.join("\n")
    }

    /// Convert WorldSnapshot to concise JSON representation
    fn snapshot_to_json(snapshot: &WorldSnapshot) -> String {
        json!({
            "player": {
                "position": snapshot.player.pos,
                "health": snapshot.player.hp,
                "stance": snapshot.player.stance,
            },
            "me": {
                "position": snapshot.me.pos,
                "morale": snapshot.me.morale,
                "cooldowns": snapshot.me.cooldowns,
                "ammo": snapshot.me.ammo,
            },
            "enemies": snapshot.enemies.iter().map(|e| json!({
                "id": e.id,
                "position": e.pos,
                "health": e.hp,
                "cover": e.cover,
                "last_seen": e.last_seen,
            })).collect::<Vec<_>>(),
            "points_of_interest": snapshot.pois.iter().map(|poi| json!({
                "kind": poi.k,
                "position": poi.pos,
            })).collect::<Vec<_>>(),
            "obstacles": snapshot.obstacles.iter().map(|obs| json!({
                "position": {"x": obs.x, "y": obs.y},
            })).collect::<Vec<_>>(),
        })
        .to_string()
    }

    /// Convert ActionStep to JSON string
    fn action_to_json(step: &ActionStep) -> String {
        match step {
            ActionStep::MoveTo { x, y, speed: _ } => {
                json!({"act": "MoveTo", "x": x, "y": y}).to_string()
            }
            ActionStep::Throw { item, x, y } => {
                json!({"act": "Throw", "item": item, "x": x, "y": y}).to_string()
            }
            ActionStep::CoverFire {
                target_id,
                duration,
            } => json!({"act": "CoverFire", "target_id": target_id, "duration": duration})
                .to_string(),
            ActionStep::Revive { ally_id } => {
                json!({"act": "Revive", "ally_id": ally_id}).to_string()
            }
            // Phase 7: Handle all new tools with generic JSON serialization
            _ => {
                json!({"act": "Unknown", "note": "Phase 7 tool - implement specific serialization"})
                    .to_string()
            }
        }
    }
}

impl Default for PromptBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Quick prompt generation functions
pub mod quick {
    use super::*;

    /// Generate tactical combat prompt from snapshot
    pub fn tactical_prompt(snapshot: &WorldSnapshot, goal: &str) -> String {
        PromptBuilder::new()
            .system_role("tactical")
            .add_snapshot(snapshot)
            .add_goal(goal)
            .build()
    }

    /// Generate stealth infiltration prompt
    pub fn stealth_prompt(snapshot: &WorldSnapshot, target: &str) -> String {
        PromptBuilder::new()
            .system_role("stealth")
            .add_snapshot(snapshot)
            .add_goal(&format!("Reach {} without detection", target))
            .build()
    }

    /// Generate support/medic prompt
    pub fn support_prompt(snapshot: &WorldSnapshot, ally_id: u64) -> String {
        PromptBuilder::new()
            .system_role("support")
            .add_snapshot(snapshot)
            .add_goal(&format!("Revive ally {} and provide cover", ally_id))
            .build()
    }

    /// Generate exploration prompt
    pub fn exploration_prompt(snapshot: &WorldSnapshot) -> String {
        PromptBuilder::new()
            .system_role("exploration")
            .add_snapshot(snapshot)
            .add_goal("Map the area and locate objectives")
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use astraweave_core::{CompanionState, CoverType, EnemyState, IVec2, PlayerState, Stance};
    use std::collections::BTreeMap;

    fn create_test_snapshot() -> WorldSnapshot {
        WorldSnapshot {
            t: 0.0,
            player: PlayerState {
                hp: 100,
                pos: IVec2 { x: 5, y: 5 },
                stance: Stance::Stand,
                orders: vec![],
            },
            me: CompanionState {
                pos: IVec2 { x: 5, y: 5 },
                ammo: 10,
                morale: 1.0,
                cooldowns: BTreeMap::new(),
            },
            enemies: vec![EnemyState {
                id: 99,
                pos: IVec2 { x: 10, y: 8 },
                hp: 100,
                cover: CoverType::Wall,
                last_seen: 0.0,
            }],
            pois: vec![],
            obstacles: vec![],
            objective: Some("Eliminate enemy 99".to_string()),
        }
    }

    #[test]
    fn test_prompt_builder_basic() {
        let snapshot = create_test_snapshot();
        let prompt = PromptBuilder::new()
            .add_snapshot(&snapshot)
            .add_goal("Eliminate enemy 99")
            .build();

        assert!(prompt.contains("WORLD STATE"));
        assert!(prompt.contains("YOUR PLAN"));
        assert!(prompt.contains("Eliminate enemy 99"));
    }

    #[test]
    fn test_prompt_builder_roles() {
        let snapshot = create_test_snapshot();

        let tactical = PromptBuilder::new()
            .system_role("tactical")
            .add_snapshot(&snapshot)
            .build();
        assert!(tactical.contains("tactical AI"));

        let stealth = PromptBuilder::new()
            .system_role("stealth")
            .add_snapshot(&snapshot)
            .build();
        assert!(stealth.contains("stealth infiltration"));
    }

    #[test]
    fn test_quick_prompts() {
        let snapshot = create_test_snapshot();

        let tactical = quick::tactical_prompt(&snapshot, "Test goal");
        assert!(tactical.contains("tactical AI"));
        assert!(tactical.contains("Test goal"));

        let stealth = quick::stealth_prompt(&snapshot, "waypoint_5");
        assert!(stealth.contains("Reach waypoint_5"));
    }

    #[test]
    fn test_snapshot_json_format() {
        let snapshot = create_test_snapshot();
        let json_str = PromptBuilder::snapshot_to_json(&snapshot);

        // Verify JSON is valid
        let parsed: serde_json::Value =
            serde_json::from_str(&json_str).expect("Generated invalid JSON");

        // Check structure (position is IVec2 object with x,y)
        assert!(parsed["player"]["position"].is_object());
        assert_eq!(parsed["player"]["position"]["x"], 5);
        assert_eq!(parsed["player"]["position"]["y"], 5);
        assert!(parsed["me"]["morale"].is_number());
        assert!(parsed["enemies"].is_array());
        assert_eq!(parsed["enemies"][0]["id"], 99);
    }

    #[test]
    fn test_action_history() {
        let snapshot = create_test_snapshot();
        let history = vec![
            ActionStep::MoveTo {
                x: 4,
                y: 4,
                speed: None,
            },
            ActionStep::CoverFire {
                target_id: 99,
                duration: 2.0,
            },
        ];

        let prompt = PromptBuilder::new()
            .add_snapshot(&snapshot)
            .add_history(&history)
            .build();

        assert!(prompt.contains("ACTION HISTORY"));
        assert!(prompt.contains("MoveTo"));
        assert!(prompt.contains("CoverFire"));
    }

    #[test]
    fn test_constraints() {
        let snapshot = create_test_snapshot();
        let prompt = PromptBuilder::new()
            .add_snapshot(&snapshot)
            .add_constraint("Do not use grenades")
            .add_constraint("Stay within 10 units of player")
            .build();

        assert!(prompt.contains("Additional Constraints"));
        assert!(prompt.contains("Do not use grenades"));
        assert!(prompt.contains("Stay within 10 units"));
    }

    #[test]
    fn test_prompt_builder_custom_role() {
        let prompt = PromptBuilder::new()
            .system_role("You are a helpful assistant")
            .build();
        assert!(prompt.contains("You are a helpful assistant"));
    }

    #[test]
    fn test_prompt_builder_all_roles() {
        let roles = ["tactical", "stealth", "support", "exploration"];
        for role in roles {
            let prompt = PromptBuilder::new().system_role(role).build();
            assert!(!prompt.is_empty());
        }
    }

    #[test]
    fn test_action_to_json_all_variants() {
        let steps = vec![
            ActionStep::MoveTo {
                x: 1,
                y: 2,
                speed: None,
            },
            ActionStep::Throw {
                item: "smoke".into(),
                x: 3,
                y: 4,
            },
            ActionStep::CoverFire {
                target_id: 1,
                duration: 5.0,
            },
            ActionStep::Revive { ally_id: 2 },
            ActionStep::Wait { duration: 1.0 }, // Should hit the _ branch
        ];

        for step in steps {
            let json = PromptBuilder::action_to_json(&step);
            assert!(json.contains("act"));
        }
    }

    #[test]
    fn test_quick_prompts_all() {
        let snap = create_test_snapshot();

        assert!(quick::support_prompt(&snap, 1).contains("Revive ally 1"));
        assert!(quick::exploration_prompt(&snap).contains("Map the area"));
    }

    #[test]
    fn test_snapshot_json_with_pois_and_obstacles() {
        use astraweave_core::{IVec2, Poi};
        let mut snap = create_test_snapshot();
        snap.pois.push(Poi {
            k: "ammo".into(),
            pos: IVec2 { x: 1, y: 1 },
        });
        snap.obstacles.push(IVec2 { x: 2, y: 2 });

        let json_str = PromptBuilder::snapshot_to_json(&snap);
        assert!(json_str.contains("points_of_interest"));
        assert!(json_str.contains("obstacles"));
        assert!(json_str.contains("ammo"));
    }
}
