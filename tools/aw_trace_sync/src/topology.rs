//! Cargo-derived map topology.
//!
//! Node ids (every non-example workspace package, plus one `examples_collapsed`
//! aggregate) and normal path-dependency edges come from `cargo metadata`.
//! Dev-dependencies and build-dependencies are not edges. Hand-curated fields
//! on an existing node or edge are kept: descriptions live outside this JSON,
//! and `silentFailures`, stories, edge `types`, and `declared-unused` /
//! `anomalous` annotations stay put. `optional` is cargo's value.
//!
//! An `examples_collapsed` edge is optional only when every example's normal
//! dependency on that target is optional. One required example makes the
//! aggregate edge required.
//!
//! Runtime edges are not cargo edges. `from` (default `primary_crate`) → `to`
//! follows the map's "source depends on target" direction. A runtime edge that
//! matches a cargo edge sets `runtime` and `runtimeNote` on that edge and
//! leaves its cargo kind. A runtime edge with no cargo dependency is kind
//! `runtime`, never `workspace-dep`.

use anyhow::{Context, Result};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::{domain_id, FrontMatter, TraceDoc};

#[derive(Clone, Debug)]
pub struct Pkg {
    pub name: String,
    pub is_example: bool,
    /// Workspace-relative manifest path, `/` separators.
    pub manifest: String,
}

#[derive(Clone, Debug, Default)]
pub struct Graph {
    pub packages: Vec<Pkg>,
    /// `(from, to) → optional`. Optional is true only when every normal
    /// declaration of that pair is optional.
    pub deps: BTreeMap<(String, String), bool>,
}

#[derive(Clone, Debug, Default)]
pub struct TopologyDiff {
    pub missing_nodes: Vec<String>,
    pub extra_nodes: Vec<String>,
    pub missing_edges: Vec<String>,
    pub extra_edges: Vec<String>,
    /// `source -> target optional old -> new`
    pub optional_changes: Vec<String>,
    /// Membership or order changes in `examples_collapsed.exampleList`.
    pub example_list: Vec<String>,
    /// `id old_loc/old_band -> new_loc/new_band`
    pub loc_changes: Vec<String>,
}

pub fn workspace_graph(root: &Path) -> Result<Graph> {
    let meta = cargo_metadata::MetadataCommand::new()
        .manifest_path(root.join("Cargo.toml"))
        .no_deps()
        .exec()
        .context("running cargo metadata")?;
    let ws: BTreeSet<_> = meta.workspace_members.iter().cloned().collect();
    let root_s = root.to_string_lossy().replace('\\', "/");
    let root_s = root_s.trim_end_matches('/').to_string();

    let mut dir_to_name: BTreeMap<String, String> = BTreeMap::new();
    let mut packages = Vec::new();
    for p in &meta.packages {
        if !ws.contains(&p.id) {
            continue;
        }
        let manifest_abs = p.manifest_path.as_str().replace('\\', "/");
        let manifest = strip_root(&root_s, &manifest_abs);
        let dir = p
            .manifest_path
            .parent()
            .map(|d| d.as_str().replace('\\', "/"))
            .unwrap_or_default();
        dir_to_name.insert(dir.trim_end_matches('/').to_string(), p.name.to_string());
        packages.push(Pkg {
            name: p.name.to_string(),
            is_example: is_example_manifest(&manifest),
            manifest,
        });
    }
    packages.sort_by(|a, b| a.name.cmp(&b.name));
    packages.dedup_by(|a, b| a.name == b.name);
    let names: BTreeSet<String> = packages.iter().map(|p| p.name.clone()).collect();

    let mut raw: BTreeMap<(String, String), Vec<bool>> = BTreeMap::new();
    for p in &meta.packages {
        if !ws.contains(&p.id) {
            continue;
        }
        let from = p.name.to_string();
        for d in &p.dependencies {
            if d.kind != cargo_metadata::DependencyKind::Normal {
                continue;
            }
            let Some(path) = &d.path else { continue };
            let dir = path.as_str().replace('\\', "/");
            let dir = dir.trim_end_matches('/').to_string();
            let to = if let Some(name) = dir_to_name.get(&dir) {
                name.clone()
            } else if names.contains(&d.name) {
                d.name.clone()
            } else {
                continue;
            };
            if to == from {
                continue;
            }
            raw.entry((from.clone(), to)).or_default().push(d.optional);
        }
    }
    let mut deps = BTreeMap::new();
    for (k, flags) in raw {
        deps.insert(k, flags.iter().all(|o| *o));
    }
    Ok(Graph { packages, deps })
}

