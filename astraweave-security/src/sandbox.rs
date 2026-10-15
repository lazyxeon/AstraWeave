//! Deterministic Rhai sandbox and mod loader.
//!
//! One implementation. `astraweave-author` and `astraweave-scripting` call this
//! module; they do not build their own engines. The sim path uses the operation
//! budget as the only execution limit. It does not read `SystemTime`, `Instant`,
//! or a tokio timeout.
//!
//! Memory is bounded by the string, array, map, and variable caps Rhai enforces.
//! There is no host filesystem, network, or process API. `import` and `eval` are
//! disabled, and the module resolver rejects every path.

use crate::{ExecutionLimits, ScriptSandbox};
use anyhow::{bail, Context, Result};
use astraweave_core::sim::{mix_sim_seed, SimRng};
use astraweave_weaving::parse_template_toml;
use rand::Rng;
use rhai::packages::{
    ArithmeticPackage, BasicArrayPackage, BasicIteratorPackage, BasicMapPackage,
    BasicStringPackage, LogicPackage, Package,
};
use rhai::{Dynamic, Engine, Scope, AST};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Mod manifest `api` field this engine accepts. A mismatch is a load error.
pub const MOD_API_VERSION: u32 = 1;

/// Public command set. Bump when a command's name or result changes.
pub const MOD_COMMAND_API_VERSION: u32 = 2;

/// Commands a sandboxed script may be granted. Sorted. This is the allowlist.
pub static COMMANDS: &[&str] = &[
    "apply_damage",
    "configure",
    "despawn",
    "move_to",
    "play_sound",
    "queue_weave",
    "set_position",
    "sim_roll",
    "spawn_particle",
    "spawn_prefab",
];

pub const DEFAULT_MAX_OPERATIONS: u64 = 50_000;
pub const DEFAULT_MAX_STRING_SIZE: usize = 4_096;
pub const DEFAULT_MAX_ARRAY_SIZE: usize = 1_024;
pub const DEFAULT_MAX_MAP_SIZE: usize = 256;
pub const DEFAULT_MAX_VARIABLES: usize = 256;
pub const DEFAULT_MAX_FUNCTIONS: usize = 64;
pub const DEFAULT_MAX_EXPR_DEPTH: usize = 64;
pub const DEFAULT_MAX_CALL_LEVELS: usize = 64;
pub const DEFAULT_MAX_SCRIPT_BYTES: usize = 256 * 1024;
pub const DEFAULT_MAX_MANIFEST_BYTES: usize = 64 * 1024;
pub const DEFAULT_MAX_SCRIPTS: usize = 32;
pub const DEFAULT_MAX_MODS: usize = 64;
pub const DEFAULT_MAX_TRAPS: i32 = 64;
pub const DEFAULT_MAX_TERRAIN_EDITS: i32 = 256;
pub const DEFAULT_MAX_SPAWNS: i32 = 128;
pub const DEFAULT_MAX_WEAVES: i32 = 64;
pub const DEFAULT_MAX_MOVE_DELTA: i32 = 100_000;
pub const DEFAULT_MAX_DAMAGE: i32 = 10_000;
pub const MAX_SIM_ROLL_BOUND: i64 = 1_000_000;

/// Groups of [`COMMANDS`] a host may register.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Capability {
    Configure,
    MoveEntity,
    SpawnEntity,
    DespawnEntity,
    ApplyDamage,
    Presentation,
    SimRng,
    Weave,
}

impl Capability {
    pub fn command_names(self) -> &'static [&'static str] {
        match self {
            Capability::Configure => &["configure"],
            Capability::MoveEntity => &["move_to", "set_position"],
            Capability::SpawnEntity => &["spawn_prefab"],
            Capability::DespawnEntity => &["despawn"],
            Capability::ApplyDamage => &["apply_damage"],
            Capability::Presentation => &["play_sound", "spawn_particle"],
            Capability::SimRng => &["sim_roll"],
            Capability::Weave => &["queue_weave"],
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "configure" => Some(Self::Configure),
            "move_entity" => Some(Self::MoveEntity),
            "spawn_entity" => Some(Self::SpawnEntity),
            "despawn_entity" => Some(Self::DespawnEntity),
            "apply_damage" => Some(Self::ApplyDamage),
            "presentation" => Some(Self::Presentation),
            "sim_rng" => Some(Self::SimRng),
            "weave" => Some(Self::Weave),
            _ => None,
        }
    }
}

/// Structural ceilings for mod data and for the Rhai engine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EngineCeilings {
    pub max_traps: i32,
    pub max_terrain_edits: i32,
    pub max_spawns: i32,
    /// Weave casts queued by `queue_weave`. Not spent from `terrain_edits`.
    pub max_weaves: i32,
    pub max_move_delta: i32,
    pub max_damage: i32,
    pub max_script_bytes: usize,
    pub max_manifest_bytes: usize,
    pub max_scripts: usize,
    pub max_mods: usize,
    pub max_operations: u64,
    pub max_string_size: usize,
    pub max_array_size: usize,
    pub max_map_size: usize,
    pub max_variables: usize,
    pub max_functions: usize,
    pub max_expr_depth: usize,
    pub max_call_levels: usize,
}

