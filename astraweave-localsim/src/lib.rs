//! Deterministic local field for fluid volume, gas, and heat.
//!
//! Storage, the activation map, and the solver live here. Weaving (PR #252)
//! writes [`FieldWrite`] stamps and reads [`FieldEvent`] plus
//! [`LocalSim::field_digest`]. GPU particle fluids are not this state.
//!
//! `tick` does not allocate and does not take a lock. Regions run in slot order.

mod types;

pub use types::*;

use astraweave_water::{ColumnWater, WaterColumn};
use sha2::{Digest, Sha256};

struct Channels {
    temp: Box<[i16]>,
    phase: Box<[u8]>,
    density_delta: Box<[i8]>,
    flags: Box<[u8]>,
    gravity: Box<[u16]>,
    volume: Box<[u8]>,
    gas: Box<[u8]>,
    wind: Box<[i8]>,
    material: Box<[u16]>,
    solid: Box<[u8]>,
    sleep: Box<[SleepCell]>,
    scratch_u8: Box<[u8]>,
    scratch_i16: Box<[i16]>,
}

impl Channels {
    fn new() -> Self {
        let n = POOL_CAP * REGION_CELLS;
        Self {
            temp: vec![0; n].into_boxed_slice(),
            phase: vec![0; n].into_boxed_slice(),
            density_delta: vec![0; n].into_boxed_slice(),
            flags: vec![0; n].into_boxed_slice(),
            gravity: vec![0; n].into_boxed_slice(),
            volume: vec![0; n].into_boxed_slice(),
            gas: vec![0; n].into_boxed_slice(),
            wind: vec![0; n * 3].into_boxed_slice(),
            material: vec![0; n].into_boxed_slice(),
            solid: vec![0; n].into_boxed_slice(),
            sleep: vec![SleepCell::ZERO; POOL_CAP * SLEEP_CAP].into_boxed_slice(),
            scratch_u8: vec![0; REGION_CELLS].into_boxed_slice(),
            scratch_i16: vec![0; REGION_CELLS].into_boxed_slice(),
        }
    }
}

pub struct LocalSim {
    meta: [SlotMeta; POOL_CAP],
    ch: Channels,
    tick: u64,
    events: [FieldEvent; MAX_FIELD_EVENTS],
    event_len: u16,
    event_drops: u16,
    hazards: [HazardSample; MAX_HAZARDS],
    hazard_len: u16,
    columns: Vec<WaterColumn>,
    column_len: u16,
    column_drops: u16,
    write_drops: u16,
    /// Cells whose compact sleep encoding was restored on this tick.
    wake_count: u16,
}

impl LocalSim {
    pub fn new() -> Self {
        Self {
            meta: [SlotMeta::EMPTY; POOL_CAP],
            ch: Channels::new(),
            tick: 0,
            events: [FieldEvent {
                x: 0,
                y: 0,
                z: 0,
                kind: 0,
                material_id: 0,
                temp: 0,
            }; MAX_FIELD_EVENTS],
            event_len: 0,
            event_drops: 0,
            hazards: [HazardSample {
                cell: [0, 0, 0],
                phase: 0,
                temp: 0,
                flags: 0,
            }; MAX_HAZARDS],
            hazard_len: 0,
            columns: vec![WaterColumn::EMPTY; MAX_COLUMNS],
            column_len: 0,
            column_drops: 0,
            write_drops: 0,
            wake_count: 0,
        }
    }

    pub fn tick_index(&self) -> u64 {
        self.tick
    }

    pub fn active_count(&self) -> u16 {
        self.meta.iter().filter(|m| m.active).count() as u16
    }

    pub fn occupied_count(&self) -> u16 {
        self.meta.iter().filter(|m| m.occupied).count() as u16
    }

    pub fn events(&self) -> &[FieldEvent] {
        &self.events[..self.event_len as usize]
    }

    pub fn hazards(&self) -> &[HazardSample] {
        &self.hazards[..self.hazard_len as usize]
    }

    pub fn wake_count(&self) -> u16 {
        self.wake_count
    }

    /// One tick. `subjects` is the activation set for this tick.
    /// `writes` must already be ordered by `seq`.
    pub fn tick(&mut self, tick: u64, input: TickInput<'_>, solid: &impl SolidField) {
        self.tick = tick;
        self.event_len = 0;
        self.event_drops = 0;
        self.write_drops = 0;
        self.wake_count = 0;
        let nwrite = input.writes.len().min(MAX_FIELD_WRITES);
        if input.writes.len() > MAX_FIELD_WRITES {
            self.write_drops = (input.writes.len() - MAX_FIELD_WRITES) as u16;
        }
        self.activate(input.subjects);
        self.apply_writes(&input.writes[..nwrite]);
        self.select_active();
        for slot in 0..POOL_CAP {
            if self.meta[slot].active && self.meta[slot].sleeping {
                self.decode_slot(slot);
                self.meta[slot].sleeping = false;
                self.wake_count = self.wake_count.saturating_add(1);
            }
        }
        self.sample_solid(solid);
        for _ in 0..FLOW_ITERS {
            self.flow_pass(input.zones);
        }
        for _ in 0..GAS_ITERS {
            self.gas_pass();
        }
        for _ in 0..HEAT_ITERS {
            self.heat_pass(input.materials);
        }
        self.reconcile(input.materials);
        self.mark_lively();
        self.sleep_inactive();
        self.publish(input.subjects, input.materials);
    }

    pub fn volume_at(&self, world: [i32; 3]) -> u8 {
        let mut found = 0u8;
        let mut owned = false;
        for slot in 0..POOL_CAP {
            if !self.meta[slot].occupied {
                continue;
            }
            if let Some(cell) = local_cell(self.meta[slot].origin, world) {
                if !owned {
                    found = self.vol(slot, cell);
                    owned = true;
                }
            }
        }
        found
    }

    pub fn gas_at(&self, world: [i32; 3]) -> u8 {
        for slot in 0..POOL_CAP {
            if !self.meta[slot].occupied {
                continue;
            }
            if let Some(cell) = local_cell(self.meta[slot].origin, world) {
                return self.gas(slot, cell);
            }
        }
        0
    }

    pub fn temp_at(&self, world: [i32; 3]) -> i16 {
        for slot in 0..POOL_CAP {
            if !self.meta[slot].occupied {
                continue;
            }
            if let Some(cell) = local_cell(self.meta[slot].origin, world) {
                return self.temp(slot, cell);
            }
        }
        0
    }

    pub fn phase_at(&self, world: [i32; 3]) -> u8 {
        for slot in 0..POOL_CAP {
            if !self.meta[slot].occupied {
                continue;
            }
            if let Some(cell) = local_cell(self.meta[slot].origin, world) {
                return self.phase(slot, cell);
            }
        }
        0
    }

    pub fn write_columns(&self, out: &mut ColumnWater) {
        let n = self.column_len as usize;
        out.fill(&self.columns[..n]);
    }

    /// Visual samples for a GPU accent upload. Not part of the digest.
    pub fn write_visual(&self, out: &mut [VisualCell]) -> usize {
        let mut n = 0usize;
        for slot in 0..POOL_CAP {
            if !self.meta[slot].occupied || n >= out.len() {
                continue;
            }
            let origin = self.meta[slot].origin;
            if self.meta[slot].sleeping && !self.meta[slot].dense_sleep {
                let len = self.meta[slot].sleep_len as usize;
                for s in 0..len {
                    if n >= out.len() {
                        break;
                    }
                    let cell = self.sleep_at(slot, s);
                    if cell.volume == 0 && cell.gas == 0 {
                        continue;
                    }
                    let (x, y, z) = decode_index(cell.index as usize);
                    out[n] = VisualCell {
                        cell: [origin[0] + x, origin[1] + y, origin[2] + z],
                        volume: cell.volume,
                        gas: cell.gas,
                        temp: cell.temp,
                        phase: cell.phase,
                    };
                    n += 1;
                }
            } else {
                for cell in 0..REGION_CELLS {
                    if n >= out.len() {
                        break;
                    }
                    let volume = self.vol(slot, cell);
                    let gas = self.gas(slot, cell);
                    if volume == 0 && gas == 0 {
                        continue;
                    }
                    let (x, y, z) = decode_index(cell);
                    out[n] = VisualCell {
                        cell: [origin[0] + x, origin[1] + y, origin[2] + z],
                        volume,
                        gas,
                        temp: self.temp(slot, cell),
                        phase: self.phase(slot, cell),
                    };
                    n += 1;
                }
            }
        }
        n
    }

