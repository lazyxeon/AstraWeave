//! aw_doc_lint — closed-vocabulary documentation-truth lint.
//!
//! The standing regression guard for the Documentation-Truth (D-series) campaign.
//! Two closed vocabularies are the source of truth, read directly from the spec
//! (no duplication — the `aw_trace_sync` pattern): Vocab A (poison strings —
//! retired/fabricated/superseded values) and Vocab B (provenance-free
//! superlatives), both fenced between machine-readable markers in
//! `docs/campaigns/doc-truth/CLOSED_VOCABULARY_LINT.md`. Legitimate occurrences
//! (cited competitor figures, dated records, supersession context, report
//! descriptions, …) are suppressed by `(file, match)` keys read from
//! `docs/campaigns/doc-truth/CLOSED_VOCABULARY_ALLOWLIST.md`.
//!
//! The tool scans the tracked prose surface (all `*.md` plus the gh-pages
//! `.html`/`_config.yml`, minus the campaign's excluded paths), reports every
//! un-allowlisted occurrence, and — in `--mode enforce` — exits nonzero.
//! `--mode warn` (default) annotates but always exits 0 (the warn→enforce rollout).
//!
//! Matching semantics (documented + tested): literal substring match with ASCII
//! word-boundary (`[A-Za-z0-9_]`) at both ends, so `977` does not match inside
//! `9779`/`1977`; longest-literal-first with non-overlapping claims, so a `59.3%`
//! occurrence is recorded as the `59.3%` key (not the bare `59.3` alternative);
//! and CRLF/LF agnostic (matching iterates `str::lines()`, which strips both).
//!
//! Spec: `docs/campaigns/doc-truth/CLOSED_VOCABULARY_LINT.md` (the carve-out
//! section documents why frame-time `2.70 ms` is deliberately NOT in Vocab A).

use anyhow::{bail, Context, Result};
use clap::{Parser, ValueEnum};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;

const SPEC_REL: &str = "docs/campaigns/doc-truth/CLOSED_VOCABULARY_LINT.md";
const ALLOWLIST_REL: &str = "docs/campaigns/doc-truth/CLOSED_VOCABULARY_ALLOWLIST.md";

const VOCAB_A_START: &str = "<!-- VOCAB-A-START -->";
const VOCAB_A_END: &str = "<!-- VOCAB-A-END -->";
const VOCAB_B_START: &str = "<!-- VOCAB-B-START -->";
const VOCAB_B_END: &str = "<!-- VOCAB-B-END -->";
const ALLOWLIST_START: &str = "<!-- ALLOWLIST-START -->";
const ALLOWLIST_END: &str = "<!-- ALLOWLIST-END -->";

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
enum Mode {
    /// Print + annotate violations, always exit 0 (the warn→enforce soak phase).
    Warn,
    /// Print + annotate violations and exit nonzero if any are un-allowlisted.
    Enforce,
}

#[derive(Parser, Debug)]
#[command(
    name = "aw_doc_lint",
    about = "Closed-vocabulary documentation-truth lint (spec-as-source; warn/enforce CI gate)."
)]
struct Cli {
    /// `warn` (default): annotate, exit 0. `enforce`: exit nonzero on any un-allowlisted hit.
    #[arg(long, value_enum, default_value_t = Mode::Warn)]
    mode: Mode,
    /// Repo root. Defaults to `git rev-parse --show-toplevel` from the CWD.
    #[arg(long)]
    root: Option<PathBuf>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Vocab {
    A,
    B,
}
impl Vocab {
    fn tag(self) -> &'static str {
        match self {
            Vocab::A => "VOCAB-A",
            Vocab::B => "VOCAB-B",
        }
    }
}

#[derive(Clone, Debug)]
struct Literal {
    text: String,
    vocab: Vocab,
}

// ---- spec-as-source parsing --------------------------------------------------

/// Slice between `start` and `end` markers (exclusive of both). CRLF/LF agnostic
/// (operates on byte offsets via `find`).
fn extract_between<'a>(content: &'a str, start: &str, end: &str, file: &str) -> Result<&'a str> {
    let s = content
        .find(start)
        .with_context(|| format!("`{start}` marker not found in {file}"))?;
    let after = s + start.len();
    let e = content[after..]
        .find(end)
        .with_context(|| format!("`{end}` marker not found in {file}"))?;
    Ok(&content[after..after + e])
}

