use astraweave_behavior::goap::{GoapAction, GoapGoal, GoapPlanner, WorldState};
use rand::prelude::*;

#[test]
fn test_planner_fuzz() {
    use crate::test_support::TestValue;
    let mut rng = StdRng::seed_from_u64(42);
    let planner = GoapPlanner::new();

    for i in 0..100 {
        let _ = i;

        // 1. Generate random facts
        let facts = [
            "has_key",
            "door_open",
            "enemy_dead",
            "has_weapon",
            "in_cover",
            "low_health",
        ];
        let mut current_state = WorldState::new();
        for fact in facts {
            if rng.random_bool(0.5) {
                current_state.set(fact, rng.random_bool(0.5));
            }
        }

        // 2. Generate random goal
        let goal_fact = facts.choose(&mut rng).test_unwrap();
        let goal_val = rng.random_bool(0.5);
        let goal = GoapGoal::new(
            "fuzz_goal",
            WorldState::from_facts(&[(*goal_fact, goal_val)]),
        );

        // 3. Generate random actions
        let mut actions = Vec::new();
        for j in 0..10 {
            let mut action = GoapAction::new(format!("action_{}", j));

            // Random preconditions
            if rng.random_bool(0.7) {
                let pre_fact = facts.choose(&mut rng).test_unwrap();
                action = action.with_precondition(pre_fact, rng.random_bool(0.5));
            }

            // Random effects
            let eff_fact = facts.choose(&mut rng).test_unwrap();
            action = action.with_effect(eff_fact, rng.random_bool(0.5));

            actions.push(action);
        }

        // 4. Run planner
        let plan = planner.plan(&current_state, &goal, &actions);

        // 5. Verify plan validity if found
        if let Some(steps) = plan {
            let mut sim_state = current_state.clone();
            for step in &steps {
                assert!(
                    step.can_apply(&sim_state),
                    "Plan step {} precondition failed in fuzz iter {}",
                    step.name,
                    i
                );
                sim_state = step.apply(&sim_state);
            }
            assert!(
                goal.is_satisfied(&sim_state),
                "Plan failed to satisfy goal in fuzz iter {}",
                i
            );
        }
    }
}

#[path = "../src/test_support.rs"]
mod test_support;