impl Default for EngineCeilings {
    fn default() -> Self {
        Self {
            max_traps: DEFAULT_MAX_TRAPS,
            max_terrain_edits: DEFAULT_MAX_TERRAIN_EDITS,
            max_spawns: DEFAULT_MAX_SPAWNS,
            max_weaves: DEFAULT_MAX_WEAVES,
            max_move_delta: DEFAULT_MAX_MOVE_DELTA,
            max_damage: DEFAULT_MAX_DAMAGE,
            max_script_bytes: DEFAULT_MAX_SCRIPT_BYTES,
            max_manifest_bytes: DEFAULT_MAX_MANIFEST_BYTES,
            max_scripts: DEFAULT_MAX_SCRIPTS,
            max_mods: DEFAULT_MAX_MODS,
            max_operations: DEFAULT_MAX_OPERATIONS,
            max_string_size: DEFAULT_MAX_STRING_SIZE,
            max_array_size: DEFAULT_MAX_ARRAY_SIZE,
            max_map_size: DEFAULT_MAX_MAP_SIZE,
            max_variables: DEFAULT_MAX_VARIABLES,
            max_functions: DEFAULT_MAX_FUNCTIONS,
            max_expr_depth: DEFAULT_MAX_EXPR_DEPTH,
            max_call_levels: DEFAULT_MAX_CALL_LEVELS,
        }
    }
}

impl EngineCeilings {
    /// Structural ceiling: the product of the string, array, and map caps.
    ///
    /// This is not a process RSS limit and it is not enforced by a wall clock.
    /// The sim executor keeps `timeout_ms` at 0 and stops scripts with the
    /// operation budget.
    pub fn advertised_memory_bytes(&self) -> usize {
        self.max_string_size
            .saturating_mul(self.max_variables)
            .saturating_add(self.max_array_size.saturating_mul(16))
            .saturating_add(self.max_map_size.saturating_mul(64))
    }
}

/// Integer director budget checked against [`EngineCeilings`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BudgetLimits {
    pub traps: i32,
    pub terrain_edits: i32,
    pub spawns: i32,
}

pub fn validate_budget(budget: BudgetLimits, ceilings: &EngineCeilings) -> Result<()> {
    check_nonneg("traps", budget.traps, ceilings.max_traps)?;
    check_nonneg(
        "terrain_edits",
        budget.terrain_edits,
        ceilings.max_terrain_edits,
    )?;
    check_nonneg("spawns", budget.spawns, ceilings.max_spawns)?;
    Ok(())
}

fn check_nonneg(name: &str, value: i32, max: i32) -> Result<()> {
    if value < 0 {
        bail!("{name} {value} is negative");
    }
    if value > max {
        bail!("{name} {value} exceeds engine ceiling {max}");
    }
    Ok(())
}

/// Which entry point and host commands the engine exposes.
#[derive(Clone, Debug)]
pub struct SandboxProfile {
    pub capabilities: BTreeSet<Capability>,
    pub ceilings: EngineCeilings,
}

impl SandboxProfile {
    pub fn author() -> Self {
        let mut capabilities = BTreeSet::new();
        capabilities.insert(Capability::Configure);
        Self {
            capabilities,
            ceilings: EngineCeilings::default(),
        }
    }

    pub fn sim() -> Self {
        let mut capabilities = BTreeSet::new();
        capabilities.insert(Capability::MoveEntity);
        capabilities.insert(Capability::SimRng);
        capabilities.insert(Capability::Weave);
        Self {
            capabilities,
            ceilings: EngineCeilings::default(),
        }
    }

    pub fn gameplay() -> Self {
        Self {
            capabilities: all_capabilities(),
            ceilings: EngineCeilings::default(),
        }
    }
}

fn all_capabilities() -> BTreeSet<Capability> {
    [
        Capability::Configure,
        Capability::MoveEntity,
        Capability::SpawnEntity,
        Capability::DespawnEntity,
        Capability::ApplyDamage,
        Capability::Presentation,
        Capability::SimRng,
        Capability::Weave,
    ]
    .into_iter()
    .collect()
}

/// Engine plus the capability allowlist that was registered into it.
pub struct SandboxEngine {
    engine: Engine,
    allow: BTreeMap<String, String>,
    ceilings: EngineCeilings,
}

impl SandboxEngine {
    pub fn build(profile: &SandboxProfile) -> Self {
        let mut engine = Engine::new_raw();
        engine.set_optimization_level(rhai::OptimizationLevel::None);
        let c = &profile.ceilings;
        engine.set_max_operations(c.max_operations);
        engine.set_max_string_size(c.max_string_size);
        engine.set_max_array_size(c.max_array_size);
        engine.set_max_map_size(c.max_map_size);
        engine.set_max_variables(c.max_variables);
        engine.set_max_functions(c.max_functions);
        engine.set_max_expr_depths(c.max_expr_depth, c.max_expr_depth);
        engine.set_max_call_levels(c.max_call_levels);
        engine.set_max_modules(0);
        engine.set_module_resolver(rhai::module_resolvers::DummyModuleResolver::new());
        engine.disable_symbol("eval");
        engine.disable_symbol("import");
        engine.disable_symbol("export");
        engine.disable_symbol("print");
        engine.disable_symbol("debug");
        engine.on_print(|_| {});
        engine.on_debug(|_, _, _| {});

        ArithmeticPackage::new().register_into_engine(&mut engine);
        LogicPackage::new().register_into_engine(&mut engine);
        BasicArrayPackage::new().register_into_engine(&mut engine);
        BasicMapPackage::new().register_into_engine(&mut engine);
        BasicStringPackage::new().register_into_engine(&mut engine);
        BasicIteratorPackage::new().register_into_engine(&mut engine);

        let mut allow = BTreeMap::new();
        let version = MOD_COMMAND_API_VERSION.to_string();
        for cap in &profile.capabilities {
            for name in cap.command_names() {
                allow.insert((*name).to_string(), version.clone());
            }
        }

        Self {
            engine,
            allow,
            ceilings: profile.ceilings.clone(),
        }
    }

    pub fn engine(&self) -> &Engine {
        &self.engine
    }

