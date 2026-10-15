//! Fixed rings. `tick` writes slots that already exist.

use astraweave_core::{SimRng, SIM_SYSTEM_CAUSAL};
use sha2::{Digest, Sha256};

use crate::{
    CausalInput, CausalRule, COMPLETE_LINE, KIND_NPC_STANCE, KIND_QUEST_COMPLETE, KIND_QUEST_OFFER,
    KIND_ROUTE_CLASS, OFFER_LINE,
};

pub const MAX_CAUSES: usize = 4;
pub const MAX_EVENTS: usize = 256;
pub const MAX_PENDING: usize = 64;
pub const MAX_INPUTS: usize = 32;
pub const MAX_RULES: usize = 32;
pub const MAX_CASCADE_DEPTH: usize = 4;
pub const MAX_LINES: usize = 4;
const LINE_CAP: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CausalError {
    RingFull,
    PendingFull,
    InputFull,
    LinesFull,
    LineTooLong,
    CascadeOverflow,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CausalEvent {
    pub id: u32,
    pub tick: u64,
    pub seq: u32,
    pub kind: u16,
    pub class: u16,
    pub subject: u64,
    pub cause_count: u8,
    pub cause_ids: [u32; MAX_CAUSES],
    pub region: [i32; 6],
    pub payload: [i32; 4],
    pub rule_id: u16,
}

#[derive(Clone, Copy)]
struct InputSlot {
    occupied: bool,
    input: CausalInput,
}

#[derive(Clone, Copy)]
struct Pending {
    occupied: bool,
    due: u64,
    rule_id: u16,
    cause_id: u32,
}

#[derive(Clone, Copy)]
struct Fired {
    rule_id: u16,
    cause_id: u32,
}

#[derive(Clone, Copy)]
struct LineSlot {
    occupied: bool,
    tick: u64,
    template: u16,
    len: u16,
    bytes: [u8; LINE_CAP],
}

#[derive(Clone, Copy)]
struct Snap {
    tick: u64,
    next_id: u32,
    event_len: u16,
    fired_len: u16,
    stock: i32,
    stance: u8,
    route_class: u8,
    quest_template: u16,
    quest_complete: u8,
    draw_count: u64,
    input_mask: u32,
    pending_mask: u64,
}

/// Causal ring. Construct once. `tick` does not allocate.
pub struct CausalWorld {
    seed: u64,
    tick: u64,
    next_id: u32,
    stock: i32,
    stance: u8,
    route_class: u8,
    quest_template: u16,
    quest_complete: u8,
    draw_count: u64,
    events: [CausalEvent; MAX_EVENTS],
    event_len: u16,
    inputs: [InputSlot; MAX_INPUTS],
    pending: [Pending; MAX_PENDING],
    fired: [Fired; MAX_EVENTS],
    fired_len: u16,
    rules: [CausalRule; MAX_RULES],
    rule_len: u16,
    lines: [LineSlot; MAX_LINES],
}

impl CausalWorld {
    pub fn new(seed: u64, stock: i32, rules: &[CausalRule]) -> Self {
        let mut world = Self {
            seed,
            tick: 0,
            next_id: 1,
            stock,
            stance: 1,
            route_class: 1,
            quest_template: 0,
            quest_complete: 0,
            draw_count: 0,
            events: [empty_event(); MAX_EVENTS],
            event_len: 0,
            inputs: [InputSlot {
                occupied: false,
                input: empty_input(),
            }; MAX_INPUTS],
            pending: [Pending {
                occupied: false,
                due: 0,
                rule_id: 0,
                cause_id: 0,
            }; MAX_PENDING],
            fired: [Fired {
                rule_id: 0,
                cause_id: 0,
            }; MAX_EVENTS],
            fired_len: 0,
            rules: [CausalRule::empty(); MAX_RULES],
            rule_len: 0,
            lines: [LineSlot {
                occupied: false,
                tick: 0,
                template: 0,
                len: 0,
                bytes: [0; LINE_CAP],
            }; MAX_LINES],
        };
        let n = rules.len().min(MAX_RULES);
        world.rules[..n].copy_from_slice(&rules[..n]);
        world.rule_len = n as u16;
        world
    }

    #[must_use]
    pub fn tick_index(&self) -> u64 {
        self.tick
    }

    #[must_use]
    pub fn stock(&self) -> i32 {
        self.stock
    }

    #[must_use]
    pub fn stance(&self) -> u8 {
        self.stance
    }

    #[must_use]
    pub fn route_class(&self) -> u8 {
        self.route_class
    }

    #[must_use]
    pub fn quest_template(&self) -> u16 {
        self.quest_template
    }

    #[must_use]
    pub fn quest_complete(&self) -> u8 {
        self.quest_complete
    }

    #[must_use]
    pub fn draw_count(&self) -> u64 {
        self.draw_count
    }

    #[must_use]
    pub fn events(&self) -> &[CausalEvent] {
        &self.events[..self.event_len as usize]
    }

    /// A cause id must name an event that is already stored and strictly older.
    #[must_use]
    pub fn cause_is_admissible(&self, cause_id: u32, child_tick: u64, child_seq: u32) -> bool {
        let Some(cause) = self.event_by_id(cause_id) else {
            return false;
        };
        (cause.tick, cause.seq, cause.id) < (child_tick, child_seq, u32::MAX)
    }

    pub fn push_input(&mut self, input: CausalInput) -> Result<(), CausalError> {
        for slot in &mut self.inputs {
            if !slot.occupied {
                slot.occupied = true;
                slot.input = input;
                return Ok(());
            }
        }
        Err(CausalError::InputFull)
    }

    pub fn record_line(&mut self, tick: u64, template: u16, line: &str) -> Result<(), CausalError> {
        let bytes = line.as_bytes();
        if bytes.len() > LINE_CAP {
            return Err(CausalError::LineTooLong);
        }
        for slot in &mut self.lines {
            if slot.occupied && slot.tick == tick && slot.template == template {
                slot.len = bytes.len() as u16;
                slot.bytes[..bytes.len()].copy_from_slice(bytes);
                return Ok(());
            }
        }
        for slot in &mut self.lines {
            if !slot.occupied {
                slot.occupied = true;
                slot.tick = tick;
                slot.template = template;
                slot.len = bytes.len() as u16;
                slot.bytes[..bytes.len()].copy_from_slice(bytes);
                return Ok(());
            }
        }
        Err(CausalError::LinesFull)
    }

    pub fn tick(&mut self) -> Result<(), CausalError> {
        let snap = self.snap();
        if let Err(err) = self.tick_inner() {
            self.restore(snap);
            return Err(err);
        }
        Ok(())
    }

    /// Little-endian state. Allocates. Not the frame path.
    #[must_use]
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(4096);
        push_u64(&mut out, self.seed);
        push_u64(&mut out, self.tick);
        push_u64(&mut out, self.next_id as u64);
        push_i32(&mut out, self.stock);
        out.push(self.stance);
        out.push(self.route_class);
        push_u64(&mut out, self.quest_template as u64);
        out.push(self.quest_complete);
        push_u64(&mut out, self.draw_count);
        push_u64(&mut out, self.event_len as u64);
        for event in self.events() {
            push_event(&mut out, event);
            if event.kind == KIND_QUEST_OFFER || event.kind == KIND_QUEST_COMPLETE {
                self.push_line(&mut out, event.kind, event.payload[0] as u16, event.tick);
            }
        }
        let mut pending_ids = [0u16; MAX_PENDING];
        let mut pending_n = 0usize;
        for (i, slot) in self.pending.iter().enumerate() {
            if slot.occupied {
                pending_ids[pending_n] = i as u16;
                pending_n += 1;
            }
        }
        sort_pending(&self.pending, &mut pending_ids[..pending_n]);
        push_u64(&mut out, pending_n as u64);
        for index in pending_ids.iter().take(pending_n) {
            let slot = self.pending[*index as usize];
            push_u64(&mut out, slot.due);
            push_u64(&mut out, slot.rule_id as u64);
            push_u64(&mut out, slot.cause_id as u64);
        }
        out
    }

    #[must_use]
    pub fn replay_hash(&self) -> String {
        hex_encode(&Sha256::digest(self.canonical_bytes()))
    }

    fn tick_inner(&mut self) -> Result<(), CausalError> {
        let rng = SimRng::new(self.seed, self.tick, SIM_SYSTEM_CAUSAL);
        debug_assert_eq!(rng.tick(), self.tick);
        let _ = (rng.world_seed(), rng.system_id());

        let start = self.event_len;
        self.admit_inputs()?;
        self.apply_due()?;
        self.cascade(start)?;
        self.tick = self.tick.saturating_add(1);
        Ok(())
    }

    fn admit_inputs(&mut self) -> Result<(), CausalError> {
        let mut idxs = [0u16; MAX_INPUTS];
        let mut n = 0usize;
        for (i, slot) in self.inputs.iter().enumerate() {
            if slot.occupied && slot.input.tick == self.tick {
                idxs[n] = i as u16;
                n += 1;
            }
        }
        sort_inputs(&self.inputs, &mut idxs[..n]);
        for index in idxs.iter().take(n) {
            let slot_i = *index as usize;
            let input = self.inputs[slot_i].input;
            self.inputs[slot_i].occupied = false;
            self.push_event(CausalEvent {
                id: self.next_id,
                tick: self.tick,
                seq: input.seq,
                kind: input.kind,
                class: input.class,
                subject: input.subject,
                cause_count: 0,
                cause_ids: [0; MAX_CAUSES],
                region: input.region,
                payload: input.payload,
                rule_id: 0,
            })?;
        }
        Ok(())
    }

    fn apply_due(&mut self) -> Result<(), CausalError> {
        let mut idxs = [0u16; MAX_PENDING];
        let mut n = 0usize;
        for (i, slot) in self.pending.iter().enumerate() {
            if slot.occupied && slot.due == self.tick {
                idxs[n] = i as u16;
                n += 1;
            }
        }
        sort_pending(&self.pending, &mut idxs[..n]);
        for index in idxs.iter().take(n) {
            let slot_i = *index as usize;
            let pending = self.pending[slot_i];
            self.pending[slot_i].occupied = false;
            self.emit_from_rule(pending.rule_id, pending.cause_id, true)?;
        }
        Ok(())
    }

    fn cascade(&mut self, mut cursor: u16) -> Result<(), CausalError> {
        for _ in 0..MAX_CASCADE_DEPTH {
            let end = self.event_len;
            if cursor == end {
                return Ok(());
            }
            for ei in cursor..end {
                self.match_event(ei)?;
            }
            cursor = end;
        }
        if cursor != self.event_len {
            return Err(CausalError::CascadeOverflow);
        }
        Ok(())
    }

    fn match_event(&mut self, index: u16) -> Result<(), CausalError> {
        let event = self.events[index as usize];
        let rule_len = self.rule_len as usize;
        for ri in 0..rule_len {
            let rule = self.rules[ri];
            if rule.id == 0 || self.already_fired(rule.id, event.id) || !rule_matches(&rule, &event)
            {
                continue;
            }
            if rule.requires_kind != 0 && self.find_kind(rule.requires_kind).is_none() {
                continue;
            }
            self.mark_fired(rule.id, event.id)?;
            if rule.delay_ticks == 0 && rule.every_ticks == 0 {
                self.emit_from_rule(rule.id, event.id, false)?;
            } else {
                let gap = if rule.delay_ticks == 0 {
                    rule.every_ticks
                } else {
                    rule.delay_ticks
                };
                self.schedule(event.tick.saturating_add(gap as u64), rule.id, event.id)?;
            }
        }
        Ok(())
    }

    fn emit_from_rule(
        &mut self,
        rule_id: u16,
        cause_id: u32,
        from_pending: bool,
    ) -> Result<(), CausalError> {
        let Some(rule) = self.rule_by_id(rule_id) else {
            return Ok(());
        };
        let Some(cause) = self.event_by_id(cause_id) else {
            return Ok(());
        };
        let cause_region = cause.region;
        let mut payload = rule.emit_payload;
        let mut reschedule = rule.every_ticks > 0;
        if rule.stock_delta != 0 {
            if self.stock <= 0 && rule.stock_delta < 0 {
                return Ok(());
            }
            let after = apply_stock(self.stock, rule.stock_delta);
            self.stock = after;
            payload = [rule.stock_delta, after, 0, 0];
            if after <= 0 {
                reschedule = false;
            }
        }
        let mut cause_ids = [0u32; MAX_CAUSES];
        let mut cause_count = 1u8;
        cause_ids[0] = cause_id;
        if rule.requires_kind != 0 {
            if let Some(extra) = self.find_kind(rule.requires_kind) {
                cause_ids[1] = extra;
                cause_count = 2;
                if cause_ids[0] > cause_ids[1] {
                    cause_ids.swap(0, 1);
                }
            }
        }
        let child_tick = self.tick;
        let child_seq = self.next_seq();
        if !self.cause_is_admissible(cause_id, child_tick, child_seq) {
            return Err(CausalError::CascadeOverflow);
        }
        self.push_event(CausalEvent {
            id: self.next_id,
            tick: child_tick,
            seq: child_seq,
            kind: rule.emit_kind,
            class: rule.emit_class,
            subject: rule.emit_subject,
            cause_count,
            cause_ids,
            region: cause_region,
            payload,
            rule_id,
        })?;
        if from_pending && reschedule {
            self.schedule(
                self.tick.saturating_add(rule.every_ticks as u64),
                rule_id,
                cause_id,
            )?;
        }
        Ok(())
    }

    fn push_event(&mut self, mut event: CausalEvent) -> Result<(), CausalError> {
        if self.event_len as usize >= MAX_EVENTS {
            return Err(CausalError::RingFull);
        }
        event.id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        self.note_event(&event);
        self.events[self.event_len as usize] = event;
        self.event_len += 1;
        Ok(())
    }

    fn note_event(&mut self, event: &CausalEvent) {
        if event.kind == KIND_ROUTE_CLASS {
            self.route_class = event.payload[0] as u8;
        }
        if event.kind == KIND_NPC_STANCE {
            self.stance = event.payload[0] as u8;
        }
        if event.kind == KIND_QUEST_OFFER {
            self.quest_template = event.payload[0] as u16;
        }
        if event.kind == KIND_QUEST_COMPLETE {
            self.quest_complete = 1;
            if self.quest_template == 0 {
                self.quest_template = event.payload[0] as u16;
            }
        }
    }

    fn schedule(&mut self, due: u64, rule_id: u16, cause_id: u32) -> Result<(), CausalError> {
        for slot in &mut self.pending {
            if !slot.occupied {
                *slot = Pending {
                    occupied: true,
                    due,
                    rule_id,
                    cause_id,
                };
                return Ok(());
            }
        }
        Err(CausalError::PendingFull)
    }

    fn mark_fired(&mut self, rule_id: u16, cause_id: u32) -> Result<(), CausalError> {
        if self.already_fired(rule_id, cause_id) {
            return Ok(());
        }
        if self.fired_len as usize >= MAX_EVENTS {
            return Err(CausalError::RingFull);
        }
        self.fired[self.fired_len as usize] = Fired { rule_id, cause_id };
        self.fired_len += 1;
        Ok(())
    }

    fn already_fired(&self, rule_id: u16, cause_id: u32) -> bool {
        self.fired[..self.fired_len as usize]
            .iter()
            .any(|fired| fired.rule_id == rule_id && fired.cause_id == cause_id)
    }

    fn next_seq(&self) -> u32 {
        let mut max = 0u32;
        let mut any = false;
        for event in self.events() {
            if event.tick == self.tick {
                any = true;
                max = max.max(event.seq);
            }
        }
        if any {
            max.saturating_add(1)
        } else {
            0
        }
    }

    fn event_by_id(&self, id: u32) -> Option<CausalEvent> {
        self.events().iter().copied().find(|event| event.id == id)
    }

    fn find_kind(&self, kind: u16) -> Option<u32> {
        self.events()
            .iter()
            .find(|event| event.kind == kind)
            .map(|event| event.id)
    }

    fn rule_by_id(&self, id: u16) -> Option<CausalRule> {
        self.rules[..self.rule_len as usize]
            .iter()
            .copied()
            .find(|rule| rule.id == id)
    }

    fn push_line(&self, out: &mut Vec<u8>, kind: u16, template: u16, tick: u64) {
        if let Some(recorded) = self.recorded(tick, template) {
            push_bytes(out, recorded);
            return;
        }
        let baked = baked_line(kind);
        let prefix = b"scripted:";
        push_u64(out, (prefix.len() + baked.len()) as u64);
        out.extend_from_slice(prefix);
        out.extend_from_slice(baked.as_bytes());
    }

    fn recorded(&self, tick: u64, template: u16) -> Option<&[u8]> {
        self.lines.iter().find_map(|slot| {
            if slot.occupied && slot.tick == tick && slot.template == template {
                Some(&slot.bytes[..slot.len as usize])
            } else {
                None
            }
        })
    }

    fn snap(&self) -> Snap {
        Snap {
            tick: self.tick,
            next_id: self.next_id,
            event_len: self.event_len,
            fired_len: self.fired_len,
            stock: self.stock,
            stance: self.stance,
            route_class: self.route_class,
            quest_template: self.quest_template,
            quest_complete: self.quest_complete,
            draw_count: self.draw_count,
            input_mask: mask32(self.inputs.iter().map(|slot| slot.occupied)),
            pending_mask: mask64(self.pending.iter().map(|slot| slot.occupied)),
        }
    }

    fn restore(&mut self, snap: Snap) {
        self.tick = snap.tick;
        self.next_id = snap.next_id;
        self.event_len = snap.event_len;
        self.fired_len = snap.fired_len;
        self.stock = snap.stock;
        self.stance = snap.stance;
        self.route_class = snap.route_class;
        self.quest_template = snap.quest_template;
        self.quest_complete = snap.quest_complete;
        self.draw_count = snap.draw_count;
        for (i, slot) in self.inputs.iter_mut().enumerate() {
            slot.occupied = snap.input_mask & (1u32 << i) != 0;
        }
        for (i, slot) in self.pending.iter_mut().enumerate() {
            slot.occupied = snap.pending_mask & (1u64 << i) != 0;
        }
    }
}