    pub fn field_digest(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(b"AWLOCL1");
        hasher.update(self.tick.to_le_bytes());
        hasher.update(SIM_SYSTEM_LOCAL_SIM.to_le_bytes());
        hasher.update(self.write_drops.to_le_bytes());
        hasher.update(self.event_drops.to_le_bytes());
        hasher.update(self.column_drops.to_le_bytes());
        for slot in 0..POOL_CAP {
            let m = &self.meta[slot];
            hasher.update([u8::from(m.occupied)]);
            hasher.update([u8::from(m.present)]);
            hasher.update([u8::from(m.active)]);
            hasher.update([u8::from(m.sleeping)]);
            hasher.update([u8::from(m.dense_sleep)]);
            hasher.update([u8::from(m.player)]);
            hasher.update(m.subject.to_le_bytes());
            for c in m.origin {
                hasher.update(c.to_le_bytes());
            }
            hasher.update(m.sleep_len.to_le_bytes());
            if !m.occupied {
                continue;
            }
            if m.sleeping && !m.dense_sleep {
                for i in 0..m.sleep_len as usize {
                    hash_sleep(&mut hasher, &self.sleep_at(slot, i));
                }
            } else {
                hash_raw_slot(&mut hasher, self, slot);
            }
        }
        for ev in self.events() {
            hasher.update(ev.x.to_le_bytes());
            hasher.update(ev.y.to_le_bytes());
            hasher.update(ev.z.to_le_bytes());
            hasher.update([ev.kind]);
            hasher.update(ev.material_id.to_le_bytes());
            hasher.update(ev.temp.to_le_bytes());
        }
        hasher.update(self.hazard_len.to_le_bytes());
        for hz in self.hazards() {
            for c in hz.cell {
                hasher.update(c.to_le_bytes());
            }
            hasher.update([hz.phase, hz.flags]);
            hasher.update(hz.temp.to_le_bytes());
        }
        hasher.update(self.column_len.to_le_bytes());
        for col in &self.columns[..self.column_len as usize] {
            hasher.update(col.x.to_le_bytes());
            hasher.update(col.z.to_le_bytes());
            hasher.update(col.surface_y.to_bits().to_le_bytes());
            hasher.update(col.density.to_bits().to_le_bytes());
            hasher.update([u8::from(col.ice)]);
        }
        let dig = hasher.finalize();
        let mut out = [0u8; 32];
        out.copy_from_slice(&dig);
        out
    }

    fn activate(&mut self, subjects: &[Subject]) {
        let mut matched = [false; POOL_CAP];
        for sub in subjects {
            let mut found = None;
            for slot in 0..POOL_CAP {
                if self.meta[slot].occupied && self.meta[slot].subject == sub.id {
                    found = Some(slot);
                    break;
                }
            }
            if let Some(slot) = found {
                matched[slot] = true;
                let origin = region_origin(sub.cell);
                if origin != self.meta[slot].origin {
                    self.ensure_raw(slot);
                    self.shift_slot(slot, origin);
                    self.meta[slot].origin = origin;
                }
                self.meta[slot].player = sub.player;
                self.meta[slot].present = true;
            } else {
                let slot = self.take_slot(sub.id);
                self.zero_slot(slot);
                self.meta[slot] = SlotMeta {
                    subject: sub.id,
                    origin: region_origin(sub.cell),
                    player: sub.player,
                    occupied: true,
                    present: true,
                    active: false,
                    sleeping: false,
                    dense_sleep: false,
                    lively: false,
                    sleep_len: 0,
                };
                matched[slot] = true;
            }
        }
        for (slot, was) in matched.iter().enumerate() {
            if self.meta[slot].occupied && !was {
                self.meta[slot].present = false;
            }
        }
    }

    fn take_slot(&mut self, new_id: u64) -> usize {
        for slot in 0..POOL_CAP {
            if !self.meta[slot].occupied {
                return slot;
            }
        }
        let mut best = 0usize;
        let mut best_id = 0u64;
        let mut any = false;
        for slot in 0..POOL_CAP {
            if self.meta[slot].player {
                continue;
            }
            if !any || self.meta[slot].subject >= best_id {
                any = true;
                best_id = self.meta[slot].subject;
                best = slot;
            }
        }
        if !any {
            for slot in 0..POOL_CAP {
                if self.meta[slot].subject != new_id && self.meta[slot].subject >= best_id {
                    best_id = self.meta[slot].subject;
                    best = slot;
                }
            }
        }
        best
    }

    fn select_active(&mut self) {
        for m in &mut self.meta {
            m.active = false;
        }
        let mut n = 0usize;
        for slot in 0..POOL_CAP {
            if n >= ACTIVE_BUDGET {
                break;
            }
            if self.meta[slot].occupied && self.meta[slot].present && self.meta[slot].player {
                self.meta[slot].active = true;
                n += 1;
            }
        }
        for slot in 0..POOL_CAP {
            if n >= ACTIVE_BUDGET {
                break;
            }
            if self.meta[slot].occupied
                && self.meta[slot].present
                && !self.meta[slot].active
                && self.meta[slot].lively
            {
                self.meta[slot].active = true;
                n += 1;
            }
        }
        for slot in 0..POOL_CAP {
            if n >= ACTIVE_BUDGET {
                break;
            }
            if self.meta[slot].occupied && self.meta[slot].present && !self.meta[slot].active {
                self.meta[slot].active = true;
                n += 1;
            }
        }
    }

    fn apply_writes(&mut self, writes: &[FieldWrite]) {
        if writes.is_empty() {
            return;
        }
        for slot in 0..POOL_CAP {
            if !self.meta[slot].occupied {
                continue;
            }
            let origin = self.meta[slot].origin;
            let mut hit = false;
            for cell in 0..REGION_CELLS {
                let (x, y, z) = decode_index(cell);
                let world = [origin[0] + x, origin[1] + y, origin[2] + z];
                if writes.iter().any(|w| write_hits(w, world)) {
                    hit = true;
                    break;
                }
            }
            if !hit {
                continue;
            }
            self.ensure_raw(slot);
            self.meta[slot].lively = true;
            self.fold_slot(slot, writes);
        }
    }

    fn fold_slot(&mut self, slot: usize, writes: &[FieldWrite]) {
        let origin = self.meta[slot].origin;
        for cell in 0..REGION_CELLS {
            let (x, y, z) = decode_index(cell);
            let world = [origin[0] + x, origin[1] + y, origin[2] + z];
            let mut touch = false;
            for w in writes {
                if write_hits(w, world) {
                    touch = true;
                    break;
                }
            }
            if !touch {
                continue;
            }
            let base_t = i32::from(self.temp(slot, cell));
            let base_d = i32::from(self.density(slot, cell));
            let mut pos_sum = 0i32;
            let mut pos_max = 0i32;
            let mut pos_stack = false;
            let mut pos_any = false;
            let mut neg_sum = 0i32;
            let mut neg_max = 0i32;
            let mut neg_stack = false;
            let mut neg_any = false;
            let mut dpos_sum = 0i32;
            let mut dpos_max = 0i32;
            let mut dpos_stack = false;
            let mut dpos_any = false;
            let mut dneg_sum = 0i32;
            let mut dneg_max = 0i32;
            let mut dneg_stack = false;
            let mut dneg_any = false;
            let mut phase = self.phase(slot, cell);
            let mut phase_seq = 0u32;
            let mut has_phase = false;
            let mut grav = self.gravity(slot, cell);
            let mut grav_seq = 0u32;
            let mut has_grav = false;
            let mut flags = self.flags(slot, cell);
            let mut volume = i32::from(self.vol(slot, cell));
            let mut wind = self.wind(slot, cell);
            for w in writes {
                if !write_hits(w, world) {
                    continue;
                }
                match w.op {
                    WriteOp::AddTemp { delta, stack } => {
                        if delta >= 0 {
                            pos_any = true;
                            if stack {
                                pos_stack = true;
                                pos_sum += i32::from(delta);
                            } else {
                                pos_max = pos_max.max(i32::from(delta));
                            }
                        } else {
                            neg_any = true;
                            let mag = i32::from(-delta);
                            if stack {
                                neg_stack = true;
                                neg_sum += mag;
                            } else {
                                neg_max = neg_max.max(mag);
                            }
                        }
                    }
                    WriteOp::AddDensity { delta, stack } => {
                        if delta >= 0 {
                            dpos_any = true;
                            if stack {
                                dpos_stack = true;
                                dpos_sum += i32::from(delta);
                            } else {
                                dpos_max = dpos_max.max(i32::from(delta));
                            }
                        } else {
                            dneg_any = true;
                            let mag = i32::from(-delta);
                            if stack {
                                dneg_stack = true;
                                dneg_sum += mag;
                            } else {
                                dneg_max = dneg_max.max(mag);
                            }
                        }
                    }
                    WriteOp::SetPhase { phase: p } => {
                        if !has_phase || w.seq >= phase_seq {
                            has_phase = true;
                            phase_seq = w.seq;
                            phase = p;
                        }
                    }
                    WriteOp::SetGravity { index } => {
                        if !has_grav || w.seq >= grav_seq {
                            has_grav = true;
                            grav_seq = w.seq;
                            grav = index;
                        }
                    }
                    WriteOp::SetFlags { set, clear } => {
                        flags |= set;
                        flags &= !clear;
                    }
                    WriteOp::AddVolume { delta } => {
                        volume = sat_u8_from_i32(volume + i32::from(delta)) as i32;
                    }
                    WriteOp::SetWind { x, y, z } => {
                        wind = [x, y, z];
                    }
                }
            }
            let pos = fold_side(pos_any, pos_stack, pos_sum, pos_max);
            let neg = fold_side(neg_any, neg_stack, neg_sum, neg_max);
            let net_t = if pos >= neg { pos - neg } else { -(neg - pos) };
            let dpos = fold_side(dpos_any, dpos_stack, dpos_sum, dpos_max);
            let dneg = fold_side(dneg_any, dneg_stack, dneg_sum, dneg_max);
            let net_d = if dpos >= dneg {
                dpos - dneg
            } else {
                -(dneg - dpos)
            };
            self.set_temp(slot, cell, sat_i16(base_t + net_t));
            self.set_density(slot, cell, sat_i8(base_d + net_d));
            if has_phase {
                phase = phase.min(PHASE_OIL);
                if phase == PHASE_ICE {
                    flags |= FLAG_ICE;
                }
                if phase == PHASE_BURN {
                    flags |= FLAG_BURN;
                }
                self.seed_phase(slot, cell, phase, &mut volume);
            }
            self.set_phase(slot, cell, phase);
            self.set_gravity(slot, cell, grav);
            self.set_flags(slot, cell, flags);
            self.set_vol(slot, cell, sat_u8_from_i32(volume));
            self.set_wind(slot, cell, wind);
        }
    }