    pub fn engine_mut(&mut self) -> &mut Engine {
        &mut self.engine
    }

    pub fn allowlist(&self) -> &BTreeMap<String, String> {
        &self.allow
    }

    pub fn ceilings(&self) -> &EngineCeilings {
        &self.ceilings
    }

    pub fn grant(&mut self, name: &str) -> Result<()> {
        if !COMMANDS.contains(&name) {
            bail!("unknown command {name}");
        }
        self.allow
            .insert(name.to_string(), MOD_COMMAND_API_VERSION.to_string());
        Ok(())
    }

    pub fn into_engine(self) -> Engine {
        self.engine
    }

    /// Move this engine into the existing [`ScriptSandbox`] type.
    ///
    /// `timeout_ms` is 0. The sim executor does not arm a wall clock.
    /// `allowed_functions` is the capability map, and [`execute_deterministic`]
    /// refuses to run when it is empty.
    pub fn into_script_sandbox(self) -> ScriptSandbox {
        let memory = self.ceilings.advertised_memory_bytes();
        let ops = self.ceilings.max_operations;
        ScriptSandbox {
            engine: Arc::new(Mutex::new(self.engine)),
            allowed_functions: self.allow.into_iter().collect(),
            execution_limits: ExecutionLimits {
                max_operations: ops,
                max_memory_bytes: memory,
                timeout_ms: 0,
            },
        }
    }
}

/// Run a script body. The allowlist must be non-empty. No wall clock.
pub fn execute_deterministic(
    sandbox: &ScriptSandbox,
    source: &str,
    scope: &mut Scope<'_>,
) -> Result<Dynamic> {
    if sandbox.allowed_functions.is_empty() {
        bail!("capability allowlist is empty");
    }
    if sandbox.execution_limits.max_operations == 0 {
        bail!("operation budget must be finite");
    }
    if source.len() > DEFAULT_MAX_SCRIPT_BYTES {
        bail!(
            "script is {} bytes, ceiling is {DEFAULT_MAX_SCRIPT_BYTES}",
            source.len()
        );
    }
    let engine = sandbox
        .engine
        .lock()
        .map_err(|err| anyhow::anyhow!("sandbox lock poisoned: {err}"))?;
    if engine.max_operations() == 0 {
        bail!("engine operation budget is unlimited");
    }
    if engine.max_modules() != 0 {
        bail!("engine still allows module imports");
    }
    let ast = engine
        .compile(source)
        .map_err(|err| anyhow::anyhow!("compile: {err}"))?;
    engine
        .eval_ast_with_scope(scope, &ast)
        .map_err(|err| anyhow::anyhow!("runtime: {err}"))
}

/// Compile `source` and call `fn_name` when that name is on the allowlist.
pub fn call_deterministic<A: rhai::FuncArgs>(
    sandbox: &ScriptSandbox,
    source: &str,
    fn_name: &str,
    args: A,
) -> Result<Dynamic> {
    if !sandbox.allowed_functions.contains_key(fn_name) {
        bail!("function {fn_name} is not on the capability allowlist");
    }
    if sandbox.execution_limits.max_operations == 0 {
        bail!("operation budget must be finite");
    }
    if source.len() > DEFAULT_MAX_SCRIPT_BYTES {
        bail!(
            "script is {} bytes, ceiling is {DEFAULT_MAX_SCRIPT_BYTES}",
            source.len()
        );
    }
    let engine = sandbox
        .engine
        .lock()
        .map_err(|err| anyhow::anyhow!("sandbox lock poisoned: {err}"))?;
    let ast: AST = engine
        .compile(source)
        .map_err(|err| anyhow::anyhow!("compile: {err}"))?;
    let mut scope = Scope::new();
    engine
        .call_fn(&mut scope, &ast, fn_name, args)
        .map_err(|err| anyhow::anyhow!("call {fn_name}: {err}"))
}

/// One draw in `0..bound` from the canonical [`SimRng`].
///
/// `StdRng` is not stable across `rand` releases, so this crate does not
/// construct one. The stream id is [`mix_sim_seed`]`(system_id, entity, draw)`,
/// then [`SimRng::new`]`(world_seed, tick, stream)`. The same six inputs always
/// produce the same `i64`.
pub fn roll_i64(
    world_seed: u64,
    tick: u64,
    system_id: u64,
    entity: u64,
    draw: u64,
    bound: i64,
) -> Result<i64> {
    if bound <= 0 || bound > MAX_SIM_ROLL_BOUND {
        bail!("sim_roll bound {bound} is outside 1..={MAX_SIM_ROLL_BOUND}");
    }
    let stream = mix_sim_seed(system_id, entity, draw);
    let mut rng = SimRng::new(world_seed, tick, stream);
    Ok(rng.random_range(0..bound))
}

/// One script replacement applied at a simulation tick.
///
/// `content_sha256` is SHA-256 of `source` bytes. Replay refuses a record
/// whose bytes do not match that hash.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScriptSwapRecord {
    pub tick: u64,
    pub path: String,
    pub content_sha256: [u8; 32],
    pub source: String,
}

impl ScriptSwapRecord {
    pub fn new(tick: u64, path: impl Into<String>, source: impl Into<String>) -> Self {
        let source = source.into();
        let content_sha256 = sha256_bytes(source.as_bytes());
        Self {
            tick,
            path: path.into(),
            content_sha256,
            source,
        }
    }

    pub fn hash_matches(&self) -> bool {
        sha256_bytes(self.source.as_bytes()) == self.content_sha256
    }
}

/// Ordered script swaps for one play session. Replay feeds this log back.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ScriptReplayLog {
    pub swaps: Vec<ScriptSwapRecord>,
}

