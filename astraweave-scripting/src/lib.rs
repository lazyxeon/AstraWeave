//! # AstraWeave Scripting
//!
//! Rhai-based scripting system for game logic, AI behavior, and gameplay customization.
//!
//! ## Features
//! - Entity manipulation via `ScriptCommands`
//! - Physics integration through `PhysicsProxy` and `RaycastHit`
//! - Navigation mesh queries via `NavMeshProxy`
//! - Event-driven communication with `ScriptEvent`
//!
//! ## Quick Start
//! ```ignore
//! let mut engine = Engine::new();
//! register_api(&mut engine);
//! let script = engine.compile(r#"
//!     spawn_entity(prefab_sword);
//! "#)?;
//! ```

use api::{ScriptCommand, ScriptCommands};
use astraweave_core::{CHealth, CPos, IVec2};
use astraweave_ecs::{App, Entity, Events, Plugin, SystemStage, World};
use astraweave_nav::NavMesh;
use astraweave_physics::{CollisionEvent, Layers, PhysicsWorld};
pub use astraweave_security::sandbox::ScriptReloadQueue;
use astraweave_security::sandbox::{SandboxEngine, SandboxProfile};
use events::ScriptEvent;
use glam::Vec3;
use rhai::{Dynamic, Engine, Scope, AST};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

pub mod sim_mod;
pub use sim_mod::register_mod_runtime;

pub mod api;
pub mod events;
pub mod loader;

#[derive(Clone, Copy, Debug)]
pub struct CPhysicsBody {
    pub body_id: u64,
}

/// Rhai script attached to an ECS entity
#[derive(Clone)]
pub struct CScript {
    pub script_path: String,
    /// The actual script content. If empty, system may try to load from path (not implemented in MVP).
    pub source: String,
    pub cached_ast: Option<Arc<AST>>,
    /// Persistent variables maintained across ticks. `BTreeMap` so writeback order is stable.
    pub script_state: BTreeMap<String, Dynamic>,
    pub enabled: bool,
}

impl CScript {
    pub fn new(path: impl Into<String>, source: impl Into<String>) -> Self {
        Self {
            script_path: path.into(),
            source: source.into(),
            cached_ast: None,
            script_state: BTreeMap::new(),
            enabled: true,
        }
    }
}

/// Resource wrapping the Rhai engine
#[derive(Clone)]
pub struct ScriptEngineResource(pub Arc<Engine>);

impl Default for ScriptEngineResource {
    fn default() -> Self {
        // Gameplay profile: finite op/memory caps, no file/module resolver, no clock.
        // Host commands are the ones `register_api` adds; they match MOD_COMMAND_API_VERSION.
        let mut sandbox = SandboxEngine::build(&SandboxProfile::gameplay());
        api::register_api(sandbox.engine_mut());
        Self(Arc::new(sandbox.into_engine()))
    }
}

fn spawn_prefab(world: &mut World, prefab: &str, position: Vec3) -> Entity {
    match prefab {
        "enemy_grunt" => {
            let e = world.spawn();
            world.insert(
                e,
                CPos {
                    pos: IVec2::new(position.x as i32, position.z as i32),
                },
            );
            world.insert(e, CHealth { hp: 100 });
            println!(
                "[ScriptSystem] Spawning enemy_grunt at {} (Entity: {})",
                position, e
            );
            e
        }
        "crate" => {
            let e = world.spawn();
            world.insert(
                e,
                CPos {
                    pos: IVec2::new(position.x as i32, position.z as i32),
                },
            );
            println!(
                "[ScriptSystem] Spawning crate at {} (Entity: {})",
                position, e
            );
            e
        }
        _ => {
            println!("[ScriptSystem] Unknown prefab: {}", prefab);
            // Spawn a default entity so we don't crash, but log error
            let e = world.spawn();
            world.insert(
                e,
                CPos {
                    pos: IVec2::new(position.x as i32, position.z as i32),
                },
            );
            e
        }
    }
}

const HOST_SCOPE_KEYS: &[&str] = &[
    "commands",
    "physics",
    "nav",
    "entity_id",
    "position",
    "health",
    "delta_time",
];

fn is_host_scope_key(key: &str) -> bool {
    HOST_SCOPE_KEYS.contains(&key)
}

/// Apply staged `.rhai` bytes before any script runs. The queue is the sim input.
/// File mtimes are not read here.
fn apply_script_reloads(world: &mut World) {
    let pending = match world.get_resource_mut::<ScriptReloadQueue>() {
        Some(queue) => queue.take_boundary(),
        None => return,
    };
    if pending.is_empty() {
        return;
    }
    let mut entities = world.entities_with::<CScript>();
    entities.sort();
    for entity in entities {
        let Some(script) = world.get_mut::<CScript>(entity) else {
            continue;
        };
        let Some(staged) = pending.get(&script.script_path) else {
            continue;
        };
        script.source.clone_from(&staged.source);
        script.cached_ast = None;
        script.enabled = true;
    }
}