pub fn bridge_world() -> CausalWorld {
    CausalWorld::new(
        crate::CAUSAL_BRIDGE_SEED,
        crate::BRIDGE_STOCK,
        &crate::bridge_rules(crate::BRIDGE_PACE_TICKS, crate::BRIDGE_SACK_DELTA),
    )
}

/// Ticks 0 through 120 of the bridge case. Cut at 30, merchant at the ford at 120.
pub fn run_causal_bridge() -> Result<CausalWorld, CausalError> {
    let mut world = bridge_world();
    world.push_input(crate::bridge_cut_input())?;
    world.push_input(crate::merchant_ford_input())?;
    while world.tick_index() <= 120 {
        world.tick()?;
    }
    Ok(world)
}

fn apply_stock(stock: i32, delta: i32) -> i32 {
    if delta < 0 {
        stock.saturating_sub(delta.saturating_neg())
    } else {
        stock.saturating_add(delta)
    }
}

fn rule_matches(rule: &CausalRule, event: &CausalEvent) -> bool {
    if rule.when_kind != event.kind || rule.when_class != event.class {
        return false;
    }
    if !rule.any_subject && rule.when_subject != event.subject {
        return false;
    }
    if rule.check_payload {
        let index = rule.payload_index as usize;
        if index >= event.payload.len() || event.payload[index] != rule.payload_value {
            return false;
        }
    }
    true
}

