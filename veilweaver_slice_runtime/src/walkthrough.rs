//! Walkthrough orchestrator — composes all Phase 2 subsystems into a
//! unified tick-able slice that drives the Z0→Z4 vertical slice progression.
//!
//! The [`SliceOrchestrator`] wires:
//! - **GameLoop** — zone transitions, dialogues, cinematics, storm choice
//! - **PlayerState** — player HP, zone, echoes, thread stability
//! - **CombatEncounter** — Z2 skirmish waves
//! - **TelemetryCollector** — metric accumulation for recap panel
//! - **BossEncounter** (optional, `boss-director` feature) — Z4 boss fight
//! - **CompanionAI** (optional, `ai-companion` feature) — companion planning
//!
//! The orchestrator tracks the current *beat* (narrative checkpoint) and
//! advances automatically as conditions are met, emitting [`WalkthroughEvent`]s
//! at each transition.
//!
//! Pure logic — headless-safe, no rendering.

use crate::boss_hud::BossHealthBar;
use crate::combat::{CombatEncounter, CombatEvent};
use crate::companion_hud::CompanionAffinityMeter;
use crate::decision_ui::{self, DecisionPhase, DecisionRadial};
use crate::game_loop::{GameLoop, GameLoopEvent};
use crate::hud_state::ThreadHud;
use crate::player_state::PlayerState;
use crate::recap_panel::RecapPanel;
use crate::storm_choice::StormChoice;
use crate::telemetry::TelemetryCollector;
use crate::vfx_dispatch::VfxAudioDispatch;
use serde::{Deserialize, Serialize};
use tracing::info;

// ── Walkthrough beats ──────────────────────────────────────────────────────

/// Narrative checkpoints in the vertical slice walkthrough.
///
/// The orchestrator progresses through these linearly (with optional
/// `SideAlcoveVisited` skipped if the player doesn't explore Z2a).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WalkthroughBeat {
    /// Z0 — Tutorial is in progress.
    TutorialBegin,
    /// Z0 — Tutorial complete, ready to leave sanctum.
    TutorialComplete,
    /// Z1 — Player entered Echo Grove (exploration + dialogue).
    EchoGroveEntry,
    /// Z2 — Player entered Fractured Cliffs.
    FracturedCliffEntry,
    /// Z2 — Skirmish encounter is active.
    SkirmishActive,
    /// Z2 — Skirmish encounter cleared.
    SkirmishCleared,
    /// Z2a — Side alcove visited (optional).
    SideAlcoveVisited,
    /// Z3 — Player entered Loom Crossroads.
    LoomCrossroadsEntry,
    /// Z3 — Storm decision has been made.
    StormDecisionMade,
    /// Z4 — Player entered Boss Courtyard.
    BossArenaEntry,
    /// Z4 — Boss fight active.
    BossActive,
    /// Z4 — Boss defeated.
    BossDefeated,
    /// Post-boss debrief / recap.
    Debrief,
    /// Run complete.
    Complete,
}

impl WalkthroughBeat {
    /// Returns `true` if this beat is a progression endpoint.
    #[must_use]
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Complete)
    }
}

impl std::fmt::Display for WalkthroughBeat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            Self::TutorialBegin => "Tutorial Begin",
            Self::TutorialComplete => "Tutorial Complete",
            Self::EchoGroveEntry => "Echo Grove Entry",
            Self::FracturedCliffEntry => "Fractured Cliff Entry",
            Self::SkirmishActive => "Skirmish Active",
            Self::SkirmishCleared => "Skirmish Cleared",
            Self::SideAlcoveVisited => "Side Alcove Visited",
            Self::LoomCrossroadsEntry => "Loom Crossroads Entry",
            Self::StormDecisionMade => "Storm Decision Made",
            Self::BossArenaEntry => "Boss Arena Entry",
            Self::BossActive => "Boss Active",
            Self::BossDefeated => "Boss Defeated",
            Self::Debrief => "Debrief",
            Self::Complete => "Complete",
        };
        write!(f, "{}", label)
    }
}

// ── Walkthrough events ─────────────────────────────────────────────────────

/// High-level events emitted by the walkthrough orchestrator.
///
/// These complement [`GameLoopEvent`] and [`CombatEvent`] — they represent
/// the walkthrough-level narrative progression rather than per-system internals.
#[derive(Debug, Clone, PartialEq)]
pub enum WalkthroughEvent {
    /// Beat progression.
    BeatAdvanced {
        from: WalkthroughBeat,
        to: WalkthroughBeat,
    },
    /// Tutorial was completed.
    TutorialComplete,
    /// Player took damage.
    PlayerDamaged { amount: f32, new_hp: f32 },
    /// Player was healed.
    PlayerHealed { amount: f32, new_hp: f32 },
    /// Player died and respawned.
    PlayerRespawned,
    /// Echoes collected.
    EchoCollected { amount: u32, total: u32 },
    /// An anchor was repaired.
    AnchorRepaired {
        anchor_id: String,
        total_repaired: u32,
    },
    /// Companion produced a plan (only with `ai-companion` feature).
    CompanionPlanProduced { plan_id: String },
    /// Boss fight started.
    BossStarted,
    /// Boss fight ended.
    BossCleared,
    /// Walkthrough run is complete — debrief ready.
    RunComplete { total_time: f32 },
}

impl std::fmt::Display for WalkthroughEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BeatAdvanced { from, to } => write!(f, "Beat: {from} → {to}"),
            Self::TutorialComplete => write!(f, "Tutorial complete"),
            Self::PlayerDamaged { amount, new_hp } => {
                write!(f, "Damaged {amount:.1} (HP: {new_hp:.1})")
            }
            Self::PlayerHealed { amount, new_hp } => {
                write!(f, "Healed {amount:.1} (HP: {new_hp:.1})")
            }
            Self::PlayerRespawned => write!(f, "Player respawned"),
            Self::EchoCollected { amount, total } => {
                write!(f, "Echo +{amount} (total: {total})")
            }
            Self::AnchorRepaired {
                anchor_id,
                total_repaired,
            } => write!(f, "Anchor '{anchor_id}' repaired ({total_repaired} total)"),
            Self::CompanionPlanProduced { plan_id } => {
                write!(f, "Companion plan: {plan_id}")
            }
            Self::BossStarted => write!(f, "Boss started"),
            Self::BossCleared => write!(f, "Boss cleared"),
            Self::RunComplete { total_time } => {
                write!(f, "Run complete ({total_time:.1}s)")
            }
        }
    }
}

// ── Slice orchestrator ─────────────────────────────────────────────────────

/// Top-level orchestrator that composes all subsystems into a single
/// tick-able walkthrough.
pub struct SliceOrchestrator {
    // ── Core subsystems ────────────────────────────────────────────
    pub game_loop: GameLoop,
    pub player: PlayerState,
    pub combat: CombatEncounter,
    pub telemetry: TelemetryCollector,

    // ── Feature-gated subsystems ───────────────────────────────────
    /// Boss encounter controller (requires `boss-director` feature).
    #[cfg(feature = "boss-director")]
    pub boss: crate::boss_encounter::BossEncounter,
    /// Companion AI planner (requires `ai-companion` feature).
    #[cfg(feature = "ai-companion")]
    pub companion: crate::companion_ai::CompanionAI,

    // ── Phase 3: HUD subsystems (headless-safe) ───────────────────
    /// Thread stability bar + echo counter HUD.
    pub thread_hud: ThreadHud,
    /// Boss health bar (always present; only populated during boss fight).
    pub boss_hud: BossHealthBar,
    /// Companion affinity meter.
    pub companion_hud: CompanionAffinityMeter,
    /// Radial decision widget (Z3 storm choice).
    pub decision_radial: DecisionRadial,
    /// Post-run recap panel (finalized on debrief).
    pub recap: RecapPanel,