    fn seed_phase(&mut self, slot: usize, cell: usize, phase: u8, volume: &mut i32) {
        match phase {
            PHASE_WET | PHASE_OIL => {
                if *volume == 0 {
                    *volume = i32::from(VOLUME_MAX);
                }
            }
            PHASE_ICE => {
                *volume = i32::from(VOLUME_MAX);
            }
            PHASE_GAS => {
                if self.gas(slot, cell) == 0 {
                    self.set_gas(slot, cell, VOLUME_MAX);
                }
            }
            PHASE_DRY => {
                *volume = 0;
                self.set_gas(slot, cell, 0);
            }
            _ => {}
        }
    }

    fn sample_solid(&mut self, solid: &impl SolidField) {
        for slot in 0..POOL_CAP {
            if !self.meta[slot].active || !self.meta[slot].lively {
                continue;
            }
            let origin = self.meta[slot].origin;
            for cell in 0..REGION_CELLS {
                let (x, y, z) = decode_index(cell);
                let world = [origin[0] + x, origin[1] + y, origin[2] + z];
                let is_solid = solid.solid(world[0], world[1], world[2]);
                self.set_solid(slot, cell, u8::from(is_solid));
                self.set_material(slot, cell, solid.material_id(world[0], world[1], world[2]));
            }
        }
    }

    fn flow_pass(&mut self, zones: &[GravityView]) {
        for slot in 0..POOL_CAP {
            if !self.meta[slot].active || !self.meta[slot].lively {
                continue;
            }
            let base = slot * REGION_CELLS;
            if self.ch.volume[base..base + REGION_CELLS]
                .iter()
                .all(|v| *v == 0)
            {
                continue;
            }
            for cell in 0..REGION_CELLS {
                self.ch.scratch_u8[cell] = self.vol(slot, cell);
                self.set_vol(slot, cell, 0);
            }
            for cell in 0..REGION_CELLS {
                let amount = i32::from(self.ch.scratch_u8[cell]);
                if amount == 0 {
                    continue;
                }
                let phase = self.phase(slot, cell);
                if phase == PHASE_ICE || self.solid_cell(slot, cell) {
                    self.add_vol(slot, cell, amount);
                    continue;
                }
                let grav = self.gravity(slot, cell);
                let step = gravity_step(grav, zones, self.meta[slot].origin, cell);
                if let Some((dx, dy, dz)) = step {
                    let (x, y, z) = decode_index(cell);
                    let nx = x + dx;
                    let ny = y + dy;
                    let nz = z + dz;
                    if in_region(nx, ny, nz) {
                        let dest = cell_index(nx, ny, nz);
                        if !self.solid_cell(slot, dest) {
                            let space = i32::from(VOLUME_MAX) - i32::from(self.vol(slot, dest));
                            let mv = amount.min(space.max(0));
                            if mv > 0 && self.vol(slot, dest) == 0 {
                                self.set_phase(slot, dest, phase);
                            }
                            self.add_vol(slot, dest, mv);
                            self.add_vol(slot, cell, amount - mv);
                            continue;
                        }
                    }
                }
                self.add_vol(slot, cell, amount);
            }
            self.equalize_slot(slot, zones);
        }
    }

    fn equalize_slot(&mut self, slot: usize, zones: &[GravityView]) {
        for cell in 0..REGION_CELLS {
            if self.solid_cell(slot, cell) || self.phase(slot, cell) == PHASE_ICE {
                continue;
            }
            let grav = self.gravity(slot, cell);
            let blocked = match gravity_step(grav, zones, self.meta[slot].origin, cell) {
                None => true,
                Some((dx, dy, dz)) => {
                    let (x, y, z) = decode_index(cell);
                    let nx = x + dx;
                    let ny = y + dy;
                    let nz = z + dz;
                    if !in_region(nx, ny, nz) {
                        true
                    } else {
                        let dest = cell_index(nx, ny, nz);
                        self.solid_cell(slot, dest) || self.vol(slot, dest) == VOLUME_MAX
                    }
                }
            };
            if !blocked {
                continue;
            }
            let (x, y, z) = decode_index(cell);
            for (dx, dy, dz) in [(1, 0, 0), (-1, 0, 0), (0, 0, 1), (0, 0, -1)] {
                let nx = x + dx;
                let ny = y + dy;
                let nz = z + dz;
                if !in_region(nx, ny, nz) {
                    continue;
                }
                let side = cell_index(nx, ny, nz);
                if self.solid_cell(slot, side) {
                    continue;
                }
                let a = i32::from(self.vol(slot, cell));
                let b = i32::from(self.vol(slot, side));
                if a > b + 1 {
                    let mv = ((a - b) / 2).min(i32::from(VOLUME_MAX) - b);
                    if mv > 0 {
                        if b == 0 {
                            self.set_phase(slot, side, self.phase(slot, cell));
                        }
                        self.set_vol(slot, cell, (a - mv) as u8);
                        self.set_vol(slot, side, (b + mv) as u8);
                    }
                }
            }
        }
    }

    fn gas_pass(&mut self) {
        for slot in 0..POOL_CAP {
            if !self.meta[slot].active || !self.meta[slot].lively {
                continue;
            }
            let base = slot * REGION_CELLS;
            if self.ch.gas[base..base + REGION_CELLS]
                .iter()
                .all(|g| *g == 0)
            {
                continue;
            }
            for cell in 0..REGION_CELLS {
                self.ch.scratch_u8[cell] = self.gas(slot, cell);
                self.set_gas(slot, cell, 0);
            }
            for cell in 0..REGION_CELLS {
                let g = i32::from(self.ch.scratch_u8[cell]);
                if g == 0 {
                    continue;
                }
                let give = g / 8;
                let mut rem = g - give * 6;
                let (x, y, z) = decode_index(cell);
                for (dx, dy, dz) in NEIGHBORS {
                    let nx = x + dx;
                    let ny = y + dy;
                    let nz = z + dz;
                    if in_region(nx, ny, nz) {
                        let dest = cell_index(nx, ny, nz);
                        self.add_gas(slot, dest, give);
                    } else {
                        rem += give;
                    }
                }
                self.add_gas(slot, cell, rem);
            }
            for cell in 0..REGION_CELLS {
                self.ch.scratch_u8[cell] = self.gas(slot, cell);
                self.set_gas(slot, cell, 0);
            }
            for cell in 0..REGION_CELLS {
                let g = i32::from(self.ch.scratch_u8[cell]);
                if g == 0 {
                    continue;
                }
                let (dx, dy, dz) = dominant_step(self.wind(slot, cell));
                let (x, y, z) = decode_index(cell);
                let nx = x + dx;
                let ny = y + dy;
                let nz = z + dz;
                if (dx != 0 || dy != 0 || dz != 0) && in_region(nx, ny, nz) {
                    let dest = cell_index(nx, ny, nz);
                    let space = i32::from(VOLUME_MAX) - i32::from(self.gas(slot, dest));
                    let mv = g.min(space.max(0));
                    self.add_gas(slot, dest, mv);
                    self.add_gas(slot, cell, g - mv);
                } else {
                    self.add_gas(slot, cell, g);
                }
            }
        }
    }

