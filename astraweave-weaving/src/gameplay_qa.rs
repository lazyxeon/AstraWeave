//! Headless gameplay QA for the one-tick Aravath slice.
//!
//! Scenario 1 calls [`crate::run_aravath_slice`] once. The scripted policy and
//! the goal policy each record the edges that call performs. Both traces must
//! match. The function is not split into a tick loop.

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::Deserialize;
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{run_aravath_slice, AravathSlice, ARAVATH_SLICE_SEED};

const SCHEMA: u32 = 1;
const MINIMIZE_BUDGET: u32 = 64;
const CASE_NAME: &str = "gameplay_qa_aravath_slice";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scenario {
    pub id: String,
    pub seed: u64,
    pub max_ticks: u64,
    pub steps: Vec<TickActions>,
    pub objective: String,
    pub policy: String,
    pub reach: Reach,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TickActions {
    pub edges: Vec<Edge>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Edge {
    Hit {
        attack_id: u32,
        defender: Defender,
        seq: u32,
        heal: bool,
    },
    WeaveDecay,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Defender {
    Cover,
    Woven,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reach {
    pub by_tick: u64,
    pub sword_hp: i64,
    pub weave_hp: i64,
    pub cover_intact: bool,
    pub min_cover_damage: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FailureKind {
    TerminalMismatch,
    GoalBotDiverged,
    UnstablePredicate,
}

impl FailureKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::TerminalMismatch => "terminal_mismatch",
            Self::GoalBotDiverged => "goal_bot_diverged",
            Self::UnstablePredicate => "unstable_predicate",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScenarioReport {
    pub trace_sha256: String,
    pub state_sha256: String,
    pub slice: AravathSlice,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FailureArtifact {
    pub schema: u32,
    pub scenario_id: String,
    pub seed: String,
    pub setup: String,
    pub inputs: Vec<TickRecord>,
    pub hashes: Vec<HashRow>,
    pub failure: FailureBody,
    pub original_len: usize,
    pub minimized_len: usize,
    /// `skipped` on the pull-request path, `full` when ddmin finished, `partial` at the budget.
    pub minimized: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TickRecord {
    pub tick: u64,
    pub edges: Vec<EdgeRecord>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeRecord {
    Hit {
        attack_id: u32,
        defender: String,
        seq: u32,
        heal: bool,
    },
    WeaveDecay {},
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HashRow {
    pub tick: u64,
    pub sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FailureBody {
    pub kind: String,
    pub tick: u64,
    pub detail: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MinimizeMode {
    Off,
    On,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RunOutcome {
    Pass(ScenarioReport),
    Fail(FailureArtifact),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MinimizeResult {
    pub ticks: Vec<TickActions>,
    pub calls: u32,
    pub partial: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename = "Scenario", deny_unknown_fields)]
struct ScenarioFile {
    schema: u32,
    id: String,
    seed: u64,
    max_ticks: u64,
    setup: SetupFile,
    bot: BotFile,
    goal: GoalFile,
    checks: Vec<CheckFile>,
}

#[derive(Debug, Deserialize)]
enum SetupFile {
    Slice,
}

#[derive(Debug, Deserialize)]
enum BotFile {
    Script { steps: Vec<StepFile> },
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct StepFile {
    edges: Vec<EdgeFile>,
}

#[derive(Debug, Deserialize)]
enum EdgeFile {
    Hit {
        attack_id: u32,
        defender: DefenderFile,
        seq: u32,
        heal: bool,
    },
    WeaveDecay,
}

#[derive(Debug, Deserialize)]
enum DefenderFile {
    Cover,
    Woven,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct GoalFile {
    objective: String,
    policy: String,
}

#[derive(Debug, Deserialize)]
enum CheckFile {
    Reach {
        by_tick: u64,
        sword_hp: i64,
        weave_hp: i64,
        cover_intact: bool,
        min_cover_damage: i64,
    },
}

/// `QA_MINIMIZE=1` and the GitHub event is not `pull_request`.
#[must_use]
pub fn minimize_enabled() -> bool {
    minimize_enabled_from(
        std::env::var("QA_MINIMIZE").ok().as_deref(),
        std::env::var("GITHUB_EVENT_NAME").ok().as_deref(),
    )
}

#[must_use]
pub fn minimize_enabled_from(qa_minimize: Option<&str>, github_event: Option<&str>) -> bool {
    qa_minimize == Some("1") && github_event != Some("pull_request")
}

pub fn load_scenario(text: &str) -> Result<Scenario> {
    let file: ScenarioFile = ron::from_str(text).context("parse gameplay scenario")?;
    if file.schema != SCHEMA {
        bail!("scenario schema {} is not {SCHEMA}", file.schema);
    }
    if file.seed != ARAVATH_SLICE_SEED {
        bail!(
            "scenario seed {seed:#x} does not match the slice seed {ARAVATH_SLICE_SEED:#x}",
            seed = file.seed
        );
    }
    let SetupFile::Slice = file.setup;
    let BotFile::Script { steps } = file.bot;
    if steps.is_empty() {
        bail!("scenario {} has no scripted ticks", file.id);
    }
    if steps.len() as u64 > file.max_ticks {
        bail!(
            "scenario {} has {} ticks and max_ticks {}",
            file.id,
            steps.len(),
            file.max_ticks
        );
    }
    let mut reach = None;
    for check in file.checks {
        match check {
            CheckFile::Reach {
                by_tick,
                sword_hp,
                weave_hp,
                cover_intact,
                min_cover_damage,
            } => {
                if reach.is_some() {
                    bail!("scenario {} has two Reach checks", file.id);
                }
                if by_tick < 1 {
                    bail!("Reach.by_tick starts at 1");
                }
                reach = Some(Reach {
                    by_tick,
                    sword_hp,
                    weave_hp,
                    cover_intact,
                    min_cover_damage,
                });
            }
        }
    }
    let Some(reach) = reach else {
        bail!("scenario {} has no Reach check", file.id);
    };
    Ok(Scenario {
        id: file.id,
        seed: file.seed,
        max_ticks: file.max_ticks,
        steps: steps
            .into_iter()
            .map(|step| TickActions {
                edges: step.edges.into_iter().map(Edge::from).collect(),
            })
            .collect(),
        objective: file.goal.objective,
        policy: file.goal.policy,
        reach,
    })
}

pub fn load_bundled_scenario() -> Result<Scenario> {
    let path = bundled_scenario_path();
    let text = std::fs::read_to_string(&path)
        .with_context(|| format!("read scenario {}", path.display()))?;
    load_scenario(&text)
}

#[must_use]
pub fn bundled_scenario_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("scenarios/aravath_slice_sword.ron")
}

pub fn run_scenario(scenario: &Scenario) -> RunOutcome {
    let mode = if minimize_enabled() {
        MinimizeMode::On
    } else {
        MinimizeMode::Off
    };
    run_scenario_with(scenario, mode)
}

pub fn run_scenario_with(scenario: &Scenario, minimize: MinimizeMode) -> RunOutcome {
    let scripted = scenario.steps.clone();
    let goal = match goal_edges(&scenario.objective, &scenario.policy) {
        Ok(edges) => edges,
        Err(detail) => {
            return fail_outcome(
                scenario,
                &scripted,
                FailureKind::GoalBotDiverged,
                detail.to_string(),
                minimize,
            );
        }
    };
    let scripted_trace = execute(&scripted);
    let goal_trace = execute(&goal);
    if scripted_trace.trace_sha256 != goal_trace.trace_sha256 {
        return fail_outcome(
            scenario,
            &scripted,
            FailureKind::GoalBotDiverged,
            format!(
                "scripted {} goal {}",
                scripted_trace.trace_sha256, goal_trace.trace_sha256
            ),
            minimize,
        );
    }
    if let Some(detail) = reach_detail(&scripted_trace.slice, &scenario.reach) {
        return fail_outcome(
            scenario,
            &scripted,
            FailureKind::TerminalMismatch,
            detail,
            minimize,
        );
    }
    RunOutcome::Pass(scripted_trace)
}

pub fn failure_json(artifact: &FailureArtifact) -> Result<String> {
    let mut body = serde_json::to_string_pretty(artifact).context("encode failure artifact")?;
    body.push('\n');
    Ok(body)
}

pub fn write_failure_artifact(path: &Path, artifact: &FailureArtifact) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    let body = failure_json(artifact)?;
    std::fs::write(path, body).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

#[must_use]
pub fn replay_case_path() -> PathBuf {
    if let Ok(dir) = std::env::var("CARGO_TARGET_DIR") {
        return PathBuf::from(dir).join("replay-hash-gameplay-qa.json");
    }
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path.push("target");
    path.push("replay-hash-gameplay-qa.json");
    path
}

pub fn write_replay_case(trace_sha256: &str) -> Result<PathBuf> {
    let path = replay_case_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    let body = format!("[\n  {{ \"case\": \"{CASE_NAME}\", \"sha256\": \"{trace_sha256}\" }}\n]\n");
    std::fs::write(&path, body).with_context(|| format!("write {}", path.display()))?;
    Ok(path)
}

pub fn minimize(
    input: &[TickActions],
    mut pred: impl FnMut(&[TickActions]) -> bool,
) -> MinimizeResult {
    let mut session = Session {
        calls: 0,
        partial: false,
        best: input.to_vec(),
        pred: &mut pred,
    };
    match session.probe(input) {
        Some(true) => {}
        _ => {
            return MinimizeResult {
                ticks: input.to_vec(),
                calls: session.calls,
                partial: session.partial,
            };
        }
    }
    let reduced = session.reduce_ticks(input.to_vec());
    let reduced = session.reduce_edges(reduced);
    let reduced = session.drop_empty_ticks(reduced);
    MinimizeResult {
        ticks: reduced,
        calls: session.calls,
        partial: session.partial,
    }
}

fn goal_edges(objective: &str, policy: &str) -> Result<Vec<TickActions>> {
    if objective == "cover_broken" && policy == "slice_sword_then_weave" {
        return Ok(vec![TickActions {
            edges: slice_program(),
        }]);
    }
    bail!("unknown goal objective {objective} policy {policy}");
}

fn slice_program() -> Vec<Edge> {
    vec![
        Edge::Hit {
            attack_id: 1,
            defender: Defender::Cover,
            seq: 0,
            heal: false,
        },
        Edge::Hit {
            attack_id: 3,
            defender: Defender::Woven,
            seq: 1,
            heal: false,
        },
        Edge::WeaveDecay,
    ]
}

fn execute(steps: &[TickActions]) -> ScenarioReport {
    let slice = run_aravath_slice();
    let state_sha256 = sha256_hex(&state_bytes(&slice));
    let trace_sha256 = sha256_hex(&trace_bytes(steps, &slice));
    ScenarioReport {
        trace_sha256,
        state_sha256,
        slice,
    }
}

fn reach_detail(slice: &AravathSlice, reach: &Reach) -> Option<String> {
    if slice.sword_hp != reach.sword_hp {
        return Some(format!(
            "sword_hp {} expected, got {}",
            reach.sword_hp, slice.sword_hp
        ));
    }
    if slice.weave_hp != reach.weave_hp {
        return Some(format!(
            "weave_hp {} expected, got {}",
            reach.weave_hp, slice.weave_hp
        ));
    }
    if slice.cover_intact != reach.cover_intact {
        return Some(format!(
            "cover_intact {} expected, got {}",
            reach.cover_intact, slice.cover_intact
        ));
    }
    if slice.cover_damage < reach.min_cover_damage {
        return Some(format!(
            "cover_damage {} is below {}",
            slice.cover_damage, reach.min_cover_damage
        ));
    }
    None
}

fn fail_outcome(
    scenario: &Scenario,
    inputs: &[TickActions],
    kind: FailureKind,
    detail: String,
    mode: MinimizeMode,
) -> RunOutcome {
    let original_len = inputs.len();
    let (chosen, minimized) = match mode {
        MinimizeMode::Off => (inputs.to_vec(), "skipped".to_string()),
        MinimizeMode::On => {
            let reduced = minimize(inputs, |candidate| {
                failure_kind(scenario, candidate) == Some(kind)
            });
            let label = if reduced.partial { "partial" } else { "full" };
            (reduced.ticks, label.to_string())
        }
    };
    let mut chosen = chosen;
    let mut kind = kind;
    let mut detail = detail;
    let mut minimized = minimized;
    if minimized != "skipped" {
        let once = execute(&chosen);
        let twice = execute(&chosen);
        if once.trace_sha256 != twice.trace_sha256 {
            chosen = inputs.to_vec();
            kind = FailureKind::UnstablePredicate;
            detail = format!(
                "minimized trace {} then {}",
                once.trace_sha256, twice.trace_sha256
            );
            minimized = "skipped".to_string();
        }
    }
    let report = execute(&chosen);
    RunOutcome::Fail(FailureArtifact {
        schema: SCHEMA,
        scenario_id: scenario.id.clone(),
        seed: format!("0x{:X}", scenario.seed),
        setup: "slice".to_string(),
        inputs: records(&chosen),
        hashes: vec![HashRow {
            tick: 0,
            sha256: report.state_sha256,
        }],
        failure: FailureBody {
            kind: kind.as_str().to_string(),
            tick: 0,
            detail,
        },
        original_len,
        minimized_len: chosen.len(),
        minimized,
    })
}

fn failure_kind(scenario: &Scenario, steps: &[TickActions]) -> Option<FailureKind> {
    let goal = goal_edges(&scenario.objective, &scenario.policy).ok()?;
    let scripted = execute(steps);
    let goal_trace = execute(&goal);
    if scripted.trace_sha256 != goal_trace.trace_sha256 {
        return Some(FailureKind::GoalBotDiverged);
    }
    if reach_detail(&scripted.slice, &scenario.reach).is_some() {
        return Some(FailureKind::TerminalMismatch);
    }
    None
}

fn records(steps: &[TickActions]) -> Vec<TickRecord> {
    steps
        .iter()
        .enumerate()
        .map(|(tick, step)| TickRecord {
            tick: tick as u64,
            edges: step.edges.iter().map(Edge::record).collect(),
        })
        .collect()
}

fn state_bytes(slice: &AravathSlice) -> Vec<u8> {
    let mut out = Vec::with_capacity(8 * 3 + 1);
    push_i64(&mut out, slice.sword_hp);
    push_i64(&mut out, slice.weave_hp);
    out.push(u8::from(slice.cover_intact));
    push_i64(&mut out, slice.cover_damage);
    out
}

fn trace_bytes(steps: &[TickActions], slice: &AravathSlice) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"AWQA");
    push_u32(&mut out, SCHEMA);
    push_u64(&mut out, ARAVATH_SLICE_SEED);
    push_u32(&mut out, steps.len() as u32);
    for step in steps {
        push_u32(&mut out, step.edges.len() as u32);
        for edge in &step.edges {
            encode_edge(&mut out, edge);
        }
    }
    out.extend_from_slice(&state_bytes(slice));
    out
}

fn encode_edge(out: &mut Vec<u8>, edge: &Edge) {
    match edge {
        Edge::Hit {
            attack_id,
            defender,
            seq,
            heal,
        } => {
            out.push(1);
            push_u32(out, *attack_id);
            out.push(match defender {
                Defender::Cover => 1,
                Defender::Woven => 2,
            });
            push_u32(out, *seq);
            out.push(u8::from(*heal));
        }
        Edge::WeaveDecay => out.push(2),
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        hex.push_str(&format!("{byte:02x}"));
    }
    hex
}

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_i64(out: &mut Vec<u8>, value: i64) {
    out.extend_from_slice(&value.to_le_bytes());
}

struct Session<'a> {
    calls: u32,
    partial: bool,
    best: Vec<TickActions>,
    pred: &'a mut dyn FnMut(&[TickActions]) -> bool,
}

impl Session<'_> {
    fn probe(&mut self, candidate: &[TickActions]) -> Option<bool> {
        if self.calls >= MINIMIZE_BUDGET {
            self.partial = true;
            return None;
        }
        self.calls += 1;
        let ok = (self.pred)(candidate);
        if ok && tighter(candidate, &self.best) {
            self.best = candidate.to_vec();
        }
        Some(ok)
    }

    fn reduce_ticks(&mut self, mut items: Vec<TickActions>) -> Vec<TickActions> {
        let mut parts = 2usize;
        while items.len() >= 2 && !self.partial {
            let mut start = 0usize;
            let chunk = (items.len() / parts).max(1);
            let mut reduced = false;
            while start < items.len() {
                let end = (start + chunk).min(items.len());
                let mut complement = Vec::with_capacity(items.len() - (end - start));
                complement.extend_from_slice(&items[..start]);
                complement.extend_from_slice(&items[end..]);
                match self.probe(&complement) {
                    Some(true) => {
                        items = complement;
                        parts = parts.saturating_sub(1).max(2);
                        reduced = true;
                        break;
                    }
                    None => return self.best.clone(),
                    Some(false) => start = end,
                }
            }
            if !reduced {
                if parts >= items.len() {
                    break;
                }
                parts = (parts * 2).min(items.len());
            }
        }
        items
    }

    fn reduce_edges(&mut self, mut items: Vec<TickActions>) -> Vec<TickActions> {
        for index in 0..items.len() {
            if self.partial {
                break;
            }
            let edges = items[index].edges.clone();
            if edges.len() < 2 {
                continue;
            }
            let kept = self.ddmin_edges(&items, index, edges);
            items[index].edges = kept;
        }
        items
    }

    fn ddmin_edges(
        &mut self,
        items: &[TickActions],
        index: usize,
        mut edges: Vec<Edge>,
    ) -> Vec<Edge> {
        let mut parts = 2usize;
        while edges.len() >= 2 && !self.partial {
            let mut start = 0usize;
            let chunk = (edges.len() / parts).max(1);
            let mut reduced = false;
            while start < edges.len() {
                let end = (start + chunk).min(edges.len());
                let mut complement = Vec::with_capacity(edges.len() - (end - start));
                complement.extend_from_slice(&edges[..start]);
                complement.extend_from_slice(&edges[end..]);
                let candidate = splice(items, index, &complement);
                match self.probe(&candidate) {
                    Some(true) => {
                        edges = complement;
                        parts = parts.saturating_sub(1).max(2);
                        reduced = true;
                        break;
                    }
                    None => return edges,
                    Some(false) => start = end,
                }
            }
            if !reduced {
                if parts >= edges.len() {
                    break;
                }
                parts = (parts * 2).min(edges.len());
            }
        }
        edges
    }

    fn drop_empty_ticks(&mut self, mut items: Vec<TickActions>) -> Vec<TickActions> {
        let mut index = 0usize;
        while index < items.len() && !self.partial {
            if !items[index].edges.is_empty() {
                index += 1;
                continue;
            }
            let mut candidate = items.clone();
            candidate.remove(index);
            match self.probe(&candidate) {
                Some(true) => items = candidate,
                None => break,
                Some(false) => index += 1,
            }
        }
        if tighter(&self.best, &items) {
            self.best.clone()
        } else {
            items
        }
    }
}

fn splice(items: &[TickActions], index: usize, edges: &[Edge]) -> Vec<TickActions> {
    let mut candidate = items.to_vec();
    candidate[index].edges = edges.to_vec();
    candidate
}

fn tighter(candidate: &[TickActions], best: &[TickActions]) -> bool {
    let candidate_edges: usize = candidate.iter().map(|tick| tick.edges.len()).sum();
    let best_edges: usize = best.iter().map(|tick| tick.edges.len()).sum();
    candidate_edges < best_edges || (candidate_edges == best_edges && candidate.len() < best.len())
}

impl From<EdgeFile> for Edge {
    fn from(edge: EdgeFile) -> Self {
        match edge {
            EdgeFile::Hit {
                attack_id,
                defender,
                seq,
                heal,
            } => Self::Hit {
                attack_id,
                defender: match defender {
                    DefenderFile::Cover => Defender::Cover,
                    DefenderFile::Woven => Defender::Woven,
                },
                seq,
                heal,
            },
            EdgeFile::WeaveDecay => Self::WeaveDecay,
        }
    }
}

impl Edge {
    fn record(&self) -> EdgeRecord {
        match self {
            Self::Hit {
                attack_id,
                defender,
                seq,
                heal,
            } => EdgeRecord::Hit {
                attack_id: *attack_id,
                defender: match defender {
                    Defender::Cover => "cover",
                    Defender::Woven => "woven",
                }
                .to_string(),
                seq: *seq,
                heal: *heal,
            },
            Self::WeaveDecay => EdgeRecord::WeaveDecay {},
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hit(seq: u32) -> TickActions {
        TickActions {
            edges: vec![Edge::Hit {
                attack_id: 1,
                defender: Defender::Cover,
                seq,
                heal: false,
            }],
        }
    }

    #[test]
    fn replay_aravath_slice_scenario() {
        let scenario = load_bundled_scenario().expect("scenario");
        let first = run_scenario(&scenario);
        let second = run_scenario(&scenario);
        let RunOutcome::Pass(first) = first else {
            panic!("scripted and goal traces diverged or Reach failed");
        };
        let RunOutcome::Pass(second) = second else {
            panic!("second run failed");
        };
        assert_eq!(first.trace_sha256, second.trace_sha256);
        assert_eq!(
            first.trace_sha256, "ef7e38b580f2d157041d2d58bc0cac41dcf1442ac0ff6064d6ba52039adfc089",
            "gameplay_qa_aravath_slice digest drifted"
        );
        assert_eq!(first.slice.sword_hp, 15);
        assert_eq!(first.slice.weave_hp, 25);
        assert!(!first.slice.cover_intact);
        assert!(first.slice.cover_damage >= 25);
        let path = write_replay_case(&first.trace_sha256).expect("write case");
        let body = std::fs::read_to_string(&path).expect("read case");
        assert!(body.contains(CASE_NAME));
        assert!(body.contains(&first.trace_sha256));
        println!("{CASE_NAME} {}", first.trace_sha256);
    }

    #[test]
    fn minimizer_is_off_for_pull_requests() {
        assert!(!minimize_enabled_from(Some("1"), Some("pull_request")));
        assert!(minimize_enabled_from(Some("1"), Some("push")));
        assert!(minimize_enabled_from(Some("1"), None));
        assert!(!minimize_enabled_from(None, Some("push")));
        assert!(!minimize_enabled_from(Some("0"), None));
    }

    #[test]
    fn ddmin_keeps_the_marked_tick() {
        let input = vec![hit(1), hit(2), hit(7), hit(4)];
        let result = minimize(&input, |candidate| {
            candidate.iter().any(|tick| {
                tick.edges
                    .iter()
                    .any(|edge| matches!(edge, Edge::Hit { seq: 7, .. }))
            })
        });
        assert!(!result.partial);
        assert_eq!(result.ticks, vec![hit(7)]);
        assert!(result.calls <= MINIMIZE_BUDGET);
    }

    #[test]
    fn ddmin_keeps_the_marked_edge_inside_one_tick() {
        let input = vec![TickActions {
            edges: vec![
                Edge::WeaveDecay,
                Edge::Hit {
                    attack_id: 1,
                    defender: Defender::Cover,
                    seq: 7,
                    heal: false,
                },
                Edge::WeaveDecay,
            ],
        }];
        let result = minimize(&input, |candidate| {
            candidate.iter().any(|tick| {
                tick.edges
                    .iter()
                    .any(|edge| matches!(edge, Edge::Hit { seq: 7, .. }))
            })
        });
        assert!(!result.partial);
        assert_eq!(result.ticks.len(), 1);
        assert_eq!(
            result.ticks[0].edges,
            vec![Edge::Hit {
                attack_id: 1,
                defender: Defender::Cover,
                seq: 7,
                heal: false,
            }]
        );
    }

    #[test]
    fn ddmin_stops_at_the_budget() {
        let input: Vec<_> = (0..70).map(hit).collect();
        let full = input.clone();
        let result = minimize(&input, |candidate| candidate == full.as_slice());
        assert!(result.partial);
        assert_eq!(result.calls, MINIMIZE_BUDGET);
        assert_eq!(result.ticks, full);
    }

    #[test]
    fn a_goal_mismatch_writes_a_minimized_artifact_only_when_asked() {
        let scenario = Scenario {
            id: "aravath_slice_sword".to_string(),
            seed: ARAVATH_SLICE_SEED,
            max_ticks: 2,
            steps: vec![
                TickActions {
                    edges: vec![Edge::WeaveDecay],
                },
                TickActions {
                    edges: vec![Edge::WeaveDecay],
                },
            ],
            objective: "cover_broken".to_string(),
            policy: "slice_sword_then_weave".to_string(),
            reach: Reach {
                by_tick: 1,
                sword_hp: 15,
                weave_hp: 25,
                cover_intact: false,
                min_cover_damage: 25,
            },
        };
        let RunOutcome::Fail(skipped) = run_scenario_with(&scenario, MinimizeMode::Off) else {
            panic!("expected a goal divergence");
        };
        assert_eq!(skipped.failure.kind, "goal_bot_diverged");
        assert_eq!(skipped.minimized, "skipped");
        assert_eq!(skipped.original_len, skipped.minimized_len);

        let RunOutcome::Fail(reduced) = run_scenario_with(&scenario, MinimizeMode::On) else {
            panic!("expected a goal divergence");
        };
        assert_eq!(reduced.failure.kind, "goal_bot_diverged");
        assert_eq!(reduced.minimized, "full");
        assert!(reduced.minimized_len < reduced.original_len);
        let json = failure_json(&reduced).expect("json");
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("parse");
        assert_eq!(parsed["setup"], "slice");
        assert_eq!(parsed["seed"], "0xA11CE5EE");
        assert_eq!(parsed["failure"]["kind"], "goal_bot_diverged");
    }
}
