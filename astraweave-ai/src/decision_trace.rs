//! Decision ring for one agent at one tick.
//!
//! Compiled only with `decision-trace`. Shipping builds omit this module.
//! [`DecisionTrace::new`] allocates the rings once. [`DecisionTrace::push`]
//! writes a slot and does not allocate.

#![forbid(unsafe_code)]

use astraweave_core::{ActionStep, EngineError, IVec2, WorldSnapshot};

use crate::htn::{self, plan_route, AgentPlan, SpatialHooks};
use crate::orchestrator::Orchestrator;

pub const ALT_CAP: usize = 4;
pub const RECORD_CAP: usize = 128;
pub const AGENT_CAP: usize = 32;
const WALK_CAP: usize = 8;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlannerId {
    None = 0,
    Rule = 1,
    BehaviorTree = 2,
    GoapHeuristic = 3,
    Utility = 4,
    Htn = 5,
}

#[repr(u16)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GoalId {
    None = 0,
    Cross = 1,
    Engage = 2,
    Hold = 3,
    Extract = 4,
}

#[repr(u16)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionCode {
    Empty = 0,
    MoveTo = 1,
    UseBreach = 2,
    TakeCover = 3,
    Wait = 4,
    ThrowSmoke = 5,
    CoverFire = 6,
}

#[repr(u16)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RejectCode {
    Selected = 0,
    LowerScore = 1,
    Cooldown = 2,
    SealedCell = 3,
    OutOfBand = 4,
    BudgetExhausted = 5,
    NoPath = 6,
    TieLostToLowerId = 7,
}

#[repr(u16)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FailCode {
    Ok = 0,
    InvalidAction = 1,
    Cooldown = 2,
    LosBlocked = 3,
    NoPath = 4,
    Resource = 5,
    Pending = 6,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AltScore {
    pub action: u16,
    pub reject: u16,
    pub score: i32,
    pub a: i32,
    pub b: i32,
}

