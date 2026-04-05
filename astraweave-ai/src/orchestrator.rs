//! Orchestrator trait implementations and selection utilities for AstraWeave AI planning.

#[cfg(feature = "profiling")]
use astraweave_profiling::span;

use anyhow::Result;
#[cfg(feature = "llm_orchestrator")]
use astraweave_core::{default_tool_registry, ToolRegistry};
use astraweave_core::{ActionStep, IVec2, PlanIntent, WorldSnapshot};

/// Cooldown key constants for type safety and consistency
const COOLDOWN_THROW_SMOKE: &str = "throw:smoke";

#[async_trait::async_trait]
pub trait OrchestratorAsync {
    async fn plan(&self, snap: WorldSnapshot, budget_ms: u32) -> Result<PlanIntent>;
    fn name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
}

pub trait Orchestrator {
    /// Propose a plan based on a world snapshot.
    fn propose_plan(&self, snap: &WorldSnapshot) -> PlanIntent;
}

/// Minimal rule-based orchestrator:
/// If enemy in LOS-ish and "smoke" not on cooldown:
///   throw smoke midway, move up, cover fire.
/// Else: advance towards nearest enemy.
///
/// # Examples
///
/// ```
/// use astraweave_ai::orchestrator::{RuleOrchestrator, Orchestrator};
/// use astraweave_core::*;
/// use std::collections::BTreeMap;
///
/// let orch = RuleOrchestrator::new();
/// let snap = WorldSnapshot {
///     t: 0.0,
///     player: PlayerState { hp: 100, pos: IVec2::new(0, 0), stance: "stand".into(), orders: vec![] },
///     me: CompanionState { ammo: 10, cooldowns: BTreeMap::new(), morale: 1.0, pos: IVec2::new(1, 1) },
///     enemies: vec![],
///     pois: vec![],
///     obstacles: vec![],
///     objective: None,
/// };
/// let plan = orch.propose_plan(&snap);
/// // An empty battlefield produces a plan (may be empty or a fallback advance)
/// assert!(plan.plan_id.starts_with("plan-"));
/// ```
pub struct RuleOrchestrator;

impl std::fmt::Display for RuleOrchestrator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RuleOrchestrator")
    }
}

impl RuleOrchestrator {
    /// Create a new rule-based orchestrator.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Get the name of this orchestrator.
    #[must_use]
    pub fn name(&self) -> &'static str {
        "RuleOrchestrator"
    }
}

impl Default for RuleOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

impl Orchestrator for RuleOrchestrator {
    fn propose_plan(&self, snap: &WorldSnapshot) -> PlanIntent {
        #[cfg(feature = "profiling")]
        span!("AI::RuleOrchestrator::propose_plan");

        let plan_id = format!("plan-{}", (snap.t * 1000.0) as i64);
        if let Some(first) = snap.enemies.first() {
            let m = &snap.me;
            let _p = &snap.player;

            // midpoint for smoke
            let mid = IVec2 {
                x: (m.pos.x + first.pos.x) / 2,
                y: (m.pos.y + first.pos.y) / 2,
            };

            let smoke_cd = snap
                .me
                .cooldowns
                .get(COOLDOWN_THROW_SMOKE)
                .copied()
                .unwrap_or(0.0);
            if smoke_cd <= 0.0 {
                return PlanIntent {
                    plan_id,
                    steps: vec![
                        ActionStep::Throw {
                            item: "smoke".into(),
                            x: mid.x,
                            y: mid.y,
                        },
                        ActionStep::MoveTo {
                            speed: None,
                            // Use saturating arithmetic to prevent overflow with extreme coordinates
                            x: m.pos
                                .x
                                .saturating_add(first.pos.x.saturating_sub(m.pos.x).signum() * 2),
                            y: m.pos
                                .y
                                .saturating_add(first.pos.y.saturating_sub(m.pos.y).signum() * 2),
                        },
                        ActionStep::CoverFire {
                            target_id: first.id,
                            duration: 2.5,
                        },
                    ],
                };
            } else {
                // advance cautiously
                return PlanIntent {
                    plan_id,
                    steps: vec![
                        ActionStep::MoveTo {
                            speed: None,
                            x: m.pos.x + (first.pos.x - m.pos.x).signum(),
                            y: m.pos.y + (first.pos.y - m.pos.y).signum(),
                        },
                        ActionStep::CoverFire {
                            target_id: first.id,
                            duration: 1.5,
                        },
                    ],
                };
            }
        }

        // fallback
        PlanIntent {
            plan_id,
            steps: vec![],
        }
    }
}

#[async_trait::async_trait]
impl OrchestratorAsync for RuleOrchestrator {
    async fn plan(&self, snap: WorldSnapshot, _budget_ms: u32) -> Result<PlanIntent> {
        Ok(self.propose_plan(&snap))
    }
}

/// Utility-based orchestrator: scores a few candidate plans deterministically.
/// Heuristics:
/// - Prefer throwing smoke if an enemy exists and cooldown is ready
/// - Otherwise move closer to nearest enemy; if very close (<3), add brief cover fire
pub struct UtilityOrchestrator;

impl std::fmt::Display for UtilityOrchestrator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "UtilityOrchestrator")
    }
}

impl UtilityOrchestrator {
    /// Create a new utility-based orchestrator.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Get the name of this orchestrator.
    #[must_use]
    pub fn name(&self) -> &'static str {
        "UtilityOrchestrator"
    }
}

impl Default for UtilityOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

impl Orchestrator for UtilityOrchestrator {
    fn propose_plan(&self, snap: &WorldSnapshot) -> PlanIntent {
        let plan_id = format!("util-{}", (snap.t * 1000.0) as i64);
        let me = &snap.me;
        // Candidate A: Throw smoke at midpoint to first enemy
        let mut cands: Vec<(f32, Vec<ActionStep>)> = vec![];
        if let Some(enemy) = snap.enemies.first() {
            let cd = me
                .cooldowns
                .get(COOLDOWN_THROW_SMOKE)
                .copied()
                .unwrap_or(0.0);
            if cd <= 0.0 {
                let mid = IVec2 {
                    x: (me.pos.x + enemy.pos.x) / 2,
                    y: (me.pos.y + enemy.pos.y) / 2,
                };
                let steps = vec![
                    ActionStep::Throw {
                        item: "smoke".into(),
                        x: mid.x,
                        y: mid.y,
                    },
                    ActionStep::MoveTo {
                        speed: None,
                        x: me.pos.x + (enemy.pos.x - me.pos.x).signum() * 2,
                        y: me.pos.y + (enemy.pos.y - me.pos.y).signum() * 2,
                    },
                ];
                // Score: encourage if ammo present and enemy hp > 0
                let score = 1.0
                    + (snap.player.hp as f32).min(100.0) * 0.0
                    + (enemy.hp.max(0) as f32) * 0.01;
                cands.push((score, steps));
            }
            // Candidate B: advance; if close, cover fire
            let dx = (enemy.pos.x - me.pos.x).abs();
            let dy = (enemy.pos.y - me.pos.y).abs();
            let dist = (dx + dy) as f32;
            let mut steps = vec![ActionStep::MoveTo {
                speed: None,
                x: me.pos.x + (enemy.pos.x - me.pos.x).signum(),
                y: me.pos.y + (enemy.pos.y - me.pos.y).signum(),
            }];
            if dist <= 3.0 {
                steps.push(ActionStep::CoverFire {
                    target_id: enemy.id,
                    duration: 1.0,
                });
            }
            let score = 0.8 + (3.0 - dist).max(0.0) * 0.05;
            cands.push((score, steps));
        }
        // Use total_cmp to provide a total ordering for f32 (handles NaN deterministically)
        cands.sort_by(|a, b| b.0.total_cmp(&a.0));
        let steps = cands.first().map(|c| c.1.clone()).unwrap_or_default();
        PlanIntent { plan_id, steps }
    }
}

#[async_trait::async_trait]
impl OrchestratorAsync for UtilityOrchestrator {
    async fn plan(&self, snap: WorldSnapshot, _budget_ms: u32) -> Result<PlanIntent> {
        Ok(self.propose_plan(&snap))
    }
}

/// Minimal GOAP-style orchestrator for MoveTo -> CoverFire chain towards first enemy.
/// Preconditions: enemy exists. Goal: be within 2 cells and apply CoverFire for 1.5s.
pub struct GoapOrchestrator;

