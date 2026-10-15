//! Fixed-tick weave evaluation. `weave_eval` does not allocate and does not store cells.

use crate::field::{encode_write, FieldWrite, NullField, SimField, CELL_MM, MAX_FIELD_WRITES};
use crate::templates::{builtin, writes_for, Template};
use astraweave_core::WeaveHazard;
use astraweave_nav::NavChange;

pub const MAX_COMMANDS: usize = 64;
pub const MAX_SINKS: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WeaveCommand {
    pub start: u64,
    pub until: u64,
    pub seq: u32,
    pub caster: u64,
    pub template: u16,
    pub origin: [i32; 3],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IceSheet {
    pub tick: u64,
    pub seq: u32,
    pub min_milli: [i32; 3],
    pub max_milli: [i32; 3],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QueueError {
    UnknownTemplate,
    Full,
}

#[derive(Clone)]
pub struct WeaveScratch {
    commands: [WeaveCommand; MAX_COMMANDS],
    command_count: u8,
    order: [u8; MAX_COMMANDS],
    pub hazards: [WeaveHazard; 16],
    pub hazard_count: u8,
    sheets: [IceSheet; MAX_SINKS],
    sheet_count: u8,
    nav: [NavChange; MAX_SINKS],
    nav_count: u8,
    writes: [FieldWrite; MAX_FIELD_WRITES],
    write_count: u8,
    next_seq: u32,
    loaded: [Template; 16],
    loaded_count: u8,
}

impl WeaveScratch {
    pub fn new() -> Self {
        Self {
            commands: [WeaveCommand {
                start: 0,
                until: 0,
                seq: 0,
                caster: 0,
                template: 0,
                origin: [0, 0, 0],
            }; MAX_COMMANDS],
            command_count: 0,
            order: [0; MAX_COMMANDS],
            hazards: [WeaveHazard::EMPTY; 16],
            hazard_count: 0,
            sheets: [IceSheet {
                tick: 0,
                seq: 0,
                min_milli: [0, 0, 0],
                max_milli: [0, 0, 0],
            }; MAX_SINKS],
            sheet_count: 0,
            nav: [NavChange {
                tick: 0,
                seq: 0,
                min_milli: [0, 0, 0],
                max_milli: [0, 0, 0],
            }; MAX_SINKS],
            nav_count: 0,
            writes: [FieldWrite {
                seq: 0,
                shape: crate::field::Shape::Point,
                origin: [0, 0, 0],
                end: [0, 0, 0],
                radius: 0,
                op: crate::field::WriteOp::AddTemp {
                    delta: 0,
                    stack: false,
                },
            }; MAX_FIELD_WRITES],
            write_count: 0,
            next_seq: 1,
            loaded: [Template::vacant(0); 16],
            loaded_count: 0,
        }
    }

    pub fn load_template(&mut self, template: Template) -> Result<(), QueueError> {
        if builtin(template.id).is_some() {
            return Err(QueueError::UnknownTemplate);
        }
        let index = self.loaded_count as usize;
        if index >= self.loaded.len() {
            return Err(QueueError::Full);
        }
        self.loaded[index] = template;
        self.loaded_count = self.loaded_count.saturating_add(1);
        Ok(())
    }

    fn find_template(&self, id: u16) -> Option<Template> {
        if let Some(template) = builtin(id) {
            return Some(template);
        }
        let count = self.loaded_count as usize;
        self.loaded[..count].iter().copied().find(|t| t.id == id)
    }

    pub fn enqueue(
        &mut self,
        start: u64,
        until: u64,
        seq: u32,
        caster: u64,
        template: u16,
        origin: [i32; 3],
    ) -> Result<(), QueueError> {
        if self.find_template(template).is_none() {
            return Err(QueueError::UnknownTemplate);
        }
        let index = self.command_count as usize;
        if index >= MAX_COMMANDS {
            return Err(QueueError::Full);
        }
        self.commands[index] = WeaveCommand {
            start,
            until,
            seq,
            caster,
            template,
            origin,
        };
        self.command_count = self.command_count.saturating_add(1);
        Ok(())
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

    pub fn command_count(&self) -> usize {
        self.command_count as usize
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
                    crate::field::WriteOp::SetPhase {
                        phase: crate::field::PHASE_ICE
                    }
                )
            })
    }
}