impl AltScore {
    pub const EMPTY: Self = Self {
        action: 0,
        reject: 0,
        score: 0,
        a: 0,
        b: 0,
    };
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PerceivedView {
    pub self_x: i32,
    pub self_y: i32,
    pub ammo: i32,
    pub morale_milli: i32,
    pub smoke_cd_milli: i32,
    pub enemy_count: u8,
    pub nearest_x: i32,
    pub nearest_y: i32,
    pub nearest_hp: i32,
    pub obstacle_count: u16,
    pub breach_len: u8,
    pub breach_portal: u32,
    pub breach_x: i32,
    pub breach_y: i32,
    pub lost_cover_len: u8,
    pub lost_cover_id: u32,
}

impl PerceivedView {
    pub const EMPTY: Self = Self {
        self_x: 0,
        self_y: 0,
        ammo: 0,
        morale_milli: 0,
        smoke_cd_milli: 0,
        enemy_count: 0,
        nearest_x: 0,
        nearest_y: 0,
        nearest_hp: 0,
        obstacle_count: 0,
        breach_len: 0,
        breach_portal: 0,
        breach_x: 0,
        breach_y: 0,
        lost_cover_len: 0,
        lost_cover_id: 0,
    };
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BeliefView {
    pub fact_hash: u64,
    pub strength_milli: i32,
    pub source: u8,
    pub stat_mask: u16,
}

impl BeliefView {
    pub const EMPTY: Self = Self {
        fact_hash: 0,
        strength_milli: 0,
        source: 0,
        stat_mask: 0,
    };
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DecisionRecord {
    pub tick: u64,
    pub entity_id: u64,
    pub planner: u8,
    pub band: u8,
    pub goal: u16,
    pub selected: u16,
    pub selected_score: i32,
    pub param0: i32,
    pub param1: i32,
    pub outcome: u8,
    pub fail: u16,
    pub alt_len: u8,
    pub alts: [AltScore; ALT_CAP],
    pub perceived: PerceivedView,
    pub belief: BeliefView,
}

impl DecisionRecord {
    pub const EMPTY: Self = Self {
        tick: 0,
        entity_id: 0,
        planner: 0,
        band: 0,
        goal: 0,
        selected: 0,
        selected_score: 0,
        param0: 0,
        param1: 0,
        outcome: 0,
        fail: FailCode::Pending as u16,
        alt_len: 0,
        alts: [AltScore::EMPTY; ALT_CAP],
        perceived: PerceivedView::EMPTY,
        belief: BeliefView::EMPTY,
    };
}

#[derive(Clone, Copy, Debug)]
pub struct TickSample {
    pub record: DecisionRecord,
    /// Milli-unit position the spatial planner started from.
    pub pos_milli: [i32; 3],
    /// Milli-unit goal the spatial planner walked toward.
    pub goal_milli: [i32; 3],
}

impl TickSample {
    #[must_use]
    pub const fn from_record(record: DecisionRecord) -> Self {
        Self {
            record,
            pos_milli: [0; 3],
            goal_milli: [0; 3],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Patch {
    NearestEnemy { x: i32, y: i32 },
    SmokeCooldownMilli(i32),
    Ammo(i32),
    SealCell { x: i32, y: i32 },
    GoalMilli { x: i32, y: i32, z: i32 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DecisionDiff {
    pub tick: u64,
    pub entity_id: u64,
    pub before: u16,
    pub after: u16,
    pub before_score: i32,
    pub after_score: i32,
    pub alt_len: u8,
    pub alts: [AltScore; ALT_CAP],
}

pub struct PlannerView<'a> {
    pub tiles: &'a astraweave_nav::NavTiles,
    pub scratch: &'a mut astraweave_nav::NavScratch,
    pub hooks: SpatialHooks<'a>,
}

#[derive(Clone, Copy, Debug)]
pub struct AltList {
    items: [AltScore; WALK_CAP],
    len: u8,
}

impl AltList {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            items: [AltScore::EMPTY; WALK_CAP],
            len: 0,
        }
    }

    pub fn clear(&mut self) {
        self.len = 0;
    }

    pub fn push(&mut self, alt: AltScore) {
        if (self.len as usize) < WALK_CAP {
            self.items[self.len as usize] = alt;
            self.len += 1;
            return;
        }
        for i in 1..WALK_CAP {
            self.items[i - 1] = self.items[i];
        }
        self.items[WALK_CAP - 1] = alt;
    }

    /// Keep up to four rejects. The chosen `(action, a)` pair is left out.
    #[must_use]
    pub fn rejects(&self, winner_action: u16, winner_a: i32) -> ([AltScore; ALT_CAP], u8) {
        let mut out = [AltScore::EMPTY; ALT_CAP];
        let mut n = 0u8;
        for alt in self.items.iter().take(self.len as usize) {
            if alt.action == winner_action && alt.a == winner_a {
                continue;
            }
            if n == ALT_CAP as u8 {
                for i in 1..ALT_CAP {
                    out[i - 1] = out[i];
                }
                out[ALT_CAP - 1] = *alt;
            } else {
                out[n as usize] = *alt;
                n += 1;
            }
        }
        (out, n)
    }
}

impl Default for AltList {
    fn default() -> Self {
        Self::new()
    }
}

struct AgentRing {
    entity_id: u64,
    records: [DecisionRecord; RECORD_CAP],
    head: u16,
    count: u16,
}

impl AgentRing {
    const fn empty() -> Self {
        Self {
            entity_id: 0,
            records: [DecisionRecord::EMPTY; RECORD_CAP],
            head: 0,
            count: 0,
        }
    }
}

/// Preallocated rings for [`AGENT_CAP`] agents.
pub struct DecisionTrace {
    agents: Box<[AgentRing]>,
    len: u16,
}

impl DecisionTrace {
    #[must_use]
    pub fn new() -> Self {
        let mut agents = Vec::with_capacity(AGENT_CAP);
        for _ in 0..AGENT_CAP {
            agents.push(AgentRing::empty());
        }
        Self {
            agents: agents.into_boxed_slice(),
            len: 0,
        }
    }

    /// Bind `entity_id`. A second bind of the same id is a no-op success.
    /// Entity id 0 and a 33rd distinct id are refused.
    pub fn bind(&mut self, entity_id: u64) -> bool {
        if entity_id == 0 {
            return false;
        }
        if self.index_of(entity_id).is_some() {
            return true;
        }
        if self.len as usize >= AGENT_CAP {
            return false;
        }
        let slot = &mut self.agents[self.len as usize];
        slot.entity_id = entity_id;
        slot.head = 0;
        slot.count = 0;
        self.len += 1;
        true
    }

    #[must_use]
    pub fn agent_count(&self) -> u16 {
        self.len
    }

    pub fn push(&mut self, record: &DecisionRecord) -> bool {
        let Some(index) = self.index_of(record.entity_id) else {
            return false;
        };
        let slot = &mut self.agents[index];
        let write_at = (slot.head as usize) & (RECORD_CAP - 1);
        slot.records[write_at] = *record;
        slot.head = slot.head.wrapping_add(1);
        if (slot.count as usize) < RECORD_CAP {
            slot.count += 1;
        }
        true
    }

    #[must_use]
    pub fn count_of(&self, entity_id: u64) -> u16 {
        self.index_of(entity_id)
            .map(|index| self.agents[index].count)
            .unwrap_or(0)
    }

    #[must_use]
    pub fn find_tick(&self, entity_id: u64, tick: u64) -> Option<&DecisionRecord> {
        let index = self.index_of(entity_id)?;
        let slot = &self.agents[index];
        let n = slot.count as usize;
        for age in 0..n {
            let idx = newest_index(slot.head, age);
            let record = &slot.records[idx];
            if record.tick == tick {
                return Some(record);
            }
        }
        None
    }

    pub fn store_outcome(&mut self, entity_id: u64, tick: u64, code: FailCode) -> bool {
        let Some(index) = self.index_of(entity_id) else {
            return false;
        };
        let slot = &mut self.agents[index];
        let n = slot.count as usize;
        for age in 0..n {
            let idx = newest_index(slot.head, age);
            if slot.records[idx].tick == tick {
                slot.records[idx].fail = code as u16;
                slot.records[idx].outcome = outcome_byte(code);
                return true;
            }
        }
        false
    }

    pub fn amend_band(&mut self, entity_id: u64, tick: u64, band: u8) {
        let Some(index) = self.index_of(entity_id) else {
            return;
        };
        let slot = &mut self.agents[index];
        if slot.count == 0 {
            return;
        }
        let idx = newest_index(slot.head, 0);
        if slot.records[idx].tick == tick {
            slot.records[idx].band = band;
        }
    }

    fn index_of(&self, entity_id: u64) -> Option<usize> {
        self.agents[..self.len as usize]
            .iter()
            .position(|slot| slot.entity_id == entity_id)
    }
}

impl Default for DecisionTrace {
    fn default() -> Self {
        Self::new()
    }
}

fn newest_index(head: u16, age: usize) -> usize {
    (head.wrapping_sub(1).wrapping_sub(age as u16) as usize) & (RECORD_CAP - 1)
}

fn outcome_byte(code: FailCode) -> u8 {
    match code {
        FailCode::Pending => 0,
        FailCode::Ok => 1,
        _ => 2,
    }
}

pub trait DecisionSink {
    fn record(&mut self, record: &DecisionRecord);
    fn amend_band(&mut self, entity_id: u64, tick: u64, band: u8) {
        let _ = (entity_id, tick, band);
    }
}

/// Sink that drops the record. The planner result does not read it.
pub struct NullSink;

impl DecisionSink for NullSink {
    #[inline(always)]
    fn record(&mut self, _record: &DecisionRecord) {}
}

impl DecisionSink for DecisionTrace {
    fn record(&mut self, record: &DecisionRecord) {
        let _ = self.push(record);
    }

    fn amend_band(&mut self, entity_id: u64, tick: u64, band: u8) {
        DecisionTrace::amend_band(self, entity_id, tick, band);
    }
}

/// One-record sink used by [`rerun`] so the live ring stays put.
pub struct CaptureSink {
    pub record: DecisionRecord,
    pub wrote: bool,
}

impl DecisionSink for CaptureSink {
    fn record(&mut self, record: &DecisionRecord) {
        self.record = *record;
        self.wrote = true;
    }
}

impl Default for CaptureSink {
    fn default() -> Self {
        Self {
            record: DecisionRecord::EMPTY,
            wrote: false,
        }
    }
}

impl CaptureSink {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            record: DecisionRecord::EMPTY,
            wrote: false,
        }
    }
}

#[must_use]
pub fn tick_from_time(t: f32) -> u64 {
    if t.is_finite() && t >= 0.0 {
        (t * 1000.0) as u64
    } else {
        0
    }
}

#[must_use]
pub fn goal_from_objective(objective: Option<&str>) -> u16 {
    match objective {
        Some("cross") => GoalId::Cross as u16,
        Some("engage") => GoalId::Engage as u16,
        Some("hold") => GoalId::Hold as u16,
        Some("extract") => GoalId::Extract as u16,
        _ => GoalId::None as u16,
    }
}

#[must_use]
pub fn perceived_from_snap(snap: &WorldSnapshot) -> PerceivedView {
    let enemy = snap.enemies.first();
    let cd = snap.me.cooldowns.get("throw:smoke").copied().unwrap_or(0.0);
    let breach = snap.pois.iter().find(|poi| poi.k == "breach");
    let lost = snap.pois.iter().find(|poi| poi.k == "cover_lost");
    PerceivedView {
        self_x: snap.me.pos.x,
        self_y: snap.me.pos.y,
        ammo: snap.me.ammo,
        morale_milli: (snap.me.morale * 1000.0) as i32,
        smoke_cd_milli: if cd.is_finite() {
            (cd * 1000.0) as i32
        } else {
            0
        },
        enemy_count: u8::try_from(snap.enemies.len()).unwrap_or(u8::MAX),
        nearest_x: enemy.map(|enemy| enemy.pos.x).unwrap_or(0),
        nearest_y: enemy.map(|enemy| enemy.pos.y).unwrap_or(0),
        nearest_hp: enemy.map(|enemy| enemy.hp).unwrap_or(0),
        obstacle_count: u16::try_from(snap.obstacles.len()).unwrap_or(u16::MAX),
        breach_len: u8::from(breach.is_some()),
        breach_portal: 0,
        breach_x: breach.map(|poi| poi.pos.x).unwrap_or(0),
        breach_y: breach.map(|poi| poi.pos.y).unwrap_or(0),
        lost_cover_len: u8::from(lost.is_some()),
        lost_cover_id: 0,
    }
}

pub fn record_rule(
    trace: &mut DecisionTrace,
    entity_id: u64,
    tick: u64,
    snap: &WorldSnapshot,
    plan: &astraweave_core::PlanIntent,
) {
    let _ = trace.bind(entity_id);
    trace.record(&grid_record(
        entity_id,
        tick,
        PlannerId::Rule as u8,
        snap,
        plan,
        0,
        [],
    ));
}

pub fn record_utility(trace: &mut DecisionTrace, entity_id: u64, tick: u64, snap: &WorldSnapshot) {
    let _ = trace.bind(entity_id);
    trace.record(&utility_decision(entity_id, tick, snap));
}

fn grid_record(
    entity_id: u64,
    tick: u64,
    planner: u8,
    snap: &WorldSnapshot,
    plan: &astraweave_core::PlanIntent,
    score: i32,
    alts: impl IntoIterator<Item = AltScore>,
) -> DecisionRecord {
    let (selected, param0, param1) =
        plan.steps
            .first()
            .map(action_of_step)
            .unwrap_or((ActionCode::Empty as u16, 0, 0));
    let mut record = DecisionRecord::EMPTY;
    record.tick = tick;
    record.entity_id = entity_id;
    record.planner = planner;
    record.goal = goal_from_objective(snap.objective.as_deref());
    record.selected = selected;
    record.selected_score = score;
    record.param0 = param0;
    record.param1 = param1;
    record.outcome = outcome_byte(FailCode::Pending);
    record.fail = FailCode::Pending as u16;
    record.perceived = perceived_from_snap(snap);
    record.belief = BeliefView::EMPTY;
    for alt in alts {
        if record.alt_len == ALT_CAP as u8 {
            break;
        }
        record.alts[record.alt_len as usize] = alt;
        record.alt_len += 1;
    }
    record
}

#[must_use]
pub fn utility_decision(entity_id: u64, tick: u64, snap: &WorldSnapshot) -> DecisionRecord {
    let Some(pair) = htn::utility_pair(snap) else {
        let plan = astraweave_core::PlanIntent {
            plan_id: String::new(),
            steps: Vec::new(),
        };
        return grid_record(
            entity_id,
            tick,
            PlannerId::Utility as u8,
            snap,
            &plan,
            0,
            [],
        );
    };
    let choose_smoke = pair.smoke >= pair.advance;
    let (selected, score, param0, param1, reject, other_score) = if choose_smoke {
        let mid_x = (pair.me_x + pair.enemy_x) / 2;
        let mid_y = (pair.me_y + pair.enemy_y) / 2;
        (
            ActionCode::ThrowSmoke as u16,
            pair.smoke,
            mid_x,
            mid_y,
            if pair.smoke == pair.advance {
                RejectCode::TieLostToLowerId
            } else {
                RejectCode::LowerScore
            },
            pair.advance,
        )
    } else {
        (
            ActionCode::MoveTo as u16,
            pair.advance,
            pair.me_x + (pair.enemy_x - pair.me_x).signum(),
            pair.me_y + (pair.enemy_y - pair.me_y).signum(),
            if pair.smoke == i32::MIN {
                RejectCode::Cooldown
            } else {
                RejectCode::LowerScore
            },
            pair.smoke,
        )
    };
    let other_action = if choose_smoke {
        ActionCode::MoveTo as u16
    } else {
        ActionCode::ThrowSmoke as u16
    };
    let mut record = DecisionRecord::EMPTY;
    record.tick = tick;
    record.entity_id = entity_id;
    record.planner = PlannerId::Utility as u8;
    record.goal = goal_from_objective(snap.objective.as_deref());
    record.selected = selected;
    record.selected_score = score;
    record.param0 = param0;
    record.param1 = param1;
    record.outcome = outcome_byte(FailCode::Pending);
    record.fail = FailCode::Pending as u16;
    record.alt_len = 1;
    record.alts[0] = AltScore {
        action: other_action,
        reject: reject as u16,
        score: other_score,
        a: pair.enemy_x,
        b: pair.enemy_y,
    };
    record.perceived = perceived_from_snap(snap);
    record
}

#[must_use]
pub fn action_of_step(step: &ActionStep) -> (u16, i32, i32) {
    match step {
        ActionStep::MoveTo { x, y, .. } => (ActionCode::MoveTo as u16, *x, *y),
        ActionStep::Throw { x, y, .. } => (ActionCode::ThrowSmoke as u16, *x, *y),
        ActionStep::CoverFire { target_id, .. } => {
            (ActionCode::CoverFire as u16, *target_id as i32, 0)
        }
        ActionStep::TakeCover { position } => {
            let (x, y) = position.map(|pos| (pos.x, pos.y)).unwrap_or((0, 0));
            (ActionCode::TakeCover as u16, x, y)
        }
        ActionStep::Wait { duration } => (ActionCode::Wait as u16, (*duration * 1000.0) as i32, 0),
        _ => (ActionCode::Empty as u16, 0, 0),
    }
}

pub fn store_result(
    trace: &mut DecisionTrace,
    entity_id: u64,
    tick: u64,
    result: &Result<(), EngineError>,
) -> bool {
    let code = match result {
        Ok(()) => FailCode::Ok,
        Err(err) => fail_of(err),
    };
    trace.store_outcome(entity_id, tick, code)
}

fn fail_of(err: &EngineError) -> FailCode {
    match err {
        EngineError::InvalidAction(_) => FailCode::InvalidAction,
        EngineError::Cooldown(_) => FailCode::Cooldown,
        EngineError::LosBlocked => FailCode::LosBlocked,
        EngineError::NoPath => FailCode::NoPath,
        EngineError::Resource(_) => FailCode::Resource,
        _ => FailCode::InvalidAction,
    }
}

/// Rerun one recorded tick with a single patch. The live plan and the live
/// tiles are not written. `view.scratch` is a caller-owned buffer.
pub fn rerun(sample: TickSample, patch: Patch, view: &mut PlannerView<'_>) -> DecisionDiff {
    let mut perceived = sample.record.perceived;
    let mut goal = sample.goal_milli;
    let pos = sample.pos_milli;
    let mut seal: Option<(i32, i32)> = None;
    match patch {
        Patch::NearestEnemy { x, y } => {
            perceived.nearest_x = x;
            perceived.nearest_y = y;
            perceived.enemy_count = perceived.enemy_count.max(1);
        }
        Patch::SmokeCooldownMilli(cd) => perceived.smoke_cd_milli = cd,
        Patch::Ammo(ammo) => perceived.ammo = ammo,
        Patch::SealCell { x, y } => seal = Some((x, y)),
        Patch::GoalMilli { x, y, z } => goal = [x, y, z],
    }
    let planner = sample.record.planner;
    let after = if planner == PlannerId::Htn as u8 {
        rerun_htn(&sample, pos, goal, seal, view)
    } else if planner == PlannerId::Utility as u8 {
        let snap = snapshot_from_perceived(sample.record.tick, &perceived, sample.record.goal);
        utility_decision(sample.record.entity_id, sample.record.tick, &snap)
    } else {
        let snap = snapshot_from_perceived(sample.record.tick, &perceived, sample.record.goal);
        let plan = crate::RuleOrchestrator.propose_plan(&snap);
        grid_record(
            sample.record.entity_id,
            sample.record.tick,
            PlannerId::Rule as u8,
            &snap,
            &plan,
            0,
            [],
        )
    };
    let _ = pos;
    DecisionDiff {
        tick: sample.record.tick,
        entity_id: sample.record.entity_id,
        before: sample.record.selected,
        after: after.selected,
        before_score: sample.record.selected_score,
        after_score: after.selected_score,
        alt_len: after.alt_len,
        alts: after.alts,
    }
}

fn rerun_htn(
    sample: &TickSample,
    pos: [i32; 3],
    goal: [i32; 3],
    seal: Option<(i32, i32)>,
    view: &mut PlannerView<'_>,
) -> DecisionRecord {
    let mut hazard_buf = [astraweave_core::LocalHazardHook {
        hazard_id: 0,
        x: 0,
        y: 0,
        kind: 0,
    }; 8];
    let mut hazard_len = view.hooks.hazards.len().min(8);
    hazard_buf[..hazard_len].copy_from_slice(&view.hooks.hazards[..hazard_len]);
    if let Some((x, y)) = seal {
        if hazard_len < 8 {
            hazard_buf[hazard_len] = astraweave_core::LocalHazardHook {
                hazard_id: 1,
                x,
                y,
                kind: 1,
            };
            hazard_len += 1;
        }
    }
    let hooks = SpatialHooks {
        weaving: view.hooks.weaving,
        hazards: &hazard_buf[..hazard_len],
    };
    let mut plan = AgentPlan::empty();
    let mut budget = htn::HTN_EXPANSIONS_PER_TICK as u32;
    let mut capture = CaptureSink::default();
    plan_route(
        view.tiles,
        view.scratch,
        pos,
        goal,
        &hooks,
        sample.record.tick,
        &mut plan,
        &mut budget,
        &mut capture,
        sample.record.entity_id,
    );
    if capture.wrote {
        capture.record
    } else {
        DecisionRecord::EMPTY
    }
}

fn snapshot_from_perceived(tick: u64, perceived: &PerceivedView, goal: u16) -> WorldSnapshot {
    use astraweave_core::{CompanionState, CoverType, EnemyState, PlayerState};
    use std::collections::BTreeMap;
    let mut cooldowns = BTreeMap::new();
    if perceived.smoke_cd_milli > 0 {
        cooldowns.insert(
            "throw:smoke".to_string(),
            perceived.smoke_cd_milli as f32 / 1000.0,
        );
    }
    let enemies = if perceived.enemy_count == 0 {
        Vec::new()
    } else {
        vec![EnemyState {
            id: 1,
            pos: IVec2 {
                x: perceived.nearest_x,
                y: perceived.nearest_y,
            },
            hp: perceived.nearest_hp,
            cover: CoverType::Low,
            last_seen: 0.0,
        }]
    };
    let objective = match goal {
        g if g == GoalId::Cross as u16 => Some("cross".to_string()),
        g if g == GoalId::Engage as u16 => Some("engage".to_string()),
        g if g == GoalId::Hold as u16 => Some("hold".to_string()),
        g if g == GoalId::Extract as u16 => Some("extract".to_string()),
        _ => None,
    };
    WorldSnapshot {
        t: tick as f32 / 1000.0,
        player: PlayerState {
            hp: 100,
            pos: IVec2 { x: 0, y: 0 },
            stance: "stand".into(),
            orders: Vec::new(),
        },
        me: CompanionState {
            ammo: perceived.ammo,
            cooldowns,
            morale: perceived.morale_milli as f32 / 1000.0,
            pos: IVec2 {
                x: perceived.self_x,
                y: perceived.self_y,
            },
        },
        enemies,
        pois: Vec::new(),
        obstacles: Vec::new(),
        objective,
    }
}

#[must_use]
pub fn planner_name(id: u8) -> &'static str {
    match id {
        x if x == PlannerId::Rule as u8 => "rule",
        x if x == PlannerId::BehaviorTree as u8 => "behavior",
        x if x == PlannerId::GoapHeuristic as u8 => "goap",
        x if x == PlannerId::Utility as u8 => "utility",
        x if x == PlannerId::Htn as u8 => "htn",
        _ => "none",
    }
}

#[must_use]
pub fn action_name(code: u16) -> &'static str {
    match code {
        x if x == ActionCode::MoveTo as u16 => "move",
        x if x == ActionCode::UseBreach as u16 => "use breach",
        x if x == ActionCode::TakeCover as u16 => "take cover",
        x if x == ActionCode::Wait as u16 => "wait",
        x if x == ActionCode::ThrowSmoke as u16 => "throw smoke",
        x if x == ActionCode::CoverFire as u16 => "cover fire",
        _ => "empty",
    }
}

#[must_use]
pub fn reject_name(code: u16) -> &'static str {
    match code {
        x if x == RejectCode::Selected as u16 => "selected",
        x if x == RejectCode::LowerScore as u16 => "lower score",
        x if x == RejectCode::Cooldown as u16 => "cooldown",
        x if x == RejectCode::SealedCell as u16 => "sealed cell",
        x if x == RejectCode::OutOfBand as u16 => "out of band",
        x if x == RejectCode::BudgetExhausted as u16 => "budget exhausted",
        x if x == RejectCode::NoPath as u16 => "no path",
        x if x == RejectCode::TieLostToLowerId as u16 => "tie lost to lower id",
        _ => "unknown",
    }
}