    // ── Phase 4: VFX & audio dispatch (headless-safe) ─────────────
    /// VFX/audio event dispatcher.
    pub vfx_audio: VfxAudioDispatch,

    // ── Walkthrough state ──────────────────────────────────────────
    pub current_beat: WalkthroughBeat,
    pub tick_count: u64,

    // ── Pending events ─────────────────────────────────────────────
    pending_walkthrough_events: Vec<WalkthroughEvent>,
}

impl std::fmt::Debug for SliceOrchestrator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SliceOrchestrator")
            .field("current_beat", &self.current_beat)
            .field("tick_count", &self.tick_count)
            .field("player_zone", &self.player.zone_name)
            .field("player_hp", &self.player.hp)
            .field("combat_phase", &self.combat.phase())
            .field("decision_phase", &self.decision_radial.phase())
            .field("recap_finalized", &self.recap.is_finalized())
            .finish_non_exhaustive()
    }
}

impl SliceOrchestrator {
    /// Creates a new orchestrator with all subsystems initialised.
    ///
    /// The combat encounter defaults to the Z2 skirmish.
    #[must_use]
    pub fn new() -> Self {
        Self {
            game_loop: GameLoop::new(),
            player: PlayerState::new(),
            combat: CombatEncounter::z2_skirmish(),
            telemetry: TelemetryCollector::new(),

            #[cfg(feature = "boss-director")]
            boss: crate::boss_encounter::BossEncounter::new(),

            #[cfg(feature = "ai-companion")]
            companion: crate::companion_ai::CompanionAI::new(),

            // Phase 3: HUD
            thread_hud: ThreadHud::new(0),
            boss_hud: BossHealthBar::new("Oathbound Warden", 10_000.0),
            companion_hud: CompanionAffinityMeter::new("Aria", 0.0),
            decision_radial: decision_ui::storm_choice_radial(),
            recap: RecapPanel::new(),

            // Phase 4: VFX & Audio
            vfx_audio: VfxAudioDispatch::new(),

            current_beat: WalkthroughBeat::TutorialBegin,
            tick_count: 0,

            pending_walkthrough_events: Vec::new(),
        }
    }

    // ── Main tick ──────────────────────────────────────────────────────

    /// Advances all subsystems by one frame.
    ///
    /// Returns all events produced this tick — game loop events,
    /// combat events, and walkthrough events combined.
    pub fn tick(&mut self, dt: f32) -> TickResult {
        self.tick_count += 1;
        self.pending_walkthrough_events.clear();

        // 1. Advance the game loop (zones, dialogues, cinematics, storm).
        let game_events = self.game_loop.tick(dt);

        // 2. Process game events → update player state.
        self.process_game_events(&game_events);

        // 3. Drain combat events (if combat is active).
        let combat_events = self.combat.drain_events();
        self.process_combat_events(&combat_events);

        // 4. Feed events to telemetry.
        self.telemetry.process_events(&game_events, dt);
        self.feed_combat_telemetry(&combat_events);

        // 5. Feed events to VFX/audio dispatch (Phase 4 wiring).
        self.vfx_audio.process_events(&game_events, dt);
        self.sync_vfx_combat(&combat_events);

        // 6. Sync HUD state from events (Phase 3 wiring).
        self.sync_hud_from_game_events(&game_events);
        self.sync_hud_from_combat_events(&combat_events);

        // 7. Check beat progression.
        self.check_beat_progression(&game_events);

        // 7a. Tick feature-gated AI subsystems (boss director, companion planner).
        self.tick_ai_subsystems(dt);

        // 8. Sync HUD from walkthrough events (needs final walkthrough events).
        let walkthrough_events = std::mem::take(&mut self.pending_walkthrough_events);
        self.sync_hud_from_walkthrough_events(&walkthrough_events);

        // 9. Tick all HUD animations.
        self.thread_hud.tick(dt);
        self.boss_hud.tick(dt);
        self.companion_hud.tick(dt);
        self.decision_radial.tick(dt);
        if self.recap.is_finalized() {
            self.recap.tick(dt);
        }

        TickResult {
            game_events,
            combat_events,
            walkthrough_events,
        }
    }

    // ── Game event processing ──────────────────────────────────────────

    fn process_game_events(&mut self, events: &[GameLoopEvent]) {
        for event in events {
            match event {
                GameLoopEvent::ZoneLoading {
                    zone_name, coord, ..
                } => {
                    self.player.enter_zone(zone_name.clone(), *coord);
                    info!(
                        "Walkthrough: player entered zone '{}' at {:?}",
                        zone_name, coord
                    );
                }
                GameLoopEvent::StormDecisionMade { choice } => {
                    info!("Walkthrough: storm decision = {:?}", choice);
                }
                GameLoopEvent::StormResolved { choice } => {
                    info!("Walkthrough: storm resolved = {:?}", choice);
                    #[cfg(feature = "boss-director")]
                    {
                        let modifiers =
                            crate::storm_choice::BossArenaModifiers::from_choice(*choice);
                        self.boss.start(Some(*choice));
                        info!(
                            "Walkthrough: boss encounter primed (visibility={:.0}%)",
                            modifiers.visibility * 100.0
                        );
                    }
                }
                _ => {}
            }
        }
    }

    // ── Combat event processing ────────────────────────────────────────

    fn process_combat_events(&mut self, events: &[CombatEvent]) {
        for event in events {
            match event {
                CombatEvent::EnemyDefeated { remaining, .. } => {
                    info!("Walkthrough: enemy defeated, {} remaining", remaining);
                }
                CombatEvent::EncounterCleared { encounter_id } => {
                    info!("Walkthrough: encounter '{}' cleared!", encounter_id);
                }
                _ => {}
            }
        }
    }

    fn feed_combat_telemetry(&mut self, events: &[CombatEvent]) {
        for event in events {
            match event {
                CombatEvent::EnemyDefeated { .. } => {
                    self.telemetry.record_enemy_defeated();
                }
                CombatEvent::PlayerDamaged { amount, .. } => {
                    self.telemetry.record_damage_taken(*amount);
                    self.pending_walkthrough_events
                        .push(WalkthroughEvent::PlayerDamaged {
                            amount: *amount,
                            new_hp: self.player.hp,
                        });
                }
                CombatEvent::ComboLanded { .. } => {
                    self.telemetry.record_combo();
                }
                _ => {}
            }
        }
    }

    // ── Phase 3: HUD wiring ───────────────────────────────────────────

    /// Sync HUD state from game-loop events (storm decision radial,
    /// zone transitions for VFX).
    fn sync_hud_from_game_events(&mut self, events: &[GameLoopEvent]) {
        for event in events {
            match event {
                GameLoopEvent::StormDecisionPrompt => {
                    // Open the decision radial when the storm prompt fires.
                    self.decision_radial.open();
                    info!("HUD: decision radial opened");
                }
                GameLoopEvent::StormDecisionMade { choice } => {
                    // Select the matching option in the radial.
                    let index = match choice {
                        StormChoice::Stabilize => 0,
                        StormChoice::Redirect => 1,
                    };
                    if self.decision_radial.phase() == DecisionPhase::Active {
                        self.decision_radial.select(index);
                        info!("HUD: decision radial selected index {}", index);
                    }
                }
                _ => {}
            }
        }
    }

