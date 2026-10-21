//! Fixed-tick weave evaluation. `weave_eval` does not allocate and does not store cells.
//!
//! The command queue is [`crate::mod_queue::WeaveScratch`]. This module turns
//! those commands into [`FieldWrite`]s.

use crate::field::{
    encode_write, FieldWrite, NullField, Shape, SimField, WriteOp, CELL_MM, MAX_FIELD_WRITES,
};
use crate::mod_queue::{
    WeaveCommand, WeaveNode, WeaveScratch, ISLAND_HALF_CELLS, KIND_DECAY, KIND_DENSITY,
    KIND_GRAVITY, KIND_HEAT, KIND_PHASE, KIND_VOLUME, KIND_WIND, MAX_WEAVE_COMMANDS,
};
use crate::templates::{builtin, writes_for, TEMPLATE_FREEZE};
use astraweave_nav::{NavChange, NavChangeKind};

pub const MAX_SINKS: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IceSheet {
    pub tick: u64,
    pub seq: u32,
    pub min_milli: [i32; 3],
    pub max_milli: [i32; 3],
}

#[derive(Clone)]
pub struct WeaveFrame {
    pub hazards: [astraweave_core::WeaveHazard; 16],
    pub hazard_count: u8,
    sheets: [IceSheet; MAX_SINKS],
    sheet_count: u8,
    nav: [NavChange; MAX_SINKS],
    nav_count: u8,
    writes: [FieldWrite; MAX_FIELD_WRITES],
    write_count: u8,
    next_seq: u32,
}

impl WeaveFrame {
    pub fn new() -> Self {
        Self {
            hazards: [astraweave_core::WeaveHazard::EMPTY; 16],
            hazard_count: 0,
            sheets: [IceSheet {
                tick: 0,
                seq: 0,
                min_milli: [0, 0, 0],
                max_milli: [0, 0, 0],
            }; MAX_SINKS],
            sheet_count: 0,
            nav: [NavChange::new(0, 0, [0, 0, 0], [0, 0, 0], NavChangeKind::Weave); MAX_SINKS],
            nav_count: 0,
            writes: [FieldWrite {
                seq: 0,
                shape: Shape::Point,
                origin: [0, 0, 0],
                end: [0, 0, 0],
                radius: 0,
                op: WriteOp::AddTemp {
                    delta: 0,
                    stack: false,
                },
            }; MAX_FIELD_WRITES],
            write_count: 0,
            next_seq: 1,
        }
    }

    pub fn sheet_count(&self) -> usize {
        self.sheet_count as usize
    }

    pub fn sheet(&self, index: usize) -> Option<IceSheet> {
        if index < self.sheet_count as usize {
            Some(self.sheets[index])
        } else {
            None
        }
    }

    pub fn nav_count(&self) -> usize {
        self.nav_count as usize
    }

    pub fn nav_change(&self, index: usize) -> Option<NavChange> {
        if index < self.nav_count as usize {
            Some(self.nav[index])
        } else {
            None
        }
    }

    pub fn write_count(&self) -> usize {
        self.write_count as usize
    }

    pub fn write(&self, index: usize) -> Option<FieldWrite> {
        if index < self.write_count as usize {
            Some(self.writes[index])
        } else {
            None
        }
    }

    #[cfg(test)]
    fn writes_op_is_ice(&self) -> bool {
        self.writes[..self.write_count as usize]
            .iter()
            .any(|write| {
                matches!(
                    write.op,
                    WriteOp::SetPhase {
                        phase: crate::field::PHASE_ICE
                    }
                )
            })
    }
}

impl Default for WeaveFrame {
    fn default() -> Self {
        Self::new()
    }
}