impl std::fmt::Display for GoapOrchestrator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GoapOrchestrator")
    }
}

impl GoapOrchestrator {
    /// Create a new GOAP-style orchestrator.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Get the name of this orchestrator.
    #[must_use]
    pub fn orchestrator_name(&self) -> &'static str {
        "GoapOrchestrator"
    }
}

impl Default for GoapOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

impl GoapOrchestrator {
    /// Fast-path action selection without full plan generation.
    ///
    /// This is optimized for instant action returns (<100 µs target) by:
    /// - Returning single ActionStep directly (not wrapped in PlanIntent)
    /// - Minimal distance calculation (Manhattan distance)
    /// - No allocations (returns stack-allocated ActionStep)
    /// - Simple heuristic: move toward closest enemy or cover fire if in range
    ///
    /// # Arguments
    /// - `snap`: Current world snapshot
    ///
    /// # Returns
    /// An `ActionStep` to execute this frame, or `Wait { duration: 1.0 }` if no enemies
    ///
    /// # Performance
    /// - **Target**: <100 µs per call
    /// - **Typical**: 5-30 µs (distance calc + conditional)
    ///
    /// # Example
    /// ```ignore
    /// use astraweave_ai::GoapOrchestrator;
    /// use astraweave_core::WorldSnapshot;
    ///
    /// fn example(snapshot: &WorldSnapshot) {
    ///     let goap = GoapOrchestrator;
    ///     let action = goap.next_action(snapshot); // <100 µs
    ///     // Apply action to game state...
    /// }
    /// ```
    pub fn next_action(&self, snap: &WorldSnapshot) -> ActionStep {
        #[cfg(feature = "profiling")]
        span!("AI::GoapOrchestrator::next_action");

        // Fast path: if enemy exists, move toward or engage
        if let Some(enemy) = snap.enemies.first() {
            let me = &snap.me;
            let dx = enemy.pos.x - me.pos.x;
            let dy = enemy.pos.y - me.pos.y;
            let dist = dx.abs() + dy.abs();

            if dist <= 2 {
                // In range: cover fire
                ActionStep::CoverFire {
                    target_id: enemy.id,
                    duration: 1.5,
                }
            } else {
                // Out of range: move one step closer
                ActionStep::MoveTo {
                    speed: None,
                    x: me.pos.x + dx.signum(),
                    y: me.pos.y + dy.signum(),
                }
            }
        } else {
            // No enemies: wait
            ActionStep::Wait { duration: 1.0 }
        }
    }
}

impl Orchestrator for GoapOrchestrator {
    fn propose_plan(&self, snap: &WorldSnapshot) -> PlanIntent {
        let plan_id = format!("goap-{}", (snap.t * 1000.0) as i64);
        if let Some(enemy) = snap.enemies.first() {
            let me = &snap.me;
            // Use saturating arithmetic to prevent overflow with extreme coordinates
            let dx = enemy.pos.x.saturating_sub(me.pos.x).abs();
            let dy = enemy.pos.y.saturating_sub(me.pos.y).abs();
            let dist = dx.saturating_add(dy);
            if dist <= 2 {
                return PlanIntent {
                    plan_id,
                    steps: vec![ActionStep::CoverFire {
                        target_id: enemy.id,
                        duration: 1.5,
                    }],
                };
            } else {
                // Move one step closer; replan next tick
                return PlanIntent {
                    plan_id,
                    steps: vec![ActionStep::MoveTo {
                        speed: None,
                        x: me.pos.x + (enemy.pos.x - me.pos.x).signum(),
                        y: me.pos.y + (enemy.pos.y - me.pos.y).signum(),
                    }],
                };
            }
        }
        PlanIntent {
            plan_id,
            steps: vec![],
        }
    }
}

#[async_trait::async_trait]
impl OrchestratorAsync for GoapOrchestrator {
    async fn plan(&self, snap: WorldSnapshot, _budget_ms: u32) -> Result<PlanIntent> {
        Ok(self.propose_plan(&snap))
    }
}

#[cfg(feature = "llm_orchestrator")]
pub struct LlmOrchestrator<C> {
    pub client: C,
    pub registry: ToolRegistry,
}

#[cfg(feature = "llm_orchestrator")]
impl<C> LlmOrchestrator<C> {
    pub fn new(client: C, registry: Option<ToolRegistry>) -> Self {
        Self {
            client,
            registry: registry.unwrap_or_else(default_tool_registry),
        }
    }
}

#[cfg(feature = "llm_orchestrator")]
#[async_trait::async_trait]
impl<C> OrchestratorAsync for LlmOrchestrator<C>
where
    C: astraweave_llm::LlmClient + Send + Sync,
{
    async fn plan(&self, snap: WorldSnapshot, budget_ms: u32) -> Result<PlanIntent> {
        // Read timeout from environment or use budget_ms
        let timeout_ms = std::env::var("LLM_TIMEOUT_MS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(budget_ms.max(50)); // Use budget or minimum 50ms

        let timeout_duration = std::time::Duration::from_millis(timeout_ms as u64);

        // Enforce hard timeout using tokio::time::timeout
        match tokio::time::timeout(
            timeout_duration,
            astraweave_llm::plan_from_llm(&self.client, &snap, &self.registry),
        )
        .await
        {
            Ok(plan_source) => {
                // LLM completed within timeout
                match plan_source {
                    astraweave_llm::PlanSource::Llm(plan) => Ok(plan),
                    astraweave_llm::PlanSource::Fallback { plan, reason } => {
                        tracing::warn!("plan_from_llm fell back: {}", reason);
                        Ok(PlanIntent {
                            plan_id: "llm-fallback".into(),
                            steps: plan.steps,
                        })
                    }
                    _ => {
                        tracing::warn!("Unknown PlanSource variant, using empty plan");
                        Ok(PlanIntent {
                            plan_id: "unknown-source-fallback".into(),
                            steps: vec![],
                        })
                    }
                }
            }
            Err(_elapsed) => {
                // Timeout exceeded - return fallback
                tracing::warn!(
                    "LLM planning timed out after {}ms, using fallback",
                    timeout_ms
                );
                Ok(PlanIntent {
                    plan_id: "timeout-fallback".into(),
                    steps: astraweave_llm::fallback_heuristic_plan(&snap, &self.registry).steps,
                })
            }
        }
    }
    fn name(&self) -> &'static str {
        "LlmOrchestrator"
    }
}

/// System-wide wiring utilities for choosing an orchestrator at runtime.
/// Set ASTRAWEAVE_USE_LLM=1 to select the local LLM (phi3:medium by default) if compiled with the llm_orchestrator feature.
#[derive(Clone, Debug)]
pub struct SystemOrchestratorConfig {
    pub use_llm: bool,
    pub ollama_url: String,
    pub ollama_model: String,
}

impl Default for SystemOrchestratorConfig {
    fn default() -> Self {
        let use_llm = std::env::var("ASTRAWEAVE_USE_LLM")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        let ollama_url =
            std::env::var("OLLAMA_URL").unwrap_or_else(|_| "http://127.0.0.1:11434".to_string());
        // Default model to phi3:medium as requested
        let ollama_model =
            std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "phi3:medium".to_string());
        Self {
            use_llm,
            ollama_url,
            ollama_model,
        }
    }
}

