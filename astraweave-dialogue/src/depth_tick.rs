//! Deterministic AI depth tick.
//!
//! One ECS system calls memory, embeddings, RAG consolidation, prompts,
//! context, persona, NPC planning, and coordination. Live model output is
//! not consulted. A line for tick N is taken from [`AiDepthState`]'s recorded
//! log when one was stored, and otherwise the rendered prompt is echoed with
//! a `scripted:` prefix.
//!
//! `llm_dialogue.rs` stays out of the crate. It calls `rand::thread_rng`
//! (removed in rand 0.9) and a live [`astraweave_llm::LlmClient`]. Compiling
//! it as a module would put that RNG on the dialogue crate, and calling it
//! from this system would put a nondeterministic model on the tick.

use std::collections::{BTreeMap, HashMap};

use astraweave_context::{ContextConfig, ConversationHistory, Message, Role};
use astraweave_coordination::{RoutingAction, RoutingRule, SimCoordinator, SimMessage};
use astraweave_ecs::{App, Plugin, World};
use astraweave_embeddings::{embed_sim, DistanceMetric, EmbeddingConfig, VectorStore};
use astraweave_memory::{CompanionProfile, Fact, Memory, MemoryManager};
use astraweave_npc::{
    CommandSink, NpcId, NpcManager, NpcProfile, NpcWorldView, Role as NpcRole, RulePlanner,
};
use astraweave_persona::seal_profile;
use astraweave_prompts::{ContextValue, PromptContext, PromptTemplate, TemplateEngine};
use astraweave_rag::{ConsolidationConfig, ConsolidationEngine, VectorStoreWrapper};
use glam::Vec3;
use sha2::{Digest, Sha256};

const SIM_DIMS: usize = 8;

/// ECS plugin that registers [`sys_ai_depth`] on the `ai_planning` stage.
pub struct AiDepthPlugin;

impl Plugin for AiDepthPlugin {
    fn build(&self, app: &mut App) {
        install(app);
    }
}

/// Insert [`AiDepthState`] and register the depth system.
///
/// `App::new` already has an `ai_planning` stage, so the system is not
/// silent-dropped. A second call registers the system again; call once per app.
pub fn install(app: &mut App) {
    if app.world.get_resource::<AiDepthState>().is_none() {
        app.world.insert_resource(AiDepthState::new(1));
    }
    app.add_system_known_stage("ai_planning", sys_ai_depth);
}

fn sys_ai_depth(world: &mut World) {
    if let Some(state) = world.get_resource_mut::<AiDepthState>() {
        state.step();
    }
}

/// Recorded model text for one tick.
///
/// The line is the log entry when present, otherwise the rendered prompt.
/// `SimRng` is seeded with `(world_seed, tick, SIM_SYSTEM_LLM_DIALOGUE + entity)`
/// and is not drawn. `llm_dialogue.rs` is not a module yet: wiring it in
/// currently fails to compile (see the deferred-failure issue). This function
/// is the tick boundary that module will own.
pub fn apply_recorded_line(
    world_seed: u64,
    tick: u64,
    entity: u64,
    recorded: Option<&str>,
    prompt: &str,
) -> String {
    let stream = astraweave_core::SimRng::new(
        world_seed,
        tick,
        astraweave_core::SIM_SYSTEM_LLM_DIALOGUE.wrapping_add(entity),
    );
    debug_assert_eq!(stream.tick(), tick);
    let _ = (stream.world_seed(), stream.system_id());
    match recorded {
        Some(line) => line.to_string(),
        None => format!("scripted:{prompt}"),
    }
}

/// State the depth system advances once per fixed step.
pub struct AiDepthState {
    seed: u64,
    tick: u64,
    memory: MemoryManager,
    store: VectorStoreWrapper,
    profile: CompanionProfile,
    history: ConversationHistory,
    prompts: TemplateEngine,
    npcs: NpcManager,
    merchant_id: NpcId,
    guard_id: NpcId,
    coordination: SimCoordinator,
    recorded_lines: BTreeMap<u64, String>,
    trace: Vec<TickTrace>,
}

