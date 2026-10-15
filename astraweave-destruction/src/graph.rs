//! Support graph. Tier A is connectivity. Tier B is a fixed-iteration load push.

use astraweave_core::{mix_sim_seed, SIM_SYSTEM_DESTRUCTION};
use astraweave_math::Fixed64;
use bitflags::bitflags;

use crate::budget::{MAX_NODES_PER_TICK, STRESS_ITERS};

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct PieceFlags: u32 {
        const ANCHOR = 1 << 0;
        const ANTI_DESTROY = 1 << 1;
        const CLIMBABLE = 1 << 2;
        const ANTI_CLIMB = 1 << 3;
        const DYNAMIC = 1 << 4;
    }
}

/// `ANTI_CLIMB` wins. Nothing in the character controller reads this yet.
pub fn allows_climb(flags: PieceFlags) -> bool {
    flags.contains(PieceFlags::CLIMBABLE) && !flags.contains(PieceFlags::ANTI_CLIMB)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SupportNode {
    pub id: u64,
    pub structure: u64,
    pub mass: Fixed64,
    pub pos: [Fixed64; 3],
    pub half: [Fixed64; 3],
    pub flags: PieceFlags,
    pub island: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bond {
    pub id: u64,
    pub a: u64,
    pub b: u64,
    pub strength: Fixed64,
    pub area: Fixed64,
    pub damage: Fixed64,
    pub intact: bool,
}

#[derive(Clone, Debug, Default)]
pub struct Graph {
    pub nodes: Vec<SupportNode>,
    pub bonds: Vec<Bond>,
    pub dirty: bool,
    next_node: u64,
    next_bond: u64,
}

impl Graph {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            bonds: Vec::new(),
            dirty: false,
            next_node: 1,
            next_bond: 1,
        }
    }

    pub fn add_node(
        &mut self,
        structure: u64,
        mass: Fixed64,
        pos: [Fixed64; 3],
        half: [Fixed64; 3],
        flags: PieceFlags,
    ) -> u64 {
        let id = self.next_node;
        self.next_node += 1;
        self.nodes.push(SupportNode {
            id,
            structure,
            mass,
            pos,
            half,
            flags,
            island: 0,
        });
        self.nodes.sort_by_key(|node| node.id);
        self.dirty = true;
        id
    }

    pub fn add_bond(&mut self, a: u64, b: u64, strength: Fixed64, area: Fixed64) -> u64 {
        let id = self.next_bond;
        self.next_bond += 1;
        let (a, b) = if a <= b { (a, b) } else { (b, a) };
        self.bonds.push(Bond {
            id,
            a,
            b,
            strength,
            area,
            damage: Fixed64::ZERO,
            intact: true,
        });
        self.bonds.sort_by_key(|bond| bond.id);
        self.dirty = true;
        id
    }

    pub fn node(&self, id: u64) -> Option<&SupportNode> {
        self.nodes.iter().find(|node| node.id == id)
    }

    pub fn node_mut(&mut self, id: u64) -> Option<&mut SupportNode> {
        self.nodes.iter_mut().find(|node| node.id == id)
    }

    pub fn bond_mut(&mut self, id: u64) -> Option<&mut Bond> {
        self.bonds.iter_mut().find(|bond| bond.id == id)
    }

    pub fn detach(&mut self, bond_id: u64) -> bool {
        let Some(bond) = self.bond_mut(bond_id) else {
            return false;
        };
        if !bond.intact {
            return false;
        }
        bond.intact = false;
        self.dirty = true;
        true
    }

    pub fn set_anchor(&mut self, node_id: u64, anchored: bool) -> bool {
        let Some(node) = self.node_mut(node_id) else {
            return false;
        };
        if anchored {
            node.flags.insert(PieceFlags::ANCHOR);
        } else {
            node.flags.remove(PieceFlags::ANCHOR);
        }
        self.dirty = true;
        true
    }

    pub fn add_damage(&mut self, bond_id: u64, amount: Fixed64) {
        if let Some(bond) = self.bond_mut(bond_id) {
            if bond.intact {
                bond.damage = bond.damage.saturating_add(amount);
                self.dirty = true;
            }
        }
    }

    /// Bonds whose pushed load exceeds remaining strength after `iters` passes.
    pub fn overloaded_bonds(&self, iters: u32, max_nodes: usize) -> Vec<u64> {
        let count = self.nodes.len().min(max_nodes);
        let mut bond_load = vec![Fixed64::ZERO; self.bonds.len()];
        let mut relieved = vec![false; self.nodes.len()];
        for (index, node) in self.nodes.iter().enumerate() {
            if node.flags.contains(PieceFlags::ANCHOR) {
                relieved[index] = true;
            }
        }
        for _ in 0..iters {
            let receivers = relieved.clone();
            // Several parallel arrays are indexed together.
            #[allow(clippy::needless_range_loop)]
            for index in 0..count {
                if relieved[index] {
                    continue;
                }
                let node_id = self.nodes[index].id;
                let mut incident = Vec::new();
                for (bond_index, bond) in self.bonds.iter().enumerate() {
                    if !bond.intact {
                        continue;
                    }
                    let other = if bond.a == node_id {
                        bond.b
                    } else if bond.b == node_id {
                        bond.a
                    } else {
                        continue;
                    };
                    let Some(other_index) = self.nodes.iter().position(|node| node.id == other)
                    else {
                        continue;
                    };
                    if receivers[other_index] {
                        incident.push(bond_index);
                    }
                }
                if incident.is_empty() {
                    continue;
                }
                incident.sort_by_key(|bond_index| self.bonds[*bond_index].id);
                let total_area = incident.iter().fold(Fixed64::ZERO, |sum, bond_index| {
                    sum.saturating_add(self.bonds[*bond_index].area)
                });
                let carried = self.nodes[index].mass;
                let mut remaining = carried;
                let last = incident.len() - 1;
                for (nth, bond_index) in incident.iter().copied().enumerate() {
                    let given = if nth == last {
                        remaining
                    } else {
                        let portion = mul_div(carried, self.bonds[bond_index].area, total_area);
                        remaining = remaining.saturating_sub(portion);
                        portion
                    };
                    bond_load[bond_index] = bond_load[bond_index].saturating_add(given);
                }
                relieved[index] = true;
            }
        }
        let mut broken = Vec::new();
        for (bond_index, bond) in self.bonds.iter().enumerate() {
            if !bond.intact {
                continue;
            }
            let remain = bond.strength.saturating_sub(bond.damage);
            if bond_load[bond_index] > remain {
                broken.push(bond.id);
            }
        }
        broken.sort_unstable();
        broken
    }

    pub fn solve(&mut self) {
        let broken = self.overloaded_bonds(STRESS_ITERS, MAX_NODES_PER_TICK);
        for bond_id in broken {
            self.detach(bond_id);
        }
        self.assign_islands();
        self.dirty = self.nodes.len() > MAX_NODES_PER_TICK;
    }

    pub fn assign_islands(&mut self) {
        let mut seen = vec![false; self.nodes.len()];
        let mut order: Vec<usize> = (0..self.nodes.len()).collect();
        order.sort_by_key(|index| self.nodes[*index].id);
        for start in order {
            if seen[start] {
                continue;
            }
            let mut stack = vec![start];
            seen[start] = true;
            let mut members = Vec::new();
            while let Some(index) = stack.pop() {
                members.push(index);
                let node_id = self.nodes[index].id;
                let mut neighbors = Vec::new();
                for bond in &self.bonds {
                    if !bond.intact {
                        continue;
                    }
                    let other = if bond.a == node_id {
                        Some(bond.b)
                    } else if bond.b == node_id {
                        Some(bond.a)
                    } else {
                        None
                    };
                    if let Some(other) = other {
                        if let Some(other_index) =
                            self.nodes.iter().position(|node| node.id == other)
                        {
                            neighbors.push((bond.id, other_index));
                        }
                    }
                }
                neighbors.sort_by_key(|(bond_id, _)| *bond_id);
                for (_, other_index) in neighbors {
                    if !seen[other_index] {
                        seen[other_index] = true;
                        stack.push(other_index);
                    }
                }
            }
            let supported = members
                .iter()
                .any(|index| self.nodes[*index].flags.contains(PieceFlags::ANCHOR));
            let lowest = members
                .iter()
                .map(|index| self.nodes[*index].id)
                .min()
                .unwrap_or(0);
            let structure = self.nodes[members[0]].structure;
            let island = mix_sim_seed(structure, lowest, SIM_SYSTEM_DESTRUCTION);
            for index in members {
                self.nodes[index].island = island;
                if supported || self.nodes[index].flags.contains(PieceFlags::ANTI_DESTROY) {
                    self.nodes[index].flags.remove(PieceFlags::DYNAMIC);
                } else {
                    self.nodes[index].flags.insert(PieceFlags::DYNAMIC);
                }
            }
        }
    }

    pub fn closest_node(&self, point: [Fixed64; 3], structure: u64) -> Option<u64> {
        let mut best: Option<(Fixed64, u64)> = None;
        for node in &self.nodes {
            if structure != 0 && node.structure != structure {
                continue;
            }
            let dist = dist2(node.pos, point);
            let replace = match best {
                None => true,
                Some((best_dist, best_id)) => {
                    dist < best_dist || (dist == best_dist && node.id < best_id)
                }
            };
            if replace {
                best = Some((dist, node.id));
            }
        }
        best.map(|(_, id)| id)
    }

    pub fn bonds_of(&self, node_id: u64) -> Vec<u64> {
        let mut ids: Vec<u64> = self
            .bonds
            .iter()
            .filter(|bond| bond.intact && (bond.a == node_id || bond.b == node_id))
            .map(|bond| bond.id)
            .collect();
        ids.sort_unstable();
        ids
    }
}

