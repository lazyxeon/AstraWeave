//! aw_trace_sync — deterministic architecture-trace registry/map synchronizer.
//!
//! Trace front-matter (YAML at the top of each `docs/architecture/<system>.md`) is
//! the single source of truth for trace↔crate linkage + status. This tool regenerates
//! the marker-bounded trace table in `CLAUDE.md` and the per-crate `trace` links in
//! `docs/architecture/workspace_map.html` from that front-matter, and gates drift via
//! `--check`. Design: `docs/architecture/_meta/TRACE_SYNC_PROPOSAL.md`.
//!
//! v1 scope: front-matter contract + validation + `--list-untraced` + CLAUDE.md trace
//! table generation + map `trace`-link sync. Map *status* sync and `runtime_edges`
//! are deferred to v1.1 (the map keeps its curated status fields untouched here).

use anyhow::{bail, Context, Result};
use clap::Parser;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const TABLE_START: &str = "<!-- TRACE-TABLE:START -->";
const TABLE_END: &str = "<!-- TRACE-TABLE:END -->";
const MAP_ELEMENT_ANCHOR: &str = "id=\"workspace-data\"";

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

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FrontMatter {
    schema_version: u32,
    trace_id: String,
    #[allow(dead_code)]
    title: String,
    description: String,
    primary_crate: String,
    #[allow(dead_code)]
    domain: Domain,
    #[allow(dead_code)]
    lifecycle_status: Lifecycle,
    #[allow(dead_code)]
    integration_status: Integration,
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
}

struct TraceDoc {
    /// filename stem (e.g. "water" for water.md)
    stem: String,
    fm: Option<FrontMatter>,
    fm_error: Option<String>,
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
            Some(yaml) => match serde_yaml::from_str::<FrontMatter>(&yaml) {
                Ok(fm) => (Some(fm), None),
                Err(e) => (None, Some(format!("invalid front-matter: {e}"))),
            },
        };
        out.push(TraceDoc { stem, fm, fm_error });
    }
    Ok(out)
}

struct Pkg {
    name: String,
    /// true for demo/example crates under `examples/` — not subsystems, excluded from `--list-untraced`.
    is_example: bool,
}

fn workspace_packages(root: &Path) -> Result<Vec<Pkg>> {
    let meta = cargo_metadata::MetadataCommand::new()
        .manifest_path(root.join("Cargo.toml"))
        .no_deps()
        .exec()
        .context("running cargo metadata")?;
    let mut pkgs: Vec<Pkg> = meta
        .packages
        .into_iter()
        .map(|p| {
            let mp = p.manifest_path.as_str().replace('\\', "/");
            Pkg {
                name: p.name,
                is_example: mp.contains("/examples/"),
            }
        })
        .collect();
    pkgs.sort_by(|a, b| a.name.cmp(&b.name));
    pkgs.dedup_by(|a, b| a.name == b.name);
    Ok(pkgs)
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

struct MapRender {
    html: String,
    changed_links: Vec<(String, Option<String>, Option<String>)>, // (crate, old, new)
}

fn render_map(current: &str, traces: &[TraceDoc]) -> Result<MapRender> {
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
    let owners = ownership_map(traces);

    let mut changed = Vec::new();
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
            changed.push((id.clone(), have, want.clone()));
        }
        let obj = node.as_object_mut().context("node is not an object")?;
        obj.insert(
            "trace".to_string(),
            match want {
                Some(f) => serde_json::Value::String(f),
                None => serde_json::Value::Null,
            },
        );
    }

    let new_json = escape_nonascii(&serde_json::to_string(&data)?);
    let html = format!(
        "{}{}{}",
        &current[..content_start],
        new_json,
        &current[close..]
    );
    Ok(MapRender {
        html,
        changed_links: changed,
    })
}

// ---- targets / drift ---------------------------------------------------------

struct Target {
    path: PathBuf,
    current: String,
    rendered: String,
}

fn build_targets(root: &Path, traces: &[TraceDoc]) -> Result<(Vec<Target>, MapRender)> {
    let claude_path = root.join("CLAUDE.md");
    let claude_cur = std::fs::read_to_string(&claude_path).context("reading CLAUDE.md")?;
    let claude_new = render_claude_md(&claude_cur, traces)?;

    let map_path = root.join("docs/architecture/workspace_map.html");
    let map_cur = std::fs::read_to_string(&map_path).context("reading workspace_map.html")?;
    let map_render = render_map(&map_cur, traces)?;

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
    let pkgs = workspace_packages(&root)?;
    let crates: Vec<String> = pkgs.iter().map(|p| p.name.clone()).collect();

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
        let mut untraced: Vec<&str> = pkgs
            .iter()
            .filter(|p| !p.is_example && !owners.contains_key(&p.name))
            .map(|p| p.name.as_str())
            .collect();
        untraced.sort();
        let examples = pkgs.iter().filter(|p| p.is_example).count();
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
        let (targets, map_render) = build_targets(&root, &traces)?;
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
            eprintln!("In sync: CLAUDE.md trace table + map trace links match front-matter.");
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
            fm: Some(serde_yaml::from_str(&y).expect("valid front-matter")),
            fm_error: None,
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
        assert!(serde_yaml::from_str::<FrontMatter>(y).is_err());
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
}