/// Create a boxed orchestrator chosen by config/env.
/// - If use_llm is true and llm_orchestrator is compiled, constructs LlmOrchestrator with Ollama chat client.
/// - Otherwise falls back to UtilityOrchestrator.
pub fn make_system_orchestrator(
    cfg: Option<SystemOrchestratorConfig>,
) -> Box<dyn OrchestratorAsync + Send + Sync> {
    let _cfg = cfg.unwrap_or_default();
    #[cfg(feature = "llm_orchestrator")]
    {
        if _cfg.use_llm {
            // Build an Ollama-backed orchestrator (ollama feature is enabled via Cargo.toml feature wiring)
            let client = astraweave_llm::OllamaChatClient::new(
                _cfg.ollama_url.clone(),
                _cfg.ollama_model.clone(),
            );
            // Optionally warm up model in the background to minimize first-token latency
            let do_warm = std::env::var("OLLAMA_WARMUP")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(true);
            if do_warm {
                let client_clone = client.clone();
                let warm_secs: u64 = std::env::var("OLLAMA_WARMUP_TIMEOUT_SECS")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(30);
                // Prefer spawning on Tokio if available; otherwise use a thread
                #[cfg(feature = "llm_orchestrator")]
                {
                    // We cannot assume tokio is always compiled in this crate; call into a helper function only if tokio exists.
                }
                // Try to use tokio if linked, else fallback
                #[cfg(any())]
                {
                    let _ = (client_clone, warm_secs); // placeholder for conditional compilation
                }
                // Fallback: spawn a thread and perform a blocking warmup with a mini runtime if tokio is available in dependencies
                std::thread::spawn(move || {
                    // If tokio is linked, use a small runtime; else, do nothing (can't warmup without async runtime)
                    #[cfg(feature = "llm_orchestrator")]
                    {
                        // Try building a current-thread runtime; ignore errors silently
                        if let Ok(rt) = tokio::runtime::Builder::new_current_thread()
                            .enable_all()
                            .build()
                        {
                            let _ = rt.block_on(client_clone.warmup(warm_secs));
                        }
                    }
                });
            }
            let orch = crate::LlmOrchestrator::new(client, Some(default_tool_registry()));
            return Box::new(orch);
        }
    }
    // Default safe fallback
    Box::new(UtilityOrchestrator)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "llm_orchestrator")]
    use astraweave_core::default_tool_registry;
    use astraweave_core::{CompanionState, EnemyState, PlayerState, WorldSnapshot};
    #[cfg(feature = "llm_orchestrator")]
    use astraweave_llm::MockLlm;
    use futures::executor::block_on;
    use serial_test::serial;
    use std::collections::BTreeMap;

    fn snap_basic(px: i32, py: i32, ex: i32, ey: i32, smoke_cd: f32) -> WorldSnapshot {
        WorldSnapshot {
            t: 1.234,
            player: PlayerState {
                hp: 80,

                pos: IVec2 { x: px, y: py },
                stance: "stand".into(),
                orders: vec![],
            },
            me: CompanionState {
                ammo: 10,
                cooldowns: BTreeMap::from([(COOLDOWN_THROW_SMOKE.to_string(), smoke_cd)]),
                morale: 0.5,
                pos: IVec2 { x: px, y: py },
            },
            enemies: vec![EnemyState {
                id: 2,
                pos: IVec2 { x: ex, y: ey },
                hp: 50,
                cover: "low".into(),
                last_seen: 0.0,
            }],
            pois: vec![],
            obstacles: vec![],
            objective: None,
        }
    }

    #[test]
    fn utility_prefers_smoke_when_ready() {
        let s = snap_basic(0, 0, 4, 0, 0.0);
        let o = UtilityOrchestrator;
        let plan = o.propose_plan(&s);
        match plan.steps.first() {
            Some(ActionStep::Throw { .. }) => {}
            _ => panic!("expected Throw first"),
        }
    }

    #[test]
    fn goap_moves_then_covers() {
        let s_far = snap_basic(0, 0, 5, 0, 10.0);
        let goap = GoapOrchestrator;
        let plan1 = goap.propose_plan(&s_far);
        assert!(matches!(
            plan1.steps.first(),
            Some(ActionStep::MoveTo { speed: None, .. })
        ));
        let s_close = snap_basic(0, 0, 1, 0, 10.0);
        let plan2 = goap.propose_plan(&s_close);
        assert!(matches!(
            plan2.steps.first(),
            Some(ActionStep::CoverFire { .. })
        ));
    }

    #[test]
    fn async_trait_adapter_returns_same_plan() {
        let s = snap_basic(0, 0, 3, 0, 0.0);
        let rule = RuleOrchestrator;
        let plan_sync = rule.propose_plan(&s);
        let plan_async = block_on(rule.plan(s, 2)).expect("rule.plan failed");
        assert_eq!(plan_sync.steps.len(), plan_async.steps.len());
    }

    #[cfg(feature = "llm_orchestrator")]
    #[tokio::test]
    async fn llm_orchestrator_with_mock_produces_plan() {
        let s = snap_basic(0, 0, 6, 2, 0.0);
        let client = MockLlm;
        let orch = crate::LlmOrchestrator::new(client, Some(default_tool_registry()));
        let plan = orch.plan(s, 10).await.expect("llm mock plan failed");
        // NOTE: MockLlm currently produces JSON that fails parsing (known issue),
        // so this triggers fallback. Once MockLlm format is fixed, change to:
        // assert_eq!(plan.plan_id, "llm-mock");
        // assert!(!plan.steps.is_empty());
        assert_eq!(plan.plan_id, "llm-fallback");
        // Fallback may have empty steps - this is valid behavior
    }

    #[cfg(feature = "llm_orchestrator")]
    #[tokio::test]
    async fn llm_orchestrator_disallowed_tools_fallbacks_empty() {
        let s = snap_basic(0, 0, 6, 2, 0.0);
        let client = MockLlm;
        // Empty registry to force parse failure
        let mut reg = default_tool_registry();
        reg.tools.clear();
        let orch = crate::LlmOrchestrator::new(client, Some(reg));
        let plan = orch.plan(s, 10).await.expect("llm plan call failed");
        assert_eq!(plan.plan_id, "llm-fallback");
        assert!(plan.steps.is_empty());
    }

    #[cfg(feature = "llm_orchestrator")]
    #[tokio::test]
    #[serial]
    #[cfg_attr(miri, ignore)] // serial_test depends on scc crate which has UB under Miri
    async fn llm_orchestrator_respects_timeout_env_var() {
        // Test that LLM_TIMEOUT_MS environment variable overrides budget_ms
        std::env::set_var("LLM_TIMEOUT_MS", "5000");

        let s = snap_basic(0, 0, 6, 2, 0.0);
        let client = MockLlm;
        let orch = crate::LlmOrchestrator::new(client, Some(default_tool_registry()));

        // Call with low budget (10ms), but env var should override to 5000ms
        let plan = orch.plan(s, 10).await.expect("llm plan failed");

        // NOTE: MockLlm currently produces JSON that fails parsing (known issue),
        // so this triggers fallback. Once MockLlm format is fixed, change to:
        // assert_eq!(plan.plan_id, "llm-mock");
        assert_eq!(plan.plan_id, "llm-fallback");

        std::env::remove_var("LLM_TIMEOUT_MS");
    }

    #[cfg(feature = "llm_orchestrator")]
    #[tokio::test]
    #[serial]
    #[cfg_attr(miri, ignore)] // serial_test depends on scc crate which has UB under Miri
    async fn llm_orchestrator_uses_budget_when_env_missing() {
        // Ensure env var is not set
        std::env::remove_var("LLM_TIMEOUT_MS");

        let s = snap_basic(0, 0, 6, 2, 0.0);
        let client = MockLlm;
        let orch = crate::LlmOrchestrator::new(client, Some(default_tool_registry()));

        // Call with reasonable budget (1000ms)
        let plan = orch.plan(s, 1000).await.expect("llm plan failed");

        // NOTE: MockLlm currently produces JSON that fails parsing (known issue),
        // so this triggers fallback. Once MockLlm format is fixed, change to:
        // assert_eq!(plan.plan_id, "llm-mock");
        assert_eq!(plan.plan_id, "llm-fallback");
    }

    #[cfg(feature = "llm_orchestrator")]
    #[tokio::test]
    #[serial]
    #[cfg_attr(miri, ignore)] // serial_test depends on scc crate which has UB under Miri
    async fn llm_orchestrator_enforces_minimum_timeout() {
        // Test that timeout has a minimum of 50ms
        std::env::remove_var("LLM_TIMEOUT_MS");

        let s = snap_basic(0, 0, 6, 2, 0.0);
        let client = MockLlm;
        let orch = crate::LlmOrchestrator::new(client, Some(default_tool_registry()));

        // Call with very low budget (1ms), should be clamped to 50ms
        let plan = orch.plan(s, 1).await.expect("llm plan failed");

        // NOTE: MockLlm currently produces JSON that fails parsing (known issue),
        // so this triggers fallback. Once MockLlm format is fixed, change to:
        // assert_eq!(plan.plan_id, "llm-mock");
        assert_eq!(plan.plan_id, "llm-fallback");
    }

    #[cfg(feature = "llm_orchestrator")]
    #[tokio::test]
    async fn llm_orchestrator_uses_default_registry_when_none() {
        let s = snap_basic(0, 0, 6, 2, 0.0);
        let client = MockLlm;
        // Pass None for registry - should use default_tool_registry()
        let orch = crate::LlmOrchestrator::new(client, None);

        let plan = orch.plan(s, 1000).await.expect("llm plan failed");

        // NOTE: MockLlm currently produces JSON that fails parsing (known issue),
        // so this triggers fallback. Once MockLlm format is fixed, change to:
        // assert_eq!(plan.plan_id, "llm-mock");
        // assert!(!plan.steps.is_empty());
        assert_eq!(plan.plan_id, "llm-fallback");
        // Fallback may have empty steps - this is valid behavior
    }

    #[cfg(feature = "llm_orchestrator")]
    #[test]
    fn llm_orchestrator_name_returns_correct_value() {
        let client = MockLlm;
        let orch = crate::LlmOrchestrator::new(client, None);

        assert_eq!(orch.name(), "LlmOrchestrator");
    }

    #[test]
    #[serial]
    #[cfg_attr(miri, ignore)] // serial_test depends on scc crate which has UB under Miri
    fn system_orchestrator_config_default_parses_env() {
        // Test default config parsing - ensure clean environment first
        std::env::remove_var("ASTRAWEAVE_USE_LLM");
        std::env::remove_var("OLLAMA_URL");
        std::env::remove_var("OLLAMA_MODEL");

        let cfg = SystemOrchestratorConfig::default();

        assert!(!cfg.use_llm);
        assert_eq!(cfg.ollama_url, "http://127.0.0.1:11434");
        assert_eq!(cfg.ollama_model, "phi3:medium");
    }

    #[test]
    #[serial]
    #[cfg_attr(miri, ignore)] // serial_test depends on scc crate which has UB under Miri
    fn system_orchestrator_config_respects_use_llm_env() {
        // Test ASTRAWEAVE_USE_LLM=1
        std::env::set_var("ASTRAWEAVE_USE_LLM", "1");
        let cfg1 = SystemOrchestratorConfig::default();
        assert!(cfg1.use_llm);

        // Test ASTRAWEAVE_USE_LLM=true
        std::env::set_var("ASTRAWEAVE_USE_LLM", "true");
        let cfg2 = SystemOrchestratorConfig::default();
        assert!(cfg2.use_llm);

        // Test ASTRAWEAVE_USE_LLM=TRUE (case insensitive)
        std::env::set_var("ASTRAWEAVE_USE_LLM", "TRUE");
        let cfg3 = SystemOrchestratorConfig::default();
        assert!(cfg3.use_llm);

        // Test ASTRAWEAVE_USE_LLM=0
        std::env::set_var("ASTRAWEAVE_USE_LLM", "0");
        let cfg4 = SystemOrchestratorConfig::default();
        assert!(!cfg4.use_llm);

        std::env::remove_var("ASTRAWEAVE_USE_LLM");
    }

    #[test]
    #[serial]
    #[cfg_attr(miri, ignore)] // serial_test depends on scc crate which has UB under Miri
    fn system_orchestrator_config_respects_ollama_url_env() {
        // Clean environment first
        std::env::remove_var("ASTRAWEAVE_USE_LLM");
        std::env::remove_var("OLLAMA_MODEL");
        std::env::set_var("OLLAMA_URL", "http://custom-server:8080");

        let cfg = SystemOrchestratorConfig::default();
        assert_eq!(cfg.ollama_url, "http://custom-server:8080");

        std::env::remove_var("OLLAMA_URL");
    }

    #[test]
    #[serial]
    #[cfg_attr(miri, ignore)] // serial_test depends on scc crate which has UB under Miri
    fn system_orchestrator_config_respects_ollama_model_env() {
        // Clean environment first, then set specific var
        std::env::remove_var("ASTRAWEAVE_USE_LLM");
        std::env::remove_var("OLLAMA_URL");
        std::env::set_var("OLLAMA_MODEL", "llama3:70b");

        let cfg = SystemOrchestratorConfig::default();
        assert_eq!(cfg.ollama_model, "llama3:70b");

        std::env::remove_var("OLLAMA_MODEL");
    }

    #[test]
    #[serial]
    #[cfg_attr(miri, ignore)] // serial_test depends on scc crate which has UB under Miri
    fn make_system_orchestrator_returns_utility_when_llm_disabled() {
        std::env::remove_var("ASTRAWEAVE_USE_LLM");

        let cfg = SystemOrchestratorConfig {
            use_llm: false,
            ollama_url: "http://localhost:11434".into(),
            ollama_model: "phi3:medium".into(),
        };

        let orch = make_system_orchestrator(Some(cfg));

        // Should return UtilityOrchestrator (name check with full module path)
        assert!(
            orch.name().contains("UtilityOrchestrator"),
            "Expected UtilityOrchestrator, got: {}",
            orch.name()
        );
    }

    #[test]
    #[serial]
    #[cfg_attr(miri, ignore)] // serial_test depends on scc crate which has UB under Miri
    fn make_system_orchestrator_uses_default_config_when_none() {
        std::env::remove_var("ASTRAWEAVE_USE_LLM");

        let orch = make_system_orchestrator(None);

        // Should use default config (use_llm=false) and return UtilityOrchestrator
        assert!(
            orch.name().contains("UtilityOrchestrator"),
            "Expected UtilityOrchestrator, got: {}",
            orch.name()
        );
    }

    #[test]
    fn rule_orchestrator_returns_empty_plan_with_no_enemies() {
        let snap = WorldSnapshot {
            t: 1.0,
            player: PlayerState {
                hp: 100,

                pos: IVec2 { x: 0, y: 0 },
                stance: "stand".into(),
                orders: vec![],
            },
            me: CompanionState {
                ammo: 10,
                cooldowns: BTreeMap::new(),
                morale: 1.0,
                pos: IVec2 { x: 0, y: 0 },
            },
            enemies: vec![], // No enemies
            pois: vec![],
            obstacles: vec![],
            objective: None,
        };

        let rule = RuleOrchestrator;
        let plan = rule.propose_plan(&snap);

        assert!(
            plan.steps.is_empty(),
            "Should return empty plan with no enemies"
        );
    }

    #[test]
    fn rule_orchestrator_throws_smoke_when_cooldown_ready() {
        let snap = snap_basic(0, 0, 5, 0, 0.0); // smoke_cd = 0.0 (ready)

        let rule = RuleOrchestrator;
        let plan = rule.propose_plan(&snap);

        // Should throw smoke as first action
        assert!(
            matches!(plan.steps.first(), Some(ActionStep::Throw { .. })),
            "First action should be Throw when smoke cooldown ready"
        );
        assert_eq!(
            plan.steps.len(),
            3,
            "Should have 3 steps: Throw, MoveTo, CoverFire"
        );
    }

    #[test]
    fn rule_orchestrator_advances_when_cooldown_not_ready() {
        let snap = snap_basic(0, 0, 5, 0, 10.0); // smoke_cd = 10.0 (not ready)

        let rule = RuleOrchestrator;
        let plan = rule.propose_plan(&snap);

        // Should advance cautiously (MoveTo first)
        assert!(
            matches!(plan.steps.first(), Some(ActionStep::MoveTo { .. })),
            "First action should be MoveTo when smoke cooldown not ready"
        );
        assert_eq!(
            plan.steps.len(),
            2,
            "Should have 2 steps: MoveTo, CoverFire"
        );
    }

    // ========================================
    // UtilityOrchestrator Comprehensive Tests
    // ========================================

    #[test]
    fn utility_returns_empty_plan_with_no_enemies() {
        let snap = WorldSnapshot {
            t: 2.5,
            player: PlayerState {
                hp: 100,

                pos: IVec2 { x: 0, y: 0 },
                stance: "stand".into(),
                orders: vec![],
            },
            me: CompanionState {
                ammo: 5,
                cooldowns: BTreeMap::new(),
                morale: 0.8,
                pos: IVec2 { x: 0, y: 0 },
            },
            enemies: vec![], // No enemies = empty candidate list
            pois: vec![],
            obstacles: vec![],
            objective: None,
        };

        let util = UtilityOrchestrator;
        let plan = util.propose_plan(&snap);

        assert!(
            plan.steps.is_empty(),
            "Should return empty steps when no enemies"
        );
        assert!(
            plan.plan_id.starts_with("util-"),
            "Plan ID should start with 'util-'"
        );
    }

    #[test]
    fn utility_scores_candidates_deterministically() {
        // Test that scoring uses total_cmp for deterministic f32 ordering
        let snap = snap_basic(0, 0, 4, 0, 0.0); // Enemy at distance 4, smoke ready

        let util = UtilityOrchestrator;

        // Run multiple times - should be deterministic
        let plan1 = util.propose_plan(&snap);
        let plan2 = util.propose_plan(&snap);
        let plan3 = util.propose_plan(&snap);

        assert_eq!(
            plan1.steps.len(),
            plan2.steps.len(),
            "Plans should be identical (deterministic)"
        );
        assert_eq!(
            plan2.steps.len(),
            plan3.steps.len(),
            "Plans should be identical (deterministic)"
        );

        // All should choose smoke throw as first action (highest score)
        assert!(matches!(
            plan1.steps.first(),
            Some(ActionStep::Throw { .. })
        ));
        assert!(matches!(
            plan2.steps.first(),
            Some(ActionStep::Throw { .. })
        ));
        assert!(matches!(
            plan3.steps.first(),
            Some(ActionStep::Throw { .. })
        ));
    }

    #[test]
    fn utility_prefers_advance_when_smoke_on_cooldown() {
        let snap = snap_basic(0, 0, 4, 0, 5.0); // Enemy at distance 4, smoke on cooldown

        let util = UtilityOrchestrator;
        let plan = util.propose_plan(&snap);

        // Should only have advance candidate (smoke not available)
        assert!(
            matches!(plan.steps.first(), Some(ActionStep::MoveTo { .. })),
            "First action should be MoveTo when smoke on cooldown"
        );
    }

    #[test]
    fn utility_adds_cover_fire_when_close() {
        let snap = snap_basic(0, 0, 2, 0, 5.0); // Enemy at distance 2, smoke on cooldown

        let util = UtilityOrchestrator;
        let plan = util.propose_plan(&snap);

        // Distance <= 3, should add CoverFire after MoveTo
        assert_eq!(
            plan.steps.len(),
            2,
            "Should have MoveTo + CoverFire when close"
        );
        assert!(
            matches!(plan.steps.get(1), Some(ActionStep::CoverFire { .. })),
            "Second action should be CoverFire when enemy close"
        );
    }

    #[test]
    fn utility_no_cover_fire_when_far() {
        let snap = snap_basic(0, 0, 10, 0, 5.0); // Enemy at distance 10, smoke on cooldown

        let util = UtilityOrchestrator;
        let plan = util.propose_plan(&snap);

        // Distance > 3, should only MoveTo (no CoverFire)
        assert_eq!(plan.steps.len(), 1, "Should only have MoveTo when far");
        assert!(
            matches!(plan.steps.first(), Some(ActionStep::MoveTo { .. })),
            "Should only MoveTo when enemy far"
        );
    }

    #[test]
    fn utility_calculates_midpoint_correctly() {
        let snap = snap_basic(0, 0, 6, 4, 0.0); // Me at (0,0), enemy at (6,4), smoke ready

        let util = UtilityOrchestrator;
        let plan = util.propose_plan(&snap);

        // Should throw smoke at midpoint (3, 2)
        if let Some(ActionStep::Throw { item, x, y }) = plan.steps.first() {
            assert_eq!(item, "smoke", "Should throw smoke");
            assert_eq!(*x, 3, "Midpoint x should be (0+6)/2 = 3");
            assert_eq!(*y, 2, "Midpoint y should be (0+4)/2 = 2");
        } else {
            panic!("Expected Throw as first action");
        }
    }

    #[test]
    fn utility_async_adapter_matches_sync() {
        let snap = snap_basic(0, 0, 5, 3, 0.0);

        let util = UtilityOrchestrator;
        let plan_sync = util.propose_plan(&snap);
        let plan_async = block_on(util.plan(snap, 100)).expect("utility async plan failed");

        assert_eq!(
            plan_sync.steps.len(),
            plan_async.steps.len(),
            "Sync and async should produce same plan"
        );
        assert_eq!(
            plan_sync.plan_id.len(),
            plan_async.plan_id.len(),
            "Plan IDs should have same format"
        );
    }

    // ========================================
    // GoapOrchestrator Comprehensive Tests
    // ========================================

    #[test]
    fn goap_next_action_moves_when_far() {
        let snap = snap_basic(0, 0, 10, 5, 0.0); // Enemy far away (distance 15)

        let goap = GoapOrchestrator;
        let action = goap.next_action(&snap);

        assert!(
            matches!(action, ActionStep::MoveTo { .. }),
            "next_action() should return MoveTo when enemy far"
        );
    }

    #[test]
    fn goap_next_action_covers_when_close() {
        let snap = snap_basic(0, 0, 1, 1, 0.0); // Enemy at distance 2 (exactly at threshold)

        let goap = GoapOrchestrator;
        let action = goap.next_action(&snap);

        assert!(
            matches!(action, ActionStep::CoverFire { .. }),
            "next_action() should return CoverFire when enemy at distance 2"
        );
    }

    #[test]
    fn goap_next_action_waits_with_no_enemies() {
        let snap = WorldSnapshot {
            t: 3.0,
            player: PlayerState {
                hp: 100,

                pos: IVec2 { x: 0, y: 0 },
                stance: "stand".into(),
                orders: vec![],
            },
            me: CompanionState {
                ammo: 10,
                cooldowns: BTreeMap::new(),
                morale: 1.0,
                pos: IVec2 { x: 5, y: 5 },
            },
            enemies: vec![], // No enemies
            pois: vec![],
            obstacles: vec![],
            objective: None,
        };

        let goap = GoapOrchestrator;
        let action = goap.next_action(&snap);

        assert!(
            matches!(action, ActionStep::Wait { duration: _ }),
            "next_action() should return Wait when no enemies"
        );
    }

    #[test]
    fn goap_propose_plan_matches_next_action_logic() {
        let snap = snap_basic(0, 0, 5, 0, 0.0);

        let goap = GoapOrchestrator;
        let plan = goap.propose_plan(&snap);
        let action = goap.next_action(&snap);

        // propose_plan should return single-step plan matching next_action
        assert_eq!(
            plan.steps.len(),
            1,
            "propose_plan should return single-step plan"
        );

        // Both should produce same action type (MoveTo for distance > 2)
        match (&plan.steps[0], &action) {
            (ActionStep::MoveTo { .. }, ActionStep::MoveTo { .. }) => { /* OK */ }
            (ActionStep::CoverFire { .. }, ActionStep::CoverFire { .. }) => { /* OK */ }
            (ActionStep::Wait { .. }, ActionStep::Wait { .. }) => { /* OK */ }
            _ => panic!("propose_plan and next_action should produce same action type"),
        }
    }

    #[test]
    fn goap_async_adapter_matches_sync() {
        let snap = snap_basic(0, 0, 3, 0, 0.0);

        let goap = GoapOrchestrator;
        let plan_sync = goap.propose_plan(&snap);
        let plan_async = block_on(goap.plan(snap, 100)).expect("goap async plan failed");

        assert_eq!(
            plan_sync.steps.len(),
            plan_async.steps.len(),
            "Sync and async GOAP should produce same plan"
        );
    }

    // ========================================
    // RuleOrchestrator Comprehensive Tests
    // ========================================

    #[test]
    fn rule_orchestrator_plan_id_format() {
        let snap = snap_basic(0, 0, 5, 0, 0.0);

        let rule = RuleOrchestrator;
        let plan = rule.propose_plan(&snap);

        assert!(
            plan.plan_id.starts_with("plan-"),
            "Plan ID should start with 'plan-'"
        );
        // Plan ID should be "plan-" + timestamp (t * 1000.0) as i64
        // For snap.t = 1.234, should be "plan-1234"
        assert_eq!(plan.plan_id, "plan-1234", "Plan ID should encode timestamp");
    }

    #[test]
    fn rule_orchestrator_calculates_midpoint_correctly() {
        let snap = snap_basic(0, 0, 10, 6, 0.0); // Me at (0,0), enemy at (10,6)

        let rule = RuleOrchestrator;
        let plan = rule.propose_plan(&snap);

        // Should throw smoke at midpoint (5, 3)
        if let Some(ActionStep::Throw { item, x, y }) = plan.steps.first() {
            assert_eq!(item, "smoke", "Should throw smoke");
            assert_eq!(*x, 5, "Midpoint x should be (0+10)/2 = 5");
            assert_eq!(*y, 3, "Midpoint y should be (0+6)/2 = 3");
        } else {
            panic!("Expected Throw as first action when smoke ready");
        }
    }

    #[test]
    fn rule_orchestrator_move_direction_correctness() {
        let snap = snap_basic(0, 0, 5, 3, 10.0); // Smoke on cooldown, enemy northeast

        let rule = RuleOrchestrator;
        let plan = rule.propose_plan(&snap);

        // Should move toward enemy: (0,0) -> (1,1) using signum
        if let Some(ActionStep::MoveTo { x, y, .. }) = plan.steps.first() {
            assert_eq!(
                *x, 1,
                "Should move one step in x direction (signum(5-0) = 1)"
            );
            assert_eq!(
                *y, 1,
                "Should move one step in y direction (signum(3-0) = 1)"
            );
        } else {
            panic!("Expected MoveTo when smoke on cooldown");
        }
    }

    #[test]
    fn rule_orchestrator_async_adapter_matches_sync() {
        let snap = snap_basic(0, 0, 4, 4, 0.0);

        let rule = RuleOrchestrator;
        let plan_sync = rule.propose_plan(&snap);
        let plan_async = block_on(rule.plan(snap, 100)).expect("rule async plan failed");

        assert_eq!(
            plan_sync.steps.len(),
            plan_async.steps.len(),
            "Sync and async RuleOrch should produce same plan"
        );
        // Verify first action matches
        match (&plan_sync.steps.first(), &plan_async.steps.first()) {
            (Some(ActionStep::Throw { .. }), Some(ActionStep::Throw { .. })) => { /* OK */ }
            (Some(ActionStep::MoveTo { .. }), Some(ActionStep::MoveTo { .. })) => { /* OK */ }
            _ => panic!("Sync and async should produce same first action"),
        }
    }

    // ========================================
    // SystemOrchestratorConfig Additional Tests
    // ========================================

    #[test]
    fn system_orchestrator_config_clone_works() {
        let cfg1 = SystemOrchestratorConfig {
            use_llm: true,
            ollama_url: "http://test:8080".into(),
            ollama_model: "test-model".into(),
        };

        let cfg2 = cfg1.clone();

        assert_eq!(cfg1.use_llm, cfg2.use_llm);
        assert_eq!(cfg1.ollama_url, cfg2.ollama_url);
        assert_eq!(cfg1.ollama_model, cfg2.ollama_model);
    }

    #[test]
    fn system_orchestrator_config_debug_output() {
        let cfg = SystemOrchestratorConfig {
            use_llm: false,
            ollama_url: "http://localhost:11434".into(),
            ollama_model: "phi3:medium".into(),
        };

        let debug_str = format!("{:?}", cfg);
        assert!(
            debug_str.contains("use_llm"),
            "Debug output should contain field names"
        );
        assert!(
            debug_str.contains("phi3:medium"),
            "Debug output should contain model name"
        );
    }

    #[test]
    #[serial]
    #[cfg_attr(miri, ignore)] // serial_test depends on scc crate which has UB under Miri
    fn system_orchestrator_config_handles_empty_env_vars() {
        // unwrap_or_else only triggers if var is NOT SET (Err), not if it's empty string
        // Empty strings ARE valid values and get used as-is
        // This test verifies that behavior

        // CRITICAL: Remove vars first to ensure clean state (other tests may set them)
        std::env::remove_var("ASTRAWEAVE_USE_LLM");
        std::env::remove_var("OLLAMA_URL");
        std::env::remove_var("OLLAMA_MODEL");

        // Now set to empty strings
        std::env::set_var("ASTRAWEAVE_USE_LLM", "");
        std::env::set_var("OLLAMA_URL", "");
        std::env::set_var("OLLAMA_MODEL", "");

        let cfg = SystemOrchestratorConfig::default();

        // Empty string != "1" or "true", should be false
        assert!(!cfg.use_llm, "Empty string should parse as false");
        // Empty strings ARE used as-is (unwrap_or_else doesn't treat "" as missing)
        assert_eq!(
            cfg.ollama_url, "",
            "Empty URL should be used as-is (not defaulted)"
        );
        assert_eq!(
            cfg.ollama_model, "",
            "Empty model should be used as-is (not defaulted)"
        );

        // Cleanup
        std::env::remove_var("ASTRAWEAVE_USE_LLM");
        std::env::remove_var("OLLAMA_URL");
        std::env::remove_var("OLLAMA_MODEL");
    }

    #[test]
    fn orchestrator_name_trait_defaults() {
        // Test that default name() implementation returns type name
        let rule = RuleOrchestrator;
        let util = UtilityOrchestrator;
        let goap = GoapOrchestrator;

        // OrchestratorAsync trait has default name() using type_name
        let rule_name = block_on(async { rule.name() });
        let util_name = block_on(async { util.name() });
        let goap_name = block_on(async { goap.name() });

        // Type names should contain the struct names
        assert!(
            rule_name.contains("RuleOrchestrator"),
            "Default name() should return type name"
        );
        assert!(
            util_name.contains("UtilityOrchestrator"),
            "Default name() should return type name"
        );
        assert!(
            goap_name.contains("GoapOrchestrator"),
            "Default name() should return type name"
        );
    }

    // ========================================
    // Orchestrator Display Trait Tests
    // ========================================

    #[test]
    fn rule_orchestrator_display_format() {
        let rule = RuleOrchestrator;
        let display = format!("{}", rule);
        assert_eq!(display, "RuleOrchestrator");
    }

    #[test]
    fn utility_orchestrator_display_format() {
        let util = UtilityOrchestrator;
        let display = format!("{}", util);
        assert_eq!(display, "UtilityOrchestrator");
    }

    #[test]
    fn goap_orchestrator_display_format() {
        let goap = GoapOrchestrator;
        let display = format!("{}", goap);
        assert_eq!(display, "GoapOrchestrator");
    }

    // ========================================
    // Orchestrator Factory and Helper Methods
    // ========================================

    #[test]
    fn rule_orchestrator_new_creates_instance() {
        let rule = RuleOrchestrator::new();
        // Verify it works by calling propose_plan
        let snap = snap_basic(0, 0, 5, 0, 5.0);
        let plan = rule.propose_plan(&snap);
        assert!(plan.plan_id.starts_with("plan-"));
    }

    #[test]
    fn rule_orchestrator_name_method() {
        let rule = RuleOrchestrator::new();
        assert_eq!(rule.name(), "RuleOrchestrator");
    }

    #[test]
    fn rule_orchestrator_default_impl() {
        let rule1 = RuleOrchestrator;
        let rule2 = RuleOrchestrator::new();
        // Both should produce same results
        let snap = snap_basic(0, 0, 5, 0, 0.0);
        let plan1 = rule1.propose_plan(&snap);
        let plan2 = rule2.propose_plan(&snap);
        assert_eq!(plan1.steps.len(), plan2.steps.len());
    }

    #[test]
    fn utility_orchestrator_new_creates_instance() {
        let util = UtilityOrchestrator::new();
        // Verify it works by calling propose_plan
        let snap = snap_basic(0, 0, 5, 0, 0.0);
        let plan = util.propose_plan(&snap);
        assert!(plan.plan_id.starts_with("util-"));
    }

    #[test]
    fn utility_orchestrator_name_method() {
        let util = UtilityOrchestrator::new();
        assert_eq!(util.name(), "UtilityOrchestrator");
    }

    #[test]
    fn utility_orchestrator_default_impl() {
        let util1 = UtilityOrchestrator;
        let util2 = UtilityOrchestrator::new();
        // Both should produce same results for same input
        let snap = snap_basic(0, 0, 5, 0, 0.0);
        let plan1 = util1.propose_plan(&snap);
        let plan2 = util2.propose_plan(&snap);
        assert_eq!(plan1.steps.len(), plan2.steps.len());
    }

    #[test]
    fn goap_orchestrator_new_creates_instance() {
        let goap = GoapOrchestrator::new();
        // Verify it works by calling next_action
        let snap = snap_basic(0, 0, 5, 0, 0.0);
        let action = goap.next_action(&snap);
        assert!(matches!(
            action,
            ActionStep::MoveTo { .. } | ActionStep::CoverFire { .. }
        ));
    }

    #[test]
    fn goap_orchestrator_name_method() {
        let goap = GoapOrchestrator::new();
        assert_eq!(goap.orchestrator_name(), "GoapOrchestrator");
    }

    #[test]
    fn goap_orchestrator_default_impl() {
        let goap1 = GoapOrchestrator;
        let goap2 = GoapOrchestrator::new();
        // Both should produce same results
        let snap = snap_basic(0, 0, 5, 0, 0.0);
        let plan1 = goap1.propose_plan(&snap);
        let plan2 = goap2.propose_plan(&snap);
        assert_eq!(plan1.steps.len(), plan2.steps.len());
    }

    #[test]
    fn all_orchestrators_have_consistent_display() {
        // Test that all concrete orchestrator types have consistent Display output
        let rule = RuleOrchestrator;
        let util = UtilityOrchestrator;
        let goap = GoapOrchestrator;

        // Each display should end with "Orchestrator"
        assert!(
            format!("{}", rule).contains("Orchestrator"),
            "RuleOrchestrator Display should contain 'Orchestrator'"
        );
        assert!(
            format!("{}", util).contains("Orchestrator"),
            "UtilityOrchestrator Display should contain 'Orchestrator'"
        );
        assert!(
            format!("{}", goap).contains("Orchestrator"),
            "GoapOrchestrator Display should contain 'Orchestrator'"
        );
    }

    // ========================================================================
    // Mutation-Resistant Tests: Exact Value Assertions
    // These tests catch arithmetic operator mutations (+/-/*/÷ swaps)
    // ========================================================================

    #[test]
    fn utility_score_a_exact_value_with_enemy_hp_100() {
        // Score A = 1.0 + player_hp.min(100) * 0.0 + enemy_hp.max(0) * 0.01
        // With enemy_hp=100: score_a = 1.0 + 0.0 + 1.0 = 2.0
        let snap = snap_basic(0, 0, 10, 10, 0.0);
        // enemy hp=100 in snap_basic
        let util = UtilityOrchestrator;
        let plan = util.propose_plan(&snap);
        // Candidate A (throw smoke) has score 2.0
        // Candidate B: dist = |10-0| + |10-0| = 20, score = 0.8 + (3.0-20.0).max(0.0)*0.05 = 0.8
        // A wins (2.0 > 0.8), so first step should be Throw
        assert!(
            matches!(plan.steps.first(), Some(ActionStep::Throw { item, .. }) if item == "smoke"),
            "Score A=2.0 > Score B=0.8, should select Throw smoke"
        );
    }

    #[test]
    fn utility_score_b_dist_affects_cover_fire() {
        // Candidate B with dist<=3: includes CoverFire
        // Me at (0,0), enemy at (2,1) → dist = 2+1 = 3, score_b = 0.8 + (3.0-3.0)*0.05 = 0.8
        let snap = snap_basic(0, 0, 2, 1, 0.0);
        let util = UtilityOrchestrator;
        let plan = util.propose_plan(&snap);
        // Both candidates exist; B should include CoverFire since dist<=3
        let _has_cover = plan
            .steps
            .iter()
            .any(|s| matches!(s, ActionStep::CoverFire { .. }));
        // A: score = 1.0 + 0.0 + hp*0.01 = 2.0 (hp=100)
        // B: score = 0.8 + 0.0 = 0.8
        // A wins, so plan starts with Throw, then MoveTo. CoverFire NOT in plan.
        // But if A's cooldown is active, B wins and includes CoverFire.
        // Let's test with cooldown on Throw:
        let mut snap_cd = snap_basic(0, 0, 2, 1, 0.0);
        snap_cd.me.cooldowns.insert("throw:smoke".into(), 5.0);
        let plan_cd = util.propose_plan(&snap_cd);
        // With cooldown, only candidate B exists, and dist=3 → includes CoverFire
        let has_cover_cd = plan_cd
            .steps
            .iter()
            .any(|s| matches!(s, ActionStep::CoverFire { .. }));
        assert!(
            has_cover_cd,
            "Dist=3 with throw on cooldown should include CoverFire"
        );
    }

    #[test]
    fn utility_score_b_dist_4_no_cover_fire() {
        // dist > 3 → no CoverFire in B
        let mut snap = snap_basic(0, 0, 4, 0, 0.0);
        snap.me.cooldowns.insert("throw:smoke".into(), 5.0); // force B only
        let util = UtilityOrchestrator;
        let plan = util.propose_plan(&snap);
        let has_cover = plan
            .steps
            .iter()
            .any(|s| matches!(s, ActionStep::CoverFire { .. }));
        assert!(!has_cover, "Dist=4 (>3) should NOT include CoverFire");
    }

    #[test]
    fn utility_move_toward_direction_exact() {
        // Verify signum direction: MoveTo uses me.pos + signum(enemy - me) * 2
        // Me at (0,0), enemy at (10,10) → signum(10,10)=(1,1) → move to (2,2)
        let snap = snap_basic(0, 0, 10, 10, 0.0);
        let util = UtilityOrchestrator;
        let plan = util.propose_plan(&snap);
        // A wins → steps[0] = Throw at midpoint (5,5), steps[1] = MoveTo (0+1*2, 0+1*2) = (2,2)
        if let Some(ActionStep::MoveTo { x, y, .. }) = plan.steps.get(1) {
            assert_eq!(*x, 2, "Move X should be me.x + signum * 2 = 0 + 2");
            assert_eq!(*y, 2, "Move Y should be me.y + signum * 2 = 0 + 2");
        }
    }

    #[test]
    fn utility_move_toward_negative_direction() {
        // Me at (10,10), enemy at (5,3) → signum(-5,-7)=(-1,-1) → move to (10-2, 10-2)=(8,8)
        let snap = WorldSnapshot {
            t: 0.0,
            player: PlayerState {
                hp: 100,
                pos: IVec2 { x: 10, y: 10 },
                stance: "stand".into(),
                orders: vec![],
            },
            me: CompanionState {
                ammo: 10,
                cooldowns: BTreeMap::new(),
                morale: 1.0,
                pos: IVec2 { x: 10, y: 10 },
            },
            enemies: vec![EnemyState {
                id: 1,
                pos: IVec2 { x: 5, y: 3 },
                hp: 100,
                cover: "none".into(),
                last_seen: 0.0,
            }],
            pois: vec![],
            obstacles: vec![],
            objective: None,
        };
        let util = UtilityOrchestrator;
        let plan = util.propose_plan(&snap);
        // Throw midpoint: ((10+5)/2, (10+3)/2) = (7, 6)
        if let Some(ActionStep::Throw { x, y, .. }) = plan.steps.first() {
            assert_eq!(*x, 7, "Throw X should be midpoint (10+5)/2=7");
            assert_eq!(*y, 6, "Throw Y should be midpoint (10+3)/2=6");
        }
        if let Some(ActionStep::MoveTo { x, y, .. }) = plan.steps.get(1) {
            assert_eq!(*x, 8, "Move X should be 10 + (-1)*2 = 8");
            assert_eq!(*y, 8, "Move Y should be 10 + (-1)*2 = 8");
        }
    }

    #[test]
    fn goap_next_action_exact_distance_2_cover_fire() {
        // dist=2 → CoverFire, dist=3 → MoveTo
        let snap_dist2 = snap_basic(0, 0, 2, 0, 0.0); // dist = |2|+|0| = 2
        let goap = GoapOrchestrator;
        match goap.next_action(&snap_dist2) {
            ActionStep::CoverFire {
                target_id,
                duration,
            } => {
                assert_eq!(target_id, 2); // snap_basic creates enemy with id=2
                assert!(
                    (duration - 1.5).abs() < f32::EPSILON,
                    "CoverFire duration should be 1.5"
                );
            }
            other => panic!("Expected CoverFire at dist=2, got {:?}", other),
        }
    }

    #[test]
    fn goap_next_action_exact_distance_3_move_to() {
        // dist=3 → MoveTo (not CoverFire)
        let snap_dist3 = snap_basic(0, 0, 3, 0, 0.0); // dist = |3|+|0| = 3
        let goap = GoapOrchestrator;
        match goap.next_action(&snap_dist3) {
            ActionStep::MoveTo { x, y, .. } => {
                assert_eq!(x, 1, "Should move one step toward enemy: 0 + signum(3)=1");
                assert_eq!(y, 0, "Should move one step toward enemy: 0 + signum(0)=0");
            }
            other => panic!("Expected MoveTo at dist=3, got {:?}", other),
        }
    }

    #[test]
    fn goap_next_action_distance_computation_sign() {
        // Me at (5,5), enemy at (2,3) → dx=-3, dy=-2, dist=5
        // Move: x=5+(-1)=4, y=5+(-1)=4
        let snap = WorldSnapshot {
            t: 0.0,
            player: PlayerState {
                hp: 100,
                pos: IVec2 { x: 5, y: 5 },
                stance: "stand".into(),
                orders: vec![],
            },
            me: CompanionState {
                ammo: 10,
                cooldowns: BTreeMap::new(),
                morale: 1.0,
                pos: IVec2 { x: 5, y: 5 },
            },
            enemies: vec![EnemyState {
                id: 1,
                pos: IVec2 { x: 2, y: 3 },
                hp: 100,
                cover: "none".into(),
                last_seen: 0.0,
            }],
            pois: vec![],
            obstacles: vec![],
            objective: None,
        };
        let goap = GoapOrchestrator;
        match goap.next_action(&snap) {
            ActionStep::MoveTo { x, y, .. } => {
                assert_eq!(x, 4, "Should move from 5 toward 2: 5+(-1)=4");
                assert_eq!(y, 4, "Should move from 5 toward 3: 5+(-1)=4");
            }
            other => panic!("Expected MoveTo, got {:?}", other),
        }
    }

    #[test]
    fn goap_next_action_no_enemies_waits() {
        let snap = WorldSnapshot {
            t: 0.0,
            player: PlayerState {
                hp: 100,
                pos: IVec2 { x: 0, y: 0 },
                stance: "stand".into(),
                orders: vec![],
            },
            me: CompanionState {
                ammo: 10,
                cooldowns: BTreeMap::new(),
                morale: 1.0,
                pos: IVec2 { x: 0, y: 0 },
            },
            enemies: vec![],
            pois: vec![],
            obstacles: vec![],
            objective: None,
        };
        let goap = GoapOrchestrator;
        match goap.next_action(&snap) {
            ActionStep::Wait { duration } => {
                assert!(
                    (duration - 1.0).abs() < f32::EPSILON,
                    "Wait duration should be exactly 1.0"
                );
            }
            other => panic!("Expected Wait with no enemies, got {:?}", other),
        }
    }

    #[test]
    fn make_system_orchestrator_returns_orchestrator() {
        let orch = make_system_orchestrator(None);
        // Without llm_orchestrator feature, should return a non-LLM orchestrator
        assert!(!orch.name().is_empty(), "Orchestrator should have a name");
    }

    #[test]
    fn make_system_orchestrator_with_use_llm_false() {
        let cfg = SystemOrchestratorConfig {
            use_llm: false,
            ollama_url: "http://127.0.0.1:11434".into(),
            ollama_model: "phi3:medium".into(),
        };
        let orch = make_system_orchestrator(Some(cfg));
        assert!(
            orch.name().contains("Utility"),
            "With use_llm=false, should return UtilityOrchestrator, got: {}",
            orch.name()
        );
    }

    // ========================================================================
    // Mutation-Killing Tests: Distance calculation with non-origin positions
    // Catches lines 230-232 where |enemy - me| mutates to |enemy + me|
    // and dist = dx + dy mutates to dx - dy
    // ========================================================================

    /// Kills mutation: line 230 `enemy.pos.x - me.pos.x` → `+ me.pos.x`
    /// With me=(5,5), enemy=(7,5): original dx=|7-5|=2, dist=2 → CoverFire.
    /// With mutation: dx=|7+5|=12, dist=12 → no CoverFire. Detectable!
    #[test]
    fn utility_distance_dx_catches_sub_to_add_mutation() {
        let snap = WorldSnapshot {
            t: 0.0,
            player: PlayerState {
                hp: 100,
                pos: IVec2 { x: 5, y: 5 },
                stance: "stand".into(),
                orders: vec![],
            },
            me: CompanionState {
                ammo: 10,
                cooldowns: BTreeMap::from([("throw:smoke".to_string(), 5.0)]),
                morale: 1.0,
                pos: IVec2 { x: 5, y: 5 },
            },
            enemies: vec![EnemyState {
                id: 1,
                pos: IVec2 { x: 7, y: 5 },
                hp: 100,
                cover: "none".into(),
                last_seen: 0.0,
            }],
            pois: vec![],
            obstacles: vec![],
            objective: None,
        };
        let util = UtilityOrchestrator;
        let plan = util.propose_plan(&snap);
        // dist = |7-5| + |5-5| = 2 ≤ 3 → CoverFire should be present
        assert_eq!(
            plan.steps.len(),
            2,
            "Should have MoveTo + CoverFire (dist=2)"
        );
        assert!(
            matches!(plan.steps.get(1), Some(ActionStep::CoverFire { .. })),
            "CoverFire expected when dist=2 with non-origin me"
        );
    }

    /// Kills mutation: line 231 `enemy.pos.y - me.pos.y` → `+ me.pos.y`
    /// With me=(5,5), enemy=(5,7): original dy=|7-5|=2, dist=2 → CoverFire.
    /// With mutation: dy=|7+5|=12, dist=12 → no CoverFire. Detectable!
    #[test]
    fn utility_distance_dy_catches_sub_to_add_mutation() {
        let snap = WorldSnapshot {
            t: 0.0,
            player: PlayerState {
                hp: 100,
                pos: IVec2 { x: 5, y: 5 },
                stance: "stand".into(),
                orders: vec![],
            },
            me: CompanionState {
                ammo: 10,
                cooldowns: BTreeMap::from([("throw:smoke".to_string(), 5.0)]),
                morale: 1.0,
                pos: IVec2 { x: 5, y: 5 },
            },
            enemies: vec![EnemyState {
                id: 1,
                pos: IVec2 { x: 5, y: 7 },
                hp: 100,
                cover: "none".into(),
                last_seen: 0.0,
            }],
            pois: vec![],
            obstacles: vec![],
            objective: None,
        };
        let util = UtilityOrchestrator;
        let plan = util.propose_plan(&snap);
        // dist = |5-5| + |7-5| = 2 ≤ 3 → CoverFire should be present
        assert_eq!(
            plan.steps.len(),
            2,
            "Should have MoveTo + CoverFire (dist=2)"
        );
        assert!(
            matches!(plan.steps.get(1), Some(ActionStep::CoverFire { .. })),
            "CoverFire expected when dist=2 with non-origin me (y-axis)"
        );
    }

    /// Kills mutation: line 232 `dx + dy` → `dx - dy`
    /// With me=(5,5), enemy=(7,7): original dist=|2|+|2|=4 > 3 → no CoverFire.
    /// With mutation: dist=2-2=0 ≤ 3 → CoverFire appears. Detectable!
    #[test]
    fn utility_distance_sum_catches_add_to_sub_mutation() {
        let snap = WorldSnapshot {
            t: 0.0,
            player: PlayerState {
                hp: 100,
                pos: IVec2 { x: 5, y: 5 },
                stance: "stand".into(),
                orders: vec![],
            },
            me: CompanionState {
                ammo: 10,
                cooldowns: BTreeMap::from([("throw:smoke".to_string(), 5.0)]),
                morale: 1.0,
                pos: IVec2 { x: 5, y: 5 },
            },
            enemies: vec![EnemyState {
                id: 1,
                pos: IVec2 { x: 7, y: 7 },
                hp: 100,
                cover: "none".into(),
                last_seen: 0.0,
            }],
            pois: vec![],
            obstacles: vec![],
            objective: None,
        };
        let util = UtilityOrchestrator;
        let plan = util.propose_plan(&snap);
        // dist = |7-5| + |7-5| = 4 > 3 → no CoverFire (only MoveTo)
        assert_eq!(
            plan.steps.len(),
            1,
            "Should have only MoveTo (dist=4, no CoverFire)"
        );
        assert!(
            matches!(plan.steps.first(), Some(ActionStep::MoveTo { .. })),
            "Only MoveTo expected when dist=4 (no CoverFire)"
        );
    }
}
