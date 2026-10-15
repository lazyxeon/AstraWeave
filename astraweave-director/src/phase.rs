use astraweave_core::*;
#[derive(Clone, Debug)]
pub struct PhaseSpec {
    pub name: String,
    pub hp_threshold: i32, // when boss HP <= threshold, switch to next phase
    pub terrain_bias: f32, // 0..1: how much to prefer terrain edits vs spawns
    pub aggression: f32,   // 0..1: how aggressive the plan should be
}
#[derive(Clone, Debug)]
pub struct PhaseState {
    pub idx: usize,
    pub last_switch_t: f32,
    pub telegraph: Option<String>,
}
pub struct PhaseDirector {
    pub phases: Vec<PhaseSpec>,
    pub state: PhaseState,
}
pub struct PhasePlan {
    pub phase_name: String,
    pub telegraphs: Vec<String>,
    pub director: DirectorPlan,
}
impl PhaseDirector {
    pub fn new(phases: Vec<PhaseSpec>) -> Self {
        Self {
            phases,
            state: PhaseState {
                idx: 0,
                last_switch_t: 0.0,
                telegraph: None,
            },
        }
    }
    /// Given a snapshot (boss = enemies[0]) and budget, devise a plan and maybe switch phase.
    pub fn step(&mut self, snap: &WorldSnapshot, budget: &DirectorBudget) -> PhasePlan {
        let mut tele = vec![];
        if let Some(boss) = snap.enemies.first() {
            // phase switch by hp
            while self.state.idx + 1 < self.phases.len()
                && boss.hp <= self.phases[self.state.idx + 1].hp_threshold
            {
                self.state.idx += 1;
                self.state.telegraph = Some(format!(
                    "Boss shifts into phase: {}",
                    self.phases[self.state.idx].name
                ));
                tele.push(self.state.telegraph.clone().expect("telegraph must be set"));
            }
        }
        // craft a plan using simple bias rules
        let phase = &self.phases[self.state.idx];
        let ppos = snap.player.pos;
        let tgt = snap.enemies.first().map(|e| e.pos).unwrap_or(IVec2 {
            x: ppos.x + 6,
            y: ppos.y,
        });
        let mut ops = vec![];
        if phase.terrain_bias > 0.5 && budget.terrain_edits > 0 {
            // prefer fortify/choke
            let xm = (ppos.x + tgt.x) / 2;
            let ym = (ppos.y + tgt.y) / 2;
            ops.push(DirectorOp::Fortify {
                rect: Rect {
                    x0: xm - 1,
                    y0: ym - 1,
                    x1: xm + 1,
                    y1: ym + 1,
                },
            });
            tele.push("The ground trembles—ramparts rise!".into());
        } else {
            if budget.spawns > 0 {
                ops.push(DirectorOp::SpawnWave {
                    archetype: "phase_add".into(),
                    count: 4,
                    origin: IVec2 {
                        x: ppos.x - 2,
                        y: ppos.y + 1,
                    },
                });
                tele.push("A spectral cohort joins the fray!".into());
            }
            if budget.terrain_edits > 0 {
                ops.push(DirectorOp::Collapse {
                    a: ppos,
                    b: IVec2 {
                        x: (ppos.x + tgt.x) / 2,
                        y: (ppos.y + tgt.y) / 2,
                    },
                });
                tele.push("Bridges shatter—paths rerouted!".into());
            }
        }
        PhasePlan {
            phase_name: phase.name.clone(),
            telegraphs: tele,
            director: DirectorPlan { ops },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use astraweave_core::{CompanionState, EnemyState, PlayerState};

    fn test_snapshot() -> WorldSnapshot {
        WorldSnapshot {
            t: 0.0,
            player: PlayerState {
                pos: IVec2 { x: 0, y: 0 },
                hp: 100,
                stance: "melee".into(),
                orders: vec![],
            },
            me: CompanionState {
                pos: IVec2 { x: 1, y: 1 },
                ammo: 10,
                cooldowns: std::collections::BTreeMap::new(),
                morale: 100.0,
            },
            enemies: vec![EnemyState {
                id: 1,
                pos: IVec2 { x: 5, y: 5 },
                hp: 100,
                cover: "none".into(),
                last_seen: 0.0,
            }],
            pois: vec![],
            obstacles: vec![],
            objective: None,
        
            hazards: [astraweave_core::WeaveHazard::EMPTY; astraweave_core::MAX_WEAVE_HAZARDS],
            hazard_count: 0,
        }
    }

    fn test_phases() -> Vec<PhaseSpec> {
        vec![
            PhaseSpec {
                name: "Phase 1".into(),
                hp_threshold: 100,
                terrain_bias: 0.3,
                aggression: 0.5,
            },
            PhaseSpec {
                name: "Phase 2".into(),
                hp_threshold: 50,
                terrain_bias: 0.7,
                aggression: 0.7,
            },
            PhaseSpec {
                name: "Phase 3".into(),
                hp_threshold: 20,
                terrain_bias: 0.9,
                aggression: 1.0,
            },
        ]
    }

    fn full_budget() -> DirectorBudget {
        DirectorBudget {
            spawns: 5,
            terrain_edits: 5,
            traps: 5,
        }
    }

    fn empty_budget() -> DirectorBudget {
        DirectorBudget {
            spawns: 0,
            terrain_edits: 0,
            traps: 0,
        }
    }

    // PhaseSpec tests
    #[test]
    fn test_phase_spec_creation() {
        let spec = PhaseSpec {
            name: "Test Phase".into(),
            hp_threshold: 50,
            terrain_bias: 0.5,
            aggression: 0.8,
        };
        assert_eq!(spec.name, "Test Phase");
        assert_eq!(spec.hp_threshold, 50);
        assert!((spec.terrain_bias - 0.5).abs() < f32::EPSILON);
        assert!((spec.aggression - 0.8).abs() < f32::EPSILON);
    }

    #[test]
    fn test_phase_spec_clone() {
        let spec = PhaseSpec {
            name: "Clone Test".into(),
            hp_threshold: 75,
            terrain_bias: 0.4,
            aggression: 0.6,
        };
        let cloned = spec.clone();
        assert_eq!(cloned.name, spec.name);
        assert_eq!(cloned.hp_threshold, spec.hp_threshold);
    }

    #[test]
    fn test_phase_spec_debug() {
        let spec = PhaseSpec {
            name: "Debug".into(),
            hp_threshold: 10,
            terrain_bias: 0.1,
            aggression: 0.2,
        };
        let debug_str = format!("{:?}", spec);
        assert!(debug_str.contains("PhaseSpec"));
        assert!(debug_str.contains("Debug"));
    }

    // PhaseState tests
    #[test]
    fn test_phase_state_default_values() {
        let state = PhaseState {
            idx: 0,
            last_switch_t: 0.0,
            telegraph: None,
        };
        assert_eq!(state.idx, 0);
        assert!((state.last_switch_t - 0.0).abs() < f32::EPSILON);
        assert!(state.telegraph.is_none());
    }

    #[test]
    fn test_phase_state_with_telegraph() {
        let state = PhaseState {
            idx: 2,
            last_switch_t: 5.5,
            telegraph: Some("Boss enrages!".into()),
        };
        assert_eq!(state.idx, 2);
        assert!(state.telegraph.is_some());
        assert_eq!(state.telegraph.as_ref().unwrap(), "Boss enrages!");
    }

    #[test]
    fn test_phase_state_clone() {
        let state = PhaseState {
            idx: 1,
            last_switch_t: 2.0,
            telegraph: Some("Test".into()),
        };
        let cloned = state.clone();
        assert_eq!(cloned.idx, state.idx);
    }

    // PhasePlan tests
    #[test]
    fn test_phase_plan_empty() {
        let plan = PhasePlan {
            phase_name: "Empty".into(),
            telegraphs: vec![],
            director: DirectorPlan { ops: vec![] },
        };
        assert!(plan.telegraphs.is_empty());
        assert!(plan.director.ops.is_empty());
    }

    #[test]
    fn test_phase_plan_with_content() {
        let plan = PhasePlan {
            phase_name: "Active".into(),
            telegraphs: vec!["Incoming!".into()],
            director: DirectorPlan {
                ops: vec![DirectorOp::SpawnWave {
                    archetype: "minion".into(),
                    count: 3,
                    origin: IVec2 { x: 0, y: 0 },
                }],
            },
        };
        assert_eq!(plan.phase_name, "Active");
        assert_eq!(plan.telegraphs.len(), 1);
        assert_eq!(plan.director.ops.len(), 1);
    }

    // PhaseDirector tests
    #[test]
    fn test_phase_director_creation() {
        let phases = test_phases();
        let director = PhaseDirector::new(phases.clone());
        assert_eq!(director.phases.len(), 3);
        assert_eq!(director.state.idx, 0);
    }

    #[test]
    fn test_phase_director_initial_state() {
        let director = PhaseDirector::new(test_phases());
        assert_eq!(director.state.idx, 0);
        assert!(director.state.telegraph.is_none());
    }

    #[test]
    fn test_phase_director_step_returns_plan() {
        let mut director = PhaseDirector::new(test_phases());
        let snap = test_snapshot();
        let budget = full_budget();
        let plan = director.step(&snap, &budget);
        assert_eq!(plan.phase_name, "Phase 1");
    }

    #[test]
    fn test_phase_transition_by_hp() {
        let mut director = PhaseDirector::new(test_phases());
        let mut snap = test_snapshot();
        snap.enemies[0].hp = 40; // Below phase 2 threshold
        let budget = full_budget();
        let plan = director.step(&snap, &budget);
        assert_eq!(plan.phase_name, "Phase 2");
    }

    #[test]
    fn test_phase_transition_to_phase_3() {
        let mut director = PhaseDirector::new(test_phases());
        let mut snap = test_snapshot();
        snap.enemies[0].hp = 15; // Below phase 3 threshold
        let budget = full_budget();
        let plan = director.step(&snap, &budget);
        assert_eq!(plan.phase_name, "Phase 3");
    }

    #[test]
    fn test_phase_transition_generates_telegraph() {
        let mut director = PhaseDirector::new(test_phases());
        let mut snap = test_snapshot();
        snap.enemies[0].hp = 40;
        let budget = full_budget();
        let plan = director.step(&snap, &budget);
        assert!(plan
            .telegraphs
            .iter()
            .any(|t| t.contains("Boss shifts into phase")));
    }

    #[test]
    fn test_terrain_bias_high_fortifies() {
        let phases = vec![PhaseSpec {
            name: "Fortify Phase".into(),
            hp_threshold: 100,
            terrain_bias: 0.8, // High terrain bias
            aggression: 0.5,
        }];
        let mut director = PhaseDirector::new(phases);
        let snap = test_snapshot();
        let budget = DirectorBudget {
            spawns: 0,
            terrain_edits: 1,
            traps: 0,
        };
        let plan = director.step(&snap, &budget);
        assert!(plan
            .director
            .ops
            .iter()
            .any(|op| matches!(op, DirectorOp::Fortify { .. })));
    }

    #[test]
    fn test_terrain_bias_low_spawns() {
        let phases = vec![PhaseSpec {
            name: "Spawn Phase".into(),
            hp_threshold: 100,
            terrain_bias: 0.3, // Low terrain bias
            aggression: 0.5,
        }];
        let mut director = PhaseDirector::new(phases);
        let snap = test_snapshot();
        let budget = DirectorBudget {
            spawns: 1,
            terrain_edits: 0,
            traps: 0,
        };
        let plan = director.step(&snap, &budget);
        assert!(plan
            .director
            .ops
            .iter()
            .any(|op| matches!(op, DirectorOp::SpawnWave { .. })));
    }

    #[test]
    fn test_no_enemies_uses_default() {
        let mut director = PhaseDirector::new(test_phases());
        let mut snap = test_snapshot();
        snap.enemies.clear();
        let budget = full_budget();
        let plan = director.step(&snap, &budget);
        assert_eq!(plan.phase_name, "Phase 1");
    }

    #[test]
    fn test_empty_budget_no_ops() {
        let mut director = PhaseDirector::new(test_phases());
        let snap = test_snapshot();
        let budget = empty_budget();
        let plan = director.step(&snap, &budget);
        assert!(plan.director.ops.is_empty());
    }

    #[test]
    fn test_phase_stays_same_if_hp_high() {
        let mut director = PhaseDirector::new(test_phases());
        let mut snap = test_snapshot();
        snap.enemies[0].hp = 100;
        let budget = full_budget();
        director.step(&snap, &budget);
        assert_eq!(director.state.idx, 0);
    }

    #[test]
    fn test_multiple_phase_jumps() {
        let mut director = PhaseDirector::new(test_phases());
        let mut snap = test_snapshot();
        snap.enemies[0].hp = 10; // Should jump to phase 3
        let budget = full_budget();
        let plan = director.step(&snap, &budget);
        assert_eq!(plan.phase_name, "Phase 3");
        assert_eq!(director.state.idx, 2);
    }

    #[test]
    fn test_telegraph_messages_for_terrain() {
        let phases = vec![PhaseSpec {
            name: "Terrain".into(),
            hp_threshold: 100,
            terrain_bias: 0.8,
            aggression: 0.5,
        }];
        let mut director = PhaseDirector::new(phases);
        let snap = test_snapshot();
        let budget = DirectorBudget {
            spawns: 0,
            terrain_edits: 1,
            traps: 0,
        };
        let plan = director.step(&snap, &budget);
        assert!(plan.telegraphs.iter().any(|t| t.contains("ramparts")));
    }

    #[test]
    fn test_spawn_wave_telegraph() {
        let phases = vec![PhaseSpec {
            name: "Spawn".into(),
            hp_threshold: 100,
            terrain_bias: 0.3,
            aggression: 0.5,
        }];
        let mut director = PhaseDirector::new(phases);
        let snap = test_snapshot();
        let budget = DirectorBudget {
            spawns: 1,
            terrain_edits: 0,
            traps: 0,
        };
        let plan = director.step(&snap, &budget);
        assert!(plan.telegraphs.iter().any(|t| t.contains("cohort")));
    }

    #[test]
    fn test_collapse_telegraph() {
        let phases = vec![PhaseSpec {
            name: "Collapse".into(),
            hp_threshold: 100,
            terrain_bias: 0.3,
            aggression: 0.5,
        }];
        let mut director = PhaseDirector::new(phases);
        let snap = test_snapshot();
        let budget = DirectorBudget {
            spawns: 0,
            terrain_edits: 1,
            traps: 0,
        };
        let plan = director.step(&snap, &budget);
        assert!(plan.telegraphs.iter().any(|t| t.contains("shatter")));
    }

    #[test]
    fn test_spawn_wave_archetype() {
        let phases = vec![PhaseSpec {
            name: "Test".into(),
            hp_threshold: 100,
            terrain_bias: 0.3,
            aggression: 0.5,
        }];
        let mut director = PhaseDirector::new(phases);
        let snap = test_snapshot();
        let budget = DirectorBudget {
            spawns: 1,
            terrain_edits: 0,
            traps: 0,
        };
        let plan = director.step(&snap, &budget);
        if let Some(DirectorOp::SpawnWave {
            archetype, count, ..
        }) = plan.director.ops.first()
        {
            assert_eq!(archetype, "phase_add");
            assert_eq!(*count, 4);
        } else {
            panic!("Expected SpawnWave");
        }
    }

    #[test]
    fn test_fortify_rect_calculation() {
        let phases = vec![PhaseSpec {
            name: "Fortify".into(),
            hp_threshold: 100,
            terrain_bias: 0.8,
            aggression: 0.5,
        }];
        let mut director = PhaseDirector::new(phases);
        let mut snap = test_snapshot();
        snap.player.pos = IVec2 { x: 0, y: 0 };
        snap.enemies[0].pos = IVec2 { x: 10, y: 0 };
        let budget = DirectorBudget {
            spawns: 0,
            terrain_edits: 1,
            traps: 0,
        };
        let plan = director.step(&snap, &budget);
        if let Some(DirectorOp::Fortify { rect }) = plan.director.ops.first() {
            let xm = 5;
            let ym = 0;
            assert_eq!(rect.x0, xm - 1);
            assert_eq!(rect.y0, ym - 1);
            assert_eq!(rect.x1, xm + 1);
            assert_eq!(rect.y1, ym + 1);
        } else {
            panic!("Expected Fortify");
        }
    }

    #[test]
    fn test_director_state_persistence() {
        let mut director = PhaseDirector::new(test_phases());
        let mut snap = test_snapshot();
        snap.enemies[0].hp = 40;
        let budget = full_budget();
        director.step(&snap, &budget);
        assert_eq!(director.state.idx, 1);
        snap.enemies[0].hp = 45;
        director.step(&snap, &budget);
        assert_eq!(director.state.idx, 1);
    }

    #[test]
    fn test_single_phase() {
        let phases = vec![PhaseSpec {
            name: "Only Phase".into(),
            hp_threshold: 100,
            terrain_bias: 0.5,
            aggression: 0.5,
        }];
        let mut director = PhaseDirector::new(phases);
        let mut snap = test_snapshot();
        snap.enemies[0].hp = 1;
        let budget = full_budget();
        let plan = director.step(&snap, &budget);
        assert_eq!(plan.phase_name, "Only Phase");
    }
}
