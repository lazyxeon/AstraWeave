//! aw_trace_sync — deterministic architecture-trace registry/map synchronizer.
//!
//! Trace front-matter (YAML at the top of each `docs/architecture/<system>.md`) is
//! the single source of truth for trace↔crate linkage + status. This tool regenerates
//! the marker-bounded trace table in `CLAUDE.md` and the per-crate `trace` links in
//! `docs/architecture/workspace_map.html` from that front-matter, and gates drift via
//! `--check`. Design: `docs/architecture/_meta/TRACE_SYNC_PROPOSAL.md`.
//!
//! Scope: front-matter contract + validation + `--list-untraced` + CLAUDE.md trace
//! table generation + map `trace`-link sync (v1). v1.1 adds map status sync (the
//! primary crate's `status`/`statusCategory`/`statusEvidence`, driven by the owning
//! trace) and `runtime_edges` (asserted runtime-integration edges, highlighted on the
//! map). Cargo metadata now owns the node set and normal path-dependency edges,
//! including the examples aggregate. `--check` fails when that topology, a trace
//! section anchor, or an owned crate with no node drifts.

use anyhow::{bail, Context, Result};
use clap::Parser;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

const TABLE_START: &str = "<!-- TRACE-TABLE:START -->";
const TABLE_END: &str = "<!-- TRACE-TABLE:END -->";
const MAP_ELEMENT_ANCHOR: &str = "id=\"workspace-data\"";
const FOOTER_START: &str = "<!-- TRACE-FOOTER:START -->";
const FOOTER_END: &str = "<!-- TRACE-FOOTER:END -->";
const COUNTER_START: &str = "<!-- MAP-COUNTER:START -->";
const COUNTER_END: &str = "<!-- MAP-COUNTER:END -->";
const TRACE_COUNT_START: &str = "<!-- MAP-TRACE-COUNT:START -->";
const TRACE_COUNT_END: &str = "<!-- MAP-TRACE-COUNT:END -->";
const MEMBERS_START: &str = "<!-- MAP-MEMBERS:START -->";
const MEMBERS_END: &str = "<!-- MAP-MEMBERS:END -->";

mod topology;

use topology::{
    anchor_problems, apply_runtime_edges, count_nodes_edges, footer_block, members_sentence,
    node_trace_files, owned_without_node, replace_marked, sync_examples_oneliner,
    sync_reconciliation, sync_topology, workspace_graph, Graph,
};

#[derive(Parser, Debug)]
#[command(
    name = "aw_trace_sync",
    about = "Deterministic architecture-trace registry/map synchronizer (front-matter is the source of truth)."
)]
struct Cli {
    /// Validate front-matter + references only (no generation, no diff).
    #[arg(long)]
    validate_only: bool,
    /// List workspace crates that no trace claims via `owns`.
    #[arg(long)]
    list_untraced: bool,
    /// Regenerate the CLAUDE.md trace table + workspace_map.html trace links in place.
    #[arg(long)]
    write: bool,
    /// Exit nonzero if `--write` would change anything (CI gate); prints what would change.
    #[arg(long)]
    check: bool,
    /// Repo root. Defaults to the cargo workspace root.
    #[arg(long)]
    root: Option<PathBuf>,
}

// ---- front-matter schema -----------------------------------------------------

#[derive(Debug, Deserialize, PartialEq, Eq, Clone, Copy)]
#[serde(rename_all = "kebab-case")]
enum Domain {
    Core,
    Ai,
    Rendering,
    PhysicsWorld,
    Gameplay,
    Networking,
    Tools,
}

#[derive(Debug, Deserialize, PartialEq, Eq, Clone, Copy)]
#[serde(rename_all = "snake_case")]
enum Lifecycle {
    Active,
    InDesign,
    Dormant,
    Deprecated,
    Unknown,
}

#[derive(Debug, Deserialize, PartialEq, Eq, Clone, Copy)]
#[serde(rename_all = "snake_case")]
enum Integration {
    Wired,
    Partial,
    ExampleOnly,
    TestOnly,
    Dormant,
    Mixed,
    Unknown,
}

/// An asserted runtime-integration edge. Direction is `from` → `to`, matching
/// the map ("source depends on target"). `from` defaults to `primary_crate`.
/// This is not a cargo dependency: the map stores it as `runtime` on a cargo
/// edge, or as `kind: runtime` when no cargo edge exists.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RuntimeEdge {
    /// Defaults to the trace's `primary_crate` when omitted.
    #[serde(default)]
    from: Option<String>,
    to: String,
    #[serde(default)]
    note: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FrontMatter {
    schema_version: u32,
    trace_id: String,
    #[allow(dead_code)]
    title: String,
    description: String,
    primary_crate: String,
    domain: Domain,
    lifecycle_status: Lifecycle,
    integration_status: Integration,
    /// One-line status evidence; synced to the map node's `statusEvidence` (v1.1).
    #[serde(default)]
    summary: Option<String>,
    #[serde(default)]
    owns: Vec<String>,
    #[allow(dead_code)]
    doc_version: String,
    #[allow(dead_code)]
    last_verified_commit: String,
    /// Secondary crates this trace cross-references (links only; never status owners).
    #[serde(default)]
    #[allow(dead_code)]
    also_documents: Vec<String>,
    /// Crate references that intentionally live outside the cargo workspace.
    #[serde(default)]
    external: Vec<String>,
    /// Asserted runtime-integration edges from `primary_crate` (v1.1).
    #[serde(default)]
    runtime_edges: Vec<RuntimeEdge>,
}

struct TraceDoc {
    /// filename stem (e.g. "water" for water.md)
    stem: String,
    fm: Option<FrontMatter>,
    fm_error: Option<String>,
    /// Full markdown, used for trace-section anchor checks.
    body: String,
}

impl TraceDoc {
    fn file(&self) -> String {
        format!("{}.md", self.stem)
    }
}

// ---- loading -----------------------------------------------------------------

/// Extract the YAML between leading `---` fences. Handles LF and CRLF line endings.
fn extract_front_matter(content: &str) -> Option<String> {
    let mut lines = content.lines();
    if lines.next()?.trim_end() != "---" {
        return None;
    }
    let mut yaml = String::new();
    for line in lines {
        if line.trim_end() == "---" {
            return Some(yaml);
        }
        yaml.push_str(line);
        yaml.push('\n');
    }
    None // unterminated front-matter
}

fn load_traces(arch_dir: &Path) -> Result<Vec<TraceDoc>> {
    let mut out = Vec::new();
    let mut entries: Vec<PathBuf> = std::fs::read_dir(arch_dir)
        .with_context(|| format!("reading {}", arch_dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_file() && p.extension().and_then(|e| e.to_str()) == Some("md"))
        .collect();
    entries.sort();
    for path in entries {
        let stem = path.file_stem().unwrap().to_string_lossy().to_string();
        // ARCHITECTURE_MAP.md is the dependency map, not a subsystem trace.
        if stem == "ARCHITECTURE_MAP" {
            continue;
        }
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        let (fm, fm_error) = match extract_front_matter(&content) {
            None => (None, Some("no YAML front-matter block".to_string())),
            Some(yaml) => match yaml_serde::from_str::<FrontMatter>(&yaml) {
                Ok(fm) => (Some(fm), None),
                Err(e) => (None, Some(format!("invalid front-matter: {e}"))),
            },
        };
        out.push(TraceDoc {
            stem,
            fm,
            fm_error,
            body: content,
        });
    }
    Ok(out)
}

fn domain_id(d: Domain) -> &'static str {
    match d {
        Domain::Core => "core",
        Domain::Ai => "ai",
        Domain::Rendering => "rendering",
        Domain::PhysicsWorld => "physics-world",
        Domain::Gameplay => "gameplay",
        Domain::Networking => "networking",
        Domain::Tools => "tools",
    }
}

// ---- validation --------------------------------------------------------------

/// Returns the list of validation problems (empty == valid).
fn validate(traces: &[TraceDoc], crates: &[String]) -> Vec<String> {
    let mut problems = Vec::new();
    let crate_set: std::collections::BTreeSet<&str> = crates.iter().map(|s| s.as_str()).collect();

    // trace_id uniqueness + stem agreement + schema/reference checks
    let mut seen_ids: BTreeMap<String, String> = BTreeMap::new();
    // crate -> trace that owns it (detect double ownership)
    let mut owner: BTreeMap<String, String> = BTreeMap::new();

    for t in traces {
        let file = t.file();
        let Some(fm) = &t.fm else {
            problems.push(format!(
                "{file}: {}",
                t.fm_error.as_deref().unwrap_or("missing front-matter")
            ));
            continue;
        };
        if fm.schema_version != 1 {
            problems.push(format!(
                "{file}: schema_version {} unsupported (expected 1)",
                fm.schema_version
            ));
        }
        if fm.trace_id != t.stem {
            problems.push(format!(
                "{file}: trace_id '{}' does not match filename stem '{}'",
                fm.trace_id, t.stem
            ));
        }
        if !is_plain_trace_name(&fm.trace_id) {
            problems.push(format!(
                "{file}: trace_id must match lowercase letters, digits, and underscores"
            ));
        }
        if !is_plain_trace_name(&t.stem) {
            problems.push(format!(
                "{file}: file name must match lowercase letters, digits, and underscores"
            ));
        }
        if let Some(prev) = seen_ids.insert(fm.trace_id.clone(), file.clone()) {
            problems.push(format!(
                "duplicate trace_id '{}' in {file} and {prev}",
                fm.trace_id
            ));
        }
        let external: std::collections::BTreeSet<&str> =
            fm.external.iter().map(|s| s.as_str()).collect();
        let resolves =
            |c: &str| crate_set.contains(c) || external.contains(c) || c == "design-only";
        if !resolves(&fm.primary_crate) {
            problems.push(format!(
                "{file}: primary_crate '{}' is not a workspace crate (add to `external` if intentional)",
                fm.primary_crate
            ));
        }
        for c in &fm.also_documents {
            if !resolves(c) {
                problems.push(format!(
                    "{file}: also_documents '{c}' is not a workspace crate"
                ));
            }
        }
        for c in &fm.owns {
            if !resolves(c) {
                problems.push(format!(
                    "{file}: owns '{c}' is not a workspace crate (add to `external` if intentional)"
                ));
            }
            if let Some(prev) = owner.insert(c.clone(), fm.trace_id.clone()) {
                problems.push(format!(
                    "crate '{c}' is owned by two traces: '{}' and '{prev}' (each crate may be owned by at most one trace)",
                    fm.trace_id
                ));
            }
        }
        for e in &fm.runtime_edges {
            let from = e.from.as_deref().unwrap_or(fm.primary_crate.as_str());
            if !resolves(from) {
                problems.push(format!(
                    "{file}: runtime_edges from '{from}' is not a workspace crate"
                ));
            }
            if !resolves(&e.to) {
                problems.push(format!(
                    "{file}: runtime_edges target '{}' is not a workspace crate",
                    e.to
                ));
            }
            if from == e.to {
                problems.push(format!(
                    "{file}: runtime_edges {from} -> {} is a self-edge",
                    e.to
                ));
            }
        }
    }
    problems
}

fn is_plain_trace_name(s: &str) -> bool {
    !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

// ---- generation: CLAUDE.md trace table ---------------------------------------

fn gen_trace_table(traces: &[TraceDoc]) -> String {
    let mut rows: Vec<(&str, &str)> = traces
        .iter()
        .filter_map(|t| {
            t.fm.as_ref()
                .map(|fm| (fm.trace_id.as_str(), fm.description.as_str()))
        })
        .collect();
    rows.sort_by(|a, b| a.0.cmp(b.0));
    let mut s = String::new();
    s.push_str(TABLE_START);
    s.push('\n');
    s.push_str("| Trace | Subsystem |\n");
    s.push_str("|---|---|\n");
    for (id, desc) in rows {
        s.push_str(&format!("| `docs/architecture/{id}.md` | {desc} |\n"));
    }
    s.push_str(TABLE_END);
    s
}

/// Replace the content between the table markers (inclusive) with freshly generated content.
fn render_claude_md(current: &str, traces: &[TraceDoc]) -> Result<String> {
    let start = current
        .find(TABLE_START)
        .with_context(|| format!("`{TABLE_START}` marker not found in CLAUDE.md"))?;
    let end_marker = current
        .find(TABLE_END)
        .with_context(|| format!("`{TABLE_END}` marker not found in CLAUDE.md"))?;
    let end = end_marker + TABLE_END.len();
    let mut block = gen_trace_table(traces);
    // Match the file's line-ending convention so generated output is byte-stable on
    // both CRLF (Windows working tree) and LF (CI / committed) checkouts.
    if current.contains("\r\n") {
        block = block.replace('\n', "\r\n");
    }
    Ok(format!("{}{}{}", &current[..start], block, &current[end..]))
}

// ---- generation: map trace links ---------------------------------------------

/// Escape every non-ASCII char to \uXXXX (matching the existing blob's ensure_ascii style)
/// and every `<` to `\u003c`. A string such as `</script>` must not be able to close the
/// surrounding `<script id="workspace-data">` element. `--check` fails if that block
/// still contains a raw `<`.
fn escape_nonascii(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        let cp = c as u32;
        if c == '<' {
            out.push_str("\\u003c");
        } else if cp < 0x80 {
            out.push(c);
        } else if cp <= 0xFFFF {
            out.push_str(&format!("\\u{:04x}", cp));
        } else {
            let v = cp - 0x10000;
            out.push_str(&format!(
                "\\u{:04x}\\u{:04x}",
                0xD800 + (v >> 10),
                0xDC00 + (v & 0x3FF)
            ));
        }
    }
    out
}

/// crate -> owning trace filename ("water.md")
fn ownership_map(traces: &[TraceDoc]) -> BTreeMap<String, String> {
    let mut m = BTreeMap::new();
    for t in traces {
        if let Some(fm) = &t.fm {
            for c in &fm.owns {
                m.insert(c.clone(), t.file());
            }
        }
    }
    m
}

/// Map (lifecycle, integration) -> the map's (status, statusCategory) pair, using only
/// the values the map already styles {active, dormant/in-design, partial}. Returns None
/// for unknown inputs (leave the node's curated status untouched). (v1.1)
fn status_pair(lc: Lifecycle, ig: Integration) -> Option<(&'static str, &'static str)> {
    if lc == Lifecycle::Unknown || ig == Integration::Unknown {
        return None; // leave the node's curated status untouched
    }
    match (lc, ig) {
        (Lifecycle::Active, Integration::Wired) => Some(("active", "active")),
        (Lifecycle::Active, Integration::Partial) | (Lifecycle::Active, Integration::Mixed) => {
            Some(("partial", "partial"))
        }
        // active-but-example/test/dormant integration, or any non-active lifecycle.
        _ => Some(("dormant", "in-design")),
    }
}