fn script_event_rank(event: &ScriptEvent) -> (u8, u64, u64, u32, String) {
    match event {
        ScriptEvent::OnSpawn { entity } => (0, entity.to_raw(), 0, 0, String::new()),
        ScriptEvent::OnCollision { entity, other } => {
            (1, entity.to_raw(), other.to_raw(), 0, String::new())
        }
        ScriptEvent::OnTrigger {
            entity,
            trigger_name,
        } => (2, entity.to_raw(), 0, 0, trigger_name.clone()),
        ScriptEvent::OnDamage {
            entity,
            damage,
            source,
        } => (
            3,
            entity.to_raw(),
            source.to_raw(),
            damage.to_bits(),
            String::new(),
        ),
    }
}

/// System to execute entity scripts.
///
/// Reloads apply at the start of the tick from [`ScriptReloadQueue`], in path order.
/// The tick does not stat the filesystem and does not read a clock.
pub fn script_system(world: &mut World) {
    apply_script_reloads(world);

    // 1. Get the engine (cheap clone of Arc)
    let engine = if let Some(res) = world.get_resource::<ScriptEngineResource>() {
        res.0.clone()
    } else {
        return;
    };

    // Get PhysicsWorld pointer (immutable borrow)
    // Safety: We drop the borrow immediately, but keep the pointer.
    // We must ensure we don't remove the resource during execution.
    let physics_ptr = if let Some(physics) = world.get_resource::<PhysicsWorld>() {
        physics as *const PhysicsWorld
    } else {
        std::ptr::null()
    };

    // Get NavMesh pointer (immutable borrow)
    let nav_ptr = if let Some(nav) = world.get_resource::<NavMesh>() {
        nav as *const NavMesh
    } else {
        std::ptr::null()
    };

    // Take cache out to avoid borrow conflicts
    let mut cache = if let Some(c) = world.get_resource_mut::<ScriptCache>() {
        std::mem::take(c)
    } else {
        ScriptCache::default()
    };

    // Build Body -> Entity map for PhysicsProxy
    let mut body_to_entity_raw = HashMap::new();
    for e in world.entities_with::<CPhysicsBody>() {
        if let Some(body) = world.get::<CPhysicsBody>(e) {
            body_to_entity_raw.insert(body.body_id, e.to_raw());
        }
    }
    let body_map = Arc::new(body_to_entity_raw);

    let mut all_commands = Vec::new();

    // 2. Run Scripts (Main Body), entity id order.
    let mut entities = world.entities_with::<CScript>();
    entities.sort();
    for entity in entities {
        // Gather Read Data FIRST
        let pos_data = world
            .get::<CPos>(entity)
            .map(|p| Vec3::new(p.pos.x as f32, 0.0, p.pos.y as f32));
        let health_data = world.get::<CHealth>(entity).map(|h| h.hp as i64);

        // We need to re-acquire the component mutably
        if let Some(script) = world.get_mut::<CScript>(entity) {
            if !script.enabled {
                continue;
            }

            if script.cached_ast.is_none() {
                if let Some(cached) = cache.scripts.get(&script.script_path) {
                    if cached.source_hash
                        == astraweave_security::sandbox::sha256_hex(script.source.as_bytes())
                    {
                        script.cached_ast = Some(cached.ast.clone());
                    }
                }
            }

            // Compile if needed (inline scripts or a boundary reload that cleared the AST)
            if script.cached_ast.is_none() {
                if script.source.is_empty() {
                    continue;
                }

                match engine.compile(&script.source) {
                    Ok(ast) => {
                        let ast = Arc::new(ast);
                        if !script.script_path.is_empty() {
                            cache.scripts.insert(
                                script.script_path.clone(),
                                CachedScript {
                                    ast: ast.clone(),
                                    source_hash: astraweave_security::sandbox::sha256_hex(
                                        script.source.as_bytes(),
                                    ),
                                },
                            );
                        }
                        script.cached_ast = Some(ast);
                    }
                    Err(e) => {
                        eprintln!("Script compilation error for {}: {}", script.script_path, e);
                        script.enabled = false; // Disable failing script
                        continue;
                    }
                }
            }

            // Execute
            if let Some(ast) = &script.cached_ast {
                let mut scope = Scope::new();

                // Push state to scope
                for (k, v) in &script.script_state {
                    scope.push_dynamic(k.clone(), v.clone());
                }

                // Inject Entity ID (raw u64 cast to i64 for Rhai)
                scope.push("entity_id", entity.to_raw() as i64);

                // Inject Read Data
                if let Some(p) = pos_data {
                    scope.push("position", p);
                }
                if let Some(h) = health_data {
                    scope.push("health", h);
                }
                scope.push("delta_time", 0.01667_f32);

                // Inject Physics
                let physics_proxy = api::PhysicsProxy {
                    ptr: physics_ptr,
                    body_map: body_map.clone(),
                };
                scope.push("physics", physics_proxy);

                // Inject NavMesh
                let nav_proxy = api::NavMeshProxy { ptr: nav_ptr };
                scope.push("nav", nav_proxy);

                // Inject Commands
                let commands = ScriptCommands::new();
                scope.push("commands", commands);

                // Run the script
                if let Err(e) = engine.run_ast_with_scope(&mut scope, ast) {
                    eprintln!("Script runtime error for {}: {}", script.script_path, e);
                }

                // Update state from scope
                for (k, _, v) in scope.iter() {
                    if is_host_scope_key(k) {
                        continue;
                    }
                    script.script_state.insert(k.to_string(), v);
                }

                // Extract commands
                if let Some(cmds) = scope.get_value::<ScriptCommands>("commands") {
                    all_commands.extend(cmds.commands);
                }
            }
        }
    } // Put cache back
    world.insert_resource(cache);

    // 3. Process Events
    let mut script_events = Vec::new();

    // Read from Events resource
    if let Some(events) = world.get_resource_mut::<Events>() {
        for event in events.drain::<ScriptEvent>() {
            script_events.push(event);
        }
    }

    // Physics Events
    let mut body_to_entity = HashMap::new();
    for e in world.entities_with::<CPhysicsBody>() {
        if let Some(body) = world.get::<CPhysicsBody>(e) {
            body_to_entity.insert(body.body_id, e);
        }
    }

    if let Some(physics) = world.get_resource::<PhysicsWorld>() {
        while let Ok(event) = physics.collision_recv.try_recv() {
            if let CollisionEvent::Started(h1, h2, _) = event {
                let h1_body = physics.colliders.get(h1).and_then(|c| c.parent());
                let h2_body = physics.colliders.get(h2).and_then(|c| c.parent());

                let id1 = h1_body.and_then(|h| physics.id_of(h));
                let id2 = h2_body.and_then(|h| physics.id_of(h));

                if let (Some(bid1), Some(bid2)) = (id1, id2) {
                    if let (Some(e1), Some(e2)) =
                        (body_to_entity.get(&bid1), body_to_entity.get(&bid2))
                    {
                        script_events.push(ScriptEvent::OnCollision {
                            entity: *e1,
                            other: *e2,
                        });
                        script_events.push(ScriptEvent::OnCollision {
                            entity: *e2,
                            other: *e1,
                        });
                    }
                }
            }
        }
    }

    script_events.sort_by_key(script_event_rank);

    // Execute callbacks
    for event in script_events {
        let (entity, callback, args) = match event {
            ScriptEvent::OnCollision { entity, other } => (
                entity,
                "on_collision",
                vec![Dynamic::from(other.to_raw() as i64)],
            ),
            ScriptEvent::OnTrigger {
                entity,
                trigger_name,
            } => (entity, "on_trigger", vec![Dynamic::from(trigger_name)]),
            ScriptEvent::OnDamage {
                entity,
                damage,
                source,
            } => (
                entity,
                "on_damage",
                vec![Dynamic::from(damage), Dynamic::from(source.to_raw() as i64)],
            ),
            ScriptEvent::OnSpawn { entity } => (entity, "on_spawn", vec![]),
        };

        // Gather Read Data FIRST
        let pos_data = world
            .get::<CPos>(entity)
            .map(|p| Vec3::new(p.pos.x as f32, 0.0, p.pos.y as f32));
        let health_data = world.get::<CHealth>(entity).map(|h| h.hp as i64);

        if let Some(script) = world.get_mut::<CScript>(entity) {
            if !script.enabled || script.cached_ast.is_none() {
                continue;
            }
            let ast = script.cached_ast.clone().expect("cached AST must be set");

            let mut scope = Scope::new();
            for (k, v) in &script.script_state {
                scope.push_dynamic(k.clone(), v.clone());
            }
            scope.push("entity_id", entity.to_raw() as i64);

            // Inject Read Data
            if let Some(p) = pos_data {
                scope.push("position", p);
            }
            if let Some(h) = health_data {
                scope.push("health", h);
            }
            scope.push("delta_time", 0.01667_f32);

            // Inject Physics
            let physics_proxy = api::PhysicsProxy {
                ptr: physics_ptr,
                body_map: body_map.clone(),
            };
            scope.push("physics", physics_proxy);

            // Inject NavMesh
            let nav_proxy = api::NavMeshProxy { ptr: nav_ptr };
            scope.push("nav", nav_proxy);

            scope.push("commands", ScriptCommands::new());

            // Call function
            let result: Result<Dynamic, _> = engine.call_fn(&mut scope, &ast, callback, args);

            if result.is_err() {
                // Ignore function not found
            }

            // Update state
            for (k, _, v) in scope.iter() {
                if is_host_scope_key(k) {
                    continue;
                }
                script.script_state.insert(k.to_string(), v);
            }
            if let Some(cmds) = scope.get_value::<ScriptCommands>("commands") {
                all_commands.extend(cmds.commands);
            }
        }
    }

    // 4. Execute commands
    for cmd in all_commands {
        match cmd {
            ScriptCommand::Spawn { prefab, position } => {
                let e = spawn_prefab(world, &prefab, position);
                // Add physics body
                if let Some(physics) = world.get_resource_mut::<PhysicsWorld>() {
                    let body_id = physics.add_dynamic_box(
                        position,
                        Vec3::new(0.5, 0.5, 0.5),
                        1.0,
                        Layers::DEFAULT,
                    );
                    world.insert(e, CPhysicsBody { body_id });
                }
            }
            ScriptCommand::Despawn { entity } => {
                // SAFETY: `entity` is a u32 script-side ID. We immediately
                // validate via `is_alive()` before using the constructed Entity.
                let e = unsafe { Entity::from_raw(entity as u64) };
                // Validate entity is alive before attempting despawn
                if !world.is_alive(e) {
                    println!(
                        "[ScriptSystem] Cannot despawn entity {} (already dead or invalid)",
                        e
                    );
                    continue;
                }
                if world.despawn(e) {
                    println!("[ScriptSystem] Despawned entity {}", e);
                } else {
                    println!(
                        "[ScriptSystem] Failed to despawn entity {} (not found or dead)",
                        e
                    );
                }
            }
            ScriptCommand::SetPosition { entity, position } => {
                // SAFETY: `entity` is a u32 script-side ID. We immediately
                // validate via `is_alive()` before using the constructed Entity.
                let e = unsafe { Entity::from_raw(entity as u64) };
                // Validate entity is alive before attempting operations
                if !world.is_alive(e) {
                    println!(
                        "[ScriptSystem] Cannot set position for entity {} (dead or invalid)",
                        e
                    );
                    continue;
                }
                if let Some(cpos) = world.get_mut::<CPos>(e) {
                    cpos.pos = IVec2::new(position.x as i32, position.z as i32);
                    println!("[ScriptSystem] Set position of {} to {}", e, position);
                } else {
                    println!("[ScriptSystem] Entity {} has no CPos component", e);
                }

                // Update physics
                let body_id = world.get::<CPhysicsBody>(e).map(|b| b.body_id);
                if let Some(bid) = body_id {
                    if let Some(physics) = world.get_resource_mut::<PhysicsWorld>() {
                        physics.set_body_position(bid, position);
                    }
                }
            }
            ScriptCommand::ApplyDamage { entity, amount } => {
                // SAFETY: `entity` is a u32 script-side ID. We immediately
                // validate via `is_alive()` before using the constructed Entity.
                let e = unsafe { Entity::from_raw(entity as u64) };
                // Validate entity is alive before attempting operations
                if !world.is_alive(e) {
                    println!(
                        "[ScriptSystem] Cannot apply damage to entity {} (dead or invalid)",
                        e
                    );
                    continue;
                }
                if let Some(health) = world.get_mut::<CHealth>(e) {
                    health.hp = (health.hp as f32 - amount).max(0.0) as i32;
                    println!(
                        "[ScriptSystem] Applied {} damage to {} (Health: {})",
                        amount, e, health.hp
                    );
                } else {
                    println!("[ScriptSystem] Entity {} has no CHealth component", e);
                }
            }
            ScriptCommand::PlaySound { path } => {
                println!("[ScriptSystem] PlaySound requested: {}", path);

                // TODO(scripting-audio-integration): Integrate with astraweave-audio
                // The AudioEngine exists in astraweave-audio::engine::AudioEngine with methods:
                //   - play_sfx_file(path: &str) -> Result<()>
                //   - play_sfx_3d_file(emitter: EmitterId, path: &str, pos: Vec3) -> Result<()>
                //
                // Integration steps:
                // 1. Add astraweave-audio to Cargo.toml dependencies (already added)
                // 2. Create an ECS resource wrapper for AudioEngine
                // 3. Get resource from world: world.get_resource_mut::<AudioEngineResource>()
                // 4. Call audio.play_sfx_file(&path) or audio.play_sfx_3d_file(...)
                //
                // For now, logging only to avoid silent failures.
            }
            ScriptCommand::SpawnParticle { effect, position } => {
                println!(
                    "[ScriptSystem] SpawnParticle requested: '{}' at {}",
                    effect, position
                );

                // TODO(scripting-vfx-integration): Integrate with astraweave-render particle system
                // The GPU particle system exists in astraweave-render::gpu_particles::GpuParticleSystem
                // with EmitterParams for configuring particle emission.
                //
                // Integration steps:
                // 1. Add astraweave-render to Cargo.toml dependencies
                // 2. Create an ECS resource wrapper for GpuParticleSystem or effect registry
                // 3. Get resource from world: world.get_resource_mut::<ParticleSystemResource>()
                // 4. Queue particle emission with EmitterParams:
                //    - position: [position.x, position.y, position.z, 0.0]
                //    - Parse 'effect' string to determine particle type/config
                //    - Call system.update() with appropriate params
                //
                // Alternative: Use environment::WeatherParticles for simple effects
                //
                // For now, logging only to avoid silent failures.
            }
        }
    }
}