/// Parse the poison/superlative literals from a between-markers vocab block:
/// strip `#`-comments, skip fence/blank lines, split content lines on `|`, trim,
/// and strip a trailing `(…)` annotation (e.g. `DFSPH (as live solver)` → `DFSPH`).
fn parse_vocab_block(block: &str, vocab: Vocab) -> Vec<Literal> {
    let mut out = Vec::new();
    for raw in block.lines() {
        // Strip an inline `#` comment (no poison literal contains `#`).
        let line = match raw.find('#') {
            Some(i) => &raw[..i],
            None => raw,
        };
        let line = line.trim();
        if line.is_empty() || line.starts_with("```") {
            continue;
        }
        for tok in line.split('|') {
            let mut t = tok.trim();
            // Strip a trailing parenthetical annotation: the literal is the head.
            if t.ends_with(')') {
                if let Some(open) = t.rfind('(') {
                    t = t[..open].trim_end();
                }
            }
            if !t.is_empty() {
                out.push(Literal {
                    text: t.to_string(),
                    vocab,
                });
            }
        }
    }
    out
}

/// All Vocab A + Vocab B literals, deduped by text, sorted longest-first (so the
/// matcher records `59.3%` rather than the bare `59.3`).
fn load_literals(spec: &str) -> Result<Vec<Literal>> {
    let a = extract_between(spec, VOCAB_A_START, VOCAB_A_END, SPEC_REL)?;
    let b = extract_between(spec, VOCAB_B_START, VOCAB_B_END, SPEC_REL)?;
    let mut lits = parse_vocab_block(a, Vocab::A);
    lits.extend(parse_vocab_block(b, Vocab::B));
    let mut seen = HashSet::new();
    lits.retain(|l| seen.insert(l.text.clone()));
    lits.sort_by(|x, y| {
        y.text
            .len()
            .cmp(&x.text.len())
            .then_with(|| x.text.cmp(&y.text))
    });
    Ok(lits)
}

fn strip_backticks(s: &str) -> String {
    s.trim().trim_matches('`').trim().to_string()
}

/// The `(file, match)` suppression keys from the allowlist table (reason/lifetime
/// are documentation only). Header, separator, and comment rows are skipped.
fn load_allowlist(doc: &str) -> Result<HashSet<(String, String)>> {
    let block = extract_between(doc, ALLOWLIST_START, ALLOWLIST_END, ALLOWLIST_REL)?;
    let mut set = HashSet::new();
    for line in block.lines() {
        let t = line.trim();
        if !t.starts_with('|') {
            continue; // comment / blank / prose
        }
        let fields: Vec<&str> = t.split('|').collect();
        if fields.len() < 4 {
            continue;
        }
        let file = strip_backticks(fields[1]);
        let m = strip_backticks(fields[2]);
        if file.is_empty() || m.is_empty() || file == "File" {
            continue; // header / malformed
        }
        if file.chars().all(|c| c == '-' || c == ':') {
            continue; // separator row
        }
        set.insert((file, m));
    }
    Ok(set)
}

// ---- scope -------------------------------------------------------------------

/// The lint scope (spec §"Lint scope"): all tracked `*.md`, plus the gh-pages
/// `.html`/`_config.yml`, minus the excluded historical/campaign paths.
fn in_scope(path: &str) -> bool {
    let p = path.replace('\\', "/");
    // Exclusions (never linted).
    if p.starts_with("docs/journey/")
        || p.starts_with("docs/archive/")
        || p.starts_with("docs/campaigns/doc-truth/")
        || p == ".github/copilot-instructions-old-backup.md"
    {
        return false;
    }
    // Inclusions.
    if p.ends_with(".md") {
        return true;
    }
    if p.starts_with("gh-pages/") && (p.ends_with(".html") || p.ends_with("_config.yml")) {
        return true;
    }
    false
}

fn tracked_files(root: &Path) -> Result<Vec<String>> {
    let out = Command::new("git")
        .arg("ls-files")
        .current_dir(root)
        .output()
        .context("running `git ls-files`")?;
    if !out.status.success() {
        bail!("`git ls-files` failed (not a git repo?)");
    }
    Ok(String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|s| s.to_string())
        .collect())
}