    /// Sync HUD state from combat events (enemy defeat VFX, damage flash).
    fn sync_hud_from_combat_events(&mut self, events: &[CombatEvent]) {
        use crate::vfx_specs::Vec3f;

        for event in events {
            match event {
                CombatEvent::EnemyDefeated { remaining, .. } => {
                    // Emit a small echo burst at a generated position.
                    let pos = Vec3f::new(*remaining as f32 * 2.0, 0.0, 0.0);
                    self.vfx_audio.emit_echo_burst(pos, false);
                }
                CombatEvent::EncounterCleared { .. } => {
                    // Weaving success stinger on encounter clear.
                    self.vfx_audio.weaving_success();
                }
                CombatEvent::PlayerDamaged { .. } => {
                    // VFX: damage feedback handled by presentation layer
                    // reading player HP from thread_hud.
                }
                _ => {}
            }
        }
    }

    /// Sync HUD state from high-level walkthrough events (echoes, anchors,
    /// beat transitions, debrief finalization).
    fn sync_hud_from_walkthrough_events(&mut self, events: &[WalkthroughEvent]) {
        for event in events {
            match event {
                WalkthroughEvent::EchoCollected { amount, total } => {
                    self.thread_hud.echo_transact(*amount as i32);
                    self.thread_hud.echoes.balance = *total;
                    // Echo burst VFX.
                    let large = *amount >= 10;
                    self.vfx_audio
                        .emit_echo_burst(crate::vfx_specs::Vec3f::ZERO, large);
                }
                WalkthroughEvent::AnchorRepaired {
                    anchor_id,
                    total_repaired,
                } => {
                    // Ensure the anchor exists in the HUD, then mark repaired.
                    if !self.thread_hud.anchors.contains_key(anchor_id) {
                        self.thread_hud.add_anchor(anchor_id, 0.5);
                    }
                    self.thread_hud.update_anchor_stability(anchor_id, 1.0);
                    self.thread_hud.set_anchor_repairing(anchor_id, true);
                    self.thread_hud.set_anchor_repairing(anchor_id, false);
                    // VFX: anchor repair effect.
                    self.vfx_audio.begin_anchor_repair(anchor_id);
                    info!(
                        "HUD: anchor '{}' repaired (total: {})",
                        anchor_id, total_repaired
                    );
                }
                WalkthroughEvent::BeatAdvanced { from, to } => {
                    match to {
                        WalkthroughBeat::BossArenaEntry => {
                            self.vfx_audio.enter_boss_encounter();
                            self.telemetry.start_boss_fight();
                            info!("HUD: boss encounter started, VFX primed");
                        }
                        WalkthroughBeat::BossDefeated => {
                            self.vfx_audio.boss_defeated();
                            self.telemetry.finish_boss_fight();
                            info!("HUD: boss defeated, VFX cleared");
                        }
                        WalkthroughBeat::Debrief | WalkthroughBeat::Complete => {
                            if !self.recap.is_finalized() {
                                self.telemetry.finalize(&mut self.recap);
                                info!(
                                    "HUD: recap finalized with {} metrics",
                                    self.recap.metric_count()
                                );
                            }
                        }
                        _ => {}
                    }
                    info!("HUD: beat {} → {}", from, to);
                }
                WalkthroughEvent::PlayerDamaged { new_hp, .. } => {
                    // Update thread HUD echo display to reflect player state.
                    // (HP is not tracked in ThreadHud, but we keep echo balance
                    // in sync.)
                    let _ = new_hp; // consumed by presentation layer via player()
                }
                WalkthroughEvent::RunComplete { .. } => {
                    if !self.recap.is_finalized() {
                        self.telemetry.finalize(&mut self.recap);
                    }
                }
                _ => {}
            }
        }
    }

    // ── Phase 4: VFX/Audio combat wiring ──────────────────────────────

    /// Feed combat events to VFX/audio dispatch for presentation effects.
    ///
    /// Dispatches per-combat-event audio stingers and VFX cues that are
    /// not already handled in [`sync_hud_from_combat_events`].
    fn sync_vfx_combat(&mut self, events: &[CombatEvent]) {
        for event in events {
            match event {
                CombatEvent::EncounterStarted { .. } => {
                    // Kick off encounter ambience change.
                    self.vfx_audio.audio_scene_mut().queue_stinger(
                        crate::audio_specs::UiStingerSpec::new(
                            crate::audio_specs::StingerKind::WeavingFailure,
                        ),
                    );
                }
                CombatEvent::WaveCleared { next_wave, .. } => {
                    if *next_wave {
                        // Mid-encounter wave clear — small weaving pulse.
                        self.vfx_audio.weaving_success();
                    }
                }
                CombatEvent::ComboLanded { combo_hits } => {
                    if *combo_hits >= 5 {
                        // Large combo reward VFX.
                        self.vfx_audio.ability_unlock();
                    }
                }
                // Other events handled in sync_hud_from_combat_events.
                _ => {}
            }
        }
    }

    // ── Feature-gated AI subsystem ticking ─────────────────────────────

    /// Ticks the boss encounter director and companion AI planner (when features enabled).
    ///
    /// Builds a minimal [`WorldSnapshot`] from current orchestrator state
    /// and forwards it to each AI subsystem.
    fn tick_ai_subsystems(&mut self, _dt: f32) {
        // Only tick during combat-relevant beats.
        let in_boss = matches!(
            self.current_beat,
            WalkthroughBeat::BossArenaEntry | WalkthroughBeat::BossActive
        );
        let _ = in_boss; // suppress unused-variable when features are disabled

        #[cfg(feature = "boss-director")]
        if in_boss && self.boss.is_active() {
            let snapshot = self.build_world_snapshot();
            if let Some(directive) = self.boss.tick(&snapshot) {
                // Forward boss telegraphs to HUD + VFX.
                for telegraph in &directive.telegraphs {
                    self.boss_hud
                        .add_telegraph(telegraph.clone(), telegraph.clone(), 2.0);
                    use crate::vfx_specs::{TelegraphVfxSpec, Vec3f};
                    let spec = TelegraphVfxSpec::cleave(Vec3f::ZERO, Vec3f::new(1.0, 0.0, 0.0));
                    self.vfx_audio.add_telegraph(spec);
                }
                // Phase transitions update boss HUD.
                let phase_idx = directive.phase as u32;
                if phase_idx > 0 {
                    let hp_frac = 1.0 - (phase_idx as f32 * 0.33);
                    self.boss_hud
                        .set_hp(self.boss_hud.max_hp * hp_frac.max(0.0));
                }
            }
            // Process boss encounter events.
            let boss_events = self.boss.drain_events();
            for event in &boss_events {
                match event {
                    crate::boss_encounter::BossEncounterEvent::Defeated => {
                        self.pending_walkthrough_events
                            .push(WalkthroughEvent::BossCleared);
                    }
                    crate::boss_encounter::BossEncounterEvent::EncounterStarted { .. } => {
                        self.pending_walkthrough_events
                            .push(WalkthroughEvent::BossStarted);
                    }
                    _ => {}
                }
            }
        }

        #[cfg(feature = "ai-companion")]
        {
            let snapshot = self.build_world_snapshot();
            if let Some(plan) = self.companion.tick(&snapshot) {
                if !plan.steps.is_empty() {
                    self.pending_walkthrough_events
                        .push(WalkthroughEvent::CompanionPlanProduced {
                            plan_id: plan.plan_id.clone(),
                        });
                    self.telemetry.record_companion_action();
                }
            }
        }
    }