fn baked_line(kind: u16) -> &'static str {
    if kind == KIND_QUEST_COMPLETE {
        COMPLETE_LINE
    } else {
        OFFER_LINE
    }
}

fn empty_event() -> CausalEvent {
    CausalEvent {
        id: 0,
        tick: 0,
        seq: 0,
        kind: 0,
        class: 0,
        subject: 0,
        cause_count: 0,
        cause_ids: [0; MAX_CAUSES],
        region: [0; 6],
        payload: [0; 4],
        rule_id: 0,
    }
}

fn empty_input() -> CausalInput {
    CausalInput {
        tick: 0,
        seq: 0,
        kind: 0,
        class: 0,
        subject: 0,
        region: [0; 6],
        payload: [0; 4],
    }
}

fn sort_inputs(inputs: &[InputSlot], idxs: &mut [u16]) {
    for i in 1..idxs.len() {
        let mut j = i;
        while j > 0 {
            let a = inputs[idxs[j] as usize].input;
            let b = inputs[idxs[j - 1] as usize].input;
            let ka = (a.tick, a.seq, a.subject, a.kind);
            let kb = (b.tick, b.seq, b.subject, b.kind);
            if ka >= kb {
                break;
            }
            idxs.swap(j, j - 1);
            j -= 1;
        }
    }
}

