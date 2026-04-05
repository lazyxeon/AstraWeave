use astraweave_core::{CHealth, CPos, IVec2};
use astraweave_ecs::App;
use astraweave_nav::{NavMesh, Triangle};
use astraweave_physics::PhysicsWorld;
use astraweave_scripting::{CScript, ScriptingPlugin};
use glam::Vec3;
use std::path::PathBuf;

#[tokio::main]
async fn main() {
    let mut app = App::new();
    app = app.add_plugin(ScriptingPlugin);

    // Setup Physics
    let physics = PhysicsWorld::new(Vec3::new(0.0, -9.8, 0.0));
    app.world.insert_resource(physics);

    // Setup NavMesh (Simple ground plane)
    let tris = vec![
        Triangle {
            a: Vec3::new(-50.0, 0.0, -50.0),
            b: Vec3::new(-50.0, 0.0, 50.0),
            c: Vec3::new(50.0, 0.0, -50.0),
        },
        Triangle {
            a: Vec3::new(50.0, 0.0, -50.0),
            b: Vec3::new(-50.0, 0.0, 50.0),
            c: Vec3::new(50.0, 0.0, 50.0),
        },
    ];
    let nav = NavMesh::bake(&tris, 0.5, 60.0);
    app.world.insert_resource(nav);

    // Spawn AI Entity
    let ai_entity = app.world.spawn();
    app.world.insert(
        ai_entity,
        CPos {
            pos: IVec2::new(0, 0),
        },
    );
    app.world.insert(ai_entity, CHealth { hp: 100 });

    // Load Script
    // Note: Path is relative to workspace root when running via cargo run
    let script_path = PathBuf::from("examples/scripting_advanced_demo/assets/advanced_ai.rhai");

    // Fallback if running from crate dir
    let script_path = if script_path.exists() {
        script_path
    } else {
        PathBuf::from("assets/advanced_ai.rhai")
    };

    let source = std::fs::read_to_string(&script_path).expect("Failed to read script");

    let mut script = CScript::new(script_path.to_string_lossy(), source);

    // Initialize script state
    use rhai::Dynamic;
    script
        .script_state
        .insert("state".to_string(), Dynamic::from("idle".to_string()));
    script
        .script_state
        .insert("target_id".to_string(), Dynamic::from(-1_i64));
    script
        .script_state
        .insert("path".to_string(), Dynamic::from(Vec::<Dynamic>::new()));
    script
        .script_state
        .insert("current_path_idx".to_string(), Dynamic::from(0_i64));

    let p1 = Vec3::new(0.0, 0.0, 0.0);
    let p2 = Vec3::new(10.0, 0.0, 0.0);
    let p3 = Vec3::new(10.0, 0.0, 10.0);
    let p4 = Vec3::new(0.0, 0.0, 10.0);
    let points = vec![
        Dynamic::from(p1),
        Dynamic::from(p2),
        Dynamic::from(p3),
        Dynamic::from(p4),
    ];
    script
        .script_state
        .insert("patrol_points".to_string(), Dynamic::from(points));
    script
        .script_state
        .insert("patrol_idx".to_string(), Dynamic::from(0_i64));
    script
        .script_state
        .insert("float_pos".to_string(), Dynamic::from(Vec3::ZERO));

    app.world.insert(ai_entity, script);

    println!("Starting Advanced AI Demo...");

    // Run loop
    for i in 0..100 {
        app.schedule.run(&mut app.world);

        // Print AI position
        if let Some(pos) = app.world.get::<CPos>(ai_entity) {
            println!("Frame {}: AI Pos: {:?}", i, pos.pos);
        }

        // Simulate time passing
        // std::thread::sleep(std::time::Duration::from_millis(16));
    }
}