/// Expand active templates into [`FieldWrite`]s and hand them to the field.
pub fn weave_eval(
    scratch: &WeaveScratch,
    frame: &mut WeaveFrame,
    field: &mut impl SimField,
    tick: u64,
) {
    frame.write_count = 0;
    let count = scratch.command_count();
    let mut order = [0u8; MAX_WEAVE_COMMANDS];
    for (i, slot) in order.iter_mut().enumerate().take(count) {
        *slot = i as u8;
    }
    for i in 1..count {
        let mut j = i;
        while j > 0
            && command_less(
                &scratch.command(order[j] as usize).expect("index"),
                &scratch.command(order[j - 1] as usize).expect("index"),
            )
        {
            order.swap(j, j - 1);
            j -= 1;
        }
    }
    for slot in order.iter().take(count) {
        let Some(command) = scratch.command(*slot as usize) else {
            continue;
        };
        if tick < command.start || tick >= command.until {
            continue;
        }
        push_command(scratch, frame, command, tick);
    }
    let n = frame.write_count as usize;
    let staged = frame.writes;
    field.apply_writes(&staged[..n]);
}

fn command_less(a: &WeaveCommand, b: &WeaveCommand) -> bool {
    (a.start, a.seq, a.caster, a.template, a.origin)
        < (b.start, b.seq, b.caster, b.template, b.origin)
}

fn push_command(scratch: &WeaveScratch, frame: &mut WeaveFrame, command: WeaveCommand, tick: u64) {
    if let Some(template) = builtin(command.template) {
        let seq_base = frame.next_seq;
        let (mods, mod_count) = writes_for(&template, command.origin, seq_base);
        let pushed = store_writes(frame, &mods[..mod_count as usize]);
        frame.next_seq = frame.next_seq.saturating_add(pushed);
        if tick == command.start && template.ice_radius > 0 {
            push_ice(
                frame,
                tick,
                command.seq,
                command.origin,
                template.ice_radius,
            );
        }
        return;
    }
    let Some(loaded) = scratch.loaded(command.template) else {
        return;
    };
    let mut batch = [FieldWrite {
        seq: 0,
        shape: Shape::Point,
        origin: command.origin,
        end: command.origin,
        radius: 0,
        op: WriteOp::AddTemp {
            delta: 0,
            stack: false,
        },
    }; 8];
    let count = loaded.node_count as usize;
    for (index, (slot, node)) in batch
        .iter_mut()
        .zip(loaded.nodes.iter())
        .take(count)
        .enumerate()
    {
        *slot = write_from_node(
            node,
            command.origin,
            frame.next_seq.saturating_add(index as u32),
        );
    }
    let pushed = store_writes(frame, &batch[..count]);
    frame.next_seq = frame.next_seq.saturating_add(pushed);
}

fn store_writes(frame: &mut WeaveFrame, writes: &[FieldWrite]) -> u32 {
    let mut pushed = 0u32;
    for write in writes {
        let index = frame.write_count as usize;
        if index >= MAX_FIELD_WRITES {
            break;
        }
        frame.writes[index] = *write;
        frame.write_count = frame.write_count.saturating_add(1);
        pushed = pushed.saturating_add(1);
    }
    pushed
}

fn push_ice(frame: &mut WeaveFrame, tick: u64, seq: u32, origin: [i32; 3], radius: u8) {
    debug_assert!(
        origin
            .iter()
            .all(|cell| (-ISLAND_HALF_CELLS..=ISLAND_HALF_CELLS).contains(cell)),
        "freeze origin is outside the island; queue_weave rejects it before writes are stored"
    );
    let radius = radius as i32;
    let min_milli = [
        (origin[0] - radius) * CELL_MM,
        origin[1] * CELL_MM,
        (origin[2] - radius) * CELL_MM,
    ];
    let max_milli = [
        (origin[0] + radius + 1) * CELL_MM,
        (origin[1] + 1) * CELL_MM,
        (origin[2] + radius + 1) * CELL_MM,
    ];
    let sheet_index = frame.sheet_count as usize;
    if sheet_index < MAX_SINKS {
        frame.sheets[sheet_index] = IceSheet {
            tick,
            seq,
            min_milli,
            max_milli,
        };
        frame.sheet_count = frame.sheet_count.saturating_add(1);
    }
    let nav_index = frame.nav_count as usize;
    if nav_index < MAX_SINKS {
        frame.nav[nav_index] =
            NavChange::new(tick, seq, min_milli, max_milli, NavChangeKind::Weave);
        frame.nav_count = frame.nav_count.saturating_add(1);
    }
}