fn strip_root(root: &str, abs: &str) -> String {
    abs.strip_prefix(root)
        .unwrap_or(abs)
        .trim_start_matches('/')
        .to_string()
}

fn is_example_manifest(manifest: &str) -> bool {
    manifest.starts_with("examples/") || manifest.contains("/examples/")
}

/// Crates a trace owns that are workspace packages and still have no node
/// after generation. Example crates are collapsed, so owning one is an error.
pub fn owned_without_node(
    traces: &[TraceDoc],
    node_ids: &BTreeSet<String>,
    graph: &Graph,
) -> Vec<String> {
    let workspace: BTreeSet<&str> = graph.packages.iter().map(|p| p.name.as_str()).collect();
    let mut problems = Vec::new();
    for t in traces {
        let Some(fm) = &t.fm else { continue };
        for c in &fm.owns {
            if !workspace.contains(c.as_str()) {
                continue;
            }
            if !node_ids.contains(c) {
                problems.push(format!(
                    "{}: owns '{c}' but that crate has no map node",
                    t.file()
                ));
            }
        }
    }
    problems
}

pub fn sync_topology(
    data: &mut Value,
    graph: &Graph,
    root: &Path,
    traces: &[TraceDoc],
) -> Result<TopologyDiff> {
    let mut diff = TopologyDiff::default();
    let non_example: BTreeMap<&str, &Pkg> = graph
        .packages
        .iter()
        .filter(|p| !p.is_example)
        .map(|p| (p.name.as_str(), p))
        .collect();
    let examples: Vec<&Pkg> = graph.packages.iter().filter(|p| p.is_example).collect();
    let labels = domain_labels(data);

    let node_ids: BTreeSet<String> = {
        let nodes = data
            .get_mut("nodes")
            .and_then(|n| n.as_array_mut())
            .context("workspace-data has no `nodes` array")?;

        let mut by_id: BTreeMap<String, Value> = BTreeMap::new();
        for n in nodes.drain(..) {
            if let Some(id) = n.get("id").and_then(|v| v.as_str()) {
                by_id.insert(id.to_string(), n);
            }
        }

        let mut next_nodes = Vec::new();
        for (name, pkg) in &non_example {
            if let Some(mut existing) = by_id.remove(*name) {
                refresh_loc(&mut existing, root, pkg, &mut diff);
                next_nodes.push(existing);
            } else {
                diff.missing_nodes.push((*name).to_string());
                next_nodes.push(new_node(name, pkg, traces, &labels, root));
            }
        }

        if !examples.is_empty() {
            let names: Vec<String> = examples.iter().map(|p| p.name.clone()).collect();
            if let Some(mut existing) = by_id.remove("examples_collapsed") {
                record_example_list(&existing, &names, &mut diff);
                refresh_examples_node(&mut existing, &names);
                next_nodes.push(existing);
            } else {
                diff.missing_nodes.push("examples_collapsed".to_string());
                next_nodes.push(new_examples_node(&names, &labels));
            }
        }

        for id in by_id.keys() {
            diff.extra_nodes.push(id.clone());
        }
        next_nodes.sort_by(|a, b| {
            a.get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .cmp(b.get("id").and_then(|v| v.as_str()).unwrap_or(""))
        });
        *nodes = next_nodes;

        nodes
            .iter()
            .filter_map(|n| n.get("id").and_then(|v| v.as_str()).map(str::to_string))
            .collect()
    };

    // Expected cargo edges: non-example → non-example, plus the aggregate.
    let mut expected: BTreeMap<(String, String), bool> = BTreeMap::new();
    for ((from, to), optional) in &graph.deps {
        let from_ex = graph
            .packages
            .iter()
            .any(|p| p.name == *from && p.is_example);
        let to_ex = graph.packages.iter().any(|p| p.name == *to && p.is_example);
        if from_ex || to_ex {
            continue;
        }
        if node_ids.contains(from) && node_ids.contains(to) {
            expected.insert((from.clone(), to.clone()), *optional);
        }
    }
    let mut agg: BTreeMap<String, Vec<bool>> = BTreeMap::new();
    for ((from, to), optional) in &graph.deps {
        let from_ex = graph
            .packages
            .iter()
            .any(|p| p.name == *from && p.is_example);
        if from_ex && node_ids.contains(to) && !examples.is_empty() {
            agg.entry(to.clone()).or_default().push(*optional);
        }
    }
    for (to, flags) in agg {
        expected.insert(
            ("examples_collapsed".to_string(), to),
            flags.iter().all(|o| *o),
        );
    }

    let edges = data
        .get_mut("edges")
        .and_then(|e| e.as_array_mut())
        .context("workspace-data has no `edges` array")?;
    let mut by_edge: BTreeMap<(String, String), Value> = BTreeMap::new();
    for e in edges.drain(..) {
        let s = e
            .get("source")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let t = e
            .get("target")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        by_edge.entry((s, t)).or_insert(e);
    }
    let mut next_edges = Vec::new();
    for ((src, tgt), optional) in &expected {
        let key = (src.clone(), tgt.clone());
        if let Some(mut edge) = by_edge.remove(&key) {
            let obj = edge.as_object_mut().context("edge is not an object")?;
            let kind = obj.get("kind").and_then(|v| v.as_str()).unwrap_or("");
            if kind != "declared-unused" && kind != "anomalous" {
                obj.insert("kind".into(), Value::String("workspace-dep".into()));
            }
            let had = obj.get("optional").and_then(|v| v.as_bool());
            if had != Some(*optional) {
                let old = had
                    .map(|b| b.to_string())
                    .unwrap_or_else(|| "absent".to_string());
                diff.optional_changes
                    .push(format!("{src} -> {tgt} optional {old} -> {optional}"));
            }
            obj.insert("optional".into(), Value::Bool(*optional));
            obj.remove("runtime");
            obj.remove("runtimeNote");
            next_edges.push(edge);
        } else {
            diff.missing_edges.push(format!("{src} -> {tgt}"));
            next_edges.push(new_cargo_edge(src, tgt, *optional));
        }
    }
    for (src, tgt) in by_edge.keys() {
        diff.extra_edges.push(format!("{src} -> {tgt}"));
    }
    next_edges.sort_by(|a, b| {
        let ka = (
            a.get("source").and_then(|v| v.as_str()).unwrap_or(""),
            a.get("target").and_then(|v| v.as_str()).unwrap_or(""),
        );
        let kb = (
            b.get("source").and_then(|v| v.as_str()).unwrap_or(""),
            b.get("target").and_then(|v| v.as_str()).unwrap_or(""),
        );
        ka.cmp(&kb)
    });
    *edges = next_edges;

    sync_counts(data, graph);
    Ok(diff)
}