pub struct ScriptingPlugin;

impl Plugin for ScriptingPlugin {
    fn build(&self, app: &mut App) {
        app.world.insert_resource(ScriptEngineResource::default());
        app.world.insert_resource(ScriptCache::default());
        if app.world.get_resource::<ScriptReloadQueue>().is_none() {
            app.world.insert_resource(ScriptReloadQueue::default());
        }
        if app.world.get_resource::<Events>().is_none() {
            app.world.insert_resource(Events::default());
        }
        app.add_system(SystemStage::SIMULATION, script_system)
            .expect("registered schedule stage");
    }
}

pub struct CachedScript {
    pub ast: Arc<AST>,
    pub source_hash: String,
}

#[derive(Default)]
pub struct ScriptCache {
    pub scripts: BTreeMap<String, CachedScript>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_script_execution() {
        let mut app = App::new();
        app = app.add_plugin(ScriptingPlugin);

        let e = app.world.spawn();
        let script_source = r#"
            let x = 10;
            let y = 20;
            result = x + y;
        "#;

        let mut script = CScript::new("test.rhai", script_source);
        script
            .script_state
            .insert("result".to_string(), Dynamic::from(0_i64));
        app.world.insert(e, script);

        // Run one tick
        app.schedule.run(&mut app.world);

        // Check result
        let script = app.world.get::<CScript>(e).unwrap();
        let result = script.script_state.get("result").unwrap().as_int().unwrap();
        assert_eq!(result, 30);
    }

