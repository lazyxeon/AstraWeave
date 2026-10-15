//! Headless inspector for the Aravath bridge companion.
//!
//! The script uses the same span as the nav reroute case. It does not call
//! the greybox `BridgeAgent` type, which lives on another branch.

use std::fs;
use std::path::Path;

use astraweave_ai::decision_trace::{
    format_record, rerun, DecisionTrace, Patch, PlannerView, TickSample,
};
use astraweave_ai::htn::{plan_route, AgentPlan, SpatialHooks};
use astraweave_nav::{bridge_and_ford, nav_change_from_kind, NavChangeKind, NavScratch, NavTiles};

/// Player-facing scene name.
pub const SCENE_LABEL: &str = "Aravath";

const ENTITY: u64 = 1;
const POS: [i32; 3] = [1_000, 0, 1_000];
const GOAL: [i32; 3] = [1_000, 0, 46_000];

pub fn show(tick: u64, entity: u64) -> Result<String, String> {
    let (trace, _tiles) = simulate_through(tick.max(31));
    let record = trace.find_tick(entity, tick).ok_or_else(|| {
        format!(
            "no record for entity {entity} at tick {tick} (held {})",
            trace.count_of(entity)
        )
    })?;
    Ok(format!("{SCENE_LABEL}\n{}", format_record(record)))
}

pub fn diff_seal(tick: u64, entity: u64, x: i32, y: i32) -> Result<String, String> {
    let (trace, tiles) = simulate_through(tick);
    let record = trace
        .find_tick(entity, tick)
        .ok_or_else(|| format!("no record for entity {entity} at tick {tick}"))?;
    let mut sample = TickSample::from_record(*record);
    sample.pos_milli = POS;
    sample.goal_milli = GOAL;
    let mut scratch = NavScratch::new();
    let hooks = SpatialHooks::none();
    let mut view = PlannerView {
        tiles: &tiles,
        scratch: &mut scratch,
        hooks,
    };
    let diff = rerun(sample, Patch::SealCell { x, y }, &mut view);
    let mut text = format!(
        "{SCENE_LABEL}\ntick {tick}\nentity {entity}\nbefore {before} score {before_score}\nafter {after} score {after_score}\n",
        before = astraweave_ai::decision_trace::action_name(diff.before),
        before_score = diff.before_score,
        after = astraweave_ai::decision_trace::action_name(diff.after),
        after_score = diff.after_score,
    );
    for alt in diff.alts.iter().take(diff.alt_len as usize) {
        text.push_str(&format!(
            "rejected {} {} {} score {} {}\n",
            astraweave_ai::decision_trace::action_name(alt.action),
            alt.a,
            alt.b,
            alt.score,
            astraweave_ai::decision_trace::reject_name(alt.reject),
        ));
    }
    Ok(text)
}

pub fn dump_record(path: &Path, tick: u64, entity: u64) -> Result<(), String> {
    let (trace, _tiles) = simulate_through(tick);
    let record = trace
        .find_tick(entity, tick)
        .ok_or_else(|| format!("no record for entity {entity} at tick {tick}"))?;
    let bytes = record_bytes(record);
    fs::write(path, bytes).map_err(|err| format!("writing {}: {err}", path.display()))
}

fn record_bytes(record: &astraweave_ai::decision_trace::DecisionRecord) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"AWDT");
    out.extend_from_slice(&1u32.to_le_bytes());
    out.extend_from_slice(&record.tick.to_le_bytes());
    out.extend_from_slice(&record.entity_id.to_le_bytes());
    out.push(record.planner);
    out.push(record.band);
    out.extend_from_slice(&record.goal.to_le_bytes());
    out.extend_from_slice(&record.selected.to_le_bytes());
    out.extend_from_slice(&record.selected_score.to_le_bytes());
    out.extend_from_slice(&record.param0.to_le_bytes());
    out.extend_from_slice(&record.param1.to_le_bytes());
    out.push(record.outcome);
    out.extend_from_slice(&record.fail.to_le_bytes());
    out.push(record.alt_len);
    for alt in &record.alts {
        out.extend_from_slice(&alt.action.to_le_bytes());
        out.extend_from_slice(&alt.reject.to_le_bytes());
        out.extend_from_slice(&alt.score.to_le_bytes());
        out.extend_from_slice(&alt.a.to_le_bytes());
        out.extend_from_slice(&alt.b.to_le_bytes());
    }
    out
}

fn simulate_through(last_tick: u64) -> (DecisionTrace, NavTiles) {
    let mut tiles = bridge_and_ford();
    let mut scratch = NavScratch::new();
    let hooks = SpatialHooks::none();
    let mut trace = DecisionTrace::new();
    let _ = trace.bind(ENTITY);
    for tick in 0..=last_tick {
        if tick == 30 {
            tiles.mark(nav_change_from_kind(
                NavChangeKind::Detach,
                tick,
                0,
                [-1_000, -1_000, 17_000],
                [5_000, 1_000, 31_999],
            ));
        }
        let _ = tiles.commit_tick();
        let mut budget = 64u32;
        let mut plan = AgentPlan::empty();
        let _guard = astraweave_ai::htn::attach_trace(&mut trace, ENTITY);
        plan_route(
            &tiles,
            &mut scratch,
            POS,
            GOAL,
            &hooks,
            tick,
            &mut plan,
            &mut budget,
        );
    }
    (trace, tiles)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn show_prints_the_bridge_decision() {
        let text = show(31, 1).expect("record");
        assert!(text.starts_with("Aravath\n"), "{text}");
        assert!(text.contains("planner htn"), "{text}");
        assert!(text.contains("selected "), "{text}");
        assert!(text.contains("outcome "), "{text}");
        assert!(text.contains("rejected "), "{text}");
    }

    #[test]
    fn diff_seal_prints_both_actions() {
        let shown = show(1, 1).expect("tick 1");
        let breach_line = shown
            .lines()
            .find(|line| line.starts_with("perceived "))
            .expect("perceived");
        let _ = breach_line;
        let text = diff_seal(1, 1, 0, 0).expect("diff");
        assert!(text.starts_with("Aravath\n"), "{text}");
        assert!(text.contains("before "), "{text}");
        assert!(text.contains("after "), "{text}");
    }
}