#[must_use]
pub fn goal_name(code: u16) -> &'static str {
    match code {
        x if x == GoalId::Cross as u16 => "cross",
        x if x == GoalId::Engage as u16 => "engage",
        x if x == GoalId::Hold as u16 => "hold",
        x if x == GoalId::Extract as u16 => "extract",
        _ => "none",
    }
}

#[must_use]
pub fn outcome_name(outcome: u8, fail: u16) -> &'static str {
    match outcome {
        1 => "ok",
        2 => match fail {
            x if x == FailCode::InvalidAction as u16 => "invalid action",
            x if x == FailCode::Cooldown as u16 => "cooldown",
            x if x == FailCode::LosBlocked as u16 => "line of sight blocked",
            x if x == FailCode::NoPath as u16 => "no path",
            x if x == FailCode::Resource as u16 => "resource missing",
            _ => "failed",
        },
        _ => "pending",
    }
}

pub fn format_record(record: &DecisionRecord) -> String {
    let mut text = format!(
        "tick {tick}\nentity {entity}\nplanner {planner}\nband {band}\ngoal {goal}\nperceived self ({sx}, {sy}) enemies {enemies} breaches {breaches}\nbelief {belief}\nselected {action} {p0} {p1} score {score}\n",
        tick = record.tick,
        entity = record.entity_id,
        planner = planner_name(record.planner),
        band = record.band,
        goal = goal_name(record.goal),
        sx = record.perceived.self_x,
        sy = record.perceived.self_y,
        enemies = record.perceived.enemy_count,
        breaches = record.perceived.breach_len,
        belief = if record.belief.fact_hash == 0 { "none" } else { "recorded" },
        action = action_name(record.selected),
        p0 = record.param0,
        p1 = record.param1,
        score = record.selected_score,
    );
    for alt in record.alts.iter().take(record.alt_len as usize) {
        text.push_str(&format!(
            "rejected {action} {a} {b} score {score} {why}\n",
            action = action_name(alt.action),
            a = alt.a,
            b = alt.b,
            score = alt.score,
            why = reject_name(alt.reject),
        ));
    }
    text.push_str(&format!(
        "outcome {outcome}\n",
        outcome = outcome_name(record.outcome, record.fail)
    ));
    text
}