impl Default for WeaveScratch {
    fn default() -> Self {
        Self::new()
    }
}

/// Expand active templates into [`FieldWrite`]s and hand them to the field.
/// Does not read or store cells.
pub fn weave_eval(scratch: &mut WeaveScratch, field: &mut impl SimField, tick: u64) {
    scratch.write_count = 0;
    let count = scratch.command_count as usize;
    for (i, slot) in scratch.order.iter_mut().enumerate().take(count) {
        *slot = i as u8;
    }
    for i in 1..count {
        let mut j = i;
        while j > 0
            && command_less(
                &scratch.commands[scratch.order[j] as usize],
                &scratch.commands[scratch.order[j - 1] as usize],
            )
        {
            scratch.order.swap(j, j - 1);
            j -= 1;
        }
    }
    for n in 0..count {
        let command = scratch.commands[scratch.order[n] as usize];
        if tick < command.start || tick >= command.until {
            continue;
        }
        let Some(template) = scratch.find_template(command.template) else {
            continue;
        };
        let seq_base = scratch.next_seq;
        let (mods, mod_count) = writes_for(&template, command.origin, seq_base);
        let mut pushed = 0u32;
        for write in mods.into_iter().take(mod_count as usize) {
            let index = scratch.write_count as usize;
            if index >= MAX_FIELD_WRITES {
                break;
            }
            scratch.writes[index] = write;
            scratch.write_count = scratch.write_count.saturating_add(1);
            pushed = pushed.saturating_add(1);
        }
        scratch.next_seq = scratch.next_seq.saturating_add(pushed);
        if tick == command.start && template.ice_radius > 0 {
            push_ice(
                scratch,
                tick,
                command.seq,
                command.origin,
                template.ice_radius,
            );
        }
    }
    let n = scratch.write_count as usize;
    let staged = scratch.writes;
    field.apply_writes(&staged[..n]);
}

fn command_less(a: &WeaveCommand, b: &WeaveCommand) -> bool {
    (a.start, a.seq, a.caster, a.template, a.origin)
        < (b.start, b.seq, b.caster, b.template, b.origin)
}

fn push_ice(scratch: &mut WeaveScratch, tick: u64, seq: u32, origin: [i32; 3], radius: u8) {
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
    let sheet_index = scratch.sheet_count as usize;
    if sheet_index < MAX_SINKS {
        scratch.sheets[sheet_index] = IceSheet {
            tick,
            seq,
            min_milli,
            max_milli,
        };
        scratch.sheet_count = scratch.sheet_count.saturating_add(1);
    }
    let nav_index = scratch.nav_count as usize;
    if nav_index < MAX_SINKS {
        scratch.nav[nav_index] = NavChange {
            tick,
            seq,
            min_milli,
            max_milli,
        };
        scratch.nav_count = scratch.nav_count.saturating_add(1);
    }
}

/// Evaluate one tick against a field that does not live in this crate.
pub fn weave_tick(scratch: &mut WeaveScratch, field: &mut impl SimField, tick: u64) {
    weave_eval(scratch, field, tick);
}

/// Mod command entry. PR #214's sandbox calls this once that API is on the branch.
pub fn queue_weave(
    scratch: &mut WeaveScratch,
    start: u64,
    until: u64,
    seq: u32,
    caster: u64,
    template: u16,
    origin: [i32; 3],
) -> Result<(), QueueError> {
    scratch.enqueue(start, until, seq, caster, template, origin)
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
    scratch
        .enqueue(1, 50, 0, 1, crate::templates::TEMPLATE_HEAT, [0, 0, 0])
        .expect("heat");
    scratch
        .enqueue(80, 160, 1, 1, crate::templates::TEMPLATE_FREEZE, [2, 0, 0])
        .expect("freeze");
    scratch
        .enqueue(
            120,
            140,
            2,
            1,
            crate::templates::TEMPLATE_DENSITY,
            [0, 1, 0],
        )
        .expect("density");
    scratch
        .enqueue(
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
        weave_tick(&mut scratch, field, tick);
        if tick == 80 {
            debug_assert!(scratch.sheet_count() >= 1);
            debug_assert!(scratch.nav_count() >= 1);
        }
        push_tick(&mut out, tick, &scratch, field.field_digest());
    }
    out
}

