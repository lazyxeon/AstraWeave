//! One anchored box. Health is the bond strength. The break impulse is an impact.

use astraweave_destruction::command::{CommandKind, Impact, SpawnBond, SpawnNode};
use astraweave_destruction::{Command, DestructionCommandError, DestructionWorld, FractureFlags};
use astraweave_math::Fixed64;

pub struct BreakStage {
    world: DestructionWorld,
    anchor_id: u64,
}

pub struct IntactBox {
    pub node_id: u64,
    pub health: i64,
    pub impulse: i64,
    pub half: [i64; 3],
}

pub struct BoxPiece {
    pub id: u64,
    pub pos_raw: [i64; 3],
    pub half_raw: [i64; 3],
    pub mass_raw: i64,
}

pub struct BreakHit {
    /// Canonical log, including the impact that was just applied.
    pub log_bytes: Vec<u8>,
    pub pieces: Vec<BoxPiece>,
}

impl BreakStage {
    pub fn new(seed: u64) -> Result<Self, DestructionCommandError> {
        let mut world = DestructionWorld::new(seed, 0);
        let anchor_id = next_node_id(&world);
        world.enqueue(Command {
            tick: world.tick_index(),
            seq: 0,
            entity_id: 0,
            kind: CommandKind::SpawnNode(SpawnNode {
                structure: 0,
                mass: Fixed64::from_int(1),
                pos: [Fixed64::ZERO; 3],
                half: [Fixed64::from_int(1); 3],
                flags: FractureFlags::ANCHOR.bits(),
            }),
        })?;
        world.tick();
        Ok(Self { world, anchor_id })
    }

    pub fn spawn_box(
        &mut self,
        pos: [i64; 3],
        half: [i64; 3],
        mass: i64,
        health: i64,
        impulse: i64,
    ) -> Result<IntactBox, DestructionCommandError> {
        let node_id = next_node_id(&self.world);
        let structure = node_id;
        let tick = self.world.tick_index();
        self.world.enqueue(Command {
            tick,
            seq: 0,
            entity_id: structure,
            kind: CommandKind::SpawnNode(SpawnNode {
                structure,
                mass: Fixed64::from_int(mass),
                pos: whole(pos),
                half: whole(half),
                flags: 0,
            }),
        })?;
        self.world.enqueue(Command {
            tick,
            seq: 0,
            entity_id: structure,
            kind: CommandKind::SpawnBond(SpawnBond {
                a: self.anchor_id,
                b: node_id,
                strength: Fixed64::from_int(health),
                area: Fixed64::from_int(1),
            }),
        })?;
        self.world.tick();
        Ok(IntactBox {
            node_id,
            health,
            impulse,
            half,
        })
    }

    /// Apply the stored break impulse to `node_id`. Pieces are the planar children.
    pub fn strike(
        &mut self,
        node_id: u64,
        impulse: i64,
    ) -> Result<BreakHit, DestructionCommandError> {
        let node = self
            .world
            .graph()
            .node(node_id)
            .ok_or(DestructionCommandError::ZeroExtent)?;
        let structure = node.structure;
        let point = node.pos;
        let tick = self.world.tick_index();
        self.world.enqueue(Command {
            tick,
            seq: 0,
            entity_id: structure,
            kind: CommandKind::Impact(Impact {
                point,
                impulse: Fixed64::from_int(impulse),
            }),
        })?;
        self.world.tick();
        Ok(BreakHit {
            log_bytes: self.world.persist_bytes(),
            pieces: children_of(&self.world, node_id, self.anchor_id),
        })
    }

    pub fn bond_strength(&self, node_id: u64) -> Option<i64> {
        self.world
            .graph()
            .bonds
            .iter()
            .find(|bond| bond.a == node_id || bond.b == node_id)
            .map(|bond| bond.strength.raw() / Fixed64::SCALE)
    }
}

fn next_node_id(world: &DestructionWorld) -> u64 {
    world.graph().nodes.len() as u64 + 1
}

fn whole(value: [i64; 3]) -> [Fixed64; 3] {
    [
        Fixed64::from_int(value[0]),
        Fixed64::from_int(value[1]),
        Fixed64::from_int(value[2]),
    ]
}

fn children_of(world: &DestructionWorld, parent: u64, anchor: u64) -> Vec<BoxPiece> {
    let Some(structure) = world.graph().node(parent).map(|node| node.structure) else {
        return Vec::new();
    };
    let mut pieces = Vec::new();
    for node in &world.graph().nodes {
        if node.id == parent || node.id == anchor || node.structure != structure {
            continue;
        }
        if !node.flags.contains(FractureFlags::DYNAMIC) {
            continue;
        }
        pieces.push(BoxPiece {
            id: node.id,
            pos_raw: node.pos.map(Fixed64::raw),
            half_raw: node.half.map(Fixed64::raw),
            mass_raw: node.mass.raw(),
        });
    }
    pieces.sort_by_key(|piece| piece.id);
    pieces
}

pub fn fixed_to_f32(raw: i64) -> f32 {
    (raw as f64 / Fixed64::SCALE as f64) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec_stage(impulse: i64) -> (BreakStage, IntactBox, BreakHit) {
        let mut stage = BreakStage::new(7).expect("anchor");
        let intact = stage
            .spawn_box([-1, 1, 2], [1, 1, 1], 3, 50, impulse)
            .expect("spawn");
        let hit = stage
            .strike(intact.node_id, intact.impulse)
            .expect("strike");
        (stage, intact, hit)
    }

    #[test]
    fn health_is_the_bond_strength() {
        let mut stage = BreakStage::new(7).expect("anchor");
        let intact = stage
            .spawn_box([-1, 1, 2], [1, 1, 1], 3, 50, 12)
            .expect("spawn");
        assert_eq!(stage.bond_strength(intact.node_id), Some(intact.health));
        assert!(stage
            .strike(intact.node_id, 12)
            .expect("weak")
            .pieces
            .is_empty());
    }

    #[test]
    fn break_command_bytes_are_stable_and_the_cut_is_two_boxes() {
        let (first_stage, intact, first) = spec_stage(50);
        let (_second_stage, _, second) = spec_stage(50);
        assert_eq!(first.log_bytes, second.log_bytes);
        assert_eq!(first.pieces.len(), 2);
        assert_ne!(first.pieces[0].id, first.pieces[1].id);
        assert_ne!(first.pieces[0].pos_raw[0], first.pieces[1].pos_raw[0]);
        assert_eq!(
            first.pieces[0].half_raw[0].saturating_add(first.pieces[1].half_raw[0]),
            Fixed64::from_int(intact.half[0]).raw()
        );
        let weak = spec_stage(12);
        assert!(weak.2.pieces.is_empty());
        assert_ne!(first.log_bytes, weak.2.log_bytes);
        let _ = first_stage;
    }
}