pub fn mul_div(value: Fixed64, numer: Fixed64, denom: Fixed64) -> Fixed64 {
    if denom.raw() == 0 {
        return Fixed64::ZERO;
    }
    let wide = i128::from(value.raw()) * i128::from(numer.raw());
    let quotient = wide.div_euclid(i128::from(denom.raw()));
    if quotient > i128::from(i64::MAX) {
        Fixed64::from_raw(i64::MAX)
    } else if quotient < i128::from(i64::MIN) {
        Fixed64::from_raw(i64::MIN)
    } else {
        Fixed64::from_raw(quotient as i64)
    }
}

pub fn dist2(a: [Fixed64; 3], b: [Fixed64; 3]) -> Fixed64 {
    let mut sum = Fixed64::ZERO;
    for axis in 0..3 {
        let delta = a[axis].saturating_sub(b[axis]);
        sum = sum.saturating_add(delta.mul(delta));
    }
    sum
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gate() -> Graph {
        let mut graph = Graph::new();
        let mass = Fixed64::from_int(20);
        let half = [Fixed64::from_int(1); 3];
        graph.add_node(
            1,
            mass,
            [Fixed64::from_int(-2), Fixed64::from_int(1), Fixed64::ZERO],
            half,
            PieceFlags::ANCHOR | PieceFlags::CLIMBABLE,
        );
        graph.add_node(
            1,
            mass,
            [Fixed64::from_int(2), Fixed64::from_int(1), Fixed64::ZERO],
            half,
            PieceFlags::ANCHOR | PieceFlags::CLIMBABLE,
        );
        graph.add_node(
            1,
            Fixed64::from_int(80),
            [Fixed64::ZERO, Fixed64::from_int(3), Fixed64::ZERO],
            half,
            PieceFlags::ANTI_CLIMB,
        );
        graph.add_bond(1, 3, Fixed64::from_int(100), Fixed64::from_int(1));
        graph.add_bond(2, 3, Fixed64::from_int(100), Fixed64::from_int(1));
        graph
    }

    #[test]
    fn intact_gate_holds_and_matches_a_long_solve() {
        let graph = gate();
        let short = graph.overloaded_bonds(STRESS_ITERS, MAX_NODES_PER_TICK);
        let long = graph.overloaded_bonds(64, MAX_NODES_PER_TICK);
        assert!(short.is_empty());
        assert_eq!(short, long);
    }

    #[test]
    fn damaged_lintel_breaks_the_same_bonds_at_8_and_64() {
        let mut graph = gate();
        graph.add_damage(1, Fixed64::from_int(80));
        graph.add_damage(2, Fixed64::from_int(80));
        let short = graph.overloaded_bonds(STRESS_ITERS, MAX_NODES_PER_TICK);
        let long = graph.overloaded_bonds(64, MAX_NODES_PER_TICK);
        assert_eq!(short, vec![1, 2]);
        assert_eq!(short, long);
    }

    #[test]
    fn unsupported_island_becomes_dynamic_and_keeps_a_stable_id() {
        let mut graph = gate();
        graph.detach(1);
        graph.detach(2);
        graph.assign_islands();
        let lintel = graph.node(3).expect("lintel");
        assert!(lintel.flags.contains(PieceFlags::DYNAMIC));
        let again = {
            let mut copy = gate();
            copy.detach(1);
            copy.detach(2);
            copy.assign_islands();
            copy.node(3).expect("lintel").island
        };
        assert_eq!(lintel.island, again);
        assert!(!allows_climb(lintel.flags));
        assert!(allows_climb(graph.node(1).expect("post").flags));
    }

    #[test]
    fn anti_destroy_island_stays_kinematic() {
        let mut graph = gate();
        graph
            .node_mut(3)
            .expect("lintel")
            .flags
            .insert(PieceFlags::ANTI_DESTROY);
        graph.detach(1);
        graph.detach(2);
        graph.assign_islands();
        assert!(!graph
            .node(3)
            .expect("lintel")
            .flags
            .contains(PieceFlags::DYNAMIC));
    }

    #[test]
    fn climb_flag_loses_to_anti_climb() {
        let both = PieceFlags::CLIMBABLE | PieceFlags::ANTI_CLIMB;
        assert!(!allows_climb(both));
    }

    #[test]
    fn chain_deeper_than_eight_is_only_partly_pushed() {
        let mut graph = Graph::new();
        let half = [Fixed64::from_int(1); 3];
        graph.add_node(
            7,
            Fixed64::from_int(1),
            [Fixed64::ZERO; 3],
            half,
            PieceFlags::ANCHOR,
        );
        for index in 1..16 {
            graph.add_node(
                7,
                Fixed64::from_int(50),
                [Fixed64::from_int(index), Fixed64::ZERO, Fixed64::ZERO],
                half,
                PieceFlags::empty(),
            );
            graph.add_bond(
                index as u64,
                index as u64 + 1,
                Fixed64::from_int(1),
                Fixed64::from_int(1),
            );
        }
        let short = graph.overloaded_bonds(STRESS_ITERS, MAX_NODES_PER_TICK);
        let long = graph.overloaded_bonds(64, MAX_NODES_PER_TICK);
        assert!(short.len() < long.len());
    }
}