fn sort_pending(pending: &[Pending], idxs: &mut [u16]) {
    for i in 1..idxs.len() {
        let mut j = i;
        while j > 0 {
            let a = pending[idxs[j] as usize];
            let b = pending[idxs[j - 1] as usize];
            if (a.rule_id, a.cause_id) >= (b.rule_id, b.cause_id) {
                break;
            }
            idxs.swap(j, j - 1);
            j -= 1;
        }
    }
}

fn mask32(flags: impl Iterator<Item = bool>) -> u32 {
    let mut mask = 0u32;
    for (i, flag) in flags.enumerate() {
        if flag {
            mask |= 1u32 << i;
        }
    }
    mask
}

fn mask64(flags: impl Iterator<Item = bool>) -> u64 {
    let mut mask = 0u64;
    for (i, flag) in flags.enumerate() {
        if flag {
            mask |= 1u64 << i;
        }
    }
    mask
}

fn push_event(out: &mut Vec<u8>, event: &CausalEvent) {
    push_u64(out, event.id as u64);
    push_u64(out, event.tick);
    push_u64(out, event.seq as u64);
    push_u64(out, event.kind as u64);
    push_u64(out, event.class as u64);
    push_u64(out, event.subject);
    out.push(event.cause_count);
    for id in event.cause_ids {
        push_u64(out, id as u64);
    }
    for value in event.region {
        push_i32(out, value);
    }
    for value in event.payload {
        push_i32(out, value);
    }
    push_u64(out, event.rule_id as u64);
}