/// Evaluate one tick against a field that does not live in this crate.
pub fn weave_tick(
    scratch: &WeaveScratch,
    frame: &mut WeaveFrame,
    field: &mut impl SimField,
    tick: u64,
) {
    weave_eval(scratch, frame, field, tick);
}

/// Bytes for the `weaving` replay case.
///
/// The 32-byte tail is [`SimField::field_digest`]. [`NullField`] returns zeros
/// until `LocalSim` is on this branch and the host passes it in.
pub fn weaving_case_bytes(world_seed: u64) -> Vec<u8> {
    weaving_case_on(world_seed, &mut NullField)
}

pub fn weaving_case_on(world_seed: u64, field: &mut impl SimField) -> Vec<u8> {
    let mut scratch = WeaveScratch::new();
    let mut frame = WeaveFrame::new();
    crate::mod_queue::queue_weave(
        &mut scratch,
        1,
        50,
        0,
        1,
        crate::templates::TEMPLATE_HEAT,
        [0, 0, 0],
    )
    .expect("heat");
    crate::mod_queue::queue_weave(
        &mut scratch,
        80,
        160,
        1,
        1,
        crate::templates::TEMPLATE_FREEZE,
        [2, 0, 0],
    )
    .expect("freeze");
    crate::mod_queue::queue_weave(
        &mut scratch,
        120,
        140,
        2,
        1,
        crate::templates::TEMPLATE_DENSITY,
        [0, 1, 0],
    )
    .expect("density");
    crate::mod_queue::queue_weave(
        &mut scratch,
        160,
        180,
        3,
        1,
        crate::templates::TEMPLATE_GRAVITY,
        [0, 0, 1],
    )
    .expect("gravity");
    let mut out = Vec::new();
    out.extend_from_slice(b"AWWEAV2");
    out.extend_from_slice(&world_seed.to_le_bytes());
    for tick in 0..300u64 {
        weave_tick(&scratch, &mut frame, field, tick);
        if tick == 80 {
            debug_assert!(frame.sheet_count() >= 1);
            debug_assert!(frame.nav_count() >= 1);
        }
        push_tick(&mut out, tick, &frame, field.field_digest());
    }
    let _ = TEMPLATE_FREEZE;
    out
}

fn push_tick(out: &mut Vec<u8>, tick: u64, frame: &WeaveFrame, digest: [u8; 32]) {
    out.extend_from_slice(&tick.to_le_bytes());
    out.push(frame.write_count);
    for index in 0..frame.write_count as usize {
        encode_write(out, &frame.writes[index]);
    }
    out.push(frame.sheet_count);
    out.push(frame.nav_count);
    out.extend_from_slice(&digest);
}

