//! Traversal against terrain that can be dug, fractured, or collapsed.
//!
//! Samples are a fixed 3×3 of column heights. [`deformed_step`] does not allocate.
//! Geometry notices and climb flags come from `astraweave-destruction` (PR #243):
//! [`GeometryNotice`] and [`allows_climb`].

pub use astraweave_destruction::{allows_climb, FractureFlags, GeometryNotice};
use glam::Vec3;

use crate::detmath;
use crate::traversal::{step, ProbeFrame, TraversalBody, TraversalInput, TraversalMode, TICK_DT};

/// One-voxel lips are ramps. Taller faces stay walls.
pub const SEAM_VOXELS: i32 = 1;

/// Nine column tops around the cell under the feet. Index `[dz + 1][dx + 1]`.
#[derive(Clone, Copy, Debug)]
pub struct ColumnStamp {
    pub x: i32,
    pub z: i32,
    pub heights: [[i32; 3]; 3],
}

impl ColumnStamp {
    pub fn from_samples(x: i32, z: i32, mut sample: impl FnMut(i32, i32) -> i32) -> Self {
        let mut heights = [[0; 3]; 3];
        for dz in 0..3 {
            for dx in 0..3 {
                heights[dz][dx] = sample(x + dx as i32 - 1, z + dz as i32 - 1);
            }
        }
        Self { x, z, heights }
    }

    pub fn center(self) -> i32 {
        self.heights[1][1]
    }
}

pub fn cell_in_notice(notice: &GeometryNotice, cell: [i32; 3]) -> bool {
    cell[0] >= notice.x0
        && cell[0] <= notice.x1
        && cell[1] >= notice.y0
        && cell[1] <= notice.y1
        && cell[2] >= notice.z0
        && cell[2] <= notice.z1
}

/// Average a normal from the 3×3, ignoring height jumps above one voxel.
pub fn smooth_normal(stamp: &ColumnStamp) -> Vec3 {
    let center = stamp.center();
    let mut limited = stamp.heights;
    for row in &mut limited {
        for height in row.iter_mut() {
            let delta = (*height - center).clamp(-SEAM_VOXELS, SEAM_VOXELS);
            *height = center + delta;
        }
    }
    let dz = (limited[2][0] + 2 * limited[2][1] + limited[2][2])
        - (limited[0][0] + 2 * limited[0][1] + limited[0][2]);
    let dx = (limited[0][2] + 2 * limited[1][2] + limited[2][2])
        - (limited[0][0] + 2 * limited[1][0] + limited[2][0]);
    let len2 = (dx * dx + dz * dz) as f32 + 64.0;
    let inv = 1.0 / detmath::sqrt(len2);
    Vec3::new(-(dx as f32) * inv, 8.0 * inv, -(dz as f32) * inv)
}

fn blend_seam(from: f32, to: f32, t: f32) -> f32 {
    let delta = to - from;
    if delta.abs() <= SEAM_VOXELS as f32 + 1e-4 {
        from + delta * t
    } else {
        from
    }
}

/// Floor height. A one-voxel neighbor is a ramp. A taller face does not snag the probe.
pub fn seam_floor(stamp: &ColumnStamp, x: f32, z: f32) -> f32 {
    let fx = (x - stamp.x as f32).clamp(0.0, 1.0);
    let fz = (z - stamp.z as f32).clamp(0.0, 1.0);
    let h00 = stamp.heights[1][1] as f32;
    let h10 = stamp.heights[1][2] as f32;
    let h01 = stamp.heights[2][1] as f32;
    let h11 = stamp.heights[2][2] as f32;
    let along_x0 = blend_seam(h00, h10, fx);
    let along_x1 = blend_seam(h01, h11, fx);
    blend_seam(along_x0, along_x1, fz)
}

fn stamp_relief(stamp: &ColumnStamp) -> i32 {
    let center = stamp.center();
    let mut relief = 0;
    for row in &stamp.heights {
        for height in row {
            relief = relief.max((height - center).abs());
        }
    }
    relief
}

pub fn is_micro_seam(stamp: &ColumnStamp) -> bool {
    let center = stamp.center();
    for row in &stamp.heights {
        for height in row {
            if (height - center).abs() > SEAM_VOXELS {
                return false;
            }
        }
    }
    true
}