    /// Builds a minimal [`WorldSnapshot`] from the current orchestrator state.
    ///
    /// Used by feature-gated AI subsystems (boss director, companion planner).
    #[allow(unused)] // Only used when boss-director or ai-companion features enabled
    fn build_world_snapshot(&self) -> astraweave_core::WorldSnapshot {
        use astraweave_core::{CompanionState, IVec2, Stance, WorldSnapshot};
        use std::collections::BTreeMap;

        let player_pos = IVec2::new(
            self.player.zone.map(|c| c.x).unwrap_or(0),
            self.player.zone.map(|c| c.z).unwrap_or(0),
        );

        WorldSnapshot {
            t: self.telemetry.total_time,
            player: astraweave_core::PlayerState {
                pos: player_pos,
                hp: self.player.hp as i32,
                stance: Stance::Stand,
                orders: vec![],
            },
            me: CompanionState {
                pos: IVec2::new(player_pos.x + 2, player_pos.y),
                ammo: 10,
                cooldowns: BTreeMap::new(),
                morale: self.companion_hud.affinity() as f64 as f32,
            },
            enemies: self.build_enemy_snapshot(),
            pois: vec![],
            obstacles: vec![],
            objective: Some(format!("{}", self.current_beat)),
        }
    }

    /// Builds a minimal enemy list from combat encounter state.
    #[allow(unused)] // Only used when boss-director or ai-companion features enabled
    fn build_enemy_snapshot(&self) -> Vec<astraweave_core::EnemyState> {
        use astraweave_core::{CoverType, EnemyState, IVec2};

        if !self.combat.is_active() {
            return vec![];
        }

        // Build enemy entries from the current wave.
        let wave_idx = self.combat.current_wave;
        if let Some(wave) = self.combat.waves.get(wave_idx) {
            wave.enemies
                .iter()
                .enumerate()
                .filter(|(_, e)| e.alive)
                .map(|(i, e)| EnemyState {
                    id: e.id,
                    pos: IVec2::new(20 + i as i32 * 3, 15),
                    hp: e.hp as i32,
                    cover: CoverType::None,
                    last_seen: 0.0,
                })
                .collect()
        } else {
            vec![]
        }
    }

    // ── Beat progression ───────────────────────────────────────────────

    fn check_beat_progression(&mut self, game_events: &[GameLoopEvent]) {
        // Loop until stable — a single tick can satisfy multiple beat
        // conditions (e.g. tutorial complete + zone entered in the same frame).
        loop {
            let new_beat = self.evaluate_beat(game_events);
            if new_beat == self.current_beat {
                break;
            }
            let old = self.current_beat;
            info!("Walkthrough beat: {} → {}", old, new_beat);
            self.pending_walkthrough_events
                .push(WalkthroughEvent::BeatAdvanced {
                    from: old,
                    to: new_beat,
                });
            self.current_beat = new_beat;
        }
    }

    fn evaluate_beat(&self, game_events: &[GameLoopEvent]) -> WalkthroughBeat {
        use WalkthroughBeat::*;

        // Terminal states are sticky.
        if self.current_beat == Complete || self.current_beat == Debrief {
            return self.current_beat;
        }

        // Check zone-entered events this tick.
        let entered_zone: Option<&str> = game_events.iter().find_map(|e| {
            if let GameLoopEvent::ZoneLoading { zone_name, .. } = e {
                Some(zone_name.as_str())
            } else {
                None
            }
        });

        let storm_resolved = game_events
            .iter()
            .any(|e| matches!(e, GameLoopEvent::StormResolved { .. }));

        let storm_decided = game_events
            .iter()
            .any(|e| matches!(e, GameLoopEvent::StormDecisionMade { .. }));

        match self.current_beat {
            TutorialBegin => {
                if self.player.tutorial_complete {
                    TutorialComplete
                } else if let Some(zone) = entered_zone {
                    if zone.contains("echo_grove") || zone.contains("Z1") {
                        // Skipped tutorial — jump ahead.
                        TutorialComplete
                    } else {
                        TutorialBegin
                    }
                } else {
                    TutorialBegin
                }
            }
            TutorialComplete => {
                if let Some(zone) = entered_zone {
                    if zone.contains("echo_grove") || zone.contains("Z1") {
                        EchoGroveEntry
                    } else {
                        TutorialComplete
                    }
                } else {
                    TutorialComplete
                }
            }
            EchoGroveEntry => {
                if let Some(zone) = entered_zone {
                    if zone.contains("fractured") || zone.contains("Z2") {
                        FracturedCliffEntry
                    } else {
                        EchoGroveEntry
                    }
                } else {
                    EchoGroveEntry
                }
            }
            FracturedCliffEntry => {
                if self.combat.is_active() {
                    SkirmishActive
                } else {
                    FracturedCliffEntry
                }
            }
            SkirmishActive => {
                if self.combat.is_cleared() {
                    SkirmishCleared
                } else {
                    SkirmishActive
                }
            }
            SkirmishCleared => {
                if let Some(zone) = entered_zone {
                    if zone.contains("alcove") || zone.contains("Z2a") {
                        SideAlcoveVisited
                    } else if zone.contains("crossroads") || zone.contains("Z3") {
                        LoomCrossroadsEntry
                    } else {
                        SkirmishCleared
                    }
                } else {
                    SkirmishCleared
                }
            }
            SideAlcoveVisited => {
                if let Some(zone) = entered_zone {
                    if zone.contains("crossroads") || zone.contains("Z3") {
                        LoomCrossroadsEntry
                    } else {
                        SideAlcoveVisited
                    }
                } else {
                    SideAlcoveVisited
                }
            }
            LoomCrossroadsEntry => {
                if storm_decided || storm_resolved {
                    StormDecisionMade
                } else {
                    LoomCrossroadsEntry
                }
            }
            StormDecisionMade => {
                if let Some(zone) = entered_zone {
                    if zone.contains("boss") || zone.contains("courtyard") || zone.contains("Z4") {
                        BossArenaEntry
                    } else {
                        StormDecisionMade
                    }
                } else {
                    StormDecisionMade
                }
            }
            BossArenaEntry => {
                #[cfg(feature = "boss-director")]
                {
                    if self.boss.is_active() {
                        return BossActive;
                    }
                }
                BossArenaEntry
            }
            BossActive => {
                #[cfg(feature = "boss-director")]
                {
                    if self.boss.is_defeated() {
                        return BossDefeated;
                    }
                }
                BossActive
            }
            BossDefeated => Debrief,
            Debrief => Complete,
            Complete => Complete,
        }
    }

    // ── External commands ──────────────────────────────────────────────

    /// Marks the tutorial as complete (from external trigger).
    pub fn complete_tutorial(&mut self) {
        self.player.complete_tutorial();
        self.pending_walkthrough_events
            .push(WalkthroughEvent::TutorialComplete);
    }

    /// Starts the Z2 combat encounter.
    pub fn start_combat(&mut self) {
        self.combat.start();
    }

    /// Deals damage to an enemy in the active encounter.
    pub fn damage_enemy(&mut self, enemy_id: u32, amount: f32) -> bool {
        let killed = self.combat.damage_enemy(enemy_id, amount);
        if killed {
            self.telemetry.record_damage_dealt(amount);
        }
        killed
    }

    /// Fast-forwards the current combat wave (kills all enemies).
    pub fn clear_combat_wave(&mut self) {
        self.combat.clear_current_wave();
    }

    /// Deals damage to the player.
    ///
    /// NaN, infinity, zero, and negative values are silently rejected.
    pub fn damage_player(&mut self, amount: f32) {
        if !amount.is_finite() || amount <= 0.0 {
            return;
        }
        let died = self.player.take_damage(amount);
        self.telemetry.record_damage_taken(amount);
        self.pending_walkthrough_events
            .push(WalkthroughEvent::PlayerDamaged {
                amount,
                new_hp: self.player.hp,
            });
        if died {
            self.telemetry.record_death();
            self.player.respawn();
            self.pending_walkthrough_events
                .push(WalkthroughEvent::PlayerRespawned);
        }
    }

