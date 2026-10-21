//! Runtime cuts and an offline bake. Cell order is the generation order.

use astraweave_core::{mix_sim_seed, SIM_SYSTEM_DESTRUCTION};
use astraweave_math::Fixed64;
use rand::RngCore;

use crate::graph::{Bond, PieceFlags, SupportNode};
use crate::rng::destruction_rng;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FractureCell {
    pub id: u64,
    pub site: [Fixed64; 3],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrefractureAsset {
    pub nodes: Vec<SupportNode>,
    pub bonds: Vec<Bond>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BreakRule {
    pub strength: Fixed64,
    pub min_chunk: u16,
    pub debris_class: u16,
    pub audio_tag: u16,
    pub vfx_tag: u16,
    pub flags: PieceFlags,
}

impl BreakRule {
    pub fn stone() -> Self {
        Self {
            strength: Fixed64::from_int(100),
            min_chunk: 1,
            debris_class: 1,
            audio_tag: 1,
            vfx_tag: 1,
            flags: PieceFlags::CLIMBABLE,
        }
    }
}

pub fn planar_children(parent: &SupportNode) -> [SupportNode; 2] {
    let mid = parent.pos[0];
    let mut left = parent.clone();
    let mut right = parent.clone();
    left.id = mix_sim_seed(parent.id, 0, SIM_SYSTEM_DESTRUCTION);
    right.id = mix_sim_seed(parent.id, 1, SIM_SYSTEM_DESTRUCTION);
    let shift = parent.half[0].mul_ratio(1, 2);
    left.pos[0] = mid.saturating_sub(shift);
    right.pos[0] = mid.saturating_add(shift);
    left.half[0] = shift;
    right.half[0] = shift;
    left.mass = parent.mass.mul_ratio(1, 2);
    right.mass = parent.mass.saturating_sub(left.mass);
    [left, right]
}

pub fn voronoi_cells(
    parent: &SupportNode,
    world_seed: u64,
    tick: u64,
    seq: u64,
    count: u32,
) -> Vec<FractureCell> {
    let mut rng = destruction_rng(world_seed, tick, parent.id, seq);
    let mut cells = Vec::with_capacity(count as usize);
    for index in 0..count {
        let site = [0, 1, 2].map(|axis| {
            let draw = rng.next_u64() % 1000;
            let t = Fixed64::from_ratio(draw as i64, 1000);
            let span = parent.half[axis].saturating_add(parent.half[axis]);
            parent.pos[axis]
                .saturating_sub(parent.half[axis])
                .saturating_add(span.mul(t))
        });
        cells.push(FractureCell {
            id: mix_sim_seed(parent.id, u64::from(index), SIM_SYSTEM_DESTRUCTION),
            site,
        });
    }
    cells
}

/// Nearest later cell, tie broken by the smaller id. A chain plus that link.
pub fn voronoi_bonds(cells: &[FractureCell], rule: BreakRule) -> Vec<(u64, u64, Fixed64)> {
    let mut bonds = Vec::new();
    for (index, cell) in cells.iter().enumerate() {
        if rule.flags.contains(PieceFlags::ANTI_DESTROY) {
            continue;
        }
        let mut best: Option<(Fixed64, u64)> = None;
        for other in cells.iter().skip(index + 1) {
            let delta = crate::graph::dist2(cell.site, other.site);
            let replace = match best {
                None => true,
                Some((best_dist, best_id)) => {
                    delta < best_dist || (delta == best_dist && other.id < best_id)
                }
            };
            if replace {
                best = Some((delta, other.id));
            }
        }
        if let Some((_, other)) = best {
            let (a, b) = if cell.id <= other {
                (cell.id, other)
            } else {
                (other, cell.id)
            };
            bonds.push((a, b, rule.strength));
        }
    }
    bonds.sort_by_key(|(a, b, _)| (*a, *b));
    bonds
}

pub fn bake_gate(world_seed: u64) -> PrefractureAsset {
    let parent = SupportNode {
        id: 1,
        structure: world_seed,
        mass: Fixed64::from_int(40),
        pos: [Fixed64::ZERO, Fixed64::from_int(2), Fixed64::ZERO],
        half: [
            Fixed64::from_int(2),
            Fixed64::from_int(1),
            Fixed64::from_int(1),
        ],
        flags: PieceFlags::CLIMBABLE,
        island: 0,
    };
    let children = planar_children(&parent);
    let bond = Bond {
        id: mix_sim_seed(parent.id, 2, SIM_SYSTEM_DESTRUCTION),
        a: children[0].id,
        b: children[1].id,
        strength: BreakRule::stone().strength,
        area: Fixed64::from_int(1),
        damage: Fixed64::ZERO,
        intact: true,
    };
    PrefractureAsset {
        nodes: children.to_vec(),
        bonds: vec![bond],
    }
}

pub fn bake_bytes(asset: &PrefractureAsset) -> Vec<u8> {
    let mut out = Vec::new();
    for node in &asset.nodes {
        out.extend_from_slice(&node.id.to_le_bytes());
        out.extend_from_slice(&node.mass.raw().to_le_bytes());
        for axis in node.pos {
            out.extend_from_slice(&axis.raw().to_le_bytes());
        }
        out.extend_from_slice(&node.flags.bits().to_le_bytes());
    }
    for bond in &asset.bonds {
        out.extend_from_slice(&bond.id.to_le_bytes());
        out.extend_from_slice(&bond.a.to_le_bytes());
        out.extend_from_slice(&bond.b.to_le_bytes());
        out.extend_from_slice(&bond.strength.raw().to_le_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parent() -> SupportNode {
        SupportNode {
            id: 9,
            structure: 1,
            mass: Fixed64::from_int(10),
            pos: [Fixed64::ZERO; 3],
            half: [Fixed64::from_int(2); 3],
            flags: PieceFlags::empty(),
            island: 0,
        }
    }

    #[test]
    fn planar_cut_is_stable() {
        let source = parent();
        let a = planar_children(&source);
        let b = planar_children(&source);
        assert_eq!(a, b);
        assert_ne!(a[0].id, a[1].id);
        assert_eq!(a[0].id, mix_sim_seed(source.id, 0, SIM_SYSTEM_DESTRUCTION));
        assert_eq!(a[1].id, mix_sim_seed(source.id, 1, SIM_SYSTEM_DESTRUCTION));
        let shift = source.half[0].mul_ratio(1, 2);
        assert_eq!(a[0].half[0], shift);
        assert_eq!(a[1].half[0], shift);
        assert_eq!(a[0].pos[0], source.pos[0].saturating_sub(shift));
        assert_eq!(a[1].pos[0], source.pos[0].saturating_add(shift));
        for axis in 1..3 {
            assert_eq!(a[0].pos[axis], source.pos[axis]);
            assert_eq!(a[1].pos[axis], source.pos[axis]);
            assert_eq!(a[0].half[axis], source.half[axis]);
            assert_eq!(a[1].half[axis], source.half[axis]);
        }
        assert_eq!(a[0].mass, source.mass.mul_ratio(1, 2));
        assert_eq!(a[1].mass, source.mass.saturating_sub(a[0].mass));
        assert_eq!(a[0].mass.saturating_add(a[1].mass), source.mass);
        assert_eq!(a[0].flags, source.flags);
        assert_eq!(a[1].flags, source.flags);
        assert_eq!(a[0].structure, source.structure);
        assert_eq!(a[1].structure, source.structure);
        assert_eq!(a[0].island, source.island);
        assert_eq!(a[1].island, source.island);
    }

    #[test]
    fn stone_break_strength_is_one_hundred() {
        assert_eq!(BreakRule::stone().strength, Fixed64::from_int(100));
    }

    #[test]
    fn voronoi_sites_follow_the_seed() {
        let a = voronoi_cells(&parent(), 4, 8, 1, 6);
        let b = voronoi_cells(&parent(), 4, 8, 1, 6);
        let other = voronoi_cells(&parent(), 4, 8, 2, 6);
        assert_eq!(a, b);
        assert_ne!(a, other);
        let bonds = voronoi_bonds(&a, BreakRule::stone());
        assert_eq!(bonds, voronoi_bonds(&a, BreakRule::stone()));
    }

    #[test]
    fn anti_destroy_material_emits_no_bonds() {
        let cells = voronoi_cells(&parent(), 1, 1, 1, 4);
        let mut rule = BreakRule::stone();
        rule.flags.insert(PieceFlags::ANTI_DESTROY);
        assert!(voronoi_bonds(&cells, rule).is_empty());
    }

    #[test]
    fn bake_is_stable() {
        assert_eq!(bake_bytes(&bake_gate(1)), bake_bytes(&bake_gate(1)));
    }

    #[test]
    fn prefracture_manifest_hash_is_locked() {
        use sha2::{Digest, Sha256};
        let digest = Sha256::digest(bake_bytes(&bake_gate(0xA2A0_0001)));
        let mut hex = String::new();
        for byte in digest {
            hex.push_str(&format!("{byte:02x}"));
        }
        assert_eq!(
            hex,
            "d652793ef4dde351df61496e401aef1cd2a18deebc54b448a139e4ddc6fb9909"
        );
    }
}
