use crate::{Entity, IVec2};
use astraweave_behavior::BehaviorGraph;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;

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
    /// BTreeMap for deterministic iteration order (important for replays)
    /// and to match WorldSnapshot schema directly (avoids clone+convert).
    pub map: BTreeMap<String, f32>,
}

#[derive(Clone, Copy, Debug)]
pub struct Pose {
    pub pos: IVec2,
    pub height: f32,     // Vertical position (Y axis in 3D)
    pub rotation: f32,   // Rotation in radians around Y axis (primary, for compatibility)
    pub rotation_x: f32, // Pitch (rotation around X axis)
    pub rotation_z: f32, // Roll (rotation around Z axis)
    pub scale: f32,      // Uniform scale factor (X axis, or uniform when scale_y/scale_z match)
    /// Per-axis Y scale. When equal to `scale`, entity has uniform scaling.
    pub scale_y: f32,
    /// Per-axis Z scale. When equal to `scale`, entity has uniform scaling.
    pub scale_z: f32,
    /// High-precision float X position (overrides pos.x when use_float_pos is true).
    /// Used for scatter objects that need sub-grid-unit precision.
    pub float_x: f32,
    /// High-precision float Z position (overrides pos.y when use_float_pos is true).
    pub float_z: f32,
    /// When true, renderer uses float_x/float_z instead of pos for world placement.
    pub use_float_pos: bool,
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
    /// Parent-child hierarchy: maps child → parent.
    parents: HashMap<Entity, Entity>,
    /// Derived index: maps parent → ordered children list.
    /// Kept in sync with `parents` by `set_parent` / `remove_parent`.
    children_map: HashMap<Entity, Vec<Entity>>,
    /// Cached obstacles as IVec2 for efficient sharing across snapshots.
    /// Rebuilt lazily when `obstacles_as_ivec2()` is called after changes.
    obstacles_cache: Option<Arc<Vec<IVec2>>>,
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
                scale_y: 1.0,
                scale_z: 1.0,
                float_x: 0.0,
                float_z: 0.0,
                use_float_pos: false,
            },
        );
        self.health.insert(id, Health { hp });
        self.team.insert(id, team);
        self.ammo.insert(id, Ammo { rounds: ammo });
        self.cds.insert(
            id,
            Cooldowns {
                map: BTreeMap::new(),
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
    /// Also cleans up hierarchy: removes from parent's children list,
    /// and orphans any children (makes them roots).
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

            // Remove from parent's children list
            if let Some(parent) = self.parents.remove(&e) {
                if let Some(siblings) = self.children_map.get_mut(&parent) {
                    siblings.retain(|&child| child != e);
                }
            }

            // Orphan any children (remove their parent reference)
            if let Some(children) = self.children_map.remove(&e) {
                for child in children {
                    self.parents.remove(&child);
                }
            }
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

    // ========================================================================
    // Hierarchy: parent-child relationships
    // ========================================================================

    /// Set the parent of a child entity. Removes any previous parent.
    /// Does nothing if child == parent or if it would create a cycle.
    pub fn set_parent(&mut self, child: Entity, parent: Entity) {
        if child == parent {
            return;
        }
        // Prevent cycles: if child is an ancestor of parent, skip
        if self.is_ancestor_of(child, parent) {
            return;
        }
        // Remove from old parent's children list
        self.remove_parent(child);
        // Set new parent
        self.parents.insert(child, parent);
        self.children_map.entry(parent).or_default().push(child);
    }

    /// Remove the parent of an entity (make it a root).
    /// Returns the old parent if one existed.
    pub fn remove_parent(&mut self, child: Entity) -> Option<Entity> {
        if let Some(old_parent) = self.parents.remove(&child) {
            if let Some(siblings) = self.children_map.get_mut(&old_parent) {
                siblings.retain(|&e| e != child);
                if siblings.is_empty() {
                    self.children_map.remove(&old_parent);
                }
            }
            Some(old_parent)
        } else {
            None
        }
    }

    /// Get the parent of an entity, if any.
    pub fn parent_of(&self, e: Entity) -> Option<Entity> {
        self.parents.get(&e).copied()
    }

    /// Get the ordered children of an entity.
    pub fn children_of(&self, e: Entity) -> &[Entity] {
        self.children_map
            .get(&e)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Check if `ancestor` is an ancestor of `descendant` (recursive).
    pub fn is_ancestor_of(&self, ancestor: Entity, descendant: Entity) -> bool {
        let mut current = descendant;
        while let Some(parent) = self.parents.get(&current) {
            if *parent == ancestor {
                return true;
            }
            current = *parent;
        }
        false
    }

    /// Collect all descendants of an entity (recursive, depth-first).
    pub fn descendants_of(&self, e: Entity) -> Vec<Entity> {
        let mut result = Vec::new();
        self.collect_descendants(e, &mut result);
        result
    }

    fn collect_descendants(&self, e: Entity, result: &mut Vec<Entity>) {
        for &child in self.children_of(e) {
            result.push(child);
            self.collect_descendants(child, result);
        }
    }

    /// Get all root entities (entities without a parent).
    pub fn root_entities(&self) -> Vec<Entity> {
        self.poses
            .keys()
            .filter(|e| !self.parents.contains_key(e))
            .copied()
            .collect()
    }

    // ========================================================================
    // Name mutation
    // ========================================================================

    /// Set or update the name of an entity.
    pub fn set_name(&mut self, e: Entity, name: String) {
        if self.poses.contains_key(&e) {
            self.names.insert(e, name);
        }
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

    /// Iterate over all entity ids without allocating a Vec.
    /// Prefer this over `entities()` on hot paths.
    pub fn iter_entities(&self) -> impl Iterator<Item = Entity> + '_ {
        self.poses.keys().copied()
    }

    /// Number of entities in the world.
    pub fn entity_count(&self) -> usize {
        self.poses.len()
    }
    pub fn obstacle(&self, p: IVec2) -> bool {
        self.obstacles.contains(&(p.x, p.y))
    }

    /// Get obstacles as a shared `Arc<Vec<IVec2>>`, suitable for embedding in
    /// multiple WorldSnapshots without deep-copying. The cache is rebuilt only
    /// when called after `invalidate_obstacles_cache()` or the first time.
    pub fn obstacles_as_ivec2(&mut self) -> Arc<Vec<IVec2>> {
        if let Some(ref cached) = self.obstacles_cache {
            return Arc::clone(cached);
        }
        let vec: Vec<IVec2> = self
            .obstacles
            .iter()
            .map(|&(x, y)| IVec2 { x, y })
            .collect();
        let arc = Arc::new(vec);
        self.obstacles_cache = Some(Arc::clone(&arc));
        arc
    }

    /// Invalidate the obstacles cache. Call after modifying `self.obstacles`.
    pub fn invalidate_obstacles_cache(&mut self) {
        self.obstacles_cache = None;
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
    // Hierarchy tests
    // ========================================================================

    #[test]
    fn test_set_parent() {
        let mut w = World::new();
        let parent = w.spawn("parent", IVec2 { x: 0, y: 0 }, Team { id: 0 }, 100, 0);
        let child = w.spawn("child", IVec2 { x: 1, y: 1 }, Team { id: 0 }, 100, 0);

        w.set_parent(child, parent);
        assert_eq!(w.parent_of(child), Some(parent));
        assert_eq!(w.children_of(parent), &[child]);
    }

    #[test]
    fn test_remove_parent() {
        let mut w = World::new();
        let parent = w.spawn("parent", IVec2 { x: 0, y: 0 }, Team { id: 0 }, 100, 0);
        let child = w.spawn("child", IVec2 { x: 1, y: 1 }, Team { id: 0 }, 100, 0);

        w.set_parent(child, parent);
        let old = w.remove_parent(child);
        assert_eq!(old, Some(parent));
        assert_eq!(w.parent_of(child), None);
        assert!(w.children_of(parent).is_empty());
    }

    #[test]
    fn test_set_parent_prevents_self_parenting() {
        let mut w = World::new();
        let e = w.spawn("entity", IVec2 { x: 0, y: 0 }, Team { id: 0 }, 100, 0);

        w.set_parent(e, e);
        assert_eq!(w.parent_of(e), None);
    }

    #[test]
    fn test_set_parent_prevents_cycles() {
        let mut w = World::new();
        let a = w.spawn("a", IVec2 { x: 0, y: 0 }, Team { id: 0 }, 100, 0);
        let b = w.spawn("b", IVec2 { x: 1, y: 1 }, Team { id: 0 }, 100, 0);
        let c = w.spawn("c", IVec2 { x: 2, y: 2 }, Team { id: 0 }, 100, 0);

        w.set_parent(b, a); // a -> b
        w.set_parent(c, b); // a -> b -> c

        // Trying to make a a child of c should be rejected (cycle)
        w.set_parent(a, c);
        assert_eq!(w.parent_of(a), None);
    }

    #[test]
    fn test_is_ancestor_of() {
        let mut w = World::new();
        let a = w.spawn("a", IVec2 { x: 0, y: 0 }, Team { id: 0 }, 100, 0);
        let b = w.spawn("b", IVec2 { x: 1, y: 1 }, Team { id: 0 }, 100, 0);
        let c = w.spawn("c", IVec2 { x: 2, y: 2 }, Team { id: 0 }, 100, 0);

        w.set_parent(b, a);
        w.set_parent(c, b);

        assert!(w.is_ancestor_of(a, c));
        assert!(w.is_ancestor_of(a, b));
        assert!(!w.is_ancestor_of(c, a));
    }

    #[test]
    fn test_descendants_of() {
        let mut w = World::new();
        let a = w.spawn("a", IVec2 { x: 0, y: 0 }, Team { id: 0 }, 100, 0);
        let b = w.spawn("b", IVec2 { x: 1, y: 1 }, Team { id: 0 }, 100, 0);
        let c = w.spawn("c", IVec2 { x: 2, y: 2 }, Team { id: 0 }, 100, 0);

        w.set_parent(b, a);
        w.set_parent(c, b);

        let desc = w.descendants_of(a);
        assert_eq!(desc.len(), 2);
        assert!(desc.contains(&b));
        assert!(desc.contains(&c));

        assert!(w.descendants_of(c).is_empty());
    }

    #[test]
    fn test_root_entities() {
        let mut w = World::new();
        let a = w.spawn("a", IVec2 { x: 0, y: 0 }, Team { id: 0 }, 100, 0);
        let b = w.spawn("b", IVec2 { x: 1, y: 1 }, Team { id: 0 }, 100, 0);
        let c = w.spawn("c", IVec2 { x: 2, y: 2 }, Team { id: 0 }, 100, 0);

        w.set_parent(b, a);

        let roots = w.root_entities();
        assert_eq!(roots.len(), 2);
        assert!(roots.contains(&a));
        assert!(roots.contains(&c));
        assert!(!roots.contains(&b));
    }

    #[test]
    fn test_destroy_entity_cleans_hierarchy() {
        let mut w = World::new();
        let parent = w.spawn("parent", IVec2 { x: 0, y: 0 }, Team { id: 0 }, 100, 0);
        let child = w.spawn("child", IVec2 { x: 1, y: 1 }, Team { id: 0 }, 100, 0);

        w.set_parent(child, parent);

        // Destroying parent should orphan child
        w.destroy_entity(parent);
        assert_eq!(w.parent_of(child), None);
    }

    #[test]
    fn test_destroy_child_cleans_parent() {
        let mut w = World::new();
        let parent = w.spawn("parent", IVec2 { x: 0, y: 0 }, Team { id: 0 }, 100, 0);
        let child = w.spawn("child", IVec2 { x: 1, y: 1 }, Team { id: 0 }, 100, 0);

        w.set_parent(child, parent);

        // Destroying child should remove from parent's children
        w.destroy_entity(child);
        assert!(w.children_of(parent).is_empty());
    }

    #[test]
    fn test_set_name() {
        let mut w = World::new();
        let e = w.spawn("original", IVec2 { x: 0, y: 0 }, Team { id: 0 }, 100, 0);

        assert_eq!(w.name(e), Some("original"));
        w.set_name(e, "renamed".to_string());
        assert_eq!(w.name(e), Some("renamed"));
    }

    #[test]
    fn test_set_name_nonexistent() {
        let mut w = World::new();
        // Should not panic
        w.set_name(999, "ghost".to_string());
        assert!(w.name(999).is_none());
    }

    #[test]
    fn test_reparent_moves_child() {
        let mut w = World::new();
        let a = w.spawn("a", IVec2 { x: 0, y: 0 }, Team { id: 0 }, 100, 0);
        let b = w.spawn("b", IVec2 { x: 1, y: 1 }, Team { id: 0 }, 100, 0);
        let child = w.spawn("child", IVec2 { x: 2, y: 2 }, Team { id: 0 }, 100, 0);

        w.set_parent(child, a);
        assert_eq!(w.children_of(a), &[child]);

        // Reparent to b
        w.set_parent(child, b);
        assert!(w.children_of(a).is_empty());
        assert_eq!(w.children_of(b), &[child]);
        assert_eq!(w.parent_of(child), Some(b));
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
