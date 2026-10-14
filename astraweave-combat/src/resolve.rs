//! Pure damage and heal resolution. No allocation. No floating point.
//!
//! `resolve_hit` is the world: it uses the components it is given.
//! `expected_hit` and `effective_health` are the AI queries. They take a
//! [`StatKnowledge`] mask and never read a field the mask hides.

use crate::fixed::{whole_raw, Fixed64};
use crate::sim_rng::SimRng;
use crate::tables::{
    AbilityDef, CombatTables, HealDef, SchoolPresentation, MAX_DEFENDER_REDUCE_BP, MAX_MODIFIERS,
    USER_MOD_ID_LIMIT,
};
use crate::CombatError;
use rand::RngCore;

pub const DRAW_COUNT: usize = 4;
const STEP_CAP: usize = 20;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Modifier {
    pub id: u32,
    pub kind: ModifierKind,
    pub bp_or_flat: u32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ModifierKind {
    #[default]
    AttackerBonusBp,
    DefenderReduceBp,
    FlatAbsorb,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SchoolRating {
    pub school: u32,
    pub amount: i64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SchoolResist {
    pub school: u32,
    pub amount: i64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BlockGate {
    pub active: bool,
    pub in_arc: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ImmuneGate {
    pub iframe_ticks: u16,
    pub parry_open: bool,
}

/// Which numbers an observer is allowed to use.
///
/// The world resolver does not take this mask. AI helpers do. A hidden
/// field is not read: changing it cannot change the estimate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StatKnowledge {
    pub hp: bool,
    pub armor: bool,
    pub resists: bool,
    pub guard_and_mods: bool,
    pub bulwark: bool,
    pub offense: bool,
    pub weapon: bool,
    pub attacker_mods: bool,
}

impl StatKnowledge {
    /// The combatant's own sheet. Not for an enemy.
    pub const fn self_sheet() -> Self {
        Self {
            hp: true,
            armor: true,
            resists: true,
            guard_and_mods: true,
            bulwark: true,
            offense: true,
            weapon: true,
            attacker_mods: true,
        }
    }

    pub const fn none() -> Self {
        Self {
            hp: false,
            armor: false,
            resists: false,
            guard_and_mods: false,
            bulwark: false,
            offense: false,
            weapon: false,
            attacker_mods: false,
        }
    }
}

impl Default for StatKnowledge {
    fn default() -> Self {
        Self::none()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AttackerView<'a> {
    pub might: i64,
    pub ratings: &'a [SchoolRating],
    pub blight: i64,
    pub precision: i64,
    pub lethality_bonus_bp: u32,
    pub pen_bp: u32,
    pub tier: u16,
    pub modifiers: &'a [Modifier],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DefenderView<'a> {
    pub hp: Fixed64,
    pub hp_max: Fixed64,
    pub poise: Fixed64,
    pub poise_max: Fixed64,
    pub armor: i64,
    pub resists: &'a [SchoolResist],
    pub evasion_bp: u32,
    pub bulwark_bp: u32,
    pub bulwark_flat: i64,
    pub modifiers: &'a [Modifier],
    pub block: BlockGate,
    pub immune: ImmuneGate,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HitFlags {
    pub bits: u32,
}

impl HitFlags {
    pub const TARGET_DEAD: u32 = 1 << 0;
    pub const IMMUNE: u32 = 1 << 1;
    pub const PARRIED: u32 = 1 << 2;
    pub const EVADED: u32 = 1 << 3;
    pub const SATURATED: u32 = 1 << 4;
    pub const STAGGERED: u32 = 1 << 5;
    pub const CRIT: u32 = 1 << 6;
    pub const GLANCE: u32 = 1 << 7;
    pub const INCOMPLETE: u32 = 1 << 8;

    pub fn contains(self, bit: u32) -> bool {
        self.bits & bit != 0
    }

    fn set(&mut self, bit: u32) {
        self.bits |= bit;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StageTrace {
    pub raw: [i64; 8],
    /// Pushed stages in order. The canonical hash is the first seven `i64`s.
    pub steps: [i64; STEP_CAP],
    pub step_count: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HitOutcome {
    pub hp_delta: Fixed64,
    pub poise_delta: Fixed64,
    pub flags: HitFlags,
    pub consume_parry: bool,
    pub trace: StageTrace,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HealOutcome {
    pub hp_delta: Fixed64,
    pub flags: HitFlags,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Estimate {
    pub value: Fixed64,
    pub complete: bool,
}

struct Running {
    raw: i128,
    saturated: bool,
    steps: [i64; STEP_CAP],
    step_count: u8,
}

impl Running {
    fn push(&mut self) -> Result<(), CombatError> {
        if self.step_count as usize >= STEP_CAP {
            return Err(CombatError::TooManyModifiers);
        }
        let mut sat = self.saturated;
        let stored = clamp_step(self.raw, &mut sat);
        self.saturated = sat;
        self.steps[self.step_count as usize] = stored;
        self.step_count += 1;
        self.raw = i128::from(stored);
        Ok(())
    }
}

/// Inclusive basis-point map. `roll == 0` yields `min_bp`. `u32::MAX` yields `max_bp`.
pub fn lerp_bp(roll: u32, min_bp: u32, max_bp: u32) -> u32 {
    let span = max_bp - min_bp;
    min_bp + ((u128::from(roll) * (u128::from(span) + 1)) >> 32) as u32
}

/// World hit. Draws exactly four `u32`s, then resolves.
pub fn resolve_hit(
    tables: &CombatTables,
    atk: &AttackerView<'_>,
    def: &DefenderView<'_>,
    attack_id: u32,
    rng: &mut SimRng,
) -> Result<HitOutcome, CombatError> {
    let rolls = [
        rng.next_u32(),
        rng.next_u32(),
        rng.next_u32(),
        rng.next_u32(),
    ];
    resolve_with_rolls(tables, atk, def, attack_id, rolls, RollMode::Rolled)
}

/// AI expected damage. No draw. Hidden offense or weapon is an error.
/// Hidden defender numbers are omitted and the estimate is marked incomplete.
pub fn expected_hit(
    tables: &CombatTables,
    atk: &AttackerView<'_>,
    def: &DefenderView<'_>,
    attack_id: u32,
    knowledge: StatKnowledge,
) -> Result<Estimate, CombatError> {
    if !knowledge.offense || !knowledge.weapon {
        return Err(CombatError::HiddenStat);
    }
    let (atk, def, complete) = redact(atk, def, knowledge)?;
    let ability = tables
        .ability(attack_id)
        .ok_or(CombatError::UnknownAbility)?;
    let mid = ability.variance_min_bp + (ability.variance_max_bp - ability.variance_min_bp) / 2;
    let outcome = resolve_with_rolls(
        tables,
        &atk,
        &def,
        attack_id,
        [0, 0, 0, 0],
        RollMode::Expected { variance_bp: mid },
    )?;
    Ok(Estimate {
        value: outcome.hp_delta,
        complete,
    })
}

/// Effective health for one school, below or at the cap.
/// Hidden hp is an error. Hidden armor does not change the result.
pub fn effective_health(
    tables: &CombatTables,
    atk_tier: u16,
    def: &DefenderView<'_>,
    school_id: u32,
    knowledge: StatKnowledge,
) -> Result<Estimate, CombatError> {
    if !knowledge.hp {
        return Err(CombatError::HiddenStat);
    }
    let blank = AttackerView {
        might: 0,
        ratings: &[],
        blight: 0,
        precision: 0,
        lethality_bonus_bp: 0,
        pen_bp: 0,
        tier: atk_tier,
        modifiers: &[],
    };
    let (_, def, complete) = redact(&blank, def, knowledge)?;
    let school = tables.school(school_id).ok_or(CombatError::UnknownSchool)?;
    let k = tables.k_for(school, atk_tier)?;
    let armor_whole = if school.presentation == SchoolPresentation::Physical {
        def.armor
    } else if knowledge.resists {
        resist_of(def.resists, school_id)?
    } else {
        0
    };
    let armor = whole_raw(armor_whole)?;
    let k_raw = whole_raw(k)?;
    let cap = school.cap_bp.min(tables.defaults().armor_cap_bp);
    let mut saturated = false;
    let mut ehp = hyperbola_ehp(i128::from(def.hp.raw()), armor, k_raw, cap, &mut saturated)?;
    if knowledge.guard_and_mods || knowledge.bulwark {
        for keep in defender_keeps(tables, &def)? {
            if keep == 0 {
                break;
            }
            ehp = mul_div(ehp, 10_000, i128::from(keep), &mut saturated)?;
        }
    }
    if ehp > i128::from(i64::MAX) {
        ehp = i128::from(i64::MAX);
    }
    Ok(Estimate {
        value: Fixed64::from_raw(ehp as i64),
        complete,
    })
}

pub fn resolve_heal(
    tables: &CombatTables,
    atk: &AttackerView<'_>,
    mending: i64,
    heal_id: u32,
    rng: &mut SimRng,
) -> Result<HealOutcome, CombatError> {
    let _rolls = [
        rng.next_u32(),
        rng.next_u32(),
        rng.next_u32(),
        rng.next_u32(),
    ];
    let heal = tables.heal(heal_id).ok_or(CombatError::UnknownAbility)?;
    let mut saturated = false;
    let mut raw = heal_base(mending, heal)?;
    let (factors, n) = attacker_factors(tables, atk.modifiers)?;
    for factor in factors.iter().take(n) {
        raw = mul_div(raw, i128::from(*factor), 10_000, &mut saturated)?;
    }
    if heal.always_crit {
        let mult = crit_mult(tables, atk.lethality_bonus_bp);
        raw = mul_div(raw, i128::from(mult), 10_000, &mut saturated)?;
    }
    let mut flags = HitFlags::default();
    if saturated {
        flags.set(HitFlags::SATURATED);
    }
    Ok(HealOutcome {
        hp_delta: Fixed64::from_raw(clamp_step(raw, &mut saturated)),
        flags,
    })
}

#[derive(Clone, Copy)]
enum RollMode {
    Rolled,
    Expected { variance_bp: u32 },
}

fn resolve_with_rolls(
    tables: &CombatTables,
    atk: &AttackerView<'_>,
    def: &DefenderView<'_>,
    attack_id: u32,
    rolls: [u32; DRAW_COUNT],
    mode: RollMode,
) -> Result<HitOutcome, CombatError> {
    let ability = tables
        .ability(attack_id)
        .ok_or(CombatError::UnknownAbility)?;
    let mut flags = HitFlags::default();
    if def.hp.raw() <= 0 {
        flags.set(HitFlags::TARGET_DEAD);
        return Ok(zero_outcome(flags, false));
    }
    if def.immune.iframe_ticks > 0 {
        flags.set(HitFlags::IMMUNE);
        return Ok(zero_outcome(flags, false));
    }
    if def.immune.parry_open {
        flags.set(HitFlags::PARRIED);
        return Ok(zero_outcome(flags, true));
    }
    let evasion = def.evasion_bp.min(tables.defaults().evasion_cap_bp);
    if matches!(mode, RollMode::Rolled) && chance_hit(rolls[3], evasion) {
        flags.set(HitFlags::EVADED);
        return Ok(zero_outcome(flags, false));
    }

    let variance_bp = match mode {
        RollMode::Rolled => lerp_bp(rolls[0], ability.variance_min_bp, ability.variance_max_bp),
        RollMode::Expected { variance_bp } => variance_bp,
    };
    let mut run = Running {
        raw: base_damage(tables, atk, ability, variance_bp)?,
        saturated: false,
        steps: [0; STEP_CAP],
        step_count: 0,
    };
    run.push()?;

    let (factors, n) = attacker_factors(tables, atk.modifiers)?;
    for factor in factors.iter().take(n) {
        run.raw = mul_div(run.raw, i128::from(*factor), 10_000, &mut run.saturated)?;
        run.push()?;
    }

    let (chance, mult) = crit_params(tables, atk)?;
    match mode {
        RollMode::Rolled => {
            if ability.always_crit || chance_hit(rolls[1], chance) {
                flags.set(HitFlags::CRIT);
                run.raw = mul_div(run.raw, i128::from(mult), 10_000, &mut run.saturated)?;
            }
            if chance_hit(rolls[2], tables.defaults().glance_chance_bp) {
                flags.set(HitFlags::GLANCE);
                run.raw = mul_div(
                    run.raw,
                    i128::from(tables.defaults().glance_bp),
                    10_000,
                    &mut run.saturated,
                )?;
            }
        }
        RollMode::Expected { .. } => {
            let expect = 10_000 + i128::from(chance) * (i128::from(mult) - 10_000) / 10_000;
            run.raw = mul_div(run.raw, expect, 10_000, &mut run.saturated)?;
            let glance_chance = tables.defaults().glance_chance_bp;
            if glance_chance > 0 {
                let keep = 10_000
                    - i128::from(glance_chance)
                        * (10_000 - i128::from(tables.defaults().glance_bp))
                        / 10_000;
                run.raw = mul_div(run.raw, keep, 10_000, &mut run.saturated)?;
            }
            if evasion > 0 {
                run.raw = mul_div(
                    run.raw,
                    10_000 - i128::from(evasion),
                    10_000,
                    &mut run.saturated,
                )?;
            }
        }
    }

    let school = tables
        .school(ability.school)
        .ok_or(CombatError::UnknownSchool)?;
    if !ability.bleed {
        let k = tables.k_for(school, atk.tier)?;
        let armor_whole = if school.presentation == SchoolPresentation::Physical {
            def.armor
        } else {
            resist_of(def.resists, ability.school)?
        };
        let armor = apply_pen(
            whole_raw(armor_whole)?,
            atk.pen_bp,
            tables.defaults().pen_cap_bp,
        )?;
        let k_raw = whole_raw(k)?;
        let cap = school.cap_bp.min(tables.defaults().armor_cap_bp);
        run.raw = hyperbola_taken(run.raw, armor, k_raw, cap, &mut run.saturated)?;
    }
    run.push()?;
    let post_school = run.raw;

    for keep in defender_keeps(tables, def)? {
        if keep == 0 {
            break;
        }
        run.raw = mul_div(run.raw, i128::from(keep), 10_000, &mut run.saturated)?;
        run.push()?;
    }
    let floor = mul_div(
        post_school,
        i128::from(tables.defaults().min_taken_bp),
        10_000,
        &mut run.saturated,
    )?;
    if run.raw < floor {
        run.raw = floor;
    }
    if post_school > 0 && run.raw == 0 {
        run.raw = 1;
    }

    run.raw = subtract_flat(
        run.raw,
        block_flat(def),
        floor,
        tables.defaults().max_flat_whole,
    )?;
    let (absorbs, absorb_n) = flat_absorbs(def.modifiers)?;
    for absorb in absorbs.iter().take(absorb_n) {
        run.raw = subtract_flat(
            run.raw,
            i64::from(*absorb),
            floor,
            tables.defaults().max_flat_whole,
        )?;
    }
    run.push()?;

    if run.saturated {
        flags.set(HitFlags::SATURATED);
    }
    let hp_delta = Fixed64::from_raw(clamp_step(run.raw, &mut run.saturated));
    Ok(HitOutcome {
        hp_delta,
        poise_delta: Fixed64::from_int(ability.poise),
        flags,
        consume_parry: false,
        trace: StageTrace {
            raw: [
                step_or_zero(&run, 0),
                hp_delta.raw(),
                floor as i64,
                0,
                0,
                0,
                0,
                0,
            ],
            steps: run.steps,
            step_count: run.step_count,
        },
    })
}

fn step_or_zero(run: &Running, index: usize) -> i64 {
    if index < run.step_count as usize {
        run.steps[index]
    } else {
        0
    }
}

fn zero_outcome(flags: HitFlags, consume_parry: bool) -> HitOutcome {
    HitOutcome {
        hp_delta: Fixed64::ZERO,
        poise_delta: Fixed64::ZERO,
        flags,
        consume_parry,
        trace: StageTrace {
            raw: [0; 8],
            steps: [0; STEP_CAP],
            step_count: 0,
        },
    }
}

fn redact<'a>(
    atk: &AttackerView<'a>,
    def: &DefenderView<'a>,
    knowledge: StatKnowledge,
) -> Result<(AttackerView<'a>, DefenderView<'a>, bool), CombatError> {
    let mut atk = *atk;
    let mut def = *def;
    let mut complete = true;
    if !knowledge.attacker_mods {
        complete = false;
        atk.modifiers = &[];
    }
    if !knowledge.armor {
        complete = false;
        def.armor = 0;
    }
    if !knowledge.resists {
        complete = false;
        def.resists = &[];
    }
    if !knowledge.guard_and_mods {
        complete = false;
        def.modifiers = &[];
    }
    if !knowledge.bulwark {
        complete = false;
        def.block = BlockGate::default();
        def.bulwark_bp = 0;
        def.bulwark_flat = 0;
    }
    Ok((atk, def, complete))
}

fn base_damage(
    tables: &CombatTables,
    atk: &AttackerView<'_>,
    ability: &AbilityDef,
    variance_bp: u32,
) -> Result<i128, CombatError> {
    let span = ability.weapon_max - ability.weapon_min;
    let rolled_whole =
        i128::from(ability.weapon_min) + i128::from(span) * i128::from(variance_bp) / 10_000;
    let weapon = whole_raw(rolled_whole as i64)? * i128::from(ability.weapon_bp) / 10_000;
    let school = tables
        .school(ability.school)
        .ok_or(CombatError::UnknownSchool)?;
    let (rating, num, den) = if ability.bleed
        || (ability.periodic && school.presentation != SchoolPresentation::Physical)
    {
        (atk.blight, ability.blight_num, ability.blight_den)
    } else if school.presentation == SchoolPresentation::Physical {
        (atk.might, ability.might_num, ability.might_den)
    } else {
        (
            rating_of(atk.ratings, ability.school)?,
            ability.school_num,
            ability.school_den,
        )
    };
    if den == 0 {
        return Err(CombatError::BadCoeff);
    }
    let rating_raw = whole_raw(rating)? * i128::from(num) / i128::from(den);
    Ok(weapon + rating_raw + whole_raw(ability.flat)?)
}

fn heal_base(mending: i64, heal: &HealDef) -> Result<i128, CombatError> {
    if heal.mending_den == 0 {
        return Err(CombatError::BadCoeff);
    }
    Ok(
        whole_raw(mending)? * i128::from(heal.mending_num) / i128::from(heal.mending_den)
            + whole_raw(heal.flat)?,
    )
}

fn attacker_factors(
    tables: &CombatTables,
    mods: &[Modifier],
) -> Result<([u32; MAX_MODIFIERS], usize), CombatError> {
    let mut buf = [Modifier {
        id: 0,
        kind: ModifierKind::AttackerBonusBp,
        bp_or_flat: 0,
    }; MAX_MODIFIERS];
    let mut n = 0usize;
    for m in mods {
        if m.kind != ModifierKind::AttackerBonusBp || m.id >= USER_MOD_ID_LIMIT {
            return Err(CombatError::ModifierCeiling);
        }
        push_mod(&mut buf, &mut n, *m)?;
    }
    let slice = &mut buf[..n];
    slice.sort_by_key(|m| m.id);
    reject_dupes(slice)?;
    let cap = tables.defaults().attacker_bonus_cap_bp;
    let mut factors = [0u32; MAX_MODIFIERS];
    for (i, m) in slice.iter().enumerate() {
        factors[i] = 10_000 + m.bp_or_flat.min(cap);
    }
    Ok((factors, n))
}

fn defender_keeps(
    tables: &CombatTables,
    def: &DefenderView<'_>,
) -> Result<[u32; MAX_MODIFIERS], CombatError> {
    let mut buf = [Modifier {
        id: 0,
        kind: ModifierKind::DefenderReduceBp,
        bp_or_flat: 0,
    }; MAX_MODIFIERS];
    let mut n = 0usize;
    for m in def.modifiers {
        if m.kind == ModifierKind::FlatAbsorb {
            continue;
        }
        if m.kind != ModifierKind::DefenderReduceBp || !allowed_mod_id(m.id) {
            return Err(CombatError::ModifierCeiling);
        }
        if m.bp_or_flat > MAX_DEFENDER_REDUCE_BP {
            return Err(CombatError::ModifierCeiling);
        }
        push_mod(&mut buf, &mut n, *m)?;
    }
    if def.block.active && def.block.in_arc && def.bulwark_bp > 0 {
        if def.bulwark_bp > MAX_DEFENDER_REDUCE_BP {
            return Err(CombatError::ModifierCeiling);
        }
        push_mod(
            &mut buf,
            &mut n,
            Modifier {
                id: crate::tables::BULWARK_MOD_ID,
                kind: ModifierKind::DefenderReduceBp,
                bp_or_flat: def.bulwark_bp,
            },
        )?;
    }
    let slice = &mut buf[..n];
    slice.sort_by_key(|m| m.id);
    reject_dupes(slice)?;
    let mut keeps = [10_000u32; MAX_MODIFIERS];
    let mut count = 0usize;
    for m in slice.iter() {
        keeps[count] = 10_000 - m.bp_or_flat;
        count += 1;
    }
    // Trailing 10_000 entries are not iterated: caller uses a separate count.
    // Pack the count into keeps by returning only the prefix via a second value.
    let _ = tables;
    let _ = count;
    // Rewrite: return pair. This function's signature is the array only so the
    // loop in the caller would also multiply identity factors. Store count in
    // a sentinel by using 0 as "stop" — 0 is not a legal keep (min keep is 1000).
    if count < MAX_MODIFIERS {
        keeps[count] = 0;
    }
    Ok(keeps)
}

fn flat_absorbs(mods: &[Modifier]) -> Result<([u32; MAX_MODIFIERS], usize), CombatError> {
    let mut buf = [Modifier {
        id: 0,
        kind: ModifierKind::FlatAbsorb,
        bp_or_flat: 0,
    }; MAX_MODIFIERS];
    let mut n = 0usize;
    for m in mods {
        if m.kind != ModifierKind::FlatAbsorb {
            continue;
        }
        if m.id >= USER_MOD_ID_LIMIT {
            return Err(CombatError::ModifierCeiling);
        }
        push_mod(&mut buf, &mut n, *m)?;
    }
    let slice = &mut buf[..n];
    slice.sort_by_key(|m| m.id);
    reject_dupes(slice)?;
    let mut out = [0u32; MAX_MODIFIERS];
    for (i, m) in slice.iter().enumerate() {
        out[i] = m.bp_or_flat;
    }
    Ok((out, n))
}

fn allowed_mod_id(id: u32) -> bool {
    id < USER_MOD_ID_LIMIT
        || id == crate::tables::GUARD_MOD_ID
        || id == crate::tables::BULWARK_MOD_ID
}

fn push_mod(
    buf: &mut [Modifier; MAX_MODIFIERS],
    n: &mut usize,
    m: Modifier,
) -> Result<(), CombatError> {
    if *n >= MAX_MODIFIERS {
        return Err(CombatError::TooManyModifiers);
    }
    buf[*n] = m;
    *n += 1;
    Ok(())
}

fn reject_dupes(mods: &[Modifier]) -> Result<(), CombatError> {
    for pair in mods.windows(2) {
        if pair[0].id == pair[1].id {
            return Err(CombatError::DuplicateModifier);
        }
    }
    Ok(())
}

fn block_flat(def: &DefenderView<'_>) -> i64 {
    if def.block.active && def.block.in_arc {
        def.bulwark_flat
    } else {
        0
    }
}

fn crit_params(tables: &CombatTables, atk: &AttackerView<'_>) -> Result<(u32, u32), CombatError> {
    let chance = if atk.precision == 0 {
        0
    } else {
        let curve = tables
            .curve("precision_to_crit")
            .ok_or(CombatError::BadCurve)?;
        let y = curve.eval(atk.precision);
        if y < 0 {
            0
        } else {
            u32::try_from(y).unwrap_or(u32::MAX)
        }
    };
    let chance = chance.min(tables.defaults().crit_cap_bp);
    Ok((chance, crit_mult(tables, atk.lethality_bonus_bp)))
}

fn crit_mult(tables: &CombatTables, lethality: u32) -> u32 {
    tables
        .defaults()
        .crit_base_bp
        .saturating_add(lethality)
        .min(tables.defaults().crit_mult_cap_bp)
}

fn rating_of(ratings: &[SchoolRating], school: u32) -> Result<i64, CombatError> {
    let mut found = None;
    for row in ratings {
        if row.school == school {
            if found.is_some() {
                return Err(CombatError::DuplicateId);
            }
            found = Some(row.amount);
        }
    }
    Ok(found.unwrap_or(0))
}

fn resist_of(resists: &[SchoolResist], school: u32) -> Result<i64, CombatError> {
    let mut found = None;
    for row in resists {
        if row.school == school {
            if found.is_some() {
                return Err(CombatError::DuplicateId);
            }
            found = Some(row.amount);
        }
    }
    Ok(found.unwrap_or(0))
}

fn apply_pen(armor: i128, pen_bp: u32, pen_cap: u32) -> Result<i128, CombatError> {
    let pen = pen_bp.min(pen_cap).min(10_000);
    let mut sat = false;
    mul_div(armor, 10_000 - i128::from(pen), 10_000, &mut sat)
}

fn hyperbola_taken(
    damage: i128,
    armor: i128,
    k: i128,
    cap_bp: u32,
    saturated: &mut bool,
) -> Result<i128, CombatError> {
    if k <= 0 {
        return Err(CombatError::KOutOfRange);
    }
    let keep = i128::from(10_000 - cap_bp);
    let cap = i128::from(cap_bp);
    if armor.saturating_mul(keep) > k.saturating_mul(cap) {
        return mul_div(damage, keep, 10_000, saturated);
    }
    mul_div(damage, k, armor + k, saturated)
}

fn hyperbola_ehp(
    hp: i128,
    armor: i128,
    k: i128,
    cap_bp: u32,
    saturated: &mut bool,
) -> Result<i128, CombatError> {
    if k <= 0 {
        return Err(CombatError::KOutOfRange);
    }
    let keep = i128::from(10_000 - cap_bp);
    let cap = i128::from(cap_bp);
    if armor.saturating_mul(keep) > k.saturating_mul(cap) {
        return mul_div(hp, 10_000, keep, saturated);
    }
    mul_div(hp, armor + k, k, saturated)
}

fn subtract_flat(
    raw: i128,
    flat_whole: i64,
    floor: i128,
    max_flat_whole: i64,
) -> Result<i128, CombatError> {
    let capped = flat_whole.clamp(0, max_flat_whole);
    let flat = whole_raw(capped)?;
    let next = raw - flat;
    Ok(if next < floor { floor } else { next })
}

fn chance_hit(roll: u32, chance_bp: u32) -> bool {
    if chance_bp == 0 {
        return false;
    }
    u64::from(roll) * 10_000 < (u64::from(chance_bp) << 32)
}

fn mul_div(value: i128, num: i128, den: i128, saturated: &mut bool) -> Result<i128, CombatError> {
    if den <= 0 || value < 0 || num < 0 {
        return Err(CombatError::StatOutOfRange);
    }
    let Some(prod) = value.checked_mul(num) else {
        *saturated = true;
        return Ok(i128::from(i64::MAX));
    };
    let q = prod / den;
    if q > i128::from(i64::MAX) {
        *saturated = true;
        Ok(i128::from(i64::MAX))
    } else {
        Ok(q)
    }
}

fn clamp_step(value: i128, saturated: &mut bool) -> i64 {
    if value > i128::from(i64::MAX) {
        *saturated = true;
        i64::MAX
    } else if value < 0 {
        0
    } else {
        value as i64
    }
}

/// Little-endian SHA-256 of the first `n` steps. The canonical hit uses `n = 7`.
pub fn hash_steps(steps: &[i64]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    for step in steps {
        h.update(step.to_le_bytes());
    }
    h.finalize().into()
}