impl ScriptReplayLog {
    pub fn record(&mut self, tick: u64, path: impl Into<String>, source: impl Into<String>) {
        self.swaps.push(ScriptSwapRecord::new(tick, path, source));
    }

    /// Swaps whose tick equals `tick`, in record order.
    pub fn at_tick(&self, tick: u64) -> impl Iterator<Item = &ScriptSwapRecord> {
        self.swaps.iter().filter(move |swap| swap.tick == tick)
    }
}

pub fn sha256_bytes(data: &[u8]) -> [u8; 32] {
    let digest = Sha256::digest(data);
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest);
    out
}

pub fn sha256_hex(data: &[u8]) -> String {
    hex::encode(sha256_bytes(data))
}

/// Bytes staged for the next tick boundary. Keys are paths, sorted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StagedScript {
    pub path: String,
    pub source: String,
    pub sha256: [u8; 32],
}

#[derive(Clone, Debug, Default)]
pub struct ScriptReloadQueue {
    pending: BTreeMap<String, StagedScript>,
}

impl ScriptReloadQueue {
    pub fn stage(&mut self, path: impl Into<String>, source: impl Into<String>) {
        let path = path.into();
        let source = source.into();
        let sha256 = sha256_bytes(source.as_bytes());
        self.pending.insert(
            path.clone(),
            StagedScript {
                path,
                source,
                sha256,
            },
        );
    }