    /// Heals the player.
    pub fn heal_player(&mut self, amount: f32) {
        self.player.heal(amount);
        self.pending_walkthrough_events
            .push(WalkthroughEvent::PlayerHealed {
                amount,
                new_hp: self.player.hp,
            });
    }

    /// Collects echo shards.
    pub fn collect_echoes(&mut self, amount: u32) {
        self.player.collect_echoes(amount);
        self.telemetry.record_echoes_collected(amount);
        // Direct HUD sync (events pushed here are cleared at tick start).
        self.thread_hud
            .echo_transact(amount.min(i32::MAX as u32) as i32);
        self.thread_hud.echoes.balance = self.player.echoes;
        self.pending_walkthrough_events
            .push(WalkthroughEvent::EchoCollected {
                amount,
                total: self.player.echoes,
            });
    }

    /// Repairs an anchor.
    pub fn repair_anchor(&mut self, anchor_id: impl Into<String>) {
        let id: String = anchor_id.into();
        self.player.repair_anchor();
        self.telemetry.record_anchor_repaired();
        // Direct HUD sync.
        if !self.thread_hud.anchors.contains_key(&id) {
            self.thread_hud.add_anchor(&id, 0.5);
        }
        self.thread_hud.update_anchor_stability(&id, 1.0);
        self.thread_hud.set_anchor_repairing(&id, true);
        self.thread_hud.set_anchor_repairing(&id, false);
        // VFX: anchor repair effect.
        self.vfx_audio.begin_anchor_repair(&id);
        self.pending_walkthrough_events
            .push(WalkthroughEvent::AnchorRepaired {
                anchor_id: id,
                total_repaired: self.player.anchors_repaired,
            });
    }

    /// Simulates the storm choice at Z3.
    pub fn make_storm_choice(&mut self, choice: StormChoice) {
        self.game_loop.notify_storm_choice(choice);
    }

    /// Forces beat progression to a specific beat (for testing).
    pub fn force_beat(&mut self, beat: WalkthroughBeat) {
        let old = self.current_beat;
        self.current_beat = beat;
        self.pending_walkthrough_events
            .push(WalkthroughEvent::BeatAdvanced {
                from: old,
                to: beat,
            });
        // Direct HUD sync for beat-driven state changes.
        match beat {
            WalkthroughBeat::BossArenaEntry => {
                self.vfx_audio.enter_boss_encounter();
                self.telemetry.start_boss_fight();
            }
            WalkthroughBeat::BossDefeated => {
                self.vfx_audio.boss_defeated();
                self.telemetry.finish_boss_fight();
            }
            WalkthroughBeat::Debrief | WalkthroughBeat::Complete => {
                if !self.recap.is_finalized() {
                    self.telemetry.finalize(&mut self.recap);
                }
            }
            _ => {}
        }
    }

    /// Advances from BossDefeated → Debrief → Complete.
    pub fn advance_to_debrief(&mut self) {
        if self.current_beat == WalkthroughBeat::BossDefeated
            || self.current_beat == WalkthroughBeat::Debrief
        {
            self.force_beat(WalkthroughBeat::Complete);
            self.pending_walkthrough_events
                .push(WalkthroughEvent::RunComplete {
                    total_time: self.telemetry.total_time,
                });
        }
    }

    // ── Queries ────────────────────────────────────────────────────────

    /// Returns the current beat.
    #[must_use]
    pub fn beat(&self) -> WalkthroughBeat {
        self.current_beat
    }

    /// Returns `true` if the walkthrough is complete.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.current_beat == WalkthroughBeat::Complete
    }

    /// Returns a reference to the player state.
    #[must_use]
    pub fn player(&self) -> &PlayerState {
        &self.player
    }

    /// Returns a reference to the telemetry collector.
    #[must_use]
    pub fn telemetry(&self) -> &TelemetryCollector {
        &self.telemetry
    }

    // ── HUD accessors (Phase 3) ───────────────────────────────────────

    /// Returns a reference to the thread HUD (stability bars + echo counter).
    #[must_use]
    pub fn thread_hud(&self) -> &ThreadHud {
        &self.thread_hud
    }

    /// Returns a reference to the boss health bar.
    #[must_use]
    pub fn boss_hud(&self) -> &BossHealthBar {
        &self.boss_hud
    }

    /// Returns a reference to the companion affinity meter.
    #[must_use]
    pub fn companion_hud(&self) -> &CompanionAffinityMeter {
        &self.companion_hud
    }

    /// Returns a reference to the decision radial (storm choice widget).
    #[must_use]
    pub fn decision_radial(&self) -> &DecisionRadial {
        &self.decision_radial
    }

    /// Returns a reference to the recap panel.
    #[must_use]
    pub fn recap(&self) -> &RecapPanel {
        &self.recap
    }

    // ── VFX/Audio accessors (Phase 4) ─────────────────────────────────

    /// Returns a reference to the VFX/audio dispatch.
    #[must_use]
    pub fn vfx_audio(&self) -> &VfxAudioDispatch {
        &self.vfx_audio
    }

    /// Returns a mutable reference for direct VFX/audio manipulation
    /// (e.g., adding telegraphs, anchors outside of event flow).
    pub fn vfx_audio_mut(&mut self) -> &mut VfxAudioDispatch {
        &mut self.vfx_audio
    }

    // ── Checkpoint ─────────────────────────────────────────────────────

    /// Capture a full serialisable checkpoint of the orchestrator state.
    ///
    /// The checkpoint includes game loop, player, combat, all HUD subsystems,
    /// telemetry, VFX/audio dispatch, and the current walkthrough beat.
    #[must_use]
    pub fn checkpoint(&self, label: impl Into<String>) -> crate::checkpoint::SliceCheckpoint {
        crate::checkpoint::SliceCheckpoint::capture_from_orchestrator(self, label)
    }

    // ── Companion affinity convenience ─────────────────────────────────

    /// Apply a companion affinity event (positive or negative delta).
    ///
    /// Automatically records telemetry and triggers VFX for rank-ups.
    pub fn apply_companion_affinity(
        &mut self,
        reason: impl Into<String>,
        delta: f32,
        game_time: f32,
    ) {
        let rank_change = self.companion_hud.apply_event(reason, delta, game_time);
        self.telemetry.record_companion_action();
        if let Some(new_rank) = rank_change {
            self.vfx_audio.affinity_rank_up();
            self.telemetry.set_final_affinity_rank(new_rank);
            info!(
                "Companion rank up to {} (affinity={:.2})",
                new_rank,
                self.companion_hud.affinity()
            );
        }
    }

    // ── Boss HUD convenience ───────────────────────────────────────────

    /// Apply damage to the boss health bar and sync telemetry.
    pub fn damage_boss(&mut self, amount: f32) {
        self.boss_hud.apply_damage(amount);
        self.telemetry.record_damage_dealt(amount);
        let phase = self.boss_hud.phase;
        self.telemetry.record_boss_phase(phase);
    }

    /// Add a boss telegraph warning (HUD + VFX).
    pub fn add_boss_telegraph(
        &mut self,
        ability_name: impl Into<String>,
        warning_text: impl Into<String>,
        duration: f32,
    ) {
        let name: String = ability_name.into();
        let text: String = warning_text.into();
        self.boss_hud
            .add_telegraph(name.clone(), text.clone(), duration);
        // Also add to VFX dispatch for visual indicators.
        use crate::vfx_specs::{TelegraphVfxSpec, Vec3f};
        let spec = TelegraphVfxSpec::cleave(Vec3f::ZERO, Vec3f::new(1.0, 0.0, 0.0));
        self.vfx_audio.add_telegraph(spec);
    }
}