struct TickTrace {
    tick: u64,
    memory_ids: Vec<String>,
    hits: Vec<(String, u64)>,
    consolidated: Vec<(String, String, u64)>,
    prompt: String,
    context_id: String,
    signature: String,
    npc_lines: Vec<String>,
    deliveries: Vec<(String, String)>,
    dialogue: String,
    note: String,
}

impl AiDepthState {
    pub fn new(seed: u64) -> Self {
        let mut prompts = TemplateEngine::new();
        let template = PromptTemplate::new("depth", "tick {{tick}} name {{name}}");
        let note = match prompts.register_template("depth", template) {
            Ok(()) => String::new(),
            Err(err) => format!("prompt-register:{err}"),
        };

        let config = EmbeddingConfig {
            dimensions: SIM_DIMS,
            distance_metric: DistanceMetric::Manhattan,
            ..EmbeddingConfig::default()
        };
        let store = VectorStoreWrapper::new(VectorStore::with_config(config));

        let mut profile = CompanionProfile::new_default();
        profile.id = format!("companion-{seed}");
        profile.persona.name = "Elena".to_string();
        profile.persona.tone = "steady".to_string();

        let mut npcs = NpcManager::new(Box::new(RulePlanner));
        let merchant_id = npcs.spawn_logical(npc_profile("merchant", NpcRole::Merchant, "Mara"));
        let guard_id = npcs.spawn_logical(npc_profile("guard", NpcRole::Guard, "Bren"));

        let mut coordination = SimCoordinator::new();
        coordination.register_agent("merchant");
        coordination.register_agent("guard");
        coordination.add_rule(RoutingRule {
            id: "hear".to_string(),
            from_pattern: Some("merchant".to_string()),
            to_pattern: Some("guard".to_string()),
            message_type: None,
            action: RoutingAction::Transform("heard:".to_string()),
            priority: 1,
        });

        Self {
            seed,
            tick: 0,
            memory: MemoryManager::new(),
            store,
            profile,
            history: ConversationHistory::new(ContextConfig::default()),
            prompts,
            npcs,
            merchant_id,
            guard_id,
            coordination,
            recorded_lines: BTreeMap::new(),
            trace: if note.is_empty() {
                Vec::new()
            } else {
                vec![TickTrace {
                    tick: 0,
                    memory_ids: Vec::new(),
                    hits: Vec::new(),
                    consolidated: Vec::new(),
                    prompt: String::new(),
                    context_id: String::new(),
                    signature: String::new(),
                    npc_lines: Vec::new(),
                    deliveries: Vec::new(),
                    dialogue: String::new(),
                    note,
                }]
            },
        }
    }

    pub fn tick(&self) -> u64 {
        self.tick
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// Store a model completion that replay will apply on `tick` instead of
    /// calling a model.
    pub fn record_line(&mut self, tick: u64, line: impl Into<String>) {
        self.recorded_lines.insert(tick, line.into());
    }

    pub fn trace_joined(&self) -> String {
        let mut lines = Vec::new();
        for trace in &self.trace {
            lines.push(format!(
                "{}|{}|{}|{}|{}|{}|{}|{}|{}",
                trace.tick,
                trace.memory_ids.join(","),
                trace
                    .hits
                    .iter()
                    .map(|(id, distance)| format!("{id}:{distance}"))
                    .collect::<Vec<_>>()
                    .join(","),
                trace
                    .consolidated
                    .iter()
                    .map(|(id, text, merged)| format!("{id}:{text}:{merged}"))
                    .collect::<Vec<_>>()
                    .join(","),
                trace.prompt,
                trace.signature,
                trace.npc_lines.join(","),
                trace
                    .deliveries
                    .iter()
                    .map(|(to, body)| format!("{to}:{body}"))
                    .collect::<Vec<_>>()
                    .join(","),
                trace.dialogue,
            ));
        }
        lines.join("\n")
    }

    /// Canonical little-endian encoding of every sim value this system wrote.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        push_u64(&mut bytes, self.seed);
        push_u64(&mut bytes, self.tick);
        push_u64(&mut bytes, self.trace.len() as u64);
        for trace in &self.trace {
            push_u64(&mut bytes, trace.tick);
            push_strings(&mut bytes, &trace.memory_ids);
            push_u64(&mut bytes, trace.hits.len() as u64);
            for (id, distance) in &trace.hits {
                push_str(&mut bytes, id);
                push_u64(&mut bytes, *distance);
            }
            push_u64(&mut bytes, trace.consolidated.len() as u64);
            for (id, text, merged) in &trace.consolidated {
                push_str(&mut bytes, id);
                push_str(&mut bytes, text);
                push_u64(&mut bytes, *merged);
            }
            push_str(&mut bytes, &trace.prompt);
            push_str(&mut bytes, &trace.context_id);
            push_str(&mut bytes, &trace.signature);
            push_strings(&mut bytes, &trace.npc_lines);
            push_u64(&mut bytes, trace.deliveries.len() as u64);
            for (to, body) in &trace.deliveries {
                push_str(&mut bytes, to);
                push_str(&mut bytes, body);
            }
            push_str(&mut bytes, &trace.dialogue);
            push_str(&mut bytes, &trace.note);
        }
        bytes
    }

