// =============================================================================
// AstraWeave Scripting — Targeted Mutation-Killing Tests
// =============================================================================
// Targets mutations missed by the initial scan:
//   lib.rs   L394: || → && in disabled script check
//   lib.rs   L468: delete ! in despawn is_alive check
//   api.rs   L171-173: Vec3 +,-,* operator registration
//   api.rs   L187: IVec2 - operator registration
//   loader.rs L34: compute_hash returns correct SHA256
// =============================================================================

use astraweave_core::CHealth;
use astraweave_ecs::{App, Events};
use astraweave_scripting::events::ScriptEvent;
use astraweave_scripting::{CScript, ScriptingPlugin};
use rhai::Dynamic;

// ===========================================================================
// 1. L394: `|| → &&` in `if !script.enabled || script.cached_ast.is_none()`
//    With &&, a DISABLED script that has a cached AST would NOT be skipped
//    and would incorrectly execute.
// ===========================================================================

#[test]
fn disabled_script_must_not_execute() {
    let mut app = App::new();
    app = app.add_plugin(ScriptingPlugin);

    let e = app.world.spawn();
    let source = r#"
        executed = true;
    "#;

    let mut script = CScript::new("disabled_test.rhai", source);
    script
        .script_state
        .insert("executed".to_string(), Dynamic::from(false));
    // CRITICALLY: First, compile and cache AST by running once while enabled
    app.world.insert(e, script);
    app.schedule.run(&mut app.world);

    // Verify it ran while enabled
    {
        let s = app.world.get::<CScript>(e).unwrap();
        assert!(s.cached_ast.is_some(), "AST must be cached after first run");
        let val = s.script_state.get("executed").unwrap().as_bool().unwrap();
        assert!(val, "Script should have run while enabled");
    }

    // Now disable the script and reset the flag
    {
        let s = app.world.get_mut::<CScript>(e).unwrap();
        s.enabled = false;
        s.script_state
            .insert("executed".to_string(), Dynamic::from(false));
    }

    // Run again — disabled script with cached AST should NOT execute
    app.schedule.run(&mut app.world);

    let s = app.world.get::<CScript>(e).unwrap();
    let executed = s.script_state.get("executed").unwrap().as_bool().unwrap();
    assert!(
        !executed,
        "Disabled script must NOT execute even with cached AST"
    );
}

// ===========================================================================
// 2. L468: `delete !` in `if !world.is_alive(e)` for Despawn command
//    With mutation, alive entities would skip despawn (bad),
//    and dead entities would attempt despawn (harmless but wrong).
// ===========================================================================

