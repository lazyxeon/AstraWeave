pub use astraweave_security::sandbox::MOD_COMMAND_API_VERSION as SCRIPT_COMMAND_API_VERSION;

use astraweave_nav::NavMesh;
use astraweave_physics::PhysicsWorld;
use glam::{IVec2, Quat, Vec3};
use rapier3d::prelude::{QueryFilter, Ray};
use rhai::{Dynamic, Engine, ImmutableString};

#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum ScriptCommand {
    Spawn { prefab: String, position: Vec3 },
    Despawn { entity: i64 },
    SetPosition { entity: i64, position: Vec3 },
    ApplyDamage { entity: i64, amount: f32 },
    PlaySound { path: String },
    SpawnParticle { effect: String, position: Vec3 },
}

#[derive(Clone, Debug, Default)]
pub struct ScriptCommands {
    pub commands: Vec<ScriptCommand>,
}

impl ScriptCommands {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn spawn(&mut self, prefab: &str, position: Vec3) {
        self.commands.push(ScriptCommand::Spawn {
            prefab: prefab.to_string(),
            position,
        });
    }

    pub fn despawn(&mut self, entity: i64) {
        self.commands.push(ScriptCommand::Despawn { entity });
    }

    pub fn set_position(&mut self, entity: i64, position: Vec3) {
        self.commands
            .push(ScriptCommand::SetPosition { entity, position });
    }

    pub fn apply_damage(&mut self, entity: i64, amount: f32) {
        self.commands
            .push(ScriptCommand::ApplyDamage { entity, amount });
    }

    pub fn play_sound(&mut self, path: &str) {
        self.commands.push(ScriptCommand::PlaySound {
            path: path.to_string(),
        });
    }

    pub fn spawn_particle(&mut self, effect: &str, position: Vec3) {
        self.commands.push(ScriptCommand::SpawnParticle {
            effect: effect.to_string(),
            position,
        });
    }
}

use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone)]
pub struct PhysicsProxy {
    pub ptr: *const PhysicsWorld,
    pub body_map: Arc<HashMap<u64, u64>>,
}

// Safety: We must ensure the pointer is valid when used.
unsafe impl Send for PhysicsProxy {}
unsafe impl Sync for PhysicsProxy {}

#[derive(Clone, Debug)]
pub struct RaycastHit {
    pub entity_id: i64,
    pub position: Vec3,
    pub normal: Vec3,
    pub distance: f32,
}

impl PhysicsProxy {
    pub fn raycast(&mut self, origin: Vec3, dir: Vec3, max_dist: f32) -> Dynamic {
        if self.ptr.is_null() {
            return Dynamic::UNIT;
        }

        // SAFETY: Null check above guarantees the pointer is non-null.
        // The pointer was created from a valid `&PhysicsWorld` reference in
        // `ScriptingPlugin::register` and the borrow is scoped to this call.
        let physics = unsafe { &*self.ptr };
        let ray = Ray::new(
            rapier3d::na::Point3::new(origin.x, origin.y, origin.z),
            rapier3d::na::Vector3::new(dir.x, dir.y, dir.z),
        );

        let filter = QueryFilter::default();

        if let Some((handle, hit)) = physics.query_pipeline.cast_ray_and_get_normal(
            &physics.bodies,
            &physics.colliders,
            &ray,
            max_dist,
            true,
            filter,
        ) {
            let collider = physics.colliders.get(handle);
            let body_handle = collider.and_then(|c| c.parent());

            let body_id = if let Some(bh) = body_handle {
                physics.id_of(bh).unwrap_or(0)
            } else {
                0
            };

            let entity_id = self.body_map.get(&body_id).copied().unwrap_or(0) as i64;

            // Calculate hit point manually
            let point = ray.point_at(hit.time_of_impact);

            let hit_result = RaycastHit {
                entity_id,
                position: Vec3::new(point.x, point.y, point.z),
                normal: Vec3::new(hit.normal.x, hit.normal.y, hit.normal.z),
                distance: hit.time_of_impact,
            };

            return rhai::Dynamic::from(hit_result);
        }

        Dynamic::UNIT
    }
}

#[derive(Clone)]
pub struct NavMeshProxy {
    pub ptr: *const NavMesh,
}

// Safety: We must ensure the pointer is valid when used.
unsafe impl Send for NavMeshProxy {}
unsafe impl Sync for NavMeshProxy {}