// ---- matching ----------------------------------------------------------------

fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Hit {
    col: usize,
    text: String,
    vocab: Vocab,
}

/// All boundary-valid, non-overlapping literal matches in one line. `lits` MUST be
/// sorted longest-first; a longer literal claims its span before a shorter one can,
/// so `59.3%` is recorded rather than the bare `59.3`.
fn lint_line(line: &str, lits: &[Literal]) -> Vec<Hit> {
    let bytes = line.as_bytes();
    let mut claimed: Vec<(usize, usize)> = Vec::new();
    let mut hits = Vec::new();
    for lit in lits {
        let l = lit.text.len();
        if l == 0 {
            continue;
        }
        for (idx, _) in line.match_indices(lit.text.as_str()) {
            let end = idx + l;
            let left_ok = idx == 0 || !is_word_byte(bytes[idx - 1]);
            let right_ok = end == bytes.len() || !is_word_byte(bytes[end]);
            if !left_ok || !right_ok {
                continue;
            }
            if claimed.iter().any(|&(s, e)| idx < e && s < end) {
                continue; // overlaps a longer literal already claimed here
            }
            claimed.push((idx, end));
            hits.push(Hit {
                col: idx,
                text: lit.text.clone(),
                vocab: lit.vocab,
            });
        }
    }
    hits
}

fn should_fail(mode: Mode, unallowlisted: usize) -> bool {
    matches!(mode, Mode::Enforce) && unallowlisted > 0
}

