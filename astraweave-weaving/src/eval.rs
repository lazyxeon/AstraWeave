//! Fixed-tick weave evaluation. `weave_tick` does not allocate.

use crate::field::{
    flags_of, phase_of, FieldSample, SimField, TestSimField, CELL_MM, MAT_FIRE, MAT_ICE,
    MAT_POISON, MAT_STEAM, MAT_WATER,
};
use crate::templates::{builtin, modifiers_for, Template, MAX_NODES};
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
    next_mod_id: u32,
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
            next_mod_id: 1,
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
}

impl Default for WeaveScratch {
    fn default() -> Self {
        Self::new()
    }
}

/// Push this tick's modifiers. Does not stamp the field.
pub fn weave_eval(scratch: &mut WeaveScratch, field: &mut impl SimField, tick: u64) {
    field.begin_tick();
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
        let id_base = scratch.next_mod_id;
        let (mods, mod_count) = modifiers_for(&template, command.origin, id_base);
        let mut pushed = 0u32;
        for modifier in mods.into_iter().take(mod_count as usize) {
            if field.try_push(modifier).is_ok() {
                pushed = pushed.saturating_add(1);
            }
        }
        scratch.next_mod_id = scratch.next_mod_id.saturating_add(pushed);
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

/// Test-double tick: evaluate, stamp, then fill the hazard tail.
pub fn weave_tick(scratch: &mut WeaveScratch, field: &mut TestSimField, tick: u64) {
    weave_eval(scratch, field, tick);
    field.stamp_and_react();
    collect_hazards(scratch, field);
}

fn collect_hazards(scratch: &mut WeaveScratch, field: &TestSimField) {
    scratch.hazard_count = 0;
    scratch.hazards = [WeaveHazard::EMPTY; 16];
    // Window is [-8, 8). Visit in z, y, x so ties are stable.
    for z in -8..8 {
        for y in -8..8 {
            for x in -8..8 {
                let sample = field.sample([x, y, z]);
                let Some(kind) = hazard_kind(&sample) else {
                    continue;
                };
                insert_hazard(
                    scratch,
                    WeaveHazard {
                        kind,
                        x,
                        y,
                        z,
                        temperature: sample.temperature,
                        flags: sample.flags,
                        radius_milli: CELL_MM as u16,
                    },
                );
            }
        }
    }
}

fn hazard_kind(sample: &FieldSample) -> Option<u8> {
    match sample.material {
        MAT_FIRE => Some(WeaveHazard::FIRE),
        MAT_STEAM => Some(WeaveHazard::STEAM),
        MAT_POISON => Some(WeaveHazard::POISON_GAS),
        MAT_WATER => Some(WeaveHazard::DEEP_WATER),
        MAT_ICE => Some(WeaveHazard::ICE),
        _ => {
            let _ = (phase_of(sample.material), flags_of(sample.material));
            None
        }
    }
}

fn insert_hazard(scratch: &mut WeaveScratch, hazard: WeaveHazard) {
    let count = scratch.hazard_count as usize;
    if count < 16 {
        scratch.hazards[count] = hazard;
        scratch.hazard_count = scratch.hazard_count.saturating_add(1);
        sort_hazards(scratch);
        return;
    }
    // Full: replace the last (farthest in the sort) if this one sorts earlier.
    if hazard_less(&hazard, &scratch.hazards[15]) {
        scratch.hazards[15] = hazard;
        sort_hazards(scratch);
    }
}

fn sort_hazards(scratch: &mut WeaveScratch) {
    let count = scratch.hazard_count as usize;
    for i in 1..count {
        let mut j = i;
        while j > 0 && hazard_less(&scratch.hazards[j], &scratch.hazards[j - 1]) {
            scratch.hazards.swap(j, j - 1);
            j -= 1;
        }
    }
}

fn hazard_less(a: &WeaveHazard, b: &WeaveHazard) -> bool {
    (a.kind, a.z, a.y, a.x) < (b.kind, b.z, b.y, b.x)
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

/// Bytes for the `weaving` replay case. 300 ticks, seed recorded by the caller.
pub fn weaving_case_bytes(world_seed: u64) -> Vec<u8> {
    let _ = world_seed;
    let mut scratch = WeaveScratch::new();
    let mut field = TestSimField::new();
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
    out.extend_from_slice(b"AWWEAV1");
    out.extend_from_slice(&world_seed.to_le_bytes());
    for tick in 0..300u64 {
        weave_tick(&mut scratch, &mut field, tick);
        if tick == 40 {
            debug_assert_eq!(field.sample([0, 0, 0]).material, MAT_FIRE);
        }
        if tick == 80 {
            debug_assert!(scratch.sheet_count() >= 1);
            debug_assert!(scratch.nav_count() >= 1);
            debug_assert_ne!(field.sample([2, 0, 0]).flags & crate::field::FLAG_ICE, 0);
        }
        push_tick(&mut out, tick, &scratch, &field);
    }
    out
}

fn push_tick(out: &mut Vec<u8>, tick: u64, scratch: &WeaveScratch, field: &TestSimField) {
    out.extend_from_slice(&tick.to_le_bytes());
    out.push(scratch.hazard_count);
    for i in 0..scratch.hazard_count as usize {
        let h = scratch.hazards[i];
        out.push(h.kind);
        out.extend_from_slice(&h.x.to_le_bytes());
        out.extend_from_slice(&h.y.to_le_bytes());
        out.extend_from_slice(&h.z.to_le_bytes());
        out.extend_from_slice(&h.temperature.to_le_bytes());
        out.push(h.flags);
    }
    out.push(scratch.sheet_count);
    out.push(scratch.nav_count);
    out.push(field.modifier_count() as u8);
    let focus = field.sample([0, 0, 0]);
    out.extend_from_slice(&focus.temperature.to_le_bytes());
    out.push(focus.phase);
    out.extend_from_slice(&focus.density.to_le_bytes());
    out.push(focus.flags);
    out.extend_from_slice(&focus.gravity_delta.to_le_bytes());
    out.extend_from_slice(&focus.material.to_le_bytes());
    let ice = field.sample([2, 0, 0]);
    out.push(ice.phase);
    out.push(ice.flags);
    let _ = MAX_NODES;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::{FLAG_ICE, PHASE_BURN, PHASE_ICE};
    use crate::templates::{TEMPLATE_FREEZE, TEMPLATE_GRAVITY, TEMPLATE_HEAT};

    #[test]
    fn heat_ignites_and_freeze_emits_sheet_and_nav() {
        let mut scratch = WeaveScratch::new();
        let mut field = TestSimField::new();
        scratch
            .enqueue(1, 10, 0, 1, TEMPLATE_HEAT, [0, 0, 0])
            .unwrap();
        scratch
            .enqueue(2, 6, 1, 1, TEMPLATE_FREEZE, [1, 0, 0])
            .unwrap();
        weave_tick(&mut scratch, &mut field, 1);
        assert_eq!(field.sample([0, 0, 0]).phase, PHASE_BURN);
        assert!(scratch.hazards[..scratch.hazard_count as usize]
            .iter()
            .any(|h| h.kind == WeaveHazard::FIRE));
        weave_tick(&mut scratch, &mut field, 2);
        assert_eq!(field.sample([1, 0, 0]).phase, PHASE_ICE);
        assert_ne!(field.sample([1, 0, 0]).flags & FLAG_ICE, 0);
        assert_eq!(scratch.sheet_count(), 1);
        assert_eq!(scratch.nav_count(), 1);
        let nav = scratch.nav_change(0).unwrap();
        assert_eq!(nav.tick, 2);
        assert_eq!(nav.min_milli[0], scratch.sheet(0).unwrap().min_milli[0]);
    }

    #[test]
    fn unknown_template_does_not_enqueue() {
        let mut scratch = WeaveScratch::new();
        let err = scratch.enqueue(0, 1, 0, 1, 999, [0, 0, 0]);
        assert_eq!(err, Err(QueueError::UnknownTemplate));
        assert_eq!(scratch.command_count(), 0);
    }

    #[test]
    fn gravity_template_zeroes_world_g() {
        let mut scratch = WeaveScratch::new();
        let mut field = TestSimField::new();
        scratch
            .enqueue(0, 2, 0, 1, TEMPLATE_GRAVITY, [0, 0, 0])
            .unwrap();
        weave_tick(&mut scratch, &mut field, 0);
        assert_eq!(field.sample([0, 0, 0]).gravity_delta, 1000);
    }

    #[test]
    fn command_order_does_not_change_the_stamp() {
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
        assert_eq!(fa.sample([0, 0, 0]), fb.sample([0, 0, 0]));
        assert_eq!(fa.sample([2, 0, 0]), fb.sample([2, 0, 0]));
    }
}