    #[test]
    fn default_engine_rejects_import_and_clock() {
        let engine = ScriptEngineResource::default();
        for source in [r#"import "std";"#, "eval(\"1\")", "timestamp()", "sleep(1)"] {
            let rejected = match engine.0.compile(source) {
                Err(_) => true,
                Ok(ast) => {
                    let mut scope = Scope::new();
                    engine.0.run_ast_with_scope(&mut scope, &ast).is_err()
                }
            };
            assert!(rejected, "{source} must fail on the gameplay sandbox");
        }
    }

    #[test]
    fn script_reload_applies_on_the_next_tick_only() {
        let mut app = App::new().add_plugin(ScriptingPlugin);
        let entity = app.world.spawn();
        let mut script = CScript::new("tick.rhai", "result = 1;");
        script
            .script_state
            .insert("result".to_string(), Dynamic::from(0_i64));
        app.world.insert(entity, script);
        app.schedule.run(&mut app.world);
        let read = |world: &World| {
            world
                .get::<CScript>(entity)
                .unwrap()
                .script_state
                .get("result")
                .unwrap()
                .as_int()
                .unwrap()
        };
        assert_eq!(read(&app.world), 1);
        app.world
            .get_resource_mut::<ScriptReloadQueue>()
            .unwrap()
            .stage("tick.rhai", "result = 2;");
        assert_eq!(read(&app.world), 1);
        app.schedule.run(&mut app.world);
        assert_eq!(read(&app.world), 2);
    }

