//! Prompt-cache key partitioning for distinct LLM clients/models.

use astraweave_core::{
    CompanionState, Constraints, IVec2, PlayerState, ToolRegistry, ToolSpec, WorldSnapshot,
};
use astraweave_llm::cache::PromptKey;
use astraweave_llm::{build_prompt, LlmClient};
use async_trait::async_trait;
use std::any::Any;
use std::collections::BTreeMap;

struct ClientAlpha;
struct ClientBeta;

#[async_trait]
impl LlmClient for ClientAlpha {
    fn llm_cache_model_id(&self) -> &str {
        "model-alpha"
    }

    async fn complete(&self, _prompt: &str) -> anyhow::Result<String> {
        Ok(String::new())
    }
}

#[async_trait]
impl LlmClient for ClientBeta {
    fn llm_cache_model_id(&self) -> &str {
        "model-beta"
    }

    async fn complete(&self, _prompt: &str) -> anyhow::Result<String> {
        Ok(String::new())
    }
}

fn snapshot_and_registry() -> (WorldSnapshot, ToolRegistry) {
    let snap = WorldSnapshot {
        t: 0.0,
        me: CompanionState {
            ammo: 10,
            cooldowns: BTreeMap::new(),
            morale: 1.0,
            pos: IVec2 { x: 0, y: 0 },
        },
        player: PlayerState {
            hp: 100,
            pos: IVec2 { x: 0, y: 0 },
            stance: "stand".into(),
            orders: vec![],
        },
        enemies: vec![],
        pois: vec![],
        obstacles: vec![],
        objective: Some("hold".into()),
        hazards: [astraweave_core::WeaveHazard::EMPTY; astraweave_core::MAX_WEAVE_HAZARDS],
        hazard_count: 0,
    };
    let reg = ToolRegistry {
        tools: vec![ToolSpec {
            name: "MoveTo".into(),
            args: BTreeMap::new(),
        }],
        constraints: Constraints {
            enforce_cooldowns: false,
            enforce_los: false,
            enforce_stamina: false,
        },
    };
    (snap, reg)
}

fn cache_key(client: &dyn LlmClient, snap: &WorldSnapshot, reg: &ToolRegistry) -> PromptKey {
    let prompt = build_prompt(snap, reg);
    let tools: Vec<&str> = reg.tools.iter().map(|t| t.name.as_str()).collect();
    let client_id = format!("{:?}", client.type_id());
    PromptKey::new(
        &prompt,
        &client_id,
        client.llm_cache_model_id(),
        client.llm_cache_temperature(),
        &tools,
    )
}

#[test]
fn different_clients_and_models_use_distinct_cache_keys_for_same_prompt() {
    let (snap, reg) = snapshot_and_registry();
    let alpha: &dyn LlmClient = &ClientAlpha;
    let beta: &dyn LlmClient = &ClientBeta;

    let key_alpha = cache_key(alpha, &snap, &reg);
    let key_beta = cache_key(beta, &snap, &reg);

    assert_ne!(key_alpha, key_beta);
    assert_eq!(key_alpha.model, "model-alpha");
    assert_eq!(key_beta.model, "model-beta");
    assert_ne!(key_alpha.client_id, key_beta.client_id);
}