/// One movement tick on deformable ground.
///
/// `anchor` is the voxel the hands are on. A [`GeometryNotice`] that covers it
/// detaches the climb and keeps the velocity from the previous tick.
pub fn deformed_step(
    body: &mut TraversalBody,
    input: TraversalInput,
    stamp: &ColumnStamp,
    notices: &[GeometryNotice],
    anchor: &mut [i32; 3],
    attached: &mut bool,
    climb_flags: FractureFlags,
    tick: u64,
) {
    let destroyed = notices.iter().any(|notice| cell_in_notice(notice, *anchor));
    if *attached && destroyed {
        *attached = false;
        let keep_x = body.vel.x;
        let keep_y = body.vel.y;
        let keep_z = body.vel.z;
        body.mode = TraversalMode::Air;
        let mut probe = ProbeFrame::plane(body.pos.y - 8.0);
        probe.support = false;
        step(
            body,
            TraversalInput {
                yaw: input.yaw,
                ..TraversalInput::default()
            },
            &probe,
            tick,
        );
        body.vel.x = keep_x;
        body.vel.y = keep_y;
        body.vel.z = keep_z;
        body.mode = TraversalMode::Air;
        return;
    }

    let floor = seam_floor(stamp, body.pos.x, body.pos.z);
    let normal = smooth_normal(stamp);
    let old_y = body.pos.y;
    let old_vx = body.vel.x;
    let old_vz = body.vel.z;
    let mut probe = ProbeFrame::plane(floor);
    probe.normal_y = normal.y;
    probe.support = true;
    let climb_ok = allows_climb(climb_flags);
    let relief = stamp_relief(stamp);
    if relief > SEAM_VOXELS && climb_ok && input.climb {
        probe.climb_point = Some(Vec3::new(body.pos.x, floor + 0.5, body.pos.z));
        probe.climb_normal = Some(normal);
    }
    step(body, input, &probe, tick);
    if body.mode == TraversalMode::Climb {
        *attached = true;
        *anchor = [stamp.x, stamp.center(), stamp.z];
    }
    let dropped = old_y - floor > (SEAM_VOXELS as f32 + 0.25);
    if dropped && body.mode != TraversalMode::Climb {
        body.mode = TraversalMode::Air;
        body.vel.x = old_vx;
        body.vel.z = old_vz;
        if body.vel.y > 0.0 {
            body.vel.y = 0.0;
        }
    }
    let _ = TICK_DT;
}

pub struct DeformedReplay {
    pub bytes: Vec<u8>,
    pub climbed: bool,
    pub detached: bool,
    pub momentum_kept: bool,
    pub crossed_seam: bool,
}