    #[tokio::test]
    async fn test_script_loading_and_execution() {
        use std::path::PathBuf;

        let mut app = App::new();
        app = app.add_plugin(ScriptingPlugin);

        let asset_path = PathBuf::from("tests/assets/test_script.rhai");
        // Ensure we are running from crate root
        let asset = loader::ScriptLoader::load(&asset_path)
            .await
            .expect("Failed to load script");

        let e = app.world.spawn();
        let mut script = CScript::new(asset.path.to_string_lossy(), asset.source);
        script
            .script_state
            .insert("result".to_string(), Dynamic::from(0_i64));
        app.world.insert(e, script);

        // Run one tick
        app.schedule.run(&mut app.world);

        // Check result
        let script = app.world.get::<CScript>(e).unwrap();
        let result = script.script_state.get("result").unwrap().as_int().unwrap();
        assert_eq!(result, 42);
    }

    #[tokio::test]
    async fn test_api_integration() {
        use glam::IVec2;
        use std::path::PathBuf;

        let mut app = App::new();
        app = app.add_plugin(ScriptingPlugin);

        let asset_path = PathBuf::from("tests/assets/test_api.rhai");
        let asset = loader::ScriptLoader::load(&asset_path)
            .await
            .expect("Failed to load script");

        let e = app.world.spawn();
        let mut script = CScript::new(asset.path.to_string_lossy(), asset.source);
        // Initialize result_pos to verify it gets updated
        script
            .script_state
            .insert("result_pos".to_string(), Dynamic::from(IVec2::new(0, 0)));
        app.world.insert(e, script);

        // Run one tick
        app.schedule.run(&mut app.world);

        // Check result
        let script = app.world.get::<CScript>(e).unwrap();
        let result = script
            .script_state
            .get("result_pos")
            .unwrap()
            .clone()
            .cast::<IVec2>();
        assert_eq!(result, IVec2::new(11, 21));
    }