#[test]
fn despawn_command_removes_alive_entity() {
    let mut app = App::new();
    app = app.add_plugin(ScriptingPlugin);

    // Spawn target entity
    let target = app.world.spawn();
    app.world.insert(target, CHealth { hp: 100 });
    let target_id = target.to_raw() as i64;

    assert!(
        app.world.is_alive(target),
        "Target must be alive before despawn"
    );

    // Script that despawns the target
    let e = app.world.spawn();
    let source = format!(r#"commands.despawn({});"#, target_id);
    let script = CScript::new("despawn_test.rhai", &source);
    app.world.insert(e, script);

    app.schedule.run(&mut app.world);

    // Target should be despawned (not alive)
    assert!(
        !app.world.is_alive(target),
        "Target must be despawned after script command"
    );
}

// ===========================================================================
// 3. api.rs L171: Vec3 `+` operator — catches `+ → -` and `+ → *`
// ===========================================================================

#[test]
fn vec3_addition_in_script() {
    let mut app = App::new();
    app = app.add_plugin(ScriptingPlugin);

    let e = app.world.spawn();
    let source = r#"
        let a = vec3(1.0, 2.0, 3.0);
        let b = vec3(10.0, 20.0, 30.0);
        let c = a + b;
        sum_x = c.x;
        sum_y = c.y;
        sum_z = c.z;
    "#;

    let mut script = CScript::new("vec3_add.rhai", source);
    script
        .script_state
        .insert("sum_x".to_string(), Dynamic::from(0.0_f64));
    script
        .script_state
        .insert("sum_y".to_string(), Dynamic::from(0.0_f64));
    script
        .script_state
        .insert("sum_z".to_string(), Dynamic::from(0.0_f64));
    app.world.insert(e, script);

    app.schedule.run(&mut app.world);

    let s = app.world.get::<CScript>(e).unwrap();
    let x = s.script_state.get("sum_x").unwrap().as_float().unwrap();
    let y = s.script_state.get("sum_y").unwrap().as_float().unwrap();
    let z = s.script_state.get("sum_z").unwrap().as_float().unwrap();
    assert!((x - 11.0).abs() < 0.01, "Vec3 + x: expected 11, got {}", x);
    assert!((y - 22.0).abs() < 0.01, "Vec3 + y: expected 22, got {}", y);
    assert!((z - 33.0).abs() < 0.01, "Vec3 + z: expected 33, got {}", z);
}

// ===========================================================================
// 4. api.rs L172: Vec3 `-` operator — catches `- → +` and `- → /`
// ===========================================================================

#[test]
fn vec3_subtraction_in_script() {
    let mut app = App::new();
    app = app.add_plugin(ScriptingPlugin);

    let e = app.world.spawn();
    let source = r#"
        let a = vec3(10.0, 20.0, 30.0);
        let b = vec3(3.0, 5.0, 7.0);
        let c = a - b;
        diff_x = c.x;
        diff_y = c.y;
        diff_z = c.z;
    "#;

    let mut script = CScript::new("vec3_sub.rhai", source);
    script
        .script_state
        .insert("diff_x".to_string(), Dynamic::from(0.0_f64));
    script
        .script_state
        .insert("diff_y".to_string(), Dynamic::from(0.0_f64));
    script
        .script_state
        .insert("diff_z".to_string(), Dynamic::from(0.0_f64));
    app.world.insert(e, script);

    app.schedule.run(&mut app.world);

    let s = app.world.get::<CScript>(e).unwrap();
    let x = s.script_state.get("diff_x").unwrap().as_float().unwrap();
    let y = s.script_state.get("diff_y").unwrap().as_float().unwrap();
    let z = s.script_state.get("diff_z").unwrap().as_float().unwrap();
    assert!((x - 7.0).abs() < 0.01, "Vec3 - x: expected 7, got {}", x);
    assert!((y - 15.0).abs() < 0.01, "Vec3 - y: expected 15, got {}", y);
    assert!((z - 23.0).abs() < 0.01, "Vec3 - z: expected 23, got {}", z);
}

// ===========================================================================
// 5. api.rs L173: Vec3 `*` operator — catches `* → +` and `* → /`
// ===========================================================================

#[test]
fn vec3_scalar_multiply_in_script() {
    let mut app = App::new();
    app = app.add_plugin(ScriptingPlugin);

    let e = app.world.spawn();
    let source = r#"
        let a = vec3(2.0, 3.0, 4.0);
        let c = a * 5.0;
        mul_x = c.x;
        mul_y = c.y;
        mul_z = c.z;
    "#;

    let mut script = CScript::new("vec3_mul.rhai", source);
    script
        .script_state
        .insert("mul_x".to_string(), Dynamic::from(0.0_f64));
    script
        .script_state
        .insert("mul_y".to_string(), Dynamic::from(0.0_f64));
    script
        .script_state
        .insert("mul_z".to_string(), Dynamic::from(0.0_f64));
    app.world.insert(e, script);

    app.schedule.run(&mut app.world);

    let s = app.world.get::<CScript>(e).unwrap();
    let x = s.script_state.get("mul_x").unwrap().as_float().unwrap();
    let y = s.script_state.get("mul_y").unwrap().as_float().unwrap();
    let z = s.script_state.get("mul_z").unwrap().as_float().unwrap();
    assert!((x - 10.0).abs() < 0.01, "Vec3 * x: expected 10, got {}", x);
    assert!((y - 15.0).abs() < 0.01, "Vec3 * y: expected 15, got {}", y);
    assert!((z - 20.0).abs() < 0.01, "Vec3 * z: expected 20, got {}", z);
}

// ===========================================================================
// 6. api.rs L187: IVec2 `-` operator — catches `- → +` and `- → /`
// ===========================================================================

#[test]
fn ivec2_subtraction_in_script() {
    let mut app = App::new();
    app = app.add_plugin(ScriptingPlugin);

    let e = app.world.spawn();
    let source = r#"
        let a = ivec2(10, 20);
        let b = ivec2(3, 7);
        let c = a - b;
        diff_x = c.x;
        diff_y = c.y;
    "#;

    let mut script = CScript::new("ivec2_sub.rhai", source);
    script
        .script_state
        .insert("diff_x".to_string(), Dynamic::from(0_i64));
    script
        .script_state
        .insert("diff_y".to_string(), Dynamic::from(0_i64));
    app.world.insert(e, script);

    app.schedule.run(&mut app.world);

    let s = app.world.get::<CScript>(e).unwrap();
    let x = s.script_state.get("diff_x").unwrap().as_int().unwrap();
    let y = s.script_state.get("diff_y").unwrap().as_int().unwrap();
    assert_eq!(x, 7, "IVec2 - x: expected 7, got {}", x);
    assert_eq!(y, 13, "IVec2 - y: expected 13, got {}", y);
}

// ===========================================================================
// 7. loader.rs L34: compute_hash correctness
//    Catches: replace compute_hash -> String with String::new() or "xyzzy"
// ===========================================================================

#[tokio::test]
async fn script_loader_produces_correct_hash() {
    use astraweave_scripting::loader::ScriptLoader;
    use std::path::PathBuf;

    let path = PathBuf::from("tests/assets/test_script.rhai");
    let asset = ScriptLoader::load(&path)
        .await
        .expect("Failed to load test script");

    // Hash should be non-empty
    assert!(!asset.hash.is_empty(), "Script hash must not be empty");

    // Hash should be hex-encoded SHA256 (64 hex chars)
    assert_eq!(
        asset.hash.len(),
        64,
        "SHA256 hash should be 64 hex chars, got {}",
        asset.hash.len()
    );

    // Hash should be deterministic — loading same file twice gives same hash
    let asset2 = ScriptLoader::load(&path)
        .await
        .expect("Failed to load second time");
    assert_eq!(
        asset.hash, asset2.hash,
        "Same file should produce same hash"
    );

    // Loading a different script should produce a different hash
    let path2 = PathBuf::from("tests/assets/test_api.rhai");
    if let Ok(asset3) = ScriptLoader::load(&path2).await {
        assert_ne!(
            asset.hash, asset3.hash,
            "Different scripts should have different hashes"
        );
    }
}

// ===========================================================================
// 8. L394: `|| → &&` in event callback section
//    `if !script.enabled || script.cached_ast.is_none()` in event processing
//    With &&, a DISABLED script with cached AST would incorrectly execute
//    event callbacks (on_damage, on_collision, etc.)
// ===========================================================================

#[test]
fn disabled_script_event_callback_must_not_execute() {
    let mut app = App::new();
    app = app.add_plugin(ScriptingPlugin);

    // Spawn script entity with on_damage callback
    let victim = app.world.spawn();
    app.world.insert(victim, CHealth { hp: 100 });

    let source = r#"
        fn on_damage(amount, source) {
            damage_received = amount;
        }
    "#;

    let mut script = CScript::new("disabled_event.rhai", source);
    script
        .script_state
        .insert("damage_received".to_string(), Dynamic::from(0.0_f32));
    app.world.insert(victim, script);

    // Run once to compile and cache AST
    app.schedule.run(&mut app.world);

    // Verify AST is cached
    {
        let s = app.world.get::<CScript>(victim).unwrap();
        assert!(s.cached_ast.is_some(), "AST must be cached");
    }

    // Disable the script and reset damage_received
    {
        let s = app.world.get_mut::<CScript>(victim).unwrap();
        s.enabled = false;
        s.script_state
            .insert("damage_received".to_string(), Dynamic::from(0.0_f32));
    }

    // Spawn attacker
    let attacker = app.world.spawn();
    app.world.insert(attacker, CHealth { hp: 100 });

    // Trigger OnDamage event on the disabled script
    if let Some(events) = app.world.get_resource_mut::<Events>() {
        events.send(ScriptEvent::OnDamage {
            entity: victim,
            damage: 25.0,
            source: attacker,
        });
    }

    // Run the system — the event should NOT trigger the callback
    app.schedule.run(&mut app.world);

    let s = app.world.get::<CScript>(victim).unwrap();
    let dmg = s
        .script_state
        .get("damage_received")
        .unwrap()
        .as_float()
        .unwrap();
    assert!(
        (dmg - 0.0).abs() < 0.01,
        "Disabled script event callback must NOT execute. damage_received = {}",
        dmg
    );
}