    pub fn len(&self) -> usize {
        self.pending.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    /// Drain every staged script. Call this only at a tick boundary.
    pub fn take_boundary(&mut self) -> BTreeMap<String, StagedScript> {
        std::mem::take(&mut self.pending)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SemVer {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

impl SemVer {
    pub fn parse(text: &str) -> Result<Self> {
        let mut parts = text.split('.');
        let major = parse_ver_component(parts.next().unwrap_or(""), "major")?;
        let minor = parse_ver_component(parts.next().unwrap_or(""), "minor")?;
        let patch = parse_ver_component(parts.next().unwrap_or(""), "patch")?;
        if parts.next().is_some() {
            bail!("version {text} has more than three components");
        }
        Ok(Self {
            major,
            minor,
            patch,
        })
    }
}

fn parse_ver_component(text: &str, name: &str) -> Result<u64> {
    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
        bail!("version {name} component is not an integer");
    }
    if text.len() > 1 && text.starts_with('0') {
        bail!("version {name} component has a leading zero");
    }
    text.parse::<u64>()
        .with_context(|| format!("version {name} component overflows"))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModManifest {
    pub schema: u32,
    pub id: String,
    pub name: String,
    pub version: SemVer,
    pub api: u32,
    pub load_after: Vec<String>,
    pub load_before: Vec<String>,
    pub scripts: Vec<String>,
    pub weaves: Vec<String>,
    pub capabilities: BTreeSet<Capability>,
    pub budget: BudgetLimits,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoadedMod {
    pub manifest: ModManifest,
    pub scripts: BTreeMap<String, String>,
    /// Template TOML keyed by the manifest path. Parsed at load, not in the tick.
    pub weaves: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ManifestFile {
    schema: u32,
    id: String,
    name: String,
    version: String,
    api: u32,
    #[serde(default)]
    load_after: Vec<String>,
    #[serde(default)]
    load_before: Vec<String>,
    #[serde(default)]
    scripts: Vec<String>,
    #[serde(default)]
    weaves: Vec<String>,
    capabilities: Vec<String>,
    #[serde(default)]
    data: ManifestData,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct ManifestData {
    #[serde(default)]
    traps: i32,
    #[serde(default)]
    terrain_edits: i32,
    #[serde(default)]
    spawns: i32,
}

/// Parse a manifest from text. Does not touch the filesystem. Never panics on input.
pub fn parse_manifest_str(text: &str, ceilings: &EngineCeilings) -> Result<ModManifest> {
    if text.len() > ceilings.max_manifest_bytes {
        bail!(
            "manifest is {} bytes, ceiling is {}",
            text.len(),
            ceilings.max_manifest_bytes
        );
    }
    let file: ManifestFile = toml::from_str(text).context("manifest toml")?;
    validate_manifest_file(file, ceilings)
}

fn validate_manifest_file(file: ManifestFile, ceilings: &EngineCeilings) -> Result<ModManifest> {
    if file.schema != 1 {
        bail!("manifest schema {} is not 1", file.schema);
    }
    if file.api != MOD_API_VERSION {
        bail!(
            "manifest api {} does not match engine {MOD_API_VERSION}",
            file.api
        );
    }
    validate_mod_id(&file.id)?;
    if file.name.is_empty() || file.name.len() > 64 || file.name.chars().any(|c| c.is_control()) {
        bail!("manifest name is empty, too long, or contains a control character");
    }
    let version = SemVer::parse(&file.version)?;
    if file.scripts.len() > ceilings.max_scripts {
        bail!(
            "manifest lists {} scripts, ceiling is {}",
            file.scripts.len(),
            ceilings.max_scripts
        );
    }
    let mut seen_scripts = BTreeSet::new();
    for script in &file.scripts {
        validate_manifest_script_path(script)?;
        if !seen_scripts.insert(script.clone()) {
            bail!("duplicate script path {script}");
        }
    }
    if file.weaves.len() > ceilings.max_scripts {
        bail!(
            "manifest lists {} weave templates, ceiling is {}",
            file.weaves.len(),
            ceilings.max_scripts
        );
    }
    let mut seen_weaves = BTreeSet::new();
    for weave in &file.weaves {
        validate_manifest_weave_path(weave)?;
        if !seen_weaves.insert(weave.clone()) {
            bail!("duplicate weave path {weave}");
        }
    }
    if file.capabilities.is_empty() {
        bail!("capability allowlist is empty");
    }
    if file.capabilities.len() > 16 {
        bail!("capability allowlist is too long");
    }
    let mut capabilities = BTreeSet::new();
    for name in &file.capabilities {
        let cap = Capability::parse(name).with_context(|| format!("unknown capability {name}"))?;
        if !capabilities.insert(cap) {
            bail!("duplicate capability {name}");
        }
    }
    if file.load_after.len() > 32 || file.load_before.len() > 32 {
        bail!("load order list is too long");
    }
    for id in file.load_after.iter().chain(file.load_before.iter()) {
        validate_mod_id(id)?;
        if id == &file.id {
            bail!("mod {} depends on itself", file.id);
        }
    }
    let budget = BudgetLimits {
        traps: file.data.traps,
        terrain_edits: file.data.terrain_edits,
        spawns: file.data.spawns,
    };
    validate_budget(budget, ceilings)?;
    Ok(ModManifest {
        schema: file.schema,
        id: file.id,
        name: file.name,
        version,
        api: file.api,
        load_after: file.load_after,
        load_before: file.load_before,
        scripts: file.scripts,
        weaves: file.weaves,
        capabilities,
        budget,
    })
}

pub fn validate_mod_id(id: &str) -> Result<()> {
    if id.is_empty() || id.len() > 64 {
        bail!("mod id length must be 1..=64");
    }
    let mut chars = id.chars();
    let Some(first) = chars.next() else {
        bail!("mod id is empty");
    };
    if !first.is_ascii_lowercase() && !first.is_ascii_digit() {
        bail!("mod id {id} must start with a lowercase letter or digit");
    }
    if !chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '_' | '-'))
    {
        bail!("mod id {id} has an illegal character");
    }
    if id.contains("..") {
        bail!("mod id {id} contains '..'");
    }
    Ok(())
}

/// Relative manifest script path. No filesystem access.
pub fn validate_manifest_script_path(path: &str) -> Result<()> {
    if path.is_empty() || path.len() > 256 {
        bail!("script path length is invalid");
    }
    if path
        .chars()
        .any(|c| c.is_control() || c == '\\' || c == '\0')
    {
        bail!("script path contains a control character or backslash");
    }
    if !path.ends_with(".rhai") {
        bail!("script path must end with .rhai");
    }
    if path.starts_with('/') || path.starts_with('~') {
        bail!("script path must be relative");
    }
    let as_path = Path::new(path);
    if as_path.is_absolute() {
        bail!("script path must be relative");
    }
    for component in as_path.components() {
        match component {
            Component::Normal(part) => {
                let text = part.to_str().context("script path is not utf-8")?;
                if text == "." || text == ".." || text.contains(':') {
                    bail!("script path component {text} is not allowed");
                }
            }
            Component::CurDir => bail!("script path contains '.'"),
            Component::ParentDir => bail!("script path contains '..'"),
            Component::RootDir | Component::Prefix(_) => {
                bail!("script path must be relative");
            }
        }
    }
    Ok(())
}

/// Relative weave template path. Toml only. No filesystem access.
pub fn validate_manifest_weave_path(path: &str) -> Result<()> {
    if path.is_empty() || path.len() > 256 {
        bail!("weave path length is invalid");
    }
    if path
        .chars()
        .any(|c| c.is_control() || c == '\\' || c == '\0')
    {
        bail!("weave path contains a control character or backslash");
    }
    if !path.ends_with(".toml") {
        bail!("weave path must end with .toml");
    }
    if path.starts_with('/') || path.starts_with('~') {
        bail!("weave path must be relative");
    }
    let as_path = Path::new(path);
    if as_path.is_absolute() {
        bail!("weave path must be relative");
    }
    for component in as_path.components() {
        match component {
            Component::Normal(part) => {
                let text = part.to_str().context("weave path is not utf-8")?;
                if text == "." || text == ".." || text.contains(':') {
                    bail!("weave path component {text} is not allowed");
                }
            }
            Component::CurDir => bail!("weave path contains '.'"),
            Component::ParentDir => bail!("weave path contains '..'"),
            Component::RootDir | Component::Prefix(_) => {
                bail!("weave path must be relative");
            }
        }
    }
    Ok(())
}

pub fn read_capped(path: &Path, max_bytes: usize) -> Result<Vec<u8>> {
    let file = File::open(path).with_context(|| format!("open {}", path.display()))?;
    let mut buf = Vec::new();
    file.take(max_bytes as u64 + 1)
        .read_to_end(&mut buf)
        .with_context(|| format!("read {}", path.display()))?;
    if buf.len() > max_bytes {
        bail!(
            "{} is {} bytes, ceiling is {max_bytes}",
            path.display(),
            buf.len()
        );
    }
    Ok(buf)
}

/// Read a `.rhai` file that canonicalizes under `root`.
///
/// Absolute event paths (file watchers) are accepted only after `canonicalize`
/// still lands inside `root`. Relative paths are rejected when they contain
/// `..`, a prefix, or a backslash, before the join.
pub fn read_rhai_under_root(
    root: &Path,
    candidate: &Path,
    max_bytes: usize,
) -> Result<(PathBuf, String)> {
    if !root.is_dir() {
        bail!("mod root {} is not a directory", root.display());
    }
    let root_canon = root
        .canonicalize()
        .with_context(|| format!("canonicalize {}", root.display()))?;
    let joined = if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        let text = candidate.to_str().context("script path is not utf-8")?;
        validate_manifest_script_path(text)?;
        root.join(candidate)
    };
    let canon = joined
        .canonicalize()
        .with_context(|| format!("canonicalize {}", joined.display()))?;
    if !canon.starts_with(&root_canon) {
        bail!(
            "{} escapes mod root {}",
            candidate.display(),
            root.display()
        );
    }
    let ext = canon.extension().and_then(|ext| ext.to_str()).unwrap_or("");
    if !ext.eq_ignore_ascii_case("rhai") {
        bail!("{} is not a .rhai file", canon.display());
    }
    let bytes = read_capped(&canon, max_bytes)?;
    let source = String::from_utf8(bytes).context("script is not utf-8")?;
    Ok((canon, source))
}

pub fn load_mod_dir(root: &Path, ceilings: &EngineCeilings) -> Result<LoadedMod> {
    let manifest_path = root.join("mod.toml");
    let bytes = read_capped(&manifest_path, ceilings.max_manifest_bytes)?;
    let text = String::from_utf8(bytes).context("manifest is not utf-8")?;
    let manifest = parse_manifest_str(&text, ceilings)?;
    let mut scripts = BTreeMap::new();
    for rel in &manifest.scripts {
        let (_canon, source) =
            read_rhai_under_root(root, Path::new(rel), ceilings.max_script_bytes)?;
        // Key is the manifest path. The canonical path embeds the machine's
        // temp directory and is not a sim input.
        scripts.insert(rel.clone(), source);
    }
    let mut weaves = BTreeMap::new();
    for rel in &manifest.weaves {
        let (_canon, source) =
            read_text_under_root(root, Path::new(rel), ceilings.max_manifest_bytes)?;
        parse_template_toml(&source).map_err(|err| anyhow::anyhow!(err))?;
        weaves.insert(rel.clone(), source);
    }
    Ok(LoadedMod {
        manifest,
        scripts,
        weaves,
    })
}

fn read_text_under_root(
    root: &Path,
    candidate: &Path,
    max_bytes: usize,
) -> Result<(PathBuf, String)> {
    if !root.is_dir() {
        bail!("mod root {} is not a directory", root.display());
    }
    let root_canon = root
        .canonicalize()
        .with_context(|| format!("canonicalize {}", root.display()))?;
    let text = candidate.to_str().context("data path is not utf-8")?;
    validate_manifest_weave_path(text)?;
    let joined = root.join(candidate);
    let canon = joined
        .canonicalize()
        .with_context(|| format!("canonicalize {}", joined.display()))?;
    if !canon.starts_with(&root_canon) {
        bail!(
            "{} escapes mod root {}",
            candidate.display(),
            root.display()
        );
    }
    let ext = canon.extension().and_then(|ext| ext.to_str()).unwrap_or("");
    if !ext.eq_ignore_ascii_case("toml") {
        bail!("{} is not a .toml file", canon.display());
    }
    let bytes = read_capped(&canon, max_bytes)?;
    let source = String::from_utf8(bytes).context("file is not utf-8")?;
    Ok((canon, source))
}

/// Stable load order. Ties break by mod id. Cycles and missing deps are errors.
pub fn order_mods(mods: &[ModManifest]) -> Result<Vec<String>> {
    if mods.len() > DEFAULT_MAX_MODS {
        bail!("mod count {} exceeds {DEFAULT_MAX_MODS}", mods.len());
    }
    let mut ids = BTreeSet::new();
    for manifest in mods {
        if !ids.insert(manifest.id.clone()) {
            bail!("duplicate mod id {}", manifest.id);
        }
    }
    let mut indegree: BTreeMap<String, usize> = ids.iter().map(|id| (id.clone(), 0)).collect();
    let mut outgoing: BTreeMap<String, BTreeSet<String>> =
        ids.iter().map(|id| (id.clone(), BTreeSet::new())).collect();

    for manifest in mods {
        for dep in &manifest.load_after {
            add_edge(dep, &manifest.id, &ids, &mut indegree, &mut outgoing)?;
        }
        for later in &manifest.load_before {
            add_edge(&manifest.id, later, &ids, &mut indegree, &mut outgoing)?;
        }
    }

    let mut ready: BTreeSet<String> = indegree
        .iter()
        .filter(|(_, degree)| **degree == 0)
        .map(|(id, _)| id.clone())
        .collect();
    let mut ordered = Vec::with_capacity(ids.len());
    while let Some(id) = ready.iter().next().cloned() {
        ready.remove(&id);
        ordered.push(id.clone());
        let children = outgoing.get(&id).cloned().unwrap_or_default();
        for child in children {
            let degree = indegree
                .get_mut(&child)
                .context("load-order edge missing indegree")?;
            *degree = degree.saturating_sub(1);
            if *degree == 0 {
                ready.insert(child);
            }
        }
    }
    if ordered.len() != ids.len() {
        bail!("mod load order contains a cycle");
    }
    Ok(ordered)
}

fn add_edge(
    earlier: &str,
    later: &str,
    ids: &BTreeSet<String>,
    indegree: &mut BTreeMap<String, usize>,
    outgoing: &mut BTreeMap<String, BTreeSet<String>>,
) -> Result<()> {
    if !ids.contains(earlier) || !ids.contains(later) {
        bail!("load order names missing mod {earlier} or {later}");
    }
    let inserted = outgoing
        .get_mut(earlier)
        .context("load-order node missing")?
        .insert(later.to_string());
    if inserted {
        let degree = indegree
            .get_mut(later)
            .context("load-order indegree missing")?;
        *degree = degree.saturating_add(1);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn manifest(body: &str) -> String {
        format!(
            r#"
schema = 1
id = "demo.mod"
name = "Demo"
version = "1.2.3"
api = 1
capabilities = ["configure"]
scripts = ["scripts/configure.rhai"]
{body}
"#
        )
    }

    #[test]
    fn sandbox_rejects_filesystem_network_process_and_clock() {
        let engine = SandboxEngine::build(&SandboxProfile::author()).into_script_sandbox();
        let mut scope = Scope::new();
        for source in [
            r#"import "std";"#,
            r#"eval("1 + 1")"#,
            r#"timestamp()"#,
            r#"sleep(1)"#,
            r#"http_get("http://127.0.0.1/")"#,
            r#"exec("id")"#,
            r#"spawn("sh")"#,
        ] {
            let err = execute_deterministic(&engine, source, &mut scope);
            assert!(err.is_err(), "{source} should fail, got {err:?}");
        }
    }

    #[test]
    fn operation_budget_stops_a_loop() {
        let mut profile = SandboxProfile::author();
        profile.ceilings.max_operations = 50;
        let engine = SandboxEngine::build(&profile).into_script_sandbox();
        let mut scope = Scope::new();
        let err = execute_deterministic(&engine, "loop { }", &mut scope);
        assert!(err.is_err(), "loop must exhaust the operation budget");
    }

    #[test]
    fn string_cap_rejects_oversized_literal() {
        let mut profile = SandboxProfile::author();
        profile.ceilings.max_string_size = 8;
        let engine = SandboxEngine::build(&profile).into_script_sandbox();
        let mut scope = Scope::new();
        let err = execute_deterministic(&engine, r#"let s = "0123456789";"#, &mut scope);
        assert!(err.is_err(), "string over the cap must fail");
    }

    #[test]
    fn empty_allowlist_is_refused() {
        let sandbox = ScriptSandbox {
            engine: std::sync::Arc::new(std::sync::Mutex::new(Engine::new_raw())),
            allowed_functions: std::collections::HashMap::new(),
            execution_limits: ExecutionLimits {
                max_operations: 10,
                max_memory_bytes: 10,
                timeout_ms: 0,
            },
        };
        let mut scope = Scope::new();
        let err = execute_deterministic(&sandbox, "1 + 1", &mut scope);
        assert!(err.is_err());
    }

    #[test]
    fn call_requires_allowlisted_name() {
        let sandbox = SandboxEngine::build(&SandboxProfile::author()).into_script_sandbox();
        let err = call_deterministic::<()>(&sandbox, "fn secret() { 1 }", "secret", ());
        assert!(err.is_err());
        let value = call_deterministic(
            &sandbox,
            r#"fn configure(meta) { meta.width }"#,
            "configure",
            ({
                let mut map = rhai::Map::new();
                map.insert("width".into(), Dynamic::from(4_i64));
                map
            },),
        )
        .expect("configure is allowlisted");
        assert_eq!(value.as_int().expect("int"), 4);
    }

    #[test]
    fn roll_uses_canonical_sim_rng_and_is_bounded() {
        let a = roll_i64(9, 3, 0x4D4F_4453, 1, 0, 4).unwrap();
        let b = roll_i64(9, 3, 0x4D4F_4453, 1, 0, 4).unwrap();
        assert_eq!(a, b);
        assert!((0..4).contains(&a));
        assert_ne!(
            roll_i64(9, 3, 0x4D4F_4453, 1, 0, 1_000_000).unwrap(),
            roll_i64(9, 3, 0x4D4F_4453, 2, 0, 1_000_000).unwrap(),
            "entity id selects a different SimRng stream"
        );
        assert!(roll_i64(9, 0, 1, 0, 0, 0).is_err());
        assert!(roll_i64(9, 0, 1, 0, 0, -1).is_err());
    }

    #[test]
    fn manifest_rejects_traversal_and_bad_version() {
        let ceilings = EngineCeilings::default();
        let err = parse_manifest_str(&manifest("scripts = [\"../secret.rhai\"]\n"), &ceilings);
        assert!(err.is_err(), "{err:?}");
        let err = parse_manifest_str(&manifest("version = \"01.2.3\"\n"), &ceilings);
        // version is in the header; override by a full document
        let _ = err;
        let err = parse_manifest_str(
            r#"
schema = 1
id = "demo.mod"
name = "Demo"
version = "01.2.3"
api = 1
capabilities = ["configure"]
scripts = ["a.rhai"]
"#,
            &ceilings,
        );
        assert!(err.is_err());
        let err = parse_manifest_str(
            r#"
schema = 1
id = "demo.mod"
name = "Demo"
version = "1.2.3"
api = 1
capabilities = ["shell"]
scripts = ["a.rhai"]
"#,
            &ceilings,
        );
        assert!(err.is_err());
    }

    #[test]
    fn budget_ceiling_rejects_oversize_data() {
        let ceilings = EngineCeilings::default();
        let err = parse_manifest_str(
            r#"
schema = 1
id = "demo.mod"
name = "Demo"
version = "1.0.0"
api = 1
capabilities = ["configure"]
scripts = ["a.rhai"]
[data]
traps = 100000
"#,
            &ceilings,
        );
        assert!(err.is_err());
    }

    #[test]
    fn load_order_is_stable_and_rejects_cycles() {
        let ceilings = EngineCeilings::default();
        let a = parse_manifest_str(
            r#"
schema = 1
id = "b.mod"
name = "B"
version = "1.0.0"
api = 1
capabilities = ["sim_rng"]
load_after = ["a.mod"]
scripts = []
"#,
            &ceilings,
        )
        .unwrap();
        let b = parse_manifest_str(
            r#"
schema = 1
id = "a.mod"
name = "A"
version = "1.0.0"
api = 1
capabilities = ["configure"]
scripts = []
"#,
            &ceilings,
        )
        .unwrap();
        assert_eq!(
            order_mods(&[a.clone(), b.clone()]).unwrap(),
            vec!["a.mod".to_string(), "b.mod".to_string()]
        );
        assert_eq!(
            order_mods(&[b, a]).unwrap(),
            vec!["a.mod".to_string(), "b.mod".to_string()]
        );
        let cycle_a = parse_manifest_str(
            r#"
schema = 1
id = "a.mod"
name = "A"
version = "1.0.0"
api = 1
capabilities = ["configure"]
load_after = ["b.mod"]
scripts = []
"#,
            &ceilings,
        )
        .unwrap();
        let cycle_b = parse_manifest_str(
            r#"
schema = 1
id = "b.mod"
name = "B"
version = "1.0.0"
api = 1
capabilities = ["configure"]
load_after = ["a.mod"]
scripts = []
"#,
            &ceilings,
        )
        .unwrap();
        assert!(order_mods(&[cycle_a, cycle_b]).is_err());
    }

    #[test]
    fn read_rhai_rejects_escape() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("mod");
        std::fs::create_dir(&root).unwrap();
        let outside = temp.path().join("outside.rhai");
        std::fs::write(&outside, "let x = 1;").unwrap();
        let err = read_rhai_under_root(&root, &outside, 1024);
        assert!(err.is_err(), "{err:?}");

        let inside = root.join("ok.rhai");
        let mut file = std::fs::File::create(&inside).unwrap();
        writeln!(file, "let x = 1;").unwrap();
        let (_path, source) = read_rhai_under_root(&root, Path::new("ok.rhai"), 1024).unwrap();
        assert!(source.contains("let x = 1;"));

        let err = read_rhai_under_root(&root, Path::new("../outside.rhai"), 1024);
        assert!(err.is_err());
    }

    #[test]
    fn reload_queue_apply_is_sorted_and_not_mid_write() {
        let mut queue = ScriptReloadQueue::default();
        queue.stage("b.rhai", "b");
        queue.stage("a.rhai", "a1");
        queue.stage("a.rhai", "a2");
        let taken = queue.take_boundary();
        let keys: Vec<_> = taken.keys().cloned().collect();
        assert_eq!(keys, vec!["a.rhai".to_string(), "b.rhai".to_string()]);
        assert_eq!(taken["a.rhai"].source, "a2");
        assert!(queue.is_empty());
    }

    #[test]
    fn sim_sandbox_keeps_the_op_budget_and_no_wall_clock() {
        let ceilings = EngineCeilings::default();
        let structural = ceilings
            .max_string_size
            .saturating_mul(ceilings.max_variables)
            .saturating_add(ceilings.max_array_size.saturating_mul(16))
            .saturating_add(ceilings.max_map_size.saturating_mul(64));
        assert_eq!(ceilings.advertised_memory_bytes(), structural);
        let sandbox = SandboxEngine::build(&SandboxProfile::sim()).into_script_sandbox();
        assert_eq!(sandbox.execution_limits.timeout_ms, 0);
        assert_eq!(
            sandbox.execution_limits.max_operations,
            DEFAULT_MAX_OPERATIONS
        );
        assert_eq!(sandbox.execution_limits.max_memory_bytes, structural);
        assert!(sandbox.allowed_functions.contains_key("queue_weave"));
        assert!(sandbox.allowed_functions.contains_key("sim_roll"));
        assert!(!sandbox.allowed_functions.contains_key("raycast"));
        assert!(!sandbox.allowed_functions.contains_key("find_path"));
    }

    #[test]
    fn queue_weave_is_a_weave_command_and_weaves_do_not_spend_terrain() {
        assert!(COMMANDS.contains(&"queue_weave"));
        let mut previous = "";
        for name in COMMANDS {
            assert!(previous < *name, "COMMANDS must stay sorted");
            previous = name;
        }
        assert_eq!(MOD_COMMAND_API_VERSION, 2);
        assert_eq!(Capability::Weave.command_names(), &["queue_weave"][..]);
        let ceilings = EngineCeilings::default();
        assert_eq!(ceilings.max_weaves, DEFAULT_MAX_WEAVES);
        assert_eq!(DEFAULT_MAX_WEAVES, 64);
        let budget = BudgetLimits {
            traps: 0,
            terrain_edits: 0,
            spawns: 0,
        };
        validate_budget(budget, &ceilings).unwrap();
        assert_eq!(budget.terrain_edits, 0);
    }

    #[test]
    fn load_mod_dir_keys_are_manifest_paths() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        std::fs::create_dir(root.join("scripts")).unwrap();
        std::fs::write(root.join("scripts/tick.rhai"), "moves.move_to(1, 0);\n").unwrap();
        std::fs::write(
            root.join("mod.toml"),
            r#"
schema = 1
id = "demo.mod"
name = "Demo"
version = "1.0.0"
api = 1
capabilities = ["move_entity"]
scripts = ["scripts/tick.rhai"]
"#,
        )
        .unwrap();
        let loaded = load_mod_dir(root, &EngineCeilings::default()).unwrap();
        assert_eq!(
            loaded.scripts.keys().cloned().collect::<Vec<_>>(),
            vec!["scripts/tick.rhai".to_string()]
        );
        assert!(loaded.scripts["scripts/tick.rhai"].contains("move_to"));
    }

    #[test]
    fn path_dotdot_helper_rejects() {
        assert!(validate_manifest_script_path("a/../../etc/passwd").is_err());
        assert!(validate_manifest_script_path("/etc/passwd").is_err());
        assert!(validate_manifest_script_path("scripts/ok.rhai").is_ok());
        assert!(validate_manifest_script_path("scripts\\ok.rhai").is_err());
    }
}
