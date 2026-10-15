//! Deterministic mod script system.
//!
//! Scripts run at the start of the simulation stage, in mod-id then path order,
//! and entities in id order. Draws come from [`roll_i64`] seeded by
//! `(world_seed, tick, system_id, entity)`. Reloads staged in
//! [`ScriptReloadQueue`] replace source bytes only at the next call of
//! [`mod_script_system`]. They do not run mid-tick.

use crate::ScriptReloadQueue;
use astraweave_ecs::{App, Entity, Plugin, SystemStage, World};
use astraweave_security::sandbox::{
    execute_deterministic, roll_i64, sha256_bytes, SandboxEngine, SandboxProfile, ScriptReplayLog,
    DEFAULT_MAX_MOVE_DELTA,
};
use astraweave_security::ScriptSandbox;
use rhai::{Engine, Scope};
use sha2::{Digest, Sha256};

/// Stable id mixed into the per-entity seed. ASCII "MODS".
pub const MOD_SYSTEM_ID: u64 = 0x4D4F_4453;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CModAgent {
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModClock {
    pub world_seed: u64,
    pub tick: u64,
}

impl ModClock {
    pub fn new(world_seed: u64) -> Self {
        Self {
            world_seed,
            tick: 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModScriptSource {
    pub mod_id: String,
    pub path: String,
    pub source: String,
    pub sha256: [u8; 32],
}

impl ModScriptSource {
    pub fn new(
        mod_id: impl Into<String>,
        path: impl Into<String>,
        source: impl Into<String>,
    ) -> Self {
        let mod_id = mod_id.into();
        let path = path.into();
        let source = source.into();
        let sha256 = sha256_bytes(source.as_bytes());
        Self {
            mod_id,
            path,
            source,
            sha256,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct ModPrograms {
    pub scripts: Vec<ModScriptSource>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ModScriptErrors {
    pub messages: Vec<String>,
}

struct ModEngine(ScriptSandbox);

/// Reload queue for [`mod_script_system`]. Separate from the `CScript` queue so
/// one system cannot consume the other's staged bytes.
#[derive(Clone, Debug, Default)]
pub struct ModReloadQueue(pub ScriptReloadQueue);

#[derive(Clone)]
struct ScriptDrawRng {
    world_seed: u64,
    tick: u64,
    system_id: u64,
    entity: u64,
    draw: u64,
}

#[derive(Clone, Default)]
struct MoveBuf {
    target: Option<(i64, i64)>,
}

pub struct ModScriptPlugin;

/// Install [`ModScriptPlugin`] on the engine app `build_app` creates.
///
/// Call this once before `astraweave_core::ecs_adapter::build_app`. Core cannot
/// name this plugin (scripting depends on core), so `build_app` runs the
/// installer recorded here. A second call is a no-op.
pub fn register_mod_runtime() {
    astraweave_core::ecs_adapter::register_sim_extension(install_mod_script_plugin);
}

fn install_mod_script_plugin(app: &mut App) {
    ModScriptPlugin.build(app);
}

impl Plugin for ModScriptPlugin {
    fn build(&self, app: &mut App) {
        if app.world.get_resource::<ModEngine>().is_some() {
            return;
        }
        let mut built = SandboxEngine::build(&SandboxProfile::sim());
        register_sim_host(built.engine_mut());
        let sandbox = built.into_script_sandbox();
        app.world.insert_resource(ModEngine(sandbox));
        if app.world.get_resource::<ModClock>().is_none() {
            app.world.insert_resource(ModClock::new(0));
        }
        if app.world.get_resource::<ModPrograms>().is_none() {
            app.world.insert_resource(ModPrograms::default());
        }
        if app.world.get_resource::<ModReloadQueue>().is_none() {
            app.world.insert_resource(ModReloadQueue::default());
        }
        if app.world.get_resource::<ScriptReplayLog>().is_none() {
            app.world.insert_resource(ScriptReplayLog::default());
        }
        app.world.insert_resource(ModScriptErrors::default());
        app.add_system(SystemStage::SIMULATION, mod_script_system)
            .expect("registered schedule stage");
    }
}

fn register_sim_host(engine: &mut Engine) {
    engine
        .register_type_with_name::<ScriptDrawRng>("SimRng")
        .register_fn(
            "sim_roll",
            |rng: &mut ScriptDrawRng, bound: i64| -> Result<i64, Box<rhai::EvalAltResult>> {
                let value = roll_i64(
                    rng.world_seed,
                    rng.tick,
                    rng.system_id,
                    rng.entity,
                    rng.draw,
                    bound,
                )
                .map_err(|err| {
                    Box::new(rhai::EvalAltResult::ErrorRuntime(
                        err.to_string().into(),
                        rhai::Position::NONE,
                    ))
                })?;
                rng.draw = rng.draw.wrapping_add(1);
                Ok(value)
            },
        );
    engine
        .register_type_with_name::<MoveBuf>("MoveBuf")
        .register_fn(
            "move_to",
            |buf: &mut MoveBuf, x: i64, y: i64| -> Result<(), Box<rhai::EvalAltResult>> {
                let limit = i64::from(DEFAULT_MAX_MOVE_DELTA);
                if x.abs() > limit || y.abs() > limit {
                    return Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        format!("move_to ({x}, {y}) exceeds ceiling {limit}").into(),
                        rhai::Position::NONE,
                    )));
                }
                buf.target = Some((x, y));
                Ok(())
            },
        );
}

/// One simulation step of every mod script.
pub fn mod_script_system(world: &mut World) {
    if let Some(slot) = world.get_resource_mut::<ModScriptErrors>() {
        slot.messages.clear();
    }
    apply_logged_swaps(world);
    apply_program_reloads(world);

    let sandbox = match world.get_resource::<ModEngine>() {
        Some(engine) => engine.0.clone(),
        None => return,
    };
    let clock = world
        .get_resource::<ModClock>()
        .copied()
        .unwrap_or(ModClock {
            world_seed: 0,
            tick: 0,
        });
    let programs = world
        .get_resource::<ModPrograms>()
        .cloned()
        .unwrap_or_default();

    let mut entities = world.entities_with::<CModAgent>();
    entities.sort();

    let mut errors = Vec::new();
    let mut writes: Vec<(Entity, i32, i32)> = Vec::new();

    for script in &programs.scripts {
        for entity in &entities {
            let Some(agent) = world.get::<CModAgent>(*entity) else {
                continue;
            };
            let agent = *agent;
            match run_one(&sandbox, script, &clock, entity.to_raw(), agent) {
                Ok(Some((x, y))) => writes.push((*entity, x, y)),
                Ok(None) => {}
                Err(message) => {
                    errors.push(format!("{}:{}:{message}", script.path, entity.to_raw()))
                }
            }
        }
    }

    for (entity, x, y) in writes {
        if let Some(agent) = world.get_mut::<CModAgent>(entity) {
            agent.x = x;
            agent.y = y;
        }
    }

    if let Some(slot) = world.get_resource_mut::<ModScriptErrors>() {
        slot.messages.extend(errors);
        slot.messages.sort();
    }
    if let Some(clock) = world.get_resource_mut::<ModClock>() {
        clock.tick = clock.tick.wrapping_add(1);
    }
}

/// Stage every [`ScriptReplayLog`] entry for this tick. A record whose bytes
/// do not match `content_sha256` is an error and is not applied.
fn apply_logged_swaps(world: &mut World) {
    let tick = world
        .get_resource::<ModClock>()
        .map(|clock| clock.tick)
        .unwrap_or(0);
    let (staged, errors) = {
        let Some(log) = world.get_resource::<ScriptReplayLog>() else {
            return;
        };
        let mut staged: Vec<(String, String)> = Vec::new();
        let mut errors = Vec::new();
        for swap in log.at_tick(tick) {
            if !swap.hash_matches() {
                errors.push(format!(
                    "script swap at tick {tick} for {} failed its content hash",
                    swap.path
                ));
                continue;
            }
            staged.push((swap.path.clone(), swap.source.clone()));
        }
        (staged, errors)
    };
    if !errors.is_empty() {
        if let Some(slot) = world.get_resource_mut::<ModScriptErrors>() {
            slot.messages.extend(errors);
        }
    }
    if staged.is_empty() {
        return;
    }
    let Some(queue) = world.get_resource_mut::<ModReloadQueue>() else {
        return;
    };
    for (path, source) in staged {
        queue.0.stage(path, source);
    }
}

fn apply_program_reloads(world: &mut World) {
    let pending = match world.get_resource_mut::<ModReloadQueue>() {
        Some(queue) => queue.0.take_boundary(),
        None => return,
    };
    if pending.is_empty() {
        return;
    }
    let Some(programs) = world.get_resource_mut::<ModPrograms>() else {
        return;
    };
    for script in &mut programs.scripts {
        if let Some(staged) = pending.get(&script.path) {
            script.source.clone_from(&staged.source);
            script.sha256 = staged.sha256;
        }
    }
}

fn run_one(
    sandbox: &ScriptSandbox,
    script: &ModScriptSource,
    clock: &ModClock,
    entity_raw: u64,
    agent: CModAgent,
) -> Result<Option<(i32, i32)>, String> {
    let mut scope = Scope::new();
    scope.push(
        "rng",
        ScriptDrawRng {
            world_seed: clock.world_seed,
            tick: clock.tick,
            system_id: MOD_SYSTEM_ID,
            entity: entity_raw,
            draw: 0,
        },
    );
    scope.push("moves", MoveBuf::default());
    scope.push("pos_x", i64::from(agent.x));
    scope.push("pos_y", i64::from(agent.y));
    scope.push("entity_id", entity_raw as i64);
    let _value = execute_deterministic(sandbox, &script.source, &mut scope)
        .map_err(|err| err.to_string())?;
    let moves = scope
        .get_value::<MoveBuf>("moves")
        .ok_or_else(|| "move buffer missing".to_string())?;
    let Some((x, y)) = moves.target else {
        return Ok(None);
    };
    let x = i32::try_from(x).map_err(|_| format!("x {x} does not fit i32"))?;
    let y = i32::try_from(y).map_err(|_| format!("y {y} does not fit i32"))?;
    Ok(Some((x, y)))
}

/// SHA-256 of the clock, script bytes, errors, and agent positions.
///
/// Layout: little-endian integers, agents sorted by entity id, `f32` absent.
pub fn mod_state_digest(world: &World) -> [u8; 32] {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"AWMOD1");
    let clock = world
        .get_resource::<ModClock>()
        .copied()
        .unwrap_or(ModClock {
            world_seed: 0,
            tick: 0,
        });
    bytes.extend_from_slice(&clock.world_seed.to_le_bytes());
    bytes.extend_from_slice(&clock.tick.to_le_bytes());
    let programs = world.get_resource::<ModPrograms>();
    let script_count = programs.map(|p| p.scripts.len()).unwrap_or(0) as u32;
    bytes.extend_from_slice(&script_count.to_le_bytes());
    if let Some(programs) = world.get_resource::<ModPrograms>() {
        for script in &programs.scripts {
            push_str(&mut bytes, &script.mod_id);
            push_str(&mut bytes, &script.path);
            bytes.extend_from_slice(&script.sha256);
        }
    }
    let errors = world.get_resource::<ModScriptErrors>();
    let messages = errors.map(|e| e.messages.as_slice()).unwrap_or(&[]);
    bytes.extend_from_slice(&(messages.len() as u32).to_le_bytes());
    for message in messages {
        push_str(&mut bytes, message);
    }
    let mut entities = world.entities_with::<CModAgent>();
    entities.sort();
    bytes.extend_from_slice(&(entities.len() as u32).to_le_bytes());
    for entity in entities {
        bytes.extend_from_slice(&entity.to_raw().to_le_bytes());
        if let Some(agent) = world.get::<CModAgent>(entity) {
            bytes.extend_from_slice(&agent.x.to_le_bytes());
            bytes.extend_from_slice(&agent.y.to_le_bytes());
        }
    }
    let digest = Sha256::digest(&bytes);
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest);
    out
}

fn push_str(bytes: &mut Vec<u8>, text: &str) {
    let len = u32::try_from(text.len()).unwrap_or(u32::MAX);
    bytes.extend_from_slice(&len.to_le_bytes());
    bytes.extend_from_slice(text.as_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;
    use astraweave_ecs::App;

    const SCRIPT: &str = r#"
        let roll = rng.sim_roll(4);
        moves.move_to(pos_x + roll, pos_y + 1);
    "#;

    fn world_with_two_agents(seed: u64) -> App {
        let mut app = App::new().add_plugin(ModScriptPlugin);
        app.world.insert_resource(ModClock::new(seed));
        app.world.insert_resource(ModPrograms {
            scripts: vec![ModScriptSource::new(
                "demo.mod",
                "scripts/tick.rhai",
                SCRIPT,
            )],
        });
        for (x, y) in [(0, 0), (3, 1)] {
            let entity = app.world.spawn();
            app.world.insert(entity, CModAgent { x, y });
        }
        app
    }

    #[test]
    fn two_runs_match_and_reload_waits_for_the_next_tick() {
        let mut first = world_with_two_agents(0xA11C);
        let mut second = world_with_two_agents(0xA11C);
        for _ in 0..8 {
            mod_script_system(&mut first.world);
            mod_script_system(&mut second.world);
        }
        let digest = mod_state_digest(&first.world);
        assert_eq!(digest, mod_state_digest(&second.world));

        let before = agents(&first.world);
        first
            .world
            .get_resource_mut::<ModReloadQueue>()
            .unwrap()
            .0
            .stage("scripts/tick.rhai", "moves.move_to(5, 7);");
        assert_eq!(agents(&first.world), before, "staging must not move agents");
        mod_script_system(&mut first.world);
        let after = agents(&first.world);
        assert_ne!(after, before);
        assert!(after.iter().all(|(_, x, y)| *x == 5 && *y == 7));

        let mut replay = world_with_two_agents(0xA11C);
        for _ in 0..8 {
            mod_script_system(&mut replay.world);
        }
        replay
            .world
            .get_resource_mut::<ModReloadQueue>()
            .unwrap()
            .0
            .stage("scripts/tick.rhai", "moves.move_to(5, 7);");
        mod_script_system(&mut replay.world);
        assert_eq!(
            mod_state_digest(&first.world),
            mod_state_digest(&replay.world)
        );
    }

    #[test]
    fn reload_mid_run_replays_to_the_identical_hash() {
        const SWAP_TICK: u64 = 4;
        const TOTAL: u64 = 12;
        const PATH: &str = "scripts/tick.rhai";
        const NEXT: &str = "moves.move_to(5, 7);\n";
        let seed = 0xA11C_E5E0;

        let mut log = ScriptReplayLog::default();
        let mut live = world_with_two_agents(seed);
        for _ in 0..TOTAL {
            let tick = live.world.get_resource::<ModClock>().unwrap().tick;
            if tick == SWAP_TICK {
                log.record(tick, PATH, NEXT);
                live.world
                    .get_resource_mut::<ModReloadQueue>()
                    .unwrap()
                    .0
                    .stage(PATH, NEXT);
            }
            mod_script_system(&mut live.world);
        }
        let live_hash = mod_state_digest(&live.world);
        assert_eq!(log.swaps.len(), 1);
        assert_eq!(log.swaps[0].tick, SWAP_TICK);
        assert!(log.swaps[0].hash_matches());

        let mut replay = world_with_two_agents(seed);
        replay.world.insert_resource(log);
        for _ in 0..TOTAL {
            mod_script_system(&mut replay.world);
        }
        assert_eq!(live_hash, mod_state_digest(&replay.world));
    }

    #[test]
    fn over_ceiling_move_does_not_change_position() {
        let mut app = world_with_two_agents(1);
        app.world.insert_resource(ModPrograms {
            scripts: vec![ModScriptSource::new(
                "demo.mod",
                "scripts/tick.rhai",
                "moves.move_to(1000000, 0);",
            )],
        });
        let before = agents(&app.world);
        mod_script_system(&mut app.world);
        assert_eq!(agents(&app.world), before);
        let errors = app.world.get_resource::<ModScriptErrors>().unwrap();
        assert!(!errors.messages.is_empty());
    }

    #[test]
    fn mod_loaded_through_build_app_replays_identically() {
        use astraweave_core::ecs_adapter::build_app;
        use astraweave_core::World as LegacyWorld;
        use astraweave_security::sandbox::{load_mod_dir, EngineCeilings, LoadedMod};

        const SWAP_TICK: u64 = 4;
        const TOTAL: u32 = 12;
        const PATH: &str = "scripts/tick.rhai";
        const NEXT: &str = "moves.move_to(5, 7);\n";
        let seed = 0xA11C_E5E0;

        register_mod_runtime();
        let loaded = load_fixture_mod();
        assert_eq!(loaded.scripts.len(), 1);
        assert!(loaded.scripts.contains_key(PATH));

        let mut live = app_with_loaded_mod(&loaded, seed);
        assert!(
            live.world.get_resource::<ModReloadQueue>().is_some(),
            "build_app must install ModScriptPlugin"
        );
        let mut recorded = ScriptReplayLog::default();
        for _ in 0..TOTAL {
            let tick = live.world.get_resource::<ModClock>().expect("clock").tick;
            if tick == SWAP_TICK {
                recorded.record(tick, PATH, NEXT);
                live.world
                    .get_resource_mut::<ScriptReplayLog>()
                    .expect("replay log")
                    .record(tick, PATH, NEXT);
            }
            live = live.run_fixed(1);
        }
        let live_hash = mod_state_digest(&live.world);
        assert!(
            agents(&live.world)
                .iter()
                .all(|(_, x, y)| *x == 5 && *y == 7),
            "the swapped script must run inside the build_app schedule"
        );
        assert_eq!(recorded.swaps.len(), 1);
        assert!(recorded.swaps[0].hash_matches());

        let mut replay = app_with_loaded_mod(&loaded, seed);
        replay.world.insert_resource(recorded);
        replay = replay.run_fixed(TOTAL);
        assert_eq!(live_hash, mod_state_digest(&replay.world));

        fn load_fixture_mod() -> LoadedMod {
            let root = std::env::temp_dir().join(format!(
                "aw-mod-build-app-{}-{}",
                std::process::id(),
                SWAP_TICK
            ));
            if root.exists() {
                std::fs::remove_dir_all(&root).expect("cleanup stale mod dir");
            }
            std::fs::create_dir_all(root.join("scripts")).expect("mod dir");
            std::fs::write(
                root.join("mod.toml"),
                r#"
schema = 1
id = "demo.mod"
name = "Demo"
version = "1.0.0"
api = 1
capabilities = ["move_entity", "sim_rng"]
scripts = ["scripts/tick.rhai"]
"#,
            )
            .expect("manifest");
            std::fs::write(root.join(PATH), SCRIPT).expect("script");
            let loaded = load_mod_dir(&root, &EngineCeilings::default()).expect("load mod");
            std::fs::remove_dir_all(&root).expect("cleanup");
            loaded
        }

        fn app_with_loaded_mod(loaded: &LoadedMod, seed: u64) -> App {
            let mut app = build_app(LegacyWorld::new(), 1.0 / 60.0);
            app.world.insert_resource(ModClock::new(seed));
            app.world.insert_resource(ModPrograms {
                scripts: loaded
                    .manifest
                    .scripts
                    .iter()
                    .map(|path| {
                        ModScriptSource::new(
                            loaded.manifest.id.clone(),
                            path.clone(),
                            loaded
                                .scripts
                                .get(path)
                                .expect("manifest path is the script key")
                                .clone(),
                        )
                    })
                    .collect(),
            });
            for (x, y) in [(0, 0), (3, 1)] {
                let entity = app.world.spawn();
                app.world.insert(entity, CModAgent { x, y });
            }
            app
        }
    }

    fn agents(world: &World) -> Vec<(u64, i32, i32)> {
        let mut entities = world.entities_with::<CModAgent>();
        entities.sort();
        entities
            .into_iter()
            .map(|entity| {
                let agent = world.get::<CModAgent>(entity).copied().unwrap();
                (entity.to_raw(), agent.x, agent.y)
            })
            .collect()
    }
}