    fn heat_pass(&mut self, materials: &[MaterialView]) {
        for slot in 0..POOL_CAP {
            if !self.meta[slot].active || !self.meta[slot].lively {
                continue;
            }
            for cell in 0..REGION_CELLS {
                self.ch.scratch_i16[cell] = self.temp(slot, cell);
            }
            let mut next = [0i32; REGION_CELLS];
            for (cell, dest) in next.iter_mut().enumerate() {
                *dest = i32::from(self.ch.scratch_i16[cell]);
            }
            for cell in 0..REGION_CELLS {
                if self.ch.scratch_i16[cell] == 0 {
                    continue;
                }
                let (x, y, z) = decode_index(cell);
                let cond_here = conductivity_of(materials, self.material(slot, cell));
                for (dx, dy, dz) in NEIGHBORS {
                    let nx = x + dx;
                    let ny = y + dy;
                    let nz = z + dz;
                    if !in_region(nx, ny, nz) {
                        continue;
                    }
                    let other = cell_index(nx, ny, nz);
                    // One flux per edge. The lower index owns it when both ends are hot.
                    // A cold end does not visit, so the hot end owns that edge.
                    if self.ch.scratch_i16[other] != 0 && other < cell {
                        continue;
                    }
                    let cond_there = conductivity_of(materials, self.material(slot, other));
                    let cond = cond_here.min(cond_there);
                    // Six neighbor slots. Dividing by 6 * 256 keeps conductivity
                    // 255 from driving a hot cell through zero in one pass.
                    let flux = (i32::from(self.ch.scratch_i16[cell])
                        - i32::from(self.ch.scratch_i16[other]))
                        * cond
                        / (256 * 6);
                    next[cell] -= flux;
                    next[other] += flux;
                }
            }
            for (cell, temp) in next.iter().enumerate() {
                self.set_temp(slot, cell, sat_i16(*temp));
            }
        }
    }

    fn reconcile(&mut self, materials: &[MaterialView]) {
        for slot in 0..POOL_CAP {
            if !self.meta[slot].active || !self.meta[slot].lively {
                continue;
            }
            let origin = self.meta[slot].origin;
            for cell in 0..REGION_CELLS {
                let before = self.phase(slot, cell);
                let mut phase = before;
                let temp = self.temp(slot, cell);
                let volume = self.vol(slot, cell);
                let gas = self.gas(slot, cell);
                let mat = material_of(materials, self.material(slot, cell));
                let tags = mat.map(|m| m.tags).unwrap_or(0);
                let ignite = mat.map(|m| m.ignite).unwrap_or(i16::MAX);
                let freeze = mat.map(|m| m.freeze).unwrap_or(0);
                let mut flags = self.flags(slot, cell);
                if phase == PHASE_ICE && temp > freeze {
                    phase = PHASE_WET;
                    flags &= !FLAG_ICE;
                }
                if volume > 0 && phase != PHASE_ICE && phase != PHASE_BURN && phase != PHASE_OIL {
                    phase = PHASE_WET;
                }
                if volume == 0 && (phase == PHASE_WET || phase == PHASE_OIL) {
                    phase = PHASE_DRY;
                    flags &= !FLAG_WET;
                }
                if volume >= 128 && (phase == PHASE_WET || phase == PHASE_OIL) {
                    flags |= FLAG_WET;
                }
                if gas >= 128 && phase == PHASE_DRY {
                    phase = PHASE_GAS;
                }
                if gas == 0 && phase == PHASE_GAS {
                    phase = PHASE_DRY;
                }
                // Strictly below the freeze point. Ambient temp 0 is the freeze
                // point itself and stays liquid until a cold stamp undershoots it.
                if phase == PHASE_WET && temp < freeze && volume > 0 {
                    phase = PHASE_ICE;
                    flags |= FLAG_ICE;
                }
                if (phase == PHASE_DRY || phase == PHASE_OIL)
                    && tags & TAG_FLAMMABLE != 0
                    && temp >= ignite
                {
                    phase = PHASE_BURN;
                    flags |= FLAG_BURN;
                }
                if phase != PHASE_BURN {
                    flags &= !FLAG_BURN;
                }
                self.set_phase(slot, cell, phase);
                self.set_flags(slot, cell, flags);
                if phase != before {
                    let (x, y, z) = decode_index(cell);
                    let kind = if phase == PHASE_BURN {
                        Some(EVENT_IGNITED)
                    } else if phase == PHASE_ICE && before != PHASE_ICE {
                        Some(EVENT_FROZE)
                    } else if before == PHASE_BURN {
                        Some(EVENT_CLEARED_BURN)
                    } else {
                        None
                    };
                    if let Some(kind) = kind {
                        self.push_event(FieldEvent {
                            x: origin[0] + x,
                            y: origin[1] + y,
                            z: origin[2] + z,
                            kind,
                            material_id: self.material(slot, cell),
                            temp,
                        });
                    }
                }
                if tags & TAG_CONDUCTIVE != 0 {
                    let (x, y, z) = decode_index(cell);
                    let mut shocked = false;
                    for (dx, dy, dz) in NEIGHBORS {
                        let nx = x + dx;
                        let ny = y + dy;
                        let nz = z + dz;
                        if in_region(nx, ny, nz)
                            && self.phase(slot, cell_index(nx, ny, nz)) == PHASE_BURN
                        {
                            shocked = true;
                            break;
                        }
                    }
                    if shocked {
                        self.push_event(FieldEvent {
                            x: origin[0] + x,
                            y: origin[1] + y,
                            z: origin[2] + z,
                            kind: EVENT_SHOCKED,
                            material_id: self.material(slot, cell),
                            temp,
                        });
                    }
                }
            }
        }
    }

    fn push_event(&mut self, ev: FieldEvent) {
        let i = self.event_len as usize;
        if i >= MAX_FIELD_EVENTS {
            self.event_drops = self.event_drops.saturating_add(1);
            return;
        }
        self.events[i] = ev;
        self.event_len += 1;
    }

    fn mark_lively(&mut self) {
        for slot in 0..POOL_CAP {
            if !self.meta[slot].active || !self.meta[slot].lively {
                continue;
            }
            let mut lively = false;
            for cell in 0..REGION_CELLS {
                if self.vol(slot, cell) > 0
                    || self.gas(slot, cell) > 0
                    || self.temp(slot, cell) != 0
                    || self.phase(slot, cell) != PHASE_DRY
                    || self.flags(slot, cell) != 0
                {
                    lively = true;
                    break;
                }
            }
            self.meta[slot].lively = lively;
        }
    }

    fn sleep_inactive(&mut self) {
        for slot in 0..POOL_CAP {
            if !self.meta[slot].occupied || self.meta[slot].active {
                continue;
            }
            if self.meta[slot].sleeping {
                continue;
            }
            self.encode_slot(slot);
            if !self.meta[slot].dense_sleep {
                self.zero_channels(slot);
            }
            self.meta[slot].sleeping = true;
        }
    }

    fn encode_slot(&mut self, slot: usize) {
        let mut n = 0usize;
        let mut overflow = false;
        for cell in 0..REGION_CELLS {
            if !self.cell_dirty(slot, cell) {
                continue;
            }
            if n >= SLEEP_CAP {
                overflow = true;
                break;
            }
            let packed = SleepCell {
                index: cell as u16,
                temp: self.temp(slot, cell),
                phase: self.phase(slot, cell),
                volume: self.vol(slot, cell),
                gas: self.gas(slot, cell),
                flags: self.flags(slot, cell),
                density_delta: self.density(slot, cell),
                gravity: self.gravity(slot, cell),
                wind: self.wind(slot, cell),
                material: self.material(slot, cell),
            };
            self.set_sleep(slot, n, packed);
            n += 1;
        }
        if overflow {
            self.meta[slot].dense_sleep = true;
            self.meta[slot].sleep_len = 0;
        } else {
            self.meta[slot].dense_sleep = false;
            self.meta[slot].sleep_len = n as u16;
        }
    }