fn push_tick(out: &mut Vec<u8>, tick: u64, scratch: &WeaveScratch, digest: [u8; 32]) {
    out.extend_from_slice(&tick.to_le_bytes());
    out.push(scratch.write_count);
    for index in 0..scratch.write_count as usize {
        encode_write(out, &scratch.writes[index]);
    }
    out.push(scratch.sheet_count);
    out.push(scratch.nav_count);
    out.extend_from_slice(&digest);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::{TestSimField, WriteOp};
    use crate::templates::{TEMPLATE_FREEZE, TEMPLATE_GRAVITY, TEMPLATE_HEAT};

    #[test]
    fn heat_writes_temp_and_freeze_emits_sheet_and_nav() {
        let mut scratch = WeaveScratch::new();
        let mut field = TestSimField::new();
        scratch
            .enqueue(1, 10, 0, 1, TEMPLATE_HEAT, [0, 0, 0])
            .unwrap();
        scratch
            .enqueue(2, 6, 1, 1, TEMPLATE_FREEZE, [1, 0, 0])
            .unwrap();
        weave_tick(&mut scratch, &mut field, 1);
        assert!(matches!(
            scratch.write(0).unwrap().op,
            WriteOp::AddTemp { delta: 400, .. }
        ));
        weave_tick(&mut scratch, &mut field, 2);
        assert!(scratch.write_count() >= 2);
        assert!(scratch.writes_op_is_ice());
        assert_eq!(scratch.sheet_count(), 1);
        assert_eq!(scratch.nav_count(), 1);
        let nav = scratch.nav_change(0).unwrap();
        assert_eq!(nav.tick, 2);
        assert_eq!(nav.min_milli, scratch.sheet(0).unwrap().min_milli);
        assert_ne!(field.field_digest(), [0; 32]);
    }

    #[test]
    fn unknown_template_does_not_enqueue() {
        let mut scratch = WeaveScratch::new();
        let err = scratch.enqueue(0, 1, 0, 1, 999, [0, 0, 0]);
        assert_eq!(err, Err(QueueError::UnknownTemplate));
        assert_eq!(scratch.command_count(), 0);
    }

    #[test]
    fn gravity_template_sets_zone_index() {
        let mut scratch = WeaveScratch::new();
        let mut field = TestSimField::new();
        scratch
            .enqueue(0, 2, 0, 1, TEMPLATE_GRAVITY, [0, 0, 0])
            .unwrap();
        weave_tick(&mut scratch, &mut field, 0);
        assert!(matches!(
            scratch.write(0).unwrap().op,
            WriteOp::SetGravity { index: 1 }
        ));
    }

    #[test]
    fn command_order_does_not_change_the_writes() {
        let mut a = WeaveScratch::new();
        let mut b = WeaveScratch::new();
        a.enqueue(0, 1, 1, 2, TEMPLATE_HEAT, [0, 0, 0]).unwrap();
        a.enqueue(0, 1, 0, 1, TEMPLATE_FREEZE, [2, 0, 0]).unwrap();
        b.enqueue(0, 1, 0, 1, TEMPLATE_FREEZE, [2, 0, 0]).unwrap();
        b.enqueue(0, 1, 1, 2, TEMPLATE_HEAT, [0, 0, 0]).unwrap();
        let mut fa = TestSimField::new();
        let mut fb = TestSimField::new();
        weave_tick(&mut a, &mut fa, 0);
        weave_tick(&mut b, &mut fb, 0);
        assert_eq!(a.write_count(), b.write_count());
        assert_eq!(fa.field_digest(), fb.field_digest());
    }
}