    pub fn replay_hash(&self) -> String {
        let digest = Sha256::digest(self.canonical_bytes());
        hex_encode(&digest)
    }

    pub fn step(&mut self) {
        self.tick = self.tick.saturating_add(1);
        let tick = self.tick;
        let mut note = String::new();

        let text = if tick % 2 == 0 {
            format!("seed {} guard saw the player at the gate", self.seed)
        } else {
            format!("seed {} guard saw the player at the gate today", self.seed)
        };
        let memory = Memory::episodic_at_tick(
            format!("mem-{tick}"),
            text.clone(),
            vec!["guard".to_string()],
            Some("gate".to_string()),
            tick,
        );
        if let Err(err) = self.memory.store_memory(memory) {
            note.push_str(&format!("memory:{err};"));
        }
        let memory_ids = self.memory.retrieve_sorted("player gate", 4);

        let vector = embed_sim(&text, SIM_DIMS);
        if let Err(err) = self
            .store
            .insert_at(format!("vec-{tick}"), vector, text, tick)
        {
            note.push_str(&format!("embed:{err};"));
        }
        let query = embed_sim(&format!("seed {} player at the gate", self.seed), SIM_DIMS);
        let hits = match self.store.search_sync(&query, 3) {
            Ok(results) => results
                .into_iter()
                .map(|hit| (hit.vector.id, exact_distance(hit.distance)))
                .collect(),
            Err(err) => {
                note.push_str(&format!("search:{err};"));
                Vec::new()
            }
        };

        let mut consolidated = Vec::new();
        if tick % 4 == 0 {
            let engine = ConsolidationEngine::new(ConsolidationConfig::default());
            match engine.consolidate(self.store.memories_sorted()) {
                Ok((memories, result)) => {
                    for memory in memories {
                        consolidated.push((memory.id, memory.text, result.merged_count as u64));
                    }
                }
                Err(err) => note.push_str(&format!("consolidate:{err};")),
            }
        }

        self.profile.add_fact(Fact {
            k: format!("tick-{tick}"),
            v: tick.to_string(),
            t: "sim".to_string(),
        });
        let signature = seal_profile(&mut self.profile);

        let context_message = Message::recorded(tick, 0, Role::Assistant, format!("tick {tick}"));
        let context_id = self.history.push_recorded(context_message);

        let mut prompt_ctx = PromptContext::new();
        prompt_ctx.set("tick".to_string(), ContextValue::String(tick.to_string()));
        prompt_ctx.set(
            "name".to_string(),
            ContextValue::String(self.profile.persona.name.clone()),
        );
        let prompt = match self.prompts.render("depth", &prompt_ctx) {
            Ok(rendered) => rendered,
            Err(err) => {
                note.push_str(&format!("render:{err};"));
                String::new()
            }
        };

        let utterance = if tick % 2 == 0 { "hello" } else { "shop" };
        let view = NpcWorldView {
            time_of_day: 12.0,
            self_pos: Vec3::ZERO,
            player_pos: None,
            player_dist: None,
            nearby_threat: false,
            location_tag: Some("gate".to_string()),
        };
        for id in [self.merchant_id, self.guard_id] {
            if let Err(err) = self.npcs.handle_player_utterance(id, &view, utterance) {
                note.push_str(&format!("npc:{err};"));
            }
        }
        let mut sink = RecordingSink { lines: Vec::new() };
        let views = HashMap::new();
        self.npcs.update(1.0, &mut sink, &views);

        let deliveries = self.coordination.tick(
            tick,
            &[SimMessage {
                from: "merchant".to_string(),
                to: "guard".to_string(),
                body: format!("tick-{tick}"),
            }],
        );
        let deliveries = deliveries
            .into_iter()
            .map(|delivery| (delivery.to, delivery.body))
            .collect();

        let recorded = self.recorded_lines.get(&tick).cloned();
        let dialogue = apply_recorded_line(self.seed, tick, 1, recorded.as_deref(), &prompt);

        self.trace.push(TickTrace {
            tick,
            memory_ids,
            hits,
            consolidated,
            prompt,
            context_id,
            signature,
            npc_lines: sink.lines,
            deliveries,
            dialogue,
            note,
        });
    }
}