fn sync_counts(data: &mut Value, graph: &Graph) {
    let members = graph.packages.len();
    let examples = graph.packages.iter().filter(|p| p.is_example).count();
    let nodes = data
        .get("nodes")
        .and_then(|n| n.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    let edges = data
        .get("edges")
        .and_then(|e| e.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    let Some(root) = data.as_object_mut() else {
        return;
    };
    let meta = root
        .entry("metadata")
        .or_insert_with(|| Value::Object(Map::new()));
    if let Some(meta) = meta.as_object_mut() {
        meta.insert("cargoNodeCount".into(), json_u(members));
        meta.insert("exampleCount".into(), json_u(examples));
        meta.insert("renderedNodeCount".into(), json_u(nodes));
        meta.insert("renderedEdgeCount".into(), json_u(edges));
        meta.insert("mapNodeClaim".into(), json_u(nodes));
        meta.insert("cargoEdgeCount".into(), json_u(edges));
    }
}

fn json_u(n: usize) -> Value {
    Value::Number(n.into())
}

pub fn sync_reconciliation(data: &mut Value, graph: &Graph) {
    let members = graph.packages.len();
    let examples = graph.packages.iter().filter(|p| p.is_example).count();
    let non_ex = members - examples;
    let nodes = data
        .get("nodes")
        .and_then(|n| n.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    let edges = data
        .get("edges")
        .and_then(|e| e.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    let note0 = format!(
        "cargo metadata reports {members} workspace members: {non_ex} non-example crates and {examples} examples. The map renders {nodes} nodes and {edges} edges. Nodes are the non-example crates plus the examples aggregate. Edges are the normal path dependencies among those nodes, plus a runtime edge when a trace asserts one that is not also a cargo dependency. Dev-dependencies and build-dependencies are excluded. astraweave-camera was added in 8c551c6ea (2026-05-18). Commit 52b9e711c is not in this repository's history."
    );
    let note1 = format!(
        "Examples count: {examples} (cargo metadata). A crate is an example when its manifest path is under examples/ or contains /examples/. Collapsed to one node. exampleList is that set, sorted by package name."
    );
    let Some(root) = data.as_object_mut() else {
        return;
    };
    let notes = root
        .entry("reconciliationNotes")
        .or_insert_with(|| Value::Array(Vec::new()));
    if let Some(arr) = notes.as_array_mut() {
        let n0 = Value::String(note0);
        let n1 = Value::String(note1);
        if arr.is_empty() {
            arr.push(n0);
            arr.push(n1);
        } else if arr.len() == 1 {
            arr[0] = n0;
            arr.push(n1);
        } else {
            arr[0] = n0;
            arr[1] = n1;
        }
    }
}

fn refresh_examples_node(node: &mut Value, names: &[String]) {
    let n = names.len();
    if let Some(obj) = node.as_object_mut() {
        obj.insert("label".into(), Value::String(format!("examples ({n})")));
        obj.insert(
            "exampleList".into(),
            Value::Array(names.iter().cloned().map(Value::String).collect()),
        );
        obj.insert("isExample".into(), Value::Bool(true));
        obj.insert(
            "statusEvidence".into(),
            Value::String(format!(
                "{n} example crates collapsed into one node. exampleList is the cargo workspace members whose manifest path is under examples/."
            )),
        );
    }
}

fn new_examples_node(names: &[String], labels: &BTreeMap<String, String>) -> Value {
    let mut node = Map::new();
    let n = names.len();
    node.insert("id".into(), Value::String("examples_collapsed".into()));
    node.insert("label".into(), Value::String(format!("examples ({n})")));
    node.insert("domain".into(), Value::String("examples".into()));
    node.insert(
        "domainLabel".into(),
        Value::String(label_for(labels, "examples")),
    );
    node.insert("kind".into(), Value::String("example".into()));
    node.insert("locBand".into(), Value::String("medium".into()));
    node.insert("locApprox".into(), json_u(0));
    node.insert("status".into(), Value::String("active".into()));
    node.insert(
        "statusCategory".into(),
        Value::String("examples-aggregate".into()),
    );
    node.insert("statusEvidence".into(), Value::Null);
    node.insert("trace".into(), Value::Null);
    node.insert("silentFailures".into(), Value::Array(Vec::new()));
    node.insert("aspirationalDocs".into(), Value::Null);
    node.insert("openQuestions".into(), Value::Array(Vec::new()));
    node.insert("isExample".into(), Value::Bool(true));
    node.insert(
        "exampleList".into(),
        Value::Array(names.iter().cloned().map(Value::String).collect()),
    );
    let mut v = Value::Object(node);
    refresh_examples_node(&mut v, names);
    v
}

fn new_node(
    name: &str,
    pkg: &Pkg,
    traces: &[TraceDoc],
    labels: &BTreeMap<String, String>,
    root: &Path,
) -> Value {
    let owner = traces.iter().find_map(|t| {
        t.fm.as_ref().and_then(|fm| {
            if fm.owns.iter().any(|c| c == name) {
                Some(fm)
            } else {
                None
            }
        })
    });
    let domain = owner
        .map(|fm| domain_id(fm.domain).to_string())
        .unwrap_or_else(|| default_domain(&pkg.manifest, name).to_string());
    let loc = src_rs_lines(root, &pkg.manifest);
    let mut node = Map::new();
    node.insert("id".into(), Value::String(name.into()));
    node.insert("label".into(), Value::String(name.into()));
    node.insert("domain".into(), Value::String(domain.clone()));
    node.insert(
        "domainLabel".into(),
        Value::String(label_for(labels, &domain)),
    );
    node.insert("kind".into(), Value::String(kind_for(&pkg.manifest).into()));
    node.insert("locBand".into(), Value::String(loc_band(loc).into()));
    node.insert("locApprox".into(), json_u(loc));
    node.insert("status".into(), Value::String("active".into()));
    node.insert("statusCategory".into(), Value::String("active".into()));
    node.insert(
        "statusEvidence".into(),
        Value::String(new_node_evidence(name, &pkg.manifest, owner)),
    );
    node.insert("trace".into(), Value::Null);
    node.insert("silentFailures".into(), Value::Array(Vec::new()));
    node.insert("aspirationalDocs".into(), Value::Null);
    node.insert("openQuestions".into(), Value::Array(Vec::new()));
    node.insert("isExample".into(), Value::Bool(false));
    Value::Object(node)
}

fn new_node_evidence(name: &str, manifest: &str, owner: Option<&FrontMatter>) -> String {
    if owner.is_some() {
        return format!("Workspace crate `{name}`. Status text comes from its trace summary when that trace owns it.");
    }
    if manifest.starts_with("tools/") || manifest.contains("/tools/") {
        return format!("Untraced workspace tool `{name}`.");
    }
    format!("Untraced workspace crate `{name}`.")
}

fn refresh_loc(node: &mut Value, root: &Path, pkg: &Pkg, diff: &mut TopologyDiff) {
    let loc = src_rs_lines(root, &pkg.manifest);
    let band = loc_band(loc);
    let old_loc = node.get("locApprox").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
    let old_band = node
        .get("locBand")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if old_loc != loc || old_band != band {
        diff.loc_changes.push(format!(
            "{} loc {old_loc}/{old_band} -> {loc}/{band}",
            pkg.name
        ));
    }
    if let Some(obj) = node.as_object_mut() {
        obj.insert("locApprox".into(), json_u(loc));
        obj.insert("locBand".into(), Value::String(band.into()));
    }
}

fn record_example_list(node: &Value, names: &[String], diff: &mut TopologyDiff) {
    let old: Vec<String> = node
        .get("exampleList")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    if old == names {
        return;
    }
    let old_set: BTreeSet<&str> = old.iter().map(String::as_str).collect();
    let new_set: BTreeSet<&str> = names.iter().map(String::as_str).collect();
    let mut removed: Vec<&str> = old_set.difference(&new_set).copied().collect();
    let mut added: Vec<&str> = new_set.difference(&old_set).copied().collect();
    removed.sort();
    added.sort();
    if removed.is_empty() && added.is_empty() {
        diff.example_list
            .push("exampleList order changed".to_string());
        return;
    }
    if !removed.is_empty() {
        diff.example_list
            .push(format!("exampleList removed {}", removed.join(", ")));
    }
    if !added.is_empty() {
        diff.example_list
            .push(format!("exampleList added {}", added.join(", ")));
    }
}

fn default_domain(manifest: &str, name: &str) -> &'static str {
    if manifest.starts_with("tools/") || manifest.contains("/tools/") {
        "tools"
    } else if manifest.starts_with("net/")
        || manifest.contains("/net/")
        || manifest.starts_with("persistence/")
        || manifest.contains("/persistence/")
    {
        "networking"
    } else if name == "astraweave-localsim" {
        "physics-world"
    } else {
        "core"
    }
}

fn kind_for(manifest: &str) -> &'static str {
    if manifest.contains("/crates/") || manifest.starts_with("crates/") {
        "crate-subdir"
    } else if manifest.contains("/net/") || manifest.starts_with("net/") {
        "net-subcrate"
    } else if manifest.contains("/persistence/") || manifest.starts_with("persistence/") {
        "persistence-subdir"
    } else if manifest.contains("/tools/") || manifest.starts_with("tools/") {
        "tool"
    } else {
        "production"
    }
}

fn loc_band(lines: usize) -> &'static str {
    if lines >= 100_000 {
        "extra-large"
    } else if lines >= 30_000 {
        "large"
    } else if lines >= 5_000 {
        "medium"
    } else {
        "small"
    }
}

fn src_rs_lines(root: &Path, manifest: &str) -> usize {
    let dir = root.join(manifest).parent().unwrap_or(root).join("src");
    fn walk(dir: &Path, total: &mut usize) {
        let Ok(rd) = std::fs::read_dir(dir) else {
            return;
        };
        for ent in rd.flatten() {
            let path = ent.path();
            if path.is_dir() {
                walk(&path, total);
            } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                if let Ok(text) = std::fs::read_to_string(&path) {
                    *total += text.lines().count();
                }
            }
        }
    }
    let mut total = 0;
    walk(&dir, &mut total);
    total
}