impl NavMeshProxy {
    pub fn find_path(&mut self, start: Vec3, goal: Vec3) -> Vec<Dynamic> {
        if self.ptr.is_null() {
            return vec![];
        }
        // SAFETY: Null check above guarantees the pointer is non-null.
        // The pointer was created from a valid `&NavMesh` reference in
        // `ScriptingPlugin::register` and the borrow is scoped to this call.
        let nav = unsafe { &*self.ptr };
        let path = nav.find_path(start, goal);
        path.into_iter().map(Dynamic::from).collect()
    }
}

pub fn register_api(engine: &mut Engine) {
    // Register Vec3
    engine
        .register_type_with_name::<Vec3>("Vec3")
        .register_fn("vec3", |x: f32, y: f32, z: f32| Vec3::new(x, y, z))
        .register_get("x", |v: &mut Vec3| v.x)
        .register_get("y", |v: &mut Vec3| v.y)
        .register_get("z", |v: &mut Vec3| v.z)
        .register_set("x", |v: &mut Vec3, val: f32| v.x = val)
        .register_set("y", |v: &mut Vec3, val: f32| v.y = val)
        .register_set("z", |v: &mut Vec3, val: f32| v.z = val)
        .register_fn("+", |a: Vec3, b: Vec3| a + b)
        .register_fn("-", |a: Vec3, b: Vec3| a - b)
        .register_fn("*", |a: Vec3, f: f32| a * f)
        .register_fn("to_string", |v: Vec3| {
            ImmutableString::from(format!("Vec3({}, {}, {})", v.x, v.y, v.z))
        });

    // Register IVec2
    engine
        .register_type_with_name::<IVec2>("IVec2")
        .register_fn("ivec2", |x: i64, y: i64| IVec2::new(x as i32, y as i32))
        .register_get("x", |v: &mut IVec2| v.x as i64)
        .register_get("y", |v: &mut IVec2| v.y as i64)
        .register_set("x", |v: &mut IVec2, val: i64| v.x = val as i32)
        .register_set("y", |v: &mut IVec2, val: i64| v.y = val as i32)
        .register_fn("+", |a: IVec2, b: IVec2| a + b)
        .register_fn("-", |a: IVec2, b: IVec2| a - b)
        .register_fn("to_string", |v: IVec2| {
            ImmutableString::from(format!("IVec2({}, {})", v.x, v.y))
        });

    // Register Quat
    engine
        .register_type_with_name::<Quat>("Quat")
        .register_fn("quat", |x: f32, y: f32, z: f32, w: f32| {
            Quat::from_xyzw(x, y, z, w)
        })
        .register_fn("to_string", |v: Quat| {
            ImmutableString::from(format!("Quat({}, {}, {}, {})", v.x, v.y, v.z, v.w))
        });

    // Register Global Functions
    engine.register_fn("log", |s: ImmutableString| println!("[Script] {}", s));

    // Register ScriptCommands
    engine
        .register_type_with_name::<ScriptCommands>("Commands")
        .register_fn(
            "spawn_prefab",
            |cmds: &mut ScriptCommands, prefab: &str, pos: Vec3| cmds.spawn(prefab, pos),
        )
        .register_fn("despawn", |cmds: &mut ScriptCommands, entity: i64| {
            cmds.despawn(entity)
        })
        .register_fn(
            "set_position",
            |cmds: &mut ScriptCommands, entity: i64, pos: Vec3| cmds.set_position(entity, pos),
        )
        .register_fn(
            "apply_damage",
            |cmds: &mut ScriptCommands, entity: i64, amount: f32| cmds.apply_damage(entity, amount),
        )
        .register_fn("play_sound", |cmds: &mut ScriptCommands, path: &str| {
            cmds.play_sound(path)
        })
        .register_fn(
            "spawn_particle",
            |cmds: &mut ScriptCommands, effect: &str, pos: Vec3| cmds.spawn_particle(effect, pos),
        );

    // Register Physics API
    engine
        .register_type_with_name::<RaycastHit>("RaycastHit")
        .register_get("entity_id", |h: &mut RaycastHit| h.entity_id)
        .register_get("position", |h: &mut RaycastHit| h.position)
        .register_get("normal", |h: &mut RaycastHit| h.normal)
        .register_get("distance", |h: &mut RaycastHit| h.distance)
        .register_fn("to_string", |h: RaycastHit| {
            ImmutableString::from(format!(
                "Hit(Entity: {}, Dist: {:.2})",
                h.entity_id, h.distance
            ))
        });

    engine
        .register_type_with_name::<PhysicsProxy>("Physics")
        .register_fn(
            "raycast",
            |p: &mut PhysicsProxy, origin: Vec3, dir: Vec3, max_dist: f32| {
                p.raycast(origin, dir, max_dist)
            },
        );

    // Register NavMesh API
    engine
        .register_type_with_name::<NavMeshProxy>("NavMesh")
        .register_fn(
            "find_path",
            |n: &mut NavMeshProxy, start: Vec3, goal: Vec3| n.find_path(start, goal),
        );
}

