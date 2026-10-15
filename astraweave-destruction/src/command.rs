//! Ordered destruction commands. The log is the replay, save, and network payload.

use astraweave_math::Fixed64;
use serde::{Deserialize, Serialize};

use crate::budget::MAX_BRUSH_CELLS;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DestructionCommand {
    pub tick: u64,
    pub seq: u32,
    pub entity_id: u64,
    pub kind: CommandKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommandKind {
    Impact(Impact),
    Explosion(Explosion),
    Brush(Brush),
    Detach(Detach),
    Anchor(Anchor),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Impact {
    pub point: [Fixed64; 3],
    pub impulse: Fixed64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Explosion {
    pub center: [Fixed64; 3],
    pub radius: Fixed64,
    pub impulse: Fixed64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BrushShape {
    Sphere,
    Box,
    Capsule,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BrushOp {
    Subtract,
    Add,
    Smooth,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Brush {
    pub shape: BrushShape,
    pub op: BrushOp,
    pub origin: [i32; 3],
    pub radius: u16,
    pub half_extents: [u16; 3],
    pub strength: i16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Detach {
    pub bond_id: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Anchor {
    pub node_id: u64,
    pub anchored: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandError {
    ZeroExtent,
    BrushTooLarge,
    LogFull,
}

pub fn validate(cmd: &DestructionCommand) -> Result<(), CommandError> {
    match cmd.kind {
        CommandKind::Brush(brush) => validate_brush(brush),
        CommandKind::Explosion(explosion) if explosion.radius.raw() <= 0 => {
            Err(CommandError::ZeroExtent)
        }
        _ => Ok(()),
    }
}

fn validate_brush(brush: Brush) -> Result<(), CommandError> {
    let cells = match brush.shape {
        BrushShape::Sphere | BrushShape::Capsule => {
            let d = u32::from(brush.radius).saturating_mul(2).saturating_add(1);
            d.saturating_mul(d).saturating_mul(d)
        }
        BrushShape::Box => {
            let x = u32::from(brush.half_extents[0])
                .saturating_mul(2)
                .saturating_add(1);
            let y = u32::from(brush.half_extents[1])
                .saturating_mul(2)
                .saturating_add(1);
            let z = u32::from(brush.half_extents[2])
                .saturating_mul(2)
                .saturating_add(1);
            x.saturating_mul(y).saturating_mul(z)
        }
    };
    if cells == 0
        || (brush.radius == 0 && matches!(brush.shape, BrushShape::Sphere | BrushShape::Capsule))
    {
        return Err(CommandError::ZeroExtent);
    }
    if cells > MAX_BRUSH_CELLS {
        return Err(CommandError::BrushTooLarge);
    }
    Ok(())
}

pub fn command_ord(cmd: &DestructionCommand) -> (u64, u32, u64) {
    (cmd.tick, cmd.seq, cmd.entity_id)
}

pub fn push_command(out: &mut Vec<u8>, cmd: &DestructionCommand) {
    out.extend_from_slice(&cmd.tick.to_le_bytes());
    out.extend_from_slice(&cmd.seq.to_le_bytes());
    out.extend_from_slice(&cmd.entity_id.to_le_bytes());
    match cmd.kind {
        CommandKind::Impact(impact) => {
            out.push(1);
            push_fixed3(out, impact.point);
            push_fixed(out, impact.impulse);
        }
        CommandKind::Explosion(explosion) => {
            out.push(2);
            push_fixed3(out, explosion.center);
            push_fixed(out, explosion.radius);
            push_fixed(out, explosion.impulse);
        }
        CommandKind::Brush(brush) => {
            out.push(3);
            out.push(match brush.shape {
                BrushShape::Sphere => 1,
                BrushShape::Box => 2,
                BrushShape::Capsule => 3,
            });
            out.push(match brush.op {
                BrushOp::Subtract => 1,
                BrushOp::Add => 2,
                BrushOp::Smooth => 3,
            });
            for value in brush.origin {
                out.extend_from_slice(&value.to_le_bytes());
            }
            out.extend_from_slice(&brush.radius.to_le_bytes());
            for value in brush.half_extents {
                out.extend_from_slice(&value.to_le_bytes());
            }
            out.extend_from_slice(&brush.strength.to_le_bytes());
        }
        CommandKind::Detach(detach) => {
            out.push(4);
            out.extend_from_slice(&detach.bond_id.to_le_bytes());
        }
        CommandKind::Anchor(anchor) => {
            out.push(5);
            out.extend_from_slice(&anchor.node_id.to_le_bytes());
            out.push(u8::from(anchor.anchored));
        }
    }
}

fn push_fixed(out: &mut Vec<u8>, value: Fixed64) {
    out.extend_from_slice(&value.raw().to_le_bytes());
}

fn push_fixed3(out: &mut Vec<u8>, value: [Fixed64; 3]) {
    for axis in value {
        push_fixed(out, axis);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn impact(seq: u32, entity: u64) -> DestructionCommand {
        DestructionCommand {
            tick: 1,
            seq,
            entity_id: entity,
            kind: CommandKind::Impact(Impact {
                point: [Fixed64::ZERO; 3],
                impulse: Fixed64::from_int(1),
            }),
        }
    }

    #[test]
    fn sort_is_tick_then_seq_then_entity() {
        let mut cmds = vec![impact(2, 1), impact(1, 9), impact(1, 2)];
        cmds[0].tick = 2;
        cmds.sort_by_key(command_ord);
        assert_eq!(
            cmds.iter()
                .map(|c| (c.tick, c.seq, c.entity_id))
                .collect::<Vec<_>>(),
            vec![(1, 1, 2), (1, 1, 9), (2, 2, 1)]
        );
    }

    #[test]
    fn zero_radius_brush_is_rejected() {
        let cmd = DestructionCommand {
            tick: 0,
            seq: 0,
            entity_id: 0,
            kind: CommandKind::Brush(Brush {
                shape: BrushShape::Sphere,
                op: BrushOp::Subtract,
                origin: [0, 0, 0],
                radius: 0,
                half_extents: [0, 0, 0],
                strength: 1,
            }),
        };
        assert_eq!(validate(&cmd), Err(CommandError::ZeroExtent));
    }
}