/// Per-primary-crate status to apply: the mapped (status, statusCategory) pair (None =
/// leave curated) plus an optional statusEvidence override from front-matter `summary`.
type StatusEntry = (Option<(&'static str, &'static str)>, Option<String>);

struct MapRender {
    html: String,
    changed_links: Vec<(String, Option<String>, Option<String>)>, // (crate, old, new)
    changed_status: Vec<(String, String, String)>,                // (crate, old_status, new_status)
    /// Integrity, anchor, and nodeless-owner problems. Non-empty => nothing may be written.
    problems: Vec<String>,
    missing_nodes: Vec<String>,
    extra_nodes: Vec<String>,
    missing_edges: Vec<String>,
    extra_edges: Vec<String>,
    optional_changes: Vec<String>,
    example_list: Vec<String>,
    loc_changes: Vec<String>,
    chrome_changes: Vec<String>,
    /// Runtime flags or notes that differ from the input map. Unchanged edges are omitted.
    runtime_changes: Vec<String>,
    /// The committed workspace-data block contains a raw `<`. `--write` escapes it.
    raw_script_lt: bool,
}

/// Referential integrity of the map's `workspace-data`.
///
/// The viewer passes every edge to `cytoscape()`, which throws on an edge whose endpoint is
/// not a node. The throw aborts the page script and the map renders zero nodes, so every
/// edge endpoint must be a node and every node domain must be a declared domain.
fn map_integrity(data: &serde_json::Value) -> Vec<String> {
    let mut problems = Vec::new();
    let nodes: &[serde_json::Value] = data
        .get("nodes")
        .and_then(|n| n.as_array())
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let edges: &[serde_json::Value] = data
        .get("edges")
        .and_then(|e| e.as_array())
        .map(Vec::as_slice)
        .unwrap_or(&[]);

    let mut ids: BTreeSet<&str> = BTreeSet::new();
    for n in nodes {
        match n.get("id").and_then(|v| v.as_str()) {
            Some(id) => {
                if !ids.insert(id) {
                    problems.push(format!("map: duplicate node id '{id}'"));
                }
            }
            None => problems.push("map: a node has no string `id`".to_string()),
        }
    }

    match data.get("domains").and_then(|d| d.as_array()) {
        Some(domains) => {
            let declared: BTreeSet<&str> = domains
                .iter()
                .filter_map(|d| d.get("id").and_then(|v| v.as_str()))
                .collect();
            for n in nodes {
                let id = n.get("id").and_then(|v| v.as_str()).unwrap_or("?");
                match n.get("domain").and_then(|v| v.as_str()) {
                    Some(d) if declared.contains(d) => {}
                    Some(d) => problems.push(format!(
                        "map: node '{id}' has domain '{d}', which is not in `domains`"
                    )),
                    None => problems.push(format!("map: node '{id}' has no `domain`")),
                }
            }
        }
        None => problems.push("map: workspace-data has no `domains` array".to_string()),
    }

    for (i, e) in edges.iter().enumerate() {
        let s = e.get("source").and_then(|v| v.as_str()).unwrap_or("");
        let t = e.get("target").and_then(|v| v.as_str()).unwrap_or("");
        for (role, end) in [("source", s), ("target", t)] {
            if !ids.contains(end) {
                problems.push(format!(
                    "map: edge #{i} {s} -> {t}: {role} '{end}' is not a node"
                ));
            }
        }
    }
    problems
}

fn render_map(current: &str, traces: &[TraceDoc], graph: &Graph, root: &Path) -> Result<MapRender> {
    render_map_inner(current, traces, graph, root, true)
}

