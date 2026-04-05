use astraweave_core::{CHealth, CPos, IVec2};
use astraweave_ecs::App;
use astraweave_physics::{Layers, PhysicsWorld};
use astraweave_scripting::{CPhysicsBody, CScript, ScriptingPlugin};
use glam::Vec3;

fn main() {
    let mut app = App::new();
    app = app.add_plugin(ScriptingPlugin);

    // Setup Physics
    let mut physics = PhysicsWorld::new(Vec3::new(0.0, -9.8, 0.0));

    // Ground
    physics.create_ground_plane(Vec3::new(50.0, 0.5, 50.0), 0.5);

    app.world.insert_resource(physics);

    // Spawn Player
    let player_body_id = {
        let e = app.world.spawn();
        app.world.insert(
            e,
            CPos {
                pos: IVec2::new(0, 0),
            },
        );
        app.world.insert(e, CHealth { hp: 100 });

        let script_source = r#"
            // Player Script
            
            fn on_trigger_enter(other) {
                print("Player entered trigger! Entity: " + other);
                commands.apply_damage(entity_id, 5);
            }
            
            // Raycast forward using injected my_pos
            // Offset ray start to avoid hitting self (radius 0.5)
            let dir = vec3(1.0, 0.0, 0.0);
            let start = my_pos + dir * 0.6;
            
            // print("Ray start: " + start);
            let hit = physics.raycast(start, dir, 10.0);
            if hit.hit {
                // print("Hit entity: " + hit.entity);
                if hit.entity == 2 {
                    print("Player sees enemy! Damaging...");
                    commands.apply_damage(hit.entity, 10);
                }
            }
        "#;
        let script = CScript::new("player.rhai", script_source);
        app.world.insert(e, script);

        // Physics Body
        let body_id = {
            let physics = app.world.get_resource_mut::<PhysicsWorld>().unwrap();
            physics.add_dynamic_box(
                Vec3::new(0.0, 1.0, 0.0),
                Vec3::new(0.5, 1.0, 0.5),
                1.0,
                Layers::DEFAULT,
            )
        };
        app.world.insert(e, CPhysicsBody { body_id });
        println!("Spawned Player (Entity: {}, Body: {})", e, body_id);
        body_id
    };

    // Spawn Trigger
    {
        let e = app.world.spawn();
        app.world.insert(
            e,
            CPos {
                pos: IVec2::new(5, 0),
            },
        );

        let script_source = r#"
            fn on_trigger_enter(other) {
                print("Trigger activated by entity: " + other);
            }
        "#;
        let script = CScript::new("trigger.rhai", script_source);
        app.world.insert(e, script);

        let body_id = {
            let physics = app.world.get_resource_mut::<PhysicsWorld>().unwrap();
            physics.add_sensor(
                Vec3::new(5.0, 1.0, 0.0),
                Vec3::new(1.0, 1.0, 1.0),
                Layers::DEFAULT,
            )
        };
        app.world.insert(e, CPhysicsBody { body_id });
        println!("Spawned Trigger (Entity: {}, Body: {})", e, body_id);
    }

    // Spawn Enemy
    {
        let e = app.world.spawn();
        app.world.insert(
            e,
            CPos {
                pos: IVec2::new(8, 0),
            },
        );
        app.world.insert(e, CHealth { hp: 50 });

        let body_id = {
            let physics = app.world.get_resource_mut::<PhysicsWorld>().unwrap();
            physics.add_dynamic_box(
                Vec3::new(8.0, 1.0, 0.0),
                Vec3::new(0.5, 0.5, 0.5),
                1.0,
                Layers::DEFAULT,
            )
        };
        app.world.insert(e, CPhysicsBody { body_id });
        println!("Spawned Enemy (Entity: {}, Body: {})", e, body_id);
    }

    println!("Starting simulation loop (100 frames)...");
    for i in 0..100 {
        // Move player towards trigger (at x=5)
        if let Some(physics) = app.world.get_resource_mut::<PhysicsWorld>() {
            let pos = Vec3::new(i as f32 * 0.1, 1.0, 0.0);
            physics.set_body_position(player_body_id, pos);
            physics.step();
        }

        // Run systems
        app.schedule.run(&mut app.world);

        // Print player pos
        if i % 10 == 0 {
            if let Some(physics) = app.world.get_resource::<PhysicsWorld>() {
                if let Some(transform) = physics.body_transform(player_body_id) {
                    println!("Frame {}: Player Pos: {:?}", i, transform.w_axis);
                }
            }
        }

        // std::thread::sleep(Duration::from_millis(16));
    }
    println!("Simulation complete.");
}