#[cfg(test)]
mod tests {
    use super::*;

    // ScriptCommand tests
    #[test]
    fn test_script_command_spawn() {
        let cmd = ScriptCommand::Spawn {
            prefab: "enemy_goblin".to_string(),
            position: Vec3::new(1.0, 2.0, 3.0),
        };
        if let ScriptCommand::Spawn { prefab, position } = cmd {
            assert_eq!(prefab, "enemy_goblin");
            assert_eq!(position, Vec3::new(1.0, 2.0, 3.0));
        } else {
            panic!("Expected Spawn");
        }
    }

    #[test]
    fn test_script_command_despawn() {
        let cmd = ScriptCommand::Despawn { entity: 42 };
        if let ScriptCommand::Despawn { entity } = cmd {
            assert_eq!(entity, 42);
        } else {
            panic!("Expected Despawn");
        }
    }

    #[test]
    fn test_script_command_set_position() {
        let cmd = ScriptCommand::SetPosition {
            entity: 1,
            position: Vec3::new(10.0, 20.0, 30.0),
        };
        if let ScriptCommand::SetPosition { entity, position } = cmd {
            assert_eq!(entity, 1);
            assert_eq!(position, Vec3::new(10.0, 20.0, 30.0));
        } else {
            panic!("Expected SetPosition");
        }
    }

    #[test]
    fn test_script_command_apply_damage() {
        let cmd = ScriptCommand::ApplyDamage {
            entity: 5,
            amount: 50.5,
        };
        if let ScriptCommand::ApplyDamage { entity, amount } = cmd {
            assert_eq!(entity, 5);
            assert!((amount - 50.5).abs() < f32::EPSILON);
        } else {
            panic!("Expected ApplyDamage");
        }
    }

    #[test]
    fn test_script_command_play_sound() {
        let cmd = ScriptCommand::PlaySound {
            path: "sounds/explosion.wav".to_string(),
        };
        if let ScriptCommand::PlaySound { path } = cmd {
            assert_eq!(path, "sounds/explosion.wav");
        } else {
            panic!("Expected PlaySound");
        }
    }

    #[test]
    fn test_script_command_spawn_particle() {
        let cmd = ScriptCommand::SpawnParticle {
            effect: "fire_burst".to_string(),
            position: Vec3::new(0.0, 5.0, 0.0),
        };
        if let ScriptCommand::SpawnParticle { effect, position } = cmd {
            assert_eq!(effect, "fire_burst");
            assert_eq!(position, Vec3::new(0.0, 5.0, 0.0));
        } else {
            panic!("Expected SpawnParticle");
        }
    }

    #[test]
    fn test_script_command_clone() {
        let cmd = ScriptCommand::Spawn {
            prefab: "test".to_string(),
            position: Vec3::ZERO,
        };
        let cloned = cmd.clone();
        if let ScriptCommand::Spawn { prefab, .. } = cloned {
            assert_eq!(prefab, "test");
        } else {
            panic!("Expected Spawn");
        }
    }

    #[test]
    fn test_script_command_debug() {
        let cmd = ScriptCommand::Despawn { entity: 1 };
        let debug_str = format!("{:?}", cmd);
        assert!(debug_str.contains("Despawn"));
        assert!(debug_str.contains("1"));
    }

    // ScriptCommands tests
    #[test]
    fn test_script_commands_new() {
        let cmds = ScriptCommands::new();
        assert!(cmds.commands.is_empty());
    }

    #[test]
    fn test_script_commands_default() {
        let cmds = ScriptCommands::default();
        assert!(cmds.commands.is_empty());
    }

    #[test]
    fn test_script_commands_spawn() {
        let mut cmds = ScriptCommands::new();
        cmds.spawn("player", Vec3::new(0.0, 0.0, 0.0));
        assert_eq!(cmds.commands.len(), 1);
        if let ScriptCommand::Spawn { prefab, position } = &cmds.commands[0] {
            assert_eq!(prefab, "player");
            assert_eq!(*position, Vec3::ZERO);
        } else {
            panic!("Expected Spawn");
        }
    }

    #[test]
    fn test_script_commands_despawn() {
        let mut cmds = ScriptCommands::new();
        cmds.despawn(100);
        assert_eq!(cmds.commands.len(), 1);
        if let ScriptCommand::Despawn { entity } = &cmds.commands[0] {
            assert_eq!(*entity, 100);
        } else {
            panic!("Expected Despawn");
        }
    }