pub(crate) fn write_from_node(node: &WeaveNode, origin: [i32; 3], seq: u32) -> FieldWrite {
    let shape = if node.radius == 0 {
        Shape::Point
    } else {
        Shape::Sphere
    };
    let op = match node.kind {
        KIND_HEAT => WriteOp::AddTemp {
            delta: node.a,
            stack: false,
        },
        KIND_DENSITY | KIND_DECAY => WriteOp::AddDensity {
            delta: node.a.clamp(-128, 127) as i8,
            stack: false,
        },
        KIND_GRAVITY => WriteOp::SetGravity {
            index: node.a.max(0) as u16,
        },
        KIND_PHASE => WriteOp::SetPhase {
            phase: node.a.max(0) as u8,
        },
        KIND_VOLUME => WriteOp::AddVolume { delta: node.a },
        KIND_WIND => WriteOp::SetWind {
            x: node.a.clamp(-128, 127) as i8,
            y: 0,
            z: 0,
        },
        _ => WriteOp::AddTemp {
            delta: 0,
            stack: false,
        },
    };
    FieldWrite {
        seq,
        shape,
        origin,
        end: origin,
        radius: node.radius,
        op,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::{TestSimField, WriteOp};
    use crate::mod_queue::queue_weave;
    use crate::templates::{TEMPLATE_FREEZE, TEMPLATE_GRAVITY, TEMPLATE_HEAT};

    #[test]
    fn heat_writes_temp_and_freeze_emits_sheet_and_nav() {
        let mut scratch = WeaveScratch::new();
        let mut frame = WeaveFrame::new();
        let mut field = TestSimField::new();
        queue_weave(&mut scratch, 1, 10, 0, 1, TEMPLATE_HEAT, [0, 0, 0]).unwrap();
        queue_weave(&mut scratch, 2, 6, 1, 1, TEMPLATE_FREEZE, [1, 0, 0]).unwrap();
        weave_tick(&scratch, &mut frame, &mut field, 1);
        assert!(matches!(
            frame.write(0).unwrap().op,
            WriteOp::AddTemp { delta: 400, .. }
        ));
        weave_tick(&scratch, &mut frame, &mut field, 2);
        assert!(frame.write_count() >= 2);
        assert!(frame.writes_op_is_ice());
        assert_eq!(frame.sheet_count(), 1);
        assert_eq!(frame.nav_count(), 1);
        let nav = frame.nav_change(0).unwrap();
        assert_eq!(nav.tick, 2);
        assert_eq!(nav.min_milli, frame.sheet(0).unwrap().min_milli);
        assert_ne!(field.field_digest(), [0; 32]);
    }

    #[test]
    fn unknown_template_does_not_enqueue() {
        let mut scratch = WeaveScratch::new();
        let err = queue_weave(&mut scratch, 0, 1, 1, 9, 99, [0, 0, 0]);
        assert_eq!(err, Err(crate::mod_queue::QueueError::UnknownTemplate));
        assert_eq!(scratch.command_count(), 0);
    }

    #[test]
    fn gravity_template_sets_zone_index() {
        let mut scratch = WeaveScratch::new();
        let mut frame = WeaveFrame::new();
        let mut field = TestSimField::new();
        queue_weave(&mut scratch, 0, 2, 0, 1, TEMPLATE_GRAVITY, [0, 0, 0]).unwrap();
        weave_tick(&scratch, &mut frame, &mut field, 0);
        assert!(matches!(
            frame.write(0).unwrap().op,
            WriteOp::SetGravity { index: 1 }
        ));
    }

    #[test]
    fn command_order_does_not_change_the_writes() {
        let mut a = WeaveScratch::new();
        let mut b = WeaveScratch::new();
        queue_weave(&mut a, 0, 1, 1, 2, TEMPLATE_HEAT, [0, 0, 0]).unwrap();
        queue_weave(&mut a, 0, 1, 0, 1, TEMPLATE_FREEZE, [2, 0, 0]).unwrap();
        queue_weave(&mut b, 0, 1, 0, 1, TEMPLATE_FREEZE, [2, 0, 0]).unwrap();
        queue_weave(&mut b, 0, 1, 1, 2, TEMPLATE_HEAT, [0, 0, 0]).unwrap();
        let mut fa = WeaveFrame::new();
        let mut fb = WeaveFrame::new();
        let mut field_a = TestSimField::new();
        let mut field_b = TestSimField::new();
        weave_tick(&a, &mut fa, &mut field_a, 0);
        weave_tick(&b, &mut fb, &mut field_b, 0);
        assert_eq!(fa.write_count(), fb.write_count());
        assert_eq!(field_a.field_digest(), field_b.field_digest());
    }
}