    fn decode_slot(&mut self, slot: usize) {
        if self.meta[slot].dense_sleep {
            return;
        }
        let len = self.meta[slot].sleep_len as usize;
        let mut saved = [SleepCell::ZERO; SLEEP_CAP];
        for (i, dest) in saved.iter_mut().enumerate().take(len) {
            *dest = self.sleep_at(slot, i);
        }
        self.zero_channels(slot);
        for cell in &saved[..len] {
            let i = cell.index as usize;
            if i >= REGION_CELLS {
                continue;
            }
            self.set_temp(slot, i, cell.temp);
            self.set_phase(slot, i, cell.phase);
            self.set_vol(slot, i, cell.volume);
            self.set_gas(slot, i, cell.gas);
            self.set_flags(slot, i, cell.flags);
            self.set_density(slot, i, cell.density_delta);
            self.set_gravity(slot, i, cell.gravity);
            self.set_wind(slot, i, cell.wind);
            self.set_material(slot, i, cell.material);
        }
    }

    fn ensure_raw(&mut self, slot: usize) {
        if self.meta[slot].sleeping && !self.meta[slot].dense_sleep {
            self.decode_slot(slot);
        }
        self.meta[slot].sleeping = false;
    }

    fn publish(&mut self, subjects: &[Subject], materials: &[MaterialView]) {
        self.column_len = 0;
        self.column_drops = 0;
        self.hazard_len = 0;
        let anchor = subjects
            .iter()
            .find(|s| s.player)
            .or_else(|| subjects.first())
            .map(|s| s.cell)
            .unwrap_or([0, 0, 0]);
        for slot in 0..POOL_CAP {
            if !self.meta[slot].occupied {
                continue;
            }
            let origin = self.meta[slot].origin;
            if self.meta[slot].sleeping && !self.meta[slot].dense_sleep {
                let len = self.meta[slot].sleep_len as usize;
                for s in 0..len {
                    let cell = self.sleep_at(slot, s);
                    let (x, y, z) = decode_index(cell.index as usize);
                    let world = [origin[0] + x, origin[1] + y, origin[2] + z];
                    self.consider_column(
                        world,
                        cell.volume,
                        cell.phase,
                        cell.density_delta,
                        cell.material,
                        materials,
                    );
                    self.consider_hazard(world, cell.phase, cell.temp, cell.flags, anchor);
                }
            } else if self.meta[slot].lively || self.meta[slot].dense_sleep {
                for cell in 0..REGION_CELLS {
                    let (x, y, z) = decode_index(cell);
                    let world = [origin[0] + x, origin[1] + y, origin[2] + z];
                    self.consider_column(
                        world,
                        self.vol(slot, cell),
                        self.phase(slot, cell),
                        self.density(slot, cell),
                        self.material(slot, cell),
                        materials,
                    );
                    self.consider_hazard(
                        world,
                        self.phase(slot, cell),
                        self.temp(slot, cell),
                        self.flags(slot, cell),
                        anchor,
                    );
                }
            }
        }
        let n = self.column_len as usize;
        sort_columns(&mut self.columns[..n]);
        let mut w = 0usize;
        let mut i = 0usize;
        while i < n {
            let mut best = self.columns[i];
            let mut j = i + 1;
            while j < n && self.columns[j].x == best.x && self.columns[j].z == best.z {
                if self.columns[j].surface_y > best.surface_y {
                    best = self.columns[j];
                }
                j += 1;
            }
            self.columns[w] = best;
            w += 1;
            i = j;
        }
        self.column_len = w as u16;
    }

    fn consider_column(
        &mut self,
        world: [i32; 3],
        volume: u8,
        phase: u8,
        density_delta: i8,
        material: u16,
        materials: &[MaterialView],
    ) {
        let wet = volume > 0 && (phase == PHASE_WET || phase == PHASE_OIL || phase == PHASE_ICE);
        if !wet {
            return;
        }
        if self.column_len as usize >= MAX_COLUMNS {
            self.column_drops = self.column_drops.saturating_add(1);
            return;
        }
        let mat = material_of(materials, material);
        let base = i32::from(mat.map(|m| m.density0).unwrap_or(1000));
        let density = if phase == PHASE_ICE {
            0.0
        } else {
            (base + i32::from(density_delta)).clamp(0, 4000) as f32
        };
        let i = self.column_len as usize;
        self.columns[i] = WaterColumn {
            x: world[0],
            z: world[2],
            surface_y: (world[1] + 1) as f32,
            density,
            ice: phase == PHASE_ICE,
        };
        self.column_len += 1;
    }

    fn consider_hazard(
        &mut self,
        world: [i32; 3],
        phase: u8,
        temp: i16,
        flags: u8,
        anchor: [i32; 3],
    ) {
        if phase == PHASE_DRY && flags == 0 {
            return;
        }
        let dist = (world[0] - anchor[0]).abs()
            + (world[1] - anchor[1]).abs()
            + (world[2] - anchor[2]).abs();
        let sample = HazardSample {
            cell: world,
            phase,
            temp,
            flags,
        };
        let len = self.hazard_len as usize;
        if len < MAX_HAZARDS {
            self.hazards[len] = sample;
            self.hazard_len += 1;
            self.sort_hazards(anchor);
            return;
        }
        let worst = self.hazard_rank(MAX_HAZARDS - 1, anchor);
        if (dist, world[2], world[1], world[0]) < worst {
            self.hazards[MAX_HAZARDS - 1] = sample;
            self.sort_hazards(anchor);
        }
    }

    fn sort_hazards(&mut self, anchor: [i32; 3]) {
        let n = self.hazard_len as usize;
        for i in 1..n {
            let mut j = i;
            while j > 0 {
                let a = self.hazard_rank(j - 1, anchor);
                let b = self.hazard_rank(j, anchor);
                if a <= b {
                    break;
                }
                self.hazards.swap(j - 1, j);
                j -= 1;
            }
        }
    }

    fn hazard_rank(&self, i: usize, anchor: [i32; 3]) -> (i32, i32, i32, i32) {
        let c = self.hazards[i].cell;
        let dist = (c[0] - anchor[0]).abs() + (c[1] - anchor[1]).abs() + (c[2] - anchor[2]).abs();
        (dist, c[2], c[1], c[0])
    }

    fn shift_slot(&mut self, slot: usize, new_origin: [i32; 3]) {
        let old = self.meta[slot].origin;
        let dx = new_origin[0] - old[0];
        let dy = new_origin[1] - old[1];
        let dz = new_origin[2] - old[2];
        if dx == 0 && dy == 0 && dz == 0 {
            return;
        }
        self.shift_i16(slot, dx, dy, dz, Channel::Temp);
        self.shift_u8(slot, dx, dy, dz, Channel::Phase);
        self.shift_u8(slot, dx, dy, dz, Channel::Volume);
        self.shift_u8(slot, dx, dy, dz, Channel::Gas);
        self.shift_u8(slot, dx, dy, dz, Channel::Flags);
        self.shift_i16(slot, dx, dy, dz, Channel::Density);
        self.shift_u16_grav(slot, dx, dy, dz);
        self.shift_wind(slot, dx, dy, dz);
    }

    fn shift_i16(&mut self, slot: usize, dx: i32, dy: i32, dz: i32, which: Channel) {
        for z in 0..REGION_EDGE {
            for y in 0..REGION_EDGE {
                for x in 0..REGION_EDGE {
                    let ox = x + dx;
                    let oy = y + dy;
                    let oz = z + dz;
                    let v = if in_region(ox, oy, oz) {
                        let src = cell_index(ox, oy, oz);
                        match which {
                            Channel::Temp => self.temp(slot, src),
                            Channel::Density => i16::from(self.density(slot, src)),
                            _ => 0,
                        }
                    } else {
                        0
                    };
                    self.ch.scratch_i16[cell_index(x, y, z)] = v;
                }
            }
        }
        for cell in 0..REGION_CELLS {
            match which {
                Channel::Temp => self.set_temp(slot, cell, self.ch.scratch_i16[cell]),
                Channel::Density => self.set_density(slot, cell, self.ch.scratch_i16[cell] as i8),
                _ => {}
            }
        }
    }

