use astraweave_core::{DirectorBudget, DirectorOp, DirectorPlan, IVec2, WorldSnapshot};
use tracing::info;

/// Three-stage adaptive encounter flow for the Oathbound Warden.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum WardenPhase {
    Assessment,
    FulcrumShift,
    DirectiveOverride,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum StormChoice {
    Unknown,
    Stabilize,
    Redirect,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum AdaptiveAbility {
    AntiRangedField,
    CounterShockAura,
}

#[derive(Debug, Clone)]
pub struct WardenDirective {
    pub phase: WardenPhase,
    pub arena_modifier: Option<StormChoice>,
    pub adaptive_ability: Option<AdaptiveAbility>,
    pub plan: DirectorPlan,
    pub telegraphs: Vec<String>,
}

pub struct OathboundWardenDirector {
    phase: WardenPhase,
    initial_hp: Option<i32>,
    storm_choice: StormChoice,
    adaptive: Option<AdaptiveAbility>,
    last_anchor_left: bool,
}

impl Default for OathboundWardenDirector {
    fn default() -> Self {
        Self::new()
    }
}

impl OathboundWardenDirector {
    pub fn new() -> Self {
        Self {
            phase: WardenPhase::Assessment,
            initial_hp: None,
            storm_choice: StormChoice::Unknown,
            adaptive: None,
            last_anchor_left: false,
        }
    }

    pub fn step(&mut self, snap: &WorldSnapshot, budget: &DirectorBudget) -> WardenDirective {
        if snap.enemies.is_empty() {
            return WardenDirective {
                phase: self.phase,
                arena_modifier: None,
                adaptive_ability: self.adaptive,
                plan: DirectorPlan { ops: Vec::new() },
                telegraphs: vec![],
            };
        }

        let boss = &snap.enemies[0];
        if self.initial_hp.is_none() {
            self.initial_hp = Some(boss.hp.max(1));
        }

        self.storm_choice = detect_storm_choice(snap).unwrap_or(self.storm_choice);
        self.update_phase(boss.hp);

        if self.phase == WardenPhase::DirectiveOverride && self.adaptive.is_none() {
            self.adaptive = Some(choose_adaptive_ability(&snap.player));
        }

        let mut telegraphs = Vec::new();
        let mut ops = Vec::new();
        let mut arena_modifier = None;

        match self.phase {
            WardenPhase::Assessment => {
                telegraphs.push("Chains tense—the Warden studies your form.".into());
                let midpoint = midpoint(snap.player.pos, boss.pos);
                if budget.terrain_edits > 0 {
                    ops.push(DirectorOp::Fortify {
                        rect: astraweave_core::Rect {
                            x0: midpoint.x - 1,
                            y0: midpoint.y - 1,
                            x1: midpoint.x + 1,
                            y1: midpoint.y + 1,
                        },
                    });
                }
            }
            WardenPhase::FulcrumShift => {
                arena_modifier = Some(self.storm_choice);
                let anchor_target = self.next_anchor_target(&snap.player.pos);
                ops.extend(build_anchor_rupture(anchor_target));
                match self.storm_choice {
                    StormChoice::Stabilize => {
                        telegraphs.push("Storm calms—armor plates seal the Warden.".into());
                        if budget.terrain_edits > 0 {
                            ops.push(DirectorOp::Fortify {
                                rect: centered_rect(boss.pos, 1),
                            });
                        }
                    }
                    StormChoice::Redirect => {
                        telegraphs.push("Motes surge across the arena—vision falters.".into());
                        if budget.terrain_edits > 0 {
                            ops.push(DirectorOp::Collapse {
                                a: boss.pos,
                                b: anchor_target,
                            });
                        }
                    }
                    StormChoice::Unknown => {
                        telegraphs.push("Currents buckle unpredictably.".into());
                    }
                }
            }
            WardenPhase::DirectiveOverride => {
                let ability = self.adaptive.unwrap_or(AdaptiveAbility::CounterShockAura);
                telegraphs.push(format!("Warden adapts—activating {:?}.", ability));
                ops.extend(build_anchor_rupture(
                    self.next_anchor_target(&snap.player.pos),
                ));
                ops.push(DirectorOp::SpawnWave {
                    archetype: "stormbound_wisp".into(),
                    count: 2,
                    origin: snap.player.pos,
                });

                match ability {
                    AdaptiveAbility::AntiRangedField => {
                        if budget.terrain_edits > 0 {
                            ops.push(DirectorOp::Fortify {
                                rect: centered_rect(boss.pos, 2),
                            });
                        }
                    }
                    AdaptiveAbility::CounterShockAura => {
                        ops.push(DirectorOp::Collapse {
                            a: snap.player.pos,
                            b: boss.pos,
                        });
                    }
                }
            }
        }

        let directive = WardenDirective {
            phase: self.phase,
            arena_modifier,
            adaptive_ability: self.adaptive,
            plan: DirectorPlan { ops },
            telegraphs,
        };

        info!(
            target: "veilweaver.director",
            event = "WardenDirective",
            phase = ?directive.phase,
            arena_modifier = ?directive.arena_modifier,
            adaptive = ?directive.adaptive_ability,
            ops = directive.plan.ops.len()
        );

        directive
    }

    fn update_phase(&mut self, current_hp: i32) {
        let Some(initial) = self.initial_hp else {
            return;
        };
        let two_thirds = initial * 2 / 3;
        let one_third = initial / 3;

        match self.phase {
            WardenPhase::Assessment if current_hp <= two_thirds => {
                self.phase = WardenPhase::FulcrumShift;
            }
            WardenPhase::FulcrumShift if current_hp <= one_third => {
                self.phase = WardenPhase::DirectiveOverride;
            }
            _ => {}
        }
    }

    fn next_anchor_target(&mut self, player_pos: &IVec2) -> IVec2 {
        self.last_anchor_left = !self.last_anchor_left;
        if self.last_anchor_left {
            IVec2 {
                x: player_pos.x + 6,
                y: player_pos.y,
            }
        } else {
            IVec2 {
                x: player_pos.x - 6,
                y: player_pos.y,
            }
        }
    }
}

fn detect_storm_choice(snap: &WorldSnapshot) -> Option<StormChoice> {
    if let Some(obj) = &snap.objective {
        if obj.contains("storm:stabilize") || obj.contains("stabilize storm") {
            return Some(StormChoice::Stabilize);
        }
        if obj.contains("storm:redirect") || obj.contains("redirect storm") {
            return Some(StormChoice::Redirect);
        }
    }
    for poi in &snap.pois {
        if poi.k == "storm_route" {
            if poi.pos.x >= 0 {
                return Some(StormChoice::Stabilize);
            } else {
                return Some(StormChoice::Redirect);
            }
        }
    }
    None
}

fn choose_adaptive_ability(player: &astraweave_core::PlayerState) -> AdaptiveAbility {
    // Stance enum has no ranged/marksman variants currently — future-proofing
    let stance_ranged = false; // Stance::Stand/Crouch/Prone are not ranged
    let order_ranged = player
        .orders
        .iter()
        .any(|o| o.to_lowercase().contains("bow") || o.to_lowercase().contains("ranged"));
    if stance_ranged || order_ranged {
        AdaptiveAbility::AntiRangedField
    } else {
        AdaptiveAbility::CounterShockAura
    }
}

fn build_anchor_rupture(target: IVec2) -> Vec<DirectorOp> {
    vec![DirectorOp::Collapse {
        a: IVec2 {
            x: target.x,
            y: target.y,
        },
        b: IVec2 {
            x: target.x,
            y: target.y + 1,
        },
    }]
}

fn midpoint(a: IVec2, b: IVec2) -> IVec2 {
    IVec2 {
        x: (a.x + b.x) / 2,
        y: (a.y + b.y) / 2,
    }
}

fn centered_rect(center: IVec2, radius: i32) -> astraweave_core::Rect {
    astraweave_core::Rect {
        x0: center.x - radius,
        y0: center.y - radius,
        x1: center.x + radius,
        y1: center.y + radius,
    }
}
