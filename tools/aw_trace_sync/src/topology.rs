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
use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path};

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
        meta.remove("cargoNodeCount");
        meta.insert("cargoMemberCount".into(), json_u(members));
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
    // `lstat` on `src` does not see a symlink in a parent, and `read_dir`
    // would follow it. Refuse a linked parent, and refuse a tree that
    // canonicalizes outside the repo root.
    if !src_tree_stays_in_repo(root, &dir) {
        return 0;
    }
    // `read_dir` follows a directory symlink. A `src` that is itself a link
    // is not this crate's tree, so the count stays 0.
    if std::fs::symlink_metadata(&dir)
        .map(|meta| meta.file_type().is_symlink())
        .unwrap_or(false)
    {
        return 0;
    }
    fn walk(dir: &Path, total: &mut usize) {
        let Ok(rd) = std::fs::read_dir(dir) else {
            return;
        };
        for ent in rd.flatten() {
            // `file_type` does not follow links. `path.is_dir()` would, and a
            // link can point outside the crate or back at itself.
            let Ok(ft) = ent.file_type() else {
                continue;
            };
            if ft.is_symlink() {
                continue;
            }
            let path = ent.path();
            if ft.is_dir() {
                walk(&path, total);
            } else if ft.is_file() && path.extension().and_then(|e| e.to_str()) == Some("rs") {
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

/// False when any path component of `dir` under `root` is a symlink, or when
/// `dir` canonicalizes outside `root`. A missing `src` is outside.
fn src_tree_stays_in_repo(root: &Path, dir: &Path) -> bool {
    if let Ok(rel) = dir.strip_prefix(root) {
        if path_has_symlink(root, rel) {
            return false;
        }
    }
    match (std::fs::canonicalize(root), std::fs::canonicalize(dir)) {
        (Ok(root_abs), Ok(dir_abs)) => dir_abs.starts_with(&root_abs),
        _ => false,
    }
}

fn path_has_symlink(root: &Path, rel: &Path) -> bool {
    let mut acc = root.to_path_buf();
    for component in rel.components() {
        match component {
            Component::Normal(name) => {
                acc.push(name);
                if std::fs::symlink_metadata(&acc)
                    .map(|meta| meta.file_type().is_symlink())
                    .unwrap_or(false)
                {
                    return true;
                }
            }
            Component::ParentDir => {
                acc.pop();
            }
            Component::CurDir => {}
            Component::Prefix(_) | Component::RootDir => return true,
        }
    }
    false
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
    let all = all_trace_anchors();
    if required.len() != all.len() {
        let missing: Vec<String> = all
            .iter()
            .copied()
            .filter(|a| !required.iter().any(|r| r == a))
            .map(|a| format!("#{a}"))
            .collect();
        return vec![format!(
            "workspace_map.html links some trace-section anchors but not {}",
            missing.join(", ")
        )];
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

/// Explicit anchor ids that occur more than once. A second copy of a section
/// anchor still matches the first, so the presence check above does not catch it.
fn duplicate_explicit_ids(md: &str) -> Vec<String> {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for id in anchor_tag_ids(md) {
        *counts.entry(id).or_insert(0) += 1;
    }
    counts
        .into_iter()
        .filter(|(_, n)| *n > 1)
        .map(|(id, _)| id)
        .collect()
}

fn all_trace_anchors() -> &'static [&'static str] {
    &[
        "5-active-file-map",
        "6-conflict-map",
        "7-decision-log",
        "8-invariants",
        "11-open-questions",
    ]
}

fn required_anchors(html: &str) -> Vec<String> {
    all_trace_anchors()
        .iter()
        .filter(|a| html_links_anchor(html, a))
        .map(|s| (*s).to_string())
        .collect()
}

/// A page link counts only when the anchor is a whole token: `#name` followed
/// by a quote or a backtick. A longer token that merely starts with the name
/// does not.
fn html_links_anchor(html: &str, anchor: &str) -> bool {
    let needle = format!("#{anchor}");
    let bytes = html.as_bytes();
    let mut from = 0;
    while let Some(rel) = html[from..].find(&needle) {
        let after = from + rel + needle.len();
        match bytes.get(after).copied() {
            Some(b'"' | b'\'' | b'`') => return true,
            _ => from += rel + 1,
        }
    }
    false
}

/// Tables, strikethrough, task lists, and the GFM blockquote tags. Fences
/// and code spans are CommonMark and stay on without these flags.
fn gfm_options() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_GFM
}

fn heading_slugs(md: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut text: Option<String> = None;
    for event in Parser::new_ext(md, gfm_options()) {
        match event {
            Event::Start(Tag::Heading {
                level: HeadingLevel::H2,
                ..
            }) => {
                text = Some(String::new());
            }
            Event::End(TagEnd::Heading(HeadingLevel::H2)) => {
                if let Some(heading) = text.take() {
                    out.insert(github_slug(&heading));
                }
            }
            Event::Text(s) | Event::Code(s) => {
                if let Some(buf) = text.as_mut() {
                    buf.push_str(&s);
                }
            }
            // Soft and hard breaks fall through here. GitHub's slugger drops
            // the line break; inserting a space would turn it into a hyphen,
            // so `8 Invari` / `ants` would miss `8-invariants`.
            _ => {}
        }
    }
    out
}

fn explicit_ids(md: &str) -> BTreeSet<String> {
    anchor_tag_ids(md).into_iter().collect()
}

/// Ids from `<a id>` and `<a name>` that the GFM parser renders.
///
/// Code fences, indented code, and code spans are not events here. An anchor
/// indented 1–3 spaces, or sitting mid-line, still counts. The tag name is the
/// whole token, so `<abbr` is not `<a`, and `data-id` is not `id`.
///
/// GitHub's tagfilter escapes `script`, `style`, `textarea`, `xmp`, `title`,
/// `iframe`, `noembed`, `noframes`, and `plaintext`, so an anchor inside one
/// of those tags is real. An anchor inside `select` is not. `<!-->` and
/// `<!--->` close a comment, and so does `--!>`. `<![CDATA[` hides until the
/// next `>`. `<!` followed by an uppercase ASCII letter hides until the next
/// `>`. `<?` does not hide the following anchor. Comment, CDATA, declaration,
/// and `select` state carry across HTML events. An id inside another
/// attribute's quoted value is not an element, including on an end tag. The
/// first `id` and the first `name` on a start tag are kept, and a valueless
/// copy still counts as seen. An attribute name may contain `.`, so `data.id`
/// is not `id`.
fn anchor_tag_ids(md: &str) -> Vec<String> {
    let mut ids = Vec::new();
    let mut scan = AnchorScan::default();
    for event in Parser::new_ext(md, gfm_options()) {
        let chunk = match event {
            Event::Html(html) | Event::InlineHtml(html) => html,
            _ => continue,
        };
        scan.feed(&chunk, &mut ids);
    }
    ids
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Skip {
    None,
    Comment,
    Cdata,
    /// `<!` plus an uppercase ASCII letter, through the next `>`.
    Decl,
    /// Through the matching `</select>`.
    Select,
}

struct AnchorScan {
    skip: Skip,
    /// Unfinished `<...`, or the suffix of an unclosed comment, CDATA,
    /// declaration, or `select` that is a prefix of its closer, so the closer
    /// can span HTML events.
    pending: String,
}

impl Default for AnchorScan {
    fn default() -> Self {
        Self {
            skip: Skip::None,
            pending: String::new(),
        }
    }
}

impl AnchorScan {
    fn feed(&mut self, input: &str, ids: &mut Vec<String>) {
        let mut buf = std::mem::take(&mut self.pending);
        buf.push_str(input);
        let bytes = buf.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if !buf.is_char_boundary(i) {
                i += 1;
                continue;
            }
            match self.skip {
                Skip::Comment => {
                    if let Some((rel, len)) = comment_end(&buf[i..]) {
                        i += rel + len;
                        self.skip = Skip::None;
                        continue;
                    }
                    self.pending = carry_comment(&buf[i..]);
                    return;
                }
                Skip::Cdata | Skip::Decl => {
                    if let Some(rel) = buf[i..].find('>') {
                        i += rel + 1;
                        self.skip = Skip::None;
                        continue;
                    }
                    self.pending = carry_tail(&buf[i..], ">");
                    return;
                }
                Skip::Select => match select_end(&buf[i..]) {
                    SelectEnd::Closed(consumed) => {
                        i += consumed;
                        self.skip = Skip::None;
                        continue;
                    }
                    SelectEnd::Open(from) => {
                        self.pending = buf[i + from..].to_string();
                        return;
                    }
                },
                Skip::None => {}
            }
            if bytes[i] != b'<' {
                i += 1;
                continue;
            }
            match classify_lt(&buf[i..]) {
                Lt::Skip { consumed } => i += consumed,
                Lt::Enter(kind, consumed) => {
                    self.skip = kind;
                    i += consumed;
                }
                Lt::NeedMore => {
                    self.pending = buf[i..].to_string();
                    return;
                }
                Lt::Start(tag) => {
                    if tag.name.eq_ignore_ascii_case("select") {
                        self.skip = Skip::Select;
                    } else {
                        ids.extend(tag.ids);
                    }
                    i += 1 + tag.consumed;
                }
                Lt::NotTag => i += 1,
            }
        }
    }
}

/// Earliest of `-->` and `--!>`.
fn comment_end(s: &str) -> Option<(usize, usize)> {
    let dash = s.find("-->").map(|i| (i, 3));
    let bang = s.find("--!>").map(|i| (i, 4));
    match (dash, bang) {
        (Some(dash), Some(bang)) if dash.0 <= bang.0 => Some(dash),
        (Some(_), Some(bang)) => Some(bang),
        (Some(dash), None) => Some(dash),
        (None, Some(bang)) => Some(bang),
        (None, None) => None,
    }
}

fn carry_comment(s: &str) -> String {
    let dash = carry_tail(s, "-->");
    let bang = carry_tail(s, "--!>");
    if bang.len() > dash.len() {
        bang
    } else {
        dash
    }
}

enum SelectEnd {
    /// Bytes from the start of `s` through the end tag's `>`.
    Closed(usize),
    /// An end tag started at this offset and has no `>` yet.
    Open(usize),
}

fn select_end(s: &str) -> SelectEnd {
    let b = s.as_bytes();
    let mut i = 0;
    while i + 1 < b.len() {
        if b[i] == b'<' && b[i + 1] == b'/' {
            let name_at = i + 2;
            if s[name_at..].len() >= "select".len()
                && s.is_char_boundary(name_at + "select".len())
                && s[name_at..name_at + "select".len()].eq_ignore_ascii_case("select")
            {
                let mut j = name_at + "select".len();
                let boundary =
                    j >= b.len() || b[j].is_ascii_whitespace() || b[j] == b'>' || b[j] == b'/';
                if boundary {
                    loop {
                        if j >= b.len() {
                            return SelectEnd::Open(i);
                        }
                        if b[j] == b'"' || b[j] == b'\'' {
                            let quote = b[j];
                            j += 1;
                            while j < b.len() && b[j] != quote {
                                j += 1;
                            }
                            if j >= b.len() {
                                return SelectEnd::Open(i);
                            }
                            j += 1;
                            continue;
                        }
                        if b[j] == b'>' {
                            return SelectEnd::Closed(j + 1);
                        }
                        j += 1;
                    }
                }
            } else if s.len() - name_at < "select".len()
                && "select".starts_with(&s[name_at..].to_ascii_lowercase())
            {
                return SelectEnd::Open(i);
            }
        }
        i += 1;
    }
    if b.last() == Some(&b'<') {
        return SelectEnd::Open(b.len() - 1);
    }
    SelectEnd::Open(s.len())
}

/// Longest suffix of `s` that is a prefix of `needle`.
///
/// `->` does not carry a `-`: `->` and `>` are not prefixes of `-->`, so the
/// next chunk's `->` does not close the comment.
fn carry_tail(s: &str, needle: &str) -> String {
    let max = needle.len().min(s.len());
    for len in (1..=max).rev() {
        let start = s.len() - len;
        if s.is_char_boundary(start) && needle.starts_with(&s[start..]) {
            return s[start..].to_string();
        }
    }
    String::new()
}

enum Lt {
    /// Bytes to advance, including the `<`. A closed `<!-->` / `<!--->` or an
    /// end tag. The contents are not anchors.
    Skip {
        consumed: usize,
    },
    Enter(Skip, usize),
    NeedMore,
    Start(DoneTag),
    NotTag,
}

/// `s` begins at `<`.
fn classify_lt(s: &str) -> Lt {
    let rest = &s[1..];
    if rest.starts_with("!--") || "!--".starts_with(rest) {
        return comment_at(rest);
    }
    if rest.starts_with("![CDATA[") {
        return Lt::Enter(Skip::Cdata, 1 + "![CDATA[".len());
    }
    if "![CDATA[".starts_with(rest) {
        return Lt::NeedMore;
    }
    // GitHub drops `<?` and leaves the following anchor in the text.
    if rest.starts_with('?') {
        return Lt::Skip { consumed: 2 };
    }
    if declaration_at(rest) {
        return Lt::Enter(Skip::Decl, 2);
    }
    if rest.starts_with('/') {
        return end_tag(rest);
    }
    match parse_start_tag(rest) {
        StartTag::Done(tag) => Lt::Start(tag),
        StartTag::NeedMore => Lt::NeedMore,
        StartTag::NotTag => Lt::NotTag,
    }
}

/// `rest` is the bytes after `<` and is either a prefix of `!--` or starts with it.
fn comment_at(rest: &str) -> Lt {
    if rest.len() < 3 {
        return Lt::NeedMore;
    }
    let after = &rest[3..];
    if after.is_empty() || after == "-" {
        return Lt::NeedMore;
    }
    if after.starts_with('>') {
        return Lt::Skip { consumed: 5 };
    }
    if after.starts_with("->") {
        return Lt::Skip { consumed: 6 };
    }
    Lt::Enter(Skip::Comment, 4)
}

/// `<!DOCTYPE` hides. `<!x` does not. `<!--` is a comment and is handled first.
fn declaration_at(rest: &str) -> bool {
    let b = rest.as_bytes();
    b.len() >= 2 && b[0] == b'!' && b[1].is_ascii_uppercase()
}

fn end_tag(rest: &str) -> Lt {
    let b = rest.as_bytes();
    let mut i = 1;
    loop {
        if i >= b.len() {
            return Lt::NeedMore;
        }
        if b[i] == b'"' || b[i] == b'\'' {
            let quote = b[i];
            i += 1;
            while i < b.len() && b[i] != quote {
                i += 1;
            }
            if i >= b.len() {
                return Lt::NeedMore;
            }
            i += 1;
            continue;
        }
        if b[i] == b'>' {
            return Lt::Skip {
                consumed: 1 + i + 1,
            };
        }
        i += 1;
    }
}

fn is_name_start(b: u8) -> bool {
    b.is_ascii_alphabetic()
}

fn is_name_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'-' || b == b':' || b == b'_'
}

fn is_attr_char(b: u8) -> bool {
    is_name_char(b) || b == b'.'
}

struct DoneTag {
    name: String,
    ids: Vec<String>,
    /// Bytes consumed from the slice that begins just after `<`, including `>`.
    consumed: usize,
}

enum StartTag {
    Done(DoneTag),
    NeedMore,
    NotTag,
}

/// `s` begins just after `<`. Attribute values honor quotes, so a `>` or a
/// nested `<a` inside a quoted value does not end the tag or open another.
fn parse_start_tag(s: &str) -> StartTag {
    let b = s.as_bytes();
    if b.is_empty() {
        return StartTag::NeedMore;
    }
    if !is_name_start(b[0]) {
        return StartTag::NotTag;
    }
    let mut i = 1;
    while i < b.len() && is_name_char(b[i]) {
        i += 1;
    }
    if i >= b.len() {
        return StartTag::NeedMore;
    }
    let name = &s[..i];
    let anchor = name.eq_ignore_ascii_case("a");
    let mut ids = Vec::new();
    let mut saw_id = false;
    let mut saw_name = false;
    loop {
        if i >= b.len() {
            return StartTag::NeedMore;
        }
        if b[i] == b'>' {
            return StartTag::Done(DoneTag {
                name: name.to_string(),
                ids,
                consumed: i + 1,
            });
        }
        if b[i] == b'/' {
            i += 1;
            continue;
        }
        if b[i].is_ascii_whitespace() {
            i += 1;
            continue;
        }
        if !is_name_start(b[i]) {
            i += 1;
            continue;
        }
        let attr_at = i;
        i += 1;
        while i < b.len() && is_attr_char(b[i]) {
            i += 1;
        }
        if i >= b.len() {
            return StartTag::NeedMore;
        }
        let attr = &s[attr_at..i];
        while i < b.len() && b[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= b.len() {
            return StartTag::NeedMore;
        }
        if b[i] != b'=' {
            if anchor && attr.eq_ignore_ascii_case("id") {
                saw_id = true;
            } else if anchor && attr.eq_ignore_ascii_case("name") {
                saw_name = true;
            }
            continue;
        }
        i += 1;
        while i < b.len() && b[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= b.len() {
            return StartTag::NeedMore;
        }
        let value = if b[i] == b'"' || b[i] == b'\'' {
            let quote = b[i];
            i += 1;
            let start = i;
            while i < b.len() && b[i] != quote {
                i += 1;
            }
            if i >= b.len() {
                return StartTag::NeedMore;
            }
            let value = s[start..i].to_string();
            i += 1;
            value
        } else {
            let start = i;
            while i < b.len() && !b[i].is_ascii_whitespace() && b[i] != b'>' {
                i += 1;
            }
            s[start..i].to_string()
        };
        if anchor && attr.eq_ignore_ascii_case("id") && !saw_id {
            saw_id = true;
            ids.push(value);
        } else if anchor && attr.eq_ignore_ascii_case("name") && !saw_name {
            saw_name = true;
            ids.push(value);
        }
    }
}

/// GitHub heading slug. Unicode-lowercase first, keep every `\p{Word}`
/// character (letters, marks, decimal digits, connector punctuation, and join
/// controls), turn spaces into hyphens, and drop everything else — including
/// line breaks, so a break does not become a hyphen. Hyphens are not collapsed
/// and the string is not trimmed, so `Map / Residue` becomes `map--residue`
/// and a leading space becomes a leading hyphen.
pub fn github_slug(heading: &str) -> String {
    let mut out = String::with_capacity(heading.len());
    for c in heading.chars() {
        for lower in c.to_lowercase() {
            if is_unicode_word(lower) {
                out.push(lower);
            } else if lower == ' ' || lower == '-' {
                out.push('-');
            }
        }
    }
    out
}

fn is_unicode_word(c: char) -> bool {
    use std::sync::OnceLock;
    static WORD: OnceLock<regex::Regex> = OnceLock::new();
    let re = WORD.get_or_init(|| regex::Regex::new(r"^\w$").expect("word class"));
    let mut buf = [0u8; 4];
    re.is_match(c.encode_utf8(&mut buf))
}

/// Escape a value that is written into HTML text or an attribute.
fn html_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
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
        let id = html_escape(id);
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
        assert_eq!(github_slug("8. Invariants_"), "8-invariants_");
        assert_eq!(
            github_slug("11. Open Questions / Parked Decisions"),
            "11-open-questions--parked-decisions"
        );
    }

    #[test]
    fn github_slug_matches_github_unicode_and_line_breaks() {
        assert_eq!(github_slug("Café Été"), "café-été");
        assert_eq!(github_slug("Über Café"), "über-café");
        assert_eq!(github_slug("Hello\nWorld"), "helloworld");
        assert_eq!(github_slug("Hello\r\nWorld"), "helloworld");
        assert_eq!(github_slug(" a"), "-a");
        assert_eq!(github_slug("a "), "a-");
        assert_eq!(github_slug("Hello 🌍 World"), "hello--world");
        assert_eq!(github_slug("Cafe\u{0301}"), "cafe\u{0301}");
        assert_eq!(github_slug("a\u{203F}b"), "a\u{203f}b");
        assert_eq!(github_slug("a\u{FF3F}b"), "a\u{ff3f}b");
        assert_eq!(github_slug("a\u{200D}b"), "a\u{200d}b");
        assert_eq!(github_slug("a\u{00B7}b"), "ab");
        // Nl is Alphabetic, so \p{Word} keeps it. No (½) is not a word character.
        assert_eq!(github_slug("a\u{2163}b"), "a\u{2173}b");
        assert_eq!(github_slug("a\u{00BD}b"), "ab");
        assert!(heading_slugs("## Café Été\n").contains("café-été"));
        assert!(heading_slugs("Café Été\n--------\n").contains("café-été"));
        assert!(heading_slugs("8 Invari\nants\n-------------\n").contains("8-invariants"));
        assert!(!heading_slugs("8 Invari\nants\n-------------\n").contains("8-invari-ants"));
    }

    fn anchor_ids_in_chunks(chunks: &[&str]) -> Vec<String> {
        let mut scan = AnchorScan::default();
        let mut ids = Vec::new();
        for chunk in chunks {
            scan.feed(chunk, &mut ids);
        }
        ids
    }

    #[test]
    fn comment_state_carries_across_html_chunks() {
        assert!(
            anchor_ids_in_chunks(&["<!--\n", "<a id=\"8-invariants\"></a>\n", "-->\n"]).is_empty(),
            "a comment reset between lines"
        );
        assert!(
            anchor_ids_in_chunks(&["<!-- <a id=\"8-invariants\"></a>\n"]).is_empty(),
            "an unclosed comment revealed the anchor"
        );
        assert_eq!(
            anchor_ids_in_chunks(&["<!-- note -->\n", "<a id=\"8-invariants\"></a>"]),
            vec!["8-invariants".to_string()]
        );
    }

    #[test]
    fn tagfilter_tags_keep_the_inner_anchor() {
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
            "noscript",
            "pre",
            "div",
        ] {
            let one = format!("<{tag}><a id=\"8-invariants\"></a></{tag}>");
            assert_eq!(
                anchor_ids_in_chunks(&[&one]),
                vec!["8-invariants".to_string()],
                "{tag}"
            );
            let split = format!("<{tag}>\n");
            let end = format!("</{tag}>\n");
            assert_eq!(
                anchor_ids_in_chunks(&[&split, "<a id=\"8-invariants\"></a>\n", &end]),
                vec!["8-invariants".to_string()],
                "split {tag}"
            );
        }
    }

    #[test]
    fn cdata_hides_until_the_next_gt_and_pi_does_not() {
        assert!(
            anchor_ids_in_chunks(&["<![CDATA[ <a id=\"8-invariants\"></a> ]]>"]).is_empty(),
            "CDATA revealed an anchor whose own > was the closer"
        );
        assert_eq!(
            anchor_ids_in_chunks(&["x <![CDATA[ q > <a id=\"8-invariants\"></a> ]]>"]),
            vec!["8-invariants".to_string()]
        );
        assert_eq!(
            anchor_ids_in_chunks(&[
                "<![CDATA[ <a id=\"hidden\"></a> ]]><a id=\"8-invariants\"></a>"
            ]),
            vec!["8-invariants".to_string()]
        );
        assert_eq!(
            anchor_ids_in_chunks(&["<? <a id=\"8-invariants\"></a> ?>"]),
            vec!["8-invariants".to_string()]
        );
        assert_eq!(
            anchor_ids_in_chunks(&["x <? q > <a id=\"8-invariants\"></a> ?>"]),
            vec!["8-invariants".to_string()]
        );
    }

    #[test]
    fn a_declaration_hides_only_when_the_next_letter_is_uppercase() {
        assert!(anchor_ids_in_chunks(&["<!DOCTYPE x <a id=\"8-invariants\"></a>"]).is_empty());
        assert_eq!(
            anchor_ids_in_chunks(&["x <!x <a id=\"8-invariants\"></a> y"]),
            vec!["8-invariants".to_string()]
        );
        assert_eq!(
            anchor_ids_in_chunks(&["<!x <a id=\"8-invariants\"></a>"]),
            vec!["8-invariants".to_string()]
        );
    }

    #[test]
    fn select_hides_an_anchor_in_a_block_and_inline() {
        assert!(
            anchor_ids_in_chunks(&["<select>\n<a id=\"8-invariants\"></a>\n</select>\n"])
                .is_empty()
        );
        assert!(
            anchor_ids_in_chunks(&["x <select><a id=\"8-invariants\"></a></select> y"]).is_empty()
        );
        assert_eq!(
            anchor_ids_in_chunks(&[
                "<select>\n<a id=\"hidden\"></a>\n</select><a id=\"8-invariants\"></a>"
            ]),
            vec!["8-invariants".to_string()]
        );
    }

    #[test]
    fn a_comment_closes_on_dash_dash_bang_gt() {
        assert_eq!(
            anchor_ids_in_chunks(&["<!-- x --!> <a id=\"8-invariants\"></a> -->"]),
            vec!["8-invariants".to_string()]
        );
    }

    #[test]
    fn github_slug_lowercases_sigma_one_character_at_a_time() {
        assert_eq!(github_slug("ΑΣ"), "ασ");
        assert_eq!(github_slug("ΣΑΣ x"), "σασ-x");
        assert_eq!(github_slug("a-b"), "a-b");
    }

    #[test]
    fn a_valueless_id_or_name_counts_as_seen() {
        assert!(anchor_ids_in_chunks(&["<a id id=\"8-invariants\"></a>"]).is_empty());
        assert_eq!(
            anchor_ids_in_chunks(&["<a id=\"8-invariants\" id></a>"]),
            vec!["8-invariants".to_string()]
        );
        assert_eq!(
            anchor_ids_in_chunks(&["<a name=\"keep\" name=\"8-invariants\"></a>"]),
            vec!["keep".to_string()]
        );
        assert!(anchor_ids_in_chunks(&["<a name name=\"8-invariants\"></a>"]).is_empty());
    }

    #[test]
    fn an_attribute_name_may_contain_a_dot() {
        assert!(anchor_ids_in_chunks(&["<a data.id=\"8-invariants\"></a>"]).is_empty());
        assert_eq!(
            anchor_ids_in_chunks(&["<a data.id=\"nope\" id=\"8-invariants\"></a>"]),
            vec!["8-invariants".to_string()]
        );
    }

    #[test]
    fn an_end_tag_honors_quotes_and_a_split_end_tag_carries() {
        assert!(anchor_ids_in_chunks(&["</div title=\"<a id=\"8-invariants\">\">"]).is_empty());
        assert_eq!(
            anchor_ids_in_chunks(&[
                "</div title=\"<a id=\"8-invariants\"",
                "\"></div><a id=\"real\"></a>"
            ]),
            vec!["real".to_string()]
        );
    }

    #[test]
    fn a_comment_closes_on_bang_dash_dash_gt() {
        assert_eq!(
            anchor_ids_in_chunks(&["<!--><a id=\"8-invariants\"></a>"]),
            vec!["8-invariants".to_string()]
        );
        assert_eq!(
            anchor_ids_in_chunks(&["<!---><a id=\"8-invariants\"></a>"]),
            vec!["8-invariants".to_string()]
        );
        assert_eq!(
            anchor_ids_in_chunks(&["<!--", "><a id=\"8-invariants\"></a>"]),
            vec!["8-invariants".to_string()]
        );
        assert_eq!(
            anchor_ids_in_chunks(&["<!--", "-><a id=\"8-invariants\"></a>"]),
            vec!["8-invariants".to_string()]
        );
        assert!(
            anchor_ids_in_chunks(&["<!-- note --", "><a id=\"8-invariants\"></a>"])
                == vec!["8-invariants".to_string()]
        );
        assert!(
            anchor_ids_in_chunks(&["<!-- note ->", "-><a id=\"8-invariants\"></a>"]).is_empty(),
            "-> plus -> closed the comment"
        );
        assert_eq!(
            anchor_ids_in_chunks(&["<!- <a id=\"8-invariants\"></a>"]),
            vec!["8-invariants".to_string()]
        );
    }

    #[test]
    fn the_first_id_and_the_first_name_are_the_anchors() {
        assert_eq!(
            anchor_ids_in_chunks(&["<a id=\"x\" id=\"8-invariants\"></a>"]),
            vec!["x".to_string()]
        );
        assert_eq!(
            anchor_ids_in_chunks(&["<a id=\"8-invariants\" id=\"x\"></a>"]),
            vec!["8-invariants".to_string()]
        );
        assert_eq!(
            anchor_ids_in_chunks(&["<a id=\"keep\" id=\"drop\" name=\"name-id\"></a>"]),
            vec!["keep".to_string(), "name-id".to_string()]
        );
        assert_eq!(
            anchor_ids_in_chunks(&["<a name=\"8-invariants\" id=\"x\"></a>"]),
            vec!["8-invariants".to_string(), "x".to_string()]
        );
    }

    #[test]
    fn an_unquoted_id_stops_at_whitespace_or_gt() {
        assert_eq!(
            anchor_ids_in_chunks(&["<a id=8-invariants class=z></a>"]),
            vec!["8-invariants".to_string()]
        );
        assert_eq!(
            anchor_ids_in_chunks(&["<a id=8-invariants></a>"]),
            vec!["8-invariants".to_string()]
        );
    }

    #[test]
    fn a_start_tag_split_across_chunks_still_yields_the_id() {
        assert_eq!(
            anchor_ids_in_chunks(&["<a id=\"8-invar", "iants\"></a>"]),
            vec!["8-invariants".to_string()]
        );
    }

    #[test]
    fn an_id_on_an_end_tag_is_not_an_anchor() {
        assert_eq!(
            anchor_ids_in_chunks(&["</a id=\"8-invariants\"><a id=\"real\"></a>"]),
            vec!["real".to_string()]
        );
        assert_eq!(
            anchor_ids_in_chunks(&["</a id=\"8-invar", "iants\"><a id=\"real\"></a>"]),
            vec!["real".to_string()]
        );
    }

    #[test]
    fn raw_text_bogus_comments_and_attribute_values_are_not_anchors() {
        assert!(anchor_ids_in_chunks(&[
            "<span title=\"<a id=&quot;8-invariants&quot;></a>\"></span>",
            "<div title='<a id=\"8-invariants\"></a>'></div>",
            "<a title=\"<a id=&quot;8-invariants&quot;>\"></a>",
            "<abbr id=\"8-invariants\">x</abbr>"
        ])
        .is_empty());
        assert_eq!(
            anchor_ids_in_chunks(&["<a title=\"id=&quot;nope&quot;\" id=\"8-invariants\"></a>"]),
            vec!["8-invariants".to_string()]
        );
        assert_eq!(
            anchor_ids_in_chunks(&["<A ID='8-invariants'></A>"]),
            vec!["8-invariants".to_string()]
        );
    }

    #[test]
    fn explicit_anchor_satisfies_the_check() {
        let md = "<a id=\"8-invariants\"></a>\n## 8. Known Invariants\n";
        let ids = explicit_ids(md);
        assert!(ids.contains("8-invariants"));
        assert!(heading_slugs(md).contains("8-known-invariants"));
        assert!(
            explicit_ids("<a class=\"x\" name=\"8-invariants\"></a>\n").contains("8-invariants")
        );
    }

    fn assert_line_count_skips_symlinks(
        link_dir: fn(&Path, &Path) -> std::io::Result<()>,
        link_file: fn(&Path, &Path) -> std::io::Result<()>,
    ) {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("demo").join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("lib.rs"), "fn real() {}\n").unwrap();
        let outside = dir.path().join("outside");
        std::fs::create_dir_all(&outside).unwrap();
        std::fs::write(outside.join("extra.rs"), "fn extra() {}\nfn more() {}\n").unwrap();
        if link_dir(&outside, &src.join("linked")).is_err() {
            return;
        }
        if link_file(&outside.join("extra.rs"), &src.join("linked.rs")).is_err() {
            return;
        }
        assert_eq!(src_rs_lines(dir.path(), "demo/Cargo.toml"), 1);

        let real_src = dir.path().join("real_src");
        std::fs::create_dir_all(&real_src).unwrap();
        std::fs::write(real_src.join("extra.rs"), "fn extra() {}\nfn more() {}\n").unwrap();
        let linked_crate = dir.path().join("linked_crate");
        std::fs::create_dir_all(&linked_crate).unwrap();
        if link_dir(&real_src, &linked_crate.join("src")).is_err() {
            return;
        }
        assert_eq!(src_rs_lines(dir.path(), "linked_crate/Cargo.toml"), 0);

        // The target is inside the repo, so a canonical-path check alone would
        // still walk it. The parent of `src` is the link.
        let real = dir.path().join("real_crate");
        std::fs::create_dir_all(real.join("src")).unwrap();
        std::fs::write(
            real.join("src").join("lib.rs"),
            "fn one() {}\nfn two() {}\n",
        )
        .unwrap();
        let alias = dir.path().join("alias_crate");
        if link_dir(&real, &alias).is_err() {
            return;
        }
        assert_eq!(src_rs_lines(dir.path(), "alias_crate/Cargo.toml"), 0);

        let outside = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(outside.path().join("src")).unwrap();
        std::fs::write(
            outside.path().join("src").join("lib.rs"),
            "fn one() {}\nfn two() {}\n",
        )
        .unwrap();
        let escaped = dir.path().join("escaped_crate");
        if link_dir(outside.path(), &escaped).is_err() {
            return;
        }
        assert_eq!(src_rs_lines(dir.path(), "escaped_crate/Cargo.toml"), 0);
    }

    #[test]
    fn line_count_rejects_src_resolved_outside_the_repo() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("repo");
        std::fs::create_dir_all(root.join("demo").join("src")).unwrap();
        std::fs::write(
            root.join("demo").join("src").join("lib.rs"),
            "fn real() {}\n",
        )
        .unwrap();
        let outside = dir.path().join("outside");
        std::fs::create_dir_all(outside.join("src")).unwrap();
        std::fs::write(
            outside.join("src").join("lib.rs"),
            "fn one() {}\nfn two() {}\n",
        )
        .unwrap();
        assert_eq!(src_rs_lines(&root, "demo/Cargo.toml"), 1);
        assert_eq!(src_rs_lines(&root, "../outside/Cargo.toml"), 0);
    }

    #[test]
    fn cargo_member_count_is_not_labeled_as_a_node_count() {
        let mut data = serde_json::json!({
            "nodes": [{"id": "a"}, {"id": "b"}],
            "edges": [{"source": "a", "target": "b"}],
            "metadata": {"cargoNodeCount": 99, "version": "v2.3"}
        });
        let graph = Graph {
            packages: vec![
                Pkg {
                    name: "a".into(),
                    is_example: false,
                    manifest: "a/Cargo.toml".into(),
                },
                Pkg {
                    name: "b".into(),
                    is_example: false,
                    manifest: "b/Cargo.toml".into(),
                },
                Pkg {
                    name: "ex".into(),
                    is_example: true,
                    manifest: "examples/ex/Cargo.toml".into(),
                },
            ],
            deps: BTreeMap::new(),
        };
        sync_counts(&mut data, &graph);
        let meta = data.get("metadata").unwrap();
        assert_eq!(
            meta.get("cargoMemberCount").and_then(|v| v.as_u64()),
            Some(3)
        );
        assert!(meta.get("cargoNodeCount").is_none());
        assert_eq!(
            meta.get("renderedNodeCount").and_then(|v| v.as_u64()),
            Some(2)
        );
    }

    #[test]
    fn the_map_stores_cargo_member_count() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/architecture/workspace_map.html");
        let html = std::fs::read_to_string(&path).unwrap();
        assert!(html.contains("\"cargoMemberCount\":"));
        assert!(!html.contains("cargoNodeCount"));
    }

    #[test]
    fn the_anchor_parser_enables_gfm_options() {
        let opts = gfm_options();
        assert!(opts.contains(Options::ENABLE_TABLES));
        assert!(opts.contains(Options::ENABLE_STRIKETHROUGH));
        assert!(opts.contains(Options::ENABLE_TASKLISTS));
        assert!(opts.contains(Options::ENABLE_GFM));
        assert!(!opts.contains(Options::ENABLE_HEADING_ATTRIBUTES));
        assert!(!opts.contains(Options::ENABLE_SMART_PUNCTUATION));
    }

    #[cfg(unix)]
    #[test]
    fn line_count_skips_symlinks() {
        assert_line_count_skips_symlinks(
            |target, link| std::os::unix::fs::symlink(target, link),
            |target, link| std::os::unix::fs::symlink(target, link),
        );
    }

    #[cfg(windows)]
    #[test]
    fn line_count_skips_symlinks() {
        assert_line_count_skips_symlinks(
            |target, link| std::os::windows::fs::symlink_dir(target, link),
            |target, link| std::os::windows::fs::symlink_file(target, link),
        );
    }
}