/// Walk, cross a one-voxel lip, climb, then lose the handhold to a brush.
/// The bytes are the replay case. The flags prove the run actually deformed.
pub fn deformed_replay() -> DeformedReplay {
    use astraweave_destruction::command::{
        Brush, BrushOp, BrushShape, CommandKind, DestructionCommand,
    };
    use astraweave_destruction::DestructionWorld;

    let mut world = DestructionWorld::new(0x7A4E_0001, 4);
    let mut body = TraversalBody::default();
    let surface = world.terrain().surface_y(0, 0) as f32;
    body.pos = Vec3::new(0.2, surface, 0.2);
    let mut anchor = [0, surface as i32, 0];
    let mut attached = false;
    let mut notices = [GeometryNotice {
        tick: 0,
        x0: 0,
        y0: 0,
        z0: 0,
        x1: 0,
        y1: 0,
        z1: 0,
        reason: 0,
    }; 8];
    let mut out = Vec::with_capacity(100 * 32);
    let mut climbed = false;
    let mut detached = false;
    let mut momentum_kept = false;
    let mut min_x_after_seam = body.pos.x;
    let mut saw_seam = false;
    let mut scheduled_break = false;
    for tick in 0..100u64 {
        if tick == 8 {
            let _ = world.enqueue(DestructionCommand {
                tick,
                seq: 0,
                entity_id: 1,
                kind: CommandKind::Brush(Brush {
                    shape: BrushShape::Box,
                    op: BrushOp::Add,
                    origin: [2, 4, 0],
                    radius: 0,
                    half_extents: [0, 0, 0],
                    strength: 1,
                }),
            });
        }
        if tick == 12 {
            let _ = world.enqueue(DestructionCommand {
                tick,
                seq: 0,
                entity_id: 2,
                kind: CommandKind::Brush(Brush {
                    shape: BrushShape::Box,
                    op: BrushOp::Add,
                    origin: [4, 4, 0],
                    radius: 0,
                    half_extents: [0, 0, 0],
                    strength: 4,
                }),
            });
        }
        if attached && !scheduled_break {
            scheduled_break = true;
            let _ = world.enqueue(DestructionCommand {
                tick,
                seq: 0,
                entity_id: 3,
                kind: CommandKind::Brush(Brush {
                    shape: BrushShape::Box,
                    op: BrushOp::Subtract,
                    origin: [anchor[0], anchor[1], anchor[2]],
                    radius: 0,
                    half_extents: [0, 1, 0],
                    strength: 3,
                }),
            });
        }
        world.tick();
        let count = world.drain_geometry_notices(&mut notices);
        let x = body.pos.x.round() as i32;
        let z = body.pos.z.round() as i32;
        let terrain = world.terrain();
        let stamp = ColumnStamp::from_samples(x, z, |sx, sz| terrain.surface_y(sx, sz));
        let keep_x = body.vel.x;
        let was_attached = attached;
        let input = if tick >= 45 {
            TraversalInput {
                climb: true,
                move_y: 1.0,
                ..TraversalInput::default()
            }
        } else {
            TraversalInput {
                move_y: 1.0,
                ..TraversalInput::default()
            }
        };
        deformed_step(
            &mut body,
            input,
            &stamp,
            &notices[..count],
            &mut anchor,
            &mut attached,
            FractureFlags::CLIMBABLE,
            tick,
        );
        if (16..36).contains(&tick)
            && body.pos.x > min_x_after_seam
            && body.mode != TraversalMode::Climb
        {
            min_x_after_seam = body.pos.x;
            saw_seam = true;
        }
        if body.mode == TraversalMode::Climb {
            climbed = true;
        }
        if was_attached && !attached {
            detached = true;
            momentum_kept = (body.vel.x - keep_x).abs() < 1e-5;
        }
        out.extend_from_slice(&tick.to_le_bytes());
        out.push(body.mode as u8);
        out.extend_from_slice(&body.pos.x.to_bits().to_le_bytes());
        out.extend_from_slice(&body.pos.y.to_bits().to_le_bytes());
        out.extend_from_slice(&body.pos.z.to_bits().to_le_bytes());
        out.extend_from_slice(&body.vel.x.to_bits().to_le_bytes());
        out.extend_from_slice(&body.vel.y.to_bits().to_le_bytes());
        out.extend_from_slice(&body.vel.z.to_bits().to_le_bytes());
        out.push(u8::from(attached));
    }
    DeformedReplay {
        bytes: out,
        climbed,
        detached,
        momentum_kept,
        crossed_seam: saw_seam && min_x_after_seam > 1.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use astraweave_destruction::FractureFlags;

    fn flat(height: i32) -> ColumnStamp {
        ColumnStamp {
            x: 0,
            z: 0,
            heights: [[height; 3]; 3],
        }
    }

    #[test]
    fn one_voxel_seam_ramps_instead_of_stopping() {
        let mut stamp = flat(4);
        stamp.heights[1][2] = 5;
        let mut body = TraversalBody::default();
        body.pos = Vec3::new(0.1, 4.0, 0.2);
        let mut anchor = [0, 4, 0];
        let mut attached = false;
        let mut last_x = body.pos.x;
        for tick in 0..20 {
            deformed_step(
                &mut body,
                TraversalInput {
                    move_y: 1.0,
                    ..TraversalInput::default()
                },
                &stamp,
                &[],
                &mut anchor,
                &mut attached,
                FractureFlags::CLIMBABLE,
                tick,
            );
            assert!(
                body.pos.x > last_x,
                "snagged on a one-voxel seam at tick {tick}"
            );
            last_x = body.pos.x;
            assert_ne!(body.mode, TraversalMode::Climb);
        }
        assert!(body.pos.y > 4.0 && body.pos.y <= 5.05, "y={}", body.pos.y);
    }

    #[test]
    fn destroyed_handhold_detaches_and_keeps_momentum() {
        let stamp = flat(4);
        let mut body = TraversalBody::default();
        body.mode = TraversalMode::Climb;
        body.pos = Vec3::new(1.0, 6.0, 0.0);
        body.vel = Vec3::new(1.5, 0.4, -0.25);
        let mut anchor = [1, 6, 0];
        let mut attached = true;
        let notice = GeometryNotice {
            tick: 3,
            x0: 1,
            y0: 5,
            z0: 0,
            x1: 1,
            y1: 7,
            z1: 0,
            reason: 1,
        };
        deformed_step(
            &mut body,
            TraversalInput {
                climb: true,
                move_y: 1.0,
                ..TraversalInput::default()
            },
            &stamp,
            &[notice],
            &mut anchor,
            &mut attached,
            FractureFlags::CLIMBABLE,
            3,
        );
        assert!(!attached);
        assert_eq!(body.mode, TraversalMode::Air);
        assert_eq!(body.vel.x.to_bits(), 1.5f32.to_bits());
        assert_eq!(body.vel.z.to_bits(), (-0.25f32).to_bits());
    }

    #[test]
    fn anti_climb_flag_refuses_the_face() {
        let mut stamp = flat(0);
        stamp.heights[1][2] = 8;
        let mut body = TraversalBody::default();
        body.pos = Vec3::new(0.2, 0.0, 0.0);
        let mut anchor = [0, 0, 0];
        let mut attached = false;
        let flags = FractureFlags::CLIMBABLE | FractureFlags::ANTI_CLIMB;
        assert!(!allows_climb(flags));
        deformed_step(
            &mut body,
            TraversalInput {
                climb: true,
                move_x: 1.0,
                ..TraversalInput::default()
            },
            &stamp,
            &[],
            &mut anchor,
            &mut attached,
            flags,
            0,
        );
        assert_ne!(body.mode, TraversalMode::Climb);
        assert!(!attached);
    }
}