    fn shift_u8(&mut self, slot: usize, dx: i32, dy: i32, dz: i32, which: Channel) {
        for z in 0..REGION_EDGE {
            for y in 0..REGION_EDGE {
                for x in 0..REGION_EDGE {
                    let ox = x + dx;
                    let oy = y + dy;
                    let oz = z + dz;
                    let v = if in_region(ox, oy, oz) {
                        let src = cell_index(ox, oy, oz);
                        match which {
                            Channel::Phase => self.phase(slot, src),
                            Channel::Volume => self.vol(slot, src),
                            Channel::Gas => self.gas(slot, src),
                            Channel::Flags => self.flags(slot, src),
                            _ => 0,
                        }
                    } else {
                        0
                    };
                    self.ch.scratch_u8[cell_index(x, y, z)] = v;
                }
            }
        }
        for cell in 0..REGION_CELLS {
            let v = self.ch.scratch_u8[cell];
            match which {
                Channel::Phase => self.set_phase(slot, cell, v),
                Channel::Volume => self.set_vol(slot, cell, v),
                Channel::Gas => self.set_gas(slot, cell, v),
                Channel::Flags => self.set_flags(slot, cell, v),
                _ => {}
            }
        }
    }

    fn shift_u16_grav(&mut self, slot: usize, dx: i32, dy: i32, dz: i32) {
        let mut tmp = [0u16; REGION_CELLS];
        for z in 0..REGION_EDGE {
            for y in 0..REGION_EDGE {
                for x in 0..REGION_EDGE {
                    let ox = x + dx;
                    let oy = y + dy;
                    let oz = z + dz;
                    let v = if in_region(ox, oy, oz) {
                        self.gravity(slot, cell_index(ox, oy, oz))
                    } else {
                        0
                    };
                    tmp[cell_index(x, y, z)] = v;
                }
            }
        }
        for (cell, value) in tmp.iter().enumerate() {
            self.set_gravity(slot, cell, *value);
        }
    }

    fn shift_wind(&mut self, slot: usize, dx: i32, dy: i32, dz: i32) {
        let mut tmp = [[0i8; 3]; REGION_CELLS];
        for z in 0..REGION_EDGE {
            for y in 0..REGION_EDGE {
                for x in 0..REGION_EDGE {
                    let ox = x + dx;
                    let oy = y + dy;
                    let oz = z + dz;
                    let v = if in_region(ox, oy, oz) {
                        self.wind(slot, cell_index(ox, oy, oz))
                    } else {
                        [0, 0, 0]
                    };
                    tmp[cell_index(x, y, z)] = v;
                }
            }
        }
        for (cell, value) in tmp.iter().enumerate() {
            self.set_wind(slot, cell, *value);
        }
    }

    fn cell_dirty(&self, slot: usize, cell: usize) -> bool {
        self.temp(slot, cell) != 0
            || self.phase(slot, cell) != 0
            || self.vol(slot, cell) != 0
            || self.gas(slot, cell) != 0
            || self.flags(slot, cell) != 0
            || self.density(slot, cell) != 0
            || self.gravity(slot, cell) != 0
            || self.wind(slot, cell) != [0, 0, 0]
    }

    fn zero_slot(&mut self, slot: usize) {
        self.zero_channels(slot);
        self.meta[slot].sleep_len = 0;
        self.meta[slot].dense_sleep = false;
        self.meta[slot].sleeping = false;
    }

    fn zero_channels(&mut self, slot: usize) {
        for cell in 0..REGION_CELLS {
            self.set_temp(slot, cell, 0);
            self.set_phase(slot, cell, 0);
            self.set_density(slot, cell, 0);
            self.set_flags(slot, cell, 0);
            self.set_gravity(slot, cell, 0);
            self.set_vol(slot, cell, 0);
            self.set_gas(slot, cell, 0);
            self.set_wind(slot, cell, [0, 0, 0]);
            self.set_material(slot, cell, 0);
            self.set_solid(slot, cell, 0);
        }
    }

    #[inline]
    fn ix(&self, slot: usize, cell: usize) -> usize {
        let _ = self;
        slot * REGION_CELLS + cell
    }
    fn temp(&self, slot: usize, cell: usize) -> i16 {
        self.ch.temp[self.ix(slot, cell)]
    }
    fn set_temp(&mut self, slot: usize, cell: usize, v: i16) {
        let i = self.ix(slot, cell);
        self.ch.temp[i] = v;
    }
    fn phase(&self, slot: usize, cell: usize) -> u8 {
        self.ch.phase[self.ix(slot, cell)]
    }
    fn set_phase(&mut self, slot: usize, cell: usize, v: u8) {
        let i = self.ix(slot, cell);
        self.ch.phase[i] = v;
    }
    fn density(&self, slot: usize, cell: usize) -> i8 {
        self.ch.density_delta[self.ix(slot, cell)]
    }
    fn set_density(&mut self, slot: usize, cell: usize, v: i8) {
        let i = self.ix(slot, cell);
        self.ch.density_delta[i] = v;
    }
    fn flags(&self, slot: usize, cell: usize) -> u8 {
        self.ch.flags[self.ix(slot, cell)]
    }
    fn set_flags(&mut self, slot: usize, cell: usize, v: u8) {
        let i = self.ix(slot, cell);
        self.ch.flags[i] = v;
    }
    fn gravity(&self, slot: usize, cell: usize) -> u16 {
        self.ch.gravity[self.ix(slot, cell)]
    }
    fn set_gravity(&mut self, slot: usize, cell: usize, v: u16) {
        let i = self.ix(slot, cell);
        self.ch.gravity[i] = v;
    }
    fn vol(&self, slot: usize, cell: usize) -> u8 {
        self.ch.volume[self.ix(slot, cell)]
    }
    fn set_vol(&mut self, slot: usize, cell: usize, v: u8) {
        let i = self.ix(slot, cell);
        self.ch.volume[i] = v;
    }
    fn add_vol(&mut self, slot: usize, cell: usize, dv: i32) {
        if dv == 0 {
            return;
        }
        let v = i32::from(self.vol(slot, cell)) + dv;
        self.set_vol(slot, cell, sat_u8_from_i32(v));
    }
    fn gas(&self, slot: usize, cell: usize) -> u8 {
        self.ch.gas[self.ix(slot, cell)]
    }
    fn set_gas(&mut self, slot: usize, cell: usize, v: u8) {
        let i = self.ix(slot, cell);
        self.ch.gas[i] = v;
    }
    fn add_gas(&mut self, slot: usize, cell: usize, dv: i32) {
        if dv == 0 {
            return;
        }
        let v = i32::from(self.gas(slot, cell)) + dv;
        self.set_gas(slot, cell, sat_u8_from_i32(v));
    }
    fn wind(&self, slot: usize, cell: usize) -> [i8; 3] {
        let i = (slot * REGION_CELLS + cell) * 3;
        [self.ch.wind[i], self.ch.wind[i + 1], self.ch.wind[i + 2]]
    }
    fn set_wind(&mut self, slot: usize, cell: usize, w: [i8; 3]) {
        let i = (slot * REGION_CELLS + cell) * 3;
        self.ch.wind[i] = w[0];
        self.ch.wind[i + 1] = w[1];
        self.ch.wind[i + 2] = w[2];
    }
    fn material(&self, slot: usize, cell: usize) -> u16 {
        self.ch.material[self.ix(slot, cell)]
    }
    fn set_material(&mut self, slot: usize, cell: usize, v: u16) {
        let i = self.ix(slot, cell);
        self.ch.material[i] = v;
    }
    fn set_solid(&mut self, slot: usize, cell: usize, v: u8) {
        let i = self.ix(slot, cell);
        self.ch.solid[i] = v;
    }
    fn solid_cell(&self, slot: usize, cell: usize) -> bool {
        self.ch.solid[self.ix(slot, cell)] != 0
    }
    fn sleep_at(&self, slot: usize, i: usize) -> SleepCell {
        self.ch.sleep[slot * SLEEP_CAP + i]
    }
    fn set_sleep(&mut self, slot: usize, i: usize, cell: SleepCell) {
        self.ch.sleep[slot * SLEEP_CAP + i] = cell;
    }
}

impl Default for LocalSim {
    fn default() -> Self {
        Self::new()
    }
}

enum Channel {
    Temp,
    Density,
    Phase,
    Volume,
    Gas,
    Flags,
}

const NEIGHBORS: [(i32, i32, i32); 6] = [
    (1, 0, 0),
    (-1, 0, 0),
    (0, 1, 0),
    (0, -1, 0),
    (0, 0, 1),
    (0, 0, -1),
];

fn region_origin(cell: [i32; 3]) -> [i32; 3] {
    [cell[0] - 8, cell[1] - 8, cell[2] - 8]
}

fn local_cell(origin: [i32; 3], world: [i32; 3]) -> Option<usize> {
    let x = world[0] - origin[0];
    let y = world[1] - origin[1];
    let z = world[2] - origin[2];
    if in_region(x, y, z) {
        Some(cell_index(x, y, z))
    } else {
        None
    }
}

