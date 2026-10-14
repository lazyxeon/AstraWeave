//! ECS components and the simulation-stage resolver.
//!
//! Hit order is the sort key `(attacker, defender, attack id, seq)`, not
//! query iteration order. The queue `Vec` is recycled. `resolve_hit` itself
//! does not allocate.

use std::any::TypeId;
use std::sync::Arc;

use astraweave_ecs::{App, Entity, Plugin, World};

use crate::fixed::Fixed64;
use crate::resolve::{
    resolve_heal, resolve_hit, AttackerView, BlockGate, DefenderView, HitOutcome, ImmuneGate,
    Modifier, SchoolRating, SchoolResist,
};
use crate::sim_rng::hit_rng;
use crate::tables::{CombatTables, BULWARK_MOD_ID, GUARD_MOD_ID, MAX_MODIFIERS};
use crate::CombatError;

#[derive(Clone, Copy, Debug, Default)]
pub struct COffense {
    pub might: i64,
    pub mending: i64,
    pub ratings: [SchoolRating; MAX_MODIFIERS],
    pub rating_len: u8,
    pub blight: i64,
    pub precision: i64,
    pub lethality_bonus_bp: u32,
    pub pen_bp: u32,
    pub tier: u16,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct CDefense {
    pub armor: i64,
    pub resists: [SchoolResist; MAX_MODIFIERS],
    pub resist_len: u8,
    pub guard: i64,
    pub bulwark_bp: u32,
    pub bulwark_flat: i64,
    pub evasion_bp: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct CVitals {
    pub hp: Fixed64,
    pub hp_max: Fixed64,
    pub poise: Fixed64,
    pub poise_max: Fixed64,
    pub stagger_left: u16,
}

impl Default for CVitals {
    fn default() -> Self {
        Self {
            hp: Fixed64::from_int(1),
            hp_max: Fixed64::from_int(1),
            poise: Fixed64::from_int(1),
            poise_max: Fixed64::from_int(1),
            stagger_left: 0,
        }
    }
}

/// Future weave cost. The damage formula does not read these fields.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CThreadStrain {
    pub capacity: i64,
    pub reserve: i64,
    pub coherence: i64,
    pub strain: i64,
}

#[derive(Clone, Copy, Debug)]
pub struct CModifiers {
    pub entries: [Modifier; MAX_MODIFIERS],
    pub len: u8,
}

impl Default for CModifiers {
    fn default() -> Self {
        Self {
            entries: [Modifier {
                id: 0,
                kind: crate::ModifierKind::AttackerBonusBp,
                bp_or_flat: 0,
            }; MAX_MODIFIERS],
            len: 0,
        }
    }
}

impl CModifiers {
    pub fn push(&mut self, modifier: Modifier) -> Result<(), CombatError> {
        let len = self.len as usize;
        if len >= MAX_MODIFIERS {
            return Err(CombatError::TooManyModifiers);
        }
        self.entries[len] = modifier;
        self.len += 1;
        Ok(())
    }