impl Default for SliceOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tick result ────────────────────────────────────────────────────────────

/// Combined event output from a single orchestrator tick.
#[derive(Debug, Clone)]
pub struct TickResult {
    /// Events from the game loop (zones, dialogues, cinematics, storm).
    pub game_events: Vec<GameLoopEvent>,
    /// Events from the combat system.
    pub combat_events: Vec<CombatEvent>,
    /// High-level walkthrough events.
    pub walkthrough_events: Vec<WalkthroughEvent>,
}

impl TickResult {
    /// Returns `true` if no events were emitted this tick.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.game_events.is_empty()
            && self.combat_events.is_empty()
            && self.walkthrough_events.is_empty()
    }

    /// Returns the total event count across all categories.
    #[must_use]
    pub fn event_count(&self) -> usize {
        self.game_events.len() + self.combat_events.len() + self.walkthrough_events.len()
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::EncounterPhase;

    const DT: f32 = 1.0 / 60.0;

    #[test]
    fn new_orchestrator_starts_at_tutorial() {
        let orch = SliceOrchestrator::new();
        assert_eq!(orch.beat(), WalkthroughBeat::TutorialBegin);
        assert!(!orch.is_complete());
        assert_eq!(orch.tick_count, 0);
    }

    #[test]
    fn empty_tick_produces_no_events() {
        let mut orch = SliceOrchestrator::new();
        let result = orch.tick(1.0 / 60.0);
        assert!(result.is_empty());
        assert_eq!(orch.tick_count, 1);
    }

    #[test]
    fn tutorial_completion_beat_advance() {
        let mut orch = SliceOrchestrator::new();
        orch.complete_tutorial();
        assert!(orch.player.tutorial_complete);

        // Need a zone transition to advance past TutorialComplete.
        orch.game_loop
            .register_trigger_action("enter_z1", "zone.transition:Z1_echo_grove");
        orch.game_loop
            .notify_trigger_enter(vec!["enter_z1".to_string()]);
        let result = orch.tick(1.0 / 60.0);

        // Should have advanced through TutorialComplete → EchoGroveEntry.
        assert_eq!(orch.beat(), WalkthroughBeat::EchoGroveEntry);
        assert!(result
            .walkthrough_events
            .iter()
            .any(|e| matches!(e, WalkthroughEvent::BeatAdvanced { .. })));
    }

    #[test]
    fn combat_lifecycle_integration() {
        let mut orch = SliceOrchestrator::new();
        orch.force_beat(WalkthroughBeat::FracturedCliffEntry);

        orch.start_combat();
        assert_eq!(orch.combat.phase(), EncounterPhase::Active);

        // Tick to advance beat → SkirmishActive.
        let _result = orch.tick(1.0 / 60.0);
        assert_eq!(orch.beat(), WalkthroughBeat::SkirmishActive);

        // Clear all waves.
        orch.clear_combat_wave(); // Wave 1
        orch.clear_combat_wave(); // Wave 2
        assert!(orch.combat.is_cleared());

        // Tick to advance beat → SkirmishCleared.
        let result = orch.tick(1.0 / 60.0);
        assert_eq!(orch.beat(), WalkthroughBeat::SkirmishCleared);
        assert!(result
            .combat_events
            .iter()
            .any(|e| matches!(e, CombatEvent::EncounterCleared { .. })));
    }

    #[test]
    fn player_damage_and_respawn() {
        let mut orch = SliceOrchestrator::new();
        orch.damage_player(90.0);
        assert_eq!(orch.player.hp, 10.0);

        orch.damage_player(20.0); // Kills the player → auto-respawn.
        assert!(orch.player.alive);
        assert_eq!(orch.player.hp, orch.player.max_hp);
        assert_eq!(orch.telemetry.deaths, 1);
    }

    #[test]
    fn echo_collection_telemetry() {
        let mut orch = SliceOrchestrator::new();
        orch.collect_echoes(5);
        orch.collect_echoes(3);
        assert_eq!(orch.player.echoes, 8);
        assert_eq!(orch.telemetry.echoes_collected, 8);
    }

    #[test]
    fn anchor_repair_telemetry() {
        let mut orch = SliceOrchestrator::new();
        orch.repair_anchor("anchor_z0_01");
        orch.repair_anchor("anchor_z0_02");
        assert_eq!(orch.player.anchors_repaired, 2);
        assert_eq!(orch.telemetry.anchors_repaired, 2);
    }

    #[test]
    fn storm_choice_integration() {
        let mut orch = SliceOrchestrator::new();
        orch.force_beat(WalkthroughBeat::LoomCrossroadsEntry);

        // Trigger the storm decision prompt.
        orch.game_loop
            .register_trigger_action("storm_trigger", "decision.open:storm_routing");
        orch.game_loop
            .notify_trigger_enter(vec!["storm_trigger".to_string()]);
        let result = orch.tick(1.0 / 60.0);
        assert!(result
            .game_events
            .iter()
            .any(|e| matches!(e, GameLoopEvent::StormDecisionPrompt)));

        // Make the choice.
        orch.make_storm_choice(StormChoice::Stabilize);
        let result = orch.tick(1.0 / 60.0);
        assert!(result.game_events.iter().any(|e| matches!(
            e,
            GameLoopEvent::StormDecisionMade { choice } if *choice == StormChoice::Stabilize
        )));
        assert_eq!(orch.beat(), WalkthroughBeat::StormDecisionMade);
    }

    #[test]
    fn full_zone_progression_z0_to_z4() {
        let mut orch = SliceOrchestrator::new();

        // Setup triggers for zone transitions.
        orch.game_loop
            .register_trigger_action("exit_z0", "zone.transition:Z1_echo_grove");
        orch.game_loop
            .register_trigger_action("exit_z1", "zone.transition:Z2_fractured_cliffs");
        orch.game_loop
            .register_trigger_action("exit_z2", "zone.transition:Z3_loom_crossroads");
        orch.game_loop
            .register_trigger_action("storm_trigger", "decision.open:storm_routing");
        orch.game_loop
            .register_trigger_action("exit_z3", "zone.transition:Z4_boss_courtyard");

        // Z0 → Z1: Tutorial complete, enter Echo Grove.
        orch.complete_tutorial();
        orch.game_loop
            .notify_trigger_enter(vec!["exit_z0".to_string()]);
        orch.tick(1.0 / 60.0);
        assert_eq!(orch.beat(), WalkthroughBeat::EchoGroveEntry);

        // Z1 → Z2: Enter Fractured Cliffs.
        orch.game_loop
            .notify_trigger_enter(vec!["exit_z1".to_string()]);
        orch.tick(1.0 / 60.0);
        assert_eq!(orch.beat(), WalkthroughBeat::FracturedCliffEntry);

        // Z2: Start & clear combat.
        orch.start_combat();
        orch.tick(1.0 / 60.0);
        assert_eq!(orch.beat(), WalkthroughBeat::SkirmishActive);

        orch.clear_combat_wave();
        orch.clear_combat_wave();
        orch.tick(1.0 / 60.0);
        assert_eq!(orch.beat(), WalkthroughBeat::SkirmishCleared);

        // Z2 → Z3: Enter Loom Crossroads.
        orch.game_loop
            .notify_trigger_enter(vec!["exit_z2".to_string()]);
        orch.tick(1.0 / 60.0);
        assert_eq!(orch.beat(), WalkthroughBeat::LoomCrossroadsEntry);

        // Z3: Storm decision.
        orch.game_loop
            .notify_trigger_enter(vec!["storm_trigger".to_string()]);
        orch.tick(1.0 / 60.0);
        orch.make_storm_choice(StormChoice::Redirect);
        orch.tick(1.0 / 60.0);
        assert_eq!(orch.beat(), WalkthroughBeat::StormDecisionMade);

        // Z3 → Z4: Enter Boss Courtyard.
        orch.game_loop
            .notify_trigger_enter(vec!["exit_z3".to_string()]);
        orch.tick(1.0 / 60.0);
        assert_eq!(orch.beat(), WalkthroughBeat::BossArenaEntry);

        // Simulate boss defeat (without boss-director feature, force it).
        orch.force_beat(WalkthroughBeat::BossDefeated);
        orch.tick(1.0 / 60.0);
        assert_eq!(orch.beat(), WalkthroughBeat::Debrief);

        orch.advance_to_debrief();
        assert!(orch.is_complete());
    }

    #[test]
    fn force_beat_emits_event() {
        let mut orch = SliceOrchestrator::new();
        orch.force_beat(WalkthroughBeat::SkirmishActive);
        assert_eq!(orch.beat(), WalkthroughBeat::SkirmishActive);
    }

    #[test]
    fn tick_count_increments() {
        let mut orch = SliceOrchestrator::new();
        assert_eq!(orch.tick_count, 0);
        orch.tick(1.0 / 60.0);
        orch.tick(1.0 / 60.0);
        orch.tick(1.0 / 60.0);
        assert_eq!(orch.tick_count, 3);
    }

    #[test]
    fn telemetry_accumulates_time() {
        let mut orch = SliceOrchestrator::new();
        orch.tick(1.0);
        orch.tick(0.5);
        orch.tick(0.25);
        assert!((orch.telemetry.total_time - 1.75).abs() < 0.001);
    }

    #[test]
    fn heal_player() {
        let mut orch = SliceOrchestrator::new();
        orch.damage_player(50.0);
        assert_eq!(orch.player.hp, 50.0);
        orch.heal_player(30.0);
        assert_eq!(orch.player.hp, 80.0);
    }

    // ── Phase 3 HUD wiring tests ──────────────────────────────────

    #[test]
    fn echo_collection_updates_thread_hud() {
        let mut orch = SliceOrchestrator::new();
        orch.collect_echoes(10);
        // The walkthrough event is pending; tick to sync HUD.
        orch.tick(DT);
        assert_eq!(orch.thread_hud.echoes.balance, 10);
    }

    #[test]
    fn anchor_repair_updates_thread_hud() {
        let mut orch = SliceOrchestrator::new();
        orch.repair_anchor("z0_core");
        orch.tick(DT);
        // The anchor should now be tracked in the HUD.
        assert!(orch.thread_hud.anchors.contains_key("z0_core"));
        let entry = &orch.thread_hud.anchors["z0_core"];
        assert!(entry.was_repaired);
        assert!((entry.stability - 1.0).abs() < 0.01);
    }

    #[test]
    fn storm_prompt_opens_decision_radial() {
        let mut orch = SliceOrchestrator::new();
        orch.force_beat(WalkthroughBeat::LoomCrossroadsEntry);

        // Trigger the storm prompt.
        orch.game_loop
            .register_trigger_action("storm", "decision.open:storm_routing");
        orch.game_loop
            .notify_trigger_enter(vec!["storm".to_string()]);
        orch.tick(DT);

        // Decision radial should be visible (Opening or Active after anim tick).
        assert!(orch.decision_radial.is_visible());
    }

    #[test]
    fn storm_decision_selects_radial_option() {
        let mut orch = SliceOrchestrator::new();
        orch.force_beat(WalkthroughBeat::LoomCrossroadsEntry);

        // Open the radial.
        orch.game_loop
            .register_trigger_action("storm", "decision.open:storm_routing");
        orch.game_loop
            .notify_trigger_enter(vec!["storm".to_string()]);
        orch.tick(DT);

        // Animate the opening to Active phase.
        for _ in 0..30 {
            orch.tick(DT);
        }
        assert_eq!(
            orch.decision_radial.phase(),
            crate::decision_ui::DecisionPhase::Active
        );

        // Make the choice → should select the radial.
        orch.make_storm_choice(StormChoice::Redirect);
        orch.tick(DT);

        // Radial should now be closing with "redirect" selected.
        assert_eq!(orch.decision_radial.selected_key(), Some("redirect"));
    }

    #[test]
    fn vfx_dispatch_receives_game_loop_events() {
        let mut orch = SliceOrchestrator::new();
        orch.complete_tutorial();

        // Zone transition → VFX dispatch should update zone.
        orch.game_loop
            .register_trigger_action("z1", "zone.transition:Z1_echo_grove");
        orch.game_loop.notify_trigger_enter(vec!["z1".to_string()]);
        orch.tick(DT);

        assert_eq!(orch.vfx_audio.current_zone(), 1);
    }

    #[test]
    fn companion_affinity_wiring() {
        let mut orch = SliceOrchestrator::new();
        assert_eq!(orch.companion_hud.affinity(), 0.0);

        orch.apply_companion_affinity("Protected player", 0.3, 1.0);
        assert!((orch.companion_hud.affinity() - 0.3).abs() < 0.01);
        assert_eq!(orch.telemetry.companion_support_actions, 1);

        // Rank up from Wary → Cautious.
        assert_eq!(
            orch.companion_hud.rank(),
            crate::companion_hud::AffinityRank::Cautious
        );
    }

    #[test]
    fn boss_damage_wiring() {
        let mut orch = SliceOrchestrator::new();
        assert_eq!(orch.boss_hud.current_hp, 10_000.0);

        orch.damage_boss(3000.0);
        assert_eq!(orch.boss_hud.current_hp, 7000.0);
        assert_eq!(orch.telemetry.damage_dealt, 3000.0);
    }

    #[test]
    fn boss_telegraph_adds_to_both_hud_and_vfx() {
        let mut orch = SliceOrchestrator::new();
        orch.add_boss_telegraph("Cleave", "Dodge!", 2.0);

        assert_eq!(orch.boss_hud.telegraphs.len(), 1);
        assert_eq!(orch.vfx_audio.vfx_scene().telegraphs.len(), 1);
    }

    #[test]
    fn debrief_finalizes_recap() {
        let mut orch = SliceOrchestrator::new();

        // Simulate some gameplay.
        orch.collect_echoes(5);
        orch.repair_anchor("a1");
        orch.damage_player(20.0);

        // Force to debrief.
        orch.force_beat(WalkthroughBeat::BossDefeated);
        orch.tick(DT); // BossDefeated → Debrief
        assert_eq!(orch.beat(), WalkthroughBeat::Debrief);

        // The recap should be finalized.
        assert!(orch.recap.is_finalized());
        assert!(orch.recap.metric_count() > 0);
        assert!(orch.recap.overall_rating().is_some());
    }

    #[test]
    fn hud_animations_advance_on_tick() {
        let mut orch = SliceOrchestrator::new();
        // Add an anchor with mismatched display.
        orch.thread_hud.add_anchor("test", 0.0);
        orch.thread_hud.update_anchor_stability("test", 1.0);

        // Tick many frames to animate.
        for _ in 0..120 {
            orch.tick(DT);
        }
        let entry = &orch.thread_hud.anchors["test"];
        assert!((entry.display_value - 1.0).abs() < 0.05);
    }

    #[test]
    fn vfx_audio_zone_ambience_on_transition() {
        let mut orch = SliceOrchestrator::new();
        orch.complete_tutorial();

        orch.game_loop
            .register_trigger_action("z1", "zone.transition:Z1_echo_grove");
        orch.game_loop.notify_trigger_enter(vec!["z1".to_string()]);
        orch.tick(DT);

        // Audio ambience should be set for zone 1.
        assert!(orch.vfx_audio.audio_scene().ambience.is_some());
    }

    #[test]
    fn full_pipeline_hud_integration() {
        let mut orch = SliceOrchestrator::new();

        // Setup the full zone progression.
        orch.game_loop
            .register_trigger_action("z1", "zone.transition:Z1_echo_grove");
        orch.game_loop
            .register_trigger_action("z2", "zone.transition:Z2_fractured_cliffs");
        orch.game_loop
            .register_trigger_action("z3", "zone.transition:Z3_loom_crossroads");
        orch.game_loop
            .register_trigger_action("storm", "decision.open:storm_routing");
        orch.game_loop
            .register_trigger_action("z4", "zone.transition:Z4_boss_courtyard");

        // Z0 → Z1.
        orch.complete_tutorial();
        orch.game_loop.notify_trigger_enter(vec!["z1".to_string()]);
        orch.tick(DT);
        assert_eq!(orch.beat(), WalkthroughBeat::EchoGroveEntry);

        // Collect echoes in Z1 → HUD updates.
        orch.collect_echoes(15);
        orch.repair_anchor("echo_grove_alpha");
        orch.tick(DT);
        assert_eq!(orch.thread_hud.echoes.balance, 15);
        assert!(orch.thread_hud.anchors.contains_key("echo_grove_alpha"));

        // Companion interactions.
        orch.apply_companion_affinity("Good teamwork", 0.25, 5.0);
        assert_eq!(
            orch.companion_hud.rank(),
            crate::companion_hud::AffinityRank::Cautious
        );

        // Z1 → Z2: Combat.
        orch.game_loop.notify_trigger_enter(vec!["z2".to_string()]);
        orch.tick(DT);
        orch.start_combat();
        orch.tick(DT);
        orch.clear_combat_wave();
        orch.clear_combat_wave();
        orch.tick(DT);

        // Z2 → Z3: Storm choice.
        orch.game_loop.notify_trigger_enter(vec!["z3".to_string()]);
        orch.tick(DT);
        orch.game_loop
            .notify_trigger_enter(vec!["storm".to_string()]);
        orch.tick(DT);

        // Decision radial should be visible.
        assert!(orch.decision_radial.is_visible());

        // Animate to Active.
        for _ in 0..30 {
            orch.tick(DT);
        }

        // Make choice.
        orch.make_storm_choice(StormChoice::Stabilize);
        orch.tick(DT);

        // Z3 → Z4.
        orch.game_loop.notify_trigger_enter(vec!["z4".to_string()]);
        orch.tick(DT);
        assert_eq!(orch.beat(), WalkthroughBeat::BossArenaEntry);

        // Boss fight — test HUD sync.
        assert!(orch.vfx_audio.in_boss_encounter());
        orch.damage_boss(5000.0);
        orch.add_boss_telegraph("Cleave", "Dodge!", 2.0);

        // Complete.
        orch.force_beat(WalkthroughBeat::BossDefeated);
        orch.tick(DT);

        // Should be in Debrief with recap finalized.
        assert_eq!(orch.beat(), WalkthroughBeat::Debrief);
        assert!(orch.recap.is_finalized());
    }

    // ── Checkpoint tests ───────────────────────────────────────────────

    #[test]
    fn checkpoint_round_trip() {
        let mut orch = SliceOrchestrator::new();
        orch.collect_echoes(10);
        orch.repair_anchor("test_anchor");
        orch.tick(DT);

        let cp = orch.checkpoint("test-checkpoint");
        let json = cp.save_to_json().expect("should serialize");
        let cp2 =
            crate::checkpoint::SliceCheckpoint::load_from_json(&json).expect("should deserialize");

        assert_eq!(cp, cp2);
    }

    #[test]
    fn checkpoint_captures_walkthrough_beat() {
        let mut orch = SliceOrchestrator::new();
        orch.complete_tutorial();
        orch.tick(DT);

        let cp = orch.checkpoint("after-tutorial");
        assert_eq!(cp.walkthrough_beat, Some(WalkthroughBeat::TutorialComplete));
    }

    #[test]
    fn checkpoint_captures_player_state() {
        let mut orch = SliceOrchestrator::new();
        orch.collect_echoes(25);
        orch.repair_anchor("anchor_a");
        orch.damage_player(30.0);

        let cp = orch.checkpoint("mid-run");
        let player = cp.player.expect("player snapshot should exist");
        assert_eq!(player.echoes, 25);
        assert_eq!(player.anchors_repaired, 1);
        assert!((player.hp - 70.0).abs() < 0.1);
        assert!(player.alive);
    }

    #[test]
    fn checkpoint_captures_boss_hud() {
        let mut orch = SliceOrchestrator::new();
        orch.damage_boss(3000.0);

        let cp = orch.checkpoint("boss-damage");
        let boss = cp.boss_hud.expect("boss hud snapshot should exist");
        assert_eq!(boss.boss_name, "Oathbound Warden");
        assert!((boss.current_hp - 7000.0).abs() < 1.0);
        assert!(!boss.is_defeated);
    }

    #[test]
    fn checkpoint_captures_companion_hud() {
        let mut orch = SliceOrchestrator::new();
        orch.apply_companion_affinity("heal", 0.15, 1.0);

        let cp = orch.checkpoint("affinity-test");
        let comp = cp.companion_hud.expect("companion snapshot should exist");
        assert_eq!(comp.companion_name, "Aria");
        assert!(comp.affinity > 0.1);
    }

    #[test]
    fn checkpoint_captures_decision_state() {
        let mut orch = SliceOrchestrator::new();

        let cp1 = orch.checkpoint("before-decision");
        let d1 = cp1.decision.expect("decision snapshot should exist");
        assert_eq!(d1.phase, "Hidden");

        orch.decision_radial.open();
        let cp2 = orch.checkpoint("opening-decision");
        let d2 = cp2.decision.expect("decision snapshot should exist");
        assert_eq!(d2.phase, "Opening");
    }

    #[test]
    fn checkpoint_captures_recap() {
        let mut orch = SliceOrchestrator::new();

        let cp1 = orch.checkpoint("pre-debrief");
        let r1 = cp1.recap.expect("recap snapshot should exist");
        assert!(!r1.is_finalized);

        orch.force_beat(WalkthroughBeat::Debrief);
        let cp2 = orch.checkpoint("post-debrief");
        let r2 = cp2.recap.expect("recap snapshot should exist");
        assert!(r2.is_finalized);
    }

    #[test]
    fn checkpoint_diff_detects_changes() {
        let mut orch = SliceOrchestrator::new();
        let cp1 = orch.checkpoint("before");
        orch.collect_echoes(50);
        orch.tick(DT);
        let cp2 = orch.checkpoint("after");

        let diff = cp1.diff(&cp2);
        assert!(!diff.is_identical);
        assert!(diff.mismatch_count > 0);
    }

    #[test]
    fn walkthrough_beat_serde_round_trip() {
        let beats = vec![
            WalkthroughBeat::TutorialBegin,
            WalkthroughBeat::EchoGroveEntry,
            WalkthroughBeat::BossArenaEntry,
            WalkthroughBeat::Complete,
        ];
        for beat in beats {
            let json = serde_json::to_string(&beat).unwrap();
            let parsed: WalkthroughBeat = serde_json::from_str(&json).unwrap();
            assert_eq!(beat, parsed);
        }
    }
}