fn domain_labels(data: &Value) -> BTreeMap<String, String> {
    let mut labels = BTreeMap::new();
    if let Some(domains) = data.get("domains").and_then(|d| d.as_array()) {
        for d in domains {
            if let (Some(id), Some(label)) = (
                d.get("id").and_then(|v| v.as_str()),
                d.get("label").and_then(|v| v.as_str()),
            ) {
                labels.insert(id.to_string(), label.to_string());
            }
        }
    }
    // A node's domainLabel wins over the compound's longer domain name.
    if let Some(nodes) = data.get("nodes").and_then(|n| n.as_array()) {
        for n in nodes {
            if let (Some(domain), Some(label)) = (
                n.get("domain").and_then(|v| v.as_str()),
                n.get("domainLabel").and_then(|v| v.as_str()),
            ) {
                labels.insert(domain.to_string(), label.to_string());
            }
        }
    }
    labels
}

fn label_for(labels: &BTreeMap<String, String>, domain: &str) -> String {
    labels
        .get(domain)
        .cloned()
        .unwrap_or_else(|| domain.to_string())
}

fn new_cargo_edge(src: &str, tgt: &str, optional: bool) -> Value {
    serde_json::json!({
        "source": src,
        "target": tgt,
        "kind": "workspace-dep",
        "anomaly": null,
        "types": [],
        "risk": "LOW",
        "optional": optional
    })
}