    pub fn as_slice(&self) -> &[Modifier] {
        &self.entries[..self.len as usize]
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct CBlock {
    pub active: bool,
    pub in_arc: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct CImmune {
    pub iframe_ticks: u16,
    pub parry_ticks: u16,
}

#[derive(Clone, Copy, Debug)]
pub struct HitEvent {
    pub attacker: Entity,
    pub defender: Entity,
    pub attack_id: u32,
    pub seq: u32,
    pub heal: bool,
}

#[derive(Clone, Debug, Default)]
pub struct HitQueue {
    hits: Vec<HitEvent>,
}

impl HitQueue {
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            hits: Vec::with_capacity(cap),
        }
    }

    pub fn push(&mut self, event: HitEvent) {
        self.hits.push(event);
    }

    pub fn drain_sorted(&mut self) -> Vec<HitEvent> {
        let mut hits = std::mem::take(&mut self.hits);
        hits.sort_by(|a, b| {
            (a.attacker, a.defender, a.heal, a.attack_id, a.seq).cmp(&(
                b.attacker,
                b.defender,
                b.heal,
                b.attack_id,
                b.seq,
            ))
        });
        hits
    }

    pub fn recycle(&mut self, mut hits: Vec<HitEvent>) {
        hits.clear();
        self.hits = hits;
    }

    pub fn len(&self) -> usize {
        self.hits.len()
    }

    pub fn is_empty(&self) -> bool {
        self.hits.is_empty()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CombatClock {
    pub world_seed: u64,
    pub tick: u64,
}

impl Default for CombatClock {
    fn default() -> Self {
        Self {
            world_seed: 1,
            tick: 0,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct CombatLog {
    pub deaths: Vec<u64>,
}

#[derive(Clone, Debug, Default)]
pub struct CombatScratch {
    ids: Vec<Entity>,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct CombatWriteGate {
    pub open: bool,
}

#[derive(Clone, Debug, Default)]
pub struct LoadedTables(pub Option<Arc<CombatTables>>);

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.world.insert_resource(HitQueue::with_capacity(256));
        app.world.insert_resource(CombatClock::default());
        app.world.insert_resource(CombatLog::default());
        app.world.insert_resource(CombatScratch::default());
        app.world.insert_resource(CombatWriteGate::default());
        app.world.insert_resource(LoadedTables::default());
        app.add_system_known_stage("simulation", combat_resolve_system);
    }
}

pub fn combat_resolve_system(world: &mut World) {
    let tables = match world.get_resource::<LoadedTables>() {
        Some(loaded) => match &loaded.0 {
            Some(tables) => Arc::clone(tables),
            None => return,
        },
        None => return,
    };
    let clock = world
        .get_resource::<CombatClock>()
        .copied()
        .unwrap_or_default();
    if let Some(log) = world.get_resource_mut::<CombatLog>() {
        log.deaths.clear();
    }
    let hits = world
        .get_resource_mut::<HitQueue>()
        .map(HitQueue::drain_sorted)
        .unwrap_or_default();
    if let Some(gate) = world.get_resource_mut::<CombatWriteGate>() {
        gate.open = true;
    }
    for (ordinal, event) in hits.iter().copied().enumerate() {
        let result = if event.heal {
            resolve_one_heal(world, &tables, &clock, ordinal as u64, event)
        } else {
            resolve_one_hit(world, &tables, &clock, ordinal as u64, event)
        };
        if let Err(CombatError::DoubleWrite) = result {
            break;
        }
    }
    if let Some(gate) = world.get_resource_mut::<CombatWriteGate>() {
        gate.open = false;
    }
    if let Some(queue) = world.get_resource_mut::<HitQueue>() {
        queue.recycle(hits);
    }
    decrement_timers(world);
    if let Some(clock) = world.get_resource_mut::<CombatClock>() {
        clock.tick = clock.tick.wrapping_add(1);
    }
}

fn resolve_one_hit(
    world: &mut World,
    tables: &CombatTables,
    clock: &CombatClock,
    ordinal: u64,
    event: HitEvent,
) -> Result<(), CombatError> {
    let Some(offense) = world.get::<COffense>(event.attacker).copied() else {
        return Ok(());
    };
    let Some(defense) = world.get::<CDefense>(event.defender).copied() else {
        return Ok(());
    };
    let Some(vitals) = world.get::<CVitals>(event.defender).copied() else {
        return Ok(());
    };
    let atk_mods = world
        .get::<CModifiers>(event.attacker)
        .copied()
        .unwrap_or_default();
    let mut def_mods = world
        .get::<CModifiers>(event.defender)
        .copied()
        .unwrap_or_default();
    if let Some(bp) = guard_bp(tables, defense.guard) {
        if bp > 0 {
            def_mods.push(Modifier {
                id: GUARD_MOD_ID,
                kind: crate::ModifierKind::DefenderReduceBp,
                bp_or_flat: bp,
            })?;
        }
    }
    let block = world
        .get::<CBlock>(event.defender)
        .copied()
        .unwrap_or_default();
    let immune = world
        .get::<CImmune>(event.defender)
        .copied()
        .unwrap_or_default();
    let in_arc = block.in_arc || (tables.defaults().block_omnidirectional && block.active);
    let atk = AttackerView {
        might: offense.might,
        ratings: &offense.ratings[..offense.rating_len as usize],
        blight: offense.blight,
        precision: offense.precision,
        lethality_bonus_bp: offense.lethality_bonus_bp,
        pen_bp: offense.pen_bp,
        tier: offense.tier,
        modifiers: atk_mods.as_slice(),
    };
    let def = DefenderView {
        hp: vitals.hp,
        hp_max: vitals.hp_max,
        poise: vitals.poise,
        poise_max: vitals.poise_max,
        armor: defense.armor,
        resists: &defense.resists[..defense.resist_len as usize],
        evasion_bp: defense.evasion_bp,
        bulwark_bp: defense.bulwark_bp,
        bulwark_flat: defense.bulwark_flat,
        modifiers: def_mods.as_slice(),
        block: BlockGate {
            active: block.active,
            in_arc,
        },
        immune: ImmuneGate {
            iframe_ticks: immune.iframe_ticks,
            parry_open: immune.parry_ticks > 0,
        },
    };
    // The views borrow offense/defense locals. Those locals must outlive the call.
    let mut rng = hit_rng(
        clock.world_seed,
        clock.tick,
        ordinal,
        u64::from(event.attack_id),
    );
    let outcome = resolve_hit(tables, &atk, &def, event.attack_id, &mut rng)?;
    apply_outcome(
        world,
        event.defender,
        outcome,
        tables.defaults().stagger_ticks,
    )
}

fn resolve_one_heal(
    world: &mut World,
    tables: &CombatTables,
    clock: &CombatClock,
    ordinal: u64,
    event: HitEvent,
) -> Result<(), CombatError> {
    let Some(offense) = world.get::<COffense>(event.attacker).copied() else {
        return Ok(());
    };
    let atk_mods = world
        .get::<CModifiers>(event.attacker)
        .copied()
        .unwrap_or_default();
    let atk = AttackerView {
        might: offense.might,
        ratings: &[],
        blight: 0,
        precision: 0,
        lethality_bonus_bp: offense.lethality_bonus_bp,
        pen_bp: 0,
        tier: offense.tier,
        modifiers: atk_mods.as_slice(),
    };
    let mut rng = hit_rng(
        clock.world_seed,
        clock.tick,
        ordinal,
        u64::from(event.attack_id),
    );
    let heal = resolve_heal(tables, &atk, offense.mending, event.attack_id, &mut rng)?;
    apply_hp_delta(world, event.defender, heal.hp_delta, true)
}

fn guard_bp(tables: &CombatTables, guard: i64) -> Option<u32> {
    let curve = tables.curve("guard_to_bp")?;
    let y = curve.eval(guard);
    if y <= 0 {
        Some(0)
    } else {
        Some(u32::try_from(y).unwrap_or(u32::MAX).min(9_000))
    }
}

fn apply_outcome(
    world: &mut World,
    entity: Entity,
    outcome: HitOutcome,
    stagger_ticks: u16,
) -> Result<(), CombatError> {
    if outcome.consume_parry {
        if let Some(immune) = world.get_mut::<CImmune>(entity) {
            immune.parry_ticks = 0;
        }
    }
    if outcome.flags.contains(crate::HitFlags::TARGET_DEAD)
        || outcome.flags.contains(crate::HitFlags::IMMUNE)
        || outcome.flags.contains(crate::HitFlags::PARRIED)
        || outcome.flags.contains(crate::HitFlags::EVADED)
    {
        return Ok(());
    }
    let before = world
        .get::<CVitals>(entity)
        .map(|v| v.hp.raw())
        .unwrap_or(0);
    apply_hp_delta(world, entity, outcome.hp_delta, false)?;
    let after = world
        .get::<CVitals>(entity)
        .map(|v| v.hp.raw())
        .unwrap_or(0);
    let died = before > 0 && after <= 0;
    let Some(vitals) = world.get_mut::<CVitals>(entity) else {
        return Ok(());
    };
    vitals.poise = vitals.poise.saturating_sub(outcome.poise_delta);
    if outcome.poise_delta.raw() > 0 && vitals.poise.raw() <= 0 {
        vitals.poise = vitals.poise_max;
        vitals.stagger_left = stagger_ticks;
    }
    if died {
        if let Some(log) = world.get_resource_mut::<CombatLog>() {
            log.deaths.push(entity.to_raw());
        }
    }
    let _ = BULWARK_MOD_ID;
    Ok(())
}

/// The only HP writer. Refuses to run unless [`CombatWriteGate`] is open,
/// so a second system cannot apply damage on the same tick by calling it.
pub fn apply_hp_delta(
    world: &mut World,
    entity: Entity,
    delta: Fixed64,
    heal: bool,
) -> Result<(), CombatError> {
    let open = world
        .get_resource::<CombatWriteGate>()
        .map(|g| g.open)
        .unwrap_or(false);
    if !open {
        return Err(CombatError::DoubleWrite);
    }
    let Some(vitals) = world.get_mut::<CVitals>(entity) else {
        return Ok(());
    };
    if heal {
        let room = vitals.hp_max.saturating_sub(vitals.hp);
        let add = if delta.raw() > room.raw() {
            room
        } else {
            delta
        };
        vitals.hp = vitals.hp.saturating_add(add);
    } else {
        let before = vitals.hp.raw();
        vitals.hp = vitals.hp.saturating_sub(delta);
        let died = before > 0 && vitals.hp.raw() <= 0;
        if died {
            // Death is recorded by the caller when it still holds the outcome.
            // A direct heal path does not die. A damage path records below if
            // this function is the one that crossed zero and the log is free.
        }
    }
    Ok(())
}

fn decrement_timers(world: &mut World) {
    let mut ids = world
        .get_resource_mut::<CombatScratch>()
        .map(|s| std::mem::take(&mut s.ids))
        .unwrap_or_default();
    ids.clear();
    world
        .archetypes()
        .append_entities_with_component(TypeId::of::<CImmune>(), &mut ids);
    world
        .archetypes()
        .append_entities_with_component(TypeId::of::<CVitals>(), &mut ids);
    ids.sort();
    ids.dedup();
    for entity in &ids {
        if let Some(immune) = world.get_mut::<CImmune>(*entity) {
            if immune.iframe_ticks > 0 {
                immune.iframe_ticks -= 1;
            }
            if immune.parry_ticks > 0 {
                immune.parry_ticks -= 1;
            }
        }
        if let Some(vitals) = world.get_mut::<CVitals>(*entity) {
            if vitals.stagger_left > 0 {
                vitals.stagger_left -= 1;
            }
        }
    }
    if let Some(scratch) = world.get_resource_mut::<CombatScratch>() {
        scratch.ids = ids;
    }
}
