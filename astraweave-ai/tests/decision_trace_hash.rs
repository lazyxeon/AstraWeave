//! Plan bytes stay identical when the decision ring records them.
//!
//! ```text
//! cargo test -p astraweave-ai --features decision-trace --test decision_trace_hash
//! ```

#![cfg(feature = "decision-trace")]

use astraweave_ai::decision_trace::{DecisionSink, DecisionTrace, NullSink, Patch, PlannerView};
use astraweave_ai::htn::{plan_route, AgentPlan, SpatialHooks};
use astraweave_nav::{bridge_and_ford, nav_change_from_kind, NavChangeKind, NavScratch};
use sha2::{Digest, Sha256};

struct RingSink {
    trace: DecisionTrace,
}

impl DecisionSink for RingSink {
    fn record(&mut self, record: &astraweave_ai::decision_trace::DecisionRecord) {
        let _ = self.trace.push(record);
    }
}

fn script_bytes<S: DecisionSink>(sink: &mut S) -> Vec<u8> {
    let mut tiles = bridge_and_ford();
    let mut scratch = NavScratch::new();
    let hooks = SpatialHooks::none();
    let mut out = Vec::new();
    for tick in 0..40u64 {
        if tick == 30 {
            tiles.mark(nav_change_from_kind(
                NavChangeKind::Detach,
                tick,
                0,
                [-1_000, -1_000, 17_000],
                [5_000, 1_000, 31_999],
            ));
        }
        let committed = tiles.commit_tick() as u32;
        let mut budget = 64u32;
        let mut plan = AgentPlan::empty();
        plan_route(
            &tiles,
            &mut scratch,
            [1_000, 0, 1_000],
            [1_000, 0, 46_000],
            &hooks,
            tick,
            &mut plan,
            &mut budget,
            sink,
            1,
        );
        out.extend_from_slice(&tick.to_le_bytes());
        out.extend_from_slice(&committed.to_le_bytes());
        out.push(plan.len);
        for slot in plan.slots.iter().take(plan.len as usize) {
            match slot {
                astraweave_ai::htn::ActionSlot::Empty => out.push(0),
                astraweave_ai::htn::ActionSlot::MoveTo { x, y } => {
                    out.push(1);
                    out.extend_from_slice(&x.to_le_bytes());
                    out.extend_from_slice(&y.to_le_bytes());
                }
                astraweave_ai::htn::ActionSlot::UseBreach { portal_id } => {
                    out.push(2);
                    out.extend_from_slice(&portal_id.to_le_bytes());
                }
                astraweave_ai::htn::ActionSlot::TakeCover { cover_id } => {
                    out.push(3);
                    out.extend_from_slice(&cover_id.to_le_bytes());
                }
                astraweave_ai::htn::ActionSlot::Wait { duration_milli } => {
                    out.push(4);
                    out.extend_from_slice(&duration_milli.to_le_bytes());
                }
            }
        }
    }
    out
}

fn sha256(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

#[test]
fn decision_trace_hash_matches_when_the_ring_is_off() {
    let mut null_sink = NullSink;
    let silent = script_bytes(&mut null_sink);

    let mut ring = RingSink {
        trace: DecisionTrace::new(),
    };
    assert!(ring.trace.bind(1));
    let traced = script_bytes(&mut ring);

    assert_eq!(sha256(&silent), sha256(&traced));
    assert_eq!(silent, traced);
    assert!(ring.trace.count_of(1) > 0);
}

#[test]
fn decision_trace_rerun_leaves_the_script_hash() {
    let mut ring = RingSink {
        trace: DecisionTrace::new(),
    };
    assert!(ring.trace.bind(1));
    let before = script_bytes(&mut ring);
    let digest = sha256(&before);

    let mut tiles = bridge_and_ford();
    let mut scratch = NavScratch::new();
    let record = ring
        .trace
        .find_tick(1, 1)
        .copied()
        .expect("tick 1 was recorded");
    let mut sample = astraweave_ai::decision_trace::TickSample::from_record(record);
    sample.pos_milli = [1_000, 0, 1_000];
    sample.goal_milli = [1_000, 0, 46_000];
    let hooks = SpatialHooks::none();
    let mut view = PlannerView {
        tiles: &tiles,
        scratch: &mut scratch,
        hooks,
    };
    let diff = astraweave_ai::decision_trace::rerun(
        sample,
        Patch::SealCell {
            x: record.perceived.breach_x,
            y: record.perceived.breach_y,
        },
        &mut view,
    );
    let _ = diff;
    let _ = tiles.commit_tick();

    let mut again = RingSink {
        trace: DecisionTrace::new(),
    };
    assert!(again.trace.bind(1));
    let after = script_bytes(&mut again);
    assert_eq!(sha256(&before), digest);
    assert_eq!(sha256(&after), digest);
}