    #[tokio::test]
    async fn test_ecs_commands() {
        use std::path::PathBuf;

        let mut app = App::new();
        app = app.add_plugin(ScriptingPlugin);

        let asset_path = PathBuf::from("tests/assets/test_ecs.rhai");
        let asset = loader::ScriptLoader::load(&asset_path)
            .await
            .expect("Failed to load script");

        let e = app.world.spawn();
        let script = CScript::new(asset.path.to_string_lossy(), asset.source);
        app.world.insert(e, script);

        // Run one tick
        // We expect output to stdout: [ScriptSystem] Spawning ...
        app.schedule.run(&mut app.world);

        // Since we only print for now, we assume success if no panic and script runs.
        // In a real test, we would mock the command execution or check side effects.
    }

    #[test]
    fn test_set_position_command() {
        use astraweave_core::{CPos, IVec2};

        let mut app = App::new();
        app = app.add_plugin(ScriptingPlugin);

        // Spawn entity with initial position
        let e = app.world.spawn();
        app.world.insert(
            e,
            CPos {
                pos: IVec2::new(0, 0),
            },
        );

        // Create script to move it
        // Note: We need to pass the entity ID to the script.
        // The script_system injects 'entity_id' automatically.
        let script_source = r#"
            let target = vec3(10.0, 0.0, 20.0);
            commands.set_position(entity_id, target);
        "#;

        let script = CScript::new("test_move.rhai", script_source);
        app.world.insert(e, script);

        // Run one tick
        app.schedule.run(&mut app.world);

        // Check if position was updated
        // Note: IVec2 uses (x, y), we map Vec3(x, y, z) -> IVec2(x, z)
        let pos = app.world.get::<CPos>(e).unwrap().pos;
        assert_eq!(pos.x, 10);
        assert_eq!(pos.y, 20);
    }

    #[test]
    fn test_spawn_prefab() {
        use astraweave_core::{CHealth, CPos};

        let mut app = App::new();
        app = app.add_plugin(ScriptingPlugin);

        let e = app.world.spawn();
        let script_source = r#"
            let pos = vec3(5.0, 0.0, 5.0);
            commands.spawn_prefab("enemy_grunt", pos);
        "#;

        let script = CScript::new("test_spawn.rhai", script_source);
        app.world.insert(e, script);

        // Run one tick
        app.schedule.run(&mut app.world);

        // Check if entity was spawned
        // We iterate all entities and check for one with CHealth and CPos at (5, 5)
        let mut found = false;
        let entities = app.world.entities_with::<CHealth>();
        for entity in entities {
            if let Some(pos) = app.world.get::<CPos>(entity) {
                if pos.pos.x == 5 && pos.pos.y == 5 {
                    found = true;
                    break;
                }
            }
        }
        assert!(found, "Spawned enemy_grunt not found");
    }

    #[test]
    fn test_collision_event() {
        use astraweave_physics::{Layers, PhysicsWorld};
        use glam::Vec3;

        let mut app = App::new();
        app = app.add_plugin(ScriptingPlugin);

        // Add PhysicsWorld resource
        let physics = PhysicsWorld::new(Vec3::new(0.0, -9.8, 0.0));
        app.world.insert_resource(physics);

        // Spawn entity 1 (Listener)
        let e1 = app.world.spawn();
        let script_source = r#"
            fn on_collision(other) {
                collided_with = other;
            }
        "#;
        let mut script = CScript::new("test_collision.rhai", script_source);
        script
            .script_state
            .insert("collided_with".to_string(), Dynamic::from(0_i64));
        app.world.insert(e1, script);

        // Add physics body to e1
        // We need to drop the resource borrow before inserting components?
        // No, app.world.get_resource_mut returns a RefMut, which borrows world.
        // We can't insert components while holding it.

        let body_id1 = {
            let physics = app.world.get_resource_mut::<PhysicsWorld>().unwrap();
            physics.add_dynamic_box(
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.5, 0.5, 0.5),
                1.0,
                Layers::DEFAULT,
            )
        };
        app.world.insert(e1, CPhysicsBody { body_id: body_id1 });