fn resolve_root(cli: &Cli) -> Result<PathBuf> {
    if let Some(r) = &cli.root {
        return Ok(r.clone());
    }
    let out = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .context("running `git rev-parse --show-toplevel`")?;
    if !out.status.success() {
        bail!("not inside a git repository (pass --root)");
    }
    Ok(PathBuf::from(
        String::from_utf8_lossy(&out.stdout).trim().to_string(),
    ))
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let root = resolve_root(&cli)?;

    let spec = std::fs::read_to_string(root.join(SPEC_REL))
        .with_context(|| format!("reading spec {SPEC_REL}"))?;
    let allowlist_doc = std::fs::read_to_string(root.join(ALLOWLIST_REL))
        .with_context(|| format!("reading allowlist {ALLOWLIST_REL}"))?;
    let lits = load_literals(&spec)?;
    let allow = load_allowlist(&allowlist_doc)?;

    let files: Vec<String> = tracked_files(&root)?
        .into_iter()
        .filter(|f| in_scope(f))
        .collect();

    let in_ci = std::env::var("GITHUB_ACTIONS")
        .map(|v| v == "true")
        .unwrap_or(false);
    let level = match cli.mode {
        Mode::Warn => "warning",
        Mode::Enforce => "error",
    };

    // (file, line, text, tag) — sortable for deterministic output.
    let mut violations: Vec<(String, usize, String, &'static str)> = Vec::new();
    let mut total_matches = 0usize;
    let mut scanned = 0usize;

    for f in &files {
        let content = match std::fs::read_to_string(root.join(f)) {
            Ok(c) => c,
            Err(_) => continue, // unreadable/binary — skip
        };
        scanned += 1;
        for (i, line) in content.lines().enumerate() {
            for hit in lint_line(line, &lits) {
                total_matches += 1;
                if !allow.contains(&(f.clone(), hit.text.clone())) {
                    violations.push((f.clone(), i + 1, hit.text, hit.vocab.tag()));
                }
            }
        }
    }

    violations.sort();
    for (file, line, text, tag) in &violations {
        println!("{file}:{line}: [{tag}] \"{text}\" (not in allowlist)");
        if in_ci {
            println!(
                "::{level} file={file},line={line}::[{tag}] \"{text}\" not in CLOSED_VOCABULARY_ALLOWLIST.md"
            );
        }
    }

    let k = violations.len();
    eprintln!(
        "{} literal(s); {} file(s) scanned; {} match(es); {} un-allowlisted.",
        lits.len(),
        scanned,
        total_matches,
        k
    );
    if k == 0 {
        eprintln!("doc-truth lint: clean (K=0 un-allowlisted).");
    }

    if should_fail(cli.mode, k) {
        bail!("{k} un-allowlisted closed-vocabulary occurrence(s) (enforce mode)");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lits(spec_a: &str, spec_b: &str) -> Vec<Literal> {
        let spec = format!(
            "intro\n{VOCAB_A_START}\n{spec_a}\n{VOCAB_A_END}\nmid\n{VOCAB_B_START}\n{spec_b}\n{VOCAB_B_END}\ntail\n"
        );
        load_literals(&spec).unwrap()
    }

    #[test]
    fn extract_between_lf_and_crlf() {
        let lf = format!("x\n{VOCAB_A_START}\nBODY\n{VOCAB_A_END}\ny");
        assert_eq!(
            extract_between(&lf, VOCAB_A_START, VOCAB_A_END, "t")
                .unwrap()
                .trim(),
            "BODY"
        );
        let crlf = format!("x\r\n{VOCAB_A_START}\r\nBODY\r\n{VOCAB_A_END}\r\ny");
        assert_eq!(
            extract_between(&crlf, VOCAB_A_START, VOCAB_A_END, "t")
                .unwrap()
                .trim(),
            "BODY"
        );
    }

    #[test]
    fn missing_marker_is_an_error() {
        let s = format!("no markers here {VOCAB_A_END}");
        assert!(extract_between(&s, VOCAB_A_START, VOCAB_A_END, "t").is_err());
    }

    #[test]
    fn vocab_parse_strips_comments_fences_and_splits_pipes() {
        let l = lits(
            "```\n# a section comment\n103,500 | 103k   # superseded\n977\n```",
            "```\nworld-class | world class\n```",
        );
        let texts: HashSet<&str> = l.iter().map(|x| x.text.as_str()).collect();
        assert!(texts.contains("103,500"));
        assert!(texts.contains("103k"));
        assert!(texts.contains("977"));
        assert!(texts.contains("world-class"));
        assert!(texts.contains("world class"));
        // the section-comment and fence lines produce no literals
        assert!(!texts.contains("a section comment"));
        assert!(!texts.iter().any(|t| t.contains("```")));
    }

    #[test]
    fn vocab_parse_strips_trailing_parenthetical_annotation() {
        // line-27 style: the literal is the head, the "(as …)" is an annotation.
        let l = lits(
            "```\nUnifiedSolver | DFSPH (as live solver) | SPH/FLIP (as the solver)\n```",
            "```\n```",
        );
        let texts: HashSet<&str> = l.iter().map(|x| x.text.as_str()).collect();
        assert!(texts.contains("UnifiedSolver"));
        assert!(texts.contains("DFSPH"));
        assert!(texts.contains("SPH/FLIP"));
        assert!(!texts.iter().any(|t| t.contains("(")));
    }

    #[test]
    fn literals_sorted_longest_first() {
        let l = lits("```\n59.3% | 59.3 | 94.57% | 94.57\n977\n```", "```\n```");
        // the first literal whose text matches "59.3" prefix must be the %-form
        let first_593 = l.iter().find(|x| x.text.starts_with("59.3")).unwrap();
        assert_eq!(first_593.text, "59.3%");
        // global ordering is non-increasing in length
        for w in l.windows(2) {
            assert!(w[0].text.len() >= w[1].text.len());
        }
    }

    #[test]
    fn longest_first_records_percent_form_not_bare() {
        let l = lits("```\n59.3% | 59.3\n```", "```\n```");
        // a "59.3%" occurrence is recorded exactly once, as "59.3%"
        let hits = lint_line("coverage 59.3% across crates", &l);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].text, "59.3%");
    }

    #[test]
    fn bare_form_matches_when_no_percent() {
        let l = lits("```\n59.3% | 59.3\n```", "```\n```");
        let hits = lint_line("coverage 59.3 weighted", &l);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].text, "59.3");
    }

    #[test]
    fn word_boundary_for_numbers() {
        let l = lits("```\n977\n```", "```\n```");
        assert_eq!(lint_line("Miri (977 tests, 0 UB)", &l).len(), 1);
        assert_eq!(lint_line("port 9779 open", &l).len(), 0);
        assert_eq!(lint_line("year 1977 was", &l).len(), 0);
        assert_eq!(lint_line("977", &l).len(), 1); // whole-line
    }

    #[test]
    fn no_double_count_via_overlap_claim() {
        let l = lits("```\n59.3% | 59.3 | 94.57% | 94.57\n```", "```\n```");
        let hits = lint_line("59.3% subset vs 94.57% prior", &l);
        let texts: Vec<&str> = hits.iter().map(|h| h.text.as_str()).collect();
        assert_eq!(hits.len(), 2);
        assert!(texts.contains(&"59.3%") && texts.contains(&"94.57%"));
    }

    #[test]
    fn matches_both_vocabs_on_one_line() {
        let l = lits("```\n59.3%\n```", "```\nworld-class\n```");
        let hits = lint_line("a world-class engine at 59.3% coverage", &l);
        assert_eq!(hits.len(), 2);
        assert!(hits
            .iter()
            .any(|h| h.vocab == Vocab::A && h.text == "59.3%"));
        assert!(hits
            .iter()
            .any(|h| h.vocab == Vocab::B && h.text == "world-class"));
    }

    #[test]
    fn crlf_content_matches_via_lines() {
        let l = lits("```\n977\n```", "```\n```");
        let content = "intro\r\nMiri 977 tests\r\ntail\r\n";
        let total: usize = content.lines().map(|ln| lint_line(ln, &l).len()).sum();
        assert_eq!(total, 1);
    }

    #[test]
    fn allowlist_parse_keys_on_file_and_match() {
        let doc = format!(
            "pre\n{ALLOWLIST_START}\n\
             | File | Match | Reason | Lifetime |\n\
             |---|---|---|:-:|\n\
             | `README.md` | `59.3%` | supersession-context | permanent |\n\
             | `CLAUDE.md` | `977` | report-description | permanent |\n\
             <!-- a comment row -->\n\
             {ALLOWLIST_END}\npost"
        );
        let set = load_allowlist(&doc).unwrap();
        assert!(set.contains(&("README.md".to_string(), "59.3%".to_string())));
        assert!(set.contains(&("CLAUDE.md".to_string(), "977".to_string())));
        // header + separator + comment rows are not keys
        assert!(!set.contains(&("File".to_string(), "Match".to_string())));
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn suppression_via_allowlist() {
        let l = lits("```\n977\n```", "```\n```");
        let mut allow = HashSet::new();
        allow.insert((
            "docs/current/MIRI_VALIDATION_REPORT.md".to_string(),
            "977".to_string(),
        ));
        // allowlisted file → suppressed
        let hits = lint_line("Total 977 tests", &l);
        assert_eq!(hits.len(), 1);
        let f1 = "docs/current/MIRI_VALIDATION_REPORT.md".to_string();
        assert!(allow.contains(&(f1, hits[0].text.clone())));
        // a different file with the same match → NOT suppressed
        let f2 = "README.md".to_string();
        assert!(!allow.contains(&(f2, hits[0].text.clone())));
    }

    #[test]
    fn scope_inclusions_and_exclusions() {
        // included
        assert!(in_scope("README.md"));
        assert!(in_scope("docs/current/PROJECT_STATUS.md"));
        assert!(in_scope(".github/copilot-instructions.md"));
        assert!(in_scope(".zencoder/rules/repo.md"));
        assert!(in_scope("astraweave-ecs/README.md"));
        assert!(in_scope("gh-pages/index.html"));
        assert!(in_scope("gh-pages/_config.yml"));
        // excluded
        assert!(!in_scope("docs/journey/weeks/WEEK_8_FINAL_SUMMARY.md"));
        assert!(!in_scope("docs/archive/WEEK_8_OPTIMIZATION_COMPLETE.md"));
        assert!(!in_scope(
            "docs/campaigns/doc-truth/CLOSED_VOCABULARY_LINT.md"
        ));
        assert!(!in_scope(".github/copilot-instructions-old-backup.md"));
        // out of scope by type
        assert!(!in_scope("src/main.rs"));
        assert!(!in_scope("gh-pages/assets/landing.css"));
        assert!(!in_scope("Cargo.toml"));
        // backslash paths normalize
        assert!(in_scope("docs\\current\\PROJECT_STATUS.md"));
        assert!(!in_scope("docs\\journey\\x.md"));
    }

    #[test]
    fn mode_exit_semantics() {
        assert!(!should_fail(Mode::Warn, 5)); // warn never fails
        assert!(!should_fail(Mode::Enforce, 0)); // clean enforce passes
        assert!(should_fail(Mode::Enforce, 1)); // dirty enforce fails
    }
}
