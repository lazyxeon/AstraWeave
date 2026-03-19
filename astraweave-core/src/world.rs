use crate::{Entity, IVec2};
use astraweave_behavior::BehaviorGraph;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug)]
pub struct Health {
    pub hp: i32,
}

#[derive(Clone, Copy, Debug)]
pub struct Team {
    pub id: u8,
} // 0: player, 1: companion, 2: enemy

#[derive(Clone, Copy, Debug)]
pub struct Ammo {
    pub rounds: i32,
}

#[derive(Clone, Debug)]
pub struct Cooldowns {
    pub map: HashMap<String, f32>,
}

#[derive(Clone, Copy, Debug)]
pub struct Pose {
    pub pos: IVec2,
    pub height: f32,   // Vertical position (Y axis in 3D)
    pub rotation: f32, // Rotation in radians around Y axis (primary, for compatibility)
    pub rotation_x: f32, // Pitch (rotation around X axis)
    pub rotation_z: f32, // Roll (rotation around Z axis)
    pub scale: f32,    // Uniform scale factor
}

#[derive(Default)]
pub struct World {
    pub t: f32,
    pub next_id: Entity,
    pub obstacles: HashSet<(i32, i32)>,
    poses: HashMap<Entity, Pose>,
    health: HashMap<Entity, Health>,
    team: HashMap<Entity, Team>,
    ammo: HashMap<Entity, Ammo>,
    cds: HashMap<Entity, Cooldowns>,
    names: HashMap<Entity, String>,
    behavior_graphs: HashMap<Entity, BehaviorGraph>,
}

impl World {
    pub fn new() -> Self {
        Self {
            t: 0.0,
            next_id: 1,
            ..Default::default()
        }
    }

    pub fn spawn(&mut self, name: &str, pos: IVec2, team: Team, hp: i32, ammo: i32) -> Entity {
        let id = self.next_id;
        self.next_id += 1;
        self.insert_entity(id, name, pos, team, hp, ammo)
    }

    /// Spawn an entity with an explicit id (used for deterministic serialization).
    pub fn spawn_with_id(
        &mut self,
        id: Entity,
        name: &str,
        pos: IVec2,
        team: Team,
        hp: i32,
        ammo: i32,
    ) -> Entity {
        if id >= self.next_id {
            self.next_id = id + 1;
        }
        self.insert_entity(id, name, pos, team, hp, ammo)
    }

    fn insert_entity(
        &mut self,
        id: Entity,
        name: &str,
        pos: IVec2,
        team: Team,
        hp: i32,
        ammo: i32,
    ) -> Entity {
        debug_assert!(!self.poses.contains_key(&id), "entity {id} already exists");
        self.poses.insert(
            id,
            Pose {
                pos,
                height: 0.0,
                rotation: 0.0,
                rotation_x: 0.0,
                rotation_z: 0.0,
                scale: 1.0,
            },
        );
        self.health.insert(id, Health { hp });
        self.team.insert(id, team);
        self.ammo.insert(id, Ammo { rounds: ammo });
        self.cds.insert(
            id,
            Cooldowns {
                map: HashMap::new(),
            },
        );
        self.names.insert(id, name.to_string());
        id
    }

    pub fn tick(&mut self, dt: f32) {
        self.t += dt;
        for cd in self.cds.values_mut() {
            for v in cd.map.values_mut() {
                *v = (*v - dt).max(0.0);
            }
        }
    }

    /// Destroy an entity, removing all its components from the world.
    /// Returns true if the entity existed and was destroyed, false otherwise.
    pub fn destroy_entity(&mut self, e: Entity) -> bool {
        let existed = self.poses.remove(&e).is_some();
        if existed {
            self.health.remove(&e);
            self.team.remove(&e);
            self.ammo.remove(&e);
            self.cds.remove(&e);
            self.names.remove(&e);
            self.behavior_graphs.remove(&e);
        }
        existed
    }