/// Apply front-matter runtime edges. Cargo kind is left alone when the pair
/// is already a cargo edge. Otherwise the edge's kind is `runtime`.
pub fn apply_runtime_edges(
    data: &mut Value,
    declared: &[(String, String, String, Option<String>)],
) -> Result<Vec<String>> {
    let node_ids: BTreeSet<String> = data
        .get("nodes")
        .and_then(|n| n.as_array())
        .map(|ns| {
            ns.iter()
                .filter_map(|n| n.get("id").and_then(|v| v.as_str()).map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    let mut problems = Vec::new();
    let edges = data
        .get_mut("edges")
        .and_then(|e| e.as_array_mut())
        .context("workspace-data has no `edges` array")?;
    let mut want: BTreeMap<(String, String), (String, Option<String>)> = BTreeMap::new();
    for (file, from, to, note) in declared {
        if !node_ids.contains(from) || !node_ids.contains(to) {
            let missing: Vec<&str> = [from, to]
                .into_iter()
                .filter(|c| !node_ids.contains(c.as_str()))
                .map(String::as_str)
                .collect();
            problems.push(format!(
                "{file}: runtime edge {from} -> {to} has no map node for {}",
                missing.join(", ")
            ));
            continue;
        }
        want.insert((from.clone(), to.clone()), (file.clone(), note.clone()));
    }
    for edge in edges.iter_mut() {
        let s = edge.get("source").and_then(|v| v.as_str()).unwrap_or("");
        let t = edge.get("target").and_then(|v| v.as_str()).unwrap_or("");
        let key = (s.to_string(), t.to_string());
        let obj = edge.as_object_mut().context("edge is not an object")?;
        if let Some((_, note)) = want.remove(&key) {
            obj.insert("runtime".into(), Value::Bool(true));
            if let Some(note) = note {
                obj.insert("runtimeNote".into(), Value::String(note));
            } else {
                obj.remove("runtimeNote");
            }
        }
    }
    for ((from, to), (_, note)) in want {
        let mut edge = serde_json::json!({
            "source": from,
            "target": to,
            "kind": "runtime",
            "anomaly": null,
            "types": [],
            "risk": "LOW",
            "optional": false,
            "runtime": true
        });
        if let Some(note) = note {
            edge.as_object_mut()
                .unwrap()
                .insert("runtimeNote".into(), Value::String(note));
        }
        edges.push(edge);
    }
    edges.sort_by(|a, b| {
        let ka = (
            a.get("source").and_then(|v| v.as_str()).unwrap_or(""),
            a.get("target").and_then(|v| v.as_str()).unwrap_or(""),
        );
        let kb = (
            b.get("source").and_then(|v| v.as_str()).unwrap_or(""),
            b.get("target").and_then(|v| v.as_str()).unwrap_or(""),
        );
        ka.cmp(&kb)
    });
    Ok(problems)
}

pub fn node_trace_files(data: &Value) -> BTreeSet<String> {
    data.get("nodes")
        .and_then(|n| n.as_array())
        .map(|ns| {
            ns.iter()
                .filter_map(|n| n.get("trace").and_then(|v| v.as_str()).map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// Anchors the page's trace deep-links use. A linked trace must contain a
/// heading whose GitHub slug equals the anchor, or an explicit `id`.
pub fn anchor_problems(html: &str, traces: &[TraceDoc], linked: &BTreeSet<String>) -> Vec<String> {
    let required = required_anchors(html);
    if required.is_empty() {
        return vec![
            "workspace_map.html names no trace-section anchors (#5-active-file-map, #6-conflict-map, #7-decision-log, #8-invariants, #11-open-questions)".into(),
        ];
    }
    let mut problems = Vec::new();
    for t in traces {
        if !linked.contains(&t.file()) {
            continue;
        }
        let slugs = heading_slugs(&t.body);
        let ids = explicit_ids(&t.body);
        for anchor in &required {
            if slugs.contains(anchor) || ids.contains(anchor) {
                continue;
            }
            problems.push(format!(
                "{}: trace section anchor '#{anchor}' does not resolve",
                t.file()
            ));
        }
        for id in duplicate_explicit_ids(&t.body) {
            problems.push(format!("{}: duplicate explicit id '{id}'", t.file()));
        }
    }
    problems
}

/// Explicit `id="..."` values that occur more than once. A second copy of a
/// section anchor (for example on `## 11.5`) still matches the first, so the
/// presence check above does not catch it.
fn duplicate_explicit_ids(md: &str) -> Vec<String> {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    let bytes = md.as_bytes();
    let mut i = 0;
    while i + 4 < bytes.len() {
        if bytes[i..].starts_with(b"id=\"") {
            let start = i + 4;
            if let Some(rel) = md[start..].find('"') {
                let id = md[start..start + rel].to_string();
                *counts.entry(id).or_insert(0) += 1;
                i = start + rel;
                continue;
            }
        }
        i += 1;
    }
    counts
        .into_iter()
        .filter(|(_, n)| *n > 1)
        .map(|(id, _)| id)
        .collect()
}

fn required_anchors(html: &str) -> Vec<String> {
    const NEED: &[&str] = &[
        "5-active-file-map",
        "6-conflict-map",
        "7-decision-log",
        "8-invariants",
        "11-open-questions",
    ];
    NEED.iter()
        .filter(|a| html.contains(&format!("#{a}")))
        .map(|s| (*s).to_string())
        .collect()
}

fn heading_slugs(md: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for line in md.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("## ") {
            out.insert(github_slug(rest));
        }
    }
    out
}

fn explicit_ids(md: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let bytes = md.as_bytes();
    let mut i = 0;
    while i + 4 < bytes.len() {
        if bytes[i..].starts_with(b"id=\"") {
            let start = i + 4;
            if let Some(rel) = md[start..].find('"') {
                out.insert(md[start..start + rel].to_string());
                i = start + rel;
                continue;
            }
        }
        if bytes[i..].starts_with(b"{#") {
            let start = i + 2;
            if let Some(rel) = md[start..].find('}') {
                out.insert(md[start..start + rel].to_string());
                i = start + rel;
                continue;
            }
        }
        i += 1;
    }
    out
}

/// GitHub heading slug: lowercase, drop punctuation, spaces become hyphens.
/// Hyphens are not collapsed, so `Map / Residue` becomes `map--residue`.
pub fn github_slug(heading: &str) -> String {
    let mut out = String::new();
    for c in heading.trim().chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if c == ' ' || c == '-' {
            out.push('-');
        }
    }
    out
}

pub fn examples_oneliner(count: usize) -> String {
    format!(
        "{count} example crates collapsed into one node. A crate is an example when its manifest path is under examples/ or contains /examples/."
    )
}

pub fn sync_examples_oneliner(html: &str, count: usize) -> Result<String> {
    let key = "'examples_collapsed':";
    let i = html
        .find(key)
        .context("examples_collapsed one-liner not found")?;
    let rest = &html[i + key.len()..];
    let q0 = rest
        .find('\'')
        .context("examples_collapsed one-liner has no quote")?;
    let q1 = rest[q0 + 1..]
        .find('\'')
        .context("examples_collapsed one-liner is unterminated")?;
    let start = i + key.len() + q0 + 1;
    let end = start + q1;
    Ok(format!(
        "{}{}{}",
        &html[..start],
        examples_oneliner(count),
        &html[end..]
    ))
}

pub fn footer_block(traces: &[TraceDoc]) -> String {
    let mut rows: Vec<&str> = traces
        .iter()
        .filter_map(|t| t.fm.as_ref().map(|fm| fm.trace_id.as_str()))
        .collect();
    rows.sort();
    rows.dedup();
    let n = rows.len();
    let mid = n.div_ceil(2);
    let mut left = String::new();
    let mut right = String::new();
    for (i, id) in rows.iter().enumerate() {
        let item = format!(
            "        <li><a href=\"./{id}.md\" target=\"_blank\" rel=\"noopener\">{id}.md</a></li>\n"
        );
        if i < mid {
            left.push_str(&item);
        } else {
            right.push_str(&item);
        }
    }
    right.push_str(
        "        <li><a href=\"./_meta/\" target=\"_blank\" rel=\"noopener\">_meta/ trace toolkit</a></li>\n",
    );
    format!(
        "\n    <div>\n      <strong>Subsystem Traces ({n})</strong>\n      <ul>\n{left}      </ul>\n    </div>\n    <div>\n      <strong>&nbsp;</strong>\n      <ul>\n{right}      </ul>\n    </div>\n"
    )
}

pub fn members_sentence(graph: &Graph, nodes: usize, edges: usize) -> String {
    let members = graph.packages.len();
    let examples = graph.packages.iter().filter(|p| p.is_example).count();
    let non_ex = members - examples;
    format!(
        "Workspace members from <code>cargo metadata</code>: {members} ({non_ex} non-example crates, {examples} examples). The map renders {nodes} nodes and {edges} edges. Edges are normal path dependencies among those nodes, with example crates collapsed into one source, plus a runtime edge only when it is not also a cargo dependency. Dev-dependencies and build-dependencies are excluded."
    )
}

pub fn replace_marked(current: &str, start: &str, end: &str, inner: &str) -> Result<String> {
    let s = current
        .find(start)
        .with_context(|| format!("`{start}` marker not found in workspace_map.html"))?;
    let after = s + start.len();
    let rel = current[after..]
        .find(end)
        .with_context(|| format!("`{end}` marker not found in workspace_map.html"))?;
    let e = after + rel;
    let inner = match_line_endings(current, inner);
    Ok(format!("{}{}{}", &current[..after], inner, &current[e..]))
}

/// Generated marker text uses `\n`. A CRLF checkout must get CRLF inserts, or
/// `--check` reports a footer change on an otherwise clean Windows tree.
fn match_line_endings(file: &str, inner: &str) -> String {
    if file.contains("\r\n") {
        inner.replace("\r\n", "\n").replace('\n', "\r\n")
    } else {
        inner.replace("\r\n", "\n")
    }
}

pub fn count_nodes_edges(data: &Value) -> (usize, usize) {
    let nodes = data
        .get("nodes")
        .and_then(|n| n.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    let edges = data
        .get("edges")
        .and_then(|e| e.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    (nodes, edges)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn github_slug_keeps_the_double_hyphen_from_a_slash() {
        assert_eq!(github_slug("5. Active File Map"), "5-active-file-map");
        assert_eq!(
            github_slug("6. Conflict Map / Residue"),
            "6-conflict-map--residue"
        );
        assert_eq!(github_slug("8. Known Invariants"), "8-known-invariants");
        assert_eq!(github_slug("8. Invariants"), "8-invariants");
        assert_eq!(
            github_slug("11. Open Questions / Parked Decisions"),
            "11-open-questions--parked-decisions"
        );
    }

    #[test]
    fn explicit_anchor_satisfies_the_check() {
        let md = "<a id=\"8-invariants\"></a>\n## 8. Known Invariants\n";
        let ids = explicit_ids(md);
        assert!(ids.contains("8-invariants"));
        assert!(heading_slugs(md).contains("8-known-invariants"));
    }
}