fn in_region(x: i32, y: i32, z: i32) -> bool {
    (0..REGION_EDGE).contains(&x) && (0..REGION_EDGE).contains(&y) && (0..REGION_EDGE).contains(&z)
}

fn conductivity_of(materials: &[MaterialView], id: u16) -> i32 {
    i32::from(
        material_of(materials, id)
            .map(|m| m.conductivity)
            .unwrap_or(0),
    )
}

fn fold_side(any: bool, stack: bool, sum: i32, max_mag: i32) -> i32 {
    if !any {
        return 0;
    }
    if stack {
        sum + max_mag
    } else {
        max_mag
    }
}

fn gravity_step(
    index: u16,
    zones: &[GravityView],
    origin: [i32; 3],
    cell: usize,
) -> Option<(i32, i32, i32)> {
    if index == 0 {
        return Some((0, -1, 0));
    }
    let zone = zones.iter().find(|z| z.index == index)?;
    match zone.axis {
        1 => None,
        2 => Some((1, 0, 0)),
        3 => Some((-1, 0, 0)),
        4 => Some((0, 1, 0)),
        5 => Some((0, -1, 0)),
        6 => Some((0, 0, 1)),
        7 => Some((0, 0, -1)),
        8 => point_step(origin, cell, zone.center),
        _ => Some((0, -1, 0)),
    }
}

fn point_step(origin: [i32; 3], cell: usize, center: [i32; 3]) -> Option<(i32, i32, i32)> {
    let (x, y, z) = decode_index(cell);
    let world = [origin[0] + x, origin[1] + y, origin[2] + z];
    let mut best: Option<(i32, (i32, i32, i32))> = None;
    let here =
        (world[0] - center[0]).abs() + (world[1] - center[1]).abs() + (world[2] - center[2]).abs();
    for (dx, dy, dz) in NEIGHBORS {
        let nx = world[0] + dx;
        let ny = world[1] + dy;
        let nz = world[2] + dz;
        let d = (nx - center[0]).abs() + (ny - center[1]).abs() + (nz - center[2]).abs();
        if d < here {
            let step = (dx, dy, dz);
            let rank = match step {
                (1, 0, 0) => 0,
                (-1, 0, 0) => 1,
                (0, 1, 0) => 2,
                (0, -1, 0) => 3,
                (0, 0, 1) => 4,
                _ => 5,
            };
            if best.map(|(r, _)| rank < r).unwrap_or(true) {
                best = Some((rank, step));
            }
        }
    }
    best.map(|(_, s)| s)
}

fn dominant_step(wind: [i8; 3]) -> (i32, i32, i32) {
    let ax = i32::from(wind[0]).abs();
    let ay = i32::from(wind[1]).abs();
    let az = i32::from(wind[2]).abs();
    if ax == 0 && ay == 0 && az == 0 {
        return (0, 0, 0);
    }
    if ax >= ay && ax >= az {
        (wind[0].signum() as i32, 0, 0)
    } else if ay >= az {
        (0, wind[1].signum() as i32, 0)
    } else {
        (0, 0, wind[2].signum() as i32)
    }
}

fn column_ord(a: &WaterColumn, b: &WaterColumn) -> core::cmp::Ordering {
    a.x.cmp(&b.x)
        .then(a.z.cmp(&b.z))
        .then(b.surface_y.total_cmp(&a.surface_y))
}

/// In-place heapsort. `slice::sort_by` allocates a merge buffer, and `tick` must not.
fn sort_columns(cols: &mut [WaterColumn]) {
    let n = cols.len();
    if n < 2 {
        return;
    }
    for i in (0..n / 2).rev() {
        sift_columns(cols, i, n);
    }
    for end in (1..n).rev() {
        cols.swap(0, end);
        sift_columns(cols, 0, end);
    }
}

fn sift_columns(cols: &mut [WaterColumn], mut root: usize, end: usize) {
    loop {
        let left = root * 2 + 1;
        if left >= end {
            break;
        }
        let right = left + 1;
        let mut child = left;
        if right < end && column_ord(&cols[right], &cols[left]).is_gt() {
            child = right;
        }
        if column_ord(&cols[root], &cols[child]).is_ge() {
            break;
        }
        cols.swap(root, child);
        root = child;
    }
}

fn hash_sleep(hasher: &mut Sha256, cell: &SleepCell) {
    hasher.update(cell.index.to_le_bytes());
    hasher.update(cell.temp.to_le_bytes());
    hasher.update([cell.phase, cell.volume, cell.gas, cell.flags]);
    hasher.update(cell.density_delta.to_le_bytes());
    hasher.update(cell.gravity.to_le_bytes());
    hasher.update([cell.wind[0] as u8, cell.wind[1] as u8, cell.wind[2] as u8]);
    hasher.update(cell.material.to_le_bytes());
}

fn hash_raw_slot(hasher: &mut Sha256, sim: &LocalSim, slot: usize) {
    // Same bytes as the per-cell record, packed so SHA-256 sees long inputs.
    // Little-endian integers keep the three replay OSes on one digest.
    let mut buf = [0u8; 64 * 14];
    let mut n = 0usize;
    for cell in 0..REGION_CELLS {
        if n + 14 > buf.len() {
            hasher.update(&buf[..n]);
            n = 0;
        }
        let temp = sim.temp(slot, cell).to_le_bytes();
        buf[n] = temp[0];
        buf[n + 1] = temp[1];
        buf[n + 2] = sim.phase(slot, cell);
        buf[n + 3] = sim.vol(slot, cell);
        buf[n + 4] = sim.gas(slot, cell);
        buf[n + 5] = sim.flags(slot, cell);
        buf[n + 6] = sim.density(slot, cell).to_le_bytes()[0];
        let grav = sim.gravity(slot, cell).to_le_bytes();
        buf[n + 7] = grav[0];
        buf[n + 8] = grav[1];
        let w = sim.wind(slot, cell);
        buf[n + 9] = w[0] as u8;
        buf[n + 10] = w[1] as u8;
        buf[n + 11] = w[2] as u8;
        let mat = sim.material(slot, cell).to_le_bytes();
        buf[n + 12] = mat[0];
        buf[n + 13] = mat[1];
        n += 14;
    }
    if n > 0 {
        hasher.update(&buf[..n]);
    }
}

/// Materials used by the replay case and the crate tests.
pub fn demo_materials() -> [MaterialView; 3] {
    [
        MaterialView {
            id: 0,
            tags: 0,
            ignite: 30_000,
            freeze: 0,
            density0: 0,
            conductivity: 1,
        },
        MaterialView {
            id: 1,
            tags: TAG_FLAMMABLE | TAG_FREEZABLE | TAG_WETTABLE,
            ignite: 800,
            freeze: 0,
            density0: 1000,
            conductivity: 8,
        },
        MaterialView {
            id: 2,
            tags: TAG_CONDUCTIVE,
            ignite: 30_000,
            freeze: 0,
            density0: 1000,
            conductivity: 200,
        },
    ]
}