    // getters/setters
    pub fn pose(&self, e: Entity) -> Option<Pose> {
        self.poses.get(&e).copied()
    }
    pub fn pose_mut(&mut self, e: Entity) -> Option<&mut Pose> {
        self.poses.get_mut(&e)
    }
    pub fn health(&self, e: Entity) -> Option<Health> {
        self.health.get(&e).copied()
    }
    pub fn health_mut(&mut self, e: Entity) -> Option<&mut Health> {
        self.health.get_mut(&e)
    }
    pub fn team(&self, e: Entity) -> Option<Team> {
        self.team.get(&e).copied()
    }
    pub fn team_mut(&mut self, e: Entity) -> Option<&mut Team> {
        self.team.get_mut(&e)
    }
    pub fn ammo(&self, e: Entity) -> Option<Ammo> {
        self.ammo.get(&e).copied()
    }
    pub fn ammo_mut(&mut self, e: Entity) -> Option<&mut Ammo> {
        self.ammo.get_mut(&e)
    }
    pub fn cooldowns(&self, e: Entity) -> Option<&Cooldowns> {
        self.cds.get(&e)
    }
    pub fn cooldowns_mut(&mut self, e: Entity) -> Option<&mut Cooldowns> {
        self.cds.get_mut(&e)
    }
    pub fn name(&self, e: Entity) -> Option<&str> {
        self.names.get(&e).map(|s| s.as_str())
    }
    pub fn behavior_graph(&self, e: Entity) -> Option<&BehaviorGraph> {
        self.behavior_graphs.get(&e)
    }
    pub fn behavior_graph_mut(&mut self, e: Entity) -> Option<&mut BehaviorGraph> {
        self.behavior_graphs.get_mut(&e)
    }
    pub fn set_behavior_graph(&mut self, e: Entity, graph: BehaviorGraph) {
        self.behavior_graphs.insert(e, graph);
    }
    pub fn remove_behavior_graph(&mut self, e: Entity) -> Option<BehaviorGraph> {
        self.behavior_graphs.remove(&e)
    }