/// Build the route record from the plan the tick just wrote and the rejects
/// collected while scoring.
#[allow(clippy::too_many_arguments)]
pub fn route_record(
    entity_id: u64,
    tick: u64,
    band: u8,
    pos_milli: [i32; 3],
    plan: &AgentPlan,
    perceived: PerceivedView,
    alts: &AltList,
    selected_score: i32,
) -> DecisionRecord {
    let (selected, param0, param1) = first_slot(plan);
    let (rejected, alt_len) = alts.rejects(selected, param0);
    let mut record = DecisionRecord::EMPTY;
    record.tick = tick;
    record.entity_id = entity_id;
    record.planner = PlannerId::Htn as u8;
    record.band = band;
    record.goal = GoalId::Cross as u16;
    record.selected = selected;
    record.selected_score = selected_score;
    record.param0 = param0;
    record.param1 = param1;
    record.outcome = outcome_byte(FailCode::Pending);
    record.fail = FailCode::Pending as u16;
    record.alt_len = alt_len;
    record.alts = rejected;
    record.perceived = perceived;
    record.perceived.self_x = pos_milli[0] / 1000;
    record.perceived.self_y = pos_milli[2] / 1000;
    record
}

fn first_slot(plan: &AgentPlan) -> (u16, i32, i32) {
    for slot in plan.slots.iter().take(plan.len as usize) {
        match slot {
            htn::ActionSlot::MoveTo { x, y } => return (ActionCode::MoveTo as u16, *x, *y),
            htn::ActionSlot::UseBreach { portal_id } => {
                return (ActionCode::UseBreach as u16, *portal_id as i32, 0);
            }
            htn::ActionSlot::TakeCover { cover_id } => {
                return (ActionCode::TakeCover as u16, *cover_id as i32, 0);
            }
            htn::ActionSlot::Wait { duration_milli } => {
                return (ActionCode::Wait as u16, *duration_milli as i32, 0);
            }
            htn::ActionSlot::Empty => {}
        }
    }
    (ActionCode::Empty as u16, 0, 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ring_wraps_at_one_hundred_twenty_eight() {
        let mut trace = DecisionTrace::new();
        assert!(trace.bind(1));
        for tick in 0..129u64 {
            let mut record = DecisionRecord::EMPTY;
            record.tick = tick;
            record.entity_id = 1;
            assert!(trace.push(&record));
        }
        assert_eq!(trace.count_of(1), RECORD_CAP as u16);
        assert!(trace.find_tick(1, 0).is_none());
        assert!(trace.find_tick(1, 1).is_some());
        assert_eq!(trace.find_tick(1, 128).map(|record| record.tick), Some(128));
    }

    #[test]
    fn a_thirty_third_agent_is_refused() {
        let mut trace = DecisionTrace::new();
        for id in 1..=AGENT_CAP as u64 {
            assert!(trace.bind(id), "bind {id}");
        }
        assert!(!trace.bind((AGENT_CAP as u64) + 1));
        assert_eq!(trace.agent_count(), AGENT_CAP as u16);
        assert!(trace.bind(1));
        assert!(!trace.bind(0));
    }

    #[test]
    fn utility_records_the_lower_score_as_rejected() {
        use astraweave_core::{CompanionState, EnemyState, IVec2, PlayerState};
        use std::collections::BTreeMap;
        let snap = WorldSnapshot {
            t: 0.0,
            player: PlayerState {
                hp: 100,
                pos: IVec2 { x: 0, y: 0 },
                stance: "stand".into(),
                orders: Vec::new(),
            },
            me: CompanionState {
                ammo: 10,
                cooldowns: BTreeMap::new(),
                morale: 1.0,
                pos: IVec2 { x: 0, y: 0 },
            },
            enemies: vec![EnemyState {
                id: 2,
                pos: IVec2 { x: 1, y: 0 },
                hp: 10,
                cover: astraweave_core::CoverType::Low,
                last_seen: 0.0,
            }],
            pois: Vec::new(),
            obstacles: Vec::new(),
            objective: Some("engage".into()),
        };
        let record = utility_decision(4, 3, &snap);
        assert_eq!(record.planner, PlannerId::Utility as u8);
        assert_eq!(record.goal, GoalId::Engage as u16);
        assert!(record.alt_len >= 1);
        assert_ne!(record.alts[0].action, record.selected);
    }
}