fn push_bytes(out: &mut Vec<u8>, bytes: &[u8]) {
    push_u64(out, bytes.len() as u64);
    out.extend_from_slice(bytes);
}

fn push_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_i32(out: &mut Vec<u8>, value: i32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        bridge_cut_input, bridge_rules, bridge_world, merchant_ford_input, CausalRule,
        BRIDGE_STOCK, CLASS_STRUCTURE, KIND_GEOMETRY_CUT, KIND_QUEST_COMPLETE, KIND_QUEST_OFFER,
        KIND_SUPPLY_DELTA,
    };

    fn run_bridge() -> CausalWorld {
        run_causal_bridge().expect("bridge")
    }

    #[test]
    fn bridge_stock_stance_and_completion() {
        let world = run_bridge();
        assert_eq!(world.tick_index(), 121);
        assert_eq!(world.route_class(), 2);
        assert_eq!(world.stock(), 0);
        assert_eq!(world.stance(), 2);
        assert_eq!(world.quest_template(), 1);
        assert_eq!(world.quest_complete(), 1);
        assert_eq!(world.draw_count(), 0);
        let offer = world
            .events()
            .iter()
            .find(|event| event.kind == KIND_QUEST_OFFER)
            .expect("offer");
        let done = world
            .events()
            .iter()
            .find(|event| event.kind == KIND_QUEST_COMPLETE)
            .expect("complete");
        assert_eq!(done.tick, 120);
        assert!(done.cause_ids[..done.cause_count as usize].contains(&offer.id));
        let deltas: Vec<_> = world
            .events()
            .iter()
            .filter(|event| event.kind == KIND_SUPPLY_DELTA)
            .map(|event| event.tick)
            .collect();
        assert_eq!(deltas, vec![40, 50, 60, 70, 80, 90, 100, 110]);
    }

    #[test]
    fn pace_is_the_rule_field() {
        let rules = bridge_rules(5, -2);
        let mut world = CausalWorld::new(1, 4, &rules);
        world.push_input(bridge_cut_input()).unwrap();
        while world.tick_index() <= 40 {
            world.tick().unwrap();
        }
        assert_eq!(world.stock(), 0);
        let first = world
            .events()
            .iter()
            .find(|event| event.kind == KIND_SUPPLY_DELTA)
            .unwrap();
        assert_eq!(first.tick, 35);
        assert_eq!(first.payload[0], -2);
    }

    #[test]
    fn early_ford_cell_does_not_complete_the_quest() {
        let mut world = bridge_world();
        world.push_input(bridge_cut_input()).unwrap();
        let mut early = merchant_ford_input();
        early.tick = 30;
        world.push_input(early).unwrap();
        while world.tick_index() <= 120 {
            world.tick().unwrap();
        }
        assert_eq!(world.quest_template(), 1);
        assert_eq!(world.quest_complete(), 0);
    }

    #[test]
    fn input_order_does_not_change_the_hash() {
        let mut late_first = CausalWorld::new(3, BRIDGE_STOCK, &[]);
        let mut early_first = CausalWorld::new(3, BRIDGE_STOCK, &[]);
        let mut a = bridge_cut_input();
        a.tick = 0;
        a.subject = 2;
        let mut b = bridge_cut_input();
        b.tick = 0;
        b.subject = 1;
        late_first.push_input(a).unwrap();
        late_first.push_input(b).unwrap();
        early_first.push_input(b).unwrap();
        early_first.push_input(a).unwrap();
        late_first.tick().unwrap();
        early_first.tick().unwrap();
        assert_eq!(late_first.replay_hash(), early_first.replay_hash());
        assert_eq!(early_first.events()[0].subject, 1);
        assert_eq!(early_first.events()[1].subject, 2);
    }

    #[test]
    fn cascade_cap_rolls_the_tick_back() {
        let mut rule = CausalRule::empty();
        rule.id = 1;
        rule.when_kind = KIND_GEOMETRY_CUT;
        rule.when_class = CLASS_STRUCTURE;
        rule.any_subject = true;
        rule.emit_kind = KIND_GEOMETRY_CUT;
        rule.emit_class = CLASS_STRUCTURE;
        rule.emit_subject = 1;
        let mut world = CausalWorld::new(1, 1, &[rule]);
        world
            .push_input(CausalInput {
                tick: 0,
                seq: 0,
                kind: KIND_GEOMETRY_CUT,
                class: CLASS_STRUCTURE,
                subject: 1,
                region: [0; 6],
                payload: [0; 4],
            })
            .unwrap();
        let err = world.tick().unwrap_err();
        assert_eq!(err, CausalError::CascadeOverflow);
        assert_eq!(world.tick_index(), 0);
        assert!(world.events().is_empty());
    }

    #[test]
    fn ring_full_rejects_the_next_admit() {
        let mut world = CausalWorld::new(1, 1, &[]);
        for tick in 0..8u64 {
            for seq in 0..32u32 {
                world
                    .push_input(CausalInput {
                        tick,
                        seq,
                        kind: 1,
                        class: 1,
                        subject: u64::from(seq),
                        region: [0; 6],
                        payload: [0; 4],
                    })
                    .unwrap();
            }
            world.tick().unwrap();
        }
        world
            .push_input(CausalInput {
                tick: 8,
                seq: 0,
                kind: 1,
                class: 1,
                subject: 0,
                region: [0; 6],
                payload: [0; 4],
            })
            .unwrap();
        assert_eq!(world.tick().unwrap_err(), CausalError::RingFull);
        assert_eq!(world.tick_index(), 8);
        assert_eq!(world.events().len(), MAX_EVENTS);
    }

    #[test]
    fn pending_full_rolls_back() {
        let rules = [1u16, 2, 3].map(|id| {
            let mut rule = CausalRule::empty();
            rule.id = id;
            rule.when_kind = 1;
            rule.when_class = 1;
            rule.any_subject = true;
            rule.emit_kind = 2;
            rule.emit_class = 1;
            rule.delay_ticks = 5;
            rule
        });
        let mut world = CausalWorld::new(1, 1, &rules);
        for seq in 0..32u32 {
            world
                .push_input(CausalInput {
                    tick: 0,
                    seq,
                    kind: 1,
                    class: 1,
                    subject: u64::from(seq),
                    region: [0; 6],
                    payload: [0; 4],
                })
                .unwrap();
        }
        assert_eq!(world.tick().unwrap_err(), CausalError::PendingFull);
        assert_eq!(world.tick_index(), 0);
        assert!(world.events().is_empty());
    }

    #[test]
    fn unknown_cause_is_rejected() {
        let world = bridge_world();
        assert!(!world.cause_is_admissible(0, 1, 0));
        assert!(!world.cause_is_admissible(9, 1, 0));
    }

    #[test]
    fn same_run_hashes_match() {
        let a = run_bridge();
        let b = run_bridge();
        assert_eq!(a.canonical_bytes(), b.canonical_bytes());
    }
}
