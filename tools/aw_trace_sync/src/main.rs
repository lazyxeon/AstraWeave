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
/// so generated output is byte-stable and `--check` stays clean.
fn escape_nonascii(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        let cp = c as u32;
        if cp < 0x80 {
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
    runtime_edges: Vec<(String, String)>,                         // (from, to)
    /// Integrity, anchor, and nodeless-owner problems. Non-empty => nothing may be written.
    problems: Vec<String>,
    missing_nodes: Vec<String>,
    extra_nodes: Vec<String>,
    missing_edges: Vec<String>,
    extra_edges: Vec<String>,
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

    let mut data: serde_json::Value =
        serde_json::from_str(json_str).context("parsing workspace-data JSON")?;
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
    let mut html = format!(
        "{}{}{}",
        &current[..content_start],
        new_json,
        &current[close..]
    );
    if chrome {
        let traced = traces.iter().filter(|t| t.fm.is_some()).count();
        let examples = graph.packages.iter().filter(|p| p.is_example).count();
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
    let rt_report: Vec<(String, String)> = rt_edges
        .iter()
        .map(|(_, from, to, _)| (from.clone(), to.clone()))
        .collect();
    Ok(MapRender {
        html,
        changed_links,
        changed_status,
        runtime_edges: rt_report,
        problems,
        missing_nodes: topo.missing_nodes,
        extra_nodes: topo.extra_nodes,
        missing_edges: topo.missing_edges,
        extra_edges: topo.extra_edges,
    })
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
                for (from, to) in &map_render.runtime_edges {
                    eprintln!("  runtime edge {from} -> {to}");
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
mod tests {
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
}