    pub fn all_of_team(&self, team_id: u8) -> Vec<Entity> {
        self.team
            .iter()
            .filter_map(|(e, t)| if t.id == team_id { Some(*e) } else { None })
            .collect()
    }
    pub fn enemies_of(&self, team_id: u8) -> Vec<Entity> {
        self.team
            .iter()
            .filter_map(|(e, t)| if t.id != team_id { Some(*e) } else { None })
            .collect()
    }
    pub fn pos_of(&self, e: Entity) -> Option<IVec2> {
        self.poses.get(&e).map(|p| p.pos)
    }
    /// Return a list of all entity ids currently present in the world.
    pub fn entities(&self) -> Vec<Entity> {
        self.poses.keys().copied().collect()
    }
    pub fn obstacle(&self, p: IVec2) -> bool {
        self.obstacles.contains(&(p.x, p.y))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_world_new() {
        let w = World::new();
        assert_eq!(w.t, 0.0);
        assert_eq!(w.next_id, 1);
        assert!(w.obstacles.is_empty());
        assert!(w.entities().is_empty());
    }

    #[test]
    fn test_world_default() {
        let w = World::default();
        assert_eq!(w.t, 0.0);
        assert_eq!(w.next_id, 0);
        assert!(w.obstacles.is_empty());
    }

    #[test]
    fn test_spawn_entity() {
        let mut w = World::new();
        let e = w.spawn("player", IVec2 { x: 5, y: 10 }, Team { id: 0 }, 100, 30);

        assert_eq!(e, 1);
        assert_eq!(w.next_id, 2);
        assert_eq!(w.name(e), Some("player"));
        assert_eq!(w.pose(e).unwrap().pos, IVec2 { x: 5, y: 10 });
        assert_eq!(w.health(e).unwrap().hp, 100);
        assert_eq!(w.team(e).unwrap().id, 0);
        assert_eq!(w.ammo(e).unwrap().rounds, 30);
        assert!(w.cooldowns(e).unwrap().map.is_empty());
    }

    #[test]
    fn test_spawn_multiple_entities() {
        let mut w = World::new();
        let e1 = w.spawn("player", IVec2 { x: 0, y: 0 }, Team { id: 0 }, 100, 30);
        let e2 = w.spawn("enemy", IVec2 { x: 10, y: 10 }, Team { id: 2 }, 50, 15);
        let e3 = w.spawn("companion", IVec2 { x: 5, y: 5 }, Team { id: 1 }, 80, 20);

        assert_eq!(e1, 1);
        assert_eq!(e2, 2);
        assert_eq!(e3, 3);
        assert_eq!(w.next_id, 4);
        assert_eq!(w.entities().len(), 3);
    }

    #[test]
    fn test_spawn_with_id_preserves_entity_id() {
        let mut w = World::new();
        let e = w.spawn_with_id(42, "custom", IVec2 { x: 1, y: 2 }, Team { id: 0 }, 90, 12);

        assert_eq!(e, 42);
        assert_eq!(w.next_id, 43);
        assert_eq!(w.pose(42).unwrap().pos, IVec2 { x: 1, y: 2 });
    }

    #[test]
    fn test_tick_updates_time() {
        let mut w = World::new();
        w.tick(0.1);
        assert!((w.t - 0.1).abs() < 1e-6);
        w.tick(0.2);
        assert!((w.t - 0.3).abs() < 1e-6);
    }

    #[test]
    fn test_tick_decrements_cooldowns() {
        let mut w = World::new();
        let e = w.spawn("player", IVec2 { x: 0, y: 0 }, Team { id: 0 }, 100, 30);

        w.cooldowns_mut(e).unwrap().map.insert("attack".into(), 5.0);
        w.cooldowns_mut(e).unwrap().map.insert("heal".into(), 10.0);

        w.tick(2.0);

        let cds = w.cooldowns(e).unwrap();
        assert!((cds.map.get("attack").unwrap() - 3.0).abs() < 1e-6);
        assert!((cds.map.get("heal").unwrap() - 8.0).abs() < 1e-6);
    }

    #[test]
    fn test_tick_cooldowns_bottom_at_zero() {
        let mut w = World::new();
        let e = w.spawn("player", IVec2 { x: 0, y: 0 }, Team { id: 0 }, 100, 30);

        w.cooldowns_mut(e).unwrap().map.insert("attack".into(), 1.0);
        w.tick(2.0);

        let cds = w.cooldowns(e).unwrap();
        assert_eq!(*cds.map.get("attack").unwrap(), 0.0);
    }

    #[test]
    fn test_pose_getter() {
        let mut w = World::new();
        let e = w.spawn("player", IVec2 { x: 7, y: 13 }, Team { id: 0 }, 100, 30);

        let pose = w.pose(e).unwrap();
        assert_eq!(pose.pos.x, 7);
        assert_eq!(pose.pos.y, 13);
    }

    #[test]
    fn test_pose_mut() {
        let mut w = World::new();
        let e = w.spawn("player", IVec2 { x: 0, y: 0 }, Team { id: 0 }, 100, 30);

        w.pose_mut(e).unwrap().pos = IVec2 { x: 20, y: 30 };

        assert_eq!(w.pose(e).unwrap().pos, IVec2 { x: 20, y: 30 });
    }

    #[test]
    fn test_pose_nonexistent_entity() {
        let w = World::new();
        assert!(w.pose(999).is_none());
        assert_eq!(w.pos_of(999), None);
    }

    #[test]
    fn test_health_getter() {
        let mut w = World::new();
        let e = w.spawn("player", IVec2 { x: 0, y: 0 }, Team { id: 0 }, 75, 30);

        assert_eq!(w.health(e).unwrap().hp, 75);
    }

    #[test]
    fn test_health_mut() {
        let mut w = World::new();
        let e = w.spawn("player", IVec2 { x: 0, y: 0 }, Team { id: 0 }, 100, 30);

        w.health_mut(e).unwrap().hp = 50;

        assert_eq!(w.health(e).unwrap().hp, 50);
    }

    #[test]
    fn test_health_nonexistent_entity() {
        let w = World::new();
        assert!(w.health(999).is_none());
    }

    #[test]
    fn test_team_getter() {
        let mut w = World::new();
        let e = w.spawn("enemy", IVec2 { x: 0, y: 0 }, Team { id: 2 }, 50, 15);

        assert_eq!(w.team(e).unwrap().id, 2);
    }

    #[test]
    fn test_team_nonexistent_entity() {
        let w = World::new();
        assert!(w.team(999).is_none());
    }

    #[test]
    fn test_ammo_getter() {
        let mut w = World::new();
        let e = w.spawn("player", IVec2 { x: 0, y: 0 }, Team { id: 0 }, 100, 42);

        assert_eq!(w.ammo(e).unwrap().rounds, 42);
    }

    #[test]
    fn test_ammo_mut() {
        let mut w = World::new();
        let e = w.spawn("player", IVec2 { x: 0, y: 0 }, Team { id: 0 }, 100, 30);

        w.ammo_mut(e).unwrap().rounds = 10;

        assert_eq!(w.ammo(e).unwrap().rounds, 10);
    }

    #[test]
    fn test_ammo_nonexistent_entity() {
        let w = World::new();
        assert!(w.ammo(999).is_none());
    }

    #[test]
    fn test_cooldowns_getter() {
        let mut w = World::new();
        let e = w.spawn("player", IVec2 { x: 0, y: 0 }, Team { id: 0 }, 100, 30);

        let cds = w.cooldowns(e).unwrap();
        assert!(cds.map.is_empty());
    }

    #[test]
    fn test_cooldowns_mut() {
        let mut w = World::new();
        let e = w.spawn("player", IVec2 { x: 0, y: 0 }, Team { id: 0 }, 100, 30);

        w.cooldowns_mut(e).unwrap().map.insert("attack".into(), 5.0);

        let cds = w.cooldowns(e).unwrap();
        assert_eq!(*cds.map.get("attack").unwrap(), 5.0);
    }

    #[test]
    fn test_cooldowns_nonexistent_entity() {
        let w = World::new();
        assert!(w.cooldowns(999).is_none());
    }

    #[test]
    fn test_name_getter() {
        let mut w = World::new();
        let e = w.spawn("hero", IVec2 { x: 0, y: 0 }, Team { id: 0 }, 100, 30);

        assert_eq!(w.name(e), Some("hero"));
    }

    #[test]
    fn test_name_nonexistent_entity() {
        let w = World::new();
        assert!(w.name(999).is_none());
    }

    #[test]
    fn test_behavior_graph_assignment_and_retrieval() {
        use astraweave_behavior::{BehaviorGraph, BehaviorNode};

        let mut world = World::new();
        let entity = world.spawn("ai", IVec2 { x: 0, y: 0 }, Team { id: 0 }, 100, 30);
        let graph = BehaviorGraph::new(BehaviorNode::Action("idle".into()));

        world.set_behavior_graph(entity, graph.clone());
        let stored = world.behavior_graph(entity).expect("graph stored");
        if let BehaviorNode::Action(name) = &stored.root {
            assert_eq!(name, "idle");
        } else {
            panic!("expected action node");
        }

        let removed = world.remove_behavior_graph(entity);
        assert!(removed.is_some());
        assert!(world.behavior_graph(entity).is_none());
    }

    #[test]
    fn test_all_of_team() {
        let mut w = World::new();
        let p1 = w.spawn("player", IVec2 { x: 0, y: 0 }, Team { id: 0 }, 100, 30);
        let e1 = w.spawn("enemy1", IVec2 { x: 10, y: 10 }, Team { id: 2 }, 50, 15);
        let e2 = w.spawn("enemy2", IVec2 { x: 15, y: 15 }, Team { id: 2 }, 50, 15);
        let c1 = w.spawn("companion", IVec2 { x: 5, y: 5 }, Team { id: 1 }, 80, 20);

        let team_0 = w.all_of_team(0);
        assert_eq!(team_0.len(), 1);
        assert!(team_0.contains(&p1));

        let team_1 = w.all_of_team(1);
        assert_eq!(team_1.len(), 1);
        assert!(team_1.contains(&c1));

        let team_2 = w.all_of_team(2);
        assert_eq!(team_2.len(), 2);
        assert!(team_2.contains(&e1));
        assert!(team_2.contains(&e2));
    }

    #[test]
    fn test_all_of_team_empty() {
        let w = World::new();
        let team_0 = w.all_of_team(0);
        assert!(team_0.is_empty());
    }

    #[test]
    fn test_enemies_of() {
        let mut w = World::new();
        let p1 = w.spawn("player", IVec2 { x: 0, y: 0 }, Team { id: 0 }, 100, 30);
        let e1 = w.spawn("enemy1", IVec2 { x: 10, y: 10 }, Team { id: 2 }, 50, 15);
        let e2 = w.spawn("enemy2", IVec2 { x: 15, y: 15 }, Team { id: 2 }, 50, 15);
        let c1 = w.spawn("companion", IVec2 { x: 5, y: 5 }, Team { id: 1 }, 80, 20);

        let enemies_of_player = w.enemies_of(0);
        assert_eq!(enemies_of_player.len(), 3);
        assert!(enemies_of_player.contains(&e1));
        assert!(enemies_of_player.contains(&e2));
        assert!(enemies_of_player.contains(&c1));
        assert!(!enemies_of_player.contains(&p1));
    }

    #[test]
    fn test_enemies_of_empty() {
        let w = World::new();
        let enemies = w.enemies_of(0);
        assert!(enemies.is_empty());
    }

    #[test]
    fn test_pos_of() {
        let mut w = World::new();
        let e = w.spawn("player", IVec2 { x: 12, y: 34 }, Team { id: 0 }, 100, 30);

        let pos = w.pos_of(e).unwrap();
        assert_eq!(pos.x, 12);
        assert_eq!(pos.y, 34);
    }

    #[test]
    fn test_pos_of_nonexistent() {
        let w = World::new();
        assert!(w.pos_of(999).is_none());
    }

    #[test]
    fn test_entities() {
        let mut w = World::new();
        assert!(w.entities().is_empty());

        let e1 = w.spawn("player", IVec2 { x: 0, y: 0 }, Team { id: 0 }, 100, 30);
        let e2 = w.spawn("enemy", IVec2 { x: 10, y: 10 }, Team { id: 2 }, 50, 15);

        let entities = w.entities();
        assert_eq!(entities.len(), 2);
        assert!(entities.contains(&e1));
        assert!(entities.contains(&e2));
    }

    #[test]
    fn test_obstacle_present() {
        let mut w = World::new();
        w.obstacles.insert((5, 10));

        assert!(w.obstacle(IVec2 { x: 5, y: 10 }));
    }

    #[test]
    fn test_obstacle_absent() {
        let w = World::new();
        assert!(!w.obstacle(IVec2 { x: 5, y: 10 }));
    }

    #[test]
    fn test_obstacle_multiple() {
        let mut w = World::new();
        w.obstacles.insert((0, 0));
        w.obstacles.insert((5, 5));
        w.obstacles.insert((10, 10));

        assert!(w.obstacle(IVec2 { x: 0, y: 0 }));
        assert!(w.obstacle(IVec2 { x: 5, y: 5 }));
        assert!(w.obstacle(IVec2 { x: 10, y: 10 }));
        assert!(!w.obstacle(IVec2 { x: 7, y: 7 }));
    }

    #[test]
    fn test_destroy_entity_removes_all_components() {
        let mut w = World::new();
        let e = w.spawn("player", IVec2 { x: 5, y: 10 }, Team { id: 0 }, 100, 30);

        assert!(w.pose(e).is_some());
        assert!(w.health(e).is_some());
        assert!(w.team(e).is_some());
        assert!(w.ammo(e).is_some());
        assert!(w.cooldowns(e).is_some());
        assert!(w.name(e).is_some());

        let destroyed = w.destroy_entity(e);
        assert!(destroyed);

        assert!(w.pose(e).is_none());
        assert!(w.health(e).is_none());
        assert!(w.team(e).is_none());
        assert!(w.ammo(e).is_none());
        assert!(w.cooldowns(e).is_none());
        assert!(w.name(e).is_none());
        assert!(w.behavior_graph(e).is_none());
    }

    #[test]
    fn test_destroy_entity_returns_false_for_nonexistent_entity() {
        let mut w = World::new();
        let destroyed = w.destroy_entity(999);
        assert!(!destroyed);
    }

    #[test]
    fn test_destroy_entity_updates_entities_list() {
        let mut w = World::new();
        let e1 = w.spawn("entity1", IVec2 { x: 0, y: 0 }, Team { id: 0 }, 100, 30);
        let e2 = w.spawn("entity2", IVec2 { x: 5, y: 5 }, Team { id: 0 }, 100, 30);
        let e3 = w.spawn("entity3", IVec2 { x: 10, y: 10 }, Team { id: 0 }, 100, 30);

        assert_eq!(w.entities().len(), 3);

        w.destroy_entity(e2);

        let entities = w.entities();
        assert_eq!(entities.len(), 2);
        assert!(entities.contains(&e1));
        assert!(!entities.contains(&e2));
        assert!(entities.contains(&e3));
    }

    #[test]
    fn test_destroy_entity_preserves_other_entities() {
        let mut w = World::new();
        let e1 = w.spawn("entity1", IVec2 { x: 0, y: 0 }, Team { id: 0 }, 100, 30);
        let e2 = w.spawn("entity2", IVec2 { x: 5, y: 5 }, Team { id: 1 }, 80, 20);

        w.destroy_entity(e1);

        assert!(w.pose(e1).is_none());
        assert!(w.pose(e2).is_some());
        assert_eq!(w.pose(e2).unwrap().pos, IVec2 { x: 5, y: 5 });
        assert_eq!(w.health(e2).unwrap().hp, 80);
        assert_eq!(w.team(e2).unwrap().id, 1);
        assert_eq!(w.ammo(e2).unwrap().rounds, 20);
    }

    // ========================================================================
    // Mutation-resistant remediation: _mut getter tests
    // ========================================================================

    /// Kills: `World::team_mut -> Option<&mut Team> with None`
    /// Existing tests only use the immutable `team()` getter.
    #[test]
    fn test_team_mut_modifies_value() {
        let mut w = World::new();
        let e = w.spawn("unit", IVec2 { x: 0, y: 0 }, Team { id: 1 }, 100, 0);
        assert_eq!(w.team(e).unwrap().id, 1);

        // Mutate through team_mut — if team_mut returns None, this panics
        w.team_mut(e).unwrap().id = 5;
        assert_eq!(w.team(e).unwrap().id, 5, "team_mut must allow mutation");
    }

    /// Kills: `World::behavior_graph_mut -> Option<&mut BehaviorGraph> with None`
    /// Existing test `test_behavior_graph_assignment_and_retrieval` never uses behavior_graph_mut.
    #[test]
    fn test_behavior_graph_mut_modifies_root() {
        use astraweave_behavior::{BehaviorGraph, BehaviorNode};

        let mut w = World::new();
        let e = w.spawn("ai", IVec2 { x: 0, y: 0 }, Team { id: 0 }, 100, 0);
        let graph = BehaviorGraph::new(BehaviorNode::Action("patrol".into()));
        w.set_behavior_graph(e, graph);

        // Mutate through behavior_graph_mut — if it returns None, this panics
        let bg = w.behavior_graph_mut(e).unwrap();
        bg.root = BehaviorNode::Action("attack".into());

        // Verify mutation persisted
        let stored = w.behavior_graph(e).unwrap();
        if let BehaviorNode::Action(name) = &stored.root {
            assert_eq!(name, "attack", "behavior_graph_mut must allow mutation");
        } else {
            panic!("expected Action node after mutation");
        }
    }
}
