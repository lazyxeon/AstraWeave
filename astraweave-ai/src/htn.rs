//! Hierarchical task planner with utility-scored method choice.
//!
//! The tick writes into [`AgentPlan`] and [`astraweave_nav::NavScratch`].
//! It does not allocate and it does not take a lock. Expansions stop at
//! [`HTN_EXPANSIONS_PER_TICK`].

use astraweave_core::{
    ActionStep, IVec2, LocalHazardHook, ObservationBuf, PlanIntent, WeaveEffectHook, WorldSnapshot,
};
use astraweave_nav::{BreachSite, NavScratch, NavTiles};

/// Expansions spent on the spatial planner in one tick.
pub const HTN_EXPANSIONS_PER_TICK: usize = 64;
/// Full HTN inside this distance to the player.
pub const BAND0_MILLI: i32 = 32_000;
/// Utility method choice, no backtrack.
pub const BAND1_MILLI: i32 = 96_000;
/// Keep the last plan.
pub const BAND2_MILLI: i32 = 192_000;
/// Replan a near agent at least this often.
pub const REPLAN_TICKS: u16 = 30;
const PLAN_SLOTS: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionSlot {
    Empty,
    MoveTo { x: i32, y: i32 },
    UseBreach { portal_id: u32 },
    TakeCover { cover_id: u32 },
    Wait { duration_milli: u32 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AgentPlan {
    pub tick: u64,
    pub slots: [ActionSlot; PLAN_SLOTS],
    pub len: u8,
    pub band: u8,
}

impl AgentPlan {
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            tick: 0,
            slots: [ActionSlot::Empty; PLAN_SLOTS],
            len: 0,
            band: 3,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct AgentState {
    pub entity_id: u64,
    pub pos_milli: [i32; 3],
    pub goal_milli: [i32; 3],
    pub player_milli: [i32; 3],
    pub ticks_since_plan: u16,
    pub plan: AgentPlan,
}

/// What the planner can see besides the nav publish.
#[derive(Clone, Copy, Debug)]
pub struct SpatialHooks<'a> {
    pub weaving: &'a [WeaveEffectHook],
    pub hazards: &'a [LocalHazardHook],
}

impl SpatialHooks<'static> {
    #[must_use]
    pub const fn none() -> Self {
        Self {
            weaving: &[],
            hazards: &[],
        }
    }
}

/// Agents the spatial tick advances. Fixed capacity, no heap.
pub struct SpatialTick {
    pub nav: NavTiles,
    pub agents: [AgentState; 32],
    pub len: u16,
    pub expansions_used: u32,
    pub clock: u64,
    pub weaving: [WeaveEffectHook; 8],
    pub weaving_len: u8,
    pub hazards: [LocalHazardHook; 8],
    pub hazard_len: u8,
}

impl SpatialTick {
    #[must_use]
    pub fn new() -> Self {
        Self {
            nav: NavTiles::new(0.4, 45.0),
            agents: [AgentState {
                entity_id: 0,
                pos_milli: [0; 3],
                goal_milli: [0; 3],
                player_milli: [0; 3],
                ticks_since_plan: 0,
                plan: AgentPlan::empty(),
            }; 32],
            len: 0,
            expansions_used: 0,
            clock: 0,
            weaving: [WeaveEffectHook {
                effect_id: 0,
                x: 0,
                y: 0,
                kind: 0,
            }; 8],
            weaving_len: 0,
            hazards: [LocalHazardHook {
                hazard_id: 0,
                x: 0,
                y: 0,
                kind: 0,
            }; 8],
            hazard_len: 0,
        }
    }

    /// One agent on the bridge span, used by the ECS demo.
    #[must_use]
    pub fn demo_agent() -> Self {
        let mut tick = Self::new();
        tick.nav = astraweave_nav::bridge_and_ford();
        tick.agents[0] = AgentState {
            entity_id: 1,
            pos_milli: [1_000, 0, 1_000],
            goal_milli: [1_000, 0, 46_000],
            player_milli: [0, 0, 0],
            ticks_since_plan: REPLAN_TICKS,
            plan: AgentPlan::empty(),
        };
        tick.len = 1;
        tick
    }
}

impl Default for SpatialTick {
    fn default() -> Self {
        Self::new()
    }
}

pub struct HtnOrchestrator;

impl HtnOrchestrator {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Default for HtnOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

impl crate::Orchestrator for HtnOrchestrator {
    fn propose_plan(&self, snap: &WorldSnapshot) -> PlanIntent {
        plan_from_snapshot(snap)
    }
}

/// Legacy smoke-versus-advance choice. One implementation, used by
/// [`crate::UtilityOrchestrator`] and by [`plan_from_snapshot`] when the
/// snapshot has no breach.
#[must_use]
pub fn legacy_utility_plan(snap: &WorldSnapshot) -> PlanIntent {
    let plan_id = format!("util-{}", (snap.t * 1000.0) as i64);
    legacy_steps(snap, plan_id)
}

/// Copy nav breaches and lost cover into a perception buffer. No allocation.
pub fn fill_observation(tiles: &NavTiles, buf: &mut ObservationBuf) {
    buf.breach_len = 0;
    for breach in tiles.breaches.iter().take(tiles.breach_count as usize) {
        if buf.breach_len == 16 {
            break;
        }
        let i = buf.breach_len as usize;
        buf.breaches[i] = astraweave_core::ObservedBreach {
            portal_id: breach.portal_id,
            x: breach.x_milli / 1000,
            y: breach.z_milli / 1000,
        };
        buf.breach_len += 1;
    }
    buf.lost_cover_len = 0;
    for cover in tiles
        .lost_cover
        .iter()
        .take(tiles.lost_cover_count as usize)
    {
        if buf.lost_cover_len == 16 {
            break;
        }
        let i = buf.lost_cover_len as usize;
        buf.lost_cover[i] = astraweave_core::ObservedCover {
            id: cover.id,
            x: cover.x_milli / 1000,
            y: cover.z_milli / 1000,
        };
        buf.lost_cover_len += 1;
    }
}

#[cfg(feature = "decision-trace")]
std::thread_local! {
    static TRACE_ACTIVE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static TRACE_ENTITY: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    static TRACE_HOLDER: std::cell::RefCell<Option<crate::decision_trace::DecisionTrace>> =
        const { std::cell::RefCell::new(None) };
}

/// Swaps `trace` into thread-local storage until the guard drops.
///
/// `plan_route` keeps the same eight arguments with the feature on or off,
/// so the replay harness compiles either way and writes the same plan bytes.
/// With the feature off this function does not exist, and `plan_route` does
/// not touch the thread local.
#[cfg(feature = "decision-trace")]
pub struct TraceGuard {
    trace: *mut crate::decision_trace::DecisionTrace,
    prev_active: bool,
    prev_entity: u64,
}

#[cfg(feature = "decision-trace")]
#[must_use]
pub fn attach_trace(
    trace: &mut crate::decision_trace::DecisionTrace,
    entity_id: u64,
) -> TraceGuard {
    let prev_active = TRACE_ACTIVE.with(|cell| cell.replace(true));
    let prev_entity = TRACE_ENTITY.with(|cell| cell.replace(entity_id));
    let trace_ptr = trace as *mut crate::decision_trace::DecisionTrace;
    swap_holder(trace);
    TraceGuard {
        trace: trace_ptr,
        prev_active,
        prev_entity,
    }
}

#[cfg(feature = "decision-trace")]
fn swap_holder(trace: &mut crate::decision_trace::DecisionTrace) {
    TRACE_HOLDER.with(|cell| {
        let mut slot = cell.borrow_mut();
        if slot.is_none() {
            *slot = Some(crate::decision_trace::DecisionTrace::new());
        }
        if let Some(held) = slot.as_mut() {
            std::mem::swap(trace, held);
        }
    });
}

#[cfg(feature = "decision-trace")]
impl Drop for TraceGuard {
    fn drop(&mut self) {
        if !self.trace.is_null() {
            // SAFETY: `attach_trace` created this pointer from the caller's
            // `&mut DecisionTrace`, and this guard is dropped before that
            // borrow ends. The swap returns the caller's rings.
            unsafe { swap_holder(&mut *self.trace) }
        }
        TRACE_ACTIVE.with(|cell| cell.set(self.prev_active));
        TRACE_ENTITY.with(|cell| cell.set(self.prev_entity));
    }
}

#[cfg(feature = "decision-trace")]
fn set_trace_entity(entity_id: u64) {
    TRACE_ENTITY.with(|cell| cell.set(entity_id));
}

#[cfg(feature = "decision-trace")]
fn emit_record(record: &crate::decision_trace::DecisionRecord) {
    if !TRACE_ACTIVE.with(|cell| cell.get()) {
        return;
    }
    TRACE_HOLDER.with(|cell| {
        if let Some(trace) = cell.borrow_mut().as_mut() {
            let _ = trace.push(record);
        }
    });
}

#[cfg(feature = "decision-trace")]
fn emit_band(entity_id: u64, tick: u64, band: u8) {
    if !TRACE_ACTIVE.with(|cell| cell.get()) {
        return;
    }
    TRACE_HOLDER.with(|cell| {
        if let Some(trace) = cell.borrow_mut().as_mut() {
            trace.amend_band(entity_id, tick, band);
        }
    });
}

/// Advance every agent. Sort key is `(band, entity_id)`.
#[allow(clippy::needless_range_loop)]
pub fn tick_agents(tick: &mut SpatialTick, scratch: &mut NavScratch, sim_tick: u64) {
    let wlen = tick.weaving_len as usize;
    let hlen = tick.hazard_len as usize;
    let mut weave_buf = [WeaveEffectHook {
        effect_id: 0,
        x: 0,
        y: 0,
        kind: 0,
    }; 8];
    let mut hazard_buf = [LocalHazardHook {
        hazard_id: 0,
        x: 0,
        y: 0,
        kind: 0,
    }; 8];
    weave_buf[..wlen].copy_from_slice(&tick.weaving[..wlen]);
    hazard_buf[..hlen].copy_from_slice(&tick.hazards[..hlen]);
    let SpatialTick {
        nav,
        agents,
        len,
        expansions_used,
        weaving: _,
        weaving_len: _,
        hazards: _,
        hazard_len: _,
        clock: _,
    } = tick;
    let n = *len as usize;
    let mut order = [0u16; 32];
    for i in 0..n {
        order[i] = i as u16;
    }
    for i in 1..n {
        let mut j = i;
        while j > 0 {
            let a = agents[order[j - 1] as usize];
            let b = agents[order[j] as usize];
            let band_a = band_of(a.pos_milli, a.player_milli);
            let band_b = band_of(b.pos_milli, b.player_milli);
            let out = band_a > band_b || (band_a == band_b && a.entity_id > b.entity_id);
            if !out {
                break;
            }
            order.swap(j - 1, j);
            j -= 1;
        }
    }
    let hook_view = SpatialHooks {
        weaving: &weave_buf[..wlen],
        hazards: &hazard_buf[..hlen],
    };
    let mut budget = HTN_EXPANSIONS_PER_TICK as u32;
    for i in 0..n {
        if budget == 0 {
            break;
        }
        let idx = order[i] as usize;
        let agent = &mut agents[idx];
        let band = band_of(agent.pos_milli, agent.player_milli);
        agent.ticks_since_plan = agent.ticks_since_plan.saturating_add(1);
        match band {
            0 if agent.ticks_since_plan >= REPLAN_TICKS || agent.plan.len == 0 => {
                #[cfg(feature = "decision-trace")]
                set_trace_entity(agent.entity_id);
                plan_route(
                    nav,
                    scratch,
                    agent.pos_milli,
                    agent.goal_milli,
                    &hook_view,
                    sim_tick,
                    &mut agent.plan,
                    &mut budget,
                );
                agent.plan.band = 0;
                agent.ticks_since_plan = 0;
                #[cfg(feature = "decision-trace")]
                emit_band(agent.entity_id, sim_tick, 0);
            }
            1 => {
                #[cfg(feature = "decision-trace")]
                set_trace_entity(agent.entity_id);
                plan_route(
                    nav,
                    scratch,
                    agent.pos_milli,
                    agent.goal_milli,
                    &hook_view,
                    sim_tick,
                    &mut agent.plan,
                    &mut budget,
                );
                agent.plan.band = 1;
                agent.ticks_since_plan = 0;
                #[cfg(feature = "decision-trace")]
                emit_band(agent.entity_id, sim_tick, 1);
            }
            2 if agent.plan.len == 0 => {
                agent.plan.slots[0] = ActionSlot::MoveTo {
                    x: agent.goal_milli[0] / 1000,
                    y: agent.goal_milli[2] / 1000,
                };
                agent.plan.len = 1;
                agent.plan.band = 2;
                agent.plan.tick = sim_tick;
                budget = budget.saturating_sub(1);
                #[cfg(feature = "decision-trace")]
                {
                    let record = crate::decision_trace::route_record(
                        agent.entity_id,
                        sim_tick,
                        2,
                        agent.pos_milli,
                        &agent.plan,
                        crate::decision_trace::PerceivedView::EMPTY,
                        &crate::decision_trace::AltList::new(),
                        0,
                    );
                    emit_record(&record);
                }
            }
            _ => {
                agent.plan.band = band;
            }
        }
    }
    *expansions_used = HTN_EXPANSIONS_PER_TICK as u32 - budget;
}

/// Decompose "cross to the goal" into a breach, cover, or a direct step.
#[allow(clippy::too_many_arguments)]
pub fn plan_route(
    tiles: &NavTiles,
    scratch: &mut NavScratch,
    pos_milli: [i32; 3],
    goal_milli: [i32; 3],
    hooks: &SpatialHooks<'_>,
    sim_tick: u64,
    out: &mut AgentPlan,
    budget: &mut u32,
) {
    #[cfg(feature = "decision-trace")]
    clear_alts();
    if *budget == 0 {
        return;
    }
    *budget = budget.saturating_sub(1);
    out.tick = sim_tick;
    out.len = 0;
    out.slots = [ActionSlot::Empty; PLAN_SLOTS];
    let start = Vec3::new(
        pos_milli[0] as f32 / 1000.0,
        pos_milli[1] as f32 / 1000.0,
        pos_milli[2] as f32 / 1000.0,
    );
    let goal = Vec3::new(
        goal_milli[0] as f32 / 1000.0,
        goal_milli[1] as f32 / 1000.0,
        goal_milli[2] as f32 / 1000.0,
    );
    let waypoints = tiles.query_path(scratch, start, goal);
    if waypoints < 2 {
        out.slots[0] = ActionSlot::Wait {
            duration_milli: 500,
        };
        out.len = 1;
        return;
    }
    *budget = budget.saturating_sub(1);
    if let Some(portal) = best_breach(tiles, scratch, hooks) {
        out.slots[0] = ActionSlot::UseBreach { portal_id: portal };
        out.len = 1;
    }
    let mut far = scratch.waypoints[0];
    for point in scratch.waypoints.iter().take(scratch.waypoint_len as usize) {
        if point.x.abs() > far.x.abs() {
            far = *point;
        }
    }
    let slot = out.len as usize;
    if slot < PLAN_SLOTS {
        out.slots[slot] = ActionSlot::MoveTo {
            x: (far.x * 1000.0) as i32 / 1000,
            y: (far.z * 1000.0) as i32 / 1000,
        };
        out.len += 1;
    }
    if tiles.lost_cover_count > 0 {
        if let Some(cover) = best_cover(tiles, pos_milli) {
            let slot = out.len as usize;
            if slot < PLAN_SLOTS {
                *budget = budget.saturating_sub(1);
                out.slots[slot] = ActionSlot::TakeCover { cover_id: cover };
                out.len += 1;
            }
        }
    }
    #[cfg(feature = "decision-trace")]
    {
        let perceived = perceived_for_route(tiles, pos_milli);
        let alts = copy_alts();
        let score = route_score(tiles, out);
        let entity_id = TRACE_ENTITY.with(|cell| cell.get());
        let record = crate::decision_trace::route_record(
            entity_id, sim_tick, 0, pos_milli, out, perceived, &alts, score,
        );
        emit_record(&record);
    }
}

fn best_breach(tiles: &NavTiles, scratch: &NavScratch, hooks: &SpatialHooks<'_>) -> Option<u32> {
    let mut best: Option<(u32, i32)> = None;
    for i in 0..scratch.portal_len as usize {
        let id = scratch.portal_ids[i];
        let Some(site) = find_breach(tiles, id) else {
            continue;
        };
        let cell_x = site.x_milli / 1000;
        let cell_y = site.z_milli / 1000;
        if sealed(hooks, cell_x, cell_y) {
            #[cfg(feature = "decision-trace")]
            push_alt(crate::decision_trace::AltScore {
                action: crate::decision_trace::ActionCode::UseBreach as u16,
                reject: crate::decision_trace::RejectCode::SealedCell as u16,
                score: i32::MIN,
                a: id as i32,
                b: 0,
            });
            continue;
        }
        let score = -site.x_milli;
        let take = match best {
            None => true,
            Some((best_id, best_score)) => {
                score > best_score || (score == best_score && id < best_id)
            }
        };
        if take {
            #[cfg(feature = "decision-trace")]
            if let Some((old_id, old_score)) = best {
                let reject = if score == old_score {
                    crate::decision_trace::RejectCode::TieLostToLowerId
                } else {
                    crate::decision_trace::RejectCode::LowerScore
                };
                push_alt(crate::decision_trace::AltScore {
                    action: crate::decision_trace::ActionCode::UseBreach as u16,
                    reject: reject as u16,
                    score: old_score,
                    a: old_id as i32,
                    b: 0,
                });
            }
            best = Some((id, score));
        } else {
            #[cfg(feature = "decision-trace")]
            {
                let best_score = best.map(|(_, best_score)| best_score).unwrap_or(score);
                let reject = if score == best_score {
                    crate::decision_trace::RejectCode::TieLostToLowerId
                } else {
                    crate::decision_trace::RejectCode::LowerScore
                };
                push_alt(crate::decision_trace::AltScore {
                    action: crate::decision_trace::ActionCode::UseBreach as u16,
                    reject: reject as u16,
                    score,
                    a: id as i32,
                    b: 0,
                });
            }
        }
    }
    best.map(|(id, _)| id)
}

fn find_breach(tiles: &NavTiles, id: u32) -> Option<BreachSite> {
    tiles
        .breaches
        .iter()
        .take(tiles.breach_count as usize)
        .find(|site| site.portal_id == id)
        .copied()
}

fn sealed(hooks: &SpatialHooks<'_>, x: i32, y: i32) -> bool {
    let hazard = hooks
        .hazards
        .iter()
        .any(|hazard| hazard.x == x && hazard.y == y && hazard.kind != 0);
    let weave = hooks
        .weaving
        .iter()
        .any(|effect| effect.x == x && effect.y == y && effect.kind != 0);
    hazard || weave
}

fn best_cover(tiles: &NavTiles, pos_milli: [i32; 3]) -> Option<u32> {
    let mut best: Option<(u32, i32)> = None;
    for cover in tiles.cover.iter().take(tiles.cover_count as usize) {
        let dist = (cover.x_milli - pos_milli[0]).abs() + (cover.z_milli - pos_milli[2]).abs();
        let take = match best {
            None => true,
            Some((id, best_dist)) => dist < best_dist || (dist == best_dist && cover.id < id),
        };
        if take {
            #[cfg(feature = "decision-trace")]
            if let Some((old_id, old_dist)) = best {
                push_alt(crate::decision_trace::AltScore {
                    action: crate::decision_trace::ActionCode::TakeCover as u16,
                    reject: crate::decision_trace::RejectCode::LowerScore as u16,
                    score: -old_dist,
                    a: old_id as i32,
                    b: 0,
                });
            }
            best = Some((cover.id, dist));
        } else {
            #[cfg(feature = "decision-trace")]
            push_alt(crate::decision_trace::AltScore {
                action: crate::decision_trace::ActionCode::TakeCover as u16,
                reject: crate::decision_trace::RejectCode::LowerScore as u16,
                score: -dist,
                a: cover.id as i32,
                b: 0,
            });
        }
    }
    best.map(|(id, _)| id)
}

#[cfg(feature = "decision-trace")]
std::thread_local! {
    static ROUTE_ALTS: std::cell::RefCell<crate::decision_trace::AltList> =
        const { std::cell::RefCell::new(crate::decision_trace::AltList::new()) };
}

#[cfg(feature = "decision-trace")]
fn clear_alts() {
    ROUTE_ALTS.with(|cell| {
        if let Ok(mut alts) = cell.try_borrow_mut() {
            alts.clear();
        }
    });
}

#[cfg(feature = "decision-trace")]
fn push_alt(alt: crate::decision_trace::AltScore) {
    ROUTE_ALTS.with(|cell| {
        if let Ok(mut alts) = cell.try_borrow_mut() {
            alts.push(alt);
        }
    });
}

#[cfg(feature = "decision-trace")]
fn copy_alts() -> crate::decision_trace::AltList {
    ROUTE_ALTS.with(|cell| {
        cell.try_borrow()
            .map(|alts| *alts)
            .unwrap_or_else(|_| crate::decision_trace::AltList::new())
    })
}

#[cfg(feature = "decision-trace")]
fn perceived_for_route(
    tiles: &NavTiles,
    pos_milli: [i32; 3],
) -> crate::decision_trace::PerceivedView {
    let mut perceived = crate::decision_trace::PerceivedView::EMPTY;
    perceived.self_x = pos_milli[0] / 1000;
    perceived.self_y = pos_milli[2] / 1000;
    perceived.breach_len = u8::try_from(tiles.breach_count).unwrap_or(u8::MAX);
    if let Some(site) = tiles.breaches.first() {
        if tiles.breach_count > 0 {
            perceived.breach_portal = site.portal_id;
            perceived.breach_x = site.x_milli / 1000;
            perceived.breach_y = site.z_milli / 1000;
        }
    }
    perceived.lost_cover_len = u8::try_from(tiles.lost_cover_count).unwrap_or(u8::MAX);
    if tiles.lost_cover_count > 0 {
        perceived.lost_cover_id = tiles.lost_cover[0].id;
    }
    perceived
}

#[cfg(feature = "decision-trace")]
fn route_score(tiles: &NavTiles, plan: &AgentPlan) -> i32 {
    for slot in plan.slots.iter().take(plan.len as usize) {
        if let ActionSlot::UseBreach { portal_id } = slot {
            if let Some(site) = find_breach(tiles, *portal_id) {
                return -site.x_milli;
            }
        }
    }
    0
}

fn band_of(pos: [i32; 3], player: [i32; 3]) -> u8 {
    let dist = (pos[0] - player[0]).abs() + (pos[2] - player[2]).abs();
    if dist <= BAND0_MILLI {
        0
    } else if dist <= BAND1_MILLI {
        1
    } else if dist <= BAND2_MILLI {
        2
    } else {
        3
    }
}

fn plan_from_snapshot(snap: &WorldSnapshot) -> PlanIntent {
    let plan_id = format!("htn-{}", (snap.t * 1000.0) as i64);
    if let Some(breach) = snap.pois.iter().find(|poi| poi.k == "breach") {
        return PlanIntent {
            plan_id,
            steps: vec![ActionStep::MoveTo {
                x: breach.pos.x,
                y: breach.pos.y,
                speed: None,
            }],
        };
    }
    legacy_steps(snap, plan_id)
}

pub(crate) struct UtilityPair {
    pub smoke: i32,
    pub advance: i32,
    pub dist: i32,
    pub me_x: i32,
    pub me_y: i32,
    pub enemy_x: i32,
    pub enemy_y: i32,
    pub enemy_id: u32,
}

/// Integer smoke-versus-advance scores. Equal scores keep smoke.
pub(crate) fn utility_pair(snap: &WorldSnapshot) -> Option<UtilityPair> {
    let enemy = snap.enemies.first()?;
    let me = &snap.me;
    let cd = me.cooldowns.get("throw:smoke").copied().unwrap_or(0.0);
    let dx = (enemy.pos.x - me.pos.x).abs();
    let dy = (enemy.pos.y - me.pos.y).abs();
    let dist = dx + dy;
    let smoke = if cd <= 0.0 {
        1000 + enemy.hp.saturating_mul(10)
    } else {
        i32::MIN
    };
    let close_bonus = (3 - dist).max(0) * 50;
    Some(UtilityPair {
        smoke,
        advance: 800 + close_bonus,
        dist,
        me_x: me.pos.x,
        me_y: me.pos.y,
        enemy_x: enemy.pos.x,
        enemy_y: enemy.pos.y,
        enemy_id: enemy.id,
    })
}

fn legacy_steps(snap: &WorldSnapshot, plan_id: String) -> PlanIntent {
    let Some(pair) = utility_pair(snap) else {
        return PlanIntent {
            plan_id,
            steps: vec![],
        };
    };
    // Integer form of the old scores: smoke 1000 + hp*10, advance 800 + close bonus.
    // Higher wins. Equal scores take the lower method id (smoke is 0).
    let smoke_score = pair.smoke;
    let advance_score = pair.advance;
    let dist = pair.dist;
    if smoke_score >= advance_score {
        let mid = IVec2 {
            x: (pair.me_x + pair.enemy_x) / 2,
            y: (pair.me_y + pair.enemy_y) / 2,
        };
        PlanIntent {
            plan_id,
            steps: vec![
                ActionStep::Throw {
                    item: "smoke".into(),
                    x: mid.x,
                    y: mid.y,
                },
                ActionStep::MoveTo {
                    speed: None,
                    x: pair.me_x + (pair.enemy_x - pair.me_x).signum() * 2,
                    y: pair.me_y + (pair.enemy_y - pair.me_y).signum() * 2,
                },
            ],
        }
    } else {
        let mut steps = vec![ActionStep::MoveTo {
            speed: None,
            x: pair.me_x + (pair.enemy_x - pair.me_x).signum(),
            y: pair.me_y + (pair.enemy_y - pair.me_y).signum(),
        }];
        if dist <= 3 {
            steps.push(ActionStep::CoverFire {
                target_id: pair.enemy_id,
                duration: 1.0,
            });
        }
        PlanIntent { plan_id, steps }
    }
}

use glam::Vec3;

#[cfg(test)]
mod tests {
    use super::*;
    use astraweave_core::{LocalHazardHook, WeaveEffectHook};
    use astraweave_nav::{bridge_and_ford, NavChangeKind, NavScratch, FRAME_BUDGET_MS};

    #[test]
    fn equal_scores_pick_the_lower_breach_id() {
        let tiles = bridge_and_ford();
        let mut scratch = NavScratch::new();
        let _ = tiles.query_path(
            &mut scratch,
            Vec3::new(1.0, 0.0, 1.0),
            Vec3::new(1.0, 0.0, 46.0),
        );
        let hooks = SpatialHooks::none();
        let mut first = None;
        let mut second = None;
        for _ in 0..2 {
            let id = best_breach(&tiles, &scratch, &hooks);
            if first.is_none() {
                first = id;
            } else {
                second = id;
            }
        }
        assert_eq!(first, second);
    }

    #[test]
    fn a_hazard_or_weave_on_the_cell_drops_the_breach() {
        let tiles = bridge_and_ford();
        let mut scratch = NavScratch::new();
        assert!(
            tiles.query_path(
                &mut scratch,
                Vec3::new(1.0, 0.0, 1.0),
                Vec3::new(1.0, 0.0, 46.0),
            ) >= 2
        );
        let portal = best_breach(&tiles, &scratch, &SpatialHooks::none())
            .expect("the first publish marks the bridge portal as a breach");
        let site = find_breach(&tiles, portal).expect("portal is a breach on first publish");
        let hazard = [LocalHazardHook {
            hazard_id: 1,
            x: site.x_milli / 1000,
            y: site.z_milli / 1000,
            kind: 1,
        }];
        let hooks = SpatialHooks {
            weaving: &[],
            hazards: &hazard,
        };
        assert_ne!(best_breach(&tiles, &scratch, &hooks), Some(portal));
        let weave = [WeaveEffectHook {
            effect_id: 2,
            x: site.x_milli / 1000,
            y: site.z_milli / 1000,
            kind: 1,
        }];
        let hooks = SpatialHooks {
            weaving: &weave,
            hazards: &[],
        };
        assert_ne!(best_breach(&tiles, &scratch, &hooks), Some(portal));
    }

    #[test]
    fn sixty_four_expansions_fit_in_the_frame_budget() {
        let tiles = bridge_and_ford();
        let mut scratch = NavScratch::new();
        let hooks = SpatialHooks::none();
        let start = std::time::Instant::now();
        let mut budget = HTN_EXPANSIONS_PER_TICK as u32;
        let mut plan = AgentPlan::empty();
        while budget > 0 {
            let before = budget;
            plan_route(
                &tiles,
                &mut scratch,
                [1_000, 0, 1_000],
                [1_000, 0, 46_000],
                &hooks,
                1,
                &mut plan,
                &mut budget,
            );
            if budget == before {
                break;
            }
        }
        let elapsed_ms = start.elapsed().as_secs_f32() * 1000.0;
        assert!(plan.len > 0);
        assert!(
            elapsed_ms < FRAME_BUDGET_MS,
            "{HTN_EXPANSIONS_PER_TICK} expansions took {elapsed_ms} ms"
        );
    }

    #[test]
    fn reroute_after_the_bridge_drops() {
        let mut tiles = bridge_and_ford();
        let mut scratch = NavScratch::new();
        let hooks = SpatialHooks::none();
        let mut budget = HTN_EXPANSIONS_PER_TICK as u32;
        let mut plan = AgentPlan::empty();
        plan_route(
            &tiles,
            &mut scratch,
            [1_000, 0, 1_000],
            [1_000, 0, 46_000],
            &hooks,
            1,
            &mut plan,
            &mut budget,
        );
        let before = move_cell(&plan);
        tiles.mark(astraweave_nav::nav_change_from_kind(
            NavChangeKind::Detach,
            30,
            0,
            [-1_000, -1_000, 17_000],
            [5_000, 1_000, 31_999],
        ));
        assert!(tiles.commit_tick() >= 1);
        plan_route(
            &tiles,
            &mut scratch,
            [1_000, 0, 1_000],
            [1_000, 0, 46_000],
            &hooks,
            31,
            &mut plan,
            &mut budget,
        );
        let after = move_cell(&plan);
        assert!(before < 8, "bridge path cell {before}");
        assert!(after > 8, "ford cell {after}, bridge cell was {before}");
    }

    fn move_cell(plan: &AgentPlan) -> i32 {
        for slot in &plan.slots {
            if let ActionSlot::MoveTo { x, .. } = slot {
                return *x;
            }
        }
        -1
    }
}