    #[test]
    fn test_script_commands_set_position() {
        let mut cmds = ScriptCommands::new();
        cmds.set_position(1, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(cmds.commands.len(), 1);
        if let ScriptCommand::SetPosition { entity, position } = &cmds.commands[0] {
            assert_eq!(*entity, 1);
            assert_eq!(*position, Vec3::new(1.0, 2.0, 3.0));
        } else {
            panic!("Expected SetPosition");
        }
    }

    #[test]
    fn test_script_commands_apply_damage() {
        let mut cmds = ScriptCommands::new();
        cmds.apply_damage(5, 25.0);
        assert_eq!(cmds.commands.len(), 1);
        if let ScriptCommand::ApplyDamage { entity, amount } = &cmds.commands[0] {
            assert_eq!(*entity, 5);
            assert!((*amount - 25.0).abs() < f32::EPSILON);
        } else {
            panic!("Expected ApplyDamage");
        }
    }

    #[test]
    fn test_script_commands_play_sound() {
        let mut cmds = ScriptCommands::new();
        cmds.play_sound("music/theme.mp3");
        assert_eq!(cmds.commands.len(), 1);
        if let ScriptCommand::PlaySound { path } = &cmds.commands[0] {
            assert_eq!(path, "music/theme.mp3");
        } else {
            panic!("Expected PlaySound");
        }
    }

    #[test]
    fn test_script_commands_spawn_particle() {
        let mut cmds = ScriptCommands::new();
        cmds.spawn_particle("smoke", Vec3::new(1.0, 1.0, 1.0));
        assert_eq!(cmds.commands.len(), 1);
        if let ScriptCommand::SpawnParticle { effect, position } = &cmds.commands[0] {
            assert_eq!(effect, "smoke");
            assert_eq!(*position, Vec3::new(1.0, 1.0, 1.0));
        } else {
            panic!("Expected SpawnParticle");
        }
    }

    #[test]
    fn test_script_commands_multiple() {
        let mut cmds = ScriptCommands::new();
        cmds.spawn("enemy", Vec3::ZERO);
        cmds.apply_damage(1, 10.0);
        cmds.play_sound("hit.wav");
        cmds.spawn_particle("blood", Vec3::new(0.0, 1.0, 0.0));
        assert_eq!(cmds.commands.len(), 4);
    }

    #[test]
    fn test_script_commands_order_preserved() {
        let mut cmds = ScriptCommands::new();
        cmds.spawn("a", Vec3::ZERO);
        cmds.spawn("b", Vec3::ONE);
        cmds.spawn("c", Vec3::new(2.0, 2.0, 2.0));

        let prefabs: Vec<&str> = cmds
            .commands
            .iter()
            .filter_map(|c| {
                if let ScriptCommand::Spawn { prefab, .. } = c {
                    Some(prefab.as_str())
                } else {
                    None
                }
            })
            .collect();

        assert_eq!(prefabs, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_script_commands_clone() {
        let mut cmds = ScriptCommands::new();
        cmds.spawn("test", Vec3::ZERO);
        let cloned = cmds.clone();
        assert_eq!(cloned.commands.len(), 1);
    }

    // RaycastHit tests
    #[test]
    fn test_raycast_hit_creation() {
        let hit = RaycastHit {
            entity_id: 42,
            position: Vec3::new(1.0, 2.0, 3.0),
            normal: Vec3::new(0.0, 1.0, 0.0),
            distance: 5.5,
        };
        assert_eq!(hit.entity_id, 42);
        assert_eq!(hit.position, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(hit.normal, Vec3::new(0.0, 1.0, 0.0));
        assert!((hit.distance - 5.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_raycast_hit_clone() {
        let hit = RaycastHit {
            entity_id: 1,
            position: Vec3::ZERO,
            normal: Vec3::Y,
            distance: 1.0,
        };
        let cloned = hit.clone();
        assert_eq!(cloned.entity_id, hit.entity_id);
        assert_eq!(cloned.position, hit.position);
    }

    #[test]
    fn test_raycast_hit_debug() {
        let hit = RaycastHit {
            entity_id: 10,
            position: Vec3::ZERO,
            normal: Vec3::Z,
            distance: 2.5,
        };
        let debug_str = format!("{:?}", hit);
        assert!(debug_str.contains("RaycastHit"));
        assert!(debug_str.contains("entity_id"));
        assert!(debug_str.contains("10"));
    }
}