        // Spawn entity 2 (Collider)
        let e2 = app.world.spawn();
        let body_id2 = {
            let physics = app.world.get_resource_mut::<PhysicsWorld>().unwrap();
            physics.add_dynamic_box(
                Vec3::new(0.8, 0.0, 0.0),
                Vec3::new(0.5, 0.5, 0.5),
                1.0,
                Layers::DEFAULT,
            )
        };
        app.world.insert(e2, CPhysicsBody { body_id: body_id2 });

        // Step physics to generate collision
        {
            let physics = app.world.get_resource_mut::<PhysicsWorld>().unwrap();
            for _ in 0..10 {
                physics.step();
            }
        }

        // Run script system to process events
        app.schedule.run(&mut app.world);

        // Check result
        let script = app.world.get::<CScript>(e1).unwrap();
        let collided_with = script
            .script_state
            .get("collided_with")
            .unwrap()
            .as_int()
            .unwrap();

        // e2 entity ID
        let e2_id = e2.to_raw() as i64;
        assert_eq!(collided_with, e2_id);
    }

    #[test]
    fn test_damage_event_and_command() {
        use crate::events::ScriptEvent;
        use astraweave_core::CHealth;
        use astraweave_ecs::Events;

        let mut app = App::new();
        app = app.add_plugin(ScriptingPlugin);

        // Spawn victim
        let victim = app.world.spawn();
        app.world.insert(victim, CHealth { hp: 100 });

        // Script that listens for damage and heals if low
        // Note: We don't have 'heal' command yet, but we can use 'apply_damage' with negative value or just check state
        let script_source = r#"
            fn on_damage(amount, source) {
                // print("Took damage: " + amount);
                last_damage = amount;
                last_source = source;
                
                // If we took significant damage, counter attack!
                if amount > 50.0 {
                    commands.apply_damage(source, 10.0);
                }
            }
        "#;
        let mut script = CScript::new("victim.rhai", script_source);
        script
            .script_state
            .insert("last_damage".to_string(), Dynamic::from(0.0));
        script
            .script_state
            .insert("last_source".to_string(), Dynamic::from(0_i64));
        app.world.insert(victim, script);

        // Spawn attacker
        let attacker = app.world.spawn();
        app.world.insert(attacker, CHealth { hp: 100 });

        // Trigger OnDamage event
        if let Some(events) = app.world.get_resource_mut::<Events>() {
            events.send(ScriptEvent::OnDamage {
                entity: victim,
                damage: 60.0,
                source: attacker,
            });
        }

        // Run system
        app.schedule.run(&mut app.world);

        // Check victim state
        let script = app.world.get::<CScript>(victim).unwrap();
        let last_damage = script
            .script_state
            .get("last_damage")
            .unwrap()
            .as_float()
            .unwrap();
        let last_source = script
            .script_state
            .get("last_source")
            .unwrap()
            .as_int()
            .unwrap();

        assert_eq!(last_damage, 60.0);
        assert_eq!(last_source, attacker.to_raw() as i64);

        // Check attacker health (should be damaged by counter attack)
        // Note: The command is executed at the end of the frame.
        // Wait, apply_damage command modifies CHealth directly in the same frame (step 4 of script_system).
        let attacker_health = app.world.get::<CHealth>(attacker).unwrap().hp;
        assert_eq!(attacker_health, 90); // 100 - 10
    }

    #[test]
    fn test_raycast_api() {
        use astraweave_physics::{Layers, PhysicsWorld};
        use glam::Vec3;

        let mut app = App::new();
        app = app.add_plugin(ScriptingPlugin);

        // Add PhysicsWorld resource
        let physics = PhysicsWorld::new(Vec3::new(0.0, -9.8, 0.0));
        app.world.insert_resource(physics);

        // Spawn an entity to hit
        let target = app.world.spawn();
        let body_id = {
            let physics = app.world.get_resource_mut::<PhysicsWorld>().unwrap();
            // Box at (5, 0, 0) size 1
            physics.add_dynamic_box(
                Vec3::new(5.0, 0.0, 0.0),
                Vec3::new(0.5, 0.5, 0.5),
                1.0,
                Layers::DEFAULT,
            )
        };
        app.world.insert(target, CPhysicsBody { body_id });

        // Update physics to build acceleration structure
        if let Some(physics) = app.world.get_resource_mut::<PhysicsWorld>() {
            physics.step();
        }

        // Spawn script entity
        let e = app.world.spawn();
        let script_source = r#"
            let start = vec3(0.0, 0.0, 0.0);
            let dir = vec3(1.0, 0.0, 0.0);
            let hit = physics.raycast(start, dir, 100.0);
            
            if hit != () {
                hit_entity = hit.entity_id;
                hit_dist = hit.distance;
            } else {
                hit_entity = -1;
            }
        "#;

        let mut script = CScript::new("test_raycast.rhai", script_source);
        script
            .script_state
            .insert("hit_entity".to_string(), Dynamic::from(0_i64));
        script
            .script_state
            .insert("hit_dist".to_string(), Dynamic::from(0.0_f32));
        app.world.insert(e, script);

        // Run one tick
        app.schedule.run(&mut app.world);

        // Check result
        let script = app.world.get::<CScript>(e).unwrap();
        let hit_entity = script
            .script_state
            .get("hit_entity")
            .unwrap()
            .as_int()
            .unwrap();
        let hit_dist = script
            .script_state
            .get("hit_dist")
            .unwrap()
            .as_float()
            .unwrap();

        assert_eq!(hit_entity, target.to_raw() as i64);
        // Distance should be 4.5 (5.0 center - 0.5 half extent)
        assert!((hit_dist - 4.5_f32).abs() < 0.001);
    }

    #[test]
    fn test_navmesh_api() {
        use astraweave_nav::{NavMesh, Triangle};
        use glam::Vec3;

        let mut app = App::new();
        app = app.add_plugin(ScriptingPlugin);

        // Create a simple NavMesh
        let tris = vec![Triangle {
            a: Vec3::new(0.0, 0.0, 0.0),
            b: Vec3::new(0.0, 0.0, 10.0),
            c: Vec3::new(10.0, 0.0, 0.0),
        }];
        let nav = NavMesh::bake(&tris, 0.5, 60.0);
        app.world.insert_resource(nav);

        // Spawn script entity
        let e = app.world.spawn();
        let script_source = r#"
            let start = vec3(1.0, 0.0, 1.0);
            let goal = vec3(2.0, 0.0, 2.0);
            let path = nav.find_path(start, goal);
            
            path_len = path.len();
            if path_len > 0 {
                first_pt = path[0];
            }
        "#;

        let mut script = CScript::new("test_nav.rhai", script_source);
        script
            .script_state
            .insert("path_len".to_string(), Dynamic::from(0_i64));
        script
            .script_state
            .insert("first_pt".to_string(), Dynamic::from(Vec3::ZERO));
        app.world.insert(e, script);

        // Run one tick
        app.schedule.run(&mut app.world);

        // Check result
        let script = app.world.get::<CScript>(e).unwrap();
        let path_len = script
            .script_state
            .get("path_len")
            .unwrap()
            .as_int()
            .unwrap();
        let first_pt = script
            .script_state
            .get("first_pt")
            .unwrap()
            .clone()
            .cast::<Vec3>();

        assert!(path_len >= 2);
        assert!((first_pt - Vec3::new(1.0, 0.0, 1.0)).length() < 0.1);
    }

    #[test]
    fn test_sandboxing() {
        let mut app = App::new();
        app = app.add_plugin(ScriptingPlugin);

        // Test 1: Forbidden types (File)
        let e = app.world.spawn();
        let script_source = r#"
            // This should fail because File is not registered
            let f = File.open("test.txt");
        "#;

        let script = CScript::new("test_sandbox_file.rhai", script_source);
        app.world.insert(e, script);

        // Test 2: Infinite loop (Max operations)
        let e_loop = app.world.spawn();
        let script_source_loop = r#"
            let x = 0;
            loop {
                x = x + 1;
            }
        "#;
        let script_loop = CScript::new("test_sandbox_loop.rhai", script_source_loop);
        app.world.insert(e_loop, script_loop);

        // Run one tick
        // We expect runtime errors to be printed to stderr, but no crash.
        app.schedule.run(&mut app.world);
    }

    #[test]
    fn test_stale_entity_validation() {
        use astraweave_core::CHealth;

        let mut app = App::new();
        app = app.add_plugin(ScriptingPlugin);

        // Spawn a victim entity and record its ID
        let victim = app.world.spawn();
        app.world.insert(victim, CHealth { hp: 100 });
        let victim_id = victim.to_raw() as i64;

        // Despawn the victim
        app.world.despawn(victim);

        // Create a script that tries to damage the now-dead entity
        let e = app.world.spawn();
        let script_source = format!(
            r#"
            // Try to damage entity {} which is already dead
            commands.apply_damage({}, 50.0);
            commands.set_position({}, vec3(10.0, 0.0, 10.0));
        "#,
            victim_id, victim_id, victim_id
        );

        let script = CScript::new("test_stale.rhai", &script_source);
        app.world.insert(e, script);

        // Run one tick - should not crash
        // The system should detect the stale entity and skip operations
        app.schedule.run(&mut app.world);

        // Test passed if we get here without panic
    }
}