/// 300-tick script hashed by `determinism-replay`.
pub fn local_sim_case_bytes() -> Vec<u8> {
    let mut sim = LocalSim::new();
    let materials = demo_materials();
    let mut solid = FloorSolid::closed(0);
    let player = Subject {
        id: 1,
        cell: [0, 0, 0],
        player: true,
    };
    let mut out = Vec::with_capacity(300 * 32);
    for tick in 0..300u64 {
        let mut writes = [FieldWrite {
            seq: 0,
            shape: Shape::Point,
            origin: [0, 0, 0],
            end: [0, 0, 0],
            radius: 0,
            op: WriteOp::AddVolume { delta: 0 },
        }; 4];
        let n = if tick == 0 {
            writes[0] = FieldWrite {
                seq: 0,
                shape: Shape::Point,
                origin: [0, 4, 0],
                end: [0, 4, 0],
                radius: 0,
                op: WriteOp::AddVolume { delta: 200 },
            };
            writes[1] = FieldWrite {
                seq: 1,
                shape: Shape::Point,
                origin: [0, 4, 0],
                end: [0, 4, 0],
                radius: 0,
                op: WriteOp::SetPhase { phase: PHASE_WET },
            };
            writes[2] = FieldWrite {
                seq: 2,
                shape: Shape::Point,
                origin: [2, 1, 0],
                end: [2, 1, 0],
                radius: 0,
                op: WriteOp::SetWind { x: 1, y: 0, z: 0 },
            };
            writes[3] = FieldWrite {
                seq: 3,
                shape: Shape::Point,
                origin: [2, 1, 0],
                end: [2, 1, 0],
                radius: 0,
                op: WriteOp::AddTemp {
                    delta: 1000,
                    stack: true,
                },
            };
            4
        } else if tick == 80 {
            solid.hole = Some([0, 0, 0]);
            0
        } else if tick == 120 {
            writes[0] = FieldWrite {
                seq: 0,
                shape: Shape::Point,
                origin: [0, 1, 0],
                end: [0, 1, 0],
                radius: 0,
                op: WriteOp::AddTemp {
                    delta: -500,
                    stack: true,
                },
            };
            1
        } else {
            0
        };
        let subjects: &[Subject] = if (150..200).contains(&tick) {
            &[]
        } else {
            &[player]
        };
        sim.tick(
            tick,
            TickInput {
                subjects,
                writes: &writes[..n],
                materials: &materials,
                zones: &[],
            },
            &solid,
        );
        out.extend_from_slice(&sim.field_digest());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn volume_above(sim: &LocalSim, min_y: i32) -> u32 {
        let mut sum = 0u32;
        for x in -8..8 {
            for y in min_y..8 {
                for z in -8..8 {
                    sum += u32::from(sim.volume_at([x, y, z]));
                }
            }
        }
        sum
    }

    fn volume_below(sim: &LocalSim, max_y: i32) -> u32 {
        let mut sum = 0u32;
        for x in -8..8 {
            for y in -8..max_y {
                for z in -8..8 {
                    sum += u32::from(sim.volume_at([x, y, z]));
                }
            }
        }
        sum
    }

    fn volume_sum(sim: &LocalSim) -> u32 {
        let mut sum = 0u32;
        for x in -8..8 {
            for y in -8..8 {
                for z in -8..8 {
                    sum += u32::from(sim.volume_at([x, y, z]));
                }
            }
        }
        sum
    }

    fn input<'a>(
        subjects: &'a [Subject],
        writes: &'a [FieldWrite],
        materials: &'a [MaterialView],
    ) -> TickInput<'a> {
        TickInput {
            subjects,
            writes,
            materials,
            zones: &[],
        }
    }

    #[test]
    fn water_falls_onto_the_floor_and_then_into_a_breach() {
        let mut sim = LocalSim::new();
        let materials = demo_materials();
        let mut solid = FloorSolid::closed(0);
        let player = [Subject {
            id: 1,
            cell: [0, 0, 0],
            player: true,
        }];
        let pour = [FieldWrite {
            seq: 0,
            shape: Shape::Point,
            origin: [0, 4, 0],
            end: [0, 4, 0],
            radius: 0,
            op: WriteOp::AddVolume { delta: 180 },
        }];
        sim.tick(0, input(&player, &pour, &materials), &solid);
        for t in 1..6 {
            sim.tick(t, input(&player, &[], &materials), &solid);
        }
        assert_eq!(
            sim.volume_at([0, 4, 0]),
            0,
            "the pour cell should have drained"
        );
        let pooled = volume_above(&sim, 1);
        assert!(pooled > 0, "volume sits above the closed floor");
        assert_eq!(volume_below(&sim, 1), 0);
        solid.open_floor = true;
        sim.tick(6, input(&player, &[], &materials), &solid);
        assert!(
            volume_below(&sim, 1) > 0,
            "opening the floor lets volume fall through"
        );
        assert!(volume_above(&sim, 1) < pooled);
    }

    #[test]
    fn gas_diffuses_and_advects_with_wind() {
        let mut sim = LocalSim::new();
        let materials = demo_materials();
        let solid = FloorSolid::closed(-20);
        let player = [Subject {
            id: 1,
            cell: [0, 0, 0],
            player: true,
        }];
        let writes = [
            FieldWrite {
                seq: 0,
                shape: Shape::Point,
                origin: [0, 0, 0],
                end: [0, 0, 0],
                radius: 0,
                op: WriteOp::SetPhase { phase: PHASE_GAS },
            },
            FieldWrite {
                seq: 1,
                shape: Shape::Sphere,
                origin: [0, 0, 0],
                end: [0, 0, 0],
                radius: 2,
                op: WriteOp::SetWind { x: 1, y: 0, z: 0 },
            },
        ];
        sim.tick(0, input(&player, &writes, &materials), &solid);
        sim.tick(1, input(&player, &[], &materials), &solid);
        let mut plus = 0i32;
        let mut minus = 0i32;
        for x in 1..8 {
            plus += i32::from(sim.gas_at([x, 0, 0]));
        }
        for x in -8..0 {
            minus += i32::from(sim.gas_at([x, 0, 0]));
        }
        assert!(
            plus > minus,
            "wind carries more gas toward +X ({plus}) than -X ({minus})"
        );
        let mut sum = 0i32;
        for x in -8..8 {
            for y in -8..8 {
                for z in -8..8 {
                    sum += i32::from(sim.gas_at([x, y, z]));
                }
            }
        }
        assert!(sum > 0);
    }

    #[test]
    fn heat_follows_material_conductivity() {
        let mut sim = LocalSim::new();
        let materials = demo_materials();
        let solid = ConductivitySolid;
        let player = [Subject {
            id: 1,
            cell: [0, 0, 0],
            player: true,
        }];
        let heat = [FieldWrite {
            seq: 0,
            shape: Shape::Point,
            origin: [0, 0, 0],
            end: [0, 0, 0],
            radius: 0,
            op: WriteOp::AddTemp {
                delta: 2000,
                stack: true,
            },
        }];
        sim.tick(0, input(&player, &heat, &materials), &solid);
        for t in 1..8 {
            sim.tick(t, input(&player, &[], &materials), &solid);
        }
        let metal = sim.temp_at([1, 0, 0]);
        let wood = sim.temp_at([-1, 0, 0]);
        assert!(
            metal > wood,
            "metal neighbor {metal} should exceed wood neighbor {wood}"
        );
    }

    struct ConductivitySolid;
    impl SolidField for ConductivitySolid {
        fn solid(&self, _: i32, _: i32, _: i32) -> bool {
            false
        }
        fn material_id(&self, x: i32, _: i32, _: i32) -> u16 {
            if x >= 0 {
                2
            } else {
                1
            }
        }
    }

    #[test]
    fn sleep_encoding_wakes_to_the_same_bytes() {
        let mut sim = LocalSim::new();
        let materials = demo_materials();
        let solid = FloorSolid::closed(-20);
        let player = [Subject {
            id: 7,
            cell: [0, 0, 0],
            player: true,
        }];
        let pour = [FieldWrite {
            seq: 0,
            shape: Shape::Point,
            origin: [1, 2, 3],
            end: [1, 2, 3],
            radius: 0,
            op: WriteOp::AddVolume { delta: 40 },
        }];
        sim.tick(0, input(&player, &pour, &materials), &solid);
        let before = volume_sum(&sim);
        assert_eq!(before, 40);
        sim.tick(1, input(&[], &[], &materials), &solid);
        assert!(sim.occupied_count() >= 1);
        assert_eq!(sim.active_count(), 0, "no subject means the region sleeps");
        let slept = sim.field_digest();
        sim.tick(2, input(&player, &[], &materials), &solid);
        assert_eq!(
            volume_sum(&sim),
            40,
            "waking restores the same fluid quanta"
        );
        assert_ne!(sim.field_digest(), slept);
    }

    #[test]
    fn active_budget_keeps_the_player_and_is_stable() {
        let mut sim = LocalSim::new();
        let materials = demo_materials();
        let solid = FloorSolid::closed(-20);
        let mut subjects = [Subject {
            id: 0,
            cell: [0, 0, 0],
            player: false,
        }; 40];
        subjects[0] = Subject {
            id: 1,
            cell: [0, 0, 0],
            player: true,
        };
        for (i, sub) in subjects.iter_mut().enumerate().skip(1) {
            *sub = Subject {
                id: 100 + i as u64,
                cell: [i as i32 * 30, 0, 0],
                player: false,
            };
        }
        sim.tick(0, input(&subjects, &[], &materials), &solid);
        assert_eq!(sim.active_count() as usize, ACTIVE_BUDGET);
        assert!(sim.meta.iter().any(|m| m.player && m.active));
        let a = sim.field_digest();
        let mut sim_b = LocalSim::new();
        sim_b.tick(0, input(&subjects, &[], &materials), &solid);
        assert_eq!(a, sim_b.field_digest());
    }

    #[test]
    fn digest_matches_across_two_runs() {
        let a = local_sim_case_bytes();
        let b = local_sim_case_bytes();
        assert_eq!(a, b);
        assert_eq!(a.len(), 300 * 32);
    }
}