struct RecordingSink {
    lines: Vec<String>,
}

impl CommandSink for RecordingSink {
    fn move_character(&mut self, body: astraweave_npc::BodyId, _dir: Vec3, _speed: f32) {
        self.lines.push(format!("move:{body}"));
    }

    fn say(&mut self, speaker: &str, text: &str) {
        self.lines.push(format!("say:{speaker}:{text}"));
    }

    fn open_shop(&mut self, npc_id: NpcId) {
        self.lines.push(format!("shop:{npc_id}"));
    }

    fn call_guards(&mut self, _pos: Vec3, reason: &str) {
        self.lines.push(format!("guards:{reason}"));
    }

    fn give_quest(&mut self, npc_id: NpcId, quest_id: &str) {
        self.lines.push(format!("quest:{npc_id}:{quest_id}"));
    }
}

fn npc_profile(id: &str, role: NpcRole, name: &str) -> NpcProfile {
    NpcProfile {
        id: id.to_string(),
        role,
        persona: astraweave_npc::Persona {
            display_name: name.to_string(),
            traits: vec!["steady".to_string()],
            backstory: String::new(),
            voice_speaker: None,
        },
        memory: astraweave_npc::Memory {
            facts: Vec::new(),
            episodes: Vec::new(),
        },
        home: [0.0, 0.0, 0.0],
        schedule: Vec::new(),
    }
}

fn exact_distance(distance: f32) -> u64 {
    if distance.is_finite() && distance >= 0.0 {
        let as_int = distance as u64;
        if (as_int as f32) == distance {
            return as_int;
        }
    }
    u64::from(distance.to_bits())
}

fn push_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_str(bytes: &mut Vec<u8>, value: &str) {
    let len = u32::try_from(value.len()).unwrap_or(u32::MAX);
    bytes.extend_from_slice(&len.to_le_bytes());
    bytes.extend_from_slice(value.as_bytes());
}

fn push_strings(bytes: &mut Vec<u8>, values: &[String]) {
    push_u64(bytes, values.len() as u64);
    for value in values {
        push_str(bytes, value);
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_steps_call_every_subsystem() {
        let mut app = App::new();
        install(&mut app);
        let app = app.run_fixed(8);
        let state = app
            .world
            .get_resource::<AiDepthState>()
            .expect("depth state");
        assert_eq!(state.tick(), 8);
        let joined = state.trace_joined();
        assert!(joined.contains("mem-"), "{joined}");
        assert!(joined.contains("vec-"), "{joined}");
        assert!(joined.contains("merged_"), "{joined}");
        assert!(joined.contains("heard:"), "{joined}");
        assert!(joined.contains("say:"), "{joined}");
        assert!(joined.contains("scripted:tick "), "{joined}");
        assert!(joined.contains("m-"), "{joined}");
        assert!(!joined.contains("prompt-register"), "{joined}");
    }
}