fn render_map_inner(
    current: &str,
    traces: &[TraceDoc],
    graph: &Graph,
    root: &Path,
    chrome: bool,
) -> Result<MapRender> {
    let anchor = current
        .find(MAP_ELEMENT_ANCHOR)
        .with_context(|| format!("`{MAP_ELEMENT_ANCHOR}` not found in workspace_map.html"))?;
    let gt = current[anchor..]
        .find('>')
        .context("malformed workspace-data <script> tag")?
        + anchor;
    let content_start = gt + 1;
    let close = current[content_start..]
        .find("</script>")
        .context("workspace-data </script> not found")?
        + content_start;
    let json_str = &current[content_start..close];
    let raw_script_lt = json_str.contains('<');

    let mut data: serde_json::Value =
        serde_json::from_str(json_str).context("parsing workspace-data JSON")?;
    let runtime_before = runtime_snapshot(&data);
    let topo = sync_topology(&mut data, graph, root, traces)?;
    let owners = ownership_map(traces);

    // primary_crate -> (mapped status pair, statusEvidence-from-summary); + runtime edges
    let mut prim: BTreeMap<String, StatusEntry> = BTreeMap::new();
    // (trace file, from, to, note)
    let mut rt_edges: Vec<(String, String, String, Option<String>)> = Vec::new();
    for t in traces {
        if let Some(fm) = &t.fm {
            // A trace drives its primary crate's status only when it OWNS that crate.
            // Slice/cross-cutting traces (primary_crate not in `owns`, e.g. terrain_materials,
            // animation) describe a crate owned by another trace and must not drive its status.
            if fm.owns.iter().any(|c| c == &fm.primary_crate) {
                prim.insert(
                    fm.primary_crate.clone(),
                    (
                        status_pair(fm.lifecycle_status, fm.integration_status),
                        fm.summary.clone(),
                    ),
                );
            }
            for e in &fm.runtime_edges {
                let from = e.from.clone().unwrap_or_else(|| fm.primary_crate.clone());
                rt_edges.push((t.file(), from, e.to.clone(), e.note.clone()));
            }
        }
    }

    let mut changed_links = Vec::new();
    let mut changed_status = Vec::new();
    {
        let nodes = data
            .get_mut("nodes")
            .and_then(|n| n.as_array_mut())
            .context("workspace-data has no `nodes` array")?;
        for node in nodes.iter_mut() {
            let Some(id) = node
                .get("id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
            else {
                continue;
            };
            let want: Option<String> = owners.get(&id).cloned();
            let have: Option<String> = node
                .get("trace")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            if have != want {
                changed_links.push((id.clone(), have, want.clone()));
            }
            let obj = node.as_object_mut().context("node is not an object")?;
            obj.insert(
                "trace".to_string(),
                want.map(serde_json::Value::String)
                    .unwrap_or(serde_json::Value::Null),
            );
            // Status sync — primary crates only (secondary owned crates keep curated status).
            if let Some((pair, summary)) = prim.get(&id) {
                if let Some((status, cat)) = pair {
                    let old = obj
                        .get("status")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    if old != *status {
                        changed_status.push((id.clone(), old, status.to_string()));
                    }
                    obj.insert(
                        "status".into(),
                        serde_json::Value::String(status.to_string()),
                    );
                    obj.insert(
                        "statusCategory".into(),
                        serde_json::Value::String(cat.to_string()),
                    );
                }
                // statusEvidence is a soft sync: only set when front-matter provides a summary
                // (so curated evidence is never wiped for traces without one).
                if let Some(ev) = summary {
                    obj.insert(
                        "statusEvidence".into(),
                        serde_json::Value::String(ev.clone()),
                    );
                }
            }
        }
    }

    // Runtime edges are applied after cargo edges exist. A pair that is also a
    // cargo dep keeps its cargo kind and gains `runtime` / `runtimeNote`. A pair
    // with no cargo dep is kind `runtime`, so it cannot be read as a dependency.
    let mut problems = apply_runtime_edges(&mut data, &rt_edges)?;
    let node_ids: BTreeSet<String> = data
        .get("nodes")
        .and_then(|n| n.as_array())
        .map(|ns| {
            ns.iter()
                .filter_map(|n| n.get("id").and_then(|v| v.as_str()).map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    problems.extend(owned_without_node(traces, &node_ids, graph));
    if chrome {
        problems.extend(anchor_problems(current, traces, &node_trace_files(&data)));
    }
    problems.extend(map_integrity(&data));

    let (node_count, edge_count) = count_nodes_edges(&data);
    sync_reconciliation(&mut data, graph);
    // Counts were taken before runtime-only edges were appended. Refresh them.
    if let Some(meta) = data.get_mut("metadata").and_then(|m| m.as_object_mut()) {
        meta.insert(
            "renderedNodeCount".into(),
            serde_json::Value::from(node_count),
        );
        meta.insert(
            "renderedEdgeCount".into(),
            serde_json::Value::from(edge_count),
        );
        meta.insert("mapNodeClaim".into(), serde_json::Value::from(node_count));
        // cargoEdgeCount stays the cargo-derived count from sync_topology.
        // renderedEdgeCount includes kind:runtime edges that are not cargo deps.
    }

    let new_json = escape_nonascii(&serde_json::to_string(&data)?);
    if new_json.contains('<') {
        problems.push(
            "workspace-data contains a raw '<' (a string could close the script element)".into(),
        );
    }
    let mut html = format!(
        "{}{}{}",
        &current[..content_start],
        new_json,
        &current[close..]
    );
    let mut oneliner_drift = None;
    if chrome {
        let traced = traces.iter().filter(|t| t.fm.is_some()).count();
        let examples = graph.packages.iter().filter(|p| p.is_example).count();
        let want_oneliner = topology::examples_oneliner(examples);
        if let Some(have) = examples_oneliner_text(&html) {
            if have != want_oneliner {
                oneliner_drift = Some(format!("examples one-liner: {have} -> {want_oneliner}"));
            }
        }
        html = sync_examples_oneliner(&html, examples)?;
        html = replace_marked(&html, FOOTER_START, FOOTER_END, &footer_block(traces))?;
        html = replace_marked(
            &html,
            COUNTER_START,
            COUNTER_END,
            &format!("Showing {node_count} of {node_count}"),
        )?;
        html = replace_marked(
            &html,
            TRACE_COUNT_START,
            TRACE_COUNT_END,
            &format!("{traced} traces"),
        )?;
        html = replace_marked(
            &html,
            MEMBERS_START,
            MEMBERS_END,
            &members_sentence(graph, node_count, edge_count),
        )?;
    }
    let mut chrome_changes = Vec::new();
    if let Some(line) = oneliner_drift {
        chrome_changes.push(line);
    }
    if chrome {
        chrome_changes.extend(chrome_region_changes(current, &html));
    }
    let runtime_changes = runtime_change_lines(&runtime_before, &runtime_snapshot(&data));
    Ok(MapRender {
        html,
        changed_links,
        changed_status,
        problems,
        missing_nodes: topo.missing_nodes,
        extra_nodes: topo.extra_nodes,
        missing_edges: topo.missing_edges,
        extra_edges: topo.extra_edges,
        optional_changes: topo.optional_changes,
        example_list: topo.example_list,
        loc_changes: topo.loc_changes,
        chrome_changes,
        runtime_changes,
        raw_script_lt,
    })
}

fn runtime_snapshot(data: &serde_json::Value) -> BTreeMap<(String, String), (bool, String)> {
    let mut out = BTreeMap::new();
    let Some(edges) = data.get("edges").and_then(|e| e.as_array()) else {
        return out;
    };
    for e in edges {
        let s = e.get("source").and_then(|v| v.as_str()).unwrap_or("");
        let t = e.get("target").and_then(|v| v.as_str()).unwrap_or("");
        let runtime = e.get("runtime").and_then(|v| v.as_bool()).unwrap_or(false);
        let note = e
            .get("runtimeNote")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if runtime || !note.is_empty() {
            out.insert((s.to_string(), t.to_string()), (runtime, note));
        }
    }
    out
}

fn runtime_change_lines(
    before: &BTreeMap<(String, String), (bool, String)>,
    after: &BTreeMap<(String, String), (bool, String)>,
) -> Vec<String> {
    let mut lines = Vec::new();
    let keys: BTreeSet<_> = before.keys().chain(after.keys()).cloned().collect();
    for key in keys {
        let old = before.get(&key);
        let new = after.get(&key);
        if old == new {
            continue;
        }
        let (src, tgt) = &key;
        match (old, new) {
            (None, Some((runtime, note))) => {
                lines.push(format!("runtime {src} -> {tgt} set ({runtime}, {note})"))
            }
            (Some(_), None) => lines.push(format!("runtime {src} -> {tgt} cleared")),
            (Some((or, on)), Some((nr, nn))) => lines.push(format!(
                "runtime {src} -> {tgt} changed ({or}, {on}) -> ({nr}, {nn})"
            )),
            (None, None) => {}
        }
    }
    lines
}

fn examples_oneliner_text(html: &str) -> Option<&str> {
    let key = "'examples_collapsed':";
    let rest = html.get(html.find(key)? + key.len()..)?;
    let q0 = rest.find('\'')?;
    let q1 = rest.get(q0 + 1..)?.find('\'')?;
    Some(&rest[q0 + 1..q0 + 1 + q1])
}

fn marked_inner<'a>(html: &'a str, start: &str, end: &str) -> Option<&'a str> {
    let s = html.find(start)? + start.len();
    let rel = html[s..].find(end)?;
    Some(&html[s..s + rel])
}

fn chrome_region_changes(before: &str, after: &str) -> Vec<String> {
    let regions = [
        (COUNTER_START, COUNTER_END, "counter"),
        (TRACE_COUNT_START, TRACE_COUNT_END, "trace count"),
        (MEMBERS_START, MEMBERS_END, "members sentence"),
        (FOOTER_START, FOOTER_END, "footer trace list"),
    ];
    let mut out = Vec::new();
    for (start, end, label) in regions {
        let old = marked_inner(before, start, end).unwrap_or("");
        let new = marked_inner(after, start, end).unwrap_or("");
        if old == new {
            continue;
        }
        if label == "footer trace list" {
            out.push(footer_drift(old, new));
        } else {
            let old_t = old.trim();
            let new_t = new.trim();
            out.push(format!("{label}: {old_t} -> {new_t}"));
        }
    }
    out
}

fn footer_drift(old: &str, new: &str) -> String {
    let old_ids = footer_ids(old);
    let new_ids = footer_ids(new);
    let removed: Vec<&str> = old_ids.difference(&new_ids).copied().collect();
    let added: Vec<&str> = new_ids.difference(&old_ids).copied().collect();
    if removed.is_empty() && added.is_empty() {
        return "footer trace list changed".to_string();
    }
    let mut parts = Vec::new();
    if !removed.is_empty() {
        parts.push(format!("removed {}", removed.join(", ")));
    }
    if !added.is_empty() {
        parts.push(format!("added {}", added.join(", ")));
    }
    format!("footer trace list: {}", parts.join("; "))
}

fn footer_ids(html: &str) -> BTreeSet<&str> {
    let mut ids = BTreeSet::new();
    let mut rest = html;
    while let Some(i) = rest.find("href=\"./") {
        rest = &rest[i + "href=\"./".len()..];
        let Some(end) = rest.find('"') else { break };
        let href = &rest[..end];
        if let Some(name) = href.strip_suffix(".md") {
            ids.insert(name);
        }
        rest = &rest[end..];
    }
    ids
}

// ---- targets / drift ---------------------------------------------------------

struct Target {
    path: PathBuf,
    current: String,
    rendered: String,
}

fn build_targets(
    root: &Path,
    traces: &[TraceDoc],
    graph: &Graph,
) -> Result<(Vec<Target>, MapRender)> {
    let claude_path = root.join("CLAUDE.md");
    let claude_cur = std::fs::read_to_string(&claude_path).context("reading CLAUDE.md")?;
    let claude_new = render_claude_md(&claude_cur, traces)?;

    let map_path = root.join("docs/architecture/workspace_map.html");
    let map_cur = std::fs::read_to_string(&map_path).context("reading workspace_map.html")?;
    let map_render = render_map(&map_cur, traces, graph, root)?;

    let targets = vec![
        Target {
            path: claude_path,
            current: claude_cur,
            rendered: claude_new,
        },
        Target {
            path: map_path,
            current: map_cur,
            rendered: map_render.html.clone(),
        },
    ];
    Ok((targets, map_render))
}

fn first_line_diff(a: &str, b: &str) -> Option<(usize, String, String)> {
    let (la, lb): (Vec<&str>, Vec<&str>) = (a.lines().collect(), b.lines().collect());
    for i in 0..la.len().max(lb.len()) {
        let x = la.get(i).copied().unwrap_or("");
        let y = lb.get(i).copied().unwrap_or("");
        if x != y {
            return Some((i + 1, x.to_string(), y.to_string()));
        }
    }
    None
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let root = match &cli.root {
        Some(r) => r.clone(),
        None => {
            // workspace root via cargo metadata from CWD
            let meta = cargo_metadata::MetadataCommand::new()
                .no_deps()
                .exec()
                .context("locating workspace root via cargo metadata")?;
            meta.workspace_root.into_std_path_buf()
        }
    };
    let arch_dir = root.join("docs/architecture");
    let traces = load_traces(&arch_dir)?;
    let graph = workspace_graph(&root)?;
    let crates: Vec<String> = graph.packages.iter().map(|p| p.name.clone()).collect();

    // Validation always runs first.
    let problems = validate(&traces, &crates);
    if !problems.is_empty() {
        eprintln!("Validation failed ({} problem(s)):", problems.len());
        for p in &problems {
            eprintln!("  - {p}");
        }
        bail!("trace front-matter validation failed");
    }
    let traced = traces.iter().filter(|t| t.fm.is_some()).count();
    eprintln!(
        "Validated {traced} trace(s); {} workspace crate(s).",
        crates.len()
    );

    if cli.list_untraced {
        let owners = ownership_map(&traces);
        // Subsystems only: examples/demos are excluded (they are not traced subsystems).
        let mut untraced: Vec<&str> = graph
            .packages
            .iter()
            .filter(|p| !p.is_example && !owners.contains_key(&p.name))
            .map(|p| p.name.as_str())
            .collect();
        untraced.sort();
        let examples = graph.packages.iter().filter(|p| p.is_example).count();
        println!(
            "Untraced subsystem crates ({}/{} non-example crates; {examples} examples excluded):",
            untraced.len(),
            crates.len() - examples
        );
        for c in untraced {
            println!("  {c}");
        }
    }

    if cli.validate_only {
        return Ok(());
    }

    if cli.write || cli.check {
        let (targets, map_render) = build_targets(&root, &traces, &graph)?;
        if !map_render.problems.is_empty() {
            eprintln!(
                "Map integrity failed ({} problem(s)):",
                map_render.problems.len()
            );
            for p in &map_render.problems {
                eprintln!("  - {p}");
            }
            bail!("workspace_map.html failed its integrity check; nothing written");
        }
        let mut drift = false;
        for t in &targets {
            if t.current == t.rendered {
                continue;
            }
            drift = true;
            let name = t.path.file_name().unwrap().to_string_lossy();
            if cli.check {
                eprintln!("DRIFT: {name} would change.");
                if name == "CLAUDE.md" {
                    if let Some((line, old, new)) = first_line_diff(&t.current, &t.rendered) {
                        eprintln!("  first change at line {line}:");
                        eprintln!("    - {old}");
                        eprintln!("    + {new}");
                    }
                }
            }
            if name.ends_with(".html") {
                for (cr, old, new) in &map_render.changed_links {
                    eprintln!(
                        "  map link {cr}: {} -> {}",
                        old.as_deref().unwrap_or("null"),
                        new.as_deref().unwrap_or("null")
                    );
                }
                for (cr, old, new) in &map_render.changed_status {
                    eprintln!(
                        "  map status {cr}: {} -> {new}",
                        if old.is_empty() { "?" } else { old }
                    );
                }
                for line in &map_render.runtime_changes {
                    eprintln!("  {line}");
                }
                for id in &map_render.missing_nodes {
                    eprintln!("  missing node {id}");
                }
                for id in &map_render.extra_nodes {
                    eprintln!("  extra node {id}");
                }
                for e in &map_render.missing_edges {
                    eprintln!("  missing edge {e}");
                }
                for e in &map_render.extra_edges {
                    eprintln!("  extra edge {e}");
                }
                for line in &map_render.optional_changes {
                    eprintln!("  {line}");
                }
                for line in &map_render.example_list {
                    eprintln!("  {line}");
                }
                for line in &map_render.loc_changes {
                    eprintln!("  {line}");
                }
                for line in &map_render.chrome_changes {
                    eprintln!("  {line}");
                }
                if map_render.raw_script_lt {
                    eprintln!("  workspace-data contains a raw '<'");
                }
            }
            if cli.write {
                std::fs::write(&t.path, &t.rendered)
                    .with_context(|| format!("writing {}", t.path.display()))?;
                eprintln!("WROTE: {name}");
            }
        }
        if cli.check && drift {
            bail!("trace registry/map is out of sync with front-matter (run `--write`)");
        }
        if !drift {
            let nodes = map_render.missing_nodes.len() + map_render.extra_nodes.len();
            let edges = map_render.missing_edges.len() + map_render.extra_edges.len();
            eprintln!(
                "In sync: CLAUDE.md trace table, map nodes/edges, footer, and trace anchors match cargo metadata and front-matter ({nodes} node diffs, {edges} edge diffs)."
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod js_sink;

#[cfg(test)]
mod tests {
    use super::js_sink::html_parsing_sink;
    use super::*;

    fn ydoc(stem: &str, primary: &str, owns: &str) -> TraceDoc {
        let y = format!(
            "schema_version: 1\ntrace_id: {stem}\ntitle: T {stem}\ndescription: Desc {stem}\n\
             primary_crate: {primary}\ndomain: core\nlifecycle_status: active\n\
             integration_status: wired\nowns: {owns}\ndoc_version: \"1.0\"\nlast_verified_commit: abc1234\n"
        );
        TraceDoc {
            stem: stem.to_string(),
            fm: Some(yaml_serde::from_str(&y).expect("valid front-matter")),
            fm_error: None,
            body: String::new(),
        }
    }

    #[test]
    fn extracts_front_matter_lf_and_crlf() {
        assert_eq!(
            extract_front_matter("---\nfoo: 1\n---\nbody").unwrap(),
            "foo: 1\n"
        );
        assert_eq!(
            extract_front_matter("---\r\nfoo: 1\r\n---\r\nbody").unwrap(),
            "foo: 1\n"
        );
        assert!(extract_front_matter("# not front-matter").is_none());
        assert!(extract_front_matter("---\nunterminated\n").is_none());
    }

    #[test]
    fn unknown_field_is_rejected() {
        // deny_unknown_fields guards typos in the contract.
        let y = "schema_version: 1\ntrace_id: x\ntitle: T\ndescription: D\nprimary_crate: c\n\
                 domain: core\nlifecycle_status: active\nintegration_status: wired\n\
                 doc_version: \"1\"\nlast_verified_commit: a\ntypo_field: 1\n";
        assert!(yaml_serde::from_str::<FrontMatter>(y).is_err());
    }

    #[test]
    fn valid_set_has_no_problems() {
        let traces = vec![ydoc("water", "astraweave-water", "[astraweave-water]")];
        let crates = vec!["astraweave-water".to_string()];
        assert!(validate(&traces, &crates).is_empty());
    }

    #[test]
    fn detects_stem_mismatch_and_bad_reference() {
        let mut t = ydoc("ocean", "astraweave-ghost", "[]");
        // trace_id is "ocean" but pretend the file stem differs
        t.stem = "sea".to_string();
        let p = validate(&[t], &[]);
        assert!(p.iter().any(|m| m.contains("does not match filename stem")));
        assert!(p.iter().any(|m| m.contains("is not a workspace crate")));
    }

    #[test]
    fn detects_double_ownership() {
        let traces = vec![
            ydoc("a", "astraweave-x", "[astraweave-shared]"),
            ydoc("b", "astraweave-y", "[astraweave-shared]"),
        ];
        let crates = vec![
            "astraweave-x".to_string(),
            "astraweave-y".to_string(),
            "astraweave-shared".to_string(),
        ];
        let p = validate(&traces, &crates);
        assert!(
            p.iter().any(|m| m.contains("owned by two traces")),
            "got {p:?}"
        );
    }

    #[test]
    fn detects_duplicate_trace_id() {
        // two docs with the same trace_id (distinct stems so stem-check passes for one)
        let mut a = ydoc("dup", "astraweave-x", "[]");
        let mut b = ydoc("dup", "astraweave-y", "[]");
        a.stem = "dup".into();
        b.stem = "dup2".into();
        // force both trace_ids to "dup"
        b.fm.as_mut().unwrap().trace_id = "dup".into();
        let crates = vec!["astraweave-x".to_string(), "astraweave-y".to_string()];
        let p = validate(&[a, b], &crates);
        assert!(
            p.iter().any(|m| m.contains("duplicate trace_id")),
            "got {p:?}"
        );
    }

    #[test]
    fn table_is_sorted_and_formatted() {
        let traces = vec![
            ydoc("zebra", "astraweave-z", "[]"),
            ydoc("alpha", "astraweave-a", "[]"),
        ];
        let table = gen_trace_table(&traces);
        assert!(table.starts_with(TABLE_START));
        assert!(table.ends_with(TABLE_END));
        let alpha = table.find("alpha.md").unwrap();
        let zebra = table.find("zebra.md").unwrap();
        assert!(alpha < zebra, "rows must be sorted by trace_id");
        assert!(table.contains("| `docs/architecture/alpha.md` | Desc alpha |"));
    }

    #[test]
    fn escape_nonascii_matches_ensure_ascii_style() {
        assert_eq!(escape_nonascii("a§b"), "a\\u00a7b"); // U+00A7
        assert_eq!(escape_nonascii("x→y"), "x\\u2192y"); // U+2192
        assert_eq!(escape_nonascii("ascii"), "ascii");
        assert_eq!(escape_nonascii("</script>"), "\\u003c/script>");
        assert!(!escape_nonascii("a < b </script>").contains('<'));
        // astral plane -> surrogate pair (U+1F600)
        assert_eq!(escape_nonascii("\u{1F600}"), "\\ud83d\\ude00");
    }

    #[test]
    fn ownership_map_links_each_owned_crate() {
        let traces = vec![ydoc(
            "ai_pipeline",
            "astraweave-ai",
            "[astraweave-ai, astraweave-llm]",
        )];
        let m = ownership_map(&traces);
        assert_eq!(
            m.get("astraweave-ai").map(String::as_str),
            Some("ai_pipeline.md")
        );
        assert_eq!(
            m.get("astraweave-llm").map(String::as_str),
            Some("ai_pipeline.md")
        );
        assert_eq!(m.get("astraweave-render"), None);
    }

    #[test]
    fn render_claude_md_matches_crlf_file() {
        let traces = vec![ydoc("alpha", "astraweave-a", "[]")];
        let current = format!("intro\r\n{TABLE_START}\r\nstale\r\n{TABLE_END}\r\ntail\r\n");
        let out = render_claude_md(&current, &traces).unwrap();
        assert!(out.contains("alpha.md"));
        // A CRLF file must not gain any lone LF in the regenerated block.
        assert!(
            !out.replace("\r\n", "").contains('\n'),
            "lone LF leaked into a CRLF file"
        );
        assert!(out.starts_with("intro\r\n") && out.ends_with("tail\r\n"));
    }

    #[test]
    fn render_claude_md_matches_lf_file() {
        let traces = vec![ydoc("alpha", "astraweave-a", "[]")];
        let current = format!("intro\n{TABLE_START}\nstale\n{TABLE_END}\ntail\n");
        let out = render_claude_md(&current, &traces).unwrap();
        assert!(out.contains("alpha.md"));
        assert!(!out.contains('\r'), "LF file must not gain CR");
    }

    fn tdoc(stem: &str, yaml: &str) -> TraceDoc {
        TraceDoc {
            stem: stem.to_string(),
            fm: Some(yaml_serde::from_str(yaml).expect("valid front-matter")),
            fm_error: None,
            body: String::new(),
        }
    }

    fn graph_of(pkgs: &[(&str, bool)], deps: &[(&str, &str, bool)]) -> Graph {
        Graph {
            packages: pkgs
                .iter()
                .map(|(name, is_example)| topology::Pkg {
                    name: (*name).to_string(),
                    is_example: *is_example,
                    manifest: format!("{name}/Cargo.toml"),
                })
                .collect(),
            deps: deps
                .iter()
                .map(|(from, to, optional)| (((*from).to_string(), (*to).to_string()), *optional))
                .collect(),
        }
    }

    #[test]
    fn status_pair_mapping() {
        use Integration as I;
        use Lifecycle as L;
        assert_eq!(status_pair(L::Active, I::Wired), Some(("active", "active")));
        assert_eq!(
            status_pair(L::Active, I::Mixed),
            Some(("partial", "partial"))
        );
        assert_eq!(
            status_pair(L::Active, I::Partial),
            Some(("partial", "partial"))
        );
        assert_eq!(
            status_pair(L::Active, I::ExampleOnly),
            Some(("dormant", "in-design"))
        );
        assert_eq!(
            status_pair(L::Active, I::TestOnly),
            Some(("dormant", "in-design"))
        );
        assert_eq!(
            status_pair(L::InDesign, I::Wired),
            Some(("dormant", "in-design"))
        );
        assert_eq!(status_pair(L::Unknown, I::Wired), None);
        assert_eq!(status_pair(L::Active, I::Unknown), None);
    }

    #[test]
    fn runtime_edge_validation_flags_bad_targets() {
        let y = "schema_version: 1\ntrace_id: a\ntitle: T\ndescription: D\nprimary_crate: astraweave-x\n\
                 domain: core\nlifecycle_status: active\nintegration_status: wired\nowns: [astraweave-x]\n\
                 doc_version: \"1\"\nlast_verified_commit: c\n\
                 runtime_edges:\n  - to: astraweave-ghost\n  - to: astraweave-x\n";
        let p = validate(&[tdoc("a", y)], &["astraweave-x".to_string()]);
        assert!(
            p.iter()
                .any(|m| m.contains("runtime_edges target 'astraweave-ghost'")),
            "got {p:?}"
        );
        assert!(p.iter().any(|m| m.contains("self-edge")), "got {p:?}");
    }

    #[test]
    fn owning_trace_drives_status_not_slice_trace() {
        // terrain.md owns astraweave-terrain (mixed -> partial); terrain_materials.md is a slice
        // (same primary_crate, owns []) and must NOT drive the node's status.
        let owner = tdoc(
            "terrain",
            "schema_version: 1\ntrace_id: terrain\ntitle: T\ndescription: D\nprimary_crate: astraweave-terrain\n\
             domain: physics-world\nlifecycle_status: active\nintegration_status: mixed\nowns: [astraweave-terrain]\n\
             doc_version: \"1\"\nlast_verified_commit: c\nruntime_edges:\n  - to: astraweave-physics\n",
        );
        let slice = tdoc(
            "terrain_materials",
            "schema_version: 1\ntrace_id: terrain_materials\ntitle: T\ndescription: D\nprimary_crate: astraweave-terrain\n\
             domain: physics-world\nlifecycle_status: active\nintegration_status: wired\nowns: []\n\
             doc_version: \"1\"\nlast_verified_commit: c\n",
        );
        let html = "<script id=\"workspace-data\">{\"nodes\":[{\"id\":\"astraweave-terrain\",\"domain\":\"physics-world\",\"status\":\"active\",\"statusCategory\":\"active\",\"statusEvidence\":null,\"trace\":null},{\"id\":\"astraweave-physics\",\"domain\":\"physics-world\",\"trace\":null}],\"edges\":[{\"source\":\"astraweave-terrain\",\"target\":\"astraweave-physics\",\"kind\":\"workspace-dep\"}],\"domains\":[{\"id\":\"physics-world\"}]}</script>";
        let graph = graph_of(
            &[("astraweave-terrain", false), ("astraweave-physics", false)],
            &[("astraweave-terrain", "astraweave-physics", false)],
        );
        let out = render_map_inner(html, &[owner, slice], &graph, Path::new("."), false).unwrap();
        assert!(out.problems.is_empty(), "got {:?}", out.problems);
        let json = &out.html[out.html.find('>').unwrap() + 1..out.html.find("</script>").unwrap()];
        let d: serde_json::Value = serde_json::from_str(json).unwrap();
        let terrain = d["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|n| n["id"] == "astraweave-terrain")
            .unwrap();
        assert_eq!(
            terrain["status"], "partial",
            "owning trace (mixed) must win, not slice (wired)"
        );
        assert_eq!(terrain["trace"], "terrain.md");
        assert_eq!(
            d["edges"][0]["runtime"], true,
            "runtime edge must be marked on the cargo dep"
        );
        assert_eq!(d["edges"][0]["kind"], "workspace-dep");
    }

    /// Regression: combat.md / destruction.md / veil.md declared runtime edges from crates
    /// with no map node; the tool emitted them anyway and Cytoscape threw on load, so the
    /// published map rendered zero nodes while `--check` reported "In sync".
    #[test]
    fn runtime_edge_from_unmapped_crate_is_not_emitted() {
        let combat = tdoc(
            "combat",
            "schema_version: 1\ntrace_id: combat\ntitle: T\ndescription: D\nprimary_crate: astraweave-combat\n\
             domain: gameplay\nlifecycle_status: active\nintegration_status: partial\nowns: [astraweave-combat]\n\
             doc_version: \"1\"\nlast_verified_commit: c\nruntime_edges:\n  - to: astraweave-ecs\n",
        );
        let html = "<script id=\"workspace-data\">{\"nodes\":[{\"id\":\"astraweave-ecs\",\"domain\":\"core\",\"trace\":null}],\"edges\":[],\"domains\":[{\"id\":\"core\"}]}</script>";
        let graph = graph_of(&[("astraweave-ecs", false)], &[]);
        let out = render_map_inner(html, &[combat], &graph, Path::new("."), false).unwrap();
        assert!(
            out.problems
                .iter()
                .any(|p| p.contains("combat.md") && p.contains("no map node for astraweave-combat")),
            "got {:?}",
            out.problems
        );
        assert!(
            !out.html.contains("astraweave-combat"),
            "dangling edge must not be written into the map"
        );
    }

    #[test]
    fn map_integrity_flags_dangling_edges_and_unknown_domains() {
        let data = serde_json::json!({
            "nodes": [
                {"id": "a", "domain": "core"},
                {"id": "b", "domain": "nowhere"},
                {"id": "a", "domain": "core"}
            ],
            "edges": [
                {"source": "a", "target": "b"},
                {"source": "ghost", "target": "a"}
            ],
            "domains": [{"id": "core"}]
        });
        let p = map_integrity(&data);
        assert!(
            p.iter().any(|m| m.contains("source 'ghost' is not a node")),
            "{p:?}"
        );
        assert!(p.iter().any(|m| m.contains("domain 'nowhere'")), "{p:?}");
        assert!(
            p.iter().any(|m| m.contains("duplicate node id 'a'")),
            "{p:?}"
        );
        assert_eq!(p.len(), 3, "the a -> b edge is valid: {p:?}");
    }

    #[test]
    fn map_integrity_accepts_a_consistent_map() {
        let data = serde_json::json!({
            "nodes": [{"id": "a", "domain": "core"}, {"id": "b", "domain": "core"}],
            "edges": [{"source": "a", "target": "b"}],
            "domains": [{"id": "core"}]
        });
        assert!(map_integrity(&data).is_empty());
    }

    fn bare(json: &str) -> String {
        format!("<script id=\"workspace-data\">{json}</script>")
    }

    #[test]
    fn removing_a_cargo_edge_is_reported_and_restored() {
        let html = bare(
            r#"{"nodes":[{"id":"a","domain":"core","trace":null},{"id":"b","domain":"core","trace":null}],"edges":[],"domains":[{"id":"core"}]}"#,
        );
        let graph = graph_of(&[("a", false), ("b", false)], &[("a", "b", false)]);
        let out = render_map_inner(&html, &[], &graph, Path::new("."), false).unwrap();
        assert!(
            out.missing_edges.iter().any(|e| e == "a -> b"),
            "{:?}",
            out.missing_edges
        );
        assert!(out.html.contains("\"source\":\"a\""));
        assert!(out.html.contains("\"target\":\"b\""));
        assert_ne!(out.html, html);
    }

    #[test]
    fn a_phantom_edge_is_reported_and_dropped() {
        let html = bare(
            r#"{"nodes":[{"id":"a","domain":"core","trace":null},{"id":"b","domain":"core","trace":null}],"edges":[{"source":"a","target":"b","kind":"workspace-dep","optional":false}],"domains":[{"id":"core"}]}"#,
        );
        let graph = graph_of(&[("a", false), ("b", false)], &[]);
        let out = render_map_inner(&html, &[], &graph, Path::new("."), false).unwrap();
        assert!(
            out.extra_edges.iter().any(|e| e == "a -> b"),
            "{:?}",
            out.extra_edges
        );
        let json = &out.html[out.html.find('>').unwrap() + 1..out.html.find("</script>").unwrap()];
        let d: serde_json::Value = serde_json::from_str(json).unwrap();
        assert!(d["edges"].as_array().unwrap().is_empty(), "{d}");
    }

    #[test]
    fn dropping_a_node_is_reported_and_restored() {
        let html = bare(
            r#"{"nodes":[{"id":"b","domain":"core","trace":null}],"edges":[],"domains":[{"id":"core"}]}"#,
        );
        let graph = graph_of(&[("a", false), ("b", false)], &[]);
        let out = render_map_inner(&html, &[], &graph, Path::new("."), false).unwrap();
        assert!(
            out.missing_nodes.iter().any(|id| id == "a"),
            "{:?}",
            out.missing_nodes
        );
        assert!(out.html.contains("\"id\":\"a\""));
        assert_ne!(out.html, html);
    }

    #[test]
    fn a_trace_that_owns_a_crateless_node_is_an_error() {
        // Example crates are collapsed, so an owned example has no node.
        let y = "schema_version: 1\ntrace_id: demo\ntitle: T\ndescription: D\nprimary_crate: hello_companion\n\
                 domain: gameplay\nlifecycle_status: active\nintegration_status: wired\nowns: [hello_companion]\n\
                 doc_version: \"1\"\nlast_verified_commit: c\n";
        let trace = tdoc("demo", y);
        let html = bare(
            r#"{"nodes":[{"id":"astraweave-core","domain":"core","trace":null}],"edges":[],"domains":[{"id":"core"},{"id":"gameplay"},{"id":"examples"}]}"#,
        );
        let graph = graph_of(
            &[("astraweave-core", false), ("hello_companion", true)],
            &[("hello_companion", "astraweave-core", false)],
        );
        let out = render_map_inner(&html, &[trace], &graph, Path::new("."), false).unwrap();
        assert!(
            out.problems.iter().any(|p| {
                p.contains("demo.md")
                    && p.contains("owns 'hello_companion'")
                    && p.contains("no map node")
            }),
            "{:?}",
            out.problems
        );
        assert!(
            !out.html.contains("\"id\":\"hello_companion\""),
            "an example must stay inside the aggregate"
        );
    }

    #[test]
    fn a_broken_trace_section_anchor_is_an_error() {
        let y = "schema_version: 1\ntrace_id: water\ntitle: T\ndescription: D\nprimary_crate: astraweave-water\n\
                 domain: physics-world\nlifecycle_status: active\nintegration_status: wired\nowns: [astraweave-water]\n\
                 doc_version: \"1\"\nlast_verified_commit: c\n";
        let mut trace = tdoc("water", y);
        trace.body = "## 5. Active File Map\n## 7. Decision Log\n".into();
        let html = format!(
            "<span><!-- MAP-TRACE-COUNT:START -->x<!-- MAP-TRACE-COUNT:END --></span>\n\
             <span><!-- MAP-COUNTER:START -->x<!-- MAP-COUNTER:END --></span>\n\
             <!-- MAP-MEMBERS:START -->x<!-- MAP-MEMBERS:END -->\n\
             <!-- TRACE-FOOTER:START -->x<!-- TRACE-FOOTER:END -->\n\
             'examples_collapsed': 'stale',\n\
             <a href=\"#5-active-file-map\"></a><a href=\"#6-conflict-map\"></a>\
             <a href=\"#7-decision-log\"></a><a href=\"#8-invariants\"></a>\
             <a href=\"#11-open-questions\"></a>\n{}",
            bare(
                r#"{"nodes":[{"id":"astraweave-water","domain":"physics-world","trace":null}],"edges":[],"domains":[{"id":"physics-world"}]}"#
            )
        );
        let graph = graph_of(&[("astraweave-water", false)], &[]);
        let out = render_map(&html, &[trace], &graph, Path::new(".")).unwrap();
        assert!(
            out.problems
                .iter()
                .any(|p| p.contains("water.md") && p.contains("#6-conflict-map")),
            "{:?}",
            out.problems
        );
        assert!(
            out.problems.iter().any(|p| p.contains("#8-invariants")),
            "{:?}",
            out.problems
        );
        assert!(
            out.problems
                .iter()
                .any(|p| p.contains("#11-open-questions")),
            "{:?}",
            out.problems
        );
    }

    #[test]
    fn an_explicit_anchor_id_resolves() {
        let y = "schema_version: 1\ntrace_id: water\ntitle: T\ndescription: D\nprimary_crate: astraweave-water\n\
                 domain: physics-world\nlifecycle_status: active\nintegration_status: wired\nowns: [astraweave-water]\n\
                 doc_version: \"1\"\nlast_verified_commit: c\n";
        let mut trace = tdoc("water", y);
        trace.body = "\
<a id=\"5-active-file-map\"></a>\n## 5. Active File Map\n\
<a id=\"6-conflict-map\"></a>\n## 6. Conflict Map / Residue\n\
<a id=\"7-decision-log\"></a>\n## 7. Decision Log\n\
<a id=\"8-invariants\"></a>\n## 8. Known Invariants\n\
<a id=\"11-open-questions\"></a>\n## 11. Open Questions / Parked Decisions\n"
            .into();
        let html = format!(
            "<span><!-- MAP-TRACE-COUNT:START -->x<!-- MAP-TRACE-COUNT:END --></span>\n\
             <span><!-- MAP-COUNTER:START -->x<!-- MAP-COUNTER:END --></span>\n\
             <!-- MAP-MEMBERS:START -->x<!-- MAP-MEMBERS:END -->\n\
             <!-- TRACE-FOOTER:START -->x<!-- TRACE-FOOTER:END -->\n\
             'examples_collapsed': 'stale',\n\
             <a href=\"#5-active-file-map\"></a><a href=\"#6-conflict-map\"></a>\
             <a href=\"#7-decision-log\"></a><a href=\"#8-invariants\"></a>\
             <a href=\"#11-open-questions\"></a>\n{}",
            bare(
                r#"{"nodes":[{"id":"astraweave-water","domain":"physics-world","trace":null}],"edges":[],"domains":[{"id":"physics-world"}]}"#
            )
        );
        let graph = graph_of(&[("astraweave-water", false)], &[]);
        let out = render_map(&html, &[trace], &graph, Path::new(".")).unwrap();
        assert!(
            out.problems.iter().all(|p| !p.contains("anchor")),
            "{:?}",
            out.problems
        );
    }

    #[test]
    fn a_duplicate_explicit_anchor_id_is_an_error() {
        let y = "schema_version: 1\ntrace_id: water\ntitle: T\ndescription: D\nprimary_crate: astraweave-water\n\
                 domain: physics-world\nlifecycle_status: active\nintegration_status: wired\nowns: [astraweave-water]\n\
                 doc_version: \"1\"\nlast_verified_commit: c\n";
        let mut trace = tdoc("water", y);
        trace.body = "\
<a id=\"5-active-file-map\"></a>\n## 5. Active File Map\n\
<a id=\"6-conflict-map\"></a>\n## 6. Conflict Map\n\
<a id=\"7-decision-log\"></a>\n## 7. Decision Log\n\
<a id=\"8-invariants\"></a>\n## 8. Invariants\n\
<a id=\"8-invariants\"></a>\n## 8.5 Extra\n\
<a id=\"11-open-questions\"></a>\n## 11. Open Questions\n"
            .into();
        let html = format!(
            "<span><!-- MAP-TRACE-COUNT:START -->x<!-- MAP-TRACE-COUNT:END --></span>\n\
             <span><!-- MAP-COUNTER:START -->x<!-- MAP-COUNTER:END --></span>\n\
             <!-- MAP-MEMBERS:START -->x<!-- MAP-MEMBERS:END -->\n\
             <!-- TRACE-FOOTER:START -->x<!-- TRACE-FOOTER:END -->\n\
             'examples_collapsed': 'stale',\n\
             <a href=\"#5-active-file-map\"></a><a href=\"#6-conflict-map\"></a>\
             <a href=\"#7-decision-log\"></a><a href=\"#8-invariants\"></a>\
             <a href=\"#11-open-questions\"></a>\n{}",
            bare(
                r#"{"nodes":[{"id":"astraweave-water","domain":"physics-world","trace":null}],"edges":[],"domains":[{"id":"physics-world"}]}"#
            )
        );
        let graph = graph_of(&[("astraweave-water", false)], &[]);
        let out = render_map(&html, &[trace], &graph, Path::new(".")).unwrap();
        assert!(
            out.problems
                .iter()
                .any(|p| p.contains("water.md")
                    && p.contains("duplicate explicit id '8-invariants'")),
            "{:?}",
            out.problems
        );
    }

    #[test]
    fn optional_drift_names_the_edge() {
        let html = bare(
            r#"{"nodes":[{"id":"a","domain":"core","trace":null},{"id":"b","domain":"core","trace":null}],"edges":[{"source":"a","target":"b","kind":"workspace-dep","optional":true}],"domains":[{"id":"core"}]}"#,
        );
        let graph = graph_of(&[("a", false), ("b", false)], &[("a", "b", false)]);
        let out = render_map_inner(&html, &[], &graph, Path::new("."), false).unwrap();
        assert!(
            out.optional_changes
                .iter()
                .any(|l| l.contains("a -> b") && l.contains("optional true -> false")),
            "{:?}",
            out.optional_changes
        );
        assert!(out.runtime_changes.is_empty(), "{:?}", out.runtime_changes);
    }

    #[test]
    fn a_closing_script_tag_in_a_string_field_stays_inside_the_json() {
        let data = serde_json::json!({
            "nodes": [{
                "id": "a",
                "domain": "core",
                "trace": null,
                "statusEvidence": "</script><script>alert(1)</script>"
            }],
            "edges": [],
            "domains": [{"id": "core"}]
        });
        // The fixture itself must not contain a raw closing tag, or the
        // `<script>` scanner would stop early. `\u003c` parses back to `<`.
        let embedded = escape_nonascii(&serde_json::to_string(&data).unwrap());
        assert!(!embedded.contains('<'));
        let html = format!("<script id=\"workspace-data\">{embedded}</script>");
        let graph = graph_of(&[("a", false)], &[]);
        let out = render_map_inner(&html, &[], &graph, Path::new("."), false).unwrap();
        assert!(
            out.problems.iter().all(|p| !p.contains("raw '<'")),
            "{:?}",
            out.problems
        );
        let json = &out.html[out.html.find('>').unwrap() + 1..out.html.find("</script>").unwrap()];
        assert!(!json.contains('<'), "{json}");
        assert!(json.contains("\\u003c/script>"));
        let parsed: serde_json::Value = serde_json::from_str(json).unwrap();
        assert_eq!(
            parsed["nodes"][0]["statusEvidence"],
            "</script><script>alert(1)</script>"
        );
    }

    #[test]
    fn a_raw_lt_in_workspace_data_is_named_and_rewritten() {
        let html = bare(
            r#"{"nodes":[{"id":"a","domain":"core","trace":null,"openQuestions":["THRESH < 1.0"]}],"edges":[],"domains":[{"id":"core"}]}"#,
        );
        let graph = graph_of(&[("a", false)], &[]);
        let out = render_map_inner(&html, &[], &graph, Path::new("."), false).unwrap();
        assert!(out.raw_script_lt);
        let json = &out.html[out.html.find('>').unwrap() + 1..out.html.find("</script>").unwrap()];
        assert!(!json.contains('<'));
        assert!(json.contains("\\u003c"));
        assert_ne!(out.html, html);
    }

    #[test]
    fn runtime_only_edge_uses_kind_runtime_not_workspace_dep() {
        let y = "schema_version: 1\ntrace_id: veil\ntitle: T\ndescription: D\nprimary_crate: astraweave-veil\n\
                 domain: gameplay\nlifecycle_status: active\nintegration_status: wired\nowns: [astraweave-veil]\n\
                 doc_version: \"1\"\nlast_verified_commit: c\n\
                 runtime_edges:\n  - from: astraweave-ai\n    to: astraweave-veil\n    note: agents read the scar\n";
        let trace = tdoc("veil", y);
        let html = bare(
            r#"{"nodes":[{"id":"astraweave-veil","domain":"gameplay","trace":null},{"id":"astraweave-ai","domain":"ai","trace":null}],"edges":[{"source":"astraweave-ai","target":"astraweave-veil","kind":"workspace-dep","anomaly":null,"types":[],"risk":"LOW","optional":false}],"domains":[{"id":"gameplay"},{"id":"ai"}]}"#,
        );
        let graph = graph_of(
            &[("astraweave-veil", false), ("astraweave-ai", false)],
            &[("astraweave-ai", "astraweave-veil", false)],
        );
        let out = render_map_inner(&html, &[trace], &graph, Path::new("."), false).unwrap();
        assert!(out.problems.is_empty(), "{:?}", out.problems);
        let json = &out.html[out.html.find('>').unwrap() + 1..out.html.find("</script>").unwrap()];
        let d: serde_json::Value = serde_json::from_str(json).unwrap();
        let edges = d["edges"].as_array().unwrap();
        assert_eq!(
            edges.len(),
            1,
            "the reverse runtime edge must not be added: {edges:?}"
        );
        assert_eq!(edges[0]["source"], "astraweave-ai");
        assert_eq!(edges[0]["target"], "astraweave-veil");
        assert_eq!(edges[0]["kind"], "workspace-dep");
        assert_eq!(edges[0]["runtime"], true);
        assert_eq!(edges[0]["runtimeNote"], "agents read the scar");
    }

    #[test]
    fn runtime_edge_without_a_cargo_dep_is_not_a_workspace_dep() {
        let y = "schema_version: 1\ntrace_id: a\ntitle: T\ndescription: D\nprimary_crate: a\n\
                 domain: core\nlifecycle_status: active\nintegration_status: wired\nowns: [a]\n\
                 doc_version: \"1\"\nlast_verified_commit: c\n\
                 runtime_edges:\n  - to: b\n    note: calls b at runtime\n";
        let trace = tdoc("a", y);
        let html = bare(
            r#"{"nodes":[{"id":"a","domain":"core","trace":null},{"id":"b","domain":"core","trace":null}],"edges":[],"domains":[{"id":"core"}]}"#,
        );
        let graph = graph_of(&[("a", false), ("b", false)], &[]);
        let out = render_map_inner(&html, &[trace], &graph, Path::new("."), false).unwrap();
        assert!(out.problems.is_empty(), "{:?}", out.problems);
        let json = &out.html[out.html.find('>').unwrap() + 1..out.html.find("</script>").unwrap()];
        let d: serde_json::Value = serde_json::from_str(json).unwrap();
        assert_eq!(d["edges"][0]["kind"], "runtime");
        assert_eq!(d["edges"][0]["source"], "a");
        assert_eq!(d["edges"][0]["target"], "b");
        assert_ne!(d["edges"][0]["kind"], "workspace-dep");
    }

    #[test]
    fn a_partial_required_anchor_set_is_an_error() {
        let html = "\
<span><!-- MAP-TRACE-COUNT:START -->x<!-- MAP-TRACE-COUNT:END --></span>\n\
<span><!-- MAP-COUNTER:START -->x<!-- MAP-COUNTER:END --></span>\n\
<!-- MAP-MEMBERS:START -->x<!-- MAP-MEMBERS:END -->\n\
<!-- TRACE-FOOTER:START -->x<!-- TRACE-FOOTER:END -->\n\
'examples_collapsed': 'stale',\n\
<a href=\"#5-active-file-map\"></a>\n\
<script id=\"workspace-data\">{\"nodes\":[{\"id\":\"a\",\"domain\":\"core\",\"trace\":null}],\"edges\":[],\"domains\":[{\"id\":\"core\"}]}</script>\n";
        let graph = graph_of(&[("a", false)], &[]);
        let out = render_map(html, &[], &graph, Path::new(".")).unwrap();
        assert!(
            out.problems.iter().any(|p| {
                p.contains("links some trace-section anchors but not")
                    && p.contains("#6-conflict-map")
                    && p.contains("#7-decision-log")
                    && p.contains("#8-invariants")
                    && p.contains("#11-open-questions")
            }),
            "{:?}",
            out.problems
        );
    }

    #[test]
    fn examples_oneliner_drift_is_named() {
        let html = "\
<span><!-- MAP-TRACE-COUNT:START -->x<!-- MAP-TRACE-COUNT:END --></span>\n\
<span><!-- MAP-COUNTER:START -->x<!-- MAP-COUNTER:END --></span>\n\
<!-- MAP-MEMBERS:START -->x<!-- MAP-MEMBERS:END -->\n\
<!-- TRACE-FOOTER:START -->x<!-- TRACE-FOOTER:END -->\n\
'examples_collapsed': 'stale',\n\
<a href=\"#5-active-file-map\"></a><a href=\"#6-conflict-map\"></a>\
<a href=\"#7-decision-log\"></a><a href=\"#8-invariants\"></a>\
<a href=\"#11-open-questions\"></a>\n\
<script id=\"workspace-data\">{\"nodes\":[{\"id\":\"a\",\"domain\":\"core\",\"trace\":null}],\"edges\":[],\"domains\":[{\"id\":\"core\"}]}</script>\n";
        let graph = graph_of(&[("a", false)], &[]);
        let out = render_map(html, &[], &graph, Path::new(".")).unwrap();
        assert!(out.problems.is_empty(), "{:?}", out.problems);
        assert!(
            out.chrome_changes
                .iter()
                .any(|l| l.starts_with("examples one-liner: stale -> 0 example crates collapsed")),
            "{:?}",
            out.chrome_changes
        );
    }

    #[test]
    fn an_empty_required_anchor_set_is_an_error() {
        let html = "\
<span><!-- MAP-TRACE-COUNT:START -->x<!-- MAP-TRACE-COUNT:END --></span>\n\
<span><!-- MAP-COUNTER:START -->x<!-- MAP-COUNTER:END --></span>\n\
<!-- MAP-MEMBERS:START -->x<!-- MAP-MEMBERS:END -->\n\
<!-- TRACE-FOOTER:START -->x<!-- TRACE-FOOTER:END -->\n\
'examples_collapsed': 'stale',\n\
<script id=\"workspace-data\">{\"nodes\":[{\"id\":\"a\",\"domain\":\"core\",\"trace\":null}],\"edges\":[],\"domains\":[{\"id\":\"core\"}]}</script>\n";
        let graph = graph_of(&[("a", false)], &[]);
        let out = render_map(html, &[], &graph, Path::new(".")).unwrap();
        assert!(
            out.problems
                .iter()
                .any(|p| p.contains("no trace-section anchors")),
            "{:?}",
            out.problems
        );
    }

    #[test]
    fn footer_and_marker_inserts_keep_crlf() {
        let y = "schema_version: 1\ntrace_id: a\ntitle: T\ndescription: D\nprimary_crate: a\n\
                 domain: core\nlifecycle_status: active\nintegration_status: wired\nowns: [a]\n\
                 doc_version: \"1\"\nlast_verified_commit: c\n";
        let mut trace = tdoc("a", y);
        trace.body = "\
<a id=\"5-active-file-map\"></a>\n## 5. Active File Map\n\
<a id=\"6-conflict-map\"></a>\n## 6. Conflict Map\n\
<a id=\"7-decision-log\"></a>\n## 7. Decision Log\n\
<a id=\"8-invariants\"></a>\n## 8. Invariants\n\
<a id=\"11-open-questions\"></a>\n## 11. Open Questions\n"
            .into();
        let html = "\
<span><!-- MAP-TRACE-COUNT:START -->x<!-- MAP-TRACE-COUNT:END --></span>\r\n\
<span><!-- MAP-COUNTER:START -->x<!-- MAP-COUNTER:END --></span>\r\n\
<!-- MAP-MEMBERS:START -->x<!-- MAP-MEMBERS:END -->\r\n\
<!-- TRACE-FOOTER:START -->\r\nstale\r\n<!-- TRACE-FOOTER:END -->\r\n\
'examples_collapsed': 'stale',\r\n\
<a href=\"#5-active-file-map\"></a><a href=\"#6-conflict-map\"></a>\
<a href=\"#7-decision-log\"></a><a href=\"#8-invariants\"></a>\
<a href=\"#11-open-questions\"></a>\r\n\
<script id=\"workspace-data\">{\"nodes\":[{\"id\":\"a\",\"domain\":\"core\",\"trace\":null}],\"edges\":[],\"domains\":[{\"id\":\"core\"}]}</script>\r\n";
        let graph = graph_of(&[("a", false)], &[]);
        let out = render_map(html, &[trace], &graph, Path::new(".")).unwrap();
        assert!(out.problems.is_empty(), "{:?}", out.problems);
        assert!(
            !out.html.replace("\r\n", "").contains('\n'),
            "lone LF leaked into a CRLF file"
        );
        assert!(out.html.contains("Subsystem Traces"));
    }

    fn anchor_problems_for(body: &str, html: &str) -> Vec<String> {
        let y = "schema_version: 1\ntrace_id: water\ntitle: T\ndescription: D\nprimary_crate: astraweave-water\n\
                 domain: physics-world\nlifecycle_status: active\nintegration_status: wired\nowns: [astraweave-water]\n\
                 doc_version: \"1\"\nlast_verified_commit: c\n";
        let mut trace = tdoc("water", y);
        trace.body = body.into();
        let linked = std::collections::BTreeSet::from(["water.md".to_string()]);
        topology::anchor_problems(html, &[trace], &linked)
    }

    fn page_links_all_five() -> &'static str {
        "<a href=\"#5-active-file-map\"></a><a href=\"#6-conflict-map\"></a>\
         <a href=\"#7-decision-log\"></a><a href=\"#8-invariants\"></a>\
         <a href=\"#11-open-questions\"></a>"
    }

    fn headings_without_invariants() -> &'static str {
        "## 5. Active File Map\n## 6. Conflict Map\n## 7. Decision Log\n## 11. Open Questions\n"
    }

    fn section_anchor_resolves(body: &str) -> bool {
        let problems = anchor_problems_for(body, page_links_all_five());
        !problems.iter().any(|p| p.contains("#8-invariants"))
    }

    #[test]
    fn a_fenced_anchor_tag_does_not_resolve() {
        let body = format!(
            "```\n<a id=\"8-invariants\"></a>\n```\n{}",
            headings_without_invariants()
        );
        let problems = anchor_problems_for(&body, page_links_all_five());
        assert!(
            problems.iter().any(|p| p.contains("#8-invariants")),
            "{problems:?}"
        );
    }

    #[test]
    fn an_inline_code_anchor_tag_does_not_resolve() {
        let body = format!(
            "`<a id=\"8-invariants\"></a>`\n{}",
            headings_without_invariants()
        );
        let problems = anchor_problems_for(&body, page_links_all_five());
        assert!(
            problems.iter().any(|p| p.contains("#8-invariants")),
            "{problems:?}"
        );
    }

    #[test]
    fn a_column_zero_anchor_after_a_list_fence_resolves() {
        let body = format!(
            "- item\n  ```\n<a id=\"8-invariants\"></a>\n  ```\n{}",
            headings_without_invariants()
        );
        assert!(
            section_anchor_resolves(&body),
            "list fence hid a column-0 anchor"
        );
    }

    #[test]
    fn a_column_zero_heading_after_a_list_fence_resolves() {
        let body = format!(
            "- item\n  ```\n## 8. Invariants\n  ```\n{}",
            headings_without_invariants()
        );
        assert!(
            section_anchor_resolves(&body),
            "list fence hid a column-0 heading"
        );
    }

    #[test]
    fn an_anchor_indented_one_to_three_spaces_resolves() {
        for indent in [" ", "  ", "   "] {
            let body = format!(
                "{indent}<a id=\"8-invariants\"></a>\n{}",
                headings_without_invariants()
            );
            assert!(
                section_anchor_resolves(&body),
                "indent {indent:?} hid the anchor"
            );
        }
    }

    #[test]
    fn an_anchor_in_the_middle_of_a_line_resolves() {
        let body = format!(
            "see <a id=\"8-invariants\"></a> in the sentence\n{}",
            headings_without_invariants()
        );
        assert!(section_anchor_resolves(&body));
    }

    #[test]
    fn a_heading_inside_a_blockquote_resolves() {
        let body = format!("> ## 8. Invariants\n{}", headings_without_invariants());
        assert!(section_anchor_resolves(&body));
    }

    #[test]
    fn a_heading_indented_four_spaces_does_not_resolve() {
        let body = format!("    ## 8. Invariants\n{}", headings_without_invariants());
        assert!(
            !section_anchor_resolves(&body),
            "indented code was counted as a heading"
        );
    }

    #[test]
    fn a_fence_closer_with_trailing_text_does_not_close() {
        let body = format!(
            "```\n```text\n<a id=\"8-invariants\"></a>\n## 8. Invariants\n```\n{}",
            headings_without_invariants()
        );
        assert!(!section_anchor_resolves(&body), "```text closed the fence");
    }

    #[test]
    fn a_code_span_continues_onto_the_next_line() {
        let body = format!(
            "`\n<a id=\"8-invariants\"></a>\n`\n{}",
            headings_without_invariants()
        );
        assert!(
            !section_anchor_resolves(&body),
            "a code span stopped at the line break"
        );
    }

    #[test]
    fn fence_and_code_span_edges() {
        let hidden = [
            (
                "unclosed fence",
                format!(
                    "```\n<a id=\"8-invariants\"></a>\n{}",
                    headings_without_invariants()
                ),
            ),
            (
                "tilde does not close a backtick fence",
                format!(
                    "```\n~~~\n<a id=\"8-invariants\"></a>\n```\n{}",
                    headings_without_invariants()
                ),
            ),
            (
                "shorter run does not close",
                format!(
                    "````\n<a id=\"8-invariants\"></a>\n```\n## 8. Invariants\n````\n{}",
                    headings_without_invariants()
                ),
            ),
            (
                "four-space closer does not close",
                format!(
                    "```\n    ```\n<a id=\"8-invariants\"></a>\n```\n{}",
                    headings_without_invariants()
                ),
            ),
            (
                "info string still opens",
                format!(
                    "```rust\n<a id=\"8-invariants\"></a>\n```\n{}",
                    headings_without_invariants()
                ),
            ),
            (
                "tilde info string may contain backticks",
                format!(
                    "~~~rust`\n<a id=\"8-invariants\"></a>\n~~~\n{}",
                    headings_without_invariants()
                ),
            ),
            (
                "shorter code span does not close",
                format!(
                    "``\n`\n<a id=\"8-invariants\"></a>\n``\n{}",
                    headings_without_invariants()
                ),
            ),
            (
                "longer code span does not close",
                format!(
                    "`\n``\n<a id=\"8-invariants\"></a>\n`\n{}",
                    headings_without_invariants()
                ),
            ),
            (
                "heading inside a table cell",
                format!(
                    "| x |\n| --- |\n| ## 8. Invariants |\n{}",
                    headings_without_invariants()
                ),
            ),
        ];
        for (name, body) in hidden {
            assert!(
                !section_anchor_resolves(&body),
                "{name} counted #8-invariants"
            );
        }

        let shown = [
            (
                "closed fence leaves a later anchor",
                format!(
                    "```\nnot an anchor\n```\n<a id=\"8-invariants\"></a>\n{}",
                    headings_without_invariants()
                ),
            ),
            (
                "longer closer ends a shorter fence",
                format!(
                    "```\nhidden\n````\n<a id=\"8-invariants\"></a>\n{}",
                    headings_without_invariants()
                ),
            ),
            (
                "closer indented more than its opener",
                format!(
                    "```\nhidden\n   ```\n<a id=\"8-invariants\"></a>\n{}",
                    headings_without_invariants()
                ),
            ),
            (
                "four-space marker does not open a fence",
                format!(
                    "    ```\n<a id=\"8-invariants\"></a>\n    ```\n{}",
                    headings_without_invariants()
                ),
            ),
            (
                "two tildes do not open a fence",
                format!(
                    "~~\n<a id=\"8-invariants\"></a>\n~~\n{}",
                    headings_without_invariants()
                ),
            ),
            (
                "two backticks do not open a fence",
                format!(
                    "``\n<a id=\"8-invariants\"></a>\n{}",
                    headings_without_invariants()
                ),
            ),
            (
                "unclosed code span stays literal",
                format!(
                    "`\n<a id=\"8-invariants\"></a>\n{}",
                    headings_without_invariants()
                ),
            ),
            (
                "blank line ends a code span",
                format!(
                    "`start\n\n<a id=\"8-invariants\"></a>\n`\n{}",
                    headings_without_invariants()
                ),
            ),
            (
                "trailing spaces still close a fence",
                format!(
                    "```\nhidden\n```   \n<a id=\"8-invariants\"></a>\n{}",
                    headings_without_invariants()
                ),
            ),
            (
                "backtick in the info string does not open",
                format!(
                    "```rust`\n<a id=\"8-invariants\"></a>\n```\n{}",
                    headings_without_invariants()
                ),
            ),
        ];
        for (name, body) in shown {
            assert!(section_anchor_resolves(&body), "{name} hid #8-invariants");
        }
    }

    #[test]
    fn an_indented_code_block_anchor_tag_does_not_resolve() {
        let body = format!(
            "    <a id=\"8-invariants\"></a>\n{}",
            headings_without_invariants()
        );
        let problems = anchor_problems_for(&body, page_links_all_five());
        assert!(
            problems.iter().any(|p| p.contains("#8-invariants")),
            "{problems:?}"
        );
    }

    #[test]
    fn a_tilde_fence_anchor_tag_does_not_resolve() {
        let body = format!(
            "~~~\n<a id=\"8-invariants\"></a>\n~~~\n{}",
            headings_without_invariants()
        );
        let problems = anchor_problems_for(&body, page_links_all_five());
        assert!(
            problems.iter().any(|p| p.contains("#8-invariants")),
            "{problems:?}"
        );
    }

    #[test]
    fn a_heading_inside_a_fence_does_not_resolve() {
        let body = format!(
            "```\n## 8. Invariants\n```\n{}",
            headings_without_invariants()
        );
        let problems = anchor_problems_for(&body, page_links_all_five());
        assert!(
            problems.iter().any(|p| p.contains("#8-invariants")),
            "{problems:?}"
        );
    }

    #[test]
    fn a_commented_anchor_tag_does_not_resolve() {
        let body = format!(
            "<!-- <a id=\"8-invariants\"></a> -->\n{}",
            headings_without_invariants()
        );
        let problems = anchor_problems_for(&body, page_links_all_five());
        assert!(
            problems.iter().any(|p| p.contains("#8-invariants")),
            "{problems:?}"
        );
    }

    #[test]
    fn a_comment_that_spans_lines_hides_the_anchor() {
        let hidden = format!(
            "<!--\n<a id=\"8-invariants\"></a>\n-->\n{}",
            headings_without_invariants()
        );
        assert!(
            !section_anchor_resolves(&hidden),
            "a multi-line comment counted the anchor"
        );
        let unclosed = format!(
            "<!-- <a id=\"8-invariants\"></a>\n{}",
            headings_without_invariants()
        );
        assert!(
            !section_anchor_resolves(&unclosed),
            "an unclosed comment counted the anchor"
        );
        let after = format!(
            "<!-- note -->\n<a id=\"8-invariants\"></a>\n{}",
            headings_without_invariants()
        );
        assert!(
            section_anchor_resolves(&after),
            "a closed comment ate the next anchor"
        );
    }

    #[test]
    fn an_anchor_inside_a_tagfiltered_tag_resolves() {
        for tag in [
            "script",
            "style",
            "textarea",
            "xmp",
            "title",
            "iframe",
            "noembed",
            "noframes",
            "plaintext",
            "SCRIPT",
        ] {
            let one = format!(
                "<{tag}><a id=\"8-invariants\"></a></{tag}>\n{}",
                headings_without_invariants()
            );
            assert!(section_anchor_resolves(&one), "{tag}");
            let split = format!(
                "<{tag}>\n<a id=\"8-invariants\"></a>\n</{tag}>\n{}",
                headings_without_invariants()
            );
            assert!(section_anchor_resolves(&split), "split {tag}");
        }
        for body in [
            "<!--><a id=\"8-invariants\"></a>\n",
            "<!---><a id=\"8-invariants\"></a>\n",
            "<!- <a id=\"8-invariants\"></a>\n",
            "<noscript><a id=\"8-invariants\"></a></noscript>\n",
            "<pre><a id=\"8-invariants\"></a></pre>\n",
        ] {
            let body = format!("{body}{}", headings_without_invariants());
            assert!(section_anchor_resolves(&body), "{body}");
        }
    }

    #[test]
    fn cdata_pi_and_attribute_values_do_not_resolve() {
        for body in [
            "<![CDATA[ <a id=\"8-invariants\"></a> ]]>\n",
            "<span title=\"<a id=&quot;8-invariants&quot;></a>\"></span>\n",
            "<div title='<a id=\"8-invariants\"></a>'></div>\n",
            "<abbr id=\"8-invariants\">x</abbr>\n",
            "<a title=\"<a id=&quot;8-invariants&quot;>\"></a>\n",
            "<a id=\"x\" id=\"8-invariants\"></a>\n",
            "</a id=\"8-invariants\">\n",
        ] {
            let body = format!("{body}{}", headings_without_invariants());
            assert!(!section_anchor_resolves(&body), "{body}");
        }
        let real = format!(
            "<a title=\"id=&quot;nope&quot;\" id=\"8-invariants\"></a>\n{}",
            headings_without_invariants()
        );
        assert!(section_anchor_resolves(&real));
        let first = format!(
            "<a id=\"8-invariants\" id=\"x\"></a>\n{}",
            headings_without_invariants()
        );
        assert!(section_anchor_resolves(&first));
        let unquoted = format!(
            "<a id=8-invariants class=z></a>\n{}",
            headings_without_invariants()
        );
        assert!(section_anchor_resolves(&unquoted));
        let pi = format!(
            "<? <a id=\"8-invariants\"></a> ?>\n{}",
            headings_without_invariants()
        );
        assert!(section_anchor_resolves(&pi), "{pi}");
    }

    #[test]
    fn an_html_block_anchor_and_a_setext_heading_resolve() {
        let block = format!(
            "<div>\n<a id=\"8-invariants\"></a>\n</div>\n{}",
            headings_without_invariants()
        );
        assert!(
            section_anchor_resolves(&block),
            "an anchor in an HTML block was dropped"
        );
        let setext = format!(
            "8 Invariants\n-------------\n{}",
            headings_without_invariants()
        );
        assert!(
            section_anchor_resolves(&setext),
            "a setext heading was dropped"
        );
        let h1 = format!(
            "8 Invariants\n=============\n{}",
            headings_without_invariants()
        );
        assert!(!section_anchor_resolves(&h1), "a setext h1 counted as h2");
        let broken = format!(
            "8 Invari\nants\n-------------\n{}",
            headings_without_invariants()
        );
        assert!(
            section_anchor_resolves(&broken),
            "a line break inside the heading became a hyphen"
        );
        let code = format!("## 8. `Invariants`\n{}", headings_without_invariants());
        assert!(
            section_anchor_resolves(&code),
            "a code span was dropped from the heading"
        );
    }

    #[test]
    fn an_attribute_that_only_ends_in_id_does_not_resolve() {
        let body = format!(
            "<a data-id=\"8-invariants\"></a>\n{}",
            headings_without_invariants()
        );
        let problems = anchor_problems_for(&body, page_links_all_five());
        assert!(
            problems.iter().any(|p| p.contains("#8-invariants")),
            "{problems:?}"
        );
    }

    #[test]
    fn a_brace_anchor_in_prose_does_not_resolve() {
        let body = format!(
            "see {{#8-invariants}} in prose\n{}",
            headings_without_invariants()
        );
        let problems = anchor_problems_for(&body, page_links_all_five());
        assert!(
            problems.iter().any(|p| p.contains("#8-invariants")),
            "{problems:?}"
        );
    }

    #[test]
    fn a_brace_anchor_after_a_heading_does_not_resolve() {
        let body = format!(
            "{}## 8. Other {{#8-invariants}}\n",
            headings_without_invariants()
        );
        let problems = anchor_problems_for(&body, page_links_all_five());
        assert!(
            problems.iter().any(|p| p.contains("#8-invariants")),
            "{problems:?}"
        );
    }

    #[test]
    fn a_trailing_underscore_stays_in_the_heading_slug() {
        let body = format!("{}## 8. Invariants_\n", headings_without_invariants());
        let problems = anchor_problems_for(&body, page_links_all_five());
        assert!(
            problems.iter().any(|p| p.contains("#8-invariants")),
            "{problems:?}"
        );
    }

    #[test]
    fn a_page_link_must_match_the_whole_anchor() {
        let html = "<a href=\"#5-active-file-map\"></a><a href=\"#6-conflict-map\"></a>\
                    <a href=\"#7-decision-logX\"></a><a href=\"#8-invariants\"></a>\
                    <a href=\"#11-open-questions\"></a>";
        let body = "## 5. Active File Map\n## 6. Conflict Map\n## 7. Decision Log\n## 8. Invariants\n## 11. Open Questions\n";
        let problems = anchor_problems_for(body, html);
        assert!(
            problems.iter().any(|p| p.contains("not #7-decision-log")),
            "{problems:?}"
        );
    }

    #[test]
    fn trace_id_and_filename_must_be_plain_identifiers() {
        let crates = vec!["astraweave-water".to_string()];
        let reject = |raw: &str| {
            let mut both = ydoc("water", "astraweave-water", "[astraweave-water]");
            both.fm.as_mut().unwrap().trace_id = raw.into();
            both.stem = raw.into();
            let problems = validate(&[both], &crates);
            assert!(
                problems.iter().any(|m| m.contains("trace_id must match")),
                "{raw:?} {problems:?}"
            );
            assert!(
                problems.iter().any(|m| m.contains("file name must match")),
                "{raw:?} {problems:?}"
            );

            let mut file_only = ydoc("water", "astraweave-water", "[astraweave-water]");
            file_only.stem = raw.into();
            let problems = validate(&[file_only], &crates);
            assert!(
                problems.iter().any(|m| m.contains("file name must match")),
                "{raw:?} {problems:?}"
            );

            let mut id_only = ydoc("water", "astraweave-water", "[astraweave-water]");
            id_only.fm.as_mut().unwrap().trace_id = raw.into();
            let problems = validate(&[id_only], &crates);
            assert!(
                problems.iter().any(|m| m.contains("trace_id must match")),
                "{raw:?} {problems:?}"
            );
        };
        for raw in [
            "bad\"id<x>",
            "Bad",
            "bad-id",
            "bad.id",
            "bad/id",
            "bad id",
            "",
        ] {
            reject(raw);
        }

        let ok = ydoc("ai_pipeline2", "astraweave-water", "[astraweave-water]");
        let problems = validate(&[ok], &crates);
        assert!(
            problems.iter().all(|m| !m.contains("must match")),
            "{problems:?}"
        );
    }

    fn workspace_map_html() -> String {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/architecture/workspace_map.html");
        std::fs::read_to_string(&path).unwrap_or_else(|err| {
            panic!("read {}: {err}", path.display());
        })
    }

    #[test]
    fn workspace_map_has_no_html_parsing_sink() {
        let html = workspace_map_html();
        let problems = super::js_sink::html_parsing_problems(&html);
        assert!(
            problems.is_empty(),
            "workspace_map.html parses HTML: {problems:?}"
        );
    }

    #[test]
    fn html_parsing_sink_rejects_every_assignment_form() {
        let sinks = [
            "node.innerHTML = x",
            "node.innerHTML=x",
            "node.innerHTML += x",
            "node.innerHTML+=x",
            "node.outerHTML = x",
            "node.outerHTML += x",
            "node.insertAdjacentHTML('beforeend', x)",
            "node['innerHTML'] = x",
            "node[\"outerHTML\"] += x",
            "node['inner'+'HTML'] = x",
            "node['outer'+'HTML'] += x",
            "node['inner'+'HT'+'ML'] = x",
            "node['inner' + 'HT' + 'ML'] = x",
            "range.createContextualFragment(x)",
            "new DOMParser().parseFromString(x, 'text/html')",
            "iframe.srcdoc = x",
            "node.setAttribute('srcdoc', x)",
            "setTimeout('alert(1)')",
            "setTimeout(\"alert(1)\", 10)",
            "window.setInterval(`alert(1)`)",
            "document.write(x)",
            "document.writeln(x)",
            "eval(x)",
            "eval (x)",
            "new Function('return 1')",
            "Function (x)",
        ];
        for sample in sinks {
            assert!(
                html_parsing_sink(sample).is_some(),
                "{sample} was not a sink"
            );
        }
        let safe = [
            "node.textContent = String(props.text)",
            "node.appendChild(document.createTextNode(String(kid)))",
            "function safeColor(value) { return ''; }",
            "setTimeout(() => {}, 10)",
            "setTimeout(function () {}, 1)",
            "setTimeout(tryRestore, 100)",
        ];
        for sample in safe {
            assert!(html_parsing_sink(sample).is_none(), "{sample} was a sink");
        }
    }

    fn assert_sink(sample: &str) {
        assert!(
            html_parsing_sink(sample).is_some(),
            "{sample} was not a sink"
        );
    }

    #[test]
    fn sink_set_html_unsafe() {
        assert_sink("node.setHTMLUnsafe(x)");
    }

    #[test]
    fn sink_parse_html_unsafe() {
        assert_sink("Document.parseHTMLUnsafe(x)");
    }

    #[test]
    fn sink_exec_command_insert_html() {
        assert_sink("document.execCommand('insertHTML', false, x)");
    }

    #[test]
    fn sink_concat_eval() {
        assert_sink("window['ev'+'al'](x)");
    }

    #[test]
    fn sink_concat_timer() {
        assert_sink("window['set'+'Timeout']('alert(1)')");
    }

    #[test]
    fn sink_constructor_constructor() {
        assert_sink("[].constructor.constructor('return 1')()");
    }

    #[test]
    fn sink_parse_from_string_variable_mime() {
        assert_sink("parser.parseFromString(x, mime)");
        assert_sink("new DOMParser().parseFromString(x, 'text/xml')");
    }

    #[test]
    fn sink_event_handler_attribute_concat() {
        assert_sink("node.setAttribute('on'+'click', x)");
    }

    #[test]
    fn sink_variable_plus_html_key() {
        assert_sink("const w='inner'; node[w+'HTML'] = x");
    }

    #[test]
    fn sink_template_key() {
        assert_sink("node[`${'inner'}HTML`] = x");
    }

    #[test]
    fn sink_hex_escape() {
        assert_sink(r"node['\x69nnerHTML'] = x");
    }

    #[test]
    fn sink_unicode_ident_escape() {
        assert_sink(r"node.inner\u0048TML = x");
    }

    #[test]
    fn sink_bracket_document_write() {
        assert_sink("document['write'](x)");
    }

    #[test]
    fn sink_spaced_document_write() {
        assert_sink("document . write(x)");
    }

    #[test]
    fn sink_indirect_eval() {
        assert_sink("(0, eval)(x)");
    }

    fn matching_brace(html: &str, open: usize) -> usize {
        let bytes = html.as_bytes();
        let mut depth = 0;
        let mut i = open;
        while i < bytes.len() {
            match bytes[i] {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        return i;
                    }
                }
                _ => {}
            }
            i += 1;
        }
        panic!("brace at {open} did not close");
    }

    /// `el` through `appendMarkup`, the helpers the page uses to build text.
    fn map_guard_source(html: &str) -> String {
        let start = html
            .find("function el(tag, props, kids)")
            .expect("workspace_map.html has no el");
        let rel = html[start..]
            .find("function appendMarkup(")
            .expect("workspace_map.html has no appendMarkup");
        let fn_at = start + rel;
        let brace = html[fn_at..].find('{').expect("appendMarkup has no body") + fn_at;
        let end = matching_brace(html, brace);
        html[start..=end].to_string()
    }

    #[test]
    fn map_color_risk_status_and_loc_guards_hold() {
        let source = map_guard_source(&workspace_map_html());
        let mut script = String::from(
            r#"
const HTML_PARSE = new Set(['innerHTML', 'outerHTML', 'insertAdjacentHTML', 'srcdoc']);
function reject(name) { throw new Error('html parse: ' + name); }
function makeNode(tag) {
  const node = {
    tagName: String(tag).toUpperCase(),
    className: '',
    id: '',
    title: '',
    textContent: '',
    disabled: false,
    children: [],
    attrs: {},
    style: {},
    setAttribute(key, value) {
      const name = String(key).toLowerCase();
      if (HTML_PARSE.has(name) || name.startsWith('on')) reject(name);
      this.attrs[name] = String(value);
    },
    appendChild(kid) { this.children.push(kid); return kid; },
    replaceChildren() { this.children.length = 0; },
    insertAdjacentHTML() { reject('insertAdjacentHTML'); },
  };
  for (const prop of ['innerHTML', 'outerHTML']) {
    Object.defineProperty(node, prop, {
      configurable: true,
      set() { reject(prop); },
      get() { return ''; },
    });
  }
  const style = node.style;
  node.style = new Proxy(style, {
    set(obj, key, value) {
      const name = String(key);
      if (name === 'cssText' || HTML_PARSE.has(name)) reject(name);
      obj[name] = String(value);
      return true;
    },
  });
  return node;
}
const ALLOWED_TAGS = new Set(['span', 'strong', 'em', 'code', 'div']);
const document = {
  createElement(tag) {
    const name = String(tag).toLowerCase();
    if (!ALLOWED_TAGS.has(name)) reject('createElement(' + name + ')');
    return makeNode(tag);
  },
  createTextNode(text) { return { nodeType: 3, text: String(text) }; },
};
"#,
        );
        script.push_str(&source);
        script.push_str(
            r#"
const assert = require('node:assert');
assert.strictEqual(safeColor('#abc'), '#abc');
assert.strictEqual(safeColor('#abcd'), '#abcd');
assert.strictEqual(safeColor('#abcdef'), '#abcdef');
assert.strictEqual(safeColor('#ABCDEFff'), '#ABCDEFff');
for (const bad of ['red', 'rgb(0,0,0)', '#gggggg', '#fff;color:red', '#ff', '#abcde', '#1234567', ' #fff', '#fff ', '', null, 12]) {
  assert.strictEqual(safeColor(bad), '', 'safeColor accepted ' + bad);
}
for (const risk of ['CRITICAL', 'HIGH', 'MEDIUM', 'LOW']) {
  const cls = riskPill(risk).className;
  assert.ok(cls.split(/\s+/).includes(risk), cls);
  assert.ok(cls.startsWith('risk-pill'), cls);
}
for (const risk of ['critical', 'LOW;x', '<img>', 'HIGH HIGH', '', 'constructor', '__proto__', 'toString']) {
  const cls = riskPill(risk).className;
  assert.strictEqual(cls, 'risk-pill', risk + ' -> ' + cls);
}
for (const status of ['active', 'dormant', 'partial']) {
  const cls = statusPill(status).className;
  assert.ok(cls.split(/\s+/).includes(status), cls);
}
for (const status of ['ACTIVE', 'active x', '<svg>', 'dormant;', 'constructor', '__proto__']) {
  const cls = statusPill(status).className;
  assert.strictEqual(cls, 'status-pill', status + ' -> ' + cls);
}
const shown = locText('medium', 2231);
assert.ok(shown.startsWith('medium (~'), shown);
assert.notStrictEqual(shown, 'medium');
assert.strictEqual(locText('medium', Infinity), 'medium');
assert.strictEqual(locText('medium', -Infinity), 'medium');
assert.strictEqual(locText('medium', NaN), 'medium');
assert.strictEqual(locText('medium', 'nope'), 'medium');
assert.strictEqual(locText('medium', '1e309'), 'medium');
assert.strictEqual(locText('medium', 0), 'medium');
assert.strictEqual(locText('small', null), 'small');
assert.strictEqual(dotSpan('dot', '#abc').style.background, '#abc');
assert.strictEqual(dotSpan('dot', 'red', '#fff').style.background, '#fff');
assert.strictEqual(dotSpan('dot', 'red').style.background, undefined);
assert.strictEqual(dotSpan('dot', 'expression(alert(1))', '#fff;color:red').style.background, undefined);
const host = document.createElement('div');
appendKids(host, ['a', document.createTextNode('b'), null, false]);
assert.strictEqual(host.children.length, 2);
const parent = document.createElement('div');
appendMarkup(parent, 'Hello <strong>go</strong> <script>alert(1)</script> <em>e</em> <code>c</code>');
appendMarkup(parent, '<constructor>no</constructor><__proto__>no</__proto__>');
function tagsOf(node, out) {
  if (node.tagName) out.push(node.tagName.toLowerCase());
  for (const kid of node.children || []) tagsOf(kid, out);
}
const tags = [];
tagsOf(parent, tags);
assert.deepStrictEqual(tags.filter(tag => tag !== 'div'), ['strong', 'em', 'code']);
const texts = [];
function textsOf(node) {
  if (node.nodeType === 3) texts.push(node.text);
  for (const kid of node.children || []) textsOf(kid);
}
textsOf(parent);
assert.ok(texts.some(text => text.includes('<script>')), texts.join('|'));
assert.ok(texts.some(text => text.includes('<constructor>')), texts.join('|'));
"#,
        );
        let output = std::process::Command::new("node")
            .arg("-")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap_or_else(|err| panic!("spawn node: {err}"));
        use std::io::Write;
        let mut child = output;
        child
            .stdin
            .as_mut()
            .unwrap()
            .write_all(script.as_bytes())
            .unwrap();
        let finished = child.wait_with_output().unwrap();
        assert!(
            finished.status.success(),
            "map guards failed\n{}",
            String::from_utf8_lossy(&finished.stderr)
        );
    }

    #[test]
    fn the_dialogue_file_row_matches_llm_dialogue_rs() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let doc = std::fs::read_to_string(root.join("docs/architecture/ai_pipeline.md")).unwrap();
        let table = doc
            .split("#### 13.4.4 Files involved")
            .nth(1)
            .expect("dialogue file table")
            .split("**Dependencies")
            .next()
            .unwrap();
        let rows = [
            ("| `lib.rs` |", "astraweave-dialogue/src/lib.rs"),
            ("| `runner.rs` |", "astraweave-dialogue/src/runner.rs"),
            (
                "| `toml_loader.rs` |",
                "astraweave-dialogue/src/toml_loader.rs",
            ),
            (
                "| `llm_dialogue.rs` |",
                "astraweave-dialogue/src/llm_dialogue.rs",
            ),
            (
                "| `tests/dialogue.rs` |",
                "astraweave-dialogue/tests/dialogue.rs",
            ),
            (
                "| `tests/mutation_resistant_comprehensive_tests.rs` |",
                "astraweave-dialogue/tests/mutation_resistant_comprehensive_tests.rs",
            ),
            (
                "| `benches/dialogue_bench.rs` |",
                "astraweave-dialogue/benches/dialogue_bench.rs",
            ),
        ];
        let mut sum = 0usize;
        let mut src_lines = 0usize;
        for (prefix, path) in rows {
            let lines = std::fs::read_to_string(root.join(path))
                .unwrap()
                .lines()
                .count();
            let row = table
                .lines()
                .find(|line| line.starts_with(prefix))
                .unwrap_or_else(|| panic!("missing {prefix}"));
            assert_eq!(table_loc(row), lines, "{row}");
            sum += lines;
            if path.contains("/src/") {
                src_lines += lines;
            }
        }
        let totals = table
            .lines()
            .find(|line| line.starts_with("| **Totals** |"))
            .expect("dialogue totals row");
        assert_eq!(table_loc(totals), sum, "{totals}");

        let llm = std::fs::read_to_string(root.join("astraweave-dialogue/src/llm_dialogue.rs"))
            .unwrap()
            .lines()
            .count();
        let llm_g = grouped(llm);
        let src_g = grouped(src_lines);
        assert!(doc.contains(&format!("`LlmDialogueSystem` ({llm_g} lines, dormant)")));
        assert!(doc.contains(&format!(
            "{llm_g} lines, about half of the {src_g} under `src/`"
        )));
        assert!(doc.contains(&format!("The entire {llm_g}-line `llm_dialogue.rs`")));
        assert!(doc.contains(&format!("the {llm_g}-line LLM-enhanced layer")));
        assert!(!doc.contains("2941"), "stale llm_dialogue.rs size");
        assert!(
            !doc.contains("60% of the crate by LoC"),
            "stale share of src/"
        );

        let mut test_sum = 0usize;
        for (prefix, path) in rows {
            let text = std::fs::read_to_string(root.join(path)).unwrap();
            let tests = test_attr_count(&text);
            let row = table
                .lines()
                .find(|line| line.starts_with(prefix))
                .unwrap_or_else(|| panic!("missing {prefix}"));
            assert_eq!(table_cell(row, 3), tests, "{row}");
            test_sum += tests;
        }
        assert_eq!(table_cell(totals, 3), test_sum, "{totals}");
        assert_eq!(
            test_attr_count(
                &std::fs::read_to_string(root.join("astraweave-dialogue/src/llm_dialogue.rs"))
                    .unwrap()
            ),
            6
        );
        assert_eq!(
            test_attr_count(
                &std::fs::read_to_string(root.join("astraweave-dialogue/src/toml_loader.rs"))
                    .unwrap()
            ),
            10
        );

        let lib_lines = std::fs::read_to_string(root.join("astraweave-dialogue/src/lib.rs"))
            .unwrap()
            .lines()
            .count();
        let toml_text =
            std::fs::read_to_string(root.join("astraweave-dialogue/src/toml_loader.rs")).unwrap();
        let toml_lines = toml_text.lines().count();
        let toml_tests = test_attr_count(&toml_text);
        let bench_lines =
            std::fs::read_to_string(root.join("astraweave-dialogue/benches/dialogue_bench.rs"))
                .unwrap()
                .lines()
                .count();
        let section_five = |prefix: &str| {
            doc.lines()
                .find(|line| line.starts_with(prefix))
                .unwrap_or_else(|| panic!("missing {prefix}"))
                .to_string()
        };
        let lib_row = section_five("| `astraweave-dialogue/src/lib.rs`");
        assert!(
            lib_row.contains(&format!("({} LoC)", grouped(lib_lines))),
            "{lib_row}"
        );
        let toml_row = section_five("| `astraweave-dialogue/src/toml_loader.rs`");
        assert!(
            toml_row.contains(&format!("({} LoC)", grouped(toml_lines))),
            "{toml_row}"
        );
        assert!(
            toml_row.contains(&format!("{toml_tests} inline tests")),
            "{toml_row}"
        );
        let bench_row = section_five("| `astraweave-dialogue/benches/dialogue_bench.rs`");
        assert!(
            bench_row.contains(&format!("({} LoC)", grouped(bench_lines))),
            "{bench_row}"
        );

        let llm_src =
            std::fs::read_to_string(root.join("astraweave-dialogue/src/llm_dialogue.rs")).unwrap();
        let struct_at = llm_src
            .lines()
            .position(|line| line.contains("pub struct LlmDialogueSystem"))
            .expect("LlmDialogueSystem")
            + 1;
        let struct_end = struct_at
            + llm_src
                .lines()
                .skip(struct_at - 1)
                .position(|line| line.trim() == "}")
                .expect("struct end");
        let cite = format!("llm_dialogue.rs:{struct_at}-{struct_end}");
        assert!(doc.contains(&cite), "missing {cite}");
        assert!(!doc.contains("llm_dialogue.rs:31-49"), "stale struct cite");
        let llm_tests = test_attr_count(&llm_src);
        assert!(doc.contains(&format!("The {llm_tests} inline tests exercise the system")));
        let dormancy = doc
            .lines()
            .find(|line| line.contains("plus tokio"))
            .expect("dependency sentence");
        assert!(
            dormancy.contains("tokio + async-trait + regex"),
            "{dormancy}"
        );
        assert!(!dormancy.contains("regex + rand"), "stale rand dependency");
        assert!(dormancy.contains(&format!(
            "{llm_tests} inline tests exercise it in isolation"
        )));
        let cargo = std::fs::read_to_string(root.join("astraweave-dialogue/Cargo.toml")).unwrap();
        assert!(cargo.contains("tokio"));
        assert!(cargo.contains("async-trait"));
        assert!(cargo.contains("regex"));
        assert!(!cargo.lines().any(|line| line.contains("rand")));
        assert!(doc.contains(&format!("Dialogue (§13.4) had {test_sum} tests")));

        let section = doc
            .split("### 13.4 ")
            .nth(1)
            .expect("§13.4")
            .split("### 13.5 ")
            .next()
            .unwrap();
        assert!(
            !section
                .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
                .any(|tok| tok == "rand"),
            "§13.4 still names rand"
        );
        assert!(section.contains("SimRng"), "§13.4 does not name SimRng");
        let recorded = llm_src
            .lines()
            .position(|line| line.starts_with("pub fn recorded_line"))
            .expect("recorded_line")
            + 1;
        assert!(
            section.contains(&format!("llm_dialogue.rs:{recorded}")),
            "missing recorded_line cite"
        );
        assert!(section.contains("game_loop.rs:17"));
        for line in section.lines() {
            if line.contains("game_loop.rs:446") {
                assert!(
                    line.contains("#[cfg(test)]"),
                    "unmarked test-only cite: {line}"
                );
            }
        }
    }

    #[test]
    fn the_fluids_figures_match_the_crate() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let lib_lines = std::fs::read_to_string(root.join("astraweave-fluids/src/lib.rs"))
            .unwrap()
            .lines()
            .count();
        let kernels = std::fs::read_to_string(root.join("astraweave-fluids/shaders/fluid.wgsl"))
            .unwrap()
            .lines()
            .filter(|line| line.contains("@compute"))
            .count();
        let doc = std::fs::read_to_string(root.join("docs/architecture/fluids.md")).unwrap();
        let row = doc
            .lines()
            .find(|line| line.starts_with("| `astraweave-fluids/src/lib.rs` |"))
            .expect("fluids lib.rs row");
        assert_eq!(table_loc(row), lib_lines, "{row}");
        let vocab = doc
            .lines()
            .find(|line| line.contains("| **`FluidSystem`** |"))
            .expect("FluidSystem vocabulary row");
        assert!(
            vocab.contains(&format!("Holds {kernels} compute pipelines")),
            "{vocab}"
        );
        assert!(!vocab.contains("8 compute pipelines"), "{vocab}");
        let lib = std::fs::read_to_string(root.join("astraweave-fluids/src/lib.rs")).unwrap();
        let struct_at = lib
            .lines()
            .position(|line| line.contains("pub struct FluidSystem"))
            .expect("FluidSystem")
            + 1;
        let struct_end = struct_at
            + lib
                .lines()
                .skip(struct_at - 1)
                .position(|line| line == "}")
                .expect("struct end");
        let new_at = lib
            .lines()
            .position(|line| {
                line.contains("pub fn new(device: &wgpu::Device, particle_count: u32)")
            })
            .expect("FluidSystem::new")
            + 1;
        assert!(
            vocab.contains(&format!("lib.rs:{struct_at}-{struct_end}")),
            "{vocab}"
        );
        assert!(vocab.contains(&format!("`:{new_at}`")), "{vocab}");
        assert!(!vocab.contains("lib.rs:250-415"), "{vocab}");
        let wgsl_lines = std::fs::read_to_string(root.join("astraweave-fluids/shaders/fluid.wgsl"))
            .unwrap()
            .lines()
            .count();
        let flow = doc
            .split("### 2.1 ")
            .nth(1)
            .expect("§2.1")
            .split("### 2.2 ")
            .next()
            .unwrap();
        assert!(
            flow.contains(&format!("lib.rs:{new_at}")),
            "§2.1 does not cite new"
        );
        assert!(
            flow.contains(&format!("{wgsl_lines} LoC")),
            "§2.1 shader size"
        );
        assert!(
            flow.contains(&format!("{kernels} compute pipelines"))
                || flow.contains(&format!("dispatch all {kernels} pipelines")),
            "{flow}"
        );
        assert!(flow.contains("Seven compute pipelines"), "{flow}");
        for stale in [
            "8-stage",
            "Eight compute",
            "8 compute",
            "dispatch all 8",
            "481 LoC",
            "secondary spawn",
            "lib.rs:362",
        ] {
            assert!(!flow.contains(stale), "§2.1 still says {stale}");
        }
        assert!(!doc.contains("lib.rs:362"), "constructor cite is still 362");
        let status = doc
            .lines()
            .find(|line| line.starts_with("| **Status** |"))
            .expect("fluids status row");
        assert!(status.contains("`examples/fluids_demo`"), "{status}");
        assert!(status.contains("`examples/weaving_playground`"), "{status}");
        assert!(
            !status.contains("fluids_demo` only"),
            "status still names one consumer"
        );
        for example in ["examples/fluids_demo", "examples/weaving_playground"] {
            let manifest = std::fs::read_to_string(root.join(example).join("Cargo.toml")).unwrap();
            assert!(
                manifest.contains("astraweave-fluids"),
                "{example} does not depend on astraweave-fluids"
            );
        }
        let note = doc
            .split("**Status note (read first):**")
            .nth(1)
            .expect("status note")
            .split("\n## ")
            .next()
            .unwrap();
        assert!(note.contains("`examples/fluids_demo`"), "{note}");
        assert!(note.contains("`examples/weaving_playground`"), "{note}");
        assert!(!note.contains("ONLY consumer"), "{note}");
        let pbd = doc
            .split("### Decision: Use Position-Based Dynamics")
            .nth(1)
            .expect("PBD decision")
            .split("### Decision:")
            .next()
            .unwrap();
        assert!(pbd.contains("**Status:** Historical"), "{pbd}");
        assert!(pbd.contains("7 compute pipelines"), "{pbd}");
        assert!(!pbd.contains("**Status:** Active"), "{pbd}");
        let eight = doc
            .split("### Decision: Eight-stage")
            .nth(1)
            .expect("eight-stage decision")
            .split("### Decision:")
            .next()
            .unwrap();
        assert!(eight.contains("**Status:** Historical"), "{eight}");
        assert!(
            eight.contains("removed the ping-pong particle buffer"),
            "{eight}"
        );
        assert!(!eight.contains("**Status:** Active"), "{eight}");
        assert!(doc.contains("`wgpu::ComputePipeline` (7 PBD pipelines)"));
        assert!(!doc.contains("8 PBD pipelines"));
        let mut manifests = Vec::new();
        fluids_manifests(&root, &root, &mut manifests);
        manifests.sort();
        assert_eq!(
            manifests,
            vec![
                "astraweave-fluids/Cargo.toml".to_string(),
                "examples/fluids_demo/Cargo.toml".to_string(),
                "examples/weaving_playground/Cargo.toml".to_string(),
            ]
        );
    }

    fn fluids_manifests(root: &std::path::Path, dir: &std::path::Path, out: &mut Vec<String>) {
        let entries = match std::fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(_) => return,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name == "target" || name == ".git" || name == "node_modules" {
                continue;
            }
            if path.is_dir() {
                fluids_manifests(root, &path, out);
                continue;
            }
            if name != "Cargo.toml" {
                continue;
            }
            let text = std::fs::read_to_string(&path).unwrap();
            let named = text.lines().any(|line| {
                line.split('#')
                    .next()
                    .unwrap()
                    .contains("name = \"astraweave-fluids\"")
            });
            let dep = text.lines().any(|line| {
                line.split('#')
                    .next()
                    .unwrap()
                    .trim_start()
                    .starts_with("astraweave-fluids")
            });
            if named || dep {
                let rel = path
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                out.push(rel);
            }
        }
    }

    fn test_attr_count(text: &str) -> usize {
        text.lines()
            .filter(|line| {
                let line = line.trim();
                line == "#[test]" || line == "#[tokio::test]"
            })
            .count()
    }

    fn table_cell(row: &str, index: usize) -> usize {
        let cell = row
            .split('|')
            .nth(index)
            .unwrap()
            .trim()
            .trim_matches('*')
            .trim();
        let token = cell.split_whitespace().next().unwrap().replace(',', "");
        token.parse().unwrap_or_else(|_| panic!("{row}"))
    }

    #[test]
    fn the_editor_dock_range_matches_the_file() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let lines = std::fs::read_to_string(root.join("tools/aw_editor/src/dock_panels.rs"))
            .unwrap()
            .lines()
            .count();
        let doc = std::fs::read_to_string(root.join("docs/architecture/aw_editor.md")).unwrap();
        let cite = format!("dock_panels.rs:1-{lines}");
        assert!(doc.contains(&cite), "missing {cite}");
        assert!(
            !doc.contains("dock_panels.rs:1-626"),
            "stale dock_panels range"
        );
        let row = doc
            .lines()
            .find(|line| line.starts_with("| `src/dock_panels.rs`"))
            .expect("dock_panels file-map row");
        assert!(row.contains(&format!("{lines} lines")), "{row}");
        assert!(!row.contains("626 lines"), "{row}");
    }

    fn table_loc(row: &str) -> usize {
        let cell = row
            .split('|')
            .nth(2)
            .unwrap()
            .trim()
            .trim_matches('*')
            .trim();
        let token = cell.split_whitespace().next().unwrap().replace(',', "");
        token.parse().expect(row)
    }

    fn grouped(n: usize) -> String {
        let raw = n.to_string();
        let mut out = String::new();
        for (i, ch) in raw.chars().rev().enumerate() {
            if i > 0 && i % 3 == 0 {
                out.push(',');
            }
            out.push(ch);
        }
        out.chars().rev().collect()
    }

    #[test]
    fn footer_escapes_markup_characters_in_the_identifier() {
        let raw = "bad\"id<x>";
        let mut trace = ydoc("water", "astraweave-water", "[]");
        trace.stem = raw.into();
        trace.fm.as_mut().unwrap().trace_id = raw.into();
        let html = topology::footer_block(&[trace]);
        assert!(
            !html.contains(raw),
            "unescaped identifier reached the footer"
        );
        assert_eq!(html.matches("bad&quot;id&lt;x&gt;").count(), 2);
    }
}
